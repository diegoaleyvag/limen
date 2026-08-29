//! Prints the minimal fixture [`ScenarioManifest`][limen_core::ScenarioManifest] and a
//! [`TrialResult`][limen_core::TrialResult] produced by running the `full-input-truncation@1`
//! stub strategy against it, both as pretty-printed, key-sorted JSON.
//!
//! This is a live, always-accurate ground-truth shape reference for downstream phases (scenario
//! authoring, real strategy implementation): run it any time with
//! `cargo run -p limen-core --example print_minimal_trial` rather than trusting a pasted-in-chat
//! copy to stay in sync with the actual types.

use limen_core::model::Budget;
use limen_core::result::TrialResult;
use limen_core::tokenizer::TOKENIZER_ID;
use limen_core::{canonical, fixtures, metrics, strategy, ENGINE_VERSION, SCHEMA_VERSION};

fn print_pretty_sorted(label: &str, value: &impl serde::Serialize) {
    let raw = serde_json::to_value(value).expect("plain data always serializes");
    let sorted = canonical::sort_json_value(raw);
    println!("=== {label} ===");
    println!(
        "{}",
        serde_json::to_string_pretty(&sorted).expect("plain data always serializes")
    );
}

fn main() {
    let manifest = fixtures::minimal_valid_manifest();
    print_pretty_sorted(
        "ScenarioManifest (fixtures::minimal_valid_manifest())",
        &manifest,
    );

    let strategy_id = "full-input-truncation@1";
    let selected = strategy::resolve_strategy(strategy_id).expect("registered strategy id");
    let input = manifest.to_strategy_input();
    let selection = selected.select(
        &input,
        &Budget {
            requested_tokens: 1000,
        },
    );
    let computed_metrics =
        metrics::compute_metrics(&manifest.annotations, &selection, &manifest.items);

    let mut trial_result = TrialResult {
        schema_version: SCHEMA_VERSION.to_string(),
        engine_version: ENGINE_VERSION.to_string(),
        tokenizer_id: TOKENIZER_ID.to_string(),
        scenario_id: manifest.scenario_id.clone(),
        scenario_version: manifest.scenario_version.clone(),
        scenario_content_digest: manifest.content_digest.clone(),
        strategy_id: selection.strategy_id.clone(),
        selection,
        metrics: computed_metrics,
        result_digest: String::new(),
    };
    trial_result.result_digest =
        canonical::digest_with_field_blanked(&trial_result, "result_digest")
            .expect("plain data always canonicalizes");

    print_pretty_sorted(
        "TrialResult (full-input-truncation@1 @ 1000 tokens)",
        &trial_result,
    );
}
