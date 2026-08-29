# 0003. Deterministic, hashable result artifacts

## Context

Limen's core integrity promise is that the same scenario, strategy, and budget always produce the
same result -- byte-for-byte, not just "equivalent JSON" -- so a user can download a `TrialResult`,
recompute its digest independently, and trust that it represents a specific, reproducible
computation rather than something that happened to come out a certain way on one run. This
requires a single, unambiguous canonical encoding (so two structurally-identical values always
serialize to identical bytes, regardless of field-construction order) and a set of guardrails
throughout the codebase that prevent non-determinism from ever entering a decision or artifact-
construction path.

Both `ScenarioManifest` (via `content_digest`) and `TrialResult` (via `result_digest`) are
digests *of the structure they are themselves a field of*, which raises a specific problem: how do
you hash a value that contains its own hash?

## Decision

**Canonical JSON.** `crates/limen-core/src/canonical.rs` is the single implementation of
canonicalization used everywhere a digest is needed (scenario digests, result digests, schema-
freshness checks). Canonicalization recursively sorts every JSON object's keys by walking the
tree and rebuilding each object through a `BTreeMap<String, Value>` explicitly, then encodes with
`serde_json::to_vec` (compact, no pretty-printing whitespace). The explicit `BTreeMap` rebuild
(rather than relying on `serde_json::Map`'s default ordering behavior) guards against a
project-wide dependency-feature change (`serde_json`'s `preserve_order` feature) ever silently
switching the map's backing store to something insertion-ordered, which would break byte-for-byte
digest stability without a compile error.

**SHA-256 digesting.** Canonical bytes are hashed with SHA-256 (the `sha2` crate) and formatted as
`"sha256:<64 lowercase hex chars>"` (`digest_bytes`).

**Field-blanking for self-referential digests.** To digest a value that contains its own digest
field, that field is made **entirely absent** from the JSON tree while hashing -- not present-but-
blank, since even an empty string is a value that would affect the hash -- then the computed digest
is inserted back afterward. `digest_value_with_field_blanked`/`digest_with_field_blanked` are the
only sanctioned way to compute `content_digest`/`result_digest`; every phase of this project uses
one of these two functions so the convention never drifts.

**Crate-wide determinism guardrails**, enforced throughout every type and code path that can reach
a decision or artifact:

- No `HashMap`/`HashSet` in any serialized type or in code that affects an artifact's content (the
  one true set, `distractor_source_ids`, is a `BTreeSet`; strategy/scenario lookup is always an
  exact-match `match` over a fixed static list, never a hash-keyed structure).
- No `f32`/`f64` anywhere in a serialized type -- numbers and dates are canonical *text* (see
  `CanonicalValue` in `model.rs`), compared by exact substring, never by floating-point equality.
- No platform-sized integers (`usize`/`isize`) in any serialized type -- byte offsets, token
  counts, and order indices are all `u32`.
- No panics escaping any public function on a decision or artifact-construction path (`Result`-
  based error handling throughout; `BudgetUsage::new` clamps rather than panics; see
  `error.rs`/`result.rs`).
- No wall-clock or RNG data anywhere in a decision or artifact path -- every strategy is a pure
  function of its `StrategyInput` and `Budget`.

## Consequences

- **The integrity requirement is provable, not just asserted.** "The same manifest+strategy+budget
  yields the same result digest" is directly tested: `crates/limen-core/tests/property_tests.rs`'s
  `determinism_select_twice_is_byte_identical` property test calls `select` twice on identical
  input and asserts byte-identical canonical JSON across randomized inputs, and
  `tests/golden_fixtures.rs` regenerates and re-digests all 75 checked-in golden fixtures on
  every `cargo test` run, failing loudly (with a clear per-fixture message) if regeneration ever
  disagrees with what is checked in.
- **Digests are stable across independent recomputation**, not just self-consistent: both
  `result.rs`'s own unit tests and `limen-wasm`'s `run_trial_impl_result_digest_matches_
  independent_recomputation` test parse a produced `TrialResult` back out of its JSON and
  recompute its digest from scratch, confirming it matches the embedded one.
- **No silent Unicode normalization.** Scenario text is hashed, tokenized, and byte-sliced exactly
  as authored (see `canonical.rs`'s module doc comment); normalizing before hashing would let
  visibly-different strings collide and would make scenario-author-recorded byte offsets
  potentially mismatch what actually gets hashed. If a future phase needs normalization, it must
  be an explicit, documented step applied *before* text reaches this module.
- **A future field addition to `TrialResult`/`ScenarioManifest` automatically changes every
  digest**, by design -- there is no allowlist of "digest-relevant" fields to remember to update;
  the whole (field-blanked) structure is always hashed.

## Alternatives considered

- **Blank the digest field to an empty string/placeholder rather than removing it.** Rejected:
  an empty string is still a JSON value and would still affect the hash, making the digest
  dependent on what placeholder was chosen -- a real correctness bug this design deliberately
  avoids by removing the key from the object entirely before hashing.
- **Rely on `serde_json::Map`'s default (BTreeMap-backed) ordering without an explicit sort step.**
  Rejected: correct today, but silently fragile against a transitive dependency anywhere in the
  build enabling `serde_json`'s `preserve_order` feature (Cargo unifies features workspace-wide),
  which would flip the map to insertion order with no compile error -- exactly the kind of
  invisible determinism regression this project's guardrails exist to prevent.
- **A general-purpose canonicalization library (e.g. JCS/RFC 8785).** Considered, but a small,
  fully-owned implementation was preferred for this foundation: it keeps the exact
  encoding/hashing behavior auditable in one small file with full control over compact-vs-pretty
  formatting and field-blanking semantics, rather than depending on an external spec's edge-case
  behavior (e.g. around number formatting) that this project does not need, since no floats are
  ever modeled.
