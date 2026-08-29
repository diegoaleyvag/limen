//! The embedded scenario catalog: the single source of truth for the three scenarios Limen
//! ships, baked into the compiled binary (native or WASM) via [`include_str!`] rather than read
//! from the filesystem at run time. This is what lets a purely static/offline build (no server,
//! no `fetch`) always have real scenario content available.
//!
//! Mirrors [`crate::strategy`]'s registry shape deliberately: a fixed, static id list plus exact-
//! match resolution (a `match`, not a `HashMap`), so lookup order and failure behavior are
//! identical on every platform and never depend on hash-iteration order. Parsing happens on
//! demand in [`get_scenario`] rather than once at startup into a cached map -- these are three
//! small files, so there is no meaningful cost to re-parsing per call, and skipping a cache
//! avoids introducing any lazily-initialized global state.

use serde::Serialize;

use crate::model::ScenarioManifest;

const INCIDENT_INVESTIGATION: &str =
    include_str!("../../../scenarios/v1/incident-investigation.json");
const PRODUCT_COMPARISON: &str = include_str!("../../../scenarios/v1/product-comparison.json");
const REQUIREMENTS_ARCHITECTURE_REVIEW: &str =
    include_str!("../../../scenarios/v1/requirements-architecture-review.json");

/// The three embedded scenario ids, in the fixed order they are documented and shipped (also the
/// order [`list_scenario_summaries`] returns them in).
pub const SCENARIO_IDS: [&str; 3] = [
    "incident-investigation",
    "product-comparison",
    "requirements-architecture-review",
];

/// Returns the embedded scenario ids, in fixed order (see [`SCENARIO_IDS`]).
pub fn all_scenario_ids() -> &'static [&'static str] {
    &SCENARIO_IDS
}

/// Resolves a scenario id to its parsed, embedded [`ScenarioManifest`] via an exact match against
/// a fixed, static table (a `match` expression, not a `HashMap`). Returns `None` for any id not
/// in [`SCENARIO_IDS`].
///
/// Parsing the matched embedded JSON text can only fail if the compiled-in file itself is
/// malformed, which every scenario's native test coverage (`scenario_manifest_parses_and_validates_cleanly`
/// below, plus `crates/limen-core/tests/scenario_validation.rs`) proves is never true for the
/// three checked-in files; rather than `.expect()`-panicking on that practically-unreachable
/// path, a parse failure collapses to `None` (the same "not available" signal an unknown id
/// produces) so this function can never panic regardless.
pub fn get_scenario(id: &str) -> Option<ScenarioManifest> {
    let raw = match id {
        "incident-investigation" => INCIDENT_INVESTIGATION,
        "product-comparison" => PRODUCT_COMPARISON,
        "requirements-architecture-review" => REQUIREMENTS_ARCHITECTURE_REVIEW,
        _ => return None,
    };
    serde_json::from_str(raw).ok()
}

/// A short, display-oriented projection of a [`ScenarioManifest`] for a scenario picker UI --
/// everything needed to populate a `<select>` and preview the task query before committing to
/// loading the full manifest via [`get_scenario`]/[`crate::validate::validate_manifest`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ScenarioSummary {
    pub scenario_id: String,
    pub scenario_version: String,
    pub title: String,
    pub task_query: String,
    pub item_count: u32,
}

/// Builds a [`ScenarioSummary`] for every embedded scenario, in [`SCENARIO_IDS`] order. Silently
/// skips any id that fails to resolve via [`get_scenario`] -- practically unreachable (see that
/// function's doc comment) but keeps this function infallible rather than panicking.
pub fn list_scenario_summaries() -> Vec<ScenarioSummary> {
    all_scenario_ids()
        .iter()
        .filter_map(|&id| get_scenario(id))
        .map(|manifest| ScenarioSummary {
            scenario_id: manifest.scenario_id,
            scenario_version: manifest.scenario_version,
            title: manifest.title,
            task_query: manifest.task_query,
            item_count: manifest.items.len() as u32,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validate::validate_manifest;

    #[test]
    fn all_scenario_ids_matches_fixed_documented_order() {
        assert_eq!(
            all_scenario_ids(),
            &[
                "incident-investigation",
                "product-comparison",
                "requirements-architecture-review",
            ]
        );
    }

    #[test]
    fn every_embedded_scenario_parses_and_round_trips_its_own_id() {
        for &id in all_scenario_ids() {
            let manifest =
                get_scenario(id).unwrap_or_else(|| panic!("expected {id} to resolve to Some"));
            assert_eq!(
                manifest.scenario_id, id,
                "resolved manifest's own scenario_id must match the lookup key"
            );
        }
    }

    #[test]
    fn every_embedded_scenario_validates_with_zero_errors() {
        for &id in all_scenario_ids() {
            let manifest = get_scenario(id).unwrap();
            let errors = validate_manifest(&manifest);
            assert!(
                errors.is_empty(),
                "expected zero validate_manifest errors for '{id}', got:\n{errors:#?}"
            );
        }
    }

    #[test]
    fn get_scenario_returns_none_for_unknown_id() {
        assert_eq!(get_scenario("totally-unknown-scenario"), None);
        assert_eq!(get_scenario(""), None);
    }

    #[test]
    fn list_scenario_summaries_has_one_entry_per_scenario_in_fixed_order() {
        let summaries = list_scenario_summaries();
        assert_eq!(summaries.len(), SCENARIO_IDS.len());
        let ids: Vec<&str> = summaries.iter().map(|s| s.scenario_id.as_str()).collect();
        assert_eq!(ids, SCENARIO_IDS.to_vec());
    }

    #[test]
    fn list_scenario_summaries_fields_match_the_full_manifest() {
        for summary in list_scenario_summaries() {
            let manifest = get_scenario(&summary.scenario_id).unwrap();
            assert_eq!(summary.scenario_version, manifest.scenario_version);
            assert_eq!(summary.title, manifest.title);
            assert_eq!(summary.task_query, manifest.task_query);
            assert_eq!(summary.item_count as usize, manifest.items.len());
            assert!(!summary.title.is_empty());
            assert!(!summary.task_query.is_empty());
        }
    }

    #[test]
    fn scenario_item_counts_are_within_the_documented_12_to_16_range() {
        for &id in all_scenario_ids() {
            let manifest = get_scenario(id).unwrap();
            let count = manifest.items.len();
            assert!(
                (12..=16).contains(&count),
                "'{id}' has {count} items, expected 12..=16"
            );
        }
    }
}
