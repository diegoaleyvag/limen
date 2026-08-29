//! Reference fixtures: small, hand-verified, always-valid example data used by this crate's own
//! tests, the schema-generation example, and downstream bootstrapping. Not used by any
//! production validation/strategy/metrics logic, and not real scenario content (real scenario
//! authoring happens in a later phase, under `scenarios/v1/`).

use std::collections::BTreeSet;

use crate::canonical;
use crate::model::{
    CanonicalValue, ContextItem, ContradictionGroup, EvidenceSpan, ExpectedFact, FactComponent,
    ScenarioAnnotations, ScenarioManifest,
};
use crate::SCHEMA_VERSION;

/// A complete, minimal, always-valid [`ScenarioManifest`]: five items (two log lines, two
/// conflicting chat messages, one distractor note), three required facts (a plain numeric fact, a
/// composite fact requiring two components, and a fact with redundant evidence across two
/// sources), one contradiction group, and one distractor. `content_digest` is computed correctly
/// at construction time, so the result always passes [`crate::validate::validate_manifest`]
/// cleanly -- see the `minimal_valid_fixture_validates_cleanly` test in `validate.rs`.
pub fn minimal_valid_manifest() -> ScenarioManifest {
    let items = vec![
        ContextItem {
            source_id: "log-1".to_string(),
            order_index: 0,
            section_label: "log_line".to_string(),
            text: "Latency was 120ms at 10:00.".to_string(),
        },
        ContextItem {
            source_id: "log-2".to_string(),
            order_index: 1,
            section_label: "log_line".to_string(),
            text: "Latency was 350ms at 10:05.".to_string(),
        },
        ContextItem {
            source_id: "chat-1".to_string(),
            order_index: 2,
            section_label: "chat_message".to_string(),
            text: "It was not caused by a deploy.".to_string(),
        },
        ContextItem {
            source_id: "chat-2".to_string(),
            order_index: 3,
            section_label: "chat_message".to_string(),
            text: "It was caused by a deploy.".to_string(),
        },
        ContextItem {
            source_id: "note-1".to_string(),
            order_index: 4,
            section_label: "note".to_string(),
            text: "Reminder: renew the certificate.".to_string(),
        },
    ];

    let required_facts = vec![
        ExpectedFact {
            fact_id: "f-latency-spike".to_string(),
            statement: "Latency spiked to 350ms at 10:05.".to_string(),
            why_it_matters: "Quantifies the severity of the incident precisely.".to_string(),
            components: vec![FactComponent {
                component_id: "c-latency-350".to_string(),
                evidence: vec![EvidenceSpan {
                    source_id: "log-2".to_string(),
                    byte_start: 12,
                    byte_end: 17,
                }],
                canonical_value: Some(CanonicalValue::Number {
                    normalized: "350".to_string(),
                    unit: Some("ms".to_string()),
                }),
                required_qualifiers: vec![],
            }],
            expected_citation_source_ids: vec!["log-2".to_string()],
        },
        ExpectedFact {
            fact_id: "f-both-readings-present".to_string(),
            statement: "Both the 120ms and 350ms latency readings are on record.".to_string(),
            why_it_matters:
                "Confirms a strategy preserved the full timeline of readings, not just one."
                    .to_string(),
            components: vec![
                FactComponent {
                    component_id: "c-reading-120".to_string(),
                    evidence: vec![EvidenceSpan {
                        source_id: "log-1".to_string(),
                        byte_start: 12,
                        byte_end: 17,
                    }],
                    canonical_value: Some(CanonicalValue::Number {
                        normalized: "120".to_string(),
                        unit: Some("ms".to_string()),
                    }),
                    required_qualifiers: vec![],
                },
                FactComponent {
                    component_id: "c-reading-350".to_string(),
                    evidence: vec![EvidenceSpan {
                        source_id: "log-2".to_string(),
                        byte_start: 12,
                        byte_end: 17,
                    }],
                    canonical_value: Some(CanonicalValue::Number {
                        normalized: "350".to_string(),
                        unit: Some("ms".to_string()),
                    }),
                    required_qualifiers: vec![],
                },
            ],
            expected_citation_source_ids: vec!["log-1".to_string(), "log-2".to_string()],
        },
        ExpectedFact {
            fact_id: "f-deploy-mentioned-as-cause".to_string(),
            statement: "A deploy is mentioned as a possible cause of the outage.".to_string(),
            why_it_matters:
                "Confirms a strategy preserved at least one of the two conflicting causal claims."
                    .to_string(),
            components: vec![FactComponent {
                component_id: "c-deploy-mention".to_string(),
                evidence: vec![
                    EvidenceSpan {
                        source_id: "chat-1".to_string(),
                        byte_start: 23,
                        byte_end: 29,
                    },
                    EvidenceSpan {
                        source_id: "chat-2".to_string(),
                        byte_start: 19,
                        byte_end: 25,
                    },
                ],
                canonical_value: Some(CanonicalValue::Text {
                    normalized: "deploy".to_string(),
                }),
                required_qualifiers: vec![],
            }],
            expected_citation_source_ids: vec!["chat-1".to_string()],
        },
    ];

    let mut distractor_source_ids = BTreeSet::new();
    distractor_source_ids.insert("note-1".to_string());

    let annotations = ScenarioAnnotations {
        required_facts,
        distractor_source_ids,
        contradiction_groups: vec![ContradictionGroup {
            group_id: "g-outage-cause".to_string(),
            members: vec![
                EvidenceSpan {
                    source_id: "chat-1".to_string(),
                    byte_start: 7,
                    byte_end: 29,
                },
                EvidenceSpan {
                    source_id: "chat-2".to_string(),
                    byte_start: 7,
                    byte_end: 25,
                },
            ],
        }],
    };

    let mut manifest = ScenarioManifest {
        schema_version: SCHEMA_VERSION.to_string(),
        scenario_id: "minimal-fixture".to_string(),
        scenario_version: "1.0.0".to_string(),
        title: "Minimal Fixture Scenario".to_string(),
        task_query: "What caused the latency incident, and how severe was it?".to_string(),
        items,
        annotations,
        content_digest: String::new(),
    };
    manifest.content_digest = canonical::digest_with_field_blanked(&manifest, "content_digest")
        .expect("fixture manifest is plain data and must always canonicalize");
    manifest
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validate::validate_manifest;

    #[test]
    fn fixture_has_a_correct_content_digest() {
        let manifest = minimal_valid_manifest();
        let recomputed = canonical::digest_with_field_blanked(&manifest, "content_digest").unwrap();
        assert_eq!(manifest.content_digest, recomputed);
    }

    #[test]
    fn fixture_validates_with_zero_errors() {
        assert_eq!(validate_manifest(&minimal_valid_manifest()), vec![]);
    }
}
