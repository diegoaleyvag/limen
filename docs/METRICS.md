# Metrics

This document formalizes the metric definitions implemented in
[`crates/limen-core/src/metrics.rs`](../crates/limen-core/src/metrics.rs). That module's own
doc comment is the ground truth; this page restates and organizes it for readers who are not
reading Rust source, without inventing any new semantics. If the two ever disagree, trust the
Rust source (and its test suite) over this document.

All metrics are computed by `compute_metrics(annotations, selection, original_items)`, which
depends only on a strategy's `SelectionOutput` and the scenario's `ScenarioAnnotations` (never on
*which* strategy produced the selection) -- so the same metric definitions apply uniformly to every
strategy, present and future.

## Fact recall

**A `FactComponent` is retained iff at least one of its `evidence` alternatives is spatially
retained** -- its source item is fully `Included`, or the evidence span lies entirely inside a
`Partial` item's retained byte sub-range -- **and**, if the component has a `canonical_value` and/or
`required_qualifiers`, the exact normalized value/unit/qualifier text is present verbatim in "the
corresponding output" for that alternative: the verbatim retained slice of the source text for
ordinary (non-transform) records, or `output_text` for transform records (structured-extraction,
hierarchical-summary).

This gives two independent multiplicities, both real and distinct:

- **Composite-AND**: an `ExpectedFact` is retained iff **all** of its `components` are retained. A
  fact with `components.len() > 1` genuinely needs every part to survive together to still support
  the claim (e.g. an incident's start time *and* end time, both needed to support a stated
  duration).
- **Redundant-OR**: within one component, `evidence` is a list of redundant alternatives -- **any
  one** surviving alternative is sufficient for that component to count as retained. This models
  the same fact being stated in more than one source.

`FactRecall` reports `retained` (count) over `required` (count) -- **the denominator is always the
total number of `required_facts` in the scenario, and is always visible alongside the numerator**,
never hidden behind a bare percentage. `per_fact: Vec<FactRecallDetail>` gives a per-fact
`retained: bool` and, for facts that were not retained, the exact `missing_components` (by
`component_id`) so a user can see *which* atomic piece was lost, not just that the fact failed
overall.

## Contradiction outcomes

Each `ContradictionGroup` (a set of competing claim locations) is scored into exactly one of four
outcomes, based purely on which members survived selection -- **never a truth judgement about which
claim was correct**:

- `AllRetained` -- every member is spatially retained (or the group has zero members).
- `NoneRetained` -- no member is spatially retained.
- `Split` -- some retained, some dropped, **and** at least one dropped member was dropped for a
  budget-cutoff reason.
- `PartialWithinRetained` -- some retained, some dropped, but every dropped member was dropped for
  a non-budget (content-level) reason.

### The precise `Split` vs `PartialWithinRetained` rule

The distinction is about *why* each dropped member was actually dropped, not about geometric
position in `order_index` space. Every strategy's `TraceStep.action` for a dropped item already
names a specific reason, and that reason is either a budget-cutoff reason -- the item was dropped
because the budget ran out (`"dropped_over_budget"` for full-input-truncation/
structured-extraction/hierarchical-summary, `"dropped_too_old"` for recency,
`"dropped_below_budget"` for retrieval-ranking) -- or a content-level reason unrelated to the
budget boundary (currently only structured-extraction's `"dropped_no_extractable_content"`, for an
item with nothing digit-bearing to extract, regardless of budget). A mixed group is `Split` iff
**at least one** of its dropped members hit a budget-cutoff reason; it is `PartialWithinRetained`
iff **every** dropped member was dropped for a non-budget reason.

This definition is deliberately not tied to a global geometric boundary (e.g. "the single highest
dropped `order_index`" or "the single lowest retained `order_index`"): a dropped member need not be
the whole-selection's most-extreme dropped item to count, as long as its own drop was genuinely a
budget decision. This also generalizes correctly to `retrieval-ranking@1`'s non-contiguous
bin-packing, where a lower-ranked item can survive after a higher-ranked one was skipped -- there
is no single geometric cutoff point in `order_index` space there at all.

This is a precise, testable, purely structural rule about *why* a member was dropped -- it says
nothing about which claim in the group was true, and scenario authors must never encode such a
judgement into a `ContradictionGroup` (see [`docs/SCENARIO_AUTHORING.md`](SCENARIO_AUTHORING.md)).

A `Split` outcome also contributes a `ContradictionSplit` distortion indicator (see below).

## Numeric/date accuracy

For every `FactComponent` whose `canonical_value` is `Number { normalized, unit }` or
`Date { normalized }` (i.e. excluding the free-text `Text` variant), the component is `checked`,
and it is `exact` iff it is retained under the same fact-recall rule above -- which, for a numeric
or date component, specifically means the exact `normalized` value **and** every `unit`/
`required_qualifiers` string are present verbatim in the corresponding output text. There is no
partial credit and no numeric tolerance: a value that survives with its unit silently dropped, or
a qualifier (e.g. a negation word) silently dropped, is not exact.

`NumericDateAccuracy` reports `exact` over `checked`, plus `mismatches: Vec<String>` -- the
`fact_id`s of every checked component that was not exactly retained (may contain repeats if one
fact has multiple mismatching numeric/date components). Despite the name, a `mismatches` entry
does **not** imply the value was present-but-wrong -- the far more common cause is that the value
was dropped/omitted entirely (its source item never survived selection, or a transform's
`output_text` never carried it). "Mismatch" here means "did not exactly retain," covering both
omission and (less commonly) an altered/incomplete value, e.g. a number surviving with its unit
silently dropped.

## Citation retention

Citation retention is **per-fact, not per-source**, and counts **only** facts with a non-empty
`expected_citation_source_ids` -- a fact with no citation expectation has nothing to check, and
counting it as either retained or not-retained would artificially inflate or deflate the
denominator. For each counted fact, its citation is "retained" iff **every** one of its
`expected_citation_source_ids` has a non-`Dropped` status in the selection (the cited *source* item
is still present/attributable at all, independent of whether that fact's own byte-level evidence
happened to survive).

`CitationRetention` reports `retained` over `expected` (both counts already restricted to facts
with a citation expectation), plus `per_fact: Vec<(String, bool)>`.

## Budget accounting

`BudgetUsage` always reports three integers: `requested_tokens`, `used_tokens`, and
`remaining_tokens`, with the invariant `used_tokens <= requested_tokens` (so
`remaining_tokens = requested_tokens - used_tokens` never underflows) enforced by
`BudgetUsage::new`, which clamps rather than panics. Token counts are always counted by the
`limen-lex@1.0.0` tokenizer over the *emitted* output text (including any strategy-added
label/template text), never over raw source text and never as a float or approximation.

## Distortion indicators

`distortion_indicators: Vec<DistortionIndicator>` is a flat list of exactly five variants -- an
enumerated, closed taxonomy of *observable* structural events, never a subjective or open-ended
"quality" judgement:

- `EvidenceClipped { fact_id }` -- at least one evidence alternative for this fact was clipped by a
  `Partial` item's cut (the evidence span existed in the source but the retained sub-range did not
  fully cover it).
- `QualifierDropped { fact_id, qualifier }` -- a specific required qualifier (e.g. a unit or a
  negation word) is missing from an otherwise spatially-retained component's output.
- `ProvenanceReordered { source_id }` -- a retained item appears "out of place": walking the
  selection in its own (output) order, this item's `order_index` is lower than the highest
  `order_index` already seen among retained items so far.
- `ContradictionSplit { group_id }` -- this contradiction group's outcome was `Split` (see above).
- `DistractorRetained { source_id }` -- a scenario-annotated distractor item survived selection
  (non-`Dropped`).

The list is assembled in one fully specified order: for each `required_facts` entry, in manifest
order, first an `EvidenceClipped` (if any) then any `QualifierDropped` entries (by component, then
by qualifier, in authored order); then all `ProvenanceReordered` entries, in selection (output)
order; then all `ContradictionSplit` entries, in `contradiction_groups` order; then all
`DistractorRetained` entries, in `distractor_source_ids` sorted (`BTreeSet`) order.

## Non-claims

**These metrics evaluate selected context against annotated expected facts. They do not claim,
measure, or predict downstream LLM answer quality or "real model performance."**

- Fact recall, contradiction outcomes, numeric/date accuracy, and citation retention are all
  computed relative to a scenario's own hand-authored annotations (`required_facts`,
  `contradiction_groups`, `expected_citation_source_ids`) -- they say what a given selection kept
  or dropped *relative to that answer key*, not whether a downstream model would answer correctly,
  partially correctly, or incorrectly if given that selection as context.
- Contradiction outcomes are purely structural (which members of a competing-claims group
  survived selection) and are never a truth judgement about which claim was correct.
- No metric in this document implies that any of the five deterministic strategies exhibits any
  form of intelligence, understanding, or reasoning about the text. Every strategy is mechanical
  and auditable (see [`docs/STRATEGIES.md`](STRATEGIES.md)); the metrics measure the mechanical
  consequences of that mechanism, nothing more.
