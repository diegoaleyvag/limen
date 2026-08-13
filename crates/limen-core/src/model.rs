//! Core data model: scenario manifests, the annotation-free strategy-facing projection, and the
//! selection/trace types strategies produce.
//!
//! Every type here derives `Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema` so
//! the JSON shape is stable, documented, and schema-checkable. `#[serde(rename_all =
//! "snake_case")]` is applied consistently so field and enum-variant names never depend on Rust
//! identifier casing. Per the crate-wide determinism rules: no `f32`/`f64`, no `usize`, and no
//! `HashMap`/`HashSet` appear in any type below (byte offsets/token counts/order indices are
//! `u32`; the one true set, `distractor_source_ids`, is a `BTreeSet`).

use std::collections::BTreeSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One source item of context available to a strategy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ContextItem {
    /// Unique within a scenario.
    pub source_id: String,
    /// 0-based original order. Recency/order-based strategies use this directly; nothing in
    /// `limen-core` ever parses dates/timestamps out of `text` to infer recency.
    pub order_index: u32,
    /// Strategy-visible structural hint, e.g. `"log_line"`, `"chat_message"`, `"spec_section"`.
    /// Must never reveal evaluator intent (which facts matter, which items are distractors, etc).
    pub section_label: String,
    pub text: String,
}

/// A requested token budget, in `limen-lex@1.0.0` tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Budget {
    pub requested_tokens: u32,
}

/// How much of a [`Budget`] was actually used.
///
/// The invariant `used_tokens <= requested_tokens` (and therefore
/// `remaining_tokens == requested_tokens - used_tokens` without underflow) must always hold.
/// Construct instances through [`BudgetUsage::new`], which clamps `used_tokens` down to
/// `requested_tokens` rather than ever panicking or producing an underflowed `remaining_tokens`
/// (per the crate-wide "no panics escaping a public function" rule) -- a strategy implementation
/// that ever hits the clamp has a bug, but callers of this type must not be able to crash because
/// of it. Directly constructing the struct literal (its fields are `pub`, e.g. for
/// deserialization) bypasses this guarantee; prefer `::new` wherever a value is being computed
/// rather than deserialized from an already-trusted artifact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct BudgetUsage {
    pub requested_tokens: u32,
    pub used_tokens: u32,
    pub remaining_tokens: u32,
}

impl BudgetUsage {
    /// Builds a `BudgetUsage`, clamping `used_tokens` to never exceed `requested_tokens` so
    /// `remaining_tokens` can never underflow.
    pub fn new(requested_tokens: u32, used_tokens: u32) -> Self {
        let used_tokens = used_tokens.min(requested_tokens);
        Self {
            requested_tokens,
            used_tokens,
            remaining_tokens: requested_tokens - used_tokens,
        }
    }
}

/// A byte-span pointer into one source item's `text`, `[byte_start, byte_end)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct EvidenceSpan {
    pub source_id: String,
    pub byte_start: u32,
    pub byte_end: u32,
}

/// A canonical, machine-checkable value backing a [`FactComponent`].
///
/// Numbers and dates are represented as their canonical *textual* form (never as `f32`/`f64`) so
/// comparisons are exact string/substring checks with no float-precision or locale ambiguity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalValue {
    /// Exact textual canonical form of a number, e.g. `"42"` or `"3.5"`, with an optional unit
    /// string, e.g. `"ms"`, that must also survive verbatim for the value to count as retained.
    Number {
        normalized: String,
        unit: Option<String>,
    },
    /// Canonical calendar form, e.g. `"2024-01-05"`. Produced by simple string parsing only --
    /// never locale-aware parsing or `Date`/timezone APIs -- since scenario authors write the
    /// canonical form directly rather than deriving it at validation time.
    Date { normalized: String },
    /// Canonical form of a free-text value that still needs exact-substring verification.
    Text { normalized: String },
}

/// One atomic, independently-checkable piece of an [`ExpectedFact`].
///
/// A composite fact (`components.len() > 1`) requires **all** of its components to be retained.
/// Within a single component, `evidence` is a list of *redundant* alternatives: any one surviving
/// alternative is sufficient for that component to count as retained.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct FactComponent {
    pub component_id: String,
    /// One-of-many redundant evidence locations; any surviving alternative satisfies this
    /// component.
    pub evidence: Vec<EvidenceSpan>,
    pub canonical_value: Option<CanonicalValue>,
    /// Exact substrings (e.g. a unit, a negation word, a qualifier) that must remain present
    /// verbatim in the corresponding output for this component to count as non-distorted.
    pub required_qualifiers: Vec<String>,
}

/// One fact a scenario expects a good strategy to preserve.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ExpectedFact {
    pub fact_id: String,
    pub statement: String,
    pub why_it_matters: String,
    /// `> 1` components makes this a composite fact; **all** components must be satisfied for
    /// the fact to be "retained".
    pub components: Vec<FactComponent>,
    pub expected_citation_source_ids: Vec<String>,
}

/// A set of mutually competing claim locations. Purely structural: never labels any member as
/// "true"; metrics over a contradiction group measure only which members survived selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ContradictionGroup {
    pub group_id: String,
    pub members: Vec<EvidenceSpan>,
}

/// Evaluator-only annotations for a scenario. Never exposed to a [`SelectionStrategy`] (see
/// [`StrategyInput`], which structurally cannot carry these fields).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ScenarioAnnotations {
    pub required_facts: Vec<ExpectedFact>,
    pub distractor_source_ids: BTreeSet<String>,
    pub contradiction_groups: Vec<ContradictionGroup>,
}

/// A complete, versioned scenario: ordered source items plus evaluator-only annotations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ScenarioManifest {
    /// Manifest schema version, e.g. `"1.0.0"` (`MAJOR.MINOR.PATCH`, numeric parts only).
    pub schema_version: String,
    pub scenario_id: String,
    /// Scenario content version, e.g. `"1.0.0"` (`MAJOR.MINOR.PATCH`, numeric parts only).
    pub scenario_version: String,
    pub title: String,
    pub task_query: String,
    /// Ordered source items.
    pub items: Vec<ContextItem>,
    pub annotations: ScenarioAnnotations,
    /// `"sha256:<hex>"` digest of the canonical manifest with this field blanked. Computed and
    /// verified via `crate::canonical::digest_with_field_blanked(manifest, "content_digest")`.
    pub content_digest: String,
}

impl ScenarioManifest {
    /// Projects this manifest down to the annotation-free view a [`SelectionStrategy`] receives.
    pub fn to_strategy_input(&self) -> StrategyInput {
        StrategyInput {
            task_query: self.task_query.clone(),
            items: self.items.clone(),
        }
    }
}

/// The strategy-visible-only projection of a scenario. Structurally cannot carry annotation
/// fields (expected facts, distractors, contradiction groups): they simply are not present on
/// this type, so a `SelectionStrategy` implementation has no field path to reach them even by
/// accident.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct StrategyInput {
    pub task_query: String,
    pub items: Vec<ContextItem>,
}

/// Whether a source item survived a strategy's selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SelectionStatus {
    Included,
    Partial,
    Dropped,
}

/// What happened to one original source item under a strategy's selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ItemSelectionRecord {
    pub source_id: String,
    pub order_index: u32,
    pub status: SelectionStatus,
    /// For `Included`/`Partial` items, the byte range of the *original source text* that was
    /// retained (for `Included`, this is always `[0, text.len())`). For transform strategies this
    /// doubles as a provenance pointer into the source even when `output_text` differs from the
    /// verbatim slice. `None` for `Dropped` items.
    pub included_byte_start: Option<u32>,
    pub included_byte_end: Option<u32>,
    /// Present only for transform strategies (structured-extraction, hierarchical-summary) whose
    /// emitted text differs from the verbatim source slice. When `Some`, metrics and budget
    /// accounting treat this text -- not the verbatim slice -- as "the output" for this item.
    pub output_text: Option<String>,
}

/// One step of a strategy's deterministic decision trace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct TraceStep {
    pub step_index: u32,
    pub source_id: String,
    /// Stable short code, e.g. `"considered"`, `"included_full"`, `"included_partial"`,
    /// `"dropped_over_budget"`, `"extracted"`, `"templated"`.
    pub action: String,
    /// Integer score if the strategy is score-based; `None` otherwise.
    pub score: Option<u32>,
    pub detail: String,
}

/// The full output of running one [`SelectionStrategy`] against a [`StrategyInput`] and
/// [`Budget`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct SelectionOutput {
    /// Exact registry id, e.g. `"recency@1"`.
    pub strategy_id: String,
    pub budget: BudgetUsage,
    pub selection: Vec<ItemSelectionRecord>,
    pub trace: Vec<TraceStep>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_usage_never_exceeds_requested() {
        let usage = BudgetUsage::new(10, 25);
        assert_eq!(usage.requested_tokens, 10);
        assert_eq!(
            usage.used_tokens, 10,
            "used_tokens must be clamped to requested_tokens"
        );
        assert_eq!(
            usage.remaining_tokens, 0,
            "remaining_tokens must never underflow"
        );
    }

    #[test]
    fn budget_usage_normal_case() {
        let usage = BudgetUsage::new(100, 40);
        assert_eq!(usage.used_tokens, 40);
        assert_eq!(usage.remaining_tokens, 60);
    }

    #[test]
    fn budget_usage_exact_boundary() {
        let usage = BudgetUsage::new(50, 50);
        assert_eq!(usage.used_tokens, 50);
        assert_eq!(usage.remaining_tokens, 0);
    }

    #[test]
    fn strategy_input_projection_carries_only_query_and_items() {
        let manifest = ScenarioManifest {
            schema_version: "1.0.0".to_string(),
            scenario_id: "s1".to_string(),
            scenario_version: "1.0.0".to_string(),
            title: "Title".to_string(),
            task_query: "What happened?".to_string(),
            items: vec![ContextItem {
                source_id: "a".to_string(),
                order_index: 0,
                section_label: "log_line".to_string(),
                text: "hello".to_string(),
            }],
            annotations: ScenarioAnnotations {
                required_facts: vec![],
                distractor_source_ids: BTreeSet::new(),
                contradiction_groups: vec![],
            },
            content_digest: "sha256:placeholder".to_string(),
        };

        let input = manifest.to_strategy_input();
        assert_eq!(input.task_query, "What happened?");
        assert_eq!(input.items.len(), 1);
        assert_eq!(input.items[0].source_id, "a");
    }
}
