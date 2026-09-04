# Handoff

This document is written for an incoming cross-review by a different reviewer/model. It
summarizes what was built, what was verified (with real numbers pulled from this repo, not
approximations), where the integrity requirements are enforced, what deserves close scrutiny, and
what was deliberately left out of scope.

## What was built, and why

Limen is a deterministic, browser-based lab for comparing five mechanical, versioned
context-selection strategies against three authored scenarios under fixed token budgets, entirely
offline. The project exists to make the context-construction decision -- "given this task and this
budget, what do you keep and what do you drop?" -- visible and reproducible in isolation, separate
from any claim about downstream model quality. The domain logic (tokenizer, strategies, manifest
validation, metrics, canonical digesting) is implemented once, in a pure Rust crate
(`crates/limen-core`), compiled to WASM through a deliberately thin adapter
(`crates/limen-wasm`), and driven by a static Vite/TypeScript UI (`web/`) that never duplicates
any of that logic. Every result is a byte-exact, re-hashable JSON artifact.

## Repo layout

| Path | Purpose |
|---|---|
| `crates/limen-core/` | Pure Rust engine: manifest model, tokenizer, validator, five strategies, metrics, canonical digesting. |
| `crates/limen-wasm/` | Thin `wasm-bindgen` adapter exposing `limen-core` to the browser; zero duplicated logic. |
| `scenarios/v1/` | The three checked-in, versioned scenario manifests, embedded into the compiled binary. |
| `schemas/` | Generated JSON Schemas for the scenario-manifest and trial-result shapes. |
| `web/` | Static Vite + TypeScript UI, Vitest unit/DOM tests, Playwright e2e suite. |
| `docs/` | `STRATEGIES.md`, `SCENARIO_AUTHORING.md`, `METRICS.md`, `docs/adr/`. |
| `docs/adr/` | Four architecture decision records covering the WASM boundary, tokenizer, digesting, and strategy/annotation isolation. |
| `scripts/` | The no-external-URL check run against the built `web/dist`. |
| `.github/workflows/ci.yml` | Pinned CI covering the full verification matrix below. |
| `PRODUCT.md` / `DESIGN.md` | The supplied product brief and visual system. |

## Verification results (re-run at handoff time, not approximated)

All commands below were re-run in this worktree immediately before writing this document.

- **`cargo test --workspace`: full native suite, all passing.** Coverage spans unit tests in
  `limen-core` and `limen-wasm`, plus integration tests across five `limen-core` files -- property
  tests (`property_tests.rs`, covering budget safety, byte-identical repeatability of `select`,
  total-order sortedness of selections, `resolve_strategy` never-panics/rejects-unknown, and
  malformed-manifest mutations always caught), golden-fixture regression tests, golden-vs-schema
  validation, scenario structural-coverage tests, and schema-freshness tests. (`crates/limen-wasm/
  tests/wasm_parity.rs` is `#[cfg(target_arch = "wasm32")]`-gated and correctly contributes 0
  tests to this native run.)
- **`cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings`: both clean.**
- **75-fixture golden matrix, native/WASM digest parity: proven, not just claimed.**
  `crates/limen-core/tests/golden/` holds 75 checked-in canonical `TrialResult` JSON files (3
  scenarios x 5 strategies x 5 budget tiers: zero, tight, exact-boundary, representative, ample).
  `wasm-pack test --node crates/limen-wasm` runs the *real compiled* WASM artifact against all 75
  and asserts byte-identical canonical JSON and identical `result_digest`s versus the native
  fixtures -- 3 passing `wasm_bindgen_test` functions, confirmed at handoff time. All 75 fixtures
  were regenerated after the `Split`/`PartialWithinRetained` rule correction below; 20 of the 75
  now genuinely show `"outcome":"split"` (zero did before the fix), across all 5 strategies --
  including `retrieval-ranking@1`, whose non-contiguous bin-packing the old geometric rule handled
  worst. All 75 still pass schema validation against `schemas/trial-result-v1.schema.json` and
  native/WASM parity.
- **Contradiction `Split` vs `PartialWithinRetained` rule corrected.** The rule used to compare a
  group's members against the whole-selection's single highest-order-index dropped item and
  single lowest-order-index retained item -- geometrically meaningless for
  `retrieval-ranking@1`'s non-contiguous bin-packing, and too narrow even for the four
  linear-cutoff strategies (a group had to contain *both* global extremes, not just straddle the
  cutoff generally). It now asks *why* each dropped member was dropped, via each strategy's own
  `TraceStep.action` vocabulary: `Split` iff at least one dropped member's action is a
  budget-cutoff reason (`is_budget_drop_action` in `crates/limen-core/src/metrics.rs`:
  `"dropped_over_budget"`/`"dropped_too_old"`/`"dropped_below_budget"`); `PartialWithinRetained`
  iff every dropped member's action is a non-budget reason (currently only structured-extraction's
  `"dropped_no_extractable_content"`). See `docs/METRICS.md` and `metrics.rs`'s module doc comment
  for the full rule, and `metrics.rs`'s test module for two hand-built tests plus two tests that
  exercise the real `recency@1` and `retrieval-ranking@1` strategy implementations end to end.
- **Vitest: full unit/DOM suite, all passing** (`web/`: `npm test`).
- **Playwright e2e: full suite, all passing** (`web/`: `npm run test:e2e`), covering real WASM
  loading, keyboard budget/strategy controls, screen-reader-relevant structure, color-independent
  status indicators, mobile stacking, reduced motion, byte-exact downloads, and same-origin-only
  network access during a full interaction session.
- **Static build + offline proof:** `npm run build` produces `web/dist` (index.html, one CSS
  bundle, one JS bundle, the `.wasm` binary); `node scripts/check-no-external-urls.mjs web/dist`
  confirms zero external URL references in the built output, at handoff time.
- **CI** (`.github/workflows/ci.yml`) runs this same matrix in one linear job, with an explicit,
  commented boundary between install/registry-access steps and a fully offline
  (`CARGO_NET_OFFLINE=true`) verification section.

## Integrity requirements and how each is satisfied

- **Same manifest + strategy + budget yields the same result digest.** Enforced by canonical-JSON
  encoding (recursively sorted keys, compact form) plus SHA-256 digesting with the digest field
  itself entirely absent from the tree while hashing (`crates/limen-core/src/canonical.rs`).
  Proven by the `determinism_select_twice_is_byte_identical` property test and by all 75 golden
  fixtures regenerating byte-identical on every `cargo test` run. See
  `docs/adr/0003-deterministic-hashable-result-artifacts.md`.
- **Budget is never exceeded.** `BudgetUsage::new` clamps `used_tokens` to `requested_tokens`
  rather than ever allowing an overrun or an underflowed `remaining_tokens`
  (`crates/limen-core/src/model.rs`); the `budget_safety_never_exceeds_requested` property test
  and the golden-fixture-derived `every_golden_fixture_used_tokens_never_exceeds_requested_tokens`
  test both check this directly, including at the 0-token and 1-token extremes.
- **Required-fact denominators are always visible.** `FactRecall.required` and
  `CitationRetention.expected` are always reported alongside their numerators; no metric is ever
  presented as a bare percentage. See `docs/METRICS.md`.
- **Tokenizer identity travels with every result.** `TrialResult.tokenizer_id` is always
  `limen-lex@1.0.0` (`crates/limen-core/src/tokenizer.rs`'s `TOKENIZER_ID`), independent of
  `engine_version`. See `docs/adr/0002-tokenizer-boundary-and-versioning.md`.
- **Contradictions are never silently collapsed.** Every `ContradictionGroup` resolves to one of
  four explicit, structural outcomes (`AllRetained`/`Split`/`PartialWithinRetained`/
  `NoneRetained`) -- there is no "just show one merged answer" path. See `docs/METRICS.md`'s
  precise `Split` boundary rule.
- **Invalid manifests and unknown strategy versions fail clearly.** `validate_manifest` collects
  every problem in one pass (never stops at the first), each with a stable `code`, a message, and
  a best-effort `path`; `resolve_strategy` returns a distinct, stable
  `EngineError::UnknownStrategyVersion` for any id that doesn't exactly match the fixed registry,
  including a known family with an unsupported version suffix (e.g. `"recency@2"`). Both are
  covered by dedicated unit and property tests. `validate_manifest` itself is a build/test-time
  guarantee (enforced on the three embedded scenarios by a dedicated `catalog.rs` unit test and by
  `crates/limen-core/tests/scenario_validation.rs`), not a runtime check: `run_trial` never calls
  it against the embedded scenarios at request time. This is an
  intentional design point, not an oversight -- the three shipped manifests are fixed, checked-in,
  compile-time-embedded content with no ingestion path for arbitrary/untrusted manifests, so there
  is nothing at runtime for `validate_manifest` to usefully guard against.
- **The UI never implies "model intelligence."** `hierarchical-summary@1`'s label is hard-pinned,
  verbatim, everywhere it is displayed: **"Hierarchical summary (deterministic/template-based)"**
  (enforced by `descriptors_match_the_exact_specified_copy_verbatim` and
  `hierarchical_summary_label_is_the_exact_required_non_claims_string` in
  `crates/limen-core/src/strategy/mod.rs`); the running app's own header includes an explicit
  non-claims banner (`web/src/main.ts`).

## What a reviewer should specifically scrutinize

- The exact contradiction `Split` vs `PartialWithinRetained` rule (whether at least one of a mixed
  group's dropped members was dropped for a budget-cutoff reason vs. every dropped member being a
  content-level drop, per each strategy's own `TraceStep.action` vocabulary -- see
  `is_budget_drop_action` in `crates/limen-core/src/metrics.rs`) -- it is subtle and worth
  re-deriving independently against the module's own test cases.
- `retrieval-ranking@1`'s greedy bin-packing behavior (`crates/limen-core/src/strategy/
  retrieval_ranking.rs`) -- it is the one strategy that does not do a hard linear cutoff, and the
  one place a smaller/lower-ranked item can be included after a larger/higher-ranked one was
  skipped.
- `structured-extraction@1`'s and `hierarchical-summary@1`'s provenance-pointer semantics -- both
  are *transform* strategies whose `output_text` differs from the verbatim source slice while
  `included_byte_start`/`included_byte_end` still point at real source spans; confirm metrics
  correctly treat `output_text` (not the raw slice) as "the output" wherever it is `Some`.
- The byte-exact-download guarantee: `web/src/wasm/engine.ts`'s `runTrial` keeps the engine's raw
  JSON string (`value.raw`) separate from a parsed copy (`value.parsed`) used only for rendering;
  confirm nothing in `web/src` ever re-serializes `parsed` and writes *that* to a download instead
  of `raw`.
- The offline/no-network proof: `scripts/check-no-external-urls.mjs` scans the built `web/dist`
  for external URL references, and `web/e2e/download-and-network.spec.ts` asserts every network
  request during a full interaction session stays same-origin against a real served production
  build (not the dev server).

## What was deliberately deferred (out of scope, not a gap)

Per the original plan, the following are intentionally out of scope for this foundation and
should not be flagged as missing functionality:

- Arbitrary/uploaded document ingestion -- only the three checked-in, authored scenario manifests
  are supported; there is no upload UI or document-parsing path.
- Live LLM/model calls of any kind -- Limen is explicitly not a benchmark of real model output;
  every "strategy" is a deterministic, mechanical selection algorithm, never a call to any model.
- Any server-side component -- the shipped product is a fully static, same-origin bundle with no
  backend, no API, and no runtime network request.
- A sixth+ strategy or a fourth+ scenario -- the matrix is fixed at 5 strategies x 3 scenarios x 5
  budget tiers for this foundation; the registry/catalog patterns (`strategy/mod.rs`,
  `catalog.rs`) are structured to make adding more straightforward later, but none are included
  now.
- A `limen-lex@2.0.0` or any alternative tokenizer -- only `limen-lex@1.0.0` exists; see
  `docs/adr/0002-tokenizer-boundary-and-versioning.md` for how a future version would be
  introduced without invalidating existing artifacts.
