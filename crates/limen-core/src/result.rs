//! [`TrialResult`]: the exported/hashed artifact produced by running one strategy against one
//! scenario at one budget. This is the top-level shape a WASM adapter and the browser UI consume.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::EngineError;
use crate::metrics::Metrics;
use crate::model::{Budget, SelectionOutput};

/// The full, self-describing, hashable record of one trial (one scenario x one strategy x one
/// budget).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TrialResult {
    /// Result schema version, e.g. `"1.0.0"`.
    pub schema_version: String,
    /// `limen-core`'s own package version (`env!("CARGO_PKG_VERSION")`), i.e.
    /// [`crate::ENGINE_VERSION`].
    pub engine_version: String,
    /// Always [`crate::tokenizer::TOKENIZER_ID`].
    pub tokenizer_id: String,
    pub scenario_id: String,
    pub scenario_version: String,
    pub scenario_content_digest: String,
    /// Exact registry id, e.g. `"recency@1"`. Matches `selection.strategy_id`.
    pub strategy_id: String,
    pub selection: SelectionOutput,
    pub metrics: Metrics,
    /// `"sha256:..."` digest of the canonical `TrialResult` with this field blanked. Computed and
    /// verified via `crate::canonical::digest_with_field_blanked(result, "result_digest")`.
    pub result_digest: String,
}

/// Runs one registered strategy against one embedded scenario at one budget, end to end:
/// resolves the scenario and strategy, calls `select`, computes metrics, and builds the fully
/// digested [`TrialResult`]. This is the single native implementation shared by
/// `crates/limen-wasm`'s `run_trial` export and this crate's own golden-fixture
/// generation/regression tests, so "the WASM adapter and the native tests run the same logic" is
/// true by construction (one function, two callers) rather than by convention alone.
pub fn run_trial(
    scenario_id: &str,
    strategy_id: &str,
    requested_tokens: u32,
) -> Result<TrialResult, EngineError> {
    let manifest = crate::catalog::get_scenario(scenario_id)
        .ok_or_else(|| EngineError::UnknownScenarioId(scenario_id.to_string()))?;
    let strategy = crate::strategy::resolve_strategy(strategy_id)?;

    let input = manifest.to_strategy_input();
    let budget = Budget { requested_tokens };
    let selection = strategy.select(&input, &budget);
    let metrics =
        crate::metrics::compute_metrics(&manifest.annotations, &selection, &manifest.items);

    let mut result = TrialResult {
        schema_version: crate::SCHEMA_VERSION.to_string(),
        engine_version: crate::ENGINE_VERSION.to_string(),
        tokenizer_id: crate::tokenizer::TOKENIZER_ID.to_string(),
        scenario_id: manifest.scenario_id.clone(),
        scenario_version: manifest.scenario_version.clone(),
        scenario_content_digest: manifest.content_digest.clone(),
        strategy_id: selection.strategy_id.clone(),
        selection,
        metrics,
        result_digest: String::new(),
    };
    result.result_digest = crate::canonical::digest_with_field_blanked(&result, "result_digest")?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical;
    use crate::fixtures::minimal_valid_manifest;
    use crate::metrics::compute_metrics;
    use crate::strategy::resolve_strategy;
    use crate::tokenizer::TOKENIZER_ID;
    use crate::{ENGINE_VERSION, SCHEMA_VERSION};

    fn sample_trial_result() -> TrialResult {
        let manifest = minimal_valid_manifest();
        let strategy = resolve_strategy("full-input-truncation@1").unwrap();
        let input = manifest.to_strategy_input();
        let selection = strategy.select(
            &input,
            &crate::model::Budget {
                requested_tokens: 1000,
            },
        );
        let metrics = compute_metrics(&manifest.annotations, &selection, &manifest.items);

        let mut result = TrialResult {
            schema_version: SCHEMA_VERSION.to_string(),
            engine_version: ENGINE_VERSION.to_string(),
            tokenizer_id: TOKENIZER_ID.to_string(),
            scenario_id: manifest.scenario_id.clone(),
            scenario_version: manifest.scenario_version.clone(),
            scenario_content_digest: manifest.content_digest.clone(),
            strategy_id: selection.strategy_id.clone(),
            selection,
            metrics,
            result_digest: String::new(),
        };
        result.result_digest =
            canonical::digest_with_field_blanked(&result, "result_digest").unwrap();
        result
    }

    #[test]
    fn result_digest_matches_recomputation_with_field_blanked() {
        let result = sample_trial_result();
        let recomputed = canonical::digest_with_field_blanked(&result, "result_digest").unwrap();
        assert_eq!(result.result_digest, recomputed);
        assert!(result.result_digest.starts_with("sha256:"));
    }

    #[test]
    fn result_digest_changes_when_any_field_mutates() {
        let mut result = sample_trial_result();
        let original_digest = result.result_digest.clone();
        result.metrics.budget.used_tokens = result
            .metrics
            .budget
            .used_tokens
            .saturating_add(1)
            .min(result.metrics.budget.requested_tokens.max(1));
        result.strategy_id = "recency@1".to_string();
        let recomputed = canonical::digest_with_field_blanked(&result, "result_digest").unwrap();
        assert_ne!(original_digest, recomputed);
    }
}
