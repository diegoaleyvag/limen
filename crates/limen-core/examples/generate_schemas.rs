//! Regenerates `schemas/scenario-manifest-v1.schema.json` and `schemas/trial-result-v1.schema.json`
//! at the workspace root, deterministically, from the current Rust types.
//!
//! Run from anywhere with: `cargo run -p limen-core --example generate_schemas`
//!
//! The `tests/schema_freshness.rs` integration test asserts these checked-in files match what
//! this example would produce right now, byte-for-byte; re-run this example and re-review the
//! diff whenever a schema-bearing type changes.

use std::path::PathBuf;

fn schemas_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR is this crate's own directory (crates/limen-core); the shared
    // `schemas/` directory lives two levels up, at the workspace root.
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas")
}

fn main() -> std::io::Result<()> {
    let dir = schemas_dir();
    std::fs::create_dir_all(&dir)?;

    let manifest_path = dir.join("scenario-manifest-v1.schema.json");
    let result_path = dir.join("trial-result-v1.schema.json");

    std::fs::write(
        &manifest_path,
        limen_core::schema_support::scenario_manifest_schema_json(),
    )?;
    std::fs::write(
        &result_path,
        limen_core::schema_support::trial_result_schema_json(),
    )?;

    println!("wrote {}", manifest_path.display());
    println!("wrote {}", result_path.display());
    Ok(())
}
