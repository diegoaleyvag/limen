//! `hierarchical-summary@1`.
//!
//! A *transform* strategy that builds one fixed-template one-line summary per item out of a
//! verbatim fragment of that item's own text, wrapped with its structural metadata
//! (`section_label`, `source_id`). Per the product brief, any user-facing label for this
//! strategy's output must read exactly **"Hierarchical summary (deterministic/template-based)"**:
//! this is template assembly of a verbatim fragment, never a paraphrasing/generative summary, and
//! must never be described as LLM-generated (see the doc comment on
//! [`HierarchicalSummaryStrategy`] below, which restates this for anyone reading the struct in
//! isolation).
//!
//! Algorithm (linear scan, hard cutoff, but the per-item unit is an atomic templated summary
//! rather than the item's own verbatim text):
//! 1. Process items in ascending `(order_index, source_id)` order (a fully specified total order;
//!    `order_index` is already unique per scenario, so the `source_id` tie-break in practice
//!    never fires).
//! 2. For each item, build its one-line summary sentence via [`first_sentence_snippet`]: the
//!    first occurrence of `.`, `!`, `?`, or `\n` (inclusive of the delimiter) if one exists within
//!    the first 200 bytes of `text`; otherwise the text truncated at the last safe token boundary
//!    at or before byte 200 (never splitting a UTF-8 codepoint), with `"..."` appended.
//! 3. Wrap it: `output_text = format!("[{section_label}] {source_id}: {sentence}")`.
//! 4. This wrapped string is atomic and whole-or-nothing per item -- this strategy never produces
//!    `Partial`. Token counts are on the *wrapped* string (the label prefix counts against budget
//!    too, per the tokenizer's "budget accounting happens on emitted output" rule). If it fits in
//!    the remaining budget: `Included` (`"templated_included"`), with `included_byte_start`/
//!    `included_byte_end` set to `[0, text.len())` -- a full-item provenance pointer, even though
//!    only a summary was templated. If it does not fit: `Dropped` (`"dropped_over_budget"`), and
//!    this is the hard-cutoff point -- every later item is unconditionally `Dropped` with the same
//!    action, regardless of whether its own summary would have fit.
//!
//! `selection` is emitted sorted by `order_index` ascending (already true of the processing order
//! itself here, but the sort is applied unconditionally so the contract holds regardless).

use crate::model::{
    Budget, BudgetUsage, ItemSelectionRecord, SelectionOutput, SelectionStatus, StrategyInput,
    TraceStep,
};
use crate::tokenizer::{count_tokens, tokenize};

use super::SelectionStrategy;

/// Exact registry id for this strategy.
pub const ID: &str = "hierarchical-summary@1";

/// Maximum byte length of the fallback (no-early-delimiter) summary snippet, before the
/// appended `"..."`.
const SUMMARY_SNIPPET_MAX_BYTES: u32 = 200;

/// Deterministic, template-based one-line summarizer. Every summary is assembled mechanically
/// from a verbatim fragment of the source item's own text plus its structural metadata -- there
/// is no paraphrasing, no generative model, and no semantic understanding involved. The
/// product-facing label for this strategy's output is exactly **"Hierarchical summary
/// (deterministic/template-based)"**; it must never be presented or described as LLM-generated.
pub struct HierarchicalSummaryStrategy;

impl SelectionStrategy for HierarchicalSummaryStrategy {
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
                    detail: "dropped: an earlier item's templated summary already exhausted the budget (hard cutoff)".to_string(),
                });
                continue;
            }

            let wrapped = format!(
                "[{}] {}: {}",
                item.section_label,
                item.source_id,
                first_sentence_snippet(&item.text)
            );
            let wrapped_tokens = count_tokens(&wrapped);
            let remaining = budget.requested_tokens.saturating_sub(used_tokens);

            if wrapped_tokens <= remaining {
                used_tokens += wrapped_tokens;
                let byte_end = u32::try_from(item.text.len()).unwrap_or(u32::MAX);
                selection.push(ItemSelectionRecord {
                    source_id: item.source_id.clone(),
                    order_index: item.order_index,
                    status: SelectionStatus::Included,
                    included_byte_start: Some(0),
                    included_byte_end: Some(byte_end),
                    output_text: Some(wrapped),
                });
                trace.push(TraceStep {
                    step_index,
                    source_id: item.source_id.clone(),
                    action: "templated_included".to_string(),
                    score: None,
                    detail: format!(
                        "templated summary ({wrapped_tokens} tokens); {used_tokens}/{} tokens used",
                        budget.requested_tokens
                    ),
                });
            } else {
                cutoff_triggered = true;
                selection.push(dropped_record(item));
                trace.push(TraceStep {
                    step_index,
                    source_id: item.source_id.clone(),
                    action: "dropped_over_budget".to_string(),
                    score: None,
                    detail: format!(
                        "dropped: templated summary needs {wrapped_tokens} tokens but only {remaining} remain; every later item is now dropped"
                    ),
                });
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

/// Builds the one-line "first sentence" snippet used in the templated summary: the first
/// occurrence of `.`, `!`, `?`, or `\n` (inclusive of the delimiter) if one starts within the
/// first [`SUMMARY_SNIPPET_MAX_BYTES`] bytes of `text`; otherwise `text` truncated at the largest
/// safe token boundary at or before that byte limit, with `"..."` appended. Never splits a UTF-8
/// codepoint.
fn first_sentence_snippet(text: &str) -> String {
    let first_delimiter = text
        .char_indices()
        .find(|&(_, ch)| matches!(ch, '.' | '!' | '?' | '\n'));

    if let Some((byte_idx, ch)) = first_delimiter {
        if (byte_idx as u32) < SUMMARY_SNIPPET_MAX_BYTES {
            let end = byte_idx + ch.len_utf8();
            return text[..end].to_string();
        }
    }

    let cut = truncate_at_token_boundary(text, SUMMARY_SNIPPET_MAX_BYTES);
    format!("{}...", &text[..cut as usize])
}

/// Finds the largest token boundary (a token's `byte_end`) that is `<= max_bytes`, scanning
/// tokens in order and stopping as soon as one would exceed the limit (tokens are produced in
/// non-decreasing byte order, so no later token could still satisfy the limit). Returns `0` if
/// even the first token's `byte_end` exceeds `max_bytes`.
fn truncate_at_token_boundary(text: &str, max_bytes: u32) -> u32 {
    let mut end: u32 = 0;
    for token in tokenize(text) {
        if token.byte_end > max_bytes {
            break;
        }
        end = token.byte_end;
    }
    end
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
        assert_eq!(HierarchicalSummaryStrategy.id(), ID);
        assert_eq!(ID, "hierarchical-summary@1");
    }

    #[test]
    fn template_format_wraps_section_label_source_id_and_first_sentence() {
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![item(
                "spec-3",
                0,
                "The system shall respond within 200ms. Additional detail follows.",
            )],
        };
        let output = HierarchicalSummaryStrategy.select(&input, &budget(1000));
        assert_eq!(output.selection[0].status, SelectionStatus::Included);
        let output_text = output.selection[0].output_text.as_deref().unwrap();
        assert_eq!(
            output_text,
            "[spec_section] spec-3: The system shall respond within 200ms."
        );
    }

    #[test]
    fn full_item_provenance_pointer_even_though_only_a_summary_was_templated() {
        let text = "First sentence here. Second sentence has more detail that is not templated.";
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![item("a", 0, text)],
        };
        let output = HierarchicalSummaryStrategy.select(&input, &budget(1000));
        let record = &output.selection[0];
        assert_eq!(record.included_byte_start, Some(0));
        assert_eq!(record.included_byte_end, Some(text.len() as u32));
    }

    #[test]
    fn long_first_sentence_without_early_delimiter_is_cut_at_200_bytes_with_ellipsis() {
        // A single "sentence" with no delimiter anywhere for the first 200+ bytes.
        let long_text = "word ".repeat(60); // 300 bytes, no '.', '!', '?', or '\n' at all.
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![item("a", 0, &long_text)],
        };
        let output = HierarchicalSummaryStrategy.select(&input, &budget(1000));
        let output_text = output.selection[0].output_text.as_deref().unwrap();
        assert!(
            output_text.ends_with("..."),
            "expected ellipsis-truncated snippet, got {output_text:?}"
        );
        // Recover just the snippet portion (after the "[label] id: " prefix) and check its byte
        // budget: the snippet itself (minus the trailing "...") must be <= 200 bytes and must be
        // a prefix of the original text, cut at a token boundary.
        let prefix = "[spec_section] a: ";
        let snippet = &output_text[prefix.len()..];
        let snippet_without_ellipsis = &snippet[..snippet.len() - 3];
        assert!(snippet_without_ellipsis.len() <= 200);
        assert!(long_text.starts_with(snippet_without_ellipsis));
    }

    #[test]
    fn delimiter_found_within_first_200_bytes_is_used_verbatim_with_no_ellipsis() {
        let text = "Short sentence. Trailing content that is not part of the summary at all and goes on for a while just to be safe about the 200 byte cutoff not mattering here.";
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![item("a", 0, text)],
        };
        let output = HierarchicalSummaryStrategy.select(&input, &budget(1000));
        let output_text = output.selection[0].output_text.as_deref().unwrap();
        assert_eq!(output_text, "[spec_section] a: Short sentence.");
        assert!(!output_text.contains("..."));
    }

    #[test]
    fn never_produces_partial_status() {
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![item("a", 0, "one two three four five six seven")],
        };
        for requested in 0..20 {
            let output = HierarchicalSummaryStrategy.select(&input, &budget(requested));
            assert!(
                output
                    .selection
                    .iter()
                    .all(|r| r.status != SelectionStatus::Partial),
                "hierarchical-summary must never produce Partial (budget={requested})"
            );
        }
    }

    #[test]
    fn whole_or_nothing_hard_cutoff_when_a_summary_does_not_fit() {
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![
                item("a", 0, "Short one."),
                item("b", 1, "Also fairly short two."),
                item("c", 2, "Also fairly short three."),
            ],
        };
        // Budget big enough for the first templated summary but not the second.
        let first_only = HierarchicalSummaryStrategy
            .select(&input, &budget(1000))
            .selection[0]
            .output_text
            .clone()
            .unwrap();
        let first_tokens = count_tokens(&first_only);
        let output = HierarchicalSummaryStrategy.select(&input, &budget(first_tokens));

        assert_eq!(output.selection[0].status, SelectionStatus::Included);
        assert_eq!(output.selection[1].status, SelectionStatus::Dropped);
        assert_eq!(output.selection[2].status, SelectionStatus::Dropped);
        assert_eq!(output.trace[1].action, "dropped_over_budget");
        assert_eq!(output.trace[2].action, "dropped_over_budget");
        assert_eq!(output.budget.used_tokens, first_tokens);
    }

    #[test]
    fn zero_budget_drops_every_item() {
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![item("a", 0, "one"), item("b", 1, "two")],
        };
        let output = HierarchicalSummaryStrategy.select(&input, &budget(0));
        assert!(output
            .selection
            .iter()
            .all(|r| r.status == SelectionStatus::Dropped));
        assert_eq!(output.budget.used_tokens, 0);
    }

    #[test]
    fn exact_boundary_budget_includes_exactly_the_first_k_items() {
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![item("a", 0, "Alpha."), item("b", 1, "Beta.")],
        };
        let ample = HierarchicalSummaryStrategy.select(&input, &budget(1000));
        let first_tokens = count_tokens(ample.selection[0].output_text.as_deref().unwrap());

        let output = HierarchicalSummaryStrategy.select(&input, &budget(first_tokens));
        assert_eq!(output.selection[0].status, SelectionStatus::Included);
        assert_eq!(output.selection[1].status, SelectionStatus::Dropped);
        assert_eq!(output.budget.used_tokens, first_tokens);
        assert_eq!(output.budget.remaining_tokens, 0);
    }

    #[test]
    fn budget_never_exceeded_across_a_sweep_including_zero() {
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![
                item("a", 0, "First item with a sentence."),
                item("b", 1, "Second item, also with one."),
                item("c", 2, "Third and final item here."),
            ],
        };
        for requested in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 100] {
            let output = HierarchicalSummaryStrategy.select(&input, &budget(requested));
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
                item("c", 2, "Gamma."),
                item("a", 0, "Alpha."),
                item("b", 1, "Beta."),
            ],
        };
        let output = HierarchicalSummaryStrategy.select(&input, &budget(1000));
        assert_eq!(output.selection.len(), 3);
        let order_indices: Vec<u32> = output.selection.iter().map(|r| r.order_index).collect();
        assert_eq!(order_indices, vec![0, 1, 2]);
    }

    #[test]
    fn determinism_two_calls_produce_byte_identical_output() {
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![
                item("a", 0, "First item with a sentence."),
                item("b", 1, "Second item, also with one."),
                item("c", 2, "Third and final item here."),
            ],
        };
        let first = HierarchicalSummaryStrategy.select(&input, &budget(6));
        let second = HierarchicalSummaryStrategy.select(&input, &budget(6));
        assert_eq!(first, second);
    }
}
