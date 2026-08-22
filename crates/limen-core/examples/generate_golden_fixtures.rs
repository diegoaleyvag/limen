//! Generates the checked-in golden-fixture matrix under `tests/golden/`: one canonical `TrialResult`
//! JSON file per [`limen_core::golden_support::GoldenCase`] (75 total: 3 scenarios x 5 strategies
//! x 5 budget tiers), plus a pretty-printed `index.json` summarizing every case.
//!
//! Run from anywhere with: `cargo run -p limen-core --example generate_golden_fixtures`
//!
//! Every trial is built via [`limen_core::run_trial`] -- the exact same native function
//! `crates/limen-wasm`'s `run_trial` export calls -- and every fixture file's content is the
//! exact canonical (compact, sorted-key) JSON string [`limen_core::canonical::canonical_json_string`]
//! produces, with one trailing newline appended for git-friendliness (regression tests trim it
//! back off before comparing). See `crates/limen-core/tests/golden_fixtures.rs` for the guard
//! that keeps these files honest.

use std::fs;
use std::path::PathBuf;

use limen_core::canonical;
use limen_core::golden_support::{golden_case_filename, golden_cases};

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

fn main() {
    let dir = golden_dir();
    fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("failed to create {}: {e}", dir.display()));

    let cases = golden_cases();
    let mut index_entries = Vec::with_capacity(cases.len());

    for case in &cases {
        let result =
            limen_core::run_trial(case.scenario_id, case.strategy_id, case.requested_tokens)
                .unwrap_or_else(|e| panic!("run_trial failed for {case:?}: {e}"));
        let canonical_json =
            canonical::canonical_json_string(&result).expect("plain data always canonicalizes");

        let filename = golden_case_filename(case);
        let path = dir.join(&filename);
        fs::write(&path, format!("{canonical_json}\n"))
            .unwrap_or_else(|e| panic!("failed to write {}: {e}", path.display()));

        index_entries.push(serde_json::json!({
            "scenario_id": case.scenario_id,
            "strategy_id": case.strategy_id,
            "tier_name": case.tier_name,
            "requested_tokens": case.requested_tokens,
            "filename": filename,
            "used_tokens": result.metrics.budget.used_tokens,
            "result_digest": result.result_digest,
        }));
    }

    let index_value = canonical::sort_json_value(serde_json::Value::Array(index_entries));
    let mut index_text =
        serde_json::to_string_pretty(&index_value).expect("plain JSON value always serializes");
    index_text.push('\n');
    let index_path = dir.join("index.json");
    fs::write(&index_path, &index_text)
        .unwrap_or_else(|e| panic!("failed to write {}: {e}", index_path.display()));

    println!(
        "wrote {} golden fixtures + index.json to {}",
        cases.len(),
        dir.display()
    );
}
