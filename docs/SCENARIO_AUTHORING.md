# Scenario authoring

This document explains how a Limen scenario manifest is put together, how it is authored safely
in practice, and the ground rules that keep scenario content fair, checkable, and reusable. It is
a first draft (a later documentation phase will polish it) but every claim in it is accurate as of
the three real scenarios under `scenarios/v1/`: `incident-investigation`, `product-comparison`,
and `requirements-architecture-review`.

Treat the Rust types themselves as ground truth over this document if the two ever disagree:
`crates/limen-core/src/model.rs` (the manifest shape), `crates/limen-core/src/validate.rs` (what
is actually rejected), `crates/limen-core/src/canonical.rs` (digesting), and
`crates/limen-core/src/fixtures.rs` (a small, always-valid worked example). The generated
`schemas/scenario-manifest-v1.schema.json` is a useful second cross-check of the shape.

## The manifest shape, in plain language

A scenario manifest (`ScenarioManifest`) is one JSON file with two halves:

- **What a strategy is allowed to see** (`items`, plus `task_query`): an ordered list of
  `ContextItem`s. Each item has a `source_id` (unique within the scenario), a 0-based
  `order_index` matching its position in `items`, a `section_label` (a strategy-visible
  structural hint like `"log_line"` or `"chat_message"` -- never anything that reveals which
  facts matter or which items are distractors), and the item's verbatim `text`. This is exactly
  what `ScenarioManifest::to_strategy_input()` projects out as `StrategyInput`; nothing else in
  the manifest is reachable from that type, so a strategy implementation has no field path to the
  evaluator-only half even by accident.
- **What only the evaluator sees** (`annotations`, a `ScenarioAnnotations`): the answer key.
  - `required_facts: Vec<ExpectedFact>` -- the facts a good strategy should preserve.
  - `distractor_source_ids: BTreeSet<String>` -- items that are plausible-looking but not needed
    to answer `task_query`.
  - `contradiction_groups: Vec<ContradictionGroup>` -- sets of competing claims (see below).

An `ExpectedFact` has a `fact_id`, a human-readable `statement`, a `why_it_matters` explanation (a
real explanation, not filler -- it should say *why a reader/system would care*, not just restate
the statement), a list of `expected_citation_source_ids` (the sources a correct answer should be
able to cite), and one or more `components: Vec<FactComponent>`.

A `FactComponent` is the atomic, independently-checkable unit: it has a `component_id`, a list of
`evidence: Vec<EvidenceSpan>` (byte-span pointers into a source item's `text`), an optional
`canonical_value` (see below), and a list of `required_qualifiers` (exact substrings, e.g. a unit
or a negation word, that must survive verbatim alongside the value).

Every `FactComponent` must have at least one of `canonical_value` or a non-empty
`required_qualifiers` (`validate_manifest` rejects `component_uncheckable_value` otherwise): with
neither, retention would be decided purely by evidence's spatial (byte-range) provenance pointer,
with no check that the retained/transformed output text actually still contains anything of the
value -- a real gap for a transform strategy, whose `output_text` could in principle omit the fact
while the byte-range pointer still "covers" the right region of the source.

A `CanonicalValue` is how a component's value is checked, as plain text rather than a parsed
number/date type (see `model.rs`'s doc comment for why: exact substring checks, no float precision
or locale ambiguity):

- `Number { normalized, unit }` -- e.g. `normalized: "420"`, `unit: Some("ms")`.
- `Date { normalized }` -- e.g. `"2025-03-11"`, written directly by the author, never derived via
  locale-aware date parsing.
- `Text { normalized }` -- any other exact-substring value.

## Composite facts vs. redundant evidence -- two different multiplicities, easy to mix up

These are opposite axes of the same `ExpectedFact`, and authoring the wrong one where you meant
the other is the single easiest scenario-authoring mistake to make:

- **Composite** = more than one `FactComponent` on the *same fact*
  (`fact.components.len() > 1`). This means the fact needs **all** of its components to count as
  retained -- it is an AND. Use this when a fact genuinely has multiple independent parts that
  must *all* survive together to be meaningful. For example, `incident-investigation`'s
  `f-incident-duration` fact has a `c-incident-start` component (the "14:07" start time) and a
  separate `c-incident-end` component (the "14:52" end time): keeping only one endpoint cannot
  support the fact's own claimed 45-minute duration, so both are required. Each component in a
  composite fact must be independently evidenced -- give each one its own `evidence` list backed
  by its own span(s), not a shared span that happens to mention both things.
- **Redundant evidence** = more than one `EvidenceSpan` inside the *same component's* `evidence`
  list (`component.evidence.len() > 1`). This means **any one** surviving alternative is enough
  for that component to count as retained -- it is an OR. Use this when the same underlying value
  is genuinely stated in more than one place in the source material, so a strategy that kept
  *either* mention should get credit. For example, `product-comparison`'s `f-corsair-pricing`
  fact's price component is evidenced both by the spec sheet's `"$18 per seat per month"` and by a
  customer quote's `"$18 per seat"` -- two different sources independently saying the same $18
  figure.

A fact can combine both: `f-incident-duration` above has two components (composite), and each of
those two components separately has two redundant evidence alternatives (one from a chat message,
one from the postmortem note repeating the same time).

## Deriving byte spans safely: the `str::find` pattern

Every `EvidenceSpan` is a `{ source_id, byte_start, byte_end }` pointing at `text[byte_start..byte_end]`
of that source item, where `byte_end` is exclusive and both offsets are UTF-8 codepoint boundaries.
Counting these by hand is exactly the kind of mechanical, easy-to-silently-miscount work that
should never be done by a human: a single earlier edit to the prose shifts every later hand-counted
offset in that item, invisibly.

Instead, author the item text first, then derive every span from it programmatically. The pattern
used throughout `crates/limen-core/examples/author_scenarios.rs` is a single small helper:

```rust
fn evidence(items: &[ContextItem], source_id: &str, needle: &str) -> EvidenceSpan {
    let source_item = items.iter().find(|i| i.source_id == source_id)
        .unwrap_or_else(|| panic!("no item with source_id {source_id:?}"));
    let start = source_item.text.find(needle)
        .unwrap_or_else(|| panic!("needle {needle:?} not found in {source_id:?}"));
    EvidenceSpan {
        source_id: source_id.to_string(),
        byte_start: start as u32,
        byte_end: (start + needle.len()) as u32,
    }
}
```

Guidance for using it well:

- Pick a `needle` that is the exact substring you want the span to cover -- typically the smallest
  span that still makes sense standing alone (a number, a clause, a short phrase), since that is
  what a `Partial` selection strategy might retain or clip.
- `str::find` returns the *first* occurrence, so choose needles that are unique within that one
  item's text (add a word or two of surrounding context if a short needle like a two-digit number
  could repeat). If you ever genuinely need a specific later occurrence, search from an offset
  (`text[prior_end..].find(needle)` and add `prior_end` back) rather than trusting the first match
  -- but in practice, writing distinct enough source text is simpler and more robust than that.
- Let this helper panic loudly (`cargo run` failing with a clear message) rather than silently
  producing a wrong span; a needle typo should never make it into a checked-in scenario file.
- When a component's `canonical_value` is `Some(...)`, prefer a needle whose text contains that
  exact `normalized` value (and any `required_qualifiers`) verbatim -- metrics computation later
  checks retained-output text for those exact substrings, so evidence spans and canonical values
  should agree about what text they are pointing at.
- This whole approach is why `validate_manifest`'s span checks (invalid range, out of bounds,
  splitting a codepoint, unknown source) essentially never fire for generator-produced content:
  the spans are correct by construction, not by careful counting.

If you ever add or reorder scenario prose by hand after the fact instead of regenerating, re-run
the generator (`cargo run -p limen-core --example author_scenarios`) rather than patching the JSON
directly -- hand-editing the JSON reintroduces exactly the hand-counted-offset risk this pattern
exists to avoid.

## Contradiction groups must stay adjudication-free

A `ContradictionGroup` (`{ group_id, members: Vec<EvidenceSpan> }`) records that two or more spans
make *competing* claims -- for example, one engineer's chat message blaming a deploy and another's
message saying the deploy was ruled out. The type has no field for which member is "true," and
that is a deliberate, structural guarantee, not just a convention: nothing about a
`ContradictionGroup` or how it is scored (`crate::metrics::ContradictionOutcome`) ever asks which
claim was correct. Metrics only ever measure *which members survived selection*
(`AllRetained` / `Split` / `PartialWithinRetained` / `NoneRetained`) -- never whether the surviving
one was the right one.

Practical implications for authoring:

- Never add a field, a naming convention, or a required-fact statement that says "member N of
  group G is the true one." If you want a required fact about the eventual documented conclusion
  (e.g. "the postmortem attributes the incident to the migration job, not the deploy"), that is
  fine -- it is just citing what a specific source (the postmortem) states, which is an
  objectively checkable fact about the text, not an adjudication of the contradiction group
  itself. Keep the two mechanisms conceptually separate: the contradiction group is about the
  *dispute existing*, a required fact about the resolution is about *what a specific later source
  documents*.
- A group can have more than two members. `incident-investigation`'s
  `g-incident-root-cause-dispute` has three: the original claim, a rebuttal, and a repeated
  restatement of the original claim -- a realistic back-and-forth, not a clean two-sided debate.
- Put members in different items when the scenario's contradiction is genuinely between two
  sources (this is the realistic case, and the one the `Split` vs. `PartialWithinRetained` metric
  distinction is designed around); nothing stops both spans from living in the same item, but that
  is a much weaker test of a strategy's behavior since a whole-item selection strategy would keep
  or drop both together.

## The digest: computing and verifying `content_digest`

`content_digest` is a `"sha256:<hex>"` string covering the *entire rest of the manifest*, computed
over its canonical JSON encoding (recursively sorted object keys, compact/no-whitespace encoding --
see `crate::canonical`) with the `content_digest` field itself entirely absent from the tree while
hashing (not blanked-to-empty-string; genuinely removed, since even an empty string is a value that
would change the hash).

Always compute it with the shared helper, never by hand:

```rust
manifest.content_digest = canonical::digest_with_field_blanked(&manifest, "content_digest")
    .expect("plain data always canonicalizes");
```

`validate_manifest` independently recomputes the same digest the same way and rejects the manifest
(`stale_content_digest`) if it does not match what is on disk. This means:

- The digest must be computed *last*, after every other field of the manifest is finalized --
  editing any item's text, any fact, any distractor, or the title after computing the digest will
  make the checked-in file fail validation on load.
- Pretty-printing the *file* on disk (`serde_json::to_string_pretty`) is fine and is what all three
  checked-in scenario files use -- only the digest computation itself needs the compact,
  sorted-key canonical form, and that happens automatically inside `digest_with_field_blanked`
  regardless of how the struct is later serialized for the file.
- If you hand-edit a checked-in scenario JSON file directly (instead of regenerating it), you must
  also manually recompute and update `content_digest`, which is exactly the kind of error-prone
  step the generator exists to avoid. Prefer changing the Rust source in `author_scenarios.rs` and
  re-running it.

## Content-safety rule: original and synthetic only

Every scenario is wholly invented: company names, product names, people, incidents, requirements,
numbers, dates, log lines, chat messages, and documents are all synthetic and created for this
project. None of it may be copied from, or be a close paraphrase of, any real certification exam
question, real vendor documentation, real incident writeup, or any personal or proprietary
material. This is a hard requirement, not a style preference -- treat any uncertainty about
whether a detail is "too close" to something real as a reason to invent a different detail rather
than to keep the original one.

This rule is compatible with (and does not conflict with) realism: source text should still read
like a real log line, chat message, spec sheet, or email, including plausible-but-wrong theories,
outdated/superseded claims, and messy human disagreement -- all of that is expected texture, as
long as the underlying company, product, person, and incident are inventions.

## Other structural coverage every real scenario is expected to meet

These are enforced by `crates/limen-core/tests/scenario_validation.rs` (structural coverage) and
`validate_manifest` (correctness), and are worth keeping in mind while drafting:

- 12-16 items, with sequential 0-based `order_index` values.
- 8-10 `required_facts`, each with a genuinely explanatory `why_it_matters`.
- At least one composite fact and, across the whole scenario set, at least one component with
  redundant evidence (see above).
- At least one non-empty `contradiction_groups` entry.
- At least 2 `distractor_source_ids` -- and a good distractor is a *plausible* red herring (an
  early theory that later gets explicitly ruled out, a discontinued/superseded feature, a
  same-keyword-but-unrelated reminder), not merely an item that is obviously off-topic.
- At least 3 distinct dates and 3 distinct numeric values backed by `CanonicalValue::Date`/
  `Number` across the scenario's required facts, with `required_qualifiers` used wherever a unit
  or a negation word (e.g. `"not"`) must survive verbatim for the fact to count as non-distorted.
- `expected_citation_source_ids` populated on every fact that should be attributable to specific
  sources.
- Prefer plain ASCII prose so byte spans stay simple to eyeball; the engine itself handles
  non-ASCII text correctly (see the codepoint-boundary checks in `validate.rs`), this is purely an
  authoring-risk-reduction convention.

## Regenerating and verifying

```bash
cargo run -p limen-core --example author_scenarios   # (re)generate scenarios/v1/*.json
cargo test -p limen-core --test scenario_validation  # structural + validation coverage
cargo fmt --check -p limen-core
cargo clippy -p limen-core --all-targets -- -D warnings
```
