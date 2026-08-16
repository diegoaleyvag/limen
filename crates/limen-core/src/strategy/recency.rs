//! `recency@1`.
//!
//! Prioritizes the *most recent* items -- highest `order_index` -- when the budget cannot fit
//! everything, dropping from the oldest end first. `order_index` is the only recency signal used:
//! this strategy never parses dates/timestamps out of `text` (see
//! [`crate::model::ContextItem::order_index`]'s doc comment).
//!
//! Algorithm (linear scan, hard cutoff -- identical mechanics to
//! [`super::full_input_truncation`], but walked backwards):
//! 1. Process items in descending `(order_index, source_id)` order: `order_index` descending
//!    primary key (newest first), `source_id` ascending as the documented tie-break (this keeps
//!    the comparator a true total order even though `order_index` is already unique per
//!    scenario, so the tie-break in practice never fires).
//! 2. For each item, while no earlier (i.e. newer) item has failed to fit: if the item's verbatim
//!    `text` fits entirely within the remaining budget, include it whole (`"included_full"`).
//!    Otherwise, truncate at the last safe token boundary (see
//!    [`super::full_input_truncation::fit_within_budget`]'s technique, reimplemented locally
//!    below): if at least one token fits, `Partial` (`"included_partial"`); if zero tokens fit,
//!    `Dropped`. This is the one cutoff point.
//! 3. Every item at or after the cutoff (i.e. every item *older* than the cutoff item) is
//!    unconditionally `Dropped` with action `"dropped_too_old"` -- distinct from
//!    `full-input-truncation@1`'s generic `"dropped_over_budget"`, since here the reason really
//!    is "this evidence is older than what the budget could keep", which is the whole point of
//!    the strategy. The item that triggers the cutoff by fitting zero tokens also uses
//!    `"dropped_too_old"` (there is nothing else to call it: it is simultaneously the oldest
//!    surviving candidate and the first one that didn't fit).
//!
//! `selection` is emitted sorted by `order_index` ascending regardless of the newest-first
//! processing order (the required output contract): the *trace* reflects processing order
//! (newest to oldest), the *selection* always reflects original document order.

use crate::model::{
    Budget, BudgetUsage, ItemSelectionRecord, SelectionOutput, SelectionStatus, StrategyInput,
    TraceStep,
};
use crate::tokenizer::{count_tokens, tokenize};

use super::SelectionStrategy;

/// Exact registry id for this strategy.
pub const ID: &str = "recency@1";

pub struct RecencyStrategy;

impl SelectionStrategy for RecencyStrategy {
    fn id(&self) -> &'static str {
        ID
    }

    fn select(&self, input: &StrategyInput, budget: &Budget) -> SelectionOutput {
        let mut order: Vec<usize> = (0..input.items.len()).collect();
        order.sort_by(|&a, &b| {
            let item_a = &input.items[a];
            let item_b = &input.items[b];
            item_b
                .order_index
                .cmp(&item_a.order_index) // descending: newest (highest order_index) first
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
                    action: "dropped_too_old".to_string(),
                    score: None,
                    detail:
                        "dropped: older than the newest items the budget could keep (hard cutoff)"
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
            // token boundary; everything older than this item is now "too old" for the budget.
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
                        "truncated to {tokens_taken}/{item_tokens} tokens (byte range [0, {byte_end})) to fit the remaining {remaining}-token budget; every older item is now dropped as too old"
                    ),
                });
            } else {
                selection.push(dropped_record(item));
                trace.push(TraceStep {
                    step_index,
                    source_id: item.source_id.clone(),
                    action: "dropped_too_old".to_string(),
                    score: None,
                    detail: format!(
                        "dropped ({item_tokens} tokens): zero tokens fit in the remaining {remaining}-token budget; every older item is now dropped too"
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
/// (Identical technique to `full_input_truncation::fit_within_budget`; duplicated locally per
/// this phase's constraint that only these five strategy files may be edited.)
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
        assert_eq!(RecencyStrategy.id(), ID);
        assert_eq!(ID, "recency@1");
    }

    #[test]
    fn includes_everything_when_budget_is_ample() {
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![item("a", 0, "one two"), item("b", 1, "three four five")],
        };
        let output = RecencyStrategy.select(&input, &budget(1000));
        assert!(output
            .selection
            .iter()
            .all(|r| r.status == SelectionStatus::Included));
        assert_eq!(output.budget.used_tokens, 5);
    }

    #[test]
    fn tight_budget_keeps_the_newest_items_and_drops_the_oldest() {
        // Four single-token items, order_index 0..3; budget=2 can only keep the two *newest*
        // (order_index 3 and 2), dropping the two oldest (0 and 1) as too old -- even though they
        // appear earlier in the input Vec / original document order.
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![
                item("oldest", 0, "aaa"),
                item("old", 1, "bbb"),
                item("newer", 2, "ccc"),
                item("newest", 3, "ddd"),
            ],
        };
        let output = RecencyStrategy.select(&input, &budget(2));

        let by_id = |id: &str| {
            output
                .selection
                .iter()
                .find(|r| r.source_id == id)
                .unwrap()
                .status
        };
        assert_eq!(by_id("newest"), SelectionStatus::Included);
        assert_eq!(by_id("newer"), SelectionStatus::Included);
        assert_eq!(by_id("old"), SelectionStatus::Dropped);
        assert_eq!(by_id("oldest"), SelectionStatus::Dropped);
        assert_eq!(output.budget.used_tokens, 2);

        // Trace must reflect newest-to-oldest *processing* order.
        let trace_order: Vec<&str> = output.trace.iter().map(|t| t.source_id.as_str()).collect();
        assert_eq!(trace_order, vec!["newest", "newer", "old", "oldest"]);
        assert_eq!(output.trace[2].action, "dropped_too_old");
        assert_eq!(output.trace[3].action, "dropped_too_old");

        // But `selection` must still be sorted by ascending order_index.
        let selection_order: Vec<u32> = output.selection.iter().map(|r| r.order_index).collect();
        assert_eq!(selection_order, vec![0, 1, 2, 3]);
    }

    #[test]
    fn partial_truncation_of_the_cutoff_item_uses_included_partial_action() {
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![item("a", 0, "one"), item("b", 1, "two three four five")],
        };
        // Newest first: "b" (order 1, 4 tokens) is tried first; budget=2 only fits "two three".
        let output = RecencyStrategy.select(&input, &budget(2));
        let b = output
            .selection
            .iter()
            .find(|r| r.source_id == "b")
            .unwrap();
        assert_eq!(b.status, SelectionStatus::Partial);
        let start = b.included_byte_start.unwrap();
        let end = b.included_byte_end.unwrap();
        assert_eq!(
            &input.items[1].text[start as usize..end as usize],
            "two three"
        );

        let a = output
            .selection
            .iter()
            .find(|r| r.source_id == "a")
            .unwrap();
        assert_eq!(
            a.status,
            SelectionStatus::Dropped,
            "older item dropped after cutoff"
        );

        let b_trace = output.trace.iter().find(|t| t.source_id == "b").unwrap();
        assert_eq!(b_trace.action, "included_partial");
        let a_trace = output.trace.iter().find(|t| t.source_id == "a").unwrap();
        assert_eq!(a_trace.action, "dropped_too_old");
    }

    #[test]
    fn exact_boundary_budget_keeps_exactly_the_newest_k_items() {
        // Newest-first tokens: "ddd" (1) + "ccc" (1) = 2 tokens exactly.
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![
                item("a", 0, "aaa"),
                item("b", 1, "bbb"),
                item("c", 2, "ccc"),
                item("d", 3, "ddd"),
            ],
        };
        let output = RecencyStrategy.select(&input, &budget(2));
        assert_eq!(output.budget.used_tokens, 2);
        assert_eq!(output.budget.remaining_tokens, 0);
        let statuses: Vec<SelectionStatus> = output.selection.iter().map(|r| r.status).collect();
        assert_eq!(
            statuses,
            vec![
                SelectionStatus::Dropped,
                SelectionStatus::Dropped,
                SelectionStatus::Included,
                SelectionStatus::Included,
            ]
        );
    }

    #[test]
    fn zero_budget_drops_every_item() {
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![item("a", 0, "one"), item("b", 1, "two")],
        };
        let output = RecencyStrategy.select(&input, &budget(0));
        assert!(output
            .selection
            .iter()
            .all(|r| r.status == SelectionStatus::Dropped));
        assert!(output.trace.iter().all(|t| t.action == "dropped_too_old"));
        assert_eq!(output.budget.used_tokens, 0);
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
            let output = RecencyStrategy.select(&input, &budget(requested));
            assert!(
                output.budget.used_tokens <= output.budget.requested_tokens,
                "used_tokens must never exceed requested_tokens at budget={requested}"
            );
            assert_eq!(output.selection.len(), input.items.len());
        }
    }

    #[test]
    fn selection_has_one_record_per_item_and_is_sorted_by_order_index_even_though_trace_is_reversed(
    ) {
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![
                item("c", 2, "gamma"),
                item("a", 0, "alpha"),
                item("b", 1, "beta"),
            ],
        };
        let output = RecencyStrategy.select(&input, &budget(1000));
        assert_eq!(output.selection.len(), 3);
        let order_indices: Vec<u32> = output.selection.iter().map(|r| r.order_index).collect();
        assert_eq!(order_indices, vec![0, 1, 2]);

        // Processing/trace order is newest (order_index 2) to oldest (order_index 0).
        let trace_order_indices: Vec<u32> = output
            .trace
            .iter()
            .map(|t| {
                input
                    .items
                    .iter()
                    .find(|i| i.source_id == t.source_id)
                    .unwrap()
                    .order_index
            })
            .collect();
        assert_eq!(trace_order_indices, vec![2, 1, 0]);
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
        let first = RecencyStrategy.select(&input, &budget(5));
        let second = RecencyStrategy.select(&input, &budget(5));
        assert_eq!(first, second);
    }
}
