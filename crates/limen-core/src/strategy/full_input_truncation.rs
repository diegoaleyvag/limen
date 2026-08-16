//! `full-input-truncation@1`.
//!
//! The simplest baseline in the strategy matrix: it models an agent that just concatenates every
//! source item, in original order, until the context window runs out. There is no notion of
//! relevance, recency, or structure -- it is a pure prefix cut over the *whole* item sequence.
//!
//! Algorithm (linear scan, hard cutoff):
//! 1. Process items in ascending `(order_index, source_id)` order -- a fully specified total
//!    order (`order_index` first, `source_id` as the documented tie-break; `order_index` is
//!    already unique per scenario, so the tie-break is never actually exercised, but it keeps the
//!    comparator a true total order regardless).
//! 2. For each item, while no earlier item has failed to fit: if the item's verbatim `text` fits
//!    entirely within the remaining budget, include it whole (`"included_full"`). Otherwise,
//!    truncate at the last token boundary that still fits (the safe byte-boundary technique
//!    documented on [`fit_within_budget`]): if at least one token fits, that item is `Partial`
//!    (`"included_partial"`); if zero tokens fit, it is `Dropped` (`"dropped_over_budget"`). This
//!    is the one and only cutoff point.
//! 3. Every item at or after the cutoff point is unconditionally `Dropped`
//!    (`"dropped_over_budget"`) -- no later, smaller item is ever considered for inclusion past
//!    that point, since this strategy never reorders or bin-packs; it only truncates a
//!    concatenated prefix.
//!
//! `selection` is emitted sorted by `order_index` ascending (already true of the processing order
//! itself here, but the sort is applied unconditionally so the contract holds even if a future
//! change to this file altered the processing order).

use crate::model::{
    Budget, BudgetUsage, ItemSelectionRecord, SelectionOutput, SelectionStatus, StrategyInput,
    TraceStep,
};
use crate::tokenizer::{count_tokens, tokenize};

use super::SelectionStrategy;

/// Exact registry id for this strategy.
pub const ID: &str = "full-input-truncation@1";

pub struct FullInputTruncationStrategy;

impl SelectionStrategy for FullInputTruncationStrategy {
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
                    detail:
                        "dropped: an earlier item already exhausted the budget (hard cutoff, no bin-packing)"
                            .to_string(),
                });
                continue;
            }

            let remaining = budget.requested_tokens.saturating_sub(used_tokens);
            let item_tokens = count_tokens(&item.text);

            if item_tokens <= remaining {
                used_tokens += item_tokens;
                let byte_end = u32::try_from(item.text.len()).unwrap_or(u32::MAX);
                selection.push(ItemSelectionRecord {
                    source_id: item.source_id.clone(),
                    order_index: item.order_index,
                    status: SelectionStatus::Included,
                    included_byte_start: Some(0),
                    included_byte_end: Some(byte_end),
                    output_text: None,
                });
                trace.push(TraceStep {
                    step_index,
                    source_id: item.source_id.clone(),
                    action: "included_full".to_string(),
                    score: None,
                    detail: format!(
                        "included whole item ({item_tokens} tokens); {used_tokens}/{} tokens used",
                        budget.requested_tokens
                    ),
                });
                continue;
            }

            // Item does not fit fully: this is the one cutoff point. Truncate at the last safe
            // token boundary and drop everything from here on, regardless of what fits.
            cutoff_triggered = true;
            let (tokens_taken, byte_end) = fit_within_budget(&item.text, remaining);

            if tokens_taken > 0 {
                used_tokens += tokens_taken;
                selection.push(ItemSelectionRecord {
                    source_id: item.source_id.clone(),
                    order_index: item.order_index,
                    status: SelectionStatus::Partial,
                    included_byte_start: Some(0),
                    included_byte_end: Some(byte_end),
                    output_text: None,
                });
                trace.push(TraceStep {
                    step_index,
                    source_id: item.source_id.clone(),
                    action: "included_partial".to_string(),
                    score: None,
                    detail: format!(
                        "truncated to {tokens_taken}/{item_tokens} tokens (byte range [0, {byte_end})) to fit the remaining {remaining}-token budget; every later item is now dropped"
                    ),
                });
            } else {
                selection.push(dropped_record(item));
                trace.push(TraceStep {
                    step_index,
                    source_id: item.source_id.clone(),
                    action: "dropped_over_budget".to_string(),
                    score: None,
                    detail: format!(
                        "dropped ({item_tokens} tokens): zero tokens fit in the remaining {remaining}-token budget; every later item is now dropped"
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

/// A `Dropped` record with no retained byte range (per
/// [`crate::model::ItemSelectionRecord::included_byte_start`]'s documented `None`-for-`Dropped`
/// contract).
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

/// Finds the longest prefix of `text` whose token count is `<= remaining_tokens`, using the safe
/// UTF-8 boundary technique: tokenize `text`, accumulate tokens one at a time while the running
/// count stays `<= remaining_tokens`, and cut at the `byte_end` of the last token taken. Returns
/// `(tokens_taken, byte_end)`; `(0, 0)` if zero tokens fit (including when `text` is empty).
///
/// This can never split a UTF-8 codepoint since tokenizer spans always land on char boundaries.
fn fit_within_budget(text: &str, remaining_tokens: u32) -> (u32, u32) {
    let mut tokens_taken: u32 = 0;
    let mut byte_end: u32 = 0;
    for token in tokenize(text) {
        if tokens_taken + 1 > remaining_tokens {
            break;
        }
        tokens_taken += 1;
        byte_end = token.byte_end;
    }
    (tokens_taken, byte_end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ContextItem;

    fn item(source_id: &str, order_index: u32, text: &str) -> ContextItem {
        ContextItem {
            source_id: source_id.to_string(),
            order_index,
            section_label: "log_line".to_string(),
            text: text.to_string(),
        }
    }

    fn budget(requested_tokens: u32) -> Budget {
        Budget { requested_tokens }
    }

    #[test]
    fn id_matches_registry_constant() {
        assert_eq!(FullInputTruncationStrategy.id(), ID);
        assert_eq!(ID, "full-input-truncation@1");
    }

    #[test]
    fn includes_everything_when_budget_is_ample() {
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![item("a", 0, "one two"), item("b", 1, "three four five")],
        };
        let output = FullInputTruncationStrategy.select(&input, &budget(1000));

        assert_eq!(output.strategy_id, ID);
        assert_eq!(output.selection.len(), 2);
        assert!(output
            .selection
            .iter()
            .all(|r| r.status == SelectionStatus::Included));
        assert_eq!(output.trace.len(), 2);
        assert_eq!(output.budget.used_tokens, 2 + 3);
        assert_eq!(output.budget.remaining_tokens, 1000 - 5);
    }

    #[test]
    fn drops_a_contiguous_tail_once_budget_is_exhausted() {
        // "one" (1 tok) fits whole; "two three" (2 tok) only partially fits the last 1 token of
        // budget (-> Partial, cutoff triggered); "four" is then unconditionally dropped.
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![
                item("a", 0, "one"),
                item("b", 1, "two three"),
                item("c", 2, "four"),
            ],
        };
        let output = FullInputTruncationStrategy.select(&input, &budget(2));

        assert_eq!(output.selection[0].status, SelectionStatus::Included);
        assert_eq!(output.selection[1].status, SelectionStatus::Partial);
        assert_eq!(output.selection[2].status, SelectionStatus::Dropped);
        assert_eq!(output.budget.used_tokens, 2);
        assert_eq!(output.budget.remaining_tokens, 0);
        assert_eq!(output.trace[0].action, "included_full");
        assert_eq!(output.trace[1].action, "included_partial");
        assert_eq!(output.trace[2].action, "dropped_over_budget");
    }

    #[test]
    fn drops_entire_tail_when_the_cutoff_item_itself_fits_zero_tokens() {
        // budget=1: "one" (1 tok) fits whole, leaving remaining=0; "two three" (2 tok) then fits
        // zero tokens at the cutoff -> Dropped (not Partial); "four" is dropped after it too.
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![
                item("a", 0, "one"),
                item("b", 1, "two three"),
                item("c", 2, "four"),
            ],
        };
        let output = FullInputTruncationStrategy.select(&input, &budget(1));

        assert_eq!(output.selection[0].status, SelectionStatus::Included);
        assert_eq!(output.selection[1].status, SelectionStatus::Dropped);
        assert_eq!(output.selection[2].status, SelectionStatus::Dropped);
        assert_eq!(output.budget.used_tokens, 1);
        assert_eq!(output.trace[1].action, "dropped_over_budget");
        assert_eq!(output.trace[2].action, "dropped_over_budget");
    }

    #[test]
    fn zero_budget_drops_every_nonempty_item() {
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![item("a", 0, "one"), item("b", 1, "two")],
        };
        let output = FullInputTruncationStrategy.select(&input, &budget(0));
        assert!(output
            .selection
            .iter()
            .all(|r| r.status == SelectionStatus::Dropped));
        assert_eq!(output.budget.used_tokens, 0);
    }

    #[test]
    fn processing_order_is_by_order_index_then_source_id_tie_break() {
        // Deliberately construct `items` out of order_index order to prove sorting happens.
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![item("z", 1, "b"), item("a", 0, "a")],
        };
        let output = FullInputTruncationStrategy.select(&input, &budget(1000));
        let order: Vec<&str> = output
            .selection
            .iter()
            .map(|r| r.source_id.as_str())
            .collect();
        assert_eq!(
            order,
            vec!["a", "z"],
            "must process in order_index order regardless of input Vec order"
        );
    }

    #[test]
    fn partial_truncation_cuts_at_a_safe_token_boundary() {
        // "abcde" is a single 5-byte/1-token run; with only 3 tokens of budget already spent by
        // the previous item and a 4-token budget, remaining=1, but a single-token item ("abcde")
        // either wholly fits or wholly doesn't -- use a multi-token item to actually exercise a
        // partial cut.
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![item("a", 0, "one two three four five")],
        };
        // 5 tokens total; budget=3 means only "one two three" (3 tokens) fits.
        let output = FullInputTruncationStrategy.select(&input, &budget(3));
        assert_eq!(output.selection[0].status, SelectionStatus::Partial);
        let start = output.selection[0].included_byte_start.unwrap();
        let end = output.selection[0].included_byte_end.unwrap();
        assert_eq!(
            &input.items[0].text[start as usize..end as usize],
            "one two three"
        );
        assert_eq!(output.budget.used_tokens, 3);
        assert_eq!(output.trace[0].action, "included_partial");
    }

    #[test]
    fn zero_tokens_fit_marks_dropped_not_partial() {
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![item("a", 0, "hello"), item("b", 1, "world")],
        };
        // budget=0 leaves remaining=0 for the very first item -> zero tokens fit -> Dropped.
        let output = FullInputTruncationStrategy.select(&input, &budget(0));
        assert_eq!(output.selection[0].status, SelectionStatus::Dropped);
        assert_eq!(output.trace[0].action, "dropped_over_budget");
    }

    #[test]
    fn exact_boundary_budget_includes_exactly_the_first_k_items() {
        // "one" (1) + "two three" (2) = 3 tokens exactly; a budget of 3 must include both and
        // drop the rest, with nothing partial.
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![
                item("a", 0, "one"),
                item("b", 1, "two three"),
                item("c", 2, "four"),
            ],
        };
        let output = FullInputTruncationStrategy.select(&input, &budget(3));
        assert_eq!(output.selection[0].status, SelectionStatus::Included);
        assert_eq!(output.selection[1].status, SelectionStatus::Included);
        assert_eq!(output.selection[2].status, SelectionStatus::Dropped);
        assert_eq!(output.budget.used_tokens, 3);
        assert_eq!(output.budget.remaining_tokens, 0);
    }

    #[test]
    fn budget_never_exceeded_across_a_sweep_including_zero() {
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![
                item("a", 0, "one two three"),
                item("b", 1, "four five"),
                item("c", 2, "six seven eight nine"),
                item("d", 3, "ten"),
            ],
        };
        for requested in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 100] {
            let output = FullInputTruncationStrategy.select(&input, &budget(requested));
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
                item("c", 2, "gamma"),
                item("a", 0, "alpha"),
                item("b", 1, "beta"),
            ],
        };
        let output = FullInputTruncationStrategy.select(&input, &budget(1000));
        assert_eq!(output.selection.len(), 3);
        let order_indices: Vec<u32> = output.selection.iter().map(|r| r.order_index).collect();
        assert_eq!(order_indices, vec![0, 1, 2]);
    }

    #[test]
    fn determinism_two_calls_produce_byte_identical_output() {
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![
                item("a", 0, "one two three"),
                item("b", 1, "four five six seven"),
                item("c", 2, "eight"),
            ],
        };
        let first = FullInputTruncationStrategy.select(&input, &budget(5));
        let second = FullInputTruncationStrategy.select(&input, &budget(5));
        assert_eq!(first, second);
    }
}
