//! Shared logic for the golden-fixture matrix (3 real scenarios x 5 registered strategies x 5
//! budget tiers = 75 trials), used identically by the `generate_golden_fixtures` example (which
//! writes the checked-in fixtures under `tests/golden/`) and this crate's own golden-fixture
//! regression/schema-validation tests (which assert the checked-in files match freshly
//! regenerated output). Routing both through the same [`golden_cases`]/[`budget_tiers_for_scenario`]
//! functions means the generator and the regression guard can never silently drift apart -- the
//! exact same tier formula always produces the exact same 75 `(scenario_id, strategy_id,
//! requested_tokens)` triples on both sides.
//!
//! # Budget tiers
//!
//! Each embedded scenario gets its own 5 tiers, all derived from that scenario's own per-item
//! token counts (via the crate's one bundled tokenizer, on raw item text in `order_index` order --
//! this is a fixture-generation convenience, not a strategy decision, so using raw text here
//! rather than a strategy's own emitted-output token counts is deliberate and documented):
//!
//! - `zero`: `requested_tokens: 0` -- the impossible budget.
//! - `tight`: one quarter of the scenario's total raw token count (at least 1) -- small enough
//!   that most strategies will drop or truncate some items, but not so small that everything is
//!   dropped.
//! - `exact_boundary`: the exact cumulative raw token count of the first half of the scenario's
//!   items (in `order_index` order) -- lands precisely on an item boundary, exercising
//!   off-by-one correctness in every strategy's own fit/cutoff arithmetic.
//! - `representative`: half of the scenario's total raw token count -- a middling, realistic
//!   budget.
//! - `ample`: the scenario's total raw token count plus a fixed 1000-token margin -- large enough
//!   that every strategy's own (possibly larger, e.g. templated/wrapped) output still fits
//!   entirely.

use crate::catalog::{all_scenario_ids, get_scenario};
use crate::model::ScenarioManifest;
use crate::strategy::list_strategy_ids;
use crate::tokenizer::count_tokens;

/// Fixed, documented order of budget tier names, also the iteration order [`golden_cases`]
/// produces.
pub const BUDGET_TIER_NAMES: [&str; 5] =
    ["zero", "tight", "exact_boundary", "representative", "ample"];

/// A fixed margin added to a scenario's total raw token count to build its `ample` tier, safely
/// larger than any per-item template/wrapper overhead a transform strategy could add.
const AMPLE_MARGIN_TOKENS: u32 = 1000;

/// One budget tier for one scenario: a documented `name` (one of [`BUDGET_TIER_NAMES`]) paired
/// with the concrete `requested_tokens` value computed for that scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudgetTier {
    pub name: &'static str,
    pub requested_tokens: u32,
}

/// Computes the 5 budget tiers for `manifest`, in [`BUDGET_TIER_NAMES`] order. See the module
/// doc comment for the exact formula behind each tier.
pub fn budget_tiers_for_scenario(manifest: &ScenarioManifest) -> [BudgetTier; 5] {
    let mut items_by_order: Vec<&crate::model::ContextItem> = manifest.items.iter().collect();
    items_by_order.sort_by_key(|item| item.order_index);

    let per_item_tokens: Vec<u32> = items_by_order
        .iter()
        .map(|item| count_tokens(&item.text))
        .collect();
    let total_tokens: u32 = per_item_tokens.iter().sum();

    let prefix_len = (items_by_order.len() / 2).clamp(1, items_by_order.len().max(1));
    let exact_boundary_tokens: u32 = per_item_tokens
        .get(..prefix_len)
        .unwrap_or(&[])
        .iter()
        .sum();

    let tight_tokens = (total_tokens / 4).max(1);
    let representative_tokens = total_tokens / 2;
    let ample_tokens = total_tokens.saturating_add(AMPLE_MARGIN_TOKENS);

    [
        BudgetTier {
            name: "zero",
            requested_tokens: 0,
        },
        BudgetTier {
            name: "tight",
            requested_tokens: tight_tokens,
        },
        BudgetTier {
            name: "exact_boundary",
            requested_tokens: exact_boundary_tokens,
        },
        BudgetTier {
            name: "representative",
            requested_tokens: representative_tokens,
        },
        BudgetTier {
            name: "ample",
            requested_tokens: ample_tokens,
        },
    ]
}

/// One cell of the golden-fixture matrix: a fully resolved `(scenario_id, strategy_id,
/// requested_tokens)` triple, plus the tier name that `requested_tokens` came from (for
/// filenames/reporting).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoldenCase {
    pub scenario_id: &'static str,
    pub strategy_id: &'static str,
    pub tier_name: &'static str,
    pub requested_tokens: u32,
}

/// The full golden-fixture matrix: for every embedded scenario (in
/// [`crate::catalog::SCENARIO_IDS`] order), its 5 budget tiers (in [`BUDGET_TIER_NAMES`] order),
/// crossed with every registered strategy (in [`crate::strategy::STRATEGY_IDS`] order) --
/// 3 x 5 x 5 = 75 cases, in a fully deterministic, fixed order.
pub fn golden_cases() -> Vec<GoldenCase> {
    let mut cases = Vec::with_capacity(75);
    for &scenario_id in all_scenario_ids() {
        let manifest = get_scenario(scenario_id).expect("every catalog id must resolve");
        let tiers = budget_tiers_for_scenario(&manifest);
        for tier in &tiers {
            for &strategy_id in list_strategy_ids() {
                cases.push(GoldenCase {
                    scenario_id,
                    strategy_id,
                    tier_name: tier.name,
                    requested_tokens: tier.requested_tokens,
                });
            }
        }
    }
    cases
}

/// The checked-in filename (relative to `tests/golden/`) for one [`GoldenCase`]: stable, human-
/// readable, and unique across the whole matrix (scenario/strategy/tier triples are never
/// repeated by [`golden_cases`]).
pub fn golden_case_filename(case: &GoldenCase) -> String {
    format!(
        "{}__{}__{}.json",
        case.scenario_id, case.strategy_id, case.tier_name
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn golden_cases_has_exactly_75_entries() {
        assert_eq!(golden_cases().len(), 75);
    }

    #[test]
    fn golden_cases_covers_every_scenario_strategy_tier_combination_exactly_once() {
        use std::collections::BTreeSet;
        let cases = golden_cases();
        let unique: BTreeSet<(&str, &str, &str)> = cases
            .iter()
            .map(|c| (c.scenario_id, c.strategy_id, c.tier_name))
            .collect();
        assert_eq!(unique.len(), 75, "expected every combination exactly once");
        assert_eq!(cases.len(), unique.len());
    }

    #[test]
    fn golden_case_filenames_are_all_unique() {
        use std::collections::BTreeSet;
        let filenames: BTreeSet<String> = golden_cases().iter().map(golden_case_filename).collect();
        assert_eq!(filenames.len(), 75);
    }

    #[test]
    fn budget_tiers_are_monotonically_sensible_for_every_scenario() {
        for &scenario_id in all_scenario_ids() {
            let manifest = get_scenario(scenario_id).unwrap();
            let tiers = budget_tiers_for_scenario(&manifest);
            let by_name = |name: &str| {
                tiers
                    .iter()
                    .find(|t| t.name == name)
                    .unwrap()
                    .requested_tokens
            };

            assert_eq!(by_name("zero"), 0);
            assert!(by_name("tight") >= 1);
            assert!(by_name("tight") <= by_name("representative"));
            assert!(
                by_name("representative") <= by_name("exact_boundary")
                    || by_name("exact_boundary") <= by_name("ample")
            );
            assert!(by_name("ample") > by_name("representative"));
        }
    }

    #[test]
    fn exact_boundary_tier_equals_a_real_prefix_cumulative_token_count() {
        for &scenario_id in all_scenario_ids() {
            let manifest = get_scenario(scenario_id).unwrap();
            let mut items_by_order: Vec<&crate::model::ContextItem> =
                manifest.items.iter().collect();
            items_by_order.sort_by_key(|item| item.order_index);
            let per_item_tokens: Vec<u32> = items_by_order
                .iter()
                .map(|item| count_tokens(&item.text))
                .collect();

            let tiers = budget_tiers_for_scenario(&manifest);
            let exact_boundary = tiers
                .iter()
                .find(|t| t.name == "exact_boundary")
                .unwrap()
                .requested_tokens;

            let mut cumulative: Vec<u32> = Vec::with_capacity(per_item_tokens.len());
            let mut running = 0u32;
            for tokens in &per_item_tokens {
                running += tokens;
                cumulative.push(running);
            }
            assert!(
                cumulative.contains(&exact_boundary),
                "exact_boundary {exact_boundary} for '{scenario_id}' must equal some real prefix's cumulative token count: {cumulative:?}"
            );
        }
    }
}
