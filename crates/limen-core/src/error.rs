//! Stable, serializable error types for `limen-core`.
//!
//! Two error surfaces exist and are kept deliberately distinct:
//!
//! - [`ValidationError`]: one *problem* found while validating a [`crate::model::ScenarioManifest`].
//!   `validate_manifest` collects every problem it finds into a `Vec<ValidationError>` rather than
//!   stopping at the first one.
//! - [`EngineError`]: a runtime failure that is not about scenario content, e.g. an unknown
//!   strategy id/version, or a batch of validation errors surfaced through the engine boundary.
//!
//! Both types derive [`serde::Serialize`] (so they can cross the WASM boundary as plain JSON) but
//! deliberately do **not** derive `Deserialize`: `ValidationError::code` is a `&'static str` (per
//! the spec), which cannot be produced generically by a deserializer, and neither type is ever
//! meant to be reconstructed from untrusted JSON inside this crate. Errors flow one direction:
//! Rust -> JSON.
//!
//! No `Display`/`Debug` implementation in this module ever embeds pointer/address text or other
//! platform-variant debug output; every message is built from plain owned data (ids, counts,
//! strings already present in the manifest).

use serde::Serialize;

/// A single validation problem found in a [`crate::model::ScenarioManifest`].
///
/// `code` is a stable, machine-matchable identifier (snake_case, never changes meaning once
/// shipped). `message` is a human-readable explanation. `path` is an optional, best-effort
/// locator (dot/bracket notation, e.g. `"annotations.required_facts[2].components[0]"`) pointing
/// at the offending value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ValidationError {
    pub code: &'static str,
    pub message: String,
    pub path: Option<String>,
}

impl ValidationError {
    pub fn new(code: &'static str, message: impl Into<String>, path: Option<String>) -> Self {
        Self {
            code,
            message: message.into(),
            path,
        }
    }

    /// Convenience constructor for the common case of a single dotted/bracketed path.
    pub fn at(code: &'static str, message: impl Into<String>, path: impl Into<String>) -> Self {
        Self::new(code, message, Some(path.into()))
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.path {
            Some(path) => write!(f, "{} ({}): {}", self.code, path, self.message),
            None => write!(f, "{}: {}", self.code, self.message),
        }
    }
}

impl std::error::Error for ValidationError {}

/// Runtime failures raised by `limen-core` that are not scenario-content validation problems.
///
/// This is intentionally the single error currency for the crate's fallible public functions
/// (besides `validate_manifest`, which returns `Vec<ValidationError>` directly since it always
/// collects rather than short-circuits). New variants may be added in later phases; treat this
/// enum as open (non-exhaustive matches recommended in downstream code).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "error", content = "detail")]
pub enum EngineError {
    /// The requested strategy id does not exactly match any entry in the strategy registry
    /// (unknown id, or a known family with an unsupported/unknown version suffix, e.g. `@2`).
    UnknownStrategyVersion(String),
    /// The requested scenario id does not exactly match any entry in the embedded scenario
    /// catalog ([`crate::catalog`]). A distinct failure path from
    /// [`UnknownStrategyVersion`][EngineError::UnknownStrategyVersion]: this is about *which
    /// scenario* was requested, not which strategy.
    UnknownScenarioId(String),
    /// The requested budget is below the minimum the engine can meaningfully honor.
    ///
    /// Not raised anywhere in this phase (no strategy currently enforces a minimum), but kept as
    /// a stable, documented variant since a later strategy-implementation phase may need it (for
    /// example a strategy whose template wrapper alone costs more tokens than the budget allows).
    BudgetBelowMinimum,
    /// One or more [`ValidationError`]s were found; carries the full collected list.
    Validation(Vec<ValidationError>),
    /// Canonicalization (`to_value`/serialization) of an in-memory value failed. In practice this
    /// is unreachable for the plain-data types in this crate (no NaN/Infinity floats are ever
    /// modeled, no non-string map keys), but the path is kept as a `Result` rather than a panic.
    Canonicalization(String),
    /// A `BudgetUsage` was asked to represent `used_tokens > requested_tokens`. Not raised by
    /// `BudgetUsage::new` itself (which clamps instead of failing, per the no-panics rule), but
    /// kept as a stable variant for any future fallible construction path that should reject
    /// rather than silently clamp.
    UsedTokensExceedRequested {
        requested_tokens: u32,
        used_tokens: u32,
    },
}

impl EngineError {
    /// Stable, machine-matchable code for this error. Never changes meaning once shipped.
    pub fn code(&self) -> &'static str {
        match self {
            EngineError::UnknownStrategyVersion(_) => "unknown_strategy_version",
            EngineError::UnknownScenarioId(_) => "unknown_scenario_id",
            EngineError::BudgetBelowMinimum => "budget_below_minimum",
            EngineError::Validation(_) => "validation_failed",
            EngineError::Canonicalization(_) => "canonicalization_failed",
            EngineError::UsedTokensExceedRequested { .. } => "used_tokens_exceed_requested",
        }
    }
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineError::UnknownStrategyVersion(id) => {
                write!(f, "unknown strategy id or version: {id}")
            }
            EngineError::UnknownScenarioId(id) => {
                write!(f, "unknown scenario id: {id}")
            }
            EngineError::BudgetBelowMinimum => {
                write!(
                    f,
                    "requested budget is below the minimum allowed token count"
                )
            }
            EngineError::Validation(errors) => {
                write!(
                    f,
                    "manifest failed validation with {} error(s)",
                    errors.len()
                )
            }
            EngineError::Canonicalization(message) => {
                write!(f, "failed to canonicalize value: {message}")
            }
            EngineError::UsedTokensExceedRequested {
                requested_tokens,
                used_tokens,
            } => write!(
                f,
                "used tokens ({used_tokens}) exceed requested tokens ({requested_tokens})"
            ),
        }
    }
}

impl std::error::Error for EngineError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_error_display_includes_code_and_path() {
        let err = ValidationError::at(
            "duplicate_source_id",
            "source_id 'a' appears twice",
            "items[3]",
        );
        let text = err.to_string();
        assert!(text.contains("duplicate_source_id"));
        assert!(text.contains("items[3]"));
        assert!(text.contains("appears twice"));
    }

    #[test]
    fn validation_error_display_without_path() {
        let err = ValidationError::new("empty_required_facts", "no required facts", None);
        assert_eq!(err.to_string(), "empty_required_facts: no required facts");
    }

    #[test]
    fn engine_error_codes_are_stable_and_distinct() {
        let variants = [
            EngineError::UnknownStrategyVersion("recency@2".to_string()),
            EngineError::UnknownScenarioId("no-such-scenario".to_string()),
            EngineError::BudgetBelowMinimum,
            EngineError::Validation(vec![]),
            EngineError::Canonicalization("boom".to_string()),
            EngineError::UsedTokensExceedRequested {
                requested_tokens: 10,
                used_tokens: 11,
            },
        ];
        let codes: Vec<&str> = variants.iter().map(EngineError::code).collect();
        let mut sorted = codes.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(
            sorted.len(),
            codes.len(),
            "expected all error codes to be distinct"
        );
    }

    #[test]
    fn engine_error_display_has_no_pointer_like_text() {
        let err = EngineError::UnknownStrategyVersion("foo@9".to_string());
        let text = err.to_string();
        assert!(!text.contains("0x"));
    }

    #[test]
    fn engine_error_serializes_as_tagged_json() {
        let err = EngineError::UnknownStrategyVersion("foo@9".to_string());
        let value = serde_json::to_value(&err).expect("serialize");
        assert_eq!(value["error"], "unknown_strategy_version");
        assert_eq!(value["detail"], "foo@9");
    }

    #[test]
    fn unknown_scenario_id_serializes_as_tagged_json() {
        let err = EngineError::UnknownScenarioId("ghost-scenario".to_string());
        let value = serde_json::to_value(&err).expect("serialize");
        assert_eq!(value["error"], "unknown_scenario_id");
        assert_eq!(value["detail"], "ghost-scenario");
        assert_eq!(err.code(), "unknown_scenario_id");
    }
}
