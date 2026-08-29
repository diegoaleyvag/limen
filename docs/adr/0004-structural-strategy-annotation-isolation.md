# 0004. Structural (type-level) isolation of strategy input from evaluator annotations

## Context

Limen's metrics are only meaningful if a strategy's selection decision is genuinely blind to the
answer key it will later be scored against -- which items are distractors, which facts are
required, which spans contradict each other. A convention like "strategies must not read
`annotations`" is easy to state but easy to violate by accident (a future strategy implementation
could reach into a shared scenario object and read an annotation field that happens to be present,
even unintentionally, e.g. while debugging or refactoring), and a violation would be silent: nothing
would fail loudly, the metrics would just quietly stop meaning what they claim to mean.

## Decision

Annotation-freeness is enforced by the Rust type system, not by a code-review convention.
`ScenarioManifest::to_strategy_input()` projects a full manifest down to `StrategyInput`
(`crates/limen-core/src/model.rs`), a distinct struct containing only `task_query` and `items`.
`StrategyInput` has no field, of any name or type, that could carry `required_facts`,
`distractor_source_ids`, or `contradiction_groups` -- they are not merely omitted at
construction time, they do not exist on the type at all. `SelectionStrategy::select` (the trait
every strategy implements, `strategy/mod.rs`) takes only `&StrategyInput` and `&Budget`; there is
no field path, reflection trick, or accidental pass-through by which an implementation could reach
`ScenarioAnnotations` even if it tried, short of deliberately threading a second parameter through
the trait signature -- which would be an obvious, reviewable change to `strategy/mod.rs` itself,
not a quiet one inside a strategy's own file.

`crates/limen-core/tests/property_tests.rs`'s `strategy_input_is_structurally_annotation_free`
test documents this explicitly as a compile-time property of `StrategyInput`'s field list, not a
runtime behavior that needs fuzzing to catch a violation.

`crate::metrics::compute_metrics` sits on the other side of this boundary: it depends only on a
`SelectionOutput` (which a strategy already produced, with no annotation visibility) plus the
`ScenarioAnnotations` the strategy never saw, and explicitly never depends on which concrete
strategy produced the selection.

## Consequences

- **Annotation leakage is a compile error, not a bug to discover at runtime.** A hypothetical
  future strategy that tried to read `fact.why_it_matters` or a distractor flag simply has no
  field to read; the mistake is caught by `cargo build`, long before any test would need to catch
  it behaviorally.
- **Metrics are guaranteed complete and stable across future strategy work.**
  `metrics.rs`'s own module doc comment states this consequence directly: because
  `compute_metrics` depends only on `SelectionOutput`/`ScenarioAnnotations`/original items, never
  on which strategy ran, it is complete and final as of this phase -- a later strategy-
  implementation phase must not need to change anything in `metrics.rs`.
- **`ContextItem.section_label` is deliberately still strategy-visible** (e.g. `"log_line"`,
  `"chat_message"`) since it is a structural hint about the input's shape, not evaluator intent;
  `model.rs`'s doc comment on `ContextItem` makes explicit that this field "must never reveal
  evaluator intent (which facts matter, which items are distractors, etc.)" -- the isolation
  boundary is about annotation content, not all metadata whatsoever.
- **The UI is allowed to show annotation content to the human user** (e.g. `why_it_matters`,
  distractor/contradiction membership) for teaching purposes, via a separate WASM call
  (`get_scenario_detail`) that returns the full `ScenarioManifest` including `annotations` -- this
  is a deliberately distinct code path from `run_trial`'s `StrategyInput` projection, and does not
  weaken the strategy-side guarantee, since the strategy itself never receives that data.

## Alternatives considered

- **A runtime check/assertion that a strategy "didn't access" annotation fields.** Rejected:
  impossible to enforce meaningfully in Rust without the type-level guarantee anyway (there is no
  general way to observe "did this function read this field" at runtime for a plain struct
  access), and would only catch a violation after it already happened rather than preventing it.
- **A single shared `Scenario` struct passed to strategies, documented as "annotations must be
  ignored."** Rejected: this is exactly the convention-based approach this decision replaces; it
  compiles cleanly even when violated, which is the failure mode this ADR exists to close off.
- **Passing annotations to strategies but only in a debug/test build.** Rejected: this would
  create two different code shapes for `select` (test vs. release) and reintroduce exactly the
  "two implementations that must be kept in sync" risk ADR 0001 avoids at the WASM boundary, for
  no real benefit -- the type-level projection is available and enforced identically in every
  build.
