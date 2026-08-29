//! Property-based tests (via `proptest`) proving crate-wide invariants hold across randomized
//! inputs, not just the hand-picked examples in each module's own unit tests.
//!
//! Five invariants are covered, matching the "verify-foundation" acceptance criteria:
//! - [`budget_safety_never_exceeds_requested`]: `used_tokens <= requested_tokens` always holds.
//! - [`determinism_select_twice_is_byte_identical`]: calling `select` twice on identical input
//!   produces byte-identical canonical JSON.
//! - [`selection_is_one_record_per_item_sorted_by_order_index`]: `SelectionOutput.selection` is
//!   always exactly one record per input item, sorted by `order_index` ascending.
//! - [`resolve_strategy_never_panics_and_rejects_unknown_ids`]: random strings never panic
//!   `resolve_strategy` and always resolve to a structured `UnknownStrategyVersion` unless they
//!   exactly match a registered id.
//! - [`malformed_manifest_mutations_are_always_caught`]: random single-field mutations of a valid
//!   manifest are always caught by `validate_manifest`, with zero panics.
//!
//! Plus one explicit, non-randomized structural test:
//! [`strategy_input_is_structurally_annotation_free`], documenting that annotation isolation is a
//! compile-time property of `StrategyInput`'s field list, not a runtime behavior to fuzz.

use std::collections::BTreeSet;

use proptest::prelude::*;
use proptest::sample::select;

use limen_core::canonical;
use limen_core::error::EngineError;
use limen_core::fixtures::minimal_valid_manifest;
use limen_core::model::{
    Budget, ContextItem, EvidenceSpan, ExpectedFact, FactComponent, ScenarioManifest, StrategyInput,
};
use limen_core::validate::{codes, validate_manifest};
use limen_core::{all_scenario_ids, get_scenario, list_strategy_ids, resolve_strategy};

// ---------------------------------------------------------------------------------------------
// Shared generators
// ---------------------------------------------------------------------------------------------

fn arb_section_label() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("log_line".to_string()),
        Just("chat_message".to_string()),
        Just("spec_section".to_string()),
        Just("note".to_string()),
    ]
}

/// Printable ASCII plus a few punctuation/digit-heavy patterns, so the tokenizer's alnum-run,
/// punctuation, and whitespace rules are all exercised without needing to model full Unicode
/// here (the tokenizer's own unit tests already cover non-ASCII scalars in isolation).
fn arb_text() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 .,!?\n_-]{0,160}"
}

fn arb_raw_item() -> impl Strategy<Value = (String, String)> {
    (arb_section_label(), arb_text())
}

/// A small, randomly generated [`StrategyInput`] fixture: 0-8 items, each with a unique
/// `source_id`/`order_index` derived from its position (uniqueness matters for the "one record
/// per item" invariant to even be well-posed).
fn arb_generated_strategy_input() -> impl Strategy<Value = StrategyInput> {
    (arb_text(), proptest::collection::vec(arb_raw_item(), 0..8)).prop_map(
        |(task_query, raw_items)| {
            let items = raw_items
                .into_iter()
                .enumerate()
                .map(|(i, (section_label, text))| ContextItem {
                    source_id: format!("item-{i}"),
                    order_index: i as u32,
                    section_label,
                    text,
                })
                .collect();
            StrategyInput { task_query, items }
        },
    )
}

/// Either one of the 3 real embedded scenarios (projected to its `StrategyInput`) or a small
/// randomly generated fixture, per the required test matrix.
fn arb_strategy_input() -> impl Strategy<Value = StrategyInput> {
    let real_scenario = select(all_scenario_ids().to_vec()).prop_map(|id| {
        get_scenario(id)
            .expect("known scenario id")
            .to_strategy_input()
    });
    prop_oneof![
        3 => real_scenario,
        7 => arb_generated_strategy_input(),
    ]
}

fn arb_strategy_id() -> impl Strategy<Value = &'static str> {
    select(list_strategy_ids().to_vec())
}

/// A "generous max" requested-token budget: comfortably larger than any single generated or real
/// scenario's full concatenated token count, so both "budget is the binding constraint" and
/// "budget is ample" regions get exercised.
const MAX_REQUESTED_TOKENS: u32 = 5_000;

// ---------------------------------------------------------------------------------------------
// Budget safety
// ---------------------------------------------------------------------------------------------

proptest! {
    #[test]
    fn budget_safety_never_exceeds_requested(
        input in arb_strategy_input(),
        strategy_id in arb_strategy_id(),
        requested_tokens in 0u32..MAX_REQUESTED_TOKENS,
    ) {
        let strategy = resolve_strategy(strategy_id).expect("registered id must resolve");
        let output = strategy.select(&input, &Budget { requested_tokens });

        prop_assert!(
            output.budget.used_tokens <= output.budget.requested_tokens,
            "{strategy_id}: used_tokens {} exceeded requested_tokens {}",
            output.budget.used_tokens,
            output.budget.requested_tokens,
        );
        prop_assert_eq!(output.budget.requested_tokens, requested_tokens);
        prop_assert_eq!(
            output.budget.remaining_tokens,
            requested_tokens - output.budget.used_tokens
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Determinism
// ---------------------------------------------------------------------------------------------

proptest! {
    #[test]
    fn determinism_select_twice_is_byte_identical(
        input in arb_strategy_input(),
        strategy_id in arb_strategy_id(),
        requested_tokens in 0u32..MAX_REQUESTED_TOKENS,
    ) {
        let strategy = resolve_strategy(strategy_id).expect("registered id must resolve");
        let budget = Budget { requested_tokens };

        let first = strategy.select(&input, &budget);
        let second = strategy.select(&input, &budget);

        prop_assert_eq!(&first, &second, "select() must be a pure function of its inputs");

        let first_bytes = canonical::canonical_bytes(&first).expect("plain data always canonicalizes");
        let second_bytes = canonical::canonical_bytes(&second).expect("plain data always canonicalizes");
        prop_assert_eq!(first_bytes, second_bytes);
    }
}

// ---------------------------------------------------------------------------------------------
// Total tie-breaks / stable ordering
// ---------------------------------------------------------------------------------------------

proptest! {
    #[test]
    fn selection_is_one_record_per_item_sorted_by_order_index(
        input in arb_strategy_input(),
        strategy_id in arb_strategy_id(),
        requested_tokens in 0u32..MAX_REQUESTED_TOKENS,
    ) {
        let strategy = resolve_strategy(strategy_id).expect("registered id must resolve");
        let output = strategy.select(&input, &Budget { requested_tokens });

        prop_assert_eq!(output.selection.len(), input.items.len());

        let mut seen_source_ids: BTreeSet<&str> = BTreeSet::new();
        for record in &output.selection {
            prop_assert!(
                seen_source_ids.insert(record.source_id.as_str()),
                "duplicate source_id {} in selection",
                record.source_id
            );
        }
        let expected_ids: BTreeSet<&str> =
            input.items.iter().map(|i| i.source_id.as_str()).collect();
        prop_assert_eq!(seen_source_ids, expected_ids);

        for pair in output.selection.windows(2) {
            prop_assert!(
                pair[0].order_index <= pair[1].order_index,
                "selection not sorted ascending by order_index: {} then {}",
                pair[0].order_index,
                pair[1].order_index
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Unknown strategy/version rejection
// ---------------------------------------------------------------------------------------------

proptest! {
    #[test]
    fn resolve_strategy_never_panics_and_rejects_unknown_ids(
        id in "[a-zA-Z0-9@._-]{0,40}",
    ) {
        let result = resolve_strategy(&id);
        let is_registered = list_strategy_ids().contains(&id.as_str());

        if is_registered {
            prop_assert!(result.is_ok(), "registered id {id} must resolve");
        } else {
            match result.err() {
                Some(EngineError::UnknownStrategyVersion(got)) => prop_assert_eq!(got, id),
                other => prop_assert!(
                    false,
                    "expected Some(UnknownStrategyVersion) for unregistered id {id:?}, got {other:?}"
                ),
            }
        }
    }

    /// Plausible-but-wrong version suffixes on a *known* family (e.g. `"recency@2"`,
    /// `"recency@999"`) must also be rejected structurally, never panic and never silently
    /// resolve to some other version.
    #[test]
    fn resolve_strategy_rejects_known_family_with_wrong_version_suffix(
        family in select(vec!["full-input-truncation", "recency", "structured-extraction", "hierarchical-summary", "retrieval-ranking"]),
        version in 2u32..1000,
    ) {
        let id = format!("{family}@{version}");
        prop_assert!(!list_strategy_ids().contains(&id.as_str()));
        let err = resolve_strategy(&id).err().expect("must be rejected");
        prop_assert_eq!(err.code(), "unknown_strategy_version");
    }
}

// ---------------------------------------------------------------------------------------------
// Malformed manifest rejection
// ---------------------------------------------------------------------------------------------

/// One kind of single-aspect mutation applied to an otherwise-valid manifest, mirroring the
/// rejection codes in `validate.rs`. Carries whatever randomized parameters that mutation needs.
#[derive(Debug, Clone)]
enum Mutation {
    DuplicateSourceId,
    DuplicateFactId,
    DuplicateComponentId,
    DuplicateGroupId,
    EvidenceSpanInvalidRange,
    EvidenceSpanOutOfBounds,
    EvidenceSpanSplitsCodepoint,
    EvidenceSpanUnknownSource(String),
    ExpectedCitationUnknownSource(String),
    DistractorUnknownSource(String),
    EmptyRequiredFacts,
    FactZeroComponents(String),
    ComponentZeroEvidence(String),
    MalformedSchemaVersion(String),
    MalformedScenarioVersion(String),
    StaleContentDigest(String),
}

fn arb_ghost_id() -> impl Strategy<Value = String> {
    "[a-z]{4,12}".prop_map(|s| format!("ghost-{s}"))
}

fn arb_mutation() -> impl Strategy<Value = Mutation> {
    prop_oneof![
        Just(Mutation::DuplicateSourceId),
        Just(Mutation::DuplicateFactId),
        Just(Mutation::DuplicateComponentId),
        Just(Mutation::DuplicateGroupId),
        Just(Mutation::EvidenceSpanInvalidRange),
        Just(Mutation::EvidenceSpanOutOfBounds),
        Just(Mutation::EvidenceSpanSplitsCodepoint),
        arb_ghost_id().prop_map(Mutation::EvidenceSpanUnknownSource),
        arb_ghost_id().prop_map(Mutation::ExpectedCitationUnknownSource),
        arb_ghost_id().prop_map(Mutation::DistractorUnknownSource),
        Just(Mutation::EmptyRequiredFacts),
        "[a-z]{3,10}".prop_map(Mutation::FactZeroComponents),
        "[a-z]{3,10}".prop_map(Mutation::ComponentZeroEvidence),
        // No dots at all -> always fails the exactly-3-numeric-parts check regardless of digits.
        "v?[0-9]{1,4}".prop_map(Mutation::MalformedSchemaVersion),
        // Exactly 2 dot-separated numeric parts -> always fails the exactly-3-parts check.
        "[0-9]{1,3}\\.[0-9]{1,3}".prop_map(Mutation::MalformedScenarioVersion),
        "[a-zA-Z0-9 ]{1,20}".prop_map(Mutation::StaleContentDigest),
    ]
}

/// Applies `mutation` to `manifest` in place, returning the rejection code that application is
/// guaranteed to trigger, or `None` if this mutation's precondition no longer holds (relevant
/// only when chaining several mutations onto one manifest in
/// [`compound_mutation_never_panics_and_reports_every_problem`]: e.g. an earlier
/// `EmptyRequiredFacts` mutation removes the `required_facts[0]` that a later
/// `EvidenceSpanInvalidRange` mutation would otherwise assume exists). Every branch checks its own
/// preconditions before indexing, so this never panics regardless of manifest state or mutation
/// order; applied singly to a fresh [`minimal_valid_manifest`] (as
/// [`malformed_manifest_mutations_are_always_caught`] does), every branch's precondition always
/// holds and this always returns `Some`.
fn apply_mutation(manifest: &mut ScenarioManifest, mutation: &Mutation) -> Option<&'static str> {
    match mutation {
        Mutation::DuplicateSourceId => {
            let mut dup = manifest.items.first()?.clone();
            dup.order_index = manifest.items.len() as u32 + 1000;
            manifest.items.push(dup);
            Some(codes::DUPLICATE_SOURCE_ID)
        }
        Mutation::DuplicateFactId => {
            let dup = manifest.annotations.required_facts.first()?.clone();
            manifest.annotations.required_facts.push(dup);
            Some(codes::DUPLICATE_FACT_ID)
        }
        Mutation::DuplicateComponentId => {
            let dup = manifest
                .annotations
                .required_facts
                .first()?
                .components
                .first()?
                .clone();
            manifest.annotations.required_facts[0].components.push(dup);
            Some(codes::DUPLICATE_COMPONENT_ID)
        }
        Mutation::DuplicateGroupId => {
            let dup = manifest.annotations.contradiction_groups.first()?.clone();
            manifest.annotations.contradiction_groups.push(dup);
            Some(codes::DUPLICATE_GROUP_ID)
        }
        Mutation::EvidenceSpanInvalidRange => {
            let evidence = manifest
                .annotations
                .required_facts
                .first_mut()?
                .components
                .first_mut()?
                .evidence
                .first_mut()?;
            evidence.byte_start = 3;
            evidence.byte_end = 3;
            Some(codes::EVIDENCE_SPAN_INVALID_RANGE)
        }
        Mutation::EvidenceSpanOutOfBounds => {
            let source_id = manifest
                .annotations
                .required_facts
                .first()?
                .components
                .first()?
                .evidence
                .first()?
                .source_id
                .clone();
            let text_len = manifest
                .items
                .iter()
                .find(|i| i.source_id == source_id)?
                .text
                .len() as u32;
            let evidence = manifest.annotations.required_facts[0].components[0]
                .evidence
                .first_mut()?;
            evidence.byte_start = text_len + 5;
            evidence.byte_end = text_len + 15;
            Some(codes::EVIDENCE_SPAN_OUT_OF_BOUNDS)
        }
        Mutation::EvidenceSpanSplitsCodepoint => {
            manifest
                .annotations
                .required_facts
                .first()?
                .components
                .first()?
                .evidence
                .first()?;
            let source_id = "prop-multibyte-source".to_string();
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
            Some(codes::EVIDENCE_SPAN_SPLITS_CODEPOINT)
        }
        Mutation::EvidenceSpanUnknownSource(ghost) => {
            let evidence = manifest
                .annotations
                .required_facts
                .first_mut()?
                .components
                .first_mut()?
                .evidence
                .first_mut()?;
            evidence.source_id = ghost.clone();
            Some(codes::EVIDENCE_SPAN_UNKNOWN_SOURCE)
        }
        Mutation::ExpectedCitationUnknownSource(ghost) => {
            manifest
                .annotations
                .required_facts
                .first_mut()?
                .expected_citation_source_ids
                .push(ghost.clone());
            Some(codes::EXPECTED_CITATION_UNKNOWN_SOURCE)
        }
        Mutation::DistractorUnknownSource(ghost) => {
            manifest
                .annotations
                .distractor_source_ids
                .insert(ghost.clone());
            Some(codes::DISTRACTOR_UNKNOWN_SOURCE)
        }
        Mutation::EmptyRequiredFacts => {
            if manifest.annotations.required_facts.is_empty() {
                return None;
            }
            manifest.annotations.required_facts.clear();
            Some(codes::EMPTY_REQUIRED_FACTS)
        }
        Mutation::FactZeroComponents(suffix) => {
            manifest.annotations.required_facts.push(ExpectedFact {
                fact_id: format!("f-empty-{suffix}"),
                statement: "statement".to_string(),
                why_it_matters: "why".to_string(),
                components: vec![],
                expected_citation_source_ids: vec![],
            });
            Some(codes::FACT_ZERO_COMPONENTS)
        }
        Mutation::ComponentZeroEvidence(suffix) => {
            manifest
                .annotations
                .required_facts
                .first_mut()?
                .components
                .push(FactComponent {
                    component_id: format!("c-empty-{suffix}"),
                    evidence: vec![],
                    canonical_value: None,
                    required_qualifiers: vec![],
                });
            Some(codes::COMPONENT_ZERO_EVIDENCE)
        }
        Mutation::MalformedSchemaVersion(v) => {
            manifest.schema_version = format!("v{v}");
            Some(codes::MALFORMED_SCHEMA_VERSION)
        }
        Mutation::MalformedScenarioVersion(v) => {
            manifest.scenario_version = v.clone();
            Some(codes::MALFORMED_SCENARIO_VERSION)
        }
        Mutation::StaleContentDigest(suffix) => {
            manifest.title = format!("{} {suffix}", manifest.title);
            Some(codes::STALE_CONTENT_DIGEST)
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]
    #[test]
    fn malformed_manifest_mutations_are_always_caught(mutation in arb_mutation()) {
        let mut manifest = minimal_valid_manifest();
        // A single mutation applied to a fresh, always-valid fixture always meets its own
        // precondition (see `apply_mutation`'s doc comment), so this is always `Some`.
        let expected_code =
            apply_mutation(&mut manifest, &mutation).expect("precondition always holds here");

        let errors = validate_manifest(&manifest);
        prop_assert!(
            !errors.is_empty(),
            "mutation {mutation:?} produced zero validation errors"
        );
        prop_assert!(
            errors.iter().any(|e| e.code == expected_code),
            "mutation {mutation:?} expected code {expected_code}, got: {errors:?}"
        );
    }

    /// Chains 1-3 mutations onto one manifest. Some combinations target the same field (e.g. two
    /// evidence-span mutations both overwrite `evidence[0]`, so only the *last* one's effect
    /// actually survives to be validated) -- so this does not assert every individual mutation's
    /// code independently survives (that would be a false expectation about mutation order, not
    /// a real crate bug). What it does prove, across arbitrary combinations and orderings: at
    /// least one problem is always still caught, and `validate_manifest` never panics no matter
    /// how many simultaneous or overlapping problems a manifest has.
    #[test]
    fn compound_mutation_never_panics_and_always_catches_something(
        mutations in proptest::collection::vec(arb_mutation(), 1..4),
    ) {
        let mut manifest = minimal_valid_manifest();
        let applied_count = mutations
            .iter()
            .filter(|m| apply_mutation(&mut manifest, m).is_some())
            .count();

        let errors = validate_manifest(&manifest);
        if applied_count > 0 {
            prop_assert!(
                !errors.is_empty(),
                "expected at least one validation error after {applied_count} applied mutation(s), got zero"
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Annotation isolation: structural, not behavioral
// ---------------------------------------------------------------------------------------------

/// `StrategyInput` structurally cannot carry annotation data (expected facts, distractor ids,
/// contradiction groups): those fields simply do not exist on the type. This is a compile-time
/// guarantee, not a runtime one, so it is proven here by exhaustive field destructuring (no `..`
/// rest pattern) rather than by fuzzing: if a future change ever added an annotation field to
/// either type below, the destructuring patterns here would fail to compile, forcing this test to
/// be revisited rather than silently passing.
#[test]
fn strategy_input_is_structurally_annotation_free() {
    let manifest = minimal_valid_manifest();
    let input = manifest.to_strategy_input();

    // Exhaustive: this line only compiles because `StrategyInput` has exactly these two fields.
    let StrategyInput { task_query, items } = input;
    assert_eq!(task_query, manifest.task_query);
    assert_eq!(items.len(), manifest.items.len());

    // Exhaustive: `ContextItem` itself carries no annotation fields either (no expected-fact
    // ids, no distractor flag, no contradiction-group membership reachable from an item).
    for item in items {
        let ContextItem {
            source_id: _,
            order_index: _,
            section_label: _,
            text: _,
        } = item;
    }
}
