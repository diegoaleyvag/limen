# Limen

Limen is a deterministic, browser-based lab for comparing context-selection strategies under
fixed token budgets.

Limen does not claim to benchmark real model quality. It isolates the context-construction
decision so its tradeoffs are visible and reproducible. Metrics evaluate selected context against
annotated expected facts; they do not claim downstream LLM answer accuracy. Every result is a
byte-exact, re-hashable artifact: the same scenario, strategy, and budget always produce the same
JSON, and that JSON, not a paraphrase of it, is what a user downloads.

Five mechanical, versioned strategies run against three authored scenarios entirely offline, in
the browser, via a compiled Rust/WASM engine: there is no server, no live model call, and no
network request of any kind at runtime.

## Architecture at a glance

```
ScenarioManifest --> Rust core (limen-core) --> thin WASM adapter (limen-wasm) --> static TS UI (web/)
```

- **Rust is the single source of truth.** The tokenizer, the five selection strategies, manifest
  validation, metrics, and canonical-JSON/SHA-256 digesting all live in `crates/limen-core`, a
  pure Rust crate with no WASM- or browser-specific code.
- **`crates/limen-wasm` is a deliberately thin adapter.** It only calls into `limen-core`, converts
  the result to a JSON string, and crosses the `wasm-bindgen` boundary. It duplicates no
  tokenizer, selector, evaluator, validator, or digester logic.
- **The browser has no parallel implementation.** `web/src` (a static Vite + TypeScript UI) calls
  the compiled WASM module for every decision and only parses a *copy* of its JSON for rendering;
  the exact string the engine returns is what byte-exact downloads write to disk. See
  [`docs/adr/0001-rust-core-compiled-to-wasm-thin-adapter.md`](docs/adr/0001-rust-core-compiled-to-wasm-thin-adapter.md) for
  why this is what makes "native and browser results are the same implementation" achievable and
  testable.

## Repo layout

| Path | Purpose |
|---|---|
| `crates/limen-core/` | Pure Rust engine: manifest model, tokenizer, validator, five strategies, metrics, canonical digesting. No WASM/browser dependency. |
| `crates/limen-wasm/` | Thin `wasm-bindgen` adapter exposing `limen-core` to the browser. |
| `scenarios/v1/` | The three checked-in, versioned scenario manifests (JSON), embedded into the compiled binary at build time. |
| `schemas/` | Generated JSON Schemas for the scenario-manifest and trial-result shapes. |
| `web/` | Static Vite + TypeScript UI: scenario/budget controls, two independent strategy columns, Vitest unit/DOM tests, Playwright e2e suite. |
| `docs/` | `STRATEGIES.md`, `SCENARIO_AUTHORING.md`, `METRICS.md`, and `docs/adr/` (architecture decision records). |
| `docs/adr/` | Architecture decision records. |
| `scripts/` | Small standalone verification scripts (e.g. the no-external-URL check run against the built `web/dist`). |
| `.github/workflows/ci.yml` | Pinned CI: Rust fmt/clippy/tests, WASM parity tests, web lint/typecheck/unit/e2e, static build, and an offline/no-external-URL assertion. |

Top-level product/design docs (`PRODUCT.md`, `DESIGN.md`) record the supplied product brief and
visual system; they are not covered further here.

## Local commands

Every command below was run against this exact worktree and confirmed to work before being
documented.

### Prerequisites

- Rust `1.97.1` (pinned in `rust-toolchain.toml`) with the `rustfmt`, `clippy` components and the
  `wasm32-unknown-unknown` target. If `cargo`/`rustc` are not already on `PATH`, source the cargo
  env first:

  ```bash
  source "$HOME/.cargo/env"
  ```

- `wasm-pack 0.15.0` (`cargo install wasm-pack --version 0.15.0 --locked`).
- Node.js `^20.19.0 || >=22.12.0` and npm (see `web/package.json`'s `engines` field).

### Rust workspace (`crates/limen-core`, `crates/limen-wasm`)

Run from the repo root:

```bash
cargo build                                                  # build both crates
cargo test --workspace                                       # unit + property + golden + schema tests
cargo fmt --check                                             # formatting check
cargo clippy --workspace --all-targets -- -D warnings         # lint, warnings as errors
```

`cargo test --workspace` runs the full native workspace suite: unit tests across every module,
property-based tests (`crates/limen-core/tests/property_tests.rs`), the 75-fixture golden-matrix
regression test (`tests/golden_fixtures.rs`), schema-freshness and golden-fixture-vs-schema
validation, and scenario structural-coverage tests. It does **not** run the WASM parity suite
(see below); that requires the `wasm32-unknown-unknown` target and `wasm-pack test`, and is
`#[cfg(target_arch = "wasm32")]`-gated so a native `cargo test` run correctly skips it.

### Building the WASM package

```bash
wasm-pack build crates/limen-wasm --target web --out-dir web/src/wasm/pkg --no-opt
```

(`web/package.json`'s `build:wasm` script runs the equivalent command relative to `web/`, and is
automatically invoked by `predev`/`prebuild`/`pretypecheck`/`pretest` hooks below.)

To run the native/WASM parity suite (75 golden fixtures, checked byte-for-byte and by
`result_digest`, against the real compiled WASM artifact):

```bash
wasm-pack test --node crates/limen-wasm
```

### Web app (`web/`)

Install dependencies once:

```bash
cd web
npm ci
```

Then, from `web/`:

```bash
npm run dev              # Vite dev server (rebuilds the WASM package first via predev)
npm test                 # Vitest unit/DOM suite
npm run test:e2e:install # one-time Playwright Chromium browser install
npm run test:e2e         # Playwright e2e suite (WASM loading, a11y, responsive, downloads, offline)
npm run typecheck        # TypeScript typecheck of web/src
npm run typecheck:e2e    # TypeScript typecheck of web/e2e
npm run lint             # Biome check
npm run build            # static production bundle -> web/dist
npm run preview          # serve the built web/dist locally
```

`npm run build` produces a same-origin static bundle (HTML, CSS, JS, and the `.wasm` binary) with
no external URLs, no API clients, no remote fonts, and no analytics. You can verify that claim
against any built `dist` directory with:

```bash
node scripts/check-no-external-urls.mjs web/dist
```

### Regenerating scenarios, schemas, and golden fixtures

These are checked-in, generated artifacts; regenerate them (from the repo root) only after
changing the Rust source that produces them, then review the diff before committing:

```bash
cargo run -p limen-core --example author_scenarios          # regenerate scenarios/v1/*.json
cargo run -p limen-core --example generate_schemas          # regenerate schemas/*.schema.json
cargo run -p limen-core --example generate_golden_fixtures  # regenerate crates/limen-core/tests/golden/*.json
```

## Verification summary

See [`HANDOFF.md`](HANDOFF.md) for the full verification matrix (golden-fixture parity, e2e
coverage) and the integrity requirements a reviewer should check.
