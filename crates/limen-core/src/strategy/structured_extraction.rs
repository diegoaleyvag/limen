//! `structured-extraction@1`.
//!
//! A *transform* strategy: instead of keeping or dropping whole items, it mechanically extracts
//! the sentences that look like they carry a number or date -- "contains at least one ASCII
//! digit" is the deterministic, purely-mechanical stand-in for that -- and discards prose
//! sentences that don't. Extracted sentences are joined back together as the item's
//! [`crate::model::ItemSelectionRecord::output_text`], which the tokenizer/metrics then treat as
//! "the output" for that item (see `output_text`'s doc comment).
//!
//! Algorithm (linear scan, hard cutoff -- but with an inner, sentence-level linear scan per item
//! rather than a single verbatim-item fit check):
//! 1. Process items in ascending `(order_index, source_id)` order (a fully specified total order;
//!    `order_index` is already unique per scenario, so the `source_id` tie-break in practice
//!    never fires).
//! 2. For each item, split `text` into sentences by scanning for `.`, `!`, `?`, or `\n` as
//!    delimiters (see [`split_into_sentences`]): each delimiter ends the sentence it terminates
//!    (the delimiter stays attached to that sentence), and any trailing undelimited remainder
//!    forms one final sentence. Sentences are treated as atomic, indivisible units -- never split
//!    mid-sentence.
//! 3. Keep only sentences containing at least one ASCII digit (`0`-`9`) -- the mechanical
//!    "extraction" rule; numbers and dates are exactly what contain digits.
//! 4. If an item has zero kept sentences, it has nothing extractable: `Dropped`
//!    (`"dropped_no_extractable_content"`, `output_text: None`). This is *not* a budget cutoff --
//!    processing continues normally to the next item.
//! 5. Otherwise, try to include as many of the item's *own* kept sentences (in order, starting
//!    from the first) as fit cumulatively in the remaining budget, checking the token count of
//!    "kept sentences joined so far" against the remaining budget one sentence at a time, never
//!    splitting a sentence:
//!    - If all of them fit: `Included` (`"extracted_included"`).
//!    - If some but not all fit: `Partial` (`"extracted_partial"`), and this is the hard-cutoff
//!      point -- every later item is unconditionally `Dropped` (`"dropped_over_budget"`)
//!      regardless of whether it would have had extractable content.
//!    - If none fit (but at least one kept sentence existed): `Dropped`
//!      (`"dropped_over_budget"`), which is *also* the hard-cutoff point.
//! 6. For `Included`/`Partial` records, `output_text` is the joined text of the kept sentences
//!    actually included (single space between them, original order); `included_byte_start`/
//!    `included_byte_end` span from the start of the first included kept sentence to the end of
//!    the last included kept sentence *in the original `text`* -- a real provenance pointer, even
//!    though `output_text` itself is the filtered/joined extraction rather than that exact
//!    verbatim slice (this is expected; see `output_text`'s documented meaning). Token counts for
//!    budget purposes are computed on `output_text`, never on the raw item text.
//!
//! `selection` is emitted sorted by `order_index` ascending (already true of the processing order
//! itself here, but the sort is applied unconditionally so the contract holds regardless).

use crate::model::{
    Budget, BudgetUsage, ItemSelectionRecord, SelectionOutput, SelectionStatus, StrategyInput,
    TraceStep,
};
use crate::tokenizer::count_tokens;

use super::SelectionStrategy;

/// Exact registry id for this strategy.
pub const ID: &str = "structured-extraction@1";

pub struct StructuredExtractionStrategy;

impl SelectionStrategy for StructuredExtractionStrategy {
    fn id(&self) -> &'static str {
        ID
    }

    fn select(&self, input: &StrategyInput, budget: &Budget) -> SelectionOutput {
        let mut order: Vec<usize> = (0..input.items.len()).collect();
        order.sort_by(|&a, &b| {
            let item_a = &input.items[a];
            let item_b = &input.items[b];
            item_a
                .order_index
                .cmp(&item_b.order_index)
                .then_with(|| item_a.source_id.cmp(&item_b.source_id))
        });

        let mut selection = Vec::with_capacity(input.items.len());
        let mut trace = Vec::with_capacity(input.items.len());
        let mut used_tokens: u32 = 0;
        let mut cutoff_triggered = false;

        for (step_index, &idx) in order.iter().enumerate() {
            let item = &input.items[idx];
            let step_index = step_index as u32;

            if cutoff_triggered {
                selection.push(dropped_record(item));
                trace.push(TraceStep {
                    step_index,
                    source_id: item.source_id.clone(),
                    action: "dropped_over_budget".to_string(),
                    score: None,
                    detail: "dropped: an earlier item's extraction already exhausted the budget (hard cutoff)".to_string(),
                });
                continue;
            }

            let kept: Vec<(u32, u32)> = split_into_sentences(&item.text)
                .into_iter()
                .filter(|&span| sentence_has_digit(&item.text, span))
                .collect();

            if kept.is_empty() {
                selection.push(dropped_record(item));
                trace.push(TraceStep {
                    step_index,
                    source_id: item.source_id.clone(),
                    action: "dropped_no_extractable_content".to_string(),
                    score: None,
                    detail: "dropped: no sentence in this item contains an ASCII digit".to_string(),
                });
                continue;
            }

            let remaining = budget.requested_tokens.saturating_sub(used_tokens);

            // Greedily grow the included prefix of `kept` one sentence at a time. Each
            // sentence's own token count is computed once (O(sentence length)); summing those
            // counts is equivalent to re-tokenizing the growing joined-with-spaces string at
            // every step (every join point sits at an existing delimiter -- already its own
            // punctuation token, or already whitespace -- so inserting one more space there
            // never merges or splits a token), but avoids the O(sentence count * item length)
            // cost of doing that. Monotonic in token count, so the first prefix that doesn't fit
            // ends the search.
            let sentence_tokens: Vec<u32> = kept
                .iter()
                .map(|&(start, end)| count_tokens(&item.text[start as usize..end as usize]))
                .collect();
            let mut included_count = 0usize;
            let mut included_tokens = 0u32;
            for &tokens in &sentence_tokens {
                let cumulative = included_tokens + tokens;
                if cumulative > remaining {
                    break;
                }
                included_count += 1;
                included_tokens = cumulative;
            }

            if included_count == kept.len() {
                let output_text = join_spans(&item.text, &kept);
                used_tokens += included_tokens;
                selection.push(ItemSelectionRecord {
                    source_id: item.source_id.clone(),
                    order_index: item.order_index,
                    status: SelectionStatus::Included,
                    included_byte_start: Some(kept[0].0),
                    included_byte_end: Some(kept[kept.len() - 1].1),
                    output_text: Some(output_text),
                });
                trace.push(TraceStep {
                    step_index,
                    source_id: item.source_id.clone(),
                    action: "extracted_included".to_string(),
                    score: None,
                    detail: format!(
                        "extracted all {} digit-bearing sentence(s) ({included_tokens} tokens); {used_tokens}/{} tokens used",
                        kept.len(),
                        budget.requested_tokens
                    ),
                });
            } else if included_count > 0 {
                let output_text = join_spans(&item.text, &kept[..included_count]);
                used_tokens += included_tokens;
                selection.push(ItemSelectionRecord {
                    source_id: item.source_id.clone(),
                    order_index: item.order_index,
                    status: SelectionStatus::Partial,
                    included_byte_start: Some(kept[0].0),
                    included_byte_end: Some(kept[included_count - 1].1),
                    output_text: Some(output_text),
                });
                trace.push(TraceStep {
                    step_index,
                    source_id: item.source_id.clone(),
                    action: "extracted_partial".to_string(),
                    score: None,
                    detail: format!(
                        "extracted {included_count}/{} digit-bearing sentence(s) ({included_tokens} tokens) before the remaining {remaining}-token budget ran out; every later item is now dropped",
                        kept.len()
                    ),
                });
                cutoff_triggered = true;
            } else {
                selection.push(dropped_record(item));
                trace.push(TraceStep {
                    step_index,
                    source_id: item.source_id.clone(),
                    action: "dropped_over_budget".to_string(),
                    score: None,
                    detail: format!(
                        "dropped: {} digit-bearing sentence(s) existed but none fit in the remaining {remaining}-token budget; every later item is now dropped",
                        kept.len()
                    ),
                });
                cutoff_triggered = true;
            }
        }

        selection.sort_by(|a, b| {
            a.order_index
                .cmp(&b.order_index)
                .then_with(|| a.source_id.cmp(&b.source_id))
        });

        SelectionOutput {
            strategy_id: ID.to_string(),
            budget: BudgetUsage::new(budget.requested_tokens, used_tokens),
            selection,
            trace,
        }
    }
}

/// A `Dropped` record with no retained byte range and no `output_text` (per
/// [`crate::model::ItemSelectionRecord`]'s documented `None`-for-`Dropped` contract).
fn dropped_record(item: &crate::model::ContextItem) -> ItemSelectionRecord {
    ItemSelectionRecord {
        source_id: item.source_id.clone(),
        order_index: item.order_index,
        status: SelectionStatus::Dropped,
        included_byte_start: None,
        included_byte_end: None,
        output_text: None,
    }
}

/// Splits `text` into contiguous sentence byte spans `[start, end)`, which always exactly
/// partition `text`: a sentence ends at (and includes) the first `.`, `!`, `?`, or `\n` after its
/// start; any trailing undelimited remainder forms one final sentence. UTF-8 safe because
/// `char_indices`/`len_utf8` always land on char boundaries, and every delimiter checked here is
/// itself a single-byte ASCII character.
fn split_into_sentences(text: &str) -> Vec<(u32, u32)> {
    let mut sentences = Vec::new();
    let mut start: u32 = 0;
    for (byte_idx, ch) in text.char_indices() {
        if matches!(ch, '.' | '!' | '?' | '\n') {
            let end = byte_idx as u32 + ch.len_utf8() as u32;
            sentences.push((start, end));
            start = end;
        }
    }
    if (start as usize) < text.len() {
        sentences.push((start, text.len() as u32));
    }
    sentences
}

/// `true` iff the sentence spanning `span` in `text` contains at least one ASCII digit -- the
/// deterministic, mechanical stand-in for "looks like it carries a number or date".
fn sentence_has_digit(text: &str, span: (u32, u32)) -> bool {
    text[span.0 as usize..span.1 as usize]
        .bytes()
        .any(|b| b.is_ascii_digit())
}

/// Joins the sentence spans in `spans` (each sliced verbatim from `text`), in order, with a
/// single space between them.
fn join_spans(text: &str, spans: &[(u32, u32)]) -> String {
    spans
        .iter()
        .map(|&(s, e)| &text[s as usize..e as usize])
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ContextItem;

    fn item(source_id: &str, order_index: u32, text: &str) -> ContextItem {
        ContextItem {
            source_id: source_id.to_string(),
            order_index,
            section_label: "spec_section".to_string(),
            text: text.to_string(),
        }
    }

    fn budget(requested_tokens: u32) -> Budget {
        Budget { requested_tokens }
    }

    #[test]
    fn id_matches_registry_constant() {
        assert_eq!(StructuredExtractionStrategy.id(), ID);
        assert_eq!(ID, "structured-extraction@1");
    }

    #[test]
    fn split_into_sentences_keeps_delimiter_attached_and_captures_trailing_remainder() {
        let text = "One. Two! Three? Four\nFive";
        let spans = split_into_sentences(text);
        let rendered: Vec<&str> = spans
            .iter()
            .map(|&(s, e)| &text[s as usize..e as usize])
            .collect();
        assert_eq!(
            rendered,
            vec!["One.", " Two!", " Three?", " Four\n", "Five"]
        );
    }

    #[test]
    fn split_into_sentences_of_undelimited_text_is_one_trailing_sentence() {
        let text = "no punctuation at all";
        let spans = split_into_sentences(text);
        assert_eq!(spans, vec![(0, text.len() as u32)]);
    }

    #[test]
    fn split_into_sentences_of_empty_text_is_empty() {
        assert_eq!(split_into_sentences(""), Vec::<(u32, u32)>::new());
    }

    #[test]
    fn drops_items_with_no_digit_bearing_sentences() {
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![item("a", 0, "No numbers here at all. Just plain prose.")],
        };
        let output = StructuredExtractionStrategy.select(&input, &budget(1000));
        assert_eq!(output.selection[0].status, SelectionStatus::Dropped);
        assert_eq!(output.selection[0].output_text, None);
        assert_eq!(output.trace[0].action, "dropped_no_extractable_content");
        assert_eq!(output.budget.used_tokens, 0);
    }

    #[test]
    fn no_extractable_content_drop_does_not_trigger_a_hard_cutoff() {
        // "a" has no digits (dropped for content, not budget); "b" has a digit and ample budget,
        // so it must still be extracted even though it comes after a Dropped item.
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![
                item("a", 0, "No numbers here at all."),
                item("b", 1, "Reading was 42 units."),
            ],
        };
        let output = StructuredExtractionStrategy.select(&input, &budget(1000));
        assert_eq!(output.selection[0].status, SelectionStatus::Dropped);
        assert_eq!(output.trace[0].action, "dropped_no_extractable_content");
        assert_eq!(output.selection[1].status, SelectionStatus::Included);
        assert_eq!(output.trace[1].action, "extracted_included");
        assert!(output.selection[1]
            .output_text
            .as_deref()
            .unwrap()
            .contains("42"));
    }

    #[test]
    fn keeps_only_digit_bearing_sentences_and_joins_them() {
        let text = "The sky is blue. It happened on 2024-01-05. Also it was nice.";
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![item("a", 0, text)],
        };
        let output = StructuredExtractionStrategy.select(&input, &budget(1000));
        assert_eq!(output.selection[0].status, SelectionStatus::Included);
        let output_text = output.selection[0].output_text.as_deref().unwrap();
        assert!(output_text.contains("2024-01-05"));
        assert!(
            !output_text.contains("blue"),
            "digit-free sentence must be dropped from the extraction: {output_text:?}"
        );
        assert!(
            !output_text.contains("nice"),
            "digit-free sentence must be dropped from the extraction: {output_text:?}"
        );
    }

    #[test]
    fn output_text_not_raw_text_is_what_gets_tokenized_against_budget() {
        // First sentence is long and digit-free (11 tokens); second is short and has a digit (4
        // tokens: "It" "was" "42" "."). A budget of 4 cannot possibly fit the *raw* 15-token item
        // text, but must fit the (correctly) tiny `output_text`.
        let long_no_digit = "aaa bbb ccc ddd eee fff ggg hhh iii jjj.";
        let short_with_digit = " It was 42.";
        let text = format!("{long_no_digit}{short_with_digit}");
        assert_eq!(count_tokens(&text), 15, "sanity-check raw token count");

        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![item("a", 0, &text)],
        };
        let output = StructuredExtractionStrategy.select(&input, &budget(4));
        assert_eq!(output.selection[0].status, SelectionStatus::Included);
        assert_eq!(output.budget.used_tokens, 4);
        let output_text = output.selection[0].output_text.as_deref().unwrap();
        assert!(output_text.contains("42"));
        assert!(!output_text.contains("aaa"));
    }

    #[test]
    fn partial_extraction_when_only_some_kept_sentences_fit_and_triggers_hard_cutoff() {
        // Two digit-bearing sentences of 2 tokens each ("X1." and " Y2."); budget=2 fits only the
        // first.
        let text = "X1. Y2.";
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![item("a", 0, text), item("b", 1, "Also has 9 in it.")],
        };
        let output = StructuredExtractionStrategy.select(&input, &budget(2));

        assert_eq!(output.selection[0].status, SelectionStatus::Partial);
        assert_eq!(output.trace[0].action, "extracted_partial");
        let output_text = output.selection[0].output_text.as_deref().unwrap();
        assert!(output_text.contains('1'));
        assert!(!output_text.contains('2'));

        // "b" has extractable content and would fit on its own, but the hard cutoff from "a"'s
        // partial extraction drops it unconditionally.
        assert_eq!(output.selection[1].status, SelectionStatus::Dropped);
        assert_eq!(output.trace[1].action, "dropped_over_budget");
        assert_eq!(output.budget.used_tokens, 2);
    }

    #[test]
    fn zero_sentences_fit_marks_dropped_over_budget_not_partial() {
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![item("a", 0, "Value is 7.")],
        };
        let output = StructuredExtractionStrategy.select(&input, &budget(0));
        assert_eq!(output.selection[0].status, SelectionStatus::Dropped);
        assert_eq!(output.selection[0].output_text, None);
        assert_eq!(output.trace[0].action, "dropped_over_budget");
        assert_eq!(output.budget.used_tokens, 0);
    }

    #[test]
    fn included_byte_range_points_at_kept_sentences_in_original_text() {
        let text = "Intro with no digits. Value is 42. Trailing prose again.";
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![item("a", 0, text)],
        };
        let output = StructuredExtractionStrategy.select(&input, &budget(1000));
        let record = &output.selection[0];
        let start = record.included_byte_start.unwrap() as usize;
        let end = record.included_byte_end.unwrap() as usize;
        let provenance_slice = &text[start..end];
        assert!(provenance_slice.contains("42"));
        assert!(!provenance_slice.contains("Trailing"));
    }

    #[test]
    fn exact_boundary_budget_behaves_correctly() {
        // Item has two 2-token kept sentences ("A1." and " B2."); an exact budget of 2 (the first
        // sentence's own token count) must yield Partial with exactly the first sentence.
        let text = "A1. B2.";
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![item("a", 0, text)],
        };
        let output = StructuredExtractionStrategy.select(&input, &budget(2));
        assert_eq!(output.selection[0].status, SelectionStatus::Partial);
        assert_eq!(output.budget.used_tokens, 2);
        let output_text = output.selection[0].output_text.as_deref().unwrap();
        assert!(output_text.contains('1'));
        assert!(!output_text.contains('2'));
    }

    #[test]
    fn budget_never_exceeded_across_a_sweep_including_zero() {
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![
                item("a", 0, "Reading one is 12. Reading two is 34."),
                item("b", 1, "No digits in this item at all."),
                item("c", 2, "Final value: 999."),
            ],
        };
        for requested in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 100] {
            let output = StructuredExtractionStrategy.select(&input, &budget(requested));
            assert!(
                output.budget.used_tokens <= output.budget.requested_tokens,
                "used_tokens must never exceed requested_tokens at budget={requested}"
            );
            assert_eq!(output.selection.len(), input.items.len());
        }
    }

    #[test]
    fn selection_has_one_record_per_item_and_is_sorted_by_order_index() {
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![
                item("c", 2, "Value 3."),
                item("a", 0, "Value 1."),
                item("b", 1, "Value 2."),
            ],
        };
        let output = StructuredExtractionStrategy.select(&input, &budget(1000));
        assert_eq!(output.selection.len(), 3);
        let order_indices: Vec<u32> = output.selection.iter().map(|r| r.order_index).collect();
        assert_eq!(order_indices, vec![0, 1, 2]);
    }

    #[test]
    fn determinism_two_calls_produce_byte_identical_output() {
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![
                item("a", 0, "Reading one is 12. Prose with no digits here."),
                item("b", 1, "No digits in this item at all."),
                item("c", 2, "Final value: 999. More prose without numbers."),
            ],
        };
        let first = StructuredExtractionStrategy.select(&input, &budget(6));
        let second = StructuredExtractionStrategy.select(&input, &budget(6));
        assert_eq!(first, second);
    }
}
