//! Validates every checked-in golden fixture (`tests/golden/*.json`) against the real, checked-in
//! `schemas/trial-result-v1.schema.json` using a real JSON Schema validator (the `jsonschema`
//! crate, Draft 2020-12) -- not a hand-rolled shape check. This is the concrete proof that the
//! 75-trial golden matrix is schema-valid, one of the required "verify-foundation" acceptance
//! criteria.

use std::path::PathBuf;

use limen_core::golden_support::{golden_case_filename, golden_cases};

fn schemas_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas")
}

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

fn load_trial_result_validator() -> jsonschema::Validator {
    let schema_path = schemas_dir().join("trial-result-v1.schema.json");
    let schema_text = std::fs::read_to_string(&schema_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", schema_path.display()));
    let schema_value: serde_json::Value =
        serde_json::from_str(&schema_text).expect("schema file must be valid JSON");
    jsonschema::validator_for(&schema_value).expect("schema must compile as a valid JSON Schema")
}

#[test]
fn every_golden_fixture_is_valid_against_the_trial_result_schema() {
    let validator = load_trial_result_validator();
    let cases = golden_cases();
    assert_eq!(cases.len(), 75);

    let mut checked = 0usize;
    for case in &cases {
        let filename = golden_case_filename(case);
        let path = golden_dir().join(&filename);
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        let instance: serde_json::Value = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("{filename} is not valid JSON: {e}"));

        let errors: Vec<String> = validator
            .iter_errors(&instance)
            .map(|e| format!("{e} (at {})", e.instance_path()))
            .collect();
        assert!(
            errors.is_empty(),
            "golden fixture {filename} failed schema validation:\n{}",
            errors.join("\n")
        );
        checked += 1;
    }
    assert_eq!(checked, 75);
}

/// A deliberately-invalid instance (missing every required property) must be rejected -- proves
/// the validator itself is actually enforcing the schema, not vacuously accepting everything.
#[test]
fn schema_validator_rejects_an_obviously_invalid_instance() {
    let validator = load_trial_result_validator();
    let empty = serde_json::json!({});
    assert!(
        !validator.is_valid(&empty),
        "an empty object must fail validation against trial-result-v1.schema.json"
    );
}
