//! The selection-strategy trait and version registry.
//!
//! Every strategy id is versioned (`"<family>@<version>"`) and resolved by **exact string
//! match** against a fixed, static list -- never a `HashMap` -- so lookup order and failure
//! behavior are identical on every platform and never depend on hash-iteration order.
//!
//! This phase ships five strategies with a shared, intentionally minimal placeholder body (see
//! each submodule's doc comment). A later phase implements the real per-strategy algorithms by
//! editing *only* the five files below; this module (`mod.rs`) should not need to change for
//! that work.

pub mod full_input_truncation;
pub mod hierarchical_summary;
pub mod recency;
pub mod retrieval_ranking;
pub mod structured_extraction;

use serde::Serialize;

use crate::error::EngineError;
use crate::model::{Budget, SelectionOutput, StrategyInput};

/// A deterministic context-selection strategy.
///
/// Implementations receive only [`StrategyInput`] (task query + strategy-visible items) and a
/// [`Budget`]; they never see [`crate::model::ScenarioAnnotations`]. `select` is infallible by
/// design (it returns [`SelectionOutput`] directly, not a `Result`): a strategy must always be
/// able to produce *some* deterministic selection for any well-formed input, even a zero budget.
pub trait SelectionStrategy {
    /// Exact registry id, e.g. `"recency@1"`. Must match the string this strategy is registered
    /// under in [`resolve_strategy`]/[`list_strategy_ids`].
    fn id(&self) -> &'static str;

    fn select(&self, input: &StrategyInput, budget: &Budget) -> SelectionOutput;
}

/// The five registered strategy ids, in the fixed order they are documented and shipped.
pub const STRATEGY_IDS: [&str; 5] = [
    "full-input-truncation@1",
    "recency@1",
    "structured-extraction@1",
    "hierarchical-summary@1",
    "retrieval-ranking@1",
];

/// Returns the registered strategy ids, in fixed order (see [`STRATEGY_IDS`]).
pub fn list_strategy_ids() -> &'static [&'static str] {
    &STRATEGY_IDS
}

/// Product-facing display metadata for one registered strategy: an exact `label` and a one-line
/// `summary` describing its mechanism precisely and neutrally, for UI use (a scenario/strategy
/// picker, a results column header, etc). Never describes a strategy as "AI", "smart",
/// "intelligent", or as "understanding" anything -- see the crate-level non-claims policy this
/// product enforces. [`hierarchical-summary@1`][hierarchical_summary]'s `label` is a hard
/// requirement, copied verbatim wherever this strategy is named in the UI: it must always read
/// exactly **"Hierarchical summary (deterministic/template-based)"**, never a bare "Hierarchical
/// summary" or anything implying generative/LLM output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct StrategyDescriptor {
    /// Exact registry id, matching one entry of [`STRATEGY_IDS`].
    pub strategy_id: &'static str,
    pub label: &'static str,
    pub summary: &'static str,
}

/// Display metadata for every registered strategy, in the same fixed order as [`STRATEGY_IDS`]
/// (enforced by the `strategy_descriptor_ids_match_strategy_ids_in_order` test below).
pub const STRATEGY_DESCRIPTORS: [StrategyDescriptor; 5] = [
    StrategyDescriptor {
        strategy_id: "full-input-truncation@1",
        label: "Full-input truncation",
        summary: "Keeps items from the start of the input, in original order, until the budget runs out.",
    },
    StrategyDescriptor {
        strategy_id: "recency@1",
        label: "Recency selection",
        summary: "Keeps the most recent items first, working backward until the budget runs out.",
    },
    StrategyDescriptor {
        strategy_id: "structured-extraction@1",
        label: "Structured extraction",
        summary: "Keeps only sentences containing a digit from each item, in original order, until the budget runs out.",
    },
    StrategyDescriptor {
        strategy_id: "hierarchical-summary@1",
        label: "Hierarchical summary (deterministic/template-based)",
        summary: "Replaces each item with a fixed-template one-line excerpt. Not an AI-generated summary.",
    },
    StrategyDescriptor {
        strategy_id: "retrieval-ranking@1",
        label: "Retrieval / ranking",
        summary: "Ranks items by lexical overlap with the task query, then greedily packs the highest-ranked items that fit.",
    },
];

/// Returns the display metadata for every registered strategy, in fixed order (see
/// [`STRATEGY_DESCRIPTORS`]).
pub fn list_strategy_descriptors() -> &'static [StrategyDescriptor] {
    &STRATEGY_DESCRIPTORS
}

/// Resolves a strategy id to its implementation via an exact match against a fixed, static table
/// (a `match` expression, not a `HashMap`). Unknown ids -- including a known family with an
/// unsupported version suffix such as `"recency@2"` -- fail with a stable
/// [`EngineError::UnknownStrategyVersion`].
///
/// This is a distinct failure path from [`crate::validate::validate_manifest`]: it is about the
/// *requested strategy*, not the scenario content.
pub fn resolve_strategy(strategy_id: &str) -> Result<Box<dyn SelectionStrategy>, EngineError> {
    match strategy_id {
        "full-input-truncation@1" => {
            Ok(Box::new(full_input_truncation::FullInputTruncationStrategy))
        }
        "recency@1" => Ok(Box::new(recency::RecencyStrategy)),
        "structured-extraction@1" => Ok(Box::new(
            structured_extraction::StructuredExtractionStrategy,
        )),
        "hierarchical-summary@1" => Ok(Box::new(hierarchical_summary::HierarchicalSummaryStrategy)),
        "retrieval-ranking@1" => Ok(Box::new(retrieval_ranking::RetrievalRankingStrategy)),
        other => Err(EngineError::UnknownStrategyVersion(other.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_strategy_ids_matches_fixed_documented_order() {
        assert_eq!(
            list_strategy_ids(),
            &[
                "full-input-truncation@1",
                "recency@1",
                "structured-extraction@1",
                "hierarchical-summary@1",
                "retrieval-ranking@1",
            ]
        );
    }

    #[test]
    fn resolve_strategy_succeeds_for_every_registered_id_and_round_trips_its_own_id() {
        for &id in list_strategy_ids() {
            let strategy = resolve_strategy(id)
                .unwrap_or_else(|e| panic!("expected {id} to resolve, got {e:?}"));
            assert_eq!(
                strategy.id(),
                id,
                "resolved strategy's own id() must match the lookup key"
            );
        }
    }

    #[test]
    fn resolve_strategy_fails_for_unknown_id() {
        // `Box<dyn SelectionStrategy>` is not `Debug`, so `Result::unwrap_err` (which would need
        // to `Debug`-format the `Ok` value in its panic message) is unavailable; go through
        // `Result::err` (which only ever needs to move/drop the `Ok` value) instead.
        let err = resolve_strategy("totally-unknown-strategy@1")
            .err()
            .unwrap();
        assert_eq!(err.code(), "unknown_strategy_version");
        match err {
            EngineError::UnknownStrategyVersion(id) => assert_eq!(id, "totally-unknown-strategy@1"),
            other => panic!("expected UnknownStrategyVersion, got {other:?}"),
        }
    }

    #[test]
    fn strategy_descriptor_ids_match_strategy_ids_in_order() {
        let descriptor_ids: Vec<&str> =
            STRATEGY_DESCRIPTORS.iter().map(|d| d.strategy_id).collect();
        assert_eq!(descriptor_ids, STRATEGY_IDS.to_vec());
        assert_eq!(list_strategy_descriptors().len(), list_strategy_ids().len());
    }

    #[test]
    fn every_descriptor_resolves_and_has_nonempty_label_and_summary() {
        for descriptor in list_strategy_descriptors() {
            assert!(resolve_strategy(descriptor.strategy_id).is_ok());
            assert!(!descriptor.label.is_empty());
            assert!(!descriptor.summary.is_empty());
        }
    }

    #[test]
    fn hierarchical_summary_label_is_the_exact_required_non_claims_string() {
        let descriptor = STRATEGY_DESCRIPTORS
            .iter()
            .find(|d| d.strategy_id == "hierarchical-summary@1")
            .expect("hierarchical-summary@1 must be registered");
        assert_eq!(
            descriptor.label,
            "Hierarchical summary (deterministic/template-based)"
        );
    }

    #[test]
    fn descriptors_match_the_exact_specified_copy_verbatim() {
        // Every label/summary pair below is copied verbatim from the product spec. This is a
        // stronger, more direct check than a generic banned-word scanner: a heuristic scanner for
        // words like "ai" would false-positive on `hierarchical-summary@1`'s required, deliberate
        // negation ("Not an AI-generated summary") -- the actual policy is "never claim
        // intelligence", which permits explicitly *disclaiming* it, so exact-copy comparison is
        // the correct tool here, not substring/word banning.
        let expected: [(&str, &str, &str); 5] = [
            (
                "full-input-truncation@1",
                "Full-input truncation",
                "Keeps items from the start of the input, in original order, until the budget runs out.",
            ),
            (
                "recency@1",
                "Recency selection",
                "Keeps the most recent items first, working backward until the budget runs out.",
            ),
            (
                "structured-extraction@1",
                "Structured extraction",
                "Keeps only sentences containing a digit from each item, in original order, until the budget runs out.",
            ),
            (
                "hierarchical-summary@1",
                "Hierarchical summary (deterministic/template-based)",
                "Replaces each item with a fixed-template one-line excerpt. Not an AI-generated summary.",
            ),
            (
                "retrieval-ranking@1",
                "Retrieval / ranking",
                "Ranks items by lexical overlap with the task query, then greedily packs the highest-ranked items that fit.",
            ),
        ];
        for ((id, label, summary), descriptor) in expected.iter().zip(STRATEGY_DESCRIPTORS.iter()) {
            assert_eq!(descriptor.strategy_id, *id);
            assert_eq!(descriptor.label, *label);
            assert_eq!(descriptor.summary, *summary);
        }
    }

    #[test]
    fn resolve_strategy_fails_for_plausible_but_wrong_version_suffix() {
        let err = resolve_strategy("recency@2").err().unwrap();
        assert_eq!(err.code(), "unknown_strategy_version");
        match err {
            EngineError::UnknownStrategyVersion(id) => assert_eq!(id, "recency@2"),
            other => panic!("expected UnknownStrategyVersion, got {other:?}"),
        }
    }
}
