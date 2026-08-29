//! Thin WASM adapter over `limen-core`.
//!
//! Every export below does exactly three things: call into `limen-core` (the catalog, the
//! strategy registry, `compute_metrics`, and the canonicalization primitives), convert the
//! result to a JSON string, and cross the `wasm-bindgen` boundary. No tokenizer, selector,
//! evaluator, validator, or digester logic is duplicated here -- this file only ever calls the
//! one real implementation of each.
//!
//! # Panic safety at the WASM boundary
//!
//! No panic may ever escape to an uncaught JS exception that poisons the module. This crate
//! deliberately does **not** use `std::panic::catch_unwind`: as of this writing, catching panics
//! across the `wasm-bindgen` boundary on `wasm32-unknown-unknown` requires building with
//! `-C panic=unwind` plus `-Z build-std=std,panic_unwind`, both of which are nightly-only
//! (`-Z build-std` is not available on stable), and additionally require a WebAssembly
//! exception-handling-capable runtime (a sufficiently recent Node.js or browser). That is
//! incompatible with this workspace's pinned *stable* `1.97.1` toolchain
//! (`rust-toolchain.toml`) and would be a disproportionate toolchain change for this foundation
//! phase. Instead, this crate relies entirely on the existing Result-based discipline already
//! proven throughout `limen-core` (no `unwrap`/`expect`/`panic!`/panicking-index on any reachable
//! decision or artifact-construction path -- see that crate's module docs), plus
//! `console_error_panic_hook` (installed once at module init, below) purely for readable
//! diagnostics in the browser console in the hypothetical case a future change introduces a
//! panic. The crate's test suite empirically proves every edge/invalid input this module accepts
//! -- an unknown scenario id, an unknown strategy id/version, `requested_tokens: 0`, and the
//! largest embedded scenario at a 1-token budget -- resolves to a structured `Ok`/`Err`, never a
//! panic.
//!
//! Every fallible export mirrors the same convention: `Err`'s payload is a JSON *string* shaped
//! like [`limen_core::EngineError`]'s own `Serialize` output (`{"error": "...", "detail": ...}`,
//! from its `#[serde(tag = "error", content = "detail")]` representation), wrapped in a
//! `JsValue` so a normal JS `catch` gets a parseable string, never an opaque generic error.

use wasm_bindgen::prelude::*;

use limen_core::canonical;
use limen_core::error::EngineError;

/// Installs `console_error_panic_hook` once at module init, so that *if* a panic were ever to
/// escape despite the Result-based discipline described in the module docs above, the browser
/// console would show a readable Rust message/stack trace instead of an opaque "unreachable
/// executed" WebAssembly trap. Purely defense-in-depth diagnostics; no exported function below
/// relies on this to behave correctly.
#[wasm_bindgen(start)]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

/// Returns `limen-core`'s package version (not `limen-wasm`'s own version), so the browser can
/// display/record exactly which engine build produced a result.
#[wasm_bindgen]
pub fn engine_version() -> String {
    limen_core::ENGINE_VERSION.to_string()
}

/// JSON array of `{ scenario_id, scenario_version, title, task_query, item_count }` for the three
/// embedded scenarios, in [`limen_core::catalog::SCENARIO_IDS`] order. Infallible: plain owned
/// `String`/`u32` data can never fail to serialize, but the practically-unreachable failure path
/// still falls back to `"[]"` rather than panicking, per this crate's no-panics policy.
#[wasm_bindgen]
pub fn list_scenarios() -> String {
    let summaries = limen_core::list_scenario_summaries();
    serde_json::to_string(&summaries).unwrap_or_else(|_| "[]".to_string())
}

/// JSON array of `{ strategy_id, label, summary }` for the five registered strategies, in
/// [`limen_core::strategy::STRATEGY_IDS`] order. Infallible for the same reason as
/// [`list_scenarios`].
#[wasm_bindgen]
pub fn list_strategies() -> String {
    let descriptors = limen_core::list_strategy_descriptors();
    serde_json::to_string(descriptors).unwrap_or_else(|_| "[]".to_string())
}

/// Serializes `err` using [`limen_core::EngineError`]'s own tagged `Serialize` representation
/// (`{"error": "<code>", "detail": ...}`). Falls back to a hand-written JSON literal of the same
/// shape on the practically-unreachable serialization-failure path (see
/// [`canonical::canonical_json_string`]'s doc comment for why plain owned data cannot actually
/// fail here) -- this function itself can therefore never panic.
fn error_json(err: &EngineError) -> String {
    serde_json::to_string(err)
        .unwrap_or_else(|_| r#"{"error":"serialization_failed","detail":null}"#.to_string())
}

/// Wraps [`error_json`] in a `JsValue` so `Err`-returning exports hand JS a parseable JSON
/// string, not an opaque error object.
fn error_js(err: &EngineError) -> JsValue {
    JsValue::from_str(&error_json(err))
}

/// The real logic behind [`get_scenario_detail`], kept free of any `wasm_bindgen`/`JsValue` type
/// so it is directly unit-testable with plain `cargo test` (no WASM runtime needed).
fn get_scenario_detail_impl(scenario_id: &str) -> Result<String, EngineError> {
    let manifest = limen_core::get_scenario(scenario_id)
        .ok_or_else(|| EngineError::UnknownScenarioId(scenario_id.to_string()))?;
    canonical::canonical_json_string(&manifest)
}

/// On success, the full [`limen_core::ScenarioManifest`] (items *and* `annotations`) serialized
/// as canonical JSON -- the UI is allowed to show evaluator-side context (`why_it_matters`,
/// distractor flags, contradiction membership) to the human user for teaching purposes; only
/// [`limen_core::StrategyInput`]/`SelectionStrategy::select` are annotation-free. On failure
/// (unknown `scenario_id`), an `Err` carrying the structured JSON-string error described in the
/// module docs.
#[wasm_bindgen]
pub fn get_scenario_detail(scenario_id: &str) -> Result<String, JsValue> {
    get_scenario_detail_impl(scenario_id).map_err(|e| error_js(&e))
}

/// The real logic behind [`run_trial`], kept free of any `wasm_bindgen`/`JsValue` type so it is
/// directly unit-testable with plain `cargo test` (no WASM runtime needed). Delegates every real
/// step to [`limen_core::run_trial`] -- the exact same native function
/// `crates/limen-core`'s own golden-fixture tests call -- so this adapter cannot drift into a
/// second, parallel implementation of trial construction; it only serializes the result to the
/// canonical JSON string the WASM boundary hands back.
fn run_trial_impl(
    scenario_id: &str,
    strategy_id: &str,
    requested_tokens: u32,
) -> Result<String, EngineError> {
    let result = limen_core::run_trial(scenario_id, strategy_id, requested_tokens)?;
    canonical::canonical_json_string(&result)
}

/// Runs one strategy against one scenario at one budget. On success, returns the exact canonical
/// [`limen_core::TrialResult`] JSON string -- the same compact, sorted-key encoding style used to
/// compute `result_digest` (via [`canonical::canonical_json_string`], built on the same
/// `crate::canonical` primitives), with **no wrapper object**: the returned string *is* the
/// artifact, byte-identical to what a caller should download. Never `JSON.parse`-then-
/// `JSON.stringify` this value expecting an equivalent "clean" artifact; parse a *copy* for
/// display only.
///
/// On failure (unknown `scenario_id`, unknown `strategy_id`/version, or any other
/// [`EngineError`]), an `Err` carrying the structured JSON-string error described in the module
/// docs. A very small or zero `requested_tokens` is **not** a failure: it is a legal trial whose
/// selection simply drops most or all items (see this module's test suite).
#[wasm_bindgen]
pub fn run_trial(
    scenario_id: &str,
    strategy_id: &str,
    requested_tokens: u32,
) -> Result<String, JsValue> {
    run_trial_impl(scenario_id, strategy_id, requested_tokens).map_err(|e| error_js(&e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_scenarios_json_has_three_entries_with_expected_fields() {
        let json = list_scenarios();
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        let arr = value.as_array().expect("array");
        assert_eq!(arr.len(), 3);
        for entry in arr {
            assert!(entry.get("scenario_id").and_then(|v| v.as_str()).is_some());
            assert!(entry
                .get("scenario_version")
                .and_then(|v| v.as_str())
                .is_some());
            assert!(entry.get("title").and_then(|v| v.as_str()).is_some());
            assert!(entry.get("task_query").and_then(|v| v.as_str()).is_some());
            assert!(entry.get("item_count").and_then(|v| v.as_u64()).is_some());
        }
    }

    #[test]
    fn list_strategies_json_has_five_entries_in_fixed_order_with_exact_hierarchical_label() {
        let json = list_strategies();
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        let arr = value.as_array().expect("array");
        assert_eq!(arr.len(), 5);
        let ids: Vec<&str> = arr
            .iter()
            .map(|e| e["strategy_id"].as_str().unwrap())
            .collect();
        assert_eq!(ids, limen_core::list_strategy_ids().to_vec());
        let hierarchical = arr
            .iter()
            .find(|e| e["strategy_id"] == "hierarchical-summary@1")
            .expect("hierarchical-summary@1 present");
        assert_eq!(
            hierarchical["label"],
            "Hierarchical summary (deterministic/template-based)"
        );
        assert!(arr.iter().all(|e| e.get("summary").is_some()));
    }

    #[test]
    fn get_scenario_detail_impl_succeeds_for_every_known_scenario_and_includes_annotations() {
        for &id in limen_core::all_scenario_ids() {
            let json = get_scenario_detail_impl(id).expect("known scenario id must succeed");
            let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
            assert_eq!(value["scenario_id"], id);
            assert!(!value["annotations"]["required_facts"]
                .as_array()
                .unwrap()
                .is_empty());
            assert!(value["annotations"]["distractor_source_ids"].is_array());
        }
    }

    #[test]
    fn get_scenario_detail_impl_fails_structurally_for_unknown_scenario() {
        let err = get_scenario_detail_impl("no-such-scenario").unwrap_err();
        assert_eq!(err.code(), "unknown_scenario_id");
    }

    #[test]
    fn run_trial_impl_succeeds_and_returns_canonical_compact_result_json() {
        let json = run_trial_impl("incident-investigation", "recency@1", 200).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(value["scenario_id"], "incident-investigation");
        assert_eq!(value["strategy_id"], "recency@1");
        assert!(value["result_digest"]
            .as_str()
            .unwrap()
            .starts_with("sha256:"));
        assert!(
            !json.contains('\n') && !json.contains("  "),
            "expected compact canonical encoding, got: {json}"
        );
    }

    #[test]
    fn run_trial_impl_result_digest_matches_independent_recomputation() {
        let json = run_trial_impl("product-comparison", "structured-extraction@1", 150).unwrap();
        let result: limen_core::TrialResult = serde_json::from_str(&json).unwrap();
        let recomputed = canonical::digest_with_field_blanked(&result, "result_digest").unwrap();
        assert_eq!(result.result_digest, recomputed);
    }

    #[test]
    fn run_trial_impl_fails_structurally_for_unknown_scenario_id() {
        let err = run_trial_impl("no-such-scenario", "recency@1", 100).unwrap_err();
        assert_eq!(err.code(), "unknown_scenario_id");
        match err {
            EngineError::UnknownScenarioId(id) => assert_eq!(id, "no-such-scenario"),
            other => panic!("expected UnknownScenarioId, got {other:?}"),
        }
    }

    #[test]
    fn run_trial_impl_fails_structurally_for_unknown_strategy_id() {
        let err = run_trial_impl("incident-investigation", "no-such-strategy@1", 100).unwrap_err();
        assert_eq!(err.code(), "unknown_strategy_version");
    }

    #[test]
    fn run_trial_impl_fails_structurally_for_known_family_unsupported_version() {
        let err = run_trial_impl("incident-investigation", "recency@2", 100).unwrap_err();
        assert_eq!(err.code(), "unknown_strategy_version");
    }

    #[test]
    fn run_trial_impl_succeeds_with_zero_requested_tokens_for_every_scenario_and_strategy() {
        // requested_tokens: 0 is a legal (if extreme) trial, not an invalid request: every
        // strategy's own test suite already proves a zero budget drops everything cleanly, so
        // this must be a well-formed `Ok` result, never an `Err`.
        for &scenario_id in limen_core::all_scenario_ids() {
            for &strategy_id in limen_core::list_strategy_ids() {
                let json = run_trial_impl(scenario_id, strategy_id, 0).unwrap_or_else(|e| {
                    panic!("expected Ok for {scenario_id}/{strategy_id}@0 tokens, got {e:?}")
                });
                let value: serde_json::Value = serde_json::from_str(&json).unwrap();
                assert_eq!(value["metrics"]["budget"]["requested_tokens"], 0);
                assert_eq!(value["metrics"]["budget"]["used_tokens"], 0);
            }
        }
    }

    #[test]
    fn run_trial_impl_succeeds_at_a_tiny_budget_for_the_largest_embedded_scenario() {
        // The largest embedded scenario (by item count) at a 1-token budget: a legal, extreme
        // trial that must still return a well-formed `Ok`, never panic or error.
        let largest_scenario_id = limen_core::all_scenario_ids()
            .iter()
            .map(|&id| limen_core::get_scenario(id).expect("known id"))
            .max_by_key(|manifest| manifest.items.len())
            .expect("at least one embedded scenario")
            .scenario_id;

        for &strategy_id in limen_core::list_strategy_ids() {
            let json = run_trial_impl(&largest_scenario_id, strategy_id, 1).unwrap_or_else(|e| {
                panic!("expected Ok for {largest_scenario_id}/{strategy_id}@1 token, got {e:?}")
            });
            let value: serde_json::Value = serde_json::from_str(&json).unwrap();
            let used = value["metrics"]["budget"]["used_tokens"].as_u64().unwrap();
            assert!(
                used <= 1,
                "used_tokens must never exceed the 1-token budget"
            );
        }
    }

    #[test]
    fn error_json_matches_engine_error_serde_shape() {
        let err = EngineError::UnknownScenarioId("ghost".to_string());
        let json = error_json(&err);
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(value["error"], "unknown_scenario_id");
        assert_eq!(value["detail"], "ghost");

        let err = EngineError::UnknownStrategyVersion("ghost@9".to_string());
        let json = error_json(&err);
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(value["error"], "unknown_strategy_version");
        assert_eq!(value["detail"], "ghost@9");
    }
}
