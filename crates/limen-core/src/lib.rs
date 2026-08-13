//! `limen-core`: the deterministic foundation contract for Limen.
//!
//! This crate defines the scenario manifest model, the bundled `limen-lex@1.0.0` tokenizer,
//! canonical-JSON/SHA-256 digesting, manifest validation, the versioned selection-strategy
//! trait/registry (with five deterministic placeholder implementations), and the full metrics
//! computation. See each module's doc comment for details; see `crate::canonical` in particular
//! for the crate-wide determinism conventions (canonical JSON, digesting, no silent Unicode
//! normalization).
//!
//! Determinism rules that apply everywhere in this crate (see individual modules for the
//! specifics): no `HashMap`/`HashSet` in anything that affects decisions, ordering, or
//! serialized/hashed output; no `f32`/`f64` in any serialized/hashed/decision-affecting type; no
//! `usize` in any serialized/hashed type (byte offsets/token counts/order indices are `u32`); no
//! wall-clock reads, RNG, UUIDs, or parallelism in any decision or artifact-construction path; no
//! panics escaping a public function (fallible public functions return `Result<T, EngineError>`);
//! every ordering/sort is a fully specified total order with a documented, unique-key tie-break.

pub mod canonical;
pub mod catalog;
pub mod error;
pub mod fixtures;
pub mod golden_support;
pub mod metrics;
pub mod model;
pub mod result;
pub mod schema_support;
pub mod strategy;
pub mod tokenizer;
pub mod validate;

// Convenient re-exports of the most commonly used types, so downstream crates can write
// `limen_core::ScenarioManifest` instead of `limen_core::model::ScenarioManifest`.
pub use catalog::{all_scenario_ids, get_scenario, list_scenario_summaries, ScenarioSummary};
pub use error::{EngineError, ValidationError};
pub use metrics::{compute_metrics, Metrics};
pub use model::{
    Budget, BudgetUsage, CanonicalValue, ContextItem, ContradictionGroup, EvidenceSpan,
    ExpectedFact, FactComponent, ItemSelectionRecord, ScenarioAnnotations, ScenarioManifest,
    SelectionOutput, SelectionStatus, StrategyInput, TraceStep,
};
pub use result::{run_trial, TrialResult};
pub use strategy::{
    list_strategy_descriptors, list_strategy_ids, resolve_strategy, SelectionStrategy,
    StrategyDescriptor,
};
pub use tokenizer::{count_tokens, tokenize, Token, TOKENIZER_ID};
pub use validate::validate_manifest;

/// This crate's own package version, as recorded in `Cargo.toml`. This is the single source of
/// truth for `TrialResult::engine_version`; the WASM adapter re-exports this same constant rather
/// than reading its own (different) package version.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Manifest/result schema version emitted by this phase. `MAJOR.MINOR.PATCH`, matching the
/// `is_semver_triplet` check in `validate.rs`.
pub const SCHEMA_VERSION: &str = "1.0.0";
