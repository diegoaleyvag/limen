//! Regression guard for the checked-in golden-fixture matrix (`tests/golden/*.json`): asserts
//! that regenerating every one of the 75 `(scenario, strategy, budget tier)` trials via
//! [`limen_core::run_trial`] reproduces byte-identical canonical JSON and the same `result_digest`
//! as the checked-in file. If a future change to `limen-core` alters any strategy's, metric's, or
//! canonicalization's behavior even slightly, this test fails with a clear, per-fixture message
//! rather than letting the change silently pass -- exactly the same freshness-guard pattern
//! `tests/schema_freshness.rs` uses for the schema files.
//!
//! Regenerate with: `cargo run -p limen-core --example generate_golden_fixtures`

use std::path::PathBuf;

use limen_core::canonical;
use limen_core::golden_support::{golden_case_filename, golden_cases};

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

#[test]
fn golden_fixture_matrix_covers_3_scenarios_x_5_strategies_x_5_tiers() {
    assert_eq!(golden_cases().len(), 75);
}

#[test]
fn every_golden_fixture_regenerates_byte_identical_canonical_json_and_digest() {
    let dir = golden_dir();
    let cases = golden_cases();
    assert_eq!(cases.len(), 75, "golden case matrix drifted from 75 cases");

    let mut checked = 0usize;
    for case in &cases {
        let filename = golden_case_filename(case);
        let path = dir.join(&filename);
        let checked_in = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "failed to read checked-in golden fixture {}: {e}\nregenerate with `cargo run -p limen-core --example generate_golden_fixtures`",
                path.display()
            )
        });
        let checked_in = checked_in.trim_end_matches('\n');

        let result =
            limen_core::run_trial(case.scenario_id, case.strategy_id, case.requested_tokens)
                .unwrap_or_else(|e| panic!("run_trial failed while regenerating {filename}: {e}"));
        let fresh =
            canonical::canonical_json_string(&result).expect("plain data always canonicalizes");

        assert_eq!(
            fresh, checked_in,
            "golden fixture {filename} is stale (regenerated canonical JSON differs); \
             regenerate with `cargo run -p limen-core --example generate_golden_fixtures` \
             and review the diff before checking it back in"
        );

        let recomputed_digest = canonical::digest_with_field_blanked(&result, "result_digest")
            .expect("plain data always canonicalizes");
        assert_eq!(
            result.result_digest, recomputed_digest,
            "result_digest for {filename} does not match an independent recomputation"
        );
        assert!(result.result_digest.starts_with("sha256:"));

        checked += 1;
    }
    assert_eq!(checked, 75);
}

#[test]
fn every_golden_fixture_used_tokens_never_exceeds_requested_tokens() {
    for case in golden_cases() {
        let filename = golden_case_filename(&case);
        let path = golden_dir().join(&filename);
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        let value: serde_json::Value = serde_json::from_str(&raw).expect("valid JSON");
        let requested = value["metrics"]["budget"]["requested_tokens"]
            .as_u64()
            .expect("requested_tokens present");
        let used = value["metrics"]["budget"]["used_tokens"]
            .as_u64()
            .expect("used_tokens present");
        assert!(
            used <= requested,
            "{filename}: used_tokens {used} exceeded requested_tokens {requested}"
        );
        assert_eq!(requested, case.requested_tokens as u64);
    }
}
