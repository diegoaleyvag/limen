//! Shared logic for generating this crate's checked-in JSON Schema files
//! (`schemas/scenario-manifest-v1.schema.json`, `schemas/trial-result-v1.schema.json`), used
//! identically by the `generate_schemas` example (which writes the files) and the
//! `schema_freshness` integration test (which asserts the checked-in files match freshly
//! regenerated output byte-for-byte). Routing both through these two functions means generation
//! and freshness-checking can never drift apart.
//!
//! Schemas are derived directly from the `ScenarioManifest`/`TrialResult` Rust types via
//! `#[derive(schemars::JsonSchema)]` (added as a normal dependency of this library, alongside
//! `Serialize`/`Deserialize`, on every type reachable from those two roots) rather than via a
//! separate `xtask`/example-only dependency. This keeps schema generation trivially in sync with
//! the types by construction (one derive, same struct) at the cost of `schemars` being compiled
//! into every build of this crate, including the WASM target -- an acceptable, documented
//! trade-off for this phase since `crates/limen-wasm` does not yet call into any schema-bearing
//! code path (its one export is `engine_version()`), so the extra code should be dead-code-
//! eliminated from the actual WASM binary. If a later phase finds WASM binary size a problem,
//! gating `schemars`/`JsonSchema` behind a Cargo feature is a straightforward follow-up.
//!
//! Output is pretty-printed (unlike the compact canonical bytes used for content/result digests)
//! since these files are meant to be read by humans and external tooling, but is still routed
//! through [`crate::canonical::sort_json_value`] first so key order is guaranteed independent of
//! whichever `serde_json::Map` backing store is active in the final dependency graph -- the exact
//! same guard `canonical.rs` uses for digesting.

use schemars::{schema_for, Schema};

use crate::canonical::sort_json_value;
use crate::model::ScenarioManifest;
use crate::result::TrialResult;

fn finalize(schema: Schema) -> String {
    let value =
        serde_json::to_value(&schema).expect("schemars::Schema serialization is infallible");
    let sorted = sort_json_value(value);
    let mut text = serde_json::to_string_pretty(&sorted)
        .expect("pretty-printing a plain JSON value is infallible");
    text.push('\n');
    text
}

/// Deterministic, pretty-printed, key-sorted JSON Schema text for [`ScenarioManifest`].
pub fn scenario_manifest_schema_json() -> String {
    finalize(schema_for!(ScenarioManifest))
}

/// Deterministic, pretty-printed, key-sorted JSON Schema text for [`TrialResult`].
pub fn trial_result_schema_json() -> String {
    finalize(schema_for!(TrialResult))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_generation_is_deterministic_across_repeated_calls() {
        assert_eq!(
            scenario_manifest_schema_json(),
            scenario_manifest_schema_json()
        );
        assert_eq!(trial_result_schema_json(), trial_result_schema_json());
    }

    #[test]
    fn schemas_are_non_empty_json_objects() {
        for text in [scenario_manifest_schema_json(), trial_result_schema_json()] {
            let value: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
            assert!(value.is_object());
        }
    }
}
