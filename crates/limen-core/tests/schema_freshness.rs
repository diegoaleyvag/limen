//! Asserts the checked-in schema files exactly match freshly regenerated output. CI and later
//! phases rely on this guard: if a schema-bearing type changes without regenerating the files
//! (`cargo run -p limen-core --example generate_schemas`), this test fails with a clear diff-able
//! message rather than letting the checked-in schema silently go stale.

use std::path::PathBuf;

fn schemas_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas")
}

fn assert_matches_checked_in_file(file_name: &str, fresh: &str) {
    let path = schemas_dir().join(file_name);
    let checked_in = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "failed to read checked-in schema at {}: {e}",
            path.display()
        )
    });
    assert_eq!(
        checked_in, fresh,
        "checked-in {file_name} is stale; regenerate with `cargo run -p limen-core --example generate_schemas`"
    );
}

#[test]
fn scenario_manifest_schema_is_fresh() {
    assert_matches_checked_in_file(
        "scenario-manifest-v1.schema.json",
        &limen_core::schema_support::scenario_manifest_schema_json(),
    );
}

#[test]
fn trial_result_schema_is_fresh() {
    assert_matches_checked_in_file(
        "trial-result-v1.schema.json",
        &limen_core::schema_support::trial_result_schema_json(),
    );
}
