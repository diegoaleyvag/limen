//! Deterministic, annotation-relative metrics computed from a [`SelectionOutput`] plus the
//! [`ScenarioAnnotations`] the strategy never saw.
//!
//! This module depends only on [`SelectionOutput`] and [`ScenarioAnnotations`] (plus the
//! original items, needed to slice verbatim text) -- never on which concrete strategy produced
//! the output -- so it is complete and final in this phase; later strategy-implementation work
//! must not need to change anything here.
//!
//! # Core retention rule
//!
//! A [`FactComponent`] is retained iff **at least one** of its `evidence` alternatives is
//! spatially retained (its source item is fully `Included`, or the evidence span lies entirely
//! inside a `Partial` item's retained sub-range) **and**, if `canonical_value`/
//! `required_qualifiers` are set, the exact normalized value/unit/qualifier text is present in
//! "the corresponding output" for that alternative -- the verbatim retained slice for ordinary
//! (non-transform) records, or `output_text` for transform records (any record with
//! `output_text.is_some()`; see [`crate::model::ItemSelectionRecord::output_text`]).
//!
//! An [`ExpectedFact`] is retained iff **all** of its components are retained (composite-AND
//! semantics); a single-component fact is retained iff that one component is retained.
//!
//! # Contradiction outcomes: `Split` vs `PartialWithinRetained`
//!
//! Both describe "some members retained, some dropped", but they answer different questions:
//! *why* was the dropped member actually dropped? Every strategy's [`crate::model::TraceStep`]
//! for a dropped item already names a specific `action`, and that action is either a
//! budget-cutoff reason (the item was dropped **because the budget ran out**: recognized exactly
//! by [`is_budget_drop_action`], e.g. `"dropped_over_budget"`/`"dropped_too_old"`/
//! `"dropped_below_budget"`) or a content reason unrelated to the budget boundary (currently only
//! `structured-extraction@1`'s `"dropped_no_extractable_content"`, emitted when an item simply has
//! nothing digit-bearing to extract, regardless of budget). A mixed group is `Split` iff **at
//! least one** of its dropped members was dropped for a budget reason -- i.e. at least one
//! competing claim's survival was genuinely decided by where the budget cutoff fell, even if that
//! member is not the single global extreme dropped/retained item. A mixed group is
//! `PartialWithinRetained` iff **every** dropped member was dropped for a non-budget reason (so
//! nothing about the group's outcome was actually decided by the budget boundary).
//!
//! This definition is deliberately *not* geometric (no comparison against a global
//! `order_index` boundary): it generalizes correctly to every strategy, including
//! `retrieval-ranking@1`'s non-contiguous bin-packing, where there is no single geometric cutoff
//! point in `order_index` space at all -- a lower-ranked item can survive after a higher-ranked
//! one was skipped, so "the item nearest the boundary" is not even a well-defined concept there.
//! It is still a precise, testable, purely structural rule -- never a truth judgement about which
//! claim is correct.
//!
//! # Citation retention is per-fact, not per-source
//!
//! Only facts with a non-empty `expected_citation_source_ids` are counted (in both the `expected`
//! denominator and `per_fact`): a fact with no citation expectation has nothing to check, and
//! counting it would artificially inflate retention. A counted fact's citation is "retained" iff
//! **every** one of its `expected_citation_source_ids` has a non-`Dropped` status in the
//! selection (citation retention is about the cited *source* still being present/attributable,
//! independent of whether the fact's own byte-level evidence survived).
//!
//! # `distortion_indicators` ordering
//!
//! The flat `Vec<DistortionIndicator>` is assembled in one fully specified order: for each
//! `required_facts` entry, in manifest order, first an `EvidenceClipped` (if any evidence for
//! that fact was clipped by a `Partial` cut) then any `QualifierDropped` entries (by component,
//! then by qualifier, in authored order); then all `ProvenanceReordered` entries, in selection
//! (output) order; then all `ContradictionSplit` entries, in `contradiction_groups` order; then
//! all `DistractorRetained` entries, in `distractor_source_ids` sorted order (it is a
//! `BTreeSet`).

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::model::{
    BudgetUsage, CanonicalValue, ContextItem, EvidenceSpan, FactComponent, ItemSelectionRecord,
    ScenarioAnnotations, SelectionOutput, SelectionStatus,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct FactRecallDetail {
    pub fact_id: String,
    pub retained: bool,
    pub missing_components: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct FactRecall {
    pub retained: u32,
    pub required: u32,
    pub per_fact: Vec<FactRecallDetail>,
}

/// Purely structural outcome for one [`crate::model::ContradictionGroup`]. Never a truth
/// judgement -- see the module doc comment for the precise `Split` vs `PartialWithinRetained`
/// rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContradictionOutcome {
    AllRetained,
    Split,
    PartialWithinRetained,
    NoneRetained,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ContradictionResult {
    pub group_id: String,
    pub outcome: ContradictionOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct NumericDateAccuracy {
    pub exact: u32,
    pub checked: u32,
    /// `fact_id`s of every checked `Number`/`Date` component that was not exactly retained. May
    /// contain repeats if a single fact has multiple mismatching numeric/date components.
    pub mismatches: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct CitationRetention {
    pub retained: u32,
    pub expected: u32,
    pub per_fact: Vec<(String, bool)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DistortionIndicator {
    EvidenceClipped { fact_id: String },
    QualifierDropped { fact_id: String, qualifier: String },
    ProvenanceReordered { source_id: String },
    ContradictionSplit { group_id: String },
    DistractorRetained { source_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Metrics {
    pub fact_recall: FactRecall,
    pub contradictions: Vec<ContradictionResult>,
    pub numeric_date_accuracy: NumericDateAccuracy,
    pub citation_retention: CitationRetention,
    pub budget: BudgetUsage,
    pub distortion_indicators: Vec<DistortionIndicator>,
}

/// Returns `true` iff `evidence` is spatially retained: its source item is `Included`, or it lies
/// entirely inside a `Partial` item's retained sub-range. Ignores `canonical_value`/
/// `required_qualifiers` entirely -- this is the spatial gate only.
fn evidence_spatially_retained(
    evidence: &EvidenceSpan,
    records_by_source: &BTreeMap<&str, &ItemSelectionRecord>,
) -> bool {
    match records_by_source.get(evidence.source_id.as_str()) {
        None => false,
        Some(record) => match record.status {
            SelectionStatus::Dropped => false,
            SelectionStatus::Included => true,
            SelectionStatus::Partial => {
                match (record.included_byte_start, record.included_byte_end) {
                    (Some(start), Some(end)) => {
                        evidence.byte_start >= start && evidence.byte_end <= end
                    }
                    _ => false,
                }
            }
        },
    }
}

/// If `evidence` is spatially retained, returns the text to check `canonical_value`/
/// `required_qualifiers` substrings against: `output_text` for transform records, otherwise the
/// verbatim retained slice of the original source text. Returns `None` if not spatially retained.
fn corresponding_output_text<'a>(
    evidence: &EvidenceSpan,
    records_by_source: &BTreeMap<&str, &'a ItemSelectionRecord>,
    items_by_source: &BTreeMap<&str, &'a ContextItem>,
) -> Option<&'a str> {
    if !evidence_spatially_retained(evidence, records_by_source) {
        return None;
    }
    let record = records_by_source.get(evidence.source_id.as_str())?;
    if let Some(output_text) = record.output_text.as_deref() {
        return Some(output_text);
    }
    let item = items_by_source.get(evidence.source_id.as_str())?;
    match record.status {
        SelectionStatus::Included => Some(item.text.as_str()),
        SelectionStatus::Partial => {
            let start = record.included_byte_start.unwrap_or(0) as usize;
            let end = record.included_byte_end.unwrap_or(0) as usize;
            item.text.get(start..end)
        }
        SelectionStatus::Dropped => None,
    }
}

/// Whether `component`'s `canonical_value` (if any) and every `required_qualifiers` entry are
/// present verbatim in `output_text`.
/// Known limitation: every check below is a plain substring search (`str::contains`), not a
/// word-boundary-aware match, so a numeric value could in principle false-positive against a
/// longer number that merely contains it as a substring (e.g. `"420"` would match inside
/// `"1420"`). Not fixed here: a word-boundary rewrite is out of scope for this pass unless it can
/// be proven to change none of the checked-in golden fixtures, which was not attempted.
fn value_and_qualifiers_satisfied(component: &FactComponent, output_text: &str) -> bool {
    let value_ok = match &component.canonical_value {
        None => true,
        Some(CanonicalValue::Number { normalized, unit }) => {
            output_text.contains(normalized.as_str())
                && unit.as_deref().is_none_or(|u| output_text.contains(u))
        }
        Some(CanonicalValue::Date { normalized }) => output_text.contains(normalized.as_str()),
        Some(CanonicalValue::Text { normalized }) => output_text.contains(normalized.as_str()),
    };
    value_ok
        && component
            .required_qualifiers
            .iter()
            .all(|q| output_text.contains(q.as_str()))
}

/// The output text of the first evidence alternative that is both spatially retained and passes
/// [`value_and_qualifiers_satisfied`] -- i.e. the component counts as retained iff this is `Some`.
fn component_retained_output<'a>(
    component: &FactComponent,
    records_by_source: &BTreeMap<&str, &'a ItemSelectionRecord>,
    items_by_source: &BTreeMap<&str, &'a ContextItem>,
) -> Option<&'a str> {
    component.evidence.iter().find_map(|evidence| {
        let text = corresponding_output_text(evidence, records_by_source, items_by_source)?;
        value_and_qualifiers_satisfied(component, text).then_some(text)
    })
}

/// The output text of the first evidence alternative that is spatially retained, regardless of
/// whether the value/qualifier check passes. Used only for `QualifierDropped` diagnostics: it
/// lets us report *which* qualifier is missing from otherwise-present text.
fn first_spatially_retained_output<'a>(
    component: &FactComponent,
    records_by_source: &BTreeMap<&str, &'a ItemSelectionRecord>,
    items_by_source: &BTreeMap<&str, &'a ContextItem>,
) -> Option<&'a str> {
    component.evidence.iter().find_map(|evidence| {
        corresponding_output_text(evidence, records_by_source, items_by_source)
    })
}

/// The closed set of `TraceStep::action` codes that mean "dropped because the budget ran out,"
/// across every registered strategy: `"dropped_over_budget"` (full-input-truncation,
/// structured-extraction, hierarchical-summary), `"dropped_too_old"` (recency), and
/// `"dropped_below_budget"` (retrieval-ranking). Deliberately a fixed array, not a `HashMap` --
/// this is a small, closed, rarely-changing set, and membership order/iteration semantics must
/// never depend on hashing. Any other action (currently only
/// `"dropped_no_extractable_content"`, structured-extraction's content-level drop) is a
/// *non*-budget reason.
const BUDGET_DROP_ACTIONS: [&str; 3] = [
    "dropped_over_budget",
    "dropped_too_old",
    "dropped_below_budget",
];

/// `true` iff `action` is one of [`BUDGET_DROP_ACTIONS`] -- i.e. the item was dropped because the
/// budget ran out, as opposed to a content-level reason unrelated to the budget boundary.
fn is_budget_drop_action(action: &str) -> bool {
    BUDGET_DROP_ACTIONS.contains(&action)
}

/// `true` iff `evidence`'s source item is `Partial` and `evidence` is not entirely contained in
/// that item's retained sub-range (a `Dropped` item is "absent", not "clipped"; an `Included`
/// item can never clip anything).
fn evidence_is_clipped(
    evidence: &EvidenceSpan,
    records_by_source: &BTreeMap<&str, &ItemSelectionRecord>,
) -> bool {
    match records_by_source.get(evidence.source_id.as_str()) {
        Some(record) if record.status == SelectionStatus::Partial => {
            match (record.included_byte_start, record.included_byte_end) {
                (Some(start), Some(end)) => evidence.byte_start < start || evidence.byte_end > end,
                _ => true,
            }
        }
        _ => false,
    }
}

pub fn compute_metrics(
    annotations: &ScenarioAnnotations,
    selection: &SelectionOutput,
    original_items: &[ContextItem],
) -> Metrics {
    let records_by_source: BTreeMap<&str, &ItemSelectionRecord> = selection
        .selection
        .iter()
        .map(|r| (r.source_id.as_str(), r))
        .collect();
    let items_by_source: BTreeMap<&str, &ContextItem> = original_items
        .iter()
        .map(|i| (i.source_id.as_str(), i))
        .collect();

    let mut per_fact_recall = Vec::with_capacity(annotations.required_facts.len());
    let mut retained_fact_count: u32 = 0;
    let mut checked_numeric_date: u32 = 0;
    let mut exact_numeric_date: u32 = 0;
    let mut numeric_date_mismatches = Vec::new();
    let mut distortions: Vec<DistortionIndicator> = Vec::new();

    for fact in &annotations.required_facts {
        let mut missing_components = Vec::new();

        for component in &fact.components {
            let retained_output =
                component_retained_output(component, &records_by_source, &items_by_source);
            let is_retained = retained_output.is_some();
            if !is_retained {
                missing_components.push(component.component_id.clone());
            }

            if let Some(canonical) = &component.canonical_value {
                let is_numeric_or_date = matches!(
                    canonical,
                    CanonicalValue::Number { .. } | CanonicalValue::Date { .. }
                );
                if is_numeric_or_date {
                    checked_numeric_date += 1;
                    if is_retained {
                        exact_numeric_date += 1;
                    } else {
                        numeric_date_mismatches.push(fact.fact_id.clone());
                    }
                }
            }

            if !is_retained && !component.required_qualifiers.is_empty() {
                if let Some(spatial_output) =
                    first_spatially_retained_output(component, &records_by_source, &items_by_source)
                {
                    for qualifier in &component.required_qualifiers {
                        if !spatial_output.contains(qualifier.as_str()) {
                            distortions.push(DistortionIndicator::QualifierDropped {
                                fact_id: fact.fact_id.clone(),
                                qualifier: qualifier.clone(),
                            });
                        }
                    }
                }
            }
        }

        let fact_retained = missing_components.is_empty() && !fact.components.is_empty();
        if fact_retained {
            retained_fact_count += 1;
        }

        let evidence_clipped_for_fact = fact
            .components
            .iter()
            .flat_map(|c| c.evidence.iter())
            .any(|evidence| evidence_is_clipped(evidence, &records_by_source));
        if evidence_clipped_for_fact {
            distortions.push(DistortionIndicator::EvidenceClipped {
                fact_id: fact.fact_id.clone(),
            });
        }

        per_fact_recall.push(FactRecallDetail {
            fact_id: fact.fact_id.clone(),
            retained: fact_retained,
            missing_components,
        });
    }

    let fact_recall = FactRecall {
        retained: retained_fact_count,
        required: annotations.required_facts.len() as u32,
        per_fact: per_fact_recall,
    };
    let numeric_date_accuracy = NumericDateAccuracy {
        exact: exact_numeric_date,
        checked: checked_numeric_date,
        mismatches: numeric_date_mismatches,
    };

    // Citation retention: per-fact, counting only facts with a non-empty citation expectation.
    let mut citation_retained: u32 = 0;
    let mut citation_expected: u32 = 0;
    let mut citation_per_fact = Vec::new();
    for fact in &annotations.required_facts {
        if fact.expected_citation_source_ids.is_empty() {
            continue;
        }
        citation_expected += 1;
        let all_present = fact.expected_citation_source_ids.iter().all(|source_id| {
            records_by_source
                .get(source_id.as_str())
                .map(|r| r.status != SelectionStatus::Dropped)
                .unwrap_or(false)
        });
        if all_present {
            citation_retained += 1;
        }
        citation_per_fact.push((fact.fact_id.clone(), all_present));
    }
    let citation_retention = CitationRetention {
        retained: citation_retained,
        expected: citation_expected,
        per_fact: citation_per_fact,
    };

    // Contradictions, classified by *why* each dropped member was dropped (see the module doc
    // comment): a mixed group is `Split` iff at least one dropped member's trace action is a
    // budget-cutoff reason.
    let trace_action_by_source: BTreeMap<&str, &str> = selection
        .trace
        .iter()
        .map(|t| (t.source_id.as_str(), t.action.as_str()))
        .collect();

    let mut contradictions = Vec::with_capacity(annotations.contradiction_groups.len());
    for group in &annotations.contradiction_groups {
        let member_retained: Vec<bool> = group
            .members
            .iter()
            .map(|m| evidence_spatially_retained(m, &records_by_source))
            .collect();
        let total = member_retained.len();
        let retained_n = member_retained.iter().filter(|&&r| r).count();

        let outcome = if total == 0 || retained_n == total {
            ContradictionOutcome::AllRetained
        } else if retained_n == 0 {
            ContradictionOutcome::NoneRetained
        } else {
            let any_dropped_member_hit_the_budget_cutoff = group
                .members
                .iter()
                .zip(member_retained.iter())
                .any(|(m, &retained)| {
                    !retained
                        && trace_action_by_source
                            .get(m.source_id.as_str())
                            .is_some_and(|action| is_budget_drop_action(action))
                });
            if any_dropped_member_hit_the_budget_cutoff {
                ContradictionOutcome::Split
            } else {
                ContradictionOutcome::PartialWithinRetained
            }
        };

        if outcome == ContradictionOutcome::Split {
            distortions.push(DistortionIndicator::ContradictionSplit {
                group_id: group.group_id.clone(),
            });
        }
        contradictions.push(ContradictionResult {
            group_id: group.group_id.clone(),
            outcome,
        });
    }

    // Provenance reordering: walk the selection in its own (output) order and flag any retained
    // item whose order_index is lower than the highest order_index seen so far among retained
    // items -- i.e. it appears "out of place" relative to original order.
    let mut max_order_index_seen: Option<u32> = None;
    for record in &selection.selection {
        if record.status == SelectionStatus::Dropped {
            continue;
        }
        if let Some(max_seen) = max_order_index_seen {
            if record.order_index < max_seen {
                distortions.push(DistortionIndicator::ProvenanceReordered {
                    source_id: record.source_id.clone(),
                });
            }
        }
        max_order_index_seen =
            Some(max_order_index_seen.map_or(record.order_index, |m| m.max(record.order_index)));
    }

    // Distractors, in BTreeSet (sorted) order.
    for source_id in &annotations.distractor_source_ids {
        let retained = records_by_source
            .get(source_id.as_str())
            .map(|r| r.status != SelectionStatus::Dropped)
            .unwrap_or(false);
        if retained {
            distortions.push(DistortionIndicator::DistractorRetained {
                source_id: source_id.clone(),
            });
        }
    }

    Metrics {
        fact_recall,
        contradictions,
        numeric_date_accuracy,
        citation_retention,
        budget: selection.budget,
        distortion_indicators: distortions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ContradictionGroup, ExpectedFact, TraceStep};
    use std::collections::BTreeSet;

    fn item(source_id: &str, order_index: u32, text: &str) -> ContextItem {
        ContextItem {
            source_id: source_id.to_string(),
            order_index,
            section_label: "log_line".to_string(),
            text: text.to_string(),
        }
    }

    fn included(source_id: &str, order_index: u32, len: u32) -> ItemSelectionRecord {
        ItemSelectionRecord {
            source_id: source_id.to_string(),
            order_index,
            status: SelectionStatus::Included,
            included_byte_start: Some(0),
            included_byte_end: Some(len),
            output_text: None,
        }
    }

    fn partial(source_id: &str, order_index: u32, start: u32, end: u32) -> ItemSelectionRecord {
        ItemSelectionRecord {
            source_id: source_id.to_string(),
            order_index,
            status: SelectionStatus::Partial,
            included_byte_start: Some(start),
            included_byte_end: Some(end),
            output_text: None,
        }
    }

    fn dropped(source_id: &str, order_index: u32) -> ItemSelectionRecord {
        ItemSelectionRecord {
            source_id: source_id.to_string(),
            order_index,
            status: SelectionStatus::Dropped,
            included_byte_start: None,
            included_byte_end: None,
            output_text: None,
        }
    }

    fn selection_output(records: Vec<ItemSelectionRecord>) -> SelectionOutput {
        SelectionOutput {
            strategy_id: "test@1".to_string(),
            budget: BudgetUsage::new(1000, 10),
            selection: records,
            trace: vec![],
        }
    }

    fn trace_step(step_index: u32, source_id: &str, action: &str) -> TraceStep {
        TraceStep {
            step_index,
            source_id: source_id.to_string(),
            action: action.to_string(),
            score: None,
            detail: "test".to_string(),
        }
    }

    fn single_component_fact(
        fact_id: &str,
        component_id: &str,
        evidence: Vec<EvidenceSpan>,
    ) -> ExpectedFact {
        ExpectedFact {
            fact_id: fact_id.to_string(),
            statement: "statement".to_string(),
            why_it_matters: "why".to_string(),
            components: vec![FactComponent {
                component_id: component_id.to_string(),
                evidence,
                canonical_value: None,
                required_qualifiers: vec![],
            }],
            expected_citation_source_ids: vec![],
        }
    }

    fn annotations_with(required_facts: Vec<ExpectedFact>) -> ScenarioAnnotations {
        ScenarioAnnotations {
            required_facts,
            distractor_source_ids: BTreeSet::new(),
            contradiction_groups: vec![],
        }
    }

    #[test]
    fn composite_fact_not_retained_when_one_component_dropped() {
        let items = vec![item("a", 0, "alpha"), item("b", 1, "beta")];
        let fact = ExpectedFact {
            fact_id: "f1".to_string(),
            statement: "s".to_string(),
            why_it_matters: "w".to_string(),
            components: vec![
                FactComponent {
                    component_id: "c1".to_string(),
                    evidence: vec![EvidenceSpan {
                        source_id: "a".to_string(),
                        byte_start: 0,
                        byte_end: 5,
                    }],
                    canonical_value: None,
                    required_qualifiers: vec![],
                },
                FactComponent {
                    component_id: "c2".to_string(),
                    evidence: vec![EvidenceSpan {
                        source_id: "b".to_string(),
                        byte_start: 0,
                        byte_end: 4,
                    }],
                    canonical_value: None,
                    required_qualifiers: vec![],
                },
            ],
            expected_citation_source_ids: vec![],
        };
        let annotations = annotations_with(vec![fact]);
        let selection = selection_output(vec![included("a", 0, 5), dropped("b", 1)]);

        let metrics = compute_metrics(&annotations, &selection, &items);
        assert_eq!(metrics.fact_recall.retained, 0);
        assert_eq!(metrics.fact_recall.required, 1);
        assert!(!metrics.fact_recall.per_fact[0].retained);
        assert_eq!(
            metrics.fact_recall.per_fact[0].missing_components,
            vec!["c2".to_string()]
        );
    }

    #[test]
    fn redundant_evidence_retained_via_second_alternative() {
        let items = vec![item("a", 0, "alpha"), item("b", 1, "alpha again")];
        let fact = single_component_fact(
            "f1",
            "c1",
            vec![
                EvidenceSpan {
                    source_id: "a".to_string(),
                    byte_start: 0,
                    byte_end: 5,
                },
                EvidenceSpan {
                    source_id: "b".to_string(),
                    byte_start: 0,
                    byte_end: 5,
                },
            ],
        );
        let annotations = annotations_with(vec![fact]);
        // First alternative's source is dropped, second's is included -> component still retained.
        let selection = selection_output(vec![dropped("a", 0), included("b", 1, 11)]);

        let metrics = compute_metrics(&annotations, &selection, &items);
        assert_eq!(metrics.fact_recall.retained, 1);
        assert!(metrics.fact_recall.per_fact[0].retained);
        assert!(metrics.fact_recall.per_fact[0]
            .missing_components
            .is_empty());
    }

    #[test]
    fn contradiction_all_retained() {
        let items = vec![item("a", 0, "claim one"), item("b", 1, "claim two")];
        let group = ContradictionGroup {
            group_id: "g1".to_string(),
            members: vec![
                EvidenceSpan {
                    source_id: "a".to_string(),
                    byte_start: 0,
                    byte_end: 9,
                },
                EvidenceSpan {
                    source_id: "b".to_string(),
                    byte_start: 0,
                    byte_end: 9,
                },
            ],
        };
        let mut annotations = annotations_with(vec![single_component_fact(
            "f1",
            "c1",
            vec![EvidenceSpan {
                source_id: "a".to_string(),
                byte_start: 0,
                byte_end: 9,
            }],
        )]);
        annotations.contradiction_groups.push(group);
        let selection = selection_output(vec![included("a", 0, 9), included("b", 1, 9)]);

        let metrics = compute_metrics(&annotations, &selection, &items);
        assert_eq!(metrics.contradictions.len(), 1);
        assert_eq!(
            metrics.contradictions[0].outcome,
            ContradictionOutcome::AllRetained
        );
        assert!(metrics.distortion_indicators.is_empty());
    }

    #[test]
    fn contradiction_none_retained() {
        let items = vec![item("a", 0, "claim one"), item("b", 1, "claim two")];
        let group = ContradictionGroup {
            group_id: "g1".to_string(),
            members: vec![
                EvidenceSpan {
                    source_id: "a".to_string(),
                    byte_start: 0,
                    byte_end: 9,
                },
                EvidenceSpan {
                    source_id: "b".to_string(),
                    byte_start: 0,
                    byte_end: 9,
                },
            ],
        };
        let mut annotations = annotations_with(vec![single_component_fact(
            "f1",
            "c1",
            vec![EvidenceSpan {
                source_id: "a".to_string(),
                byte_start: 0,
                byte_end: 9,
            }],
        )]);
        annotations.contradiction_groups.push(group);
        let selection = selection_output(vec![dropped("a", 0), dropped("b", 1)]);

        let metrics = compute_metrics(&annotations, &selection, &items);
        assert_eq!(
            metrics.contradictions[0].outcome,
            ContradictionOutcome::NoneRetained
        );
    }

    #[test]
    fn contradiction_split_when_a_non_boundary_dropped_member_hit_the_budget_cutoff() {
        // a(0) and c(2) are dropped; b(1) and d(3) are retained. Under the old *geometric* rule
        // this group -- whose only dropped member is a(0) -- could never be `Split`, because
        // a(0) is not the whole-selection's highest-order-index dropped item (c(2) is, and b(1)
        // is not the lowest-order-index retained item either -- that's a(0)... but a is dropped
        // here, so the old rule's "one foot on each side of the *global* extremes" test fails on
        // both sides). Under the corrected action-based rule it is `Split`: a's own trace action
        // is a budget-cutoff reason, and that alone is what matters -- exactly the fix this test
        // exists to lock in.
        let items = vec![
            item("a", 0, "claim one"),
            item("b", 1, "x"),
            item("c", 2, "y"),
            item("d", 3, "claim two"),
        ];
        let group = ContradictionGroup {
            group_id: "g1".to_string(),
            members: vec![
                EvidenceSpan {
                    source_id: "a".to_string(),
                    byte_start: 0,
                    byte_end: 9,
                },
                EvidenceSpan {
                    source_id: "d".to_string(),
                    byte_start: 0,
                    byte_end: 9,
                },
            ],
        };
        let fact = single_component_fact(
            "f1",
            "c1",
            vec![EvidenceSpan {
                source_id: "b".to_string(),
                byte_start: 0,
                byte_end: 1,
            }],
        );
        let mut annotations = annotations_with(vec![fact]);
        annotations.contradiction_groups.push(group);

        let mut selection = selection_output(vec![
            dropped("a", 0),
            included("b", 1, 1),
            dropped("c", 2),
            included("d", 3, 9),
        ]);
        selection.trace = vec![
            trace_step(0, "a", "dropped_over_budget"),
            trace_step(1, "b", "included_full"),
            trace_step(2, "c", "dropped_over_budget"),
            trace_step(3, "d", "included_full"),
        ];

        let metrics = compute_metrics(&annotations, &selection, &items);
        assert_eq!(
            metrics.contradictions[0].outcome,
            ContradictionOutcome::Split
        );
        assert!(metrics
            .distortion_indicators
            .iter()
            .any(|d| matches!(d, DistortionIndicator::ContradictionSplit { group_id } if group_id == "g1")));
    }

    #[test]
    fn contradiction_partial_within_retained_when_the_dropped_member_is_a_content_drop() {
        // The group's one dropped member ("a") was dropped for a content reason
        // ("dropped_no_extractable_content"), never because the budget ran out -- so nothing
        // about this outcome was actually decided by the budget boundary, even though the group
        // is "mixed" (one retained, one dropped).
        let items = vec![
            item("a", 0, "claim one"),
            item("b", 1, "claim two"),
            item("c", 2, "kept"),
        ];
        let group = ContradictionGroup {
            group_id: "g1".to_string(),
            members: vec![
                EvidenceSpan {
                    source_id: "a".to_string(),
                    byte_start: 0,
                    byte_end: 9,
                },
                EvidenceSpan {
                    source_id: "b".to_string(),
                    byte_start: 0,
                    byte_end: 9,
                },
            ],
        };
        let fact = single_component_fact(
            "f1",
            "c1",
            vec![EvidenceSpan {
                source_id: "c".to_string(),
                byte_start: 0,
                byte_end: 1,
            }],
        );
        let mut annotations = annotations_with(vec![fact]);
        annotations.contradiction_groups.push(group);

        let mut selection = selection_output(vec![
            dropped("a", 0),
            included("b", 1, 9),
            included("c", 2, 4),
        ]);
        selection.trace = vec![
            trace_step(0, "a", "dropped_no_extractable_content"),
            trace_step(1, "b", "extracted_included"),
            trace_step(2, "c", "extracted_included"),
        ];

        let metrics = compute_metrics(&annotations, &selection, &items);
        assert_eq!(
            metrics.contradictions[0].outcome,
            ContradictionOutcome::PartialWithinRetained
        );
        assert!(metrics
            .distortion_indicators
            .iter()
            .all(|d| !matches!(d, DistortionIndicator::ContradictionSplit { .. })));
    }

    #[test]
    fn contradiction_split_is_reachable_via_real_recency_strategy_output() {
        use crate::model::{Budget, StrategyInput};
        use crate::strategy::recency::RecencyStrategy;
        use crate::strategy::SelectionStrategy;

        // Five single-token items; recency@1 keeps the two *newest* under a 2-token budget.
        let items = vec![
            item("item0", 0, "item0"),
            item("item1", 1, "item1"),
            item("item2", 2, "item2"),
            item("item3", 3, "item3"),
            item("item4", 4, "item4"),
        ];
        // Deliberately pick a group whose members are *not* the two items immediately flanking
        // the actual cut (item2/item3) -- proving the fix does not depend on geometric adjacency
        // to the boundary, only on *why* the dropped member was dropped.
        let group = ContradictionGroup {
            group_id: "g-recency-split".to_string(),
            members: vec![
                EvidenceSpan {
                    source_id: "item1".to_string(),
                    byte_start: 0,
                    byte_end: 5,
                },
                EvidenceSpan {
                    source_id: "item4".to_string(),
                    byte_start: 0,
                    byte_end: 5,
                },
            ],
        };
        let annotations = ScenarioAnnotations {
            required_facts: vec![],
            distractor_source_ids: BTreeSet::new(),
            contradiction_groups: vec![group],
        };

        let input = StrategyInput {
            task_query: "q".to_string(),
            items: items.clone(),
        };
        let selection = RecencyStrategy.select(
            &input,
            &Budget {
                requested_tokens: 2,
            },
        );

        let status = |id: &str| {
            selection
                .selection
                .iter()
                .find(|r| r.source_id == id)
                .unwrap()
                .status
        };
        // Verify the real strategy's actual cutoff before trusting the metric: items 3 and 4
        // (the two newest) survive; 0, 1, 2 are dropped as too old.
        assert_eq!(status("item0"), SelectionStatus::Dropped);
        assert_eq!(status("item1"), SelectionStatus::Dropped);
        assert_eq!(status("item2"), SelectionStatus::Dropped);
        assert_eq!(status("item3"), SelectionStatus::Included);
        assert_eq!(status("item4"), SelectionStatus::Included);

        let metrics = compute_metrics(&annotations, &selection, &items);
        assert_eq!(metrics.contradictions.len(), 1);
        assert_eq!(
            metrics.contradictions[0].outcome,
            ContradictionOutcome::Split
        );
        assert!(metrics.distortion_indicators.iter().any(
            |d| matches!(d, DistortionIndicator::ContradictionSplit { group_id } if group_id == "g-recency-split")
        ));
    }

    #[test]
    fn contradiction_split_is_reachable_via_real_retrieval_ranking_strategy_output() {
        use crate::model::{Budget, StrategyInput};
        use crate::strategy::retrieval_ranking::RetrievalRankingStrategy;
        use crate::strategy::SelectionStrategy;

        // "b" scores highest (5 occurrences of the query term "gamma") but needs 5 tokens and
        // does not fit a 2-token budget; retrieval-ranking@1's bin-packing then still includes
        // the smaller, lower-ranked "a" and "c" (0 query overlap) because they fit, while the
        // unrelated, oversized "d" is dropped too. This is the non-contiguous case with no
        // single geometric boundary in `order_index` space -- exactly what the old rule handled
        // incorrectly.
        let items = vec![
            item("a", 0, "alpha"),
            item("b", 1, "gamma gamma gamma gamma gamma"),
            item("c", 2, "beta"),
            item(
                "d",
                3,
                "delta delta delta delta delta delta delta delta delta delta",
            ),
        ];
        // "b" (order_index 1, dropped) and "c" (order_index 2, retained) are neither the
        // whole-selection's highest-order dropped item ("d", order_index 3) nor its
        // lowest-order retained item ("a", order_index 0) -- so the old geometric rule would
        // have called this `PartialWithinRetained`.
        let group = ContradictionGroup {
            group_id: "g-retrieval-split".to_string(),
            members: vec![
                EvidenceSpan {
                    source_id: "b".to_string(),
                    byte_start: 0,
                    byte_end: 5,
                },
                EvidenceSpan {
                    source_id: "c".to_string(),
                    byte_start: 0,
                    byte_end: 4,
                },
            ],
        };
        let annotations = ScenarioAnnotations {
            required_facts: vec![],
            distractor_source_ids: BTreeSet::new(),
            contradiction_groups: vec![group],
        };

        let input = StrategyInput {
            task_query: "gamma".to_string(),
            items: items.clone(),
        };
        let selection = RetrievalRankingStrategy.select(
            &input,
            &Budget {
                requested_tokens: 2,
            },
        );

        let status = |id: &str| {
            selection
                .selection
                .iter()
                .find(|r| r.source_id == id)
                .unwrap()
                .status
        };
        assert_eq!(status("a"), SelectionStatus::Included);
        assert_eq!(status("b"), SelectionStatus::Dropped);
        assert_eq!(status("c"), SelectionStatus::Included);
        assert_eq!(status("d"), SelectionStatus::Dropped);

        let metrics = compute_metrics(&annotations, &selection, &items);
        assert_eq!(metrics.contradictions.len(), 1);
        assert_eq!(
            metrics.contradictions[0].outcome,
            ContradictionOutcome::Split
        );
        assert!(metrics.distortion_indicators.iter().any(
            |d| matches!(d, DistortionIndicator::ContradictionSplit { group_id } if group_id == "g-retrieval-split")
        ));
    }

    #[test]
    fn numeric_fact_mismatched_when_unit_dropped_by_transform() {
        let items = vec![item("a", 0, "latency was high")];
        let component = FactComponent {
            component_id: "c1".to_string(),
            evidence: vec![EvidenceSpan {
                source_id: "a".to_string(),
                byte_start: 0,
                byte_end: 17,
            }],
            canonical_value: Some(CanonicalValue::Number {
                normalized: "120".to_string(),
                unit: Some("ms".to_string()),
            }),
            required_qualifiers: vec![],
        };
        let fact = ExpectedFact {
            fact_id: "f1".to_string(),
            statement: "s".to_string(),
            why_it_matters: "w".to_string(),
            components: vec![component],
            expected_citation_source_ids: vec![],
        };
        let annotations = annotations_with(vec![fact]);
        // Transform output_text carries the number but drops the unit -> value_and_qualifiers_satisfied fails.
        let mut record = included("a", 0, 17);
        record.output_text = Some("latency was 120".to_string());
        let selection = selection_output(vec![record]);

        let metrics = compute_metrics(&annotations, &selection, &items);
        assert_eq!(metrics.numeric_date_accuracy.checked, 1);
        assert_eq!(metrics.numeric_date_accuracy.exact, 0);
        assert_eq!(
            metrics.numeric_date_accuracy.mismatches,
            vec!["f1".to_string()]
        );
        assert_eq!(metrics.fact_recall.retained, 0);
    }

    #[test]
    fn numeric_fact_exact_when_value_and_unit_both_present() {
        let items = vec![item("a", 0, "latency was 120ms exactly")];
        let component = FactComponent {
            component_id: "c1".to_string(),
            evidence: vec![EvidenceSpan {
                source_id: "a".to_string(),
                byte_start: 0,
                byte_end: 18,
            }],
            canonical_value: Some(CanonicalValue::Number {
                normalized: "120".to_string(),
                unit: Some("ms".to_string()),
            }),
            required_qualifiers: vec![],
        };
        let fact = ExpectedFact {
            fact_id: "f1".to_string(),
            statement: "s".to_string(),
            why_it_matters: "w".to_string(),
            components: vec![component],
            expected_citation_source_ids: vec![],
        };
        let annotations = annotations_with(vec![fact]);
        let selection = selection_output(vec![included("a", 0, 25)]);

        let metrics = compute_metrics(&annotations, &selection, &items);
        assert_eq!(metrics.numeric_date_accuracy.checked, 1);
        assert_eq!(metrics.numeric_date_accuracy.exact, 1);
        assert!(metrics.numeric_date_accuracy.mismatches.is_empty());
        assert_eq!(metrics.fact_recall.retained, 1);
    }

    #[test]
    fn distractor_retained_is_flagged() {
        let items = vec![item("a", 0, "real"), item("distractor-1", 1, "noise")];
        let mut annotations = annotations_with(vec![single_component_fact(
            "f1",
            "c1",
            vec![EvidenceSpan {
                source_id: "a".to_string(),
                byte_start: 0,
                byte_end: 4,
            }],
        )]);
        annotations
            .distractor_source_ids
            .insert("distractor-1".to_string());
        let selection = selection_output(vec![included("a", 0, 4), included("distractor-1", 1, 5)]);

        let metrics = compute_metrics(&annotations, &selection, &items);
        assert!(metrics
            .distortion_indicators
            .iter()
            .any(|d| matches!(d, DistortionIndicator::DistractorRetained { source_id } if source_id == "distractor-1")));
    }

    #[test]
    fn distractor_dropped_is_not_flagged() {
        let items = vec![item("a", 0, "real"), item("distractor-1", 1, "noise")];
        let mut annotations = annotations_with(vec![single_component_fact(
            "f1",
            "c1",
            vec![EvidenceSpan {
                source_id: "a".to_string(),
                byte_start: 0,
                byte_end: 4,
            }],
        )]);
        annotations
            .distractor_source_ids
            .insert("distractor-1".to_string());
        let selection = selection_output(vec![included("a", 0, 4), dropped("distractor-1", 1)]);

        let metrics = compute_metrics(&annotations, &selection, &items);
        assert!(!metrics
            .distortion_indicators
            .iter()
            .any(|d| matches!(d, DistortionIndicator::DistractorRetained { .. })));
    }

    #[test]
    fn qualifier_dropped_indicator_names_missing_qualifier() {
        let items = vec![item("a", 0, "the server did not crash today")];
        let component = FactComponent {
            component_id: "c1".to_string(),
            evidence: vec![EvidenceSpan {
                source_id: "a".to_string(),
                byte_start: 0,
                byte_end: 3,
            }],
            canonical_value: None,
            required_qualifiers: vec!["not".to_string()],
        };
        let fact = ExpectedFact {
            fact_id: "f1".to_string(),
            statement: "s".to_string(),
            why_it_matters: "w".to_string(),
            components: vec![component],
            expected_citation_source_ids: vec![],
        };
        let annotations = annotations_with(vec![fact]);
        // Item is fully `Included`, so the spatial gate passes regardless of the evidence span's
        // exact bounds; the transform's `output_text` (simulating a hypothetical distortion) has
        // dropped the negation word, which is what this test is actually checking.
        let mut record = included("a", 0, 31);
        record.output_text = Some("the server did crash today".to_string());
        let selection = selection_output(vec![record]);

        let metrics = compute_metrics(&annotations, &selection, &items);
        assert!(metrics.distortion_indicators.iter().any(
            |d| matches!(d, DistortionIndicator::QualifierDropped { fact_id, qualifier } if fact_id == "f1" && qualifier == "not")
        ));
        assert_eq!(metrics.fact_recall.retained, 0);
    }

    #[test]
    fn evidence_clipped_indicator_when_partial_cut_excludes_evidence() {
        let items = vec![item("a", 0, "0123456789")];
        let fact = single_component_fact(
            "f1",
            "c1",
            vec![EvidenceSpan {
                source_id: "a".to_string(),
                byte_start: 5,
                byte_end: 10,
            }],
        );
        let annotations = annotations_with(vec![fact]);
        // Partial retains only [0,5), so evidence [5,10) is entirely clipped out.
        let selection = selection_output(vec![partial("a", 0, 0, 5)]);

        let metrics = compute_metrics(&annotations, &selection, &items);
        assert!(metrics.distortion_indicators.iter().any(
            |d| matches!(d, DistortionIndicator::EvidenceClipped { fact_id } if fact_id == "f1")
        ));
        assert_eq!(metrics.fact_recall.retained, 0);
    }

    #[test]
    fn provenance_reordered_indicator_when_output_order_breaks_original_order() {
        let items = vec![item("a", 0, "x"), item("b", 1, "y"), item("c", 2, "z")];
        let annotations = annotations_with(vec![single_component_fact(
            "f1",
            "c1",
            vec![EvidenceSpan {
                source_id: "a".to_string(),
                byte_start: 0,
                byte_end: 1,
            }],
        )]);
        // Selection Vec order = output order: c(2), a(0), b(1) -- "a" appears after "c" despite a
        // lower order_index, so "a" is flagged; "b" (order 1) after "a" (order 0, now max=2 seen
        // from c) is also out of place relative to the running max.
        let selection = selection_output(vec![
            included("c", 2, 1),
            included("a", 0, 1),
            included("b", 1, 1),
        ]);

        let metrics = compute_metrics(&annotations, &selection, &items);
        let reordered: Vec<&str> = metrics
            .distortion_indicators
            .iter()
            .filter_map(|d| match d {
                DistortionIndicator::ProvenanceReordered { source_id } => Some(source_id.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(reordered, vec!["a", "b"]);
    }

    #[test]
    fn citation_retention_counts_only_facts_with_expectations() {
        let items = vec![item("a", 0, "x"), item("b", 1, "y")];
        let mut fact_with_citation = single_component_fact(
            "f1",
            "c1",
            vec![EvidenceSpan {
                source_id: "a".to_string(),
                byte_start: 0,
                byte_end: 1,
            }],
        );
        fact_with_citation.expected_citation_source_ids = vec!["a".to_string()];
        let fact_without_citation = single_component_fact(
            "f2",
            "c2",
            vec![EvidenceSpan {
                source_id: "b".to_string(),
                byte_start: 0,
                byte_end: 1,
            }],
        );
        let annotations = annotations_with(vec![fact_with_citation, fact_without_citation]);
        let selection = selection_output(vec![included("a", 0, 1), included("b", 1, 1)]);

        let metrics = compute_metrics(&annotations, &selection, &items);
        assert_eq!(metrics.citation_retention.expected, 1);
        assert_eq!(metrics.citation_retention.retained, 1);
        assert_eq!(
            metrics.citation_retention.per_fact,
            vec![("f1".to_string(), true)]
        );
    }

    #[test]
    fn citation_retention_fails_when_a_cited_source_is_dropped() {
        let items = vec![item("a", 0, "x"), item("b", 1, "y")];
        let mut fact = single_component_fact(
            "f1",
            "c1",
            vec![EvidenceSpan {
                source_id: "a".to_string(),
                byte_start: 0,
                byte_end: 1,
            }],
        );
        fact.expected_citation_source_ids = vec!["a".to_string(), "b".to_string()];
        let annotations = annotations_with(vec![fact]);
        let selection = selection_output(vec![included("a", 0, 1), dropped("b", 1)]);

        let metrics = compute_metrics(&annotations, &selection, &items);
        assert_eq!(metrics.citation_retention.expected, 1);
        assert_eq!(metrics.citation_retention.retained, 0);
        assert_eq!(
            metrics.citation_retention.per_fact,
            vec![("f1".to_string(), false)]
        );
    }

    #[test]
    fn budget_passes_through_from_selection() {
        let items = vec![item("a", 0, "x")];
        let annotations = annotations_with(vec![single_component_fact(
            "f1",
            "c1",
            vec![EvidenceSpan {
                source_id: "a".to_string(),
                byte_start: 0,
                byte_end: 1,
            }],
        )]);
        let mut selection = selection_output(vec![included("a", 0, 1)]);
        selection.budget = BudgetUsage::new(500, 123);
        selection.trace.push(TraceStep {
            step_index: 0,
            source_id: "a".to_string(),
            action: "included_full".to_string(),
            score: None,
            detail: "d".to_string(),
        });

        let metrics = compute_metrics(&annotations, &selection, &items);
        assert_eq!(metrics.budget, BudgetUsage::new(500, 123));
    }
}
