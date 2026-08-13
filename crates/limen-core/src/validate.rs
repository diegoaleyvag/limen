//! Manifest validation: [`validate_manifest`] collects **every** violation in one pass rather
//! than stopping at the first one, so a scenario author sees every problem at once.
//!
//! This module only validates scenario *content* (the [`crate::model::ScenarioManifest`]).
//! Resolving a requested strategy id/version is a structurally distinct failure path (it is about
//! the request, not the scenario) and lives at [`crate::strategy::resolve_strategy`] instead.

use std::collections::{BTreeMap, BTreeSet};

use crate::canonical;
use crate::error::ValidationError;
use crate::model::{ContextItem, EvidenceSpan, ScenarioManifest};

/// Stable validation error codes. Never change the meaning of an existing code once shipped;
/// add new ones instead.
pub mod codes {
    pub const DUPLICATE_SOURCE_ID: &str = "duplicate_source_id";
    pub const DUPLICATE_FACT_ID: &str = "duplicate_fact_id";
    pub const DUPLICATE_COMPONENT_ID: &str = "duplicate_component_id";
    pub const DUPLICATE_GROUP_ID: &str = "duplicate_group_id";
    pub const EVIDENCE_SPAN_INVALID_RANGE: &str = "evidence_span_invalid_range";
    pub const EVIDENCE_SPAN_OUT_OF_BOUNDS: &str = "evidence_span_out_of_bounds";
    pub const EVIDENCE_SPAN_SPLITS_CODEPOINT: &str = "evidence_span_splits_codepoint";
    pub const EVIDENCE_SPAN_UNKNOWN_SOURCE: &str = "evidence_span_unknown_source";
    pub const EXPECTED_CITATION_UNKNOWN_SOURCE: &str = "expected_citation_unknown_source";
    pub const DISTRACTOR_UNKNOWN_SOURCE: &str = "distractor_unknown_source";
    pub const EMPTY_REQUIRED_FACTS: &str = "empty_required_facts";
    pub const FACT_ZERO_COMPONENTS: &str = "fact_zero_components";
    pub const COMPONENT_ZERO_EVIDENCE: &str = "component_zero_evidence";
    pub const COMPONENT_UNCHECKABLE_VALUE: &str = "component_uncheckable_value";
    pub const MALFORMED_SCHEMA_VERSION: &str = "malformed_schema_version";
    pub const MALFORMED_SCENARIO_VERSION: &str = "malformed_scenario_version";
    pub const STALE_CONTENT_DIGEST: &str = "stale_content_digest";
    pub const DIGEST_COMPUTATION_FAILED: &str = "digest_computation_failed";
}

/// Validates `manifest`, returning every violation found. An empty `Vec` means the manifest is
/// valid.
pub fn validate_manifest(manifest: &ScenarioManifest) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    if !is_semver_triplet(&manifest.schema_version) {
        errors.push(ValidationError::at(
            codes::MALFORMED_SCHEMA_VERSION,
            format!(
                "schema_version '{}' is not a MAJOR.MINOR.PATCH numeric version",
                manifest.schema_version
            ),
            "schema_version",
        ));
    }
    if !is_semver_triplet(&manifest.scenario_version) {
        errors.push(ValidationError::at(
            codes::MALFORMED_SCENARIO_VERSION,
            format!(
                "scenario_version '{}' is not a MAJOR.MINOR.PATCH numeric version",
                manifest.scenario_version
            ),
            "scenario_version",
        ));
    }

    // Duplicate source_id detection. `items_by_id` keeps only the *first* occurrence of each
    // source_id, so a later duplicate is reported here but does not shadow the original when we
    // resolve evidence spans below.
    let mut items_by_id: BTreeMap<&str, &ContextItem> = BTreeMap::new();
    let mut seen_source_ids: BTreeSet<&str> = BTreeSet::new();
    for (idx, item) in manifest.items.iter().enumerate() {
        if !seen_source_ids.insert(item.source_id.as_str()) {
            errors.push(ValidationError::at(
                codes::DUPLICATE_SOURCE_ID,
                format!(
                    "source_id '{}' appears more than once in items",
                    item.source_id
                ),
                format!("items[{idx}].source_id"),
            ));
        } else {
            items_by_id.insert(item.source_id.as_str(), item);
        }
    }

    if manifest.annotations.required_facts.is_empty() {
        errors.push(ValidationError::at(
            codes::EMPTY_REQUIRED_FACTS,
            "annotations.required_facts must not be empty",
            "annotations.required_facts",
        ));
    }

    let mut seen_fact_ids: BTreeSet<&str> = BTreeSet::new();
    let mut seen_component_ids: BTreeSet<&str> = BTreeSet::new();
    for (fi, fact) in manifest.annotations.required_facts.iter().enumerate() {
        let fact_path = format!("annotations.required_facts[{fi}]");

        if !seen_fact_ids.insert(fact.fact_id.as_str()) {
            errors.push(ValidationError::at(
                codes::DUPLICATE_FACT_ID,
                format!("fact_id '{}' appears more than once", fact.fact_id),
                format!("{fact_path}.fact_id"),
            ));
        }

        if fact.components.is_empty() {
            errors.push(ValidationError::at(
                codes::FACT_ZERO_COMPONENTS,
                format!("fact '{}' has zero components", fact.fact_id),
                fact_path.clone(),
            ));
        }

        for (ci, component) in fact.components.iter().enumerate() {
            let component_path = format!("{fact_path}.components[{ci}]");

            if !seen_component_ids.insert(component.component_id.as_str()) {
                errors.push(ValidationError::at(
                    codes::DUPLICATE_COMPONENT_ID,
                    format!(
                        "component_id '{}' appears more than once",
                        component.component_id
                    ),
                    format!("{component_path}.component_id"),
                ));
            }

            if component.evidence.is_empty() {
                errors.push(ValidationError::at(
                    codes::COMPONENT_ZERO_EVIDENCE,
                    format!(
                        "component '{}' has zero evidence alternatives",
                        component.component_id
                    ),
                    component_path.clone(),
                ));
            }

            // A component with neither a `canonical_value` nor any `required_qualifiers` is
            // retained purely on the strength of its evidence's spatial (byte-range) provenance
            // pointer, with no check that the retained/transformed output text actually still
            // contains anything of the value -- e.g. a transform strategy's `output_text` could
            // in principle omit the fact entirely while the byte-range pointer still "covers" the
            // right region of the source. Requiring at least one of the two keeps every metrics
            // decision anchored to a real text check, not just provenance.
            if component.canonical_value.is_none() && component.required_qualifiers.is_empty() {
                errors.push(ValidationError::at(
                    codes::COMPONENT_UNCHECKABLE_VALUE,
                    format!(
                        "component '{}' has neither a canonical_value nor any required_qualifiers, \
                         so retention would be checked by provenance alone",
                        component.component_id
                    ),
                    component_path.clone(),
                ));
            }

            for (ei, span) in component.evidence.iter().enumerate() {
                validate_evidence_span(
                    span,
                    &items_by_id,
                    format!("{component_path}.evidence[{ei}]"),
                    &mut errors,
                );
            }
        }

        for (si, source_id) in fact.expected_citation_source_ids.iter().enumerate() {
            if !items_by_id.contains_key(source_id.as_str()) {
                errors.push(ValidationError::at(
                    codes::EXPECTED_CITATION_UNKNOWN_SOURCE,
                    format!(
                        "expected_citation_source_ids references unknown source_id '{source_id}'"
                    ),
                    format!("{fact_path}.expected_citation_source_ids[{si}]"),
                ));
            }
        }
    }

    let mut seen_group_ids: BTreeSet<&str> = BTreeSet::new();
    for (gi, group) in manifest.annotations.contradiction_groups.iter().enumerate() {
        let group_path = format!("annotations.contradiction_groups[{gi}]");

        if !seen_group_ids.insert(group.group_id.as_str()) {
            errors.push(ValidationError::at(
                codes::DUPLICATE_GROUP_ID,
                format!("group_id '{}' appears more than once", group.group_id),
                format!("{group_path}.group_id"),
            ));
        }

        // Unknown source_ids referenced by a contradiction group are reported through the same
        // shared evidence-span check as fact-component evidence (members are EvidenceSpans too),
        // via `codes::EVIDENCE_SPAN_UNKNOWN_SOURCE`.
        for (mi, span) in group.members.iter().enumerate() {
            validate_evidence_span(
                span,
                &items_by_id,
                format!("{group_path}.members[{mi}]"),
                &mut errors,
            );
        }
    }

    // BTreeSet iteration is sorted, so this loop's error order is deterministic.
    for source_id in &manifest.annotations.distractor_source_ids {
        if !items_by_id.contains_key(source_id.as_str()) {
            errors.push(ValidationError::at(
                codes::DISTRACTOR_UNKNOWN_SOURCE,
                format!("distractor_source_ids references unknown source_id '{source_id}'"),
                format!("annotations.distractor_source_ids[{source_id}]"),
            ));
        }
    }

    match canonical::digest_with_field_blanked(manifest, "content_digest") {
        Ok(fresh_digest) => {
            if fresh_digest != manifest.content_digest {
                errors.push(ValidationError::at(
                    codes::STALE_CONTENT_DIGEST,
                    format!(
                        "content_digest '{}' does not match the freshly computed digest '{fresh_digest}'",
                        manifest.content_digest
                    ),
                    "content_digest",
                ));
            }
        }
        Err(engine_error) => {
            errors.push(ValidationError::at(
                codes::DIGEST_COMPUTATION_FAILED,
                format!("failed to compute content_digest for comparison: {engine_error}"),
                "content_digest",
            ));
        }
    }

    errors
}

/// Validates one [`EvidenceSpan`] against the known source items, pushing any violations onto
/// `errors`. Shared by fact-component evidence and contradiction-group members, since both are
/// plain `EvidenceSpan`s with identical validity rules.
fn validate_evidence_span(
    span: &EvidenceSpan,
    items_by_id: &BTreeMap<&str, &ContextItem>,
    path: String,
    errors: &mut Vec<ValidationError>,
) {
    let Some(item) = items_by_id.get(span.source_id.as_str()) else {
        errors.push(ValidationError::at(
            codes::EVIDENCE_SPAN_UNKNOWN_SOURCE,
            format!("evidence references unknown source_id '{}'", span.source_id),
            path,
        ));
        return;
    };

    if span.byte_start >= span.byte_end {
        errors.push(ValidationError::at(
            codes::EVIDENCE_SPAN_INVALID_RANGE,
            format!(
                "evidence span [{}, {}) for source_id '{}' is empty or inverted",
                span.byte_start, span.byte_end, span.source_id
            ),
            path,
        ));
        return;
    }

    // Compare against the source's byte length in `usize` (never truncate the length down to
    // u32; widen the u32 offsets up to usize instead, which is always lossless).
    let text_len = item.text.len();
    if (span.byte_start as usize) > text_len || (span.byte_end as usize) > text_len {
        errors.push(ValidationError::at(
            codes::EVIDENCE_SPAN_OUT_OF_BOUNDS,
            format!(
                "evidence span [{}, {}) for source_id '{}' exceeds its {} byte-length text",
                span.byte_start, span.byte_end, span.source_id, text_len
            ),
            path,
        ));
        return;
    }

    if !item.text.is_char_boundary(span.byte_start as usize)
        || !item.text.is_char_boundary(span.byte_end as usize)
    {
        errors.push(ValidationError::at(
            codes::EVIDENCE_SPAN_SPLITS_CODEPOINT,
            format!(
                "evidence span [{}, {}) for source_id '{}' splits a UTF-8 codepoint",
                span.byte_start, span.byte_end, span.source_id
            ),
            path,
        ));
    }
}

/// Requires exactly three dot-separated, non-empty, all-ASCII-numeric parts (`MAJOR.MINOR.PATCH`,
/// e.g. `"1.0.0"`). No leading `v`, no pre-release/build metadata suffixes.
fn is_semver_triplet(value: &str) -> bool {
    let parts: Vec<&str> = value.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::minimal_valid_manifest;
    use crate::model::{ContradictionGroup, EvidenceSpan, ExpectedFact, FactComponent};

    fn assert_has_code(errors: &[ValidationError], code: &str) {
        assert!(
            errors.iter().any(|e| e.code == code),
            "expected an error with code '{code}', got: {errors:?}"
        );
    }

    #[test]
    fn minimal_valid_fixture_validates_cleanly() {
        let manifest = minimal_valid_manifest();
        let errors = validate_manifest(&manifest);
        assert_eq!(
            errors,
            vec![],
            "expected zero validation errors, got: {errors:?}"
        );
    }

    #[test]
    fn rejects_duplicate_source_id() {
        let mut manifest = minimal_valid_manifest();
        let mut dup = manifest.items[0].clone();
        dup.order_index = 999;
        manifest.items.push(dup);
        let errors = validate_manifest(&manifest);
        assert_has_code(&errors, codes::DUPLICATE_SOURCE_ID);
    }

    #[test]
    fn rejects_duplicate_fact_id() {
        let mut manifest = minimal_valid_manifest();
        let dup = manifest.annotations.required_facts[0].clone();
        manifest.annotations.required_facts.push(dup);
        let errors = validate_manifest(&manifest);
        assert_has_code(&errors, codes::DUPLICATE_FACT_ID);
    }

    #[test]
    fn rejects_duplicate_component_id() {
        let mut manifest = minimal_valid_manifest();
        let dup = manifest.annotations.required_facts[0].components[0].clone();
        manifest.annotations.required_facts[0].components.push(dup);
        let errors = validate_manifest(&manifest);
        assert_has_code(&errors, codes::DUPLICATE_COMPONENT_ID);
    }

    #[test]
    fn rejects_duplicate_group_id() {
        let mut manifest = minimal_valid_manifest();
        let source_id = manifest.items[0].source_id.clone();
        let group = ContradictionGroup {
            group_id: "g1".to_string(),
            members: vec![EvidenceSpan {
                source_id: source_id.clone(),
                byte_start: 0,
                byte_end: 1,
            }],
        };
        manifest
            .annotations
            .contradiction_groups
            .push(group.clone());
        manifest.annotations.contradiction_groups.push(group);
        let errors = validate_manifest(&manifest);
        assert_has_code(&errors, codes::DUPLICATE_GROUP_ID);
    }

    #[test]
    fn rejects_evidence_span_invalid_range() {
        let mut manifest = minimal_valid_manifest();
        manifest.annotations.required_facts[0].components[0].evidence[0].byte_start = 5;
        manifest.annotations.required_facts[0].components[0].evidence[0].byte_end = 5;
        let errors = validate_manifest(&manifest);
        assert_has_code(&errors, codes::EVIDENCE_SPAN_INVALID_RANGE);
    }

    #[test]
    fn rejects_evidence_span_out_of_bounds() {
        let mut manifest = minimal_valid_manifest();
        let text_len = manifest.items[0].text.len() as u32;
        manifest.annotations.required_facts[0].components[0].evidence[0].byte_start = text_len;
        manifest.annotations.required_facts[0].components[0].evidence[0].byte_end = text_len + 10;
        let errors = validate_manifest(&manifest);
        assert_has_code(&errors, codes::EVIDENCE_SPAN_OUT_OF_BOUNDS);
    }

    #[test]
    fn rejects_evidence_span_splitting_codepoint() {
        let mut manifest = minimal_valid_manifest();
        let source_id = "s_multibyte".to_string();
        manifest.items.push(ContextItem {
            source_id: source_id.clone(),
            order_index: manifest.items.len() as u32,
            section_label: "log_line".to_string(),
            text: "café".to_string(),
        });
        // 'é' occupies bytes [3, 5); byte_start=4 lands inside that codepoint.
        manifest.annotations.required_facts[0].components[0].evidence[0] = EvidenceSpan {
            source_id,
            byte_start: 4,
            byte_end: 5,
        };
        let errors = validate_manifest(&manifest);
        assert_has_code(&errors, codes::EVIDENCE_SPAN_SPLITS_CODEPOINT);
    }

    #[test]
    fn rejects_evidence_span_unknown_source() {
        let mut manifest = minimal_valid_manifest();
        manifest.annotations.required_facts[0].components[0].evidence[0].source_id =
            "does-not-exist".to_string();
        let errors = validate_manifest(&manifest);
        assert_has_code(&errors, codes::EVIDENCE_SPAN_UNKNOWN_SOURCE);
    }

    #[test]
    fn contradiction_group_unknown_source_reuses_evidence_span_unknown_source_code() {
        let mut manifest = minimal_valid_manifest();
        manifest
            .annotations
            .contradiction_groups
            .push(ContradictionGroup {
                group_id: "g-unknown".to_string(),
                members: vec![EvidenceSpan {
                    source_id: "ghost".to_string(),
                    byte_start: 0,
                    byte_end: 1,
                }],
            });
        let errors = validate_manifest(&manifest);
        assert_has_code(&errors, codes::EVIDENCE_SPAN_UNKNOWN_SOURCE);
    }

    #[test]
    fn rejects_expected_citation_unknown_source() {
        let mut manifest = minimal_valid_manifest();
        manifest.annotations.required_facts[0]
            .expected_citation_source_ids
            .push("ghost".to_string());
        let errors = validate_manifest(&manifest);
        assert_has_code(&errors, codes::EXPECTED_CITATION_UNKNOWN_SOURCE);
    }

    #[test]
    fn rejects_distractor_unknown_source() {
        let mut manifest = minimal_valid_manifest();
        manifest
            .annotations
            .distractor_source_ids
            .insert("ghost".to_string());
        let errors = validate_manifest(&manifest);
        assert_has_code(&errors, codes::DISTRACTOR_UNKNOWN_SOURCE);
    }

    #[test]
    fn rejects_empty_required_facts() {
        let mut manifest = minimal_valid_manifest();
        manifest.annotations.required_facts.clear();
        let errors = validate_manifest(&manifest);
        assert_has_code(&errors, codes::EMPTY_REQUIRED_FACTS);
    }

    #[test]
    fn rejects_fact_with_zero_components() {
        let mut manifest = minimal_valid_manifest();
        manifest.annotations.required_facts.push(ExpectedFact {
            fact_id: "f-empty".to_string(),
            statement: "statement".to_string(),
            why_it_matters: "why".to_string(),
            components: vec![],
            expected_citation_source_ids: vec![],
        });
        let errors = validate_manifest(&manifest);
        assert_has_code(&errors, codes::FACT_ZERO_COMPONENTS);
    }

    #[test]
    fn rejects_component_with_zero_evidence() {
        let mut manifest = minimal_valid_manifest();
        manifest.annotations.required_facts[0]
            .components
            .push(FactComponent {
                component_id: "c-empty".to_string(),
                evidence: vec![],
                canonical_value: None,
                required_qualifiers: vec![],
            });
        let errors = validate_manifest(&manifest);
        assert_has_code(&errors, codes::COMPONENT_ZERO_EVIDENCE);
    }

    #[test]
    fn rejects_component_with_neither_canonical_value_nor_required_qualifiers() {
        let mut manifest = minimal_valid_manifest();
        manifest.annotations.required_facts[0]
            .components
            .push(FactComponent {
                component_id: "c-uncheckable".to_string(),
                evidence: vec![EvidenceSpan {
                    source_id: manifest.items[0].source_id.clone(),
                    byte_start: 0,
                    byte_end: 1,
                }],
                canonical_value: None,
                required_qualifiers: vec![],
            });
        let errors = validate_manifest(&manifest);
        assert_has_code(&errors, codes::COMPONENT_UNCHECKABLE_VALUE);
    }

    #[test]
    fn accepts_component_with_only_required_qualifiers_and_no_canonical_value() {
        let mut manifest = minimal_valid_manifest();
        manifest.annotations.required_facts[0]
            .components
            .push(FactComponent {
                component_id: "c-qualifier-only".to_string(),
                evidence: vec![EvidenceSpan {
                    source_id: manifest.items[0].source_id.clone(),
                    byte_start: 0,
                    byte_end: 1,
                }],
                canonical_value: None,
                required_qualifiers: vec!["not".to_string()],
            });
        // Freshen the content digest so this test isolates the one rule under test rather than
        // also tripping `stale_content_digest`.
        manifest.content_digest =
            canonical::digest_with_field_blanked(&manifest, "content_digest").unwrap();
        let errors = validate_manifest(&manifest);
        assert!(
            !errors.iter().any(|e| e.code == codes::COMPONENT_UNCHECKABLE_VALUE),
            "a component with required_qualifiers but no canonical_value must not be flagged: {errors:?}"
        );
    }

    #[test]
    fn rejects_malformed_schema_version() {
        let mut manifest = minimal_valid_manifest();
        manifest.schema_version = "v1".to_string();
        let errors = validate_manifest(&manifest);
        assert_has_code(&errors, codes::MALFORMED_SCHEMA_VERSION);
    }

    #[test]
    fn rejects_malformed_scenario_version() {
        let mut manifest = minimal_valid_manifest();
        manifest.scenario_version = "1.0".to_string();
        let errors = validate_manifest(&manifest);
        assert_has_code(&errors, codes::MALFORMED_SCENARIO_VERSION);
    }

    #[test]
    fn rejects_stale_content_digest() {
        let mut manifest = minimal_valid_manifest();
        manifest.title = "A different title, invalidating the digest".to_string();
        let errors = validate_manifest(&manifest);
        assert_has_code(&errors, codes::STALE_CONTENT_DIGEST);
    }

    #[test]
    fn is_semver_triplet_accepts_and_rejects_expected_forms() {
        assert!(is_semver_triplet("1.0.0"));
        assert!(is_semver_triplet("10.20.30"));
        assert!(!is_semver_triplet("1.0"));
        assert!(!is_semver_triplet("1.0.0-alpha"));
        assert!(!is_semver_triplet("v1.0.0"));
        assert!(!is_semver_triplet("1.0.0.0"));
        assert!(!is_semver_triplet(""));
    }
}
