//! `retrieval-ranking@1`.
//!
//! The one strategy in the matrix that does real greedy bin-packing instead of a linear-scan hard
//! cutoff: every item is *attempted* in ranked order, and a miss does not stop the scan. This
//! means a smaller, lower-ranked item can end up included even after a larger, higher-ranked item
//! was skipped for not fitting -- a deliberate, correct contrast with the other four strategies.
//!
//! Algorithm (greedy bin-packing by integer lexical score):
//! 1. Tokenize `task_query` with the crate tokenizer; keep only its ASCII-alphanumeric tokens
//!    (i.e. exclude punctuation/symbol and non-ASCII tokens), lowercased byte-for-byte (ASCII
//!    case-folding only, never locale-aware), *preserving duplicates* -- this is the "query term
//!    list" (see [`ascii_alnum_lowercase_terms`]).
//! 2. For each item, build the same kind of lowercased ASCII-alphanumeric term list from its own
//!    `text`.
//! 3. Score each item as the symmetric bag-overlap count: for every occurrence of a term in the
//!    query term list (duplicates counted separately), add 1 for every occurrence of that exact
//!    term anywhere in the item's own term list (see [`bag_overlap_score`]). This is an integer,
//!    unnormalized count -- longer items are not penalized or boosted for length.
//! 4. Rank all items by score descending; tie-break `order_index` ascending; final tie-break
//!    `source_id` ascending (a fully specified total order, documented here even though
//!    `order_index` is already unique per scenario in practice, so the final tie-break never
//!    actually fires). Each item's trace step records `score: Some(score)`, regardless of whether
//!    it ends up included.
//! 5. Walk the ranked list top to bottom, attempting *every* item (never stopping at the first
//!    miss): if the item's verbatim `text` fits fully in the currently-remaining budget, include
//!    it whole (`"included_full"`) and deduct its tokens from the remaining budget; if it does
//!    not fit, mark it `Dropped` (`"dropped_below_budget"`) and move on to the next ranked item
//!    without truncating and without cutting off the rest. This strategy never produces
//!    `Partial`.
//!
//! `selection` is emitted sorted by `order_index` ascending regardless of the score-ranked
//! processing order (the required output contract): the *trace* reflects ranked processing
//! order, the *selection* always reflects original document order.

use crate::model::{
    Budget, BudgetUsage, ItemSelectionRecord, SelectionOutput, SelectionStatus, StrategyInput,
    TraceStep,
};
use crate::tokenizer::{count_tokens, tokenize};

use super::SelectionStrategy;

/// Exact registry id for this strategy.
pub const ID: &str = "retrieval-ranking@1";

pub struct RetrievalRankingStrategy;

impl SelectionStrategy for RetrievalRankingStrategy {
    fn id(&self) -> &'static str {
        ID
    }

    fn select(&self, input: &StrategyInput, budget: &Budget) -> SelectionOutput {
        let query_terms = ascii_alnum_lowercase_terms(&input.task_query);
        let scores: Vec<u32> = input
            .items
            .iter()
            .map(|item| bag_overlap_score(&query_terms, &ascii_alnum_lowercase_terms(&item.text)))
            .collect();

        let mut ranked: Vec<usize> = (0..input.items.len()).collect();
        ranked.sort_by(|&a, &b| {
            // Score descending, then order_index ascending, then source_id ascending: a fully
            // specified total order (see module doc comment).
            scores[b]
                .cmp(&scores[a])
                .then_with(|| input.items[a].order_index.cmp(&input.items[b].order_index))
                .then_with(|| input.items[a].source_id.cmp(&input.items[b].source_id))
        });

        let mut selection = Vec::with_capacity(input.items.len());
        let mut trace = Vec::with_capacity(input.items.len());
        let mut used_tokens: u32 = 0;

        for (step_index, &idx) in ranked.iter().enumerate() {
            let item = &input.items[idx];
            let score = scores[idx];
            let step_index = step_index as u32;
            let item_tokens = count_tokens(&item.text);
            let remaining = budget.requested_tokens.saturating_sub(used_tokens);

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
                    score: Some(score),
                    detail: format!(
                        "score {score}: included whole item ({item_tokens} tokens); {used_tokens}/{} tokens used",
                        budget.requested_tokens
                    ),
                });
            } else {
                selection.push(ItemSelectionRecord {
                    source_id: item.source_id.clone(),
                    order_index: item.order_index,
                    status: SelectionStatus::Dropped,
                    included_byte_start: None,
                    included_byte_end: None,
                    output_text: None,
                });
                trace.push(TraceStep {
                    step_index,
                    source_id: item.source_id.clone(),
                    action: "dropped_below_budget".to_string(),
                    score: Some(score),
                    detail: format!(
                        "score {score}: skipped ({item_tokens} tokens); would exceed the remaining {remaining}-token budget, continuing to try lower-ranked items"
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

/// Tokenizes `text` and returns the lowercased (ASCII-only, byte-for-byte) form of every
/// ASCII-alphanumeric token, in order, preserving duplicates. Punctuation/symbol tokens and
/// non-ASCII scalar tokens are excluded entirely -- they never participate in lexical scoring.
fn ascii_alnum_lowercase_terms(text: &str) -> Vec<String> {
    tokenize(text)
        .into_iter()
        .filter_map(|token| {
            let slice = &text[token.byte_start as usize..token.byte_end as usize];
            if slice.chars().all(|c| c.is_ascii_alphanumeric()) {
                Some(slice.to_ascii_lowercase())
            } else {
                None
            }
        })
        .collect()
}

/// The symmetric bag-overlap count between `query_terms` and `item_terms`: for every entry in
/// `query_terms` (duplicates counted separately), add the number of times that exact string
/// appears anywhere in `item_terms`. Integer-only, no length normalization; deliberately simple
/// and fully deterministic (no `HashMap` -- a direct nested scan over the two term lists).
fn bag_overlap_score(query_terms: &[String], item_terms: &[String]) -> u32 {
    let mut score: u32 = 0;
    for query_term in query_terms {
        let occurrences = item_terms.iter().filter(|term| *term == query_term).count();
        score += occurrences as u32;
    }
    score
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ContextItem;

    fn item(source_id: &str, order_index: u32, text: &str) -> ContextItem {
        ContextItem {
            source_id: source_id.to_string(),
            order_index,
            section_label: "chat_message".to_string(),
            text: text.to_string(),
        }
    }

    fn budget(requested_tokens: u32) -> Budget {
        Budget { requested_tokens }
    }

    #[test]
    fn id_matches_registry_constant() {
        assert_eq!(RetrievalRankingStrategy.id(), ID);
        assert_eq!(ID, "retrieval-ranking@1");
    }

    #[test]
    fn includes_everything_when_budget_is_ample_and_scores_are_recorded() {
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![item("a", 0, "one two"), item("b", 1, "three four five")],
        };
        let output = RetrievalRankingStrategy.select(&input, &budget(1000));
        assert!(output
            .selection
            .iter()
            .all(|r| r.status == SelectionStatus::Included));
        assert!(
            output.trace.iter().all(|t| t.score.is_some()),
            "retrieval-ranking must always record an integer score"
        );
        assert_eq!(output.budget.used_tokens, 5);
    }

    #[test]
    fn smaller_lower_ranked_item_is_included_after_larger_higher_ranked_item_is_skipped() {
        // "big-high-score" scores 10 (query "alpha" matched 10 times) but needs 10 tokens.
        // "small-low-score" scores 0 but only needs 1 token. Budget=5 cannot fit the top-ranked
        // item, but bin-packing must still include the smaller, lower-ranked one afterward.
        let input = StrategyInput {
            task_query: "alpha".to_string(),
            items: vec![
                item(
                    "big-high-score",
                    0,
                    "alpha alpha alpha alpha alpha alpha alpha alpha alpha alpha",
                ),
                item("small-low-score", 1, "beta"),
            ],
        };
        let output = RetrievalRankingStrategy.select(&input, &budget(5));

        let big = output
            .selection
            .iter()
            .find(|r| r.source_id == "big-high-score")
            .unwrap();
        let small = output
            .selection
            .iter()
            .find(|r| r.source_id == "small-low-score")
            .unwrap();
        assert_eq!(
            big.status,
            SelectionStatus::Dropped,
            "the higher-ranked item must be skipped, not truncated"
        );
        assert_eq!(
            small.status,
            SelectionStatus::Included,
            "a smaller, lower-ranked item must still be picked up by bin-packing"
        );
        assert_eq!(output.budget.used_tokens, 1);

        let big_trace = output
            .trace
            .iter()
            .find(|t| t.source_id == "big-high-score")
            .unwrap();
        let small_trace = output
            .trace
            .iter()
            .find(|t| t.source_id == "small-low-score")
            .unwrap();
        assert_eq!(big_trace.score, Some(10));
        assert_eq!(small_trace.score, Some(0));
        assert_eq!(big_trace.action, "dropped_below_budget");
        assert_eq!(small_trace.action, "included_full");
        // Ranked (trace) order must try the higher-scoring item first even though it loses.
        assert_eq!(output.trace[0].source_id, "big-high-score");
        assert_eq!(output.trace[1].source_id, "small-low-score");
    }

    #[test]
    fn score_is_a_symmetric_bag_overlap_count_with_repeats_on_both_sides() {
        // Query "cat cat" (2 occurrences); item has "cat" 3 times -> score = 2 * 3 = 6.
        let input = StrategyInput {
            task_query: "cat cat".to_string(),
            items: vec![item("a", 0, "cat dog cat cat")],
        };
        let output = RetrievalRankingStrategy.select(&input, &budget(1000));
        assert_eq!(output.trace[0].score, Some(6));
    }

    #[test]
    fn scoring_is_case_insensitive_via_ascii_lowercasing_only() {
        let input = StrategyInput {
            task_query: "Cat".to_string(),
            items: vec![item("a", 0, "cat CAT Cat cAt")],
        };
        let output = RetrievalRankingStrategy.select(&input, &budget(1000));
        assert_eq!(output.trace[0].score, Some(4));
    }

    #[test]
    fn punctuation_tokens_never_contribute_to_score() {
        // Query tokenizes to ["cat", "!"]; the "!" token must never match anything.
        let input = StrategyInput {
            task_query: "cat!".to_string(),
            items: vec![item("a", 0, "! ! ! cat")],
        };
        let output = RetrievalRankingStrategy.select(&input, &budget(1000));
        assert_eq!(
            output.trace[0].score,
            Some(1),
            "only the 'cat' term should match; '!' tokens are excluded from scoring"
        );
    }

    #[test]
    fn never_produces_partial_status() {
        let input = StrategyInput {
            task_query: "alpha".to_string(),
            items: vec![item("a", 0, "alpha alpha alpha alpha alpha alpha")],
        };
        for requested in 0..10 {
            let output = RetrievalRankingStrategy.select(&input, &budget(requested));
            assert!(
                output
                    .selection
                    .iter()
                    .all(|r| r.status != SelectionStatus::Partial),
                "retrieval-ranking must never produce Partial (budget={requested})"
            );
        }
    }

    #[test]
    fn ties_break_by_order_index_ascending_then_source_id() {
        // Both items score identically against "cat"; order_index must decide ranked order.
        let input = StrategyInput {
            task_query: "cat".to_string(),
            items: vec![item("b", 5, "cat"), item("a", 2, "cat")],
        };
        let output = RetrievalRankingStrategy.select(&input, &budget(1000));
        assert_eq!(
            output.trace[0].source_id, "a",
            "lower order_index ranks first on a score tie"
        );
        assert_eq!(output.trace[1].source_id, "b");
    }

    #[test]
    fn zero_budget_drops_every_item_with_below_budget_action() {
        let input = StrategyInput {
            task_query: "q".to_string(),
            items: vec![item("a", 0, "one"), item("b", 1, "two")],
        };
        let output = RetrievalRankingStrategy.select(&input, &budget(0));
        assert!(output
            .selection
            .iter()
            .all(|r| r.status == SelectionStatus::Dropped));
        assert!(output
            .trace
            .iter()
            .all(|t| t.action == "dropped_below_budget"));
        assert_eq!(output.budget.used_tokens, 0);
    }

    #[test]
    fn exact_boundary_budget_includes_exactly_the_items_that_sum_to_it() {
        // No query overlap at all -> every item scores 0 -> ranked purely by order_index.
        let input = StrategyInput {
            task_query: "nomatch".to_string(),
            items: vec![
                item("a", 0, "one"),
                item("b", 1, "two three"),
                item("c", 2, "four"),
            ],
        };
        // "one" (1) + "two three" (2) = 3 tokens exactly.
        let output = RetrievalRankingStrategy.select(&input, &budget(3));
        let by_id = |id: &str| {
            output
                .selection
                .iter()
                .find(|r| r.source_id == id)
                .unwrap()
                .status
        };
        assert_eq!(by_id("a"), SelectionStatus::Included);
        assert_eq!(by_id("b"), SelectionStatus::Included);
        assert_eq!(by_id("c"), SelectionStatus::Dropped);
        assert_eq!(output.budget.used_tokens, 3);
        assert_eq!(output.budget.remaining_tokens, 0);
    }

    #[test]
    fn budget_never_exceeded_across_a_sweep_including_zero() {
        let input = StrategyInput {
            task_query: "alpha beta".to_string(),
            items: vec![
                item("a", 0, "alpha one two three"),
                item("b", 1, "beta four five"),
                item("c", 2, "six seven eight nine"),
                item("d", 3, "ten alpha beta"),
            ],
        };
        for requested in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 100] {
            let output = RetrievalRankingStrategy.select(&input, &budget(requested));
            assert!(
                output.budget.used_tokens <= output.budget.requested_tokens,
                "used_tokens must never exceed requested_tokens at budget={requested}"
            );
            assert_eq!(output.selection.len(), input.items.len());
        }
    }

    #[test]
    fn selection_has_one_record_per_item_and_is_sorted_by_order_index_even_though_trace_is_ranked()
    {
        let input = StrategyInput {
            task_query: "gamma".to_string(),
            items: vec![
                item("c", 2, "gamma gamma gamma"),
                item("a", 0, "alpha"),
                item("b", 1, "beta"),
            ],
        };
        let output = RetrievalRankingStrategy.select(&input, &budget(1000));
        assert_eq!(output.selection.len(), 3);
        let order_indices: Vec<u32> = output.selection.iter().map(|r| r.order_index).collect();
        assert_eq!(order_indices, vec![0, 1, 2]);
        // Trace (ranked) order must put the highest-scoring item ("c", order_index 2) first.
        assert_eq!(output.trace[0].source_id, "c");
    }

    #[test]
    fn determinism_two_calls_produce_byte_identical_output() {
        let input = StrategyInput {
            task_query: "alpha beta".to_string(),
            items: vec![
                item("a", 0, "alpha one two three"),
                item("b", 1, "beta four five six"),
                item("c", 2, "seven eight"),
            ],
        };
        let first = RetrievalRankingStrategy.select(&input, &budget(5));
        let second = RetrievalRankingStrategy.select(&input, &budget(5));
        assert_eq!(first, second);
    }
}
