//! `wasm-bindgen-test` harness proving native/WASM parity: for every one of the 75 golden-fixture
//! trials, calls the *actual compiled* `limen-wasm` `run_trial` export -- not the native Rust
//! function directly, and not a reimplementation -- and asserts it produces byte-identical
//! canonical JSON, and the same `result_digest`, as the checked-in native golden fixture built by
//! `crates/limen-core/examples/generate_golden_fixtures.rs`. This is the concrete proof behind the
//! acceptance criterion "Rust and browser results are the same implementation, not parallel
//! logic": both sides ultimately run `limen_core::run_trial`, but this test exercises it through
//! the real compiled `wasm32-unknown-unknown` artifact and the `wasm-bindgen` JS glue, not just
//! the shared Rust source.
//!
//! Run with: `wasm-pack test --node` from `crates/limen-wasm/` (or
//! `wasm-pack test --node crates/limen-wasm` from the workspace root).
//!
//! This file is `wasm32`-only (see the `cfg` below): `wasm-bindgen-test`'s `#[wasm_bindgen_test]`
//! attribute degrades to a plain `#[test]` on native targets, which would make `cargo test
//! --workspace` attempt to run it too -- but the whole point is exercising the compiled WASM
//! artifact specifically, so this is deliberately excluded from native `cargo test` runs.
#![cfg(target_arch = "wasm32")]

mod golden_data;

use wasm_bindgen_test::wasm_bindgen_test;

// No `wasm_bindgen_test_configure!` call needed: absent any configuration, tests run in whatever
// harness `wasm-pack test` was invoked with (Node.js here, via `--node`), which is exactly what
// this parity check requires (no DOM/browser APIs are used by `run_trial`).

/// Parses `field` out of a canonical `TrialResult` JSON string as a `&str`. Small local helper so
/// the two tests below don't each hand-roll their own JSON digging.
fn extract_string_field(json: &str, field: &str) -> String {
    let value: serde_json::Value =
        serde_json::from_str(json).unwrap_or_else(|e| panic!("not valid JSON: {e}\n{json}"));
    value[field]
        .as_str()
        .unwrap_or_else(|| panic!("field {field:?} missing or not a string in: {json}"))
        .to_string()
}

#[wasm_bindgen_test]
fn wasm_run_trial_matches_all_75_native_golden_fixtures_byte_for_byte() {
    assert_eq!(
        golden_data::GOLDEN_CASES.len(),
        75,
        "golden case matrix drifted from 75 cases -- regenerate crates/limen-wasm/tests/golden_data/mod.rs"
    );

    let mut checked = 0usize;
    for &(scenario_id, strategy_id, tier_name, requested_tokens, expected_canonical_json) in
        golden_data::GOLDEN_CASES
    {
        let wasm_json = limen_wasm::run_trial(scenario_id, strategy_id, requested_tokens)
            .unwrap_or_else(|e| {
                panic!(
                    "wasm run_trial failed for {scenario_id}/{strategy_id}@{requested_tokens} \
                     (tier {tier_name}): {e:?}"
                )
            });

        let expected = expected_canonical_json.trim_end_matches('\n');
        assert_eq!(
            wasm_json, expected,
            "WASM run_trial output differs from the native golden fixture for \
             {scenario_id}/{strategy_id}@{requested_tokens} (tier {tier_name})"
        );
        checked += 1;
    }
    assert_eq!(checked, 75);
}

#[wasm_bindgen_test]
fn wasm_run_trial_result_digest_matches_native_golden_digest_for_all_75_cases() {
    let mut checked = 0usize;
    for &(scenario_id, strategy_id, tier_name, requested_tokens, expected_canonical_json) in
        golden_data::GOLDEN_CASES
    {
        let wasm_json = limen_wasm::run_trial(scenario_id, strategy_id, requested_tokens)
            .unwrap_or_else(|e| {
                panic!("wasm run_trial failed for {scenario_id}/{strategy_id}@{requested_tokens}: {e:?}")
            });

        let wasm_digest = extract_string_field(&wasm_json, "result_digest");
        let native_digest = extract_string_field(
            expected_canonical_json.trim_end_matches('\n'),
            "result_digest",
        );

        assert_eq!(
            wasm_digest, native_digest,
            "result_digest mismatch between WASM and native golden fixture for \
             {scenario_id}/{strategy_id}@{requested_tokens} (tier {tier_name})"
        );
        assert!(wasm_digest.starts_with("sha256:"));
        checked += 1;
    }
    assert_eq!(checked, 75);
}

#[wasm_bindgen_test]
fn wasm_run_trial_rejects_unknown_scenario_and_strategy_structurally() {
    let err = limen_wasm::run_trial("no-such-scenario", "recency@1", 100)
        .err()
        .expect("unknown scenario must be Err");
    let err_str = err
        .as_string()
        .expect("error payload must be a JsValue string");
    assert!(err_str.contains("unknown_scenario_id"));

    let err = limen_wasm::run_trial("incident-investigation", "no-such-strategy@1", 100)
        .err()
        .expect("unknown strategy must be Err");
    let err_str = err
        .as_string()
        .expect("error payload must be a JsValue string");
    assert!(err_str.contains("unknown_strategy_version"));
}
