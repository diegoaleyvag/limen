//! Generated data module embedding the 75 checked-in native golden fixtures
//! (`crates/limen-core/tests/golden/*.json`) at compile time via `include_str!`, so
//! `tests/wasm_parity.rs` can compare them against the compiled WASM module's own output without
//! any filesystem access at test run time (unavailable inside a `wasm32-unknown-unknown`
//! `wasm-pack test` harness) -- the same `include_str!`-at-compile-time technique
//! `limen_core::catalog` uses to embed scenario JSON into both native and WASM binaries.
//!
//! GENERATED FILE -- do not hand-edit. Regenerate with:
//! `cargo run -p limen-core --example generate_golden_fixtures` (to refresh the golden fixtures
//! themselves) followed by the small Python snippet in this crate's verification notes / the
//! agent's final report, which reads `crates/limen-core/tests/golden/index.json` and rewrites
//! this file's `GOLDEN_CASES` array to match, one `include_str!` per fixture.
//!
//! Each tuple is `(scenario_id, strategy_id, tier_name, requested_tokens, canonical_json)`, where
//! `canonical_json` is the exact checked-in file content (including its trailing newline; callers
//! trim that off before comparing).

#[allow(dead_code)]
pub const GOLDEN_CASES: &[(&str, &str, &str, u32, &str)] = &[
    ("incident-investigation", "full-input-truncation@1", "zero", 0u32, include_str!("../../../limen-core/tests/golden/incident-investigation__full-input-truncation@1__zero.json")),
    ("incident-investigation", "recency@1", "zero", 0u32, include_str!("../../../limen-core/tests/golden/incident-investigation__recency@1__zero.json")),
    ("incident-investigation", "structured-extraction@1", "zero", 0u32, include_str!("../../../limen-core/tests/golden/incident-investigation__structured-extraction@1__zero.json")),
    ("incident-investigation", "hierarchical-summary@1", "zero", 0u32, include_str!("../../../limen-core/tests/golden/incident-investigation__hierarchical-summary@1__zero.json")),
    ("incident-investigation", "retrieval-ranking@1", "zero", 0u32, include_str!("../../../limen-core/tests/golden/incident-investigation__retrieval-ranking@1__zero.json")),
    ("incident-investigation", "full-input-truncation@1", "tight", 149u32, include_str!("../../../limen-core/tests/golden/incident-investigation__full-input-truncation@1__tight.json")),
    ("incident-investigation", "recency@1", "tight", 149u32, include_str!("../../../limen-core/tests/golden/incident-investigation__recency@1__tight.json")),
    ("incident-investigation", "structured-extraction@1", "tight", 149u32, include_str!("../../../limen-core/tests/golden/incident-investigation__structured-extraction@1__tight.json")),
    ("incident-investigation", "hierarchical-summary@1", "tight", 149u32, include_str!("../../../limen-core/tests/golden/incident-investigation__hierarchical-summary@1__tight.json")),
    ("incident-investigation", "retrieval-ranking@1", "tight", 149u32, include_str!("../../../limen-core/tests/golden/incident-investigation__retrieval-ranking@1__tight.json")),
    ("incident-investigation", "full-input-truncation@1", "exact_boundary", 263u32, include_str!("../../../limen-core/tests/golden/incident-investigation__full-input-truncation@1__exact_boundary.json")),
    ("incident-investigation", "recency@1", "exact_boundary", 263u32, include_str!("../../../limen-core/tests/golden/incident-investigation__recency@1__exact_boundary.json")),
    ("incident-investigation", "structured-extraction@1", "exact_boundary", 263u32, include_str!("../../../limen-core/tests/golden/incident-investigation__structured-extraction@1__exact_boundary.json")),
    ("incident-investigation", "hierarchical-summary@1", "exact_boundary", 263u32, include_str!("../../../limen-core/tests/golden/incident-investigation__hierarchical-summary@1__exact_boundary.json")),
    ("incident-investigation", "retrieval-ranking@1", "exact_boundary", 263u32, include_str!("../../../limen-core/tests/golden/incident-investigation__retrieval-ranking@1__exact_boundary.json")),
    ("incident-investigation", "full-input-truncation@1", "representative", 298u32, include_str!("../../../limen-core/tests/golden/incident-investigation__full-input-truncation@1__representative.json")),
    ("incident-investigation", "recency@1", "representative", 298u32, include_str!("../../../limen-core/tests/golden/incident-investigation__recency@1__representative.json")),
    ("incident-investigation", "structured-extraction@1", "representative", 298u32, include_str!("../../../limen-core/tests/golden/incident-investigation__structured-extraction@1__representative.json")),
    ("incident-investigation", "hierarchical-summary@1", "representative", 298u32, include_str!("../../../limen-core/tests/golden/incident-investigation__hierarchical-summary@1__representative.json")),
    ("incident-investigation", "retrieval-ranking@1", "representative", 298u32, include_str!("../../../limen-core/tests/golden/incident-investigation__retrieval-ranking@1__representative.json")),
    ("incident-investigation", "full-input-truncation@1", "ample", 1596u32, include_str!("../../../limen-core/tests/golden/incident-investigation__full-input-truncation@1__ample.json")),
    ("incident-investigation", "recency@1", "ample", 1596u32, include_str!("../../../limen-core/tests/golden/incident-investigation__recency@1__ample.json")),
    ("incident-investigation", "structured-extraction@1", "ample", 1596u32, include_str!("../../../limen-core/tests/golden/incident-investigation__structured-extraction@1__ample.json")),
    ("incident-investigation", "hierarchical-summary@1", "ample", 1596u32, include_str!("../../../limen-core/tests/golden/incident-investigation__hierarchical-summary@1__ample.json")),
    ("incident-investigation", "retrieval-ranking@1", "ample", 1596u32, include_str!("../../../limen-core/tests/golden/incident-investigation__retrieval-ranking@1__ample.json")),
    ("product-comparison", "full-input-truncation@1", "zero", 0u32, include_str!("../../../limen-core/tests/golden/product-comparison__full-input-truncation@1__zero.json")),
    ("product-comparison", "recency@1", "zero", 0u32, include_str!("../../../limen-core/tests/golden/product-comparison__recency@1__zero.json")),
    ("product-comparison", "structured-extraction@1", "zero", 0u32, include_str!("../../../limen-core/tests/golden/product-comparison__structured-extraction@1__zero.json")),
    ("product-comparison", "hierarchical-summary@1", "zero", 0u32, include_str!("../../../limen-core/tests/golden/product-comparison__hierarchical-summary@1__zero.json")),
    ("product-comparison", "retrieval-ranking@1", "zero", 0u32, include_str!("../../../limen-core/tests/golden/product-comparison__retrieval-ranking@1__zero.json")),
    ("product-comparison", "full-input-truncation@1", "tight", 123u32, include_str!("../../../limen-core/tests/golden/product-comparison__full-input-truncation@1__tight.json")),
    ("product-comparison", "recency@1", "tight", 123u32, include_str!("../../../limen-core/tests/golden/product-comparison__recency@1__tight.json")),
    ("product-comparison", "structured-extraction@1", "tight", 123u32, include_str!("../../../limen-core/tests/golden/product-comparison__structured-extraction@1__tight.json")),
    ("product-comparison", "hierarchical-summary@1", "tight", 123u32, include_str!("../../../limen-core/tests/golden/product-comparison__hierarchical-summary@1__tight.json")),
    ("product-comparison", "retrieval-ranking@1", "tight", 123u32, include_str!("../../../limen-core/tests/golden/product-comparison__retrieval-ranking@1__tight.json")),
    ("product-comparison", "full-input-truncation@1", "exact_boundary", 235u32, include_str!("../../../limen-core/tests/golden/product-comparison__full-input-truncation@1__exact_boundary.json")),
    ("product-comparison", "recency@1", "exact_boundary", 235u32, include_str!("../../../limen-core/tests/golden/product-comparison__recency@1__exact_boundary.json")),
    ("product-comparison", "structured-extraction@1", "exact_boundary", 235u32, include_str!("../../../limen-core/tests/golden/product-comparison__structured-extraction@1__exact_boundary.json")),
    ("product-comparison", "hierarchical-summary@1", "exact_boundary", 235u32, include_str!("../../../limen-core/tests/golden/product-comparison__hierarchical-summary@1__exact_boundary.json")),
    ("product-comparison", "retrieval-ranking@1", "exact_boundary", 235u32, include_str!("../../../limen-core/tests/golden/product-comparison__retrieval-ranking@1__exact_boundary.json")),
    ("product-comparison", "full-input-truncation@1", "representative", 246u32, include_str!("../../../limen-core/tests/golden/product-comparison__full-input-truncation@1__representative.json")),
    ("product-comparison", "recency@1", "representative", 246u32, include_str!("../../../limen-core/tests/golden/product-comparison__recency@1__representative.json")),
    ("product-comparison", "structured-extraction@1", "representative", 246u32, include_str!("../../../limen-core/tests/golden/product-comparison__structured-extraction@1__representative.json")),
    ("product-comparison", "hierarchical-summary@1", "representative", 246u32, include_str!("../../../limen-core/tests/golden/product-comparison__hierarchical-summary@1__representative.json")),
    ("product-comparison", "retrieval-ranking@1", "representative", 246u32, include_str!("../../../limen-core/tests/golden/product-comparison__retrieval-ranking@1__representative.json")),
    ("product-comparison", "full-input-truncation@1", "ample", 1493u32, include_str!("../../../limen-core/tests/golden/product-comparison__full-input-truncation@1__ample.json")),
    ("product-comparison", "recency@1", "ample", 1493u32, include_str!("../../../limen-core/tests/golden/product-comparison__recency@1__ample.json")),
    ("product-comparison", "structured-extraction@1", "ample", 1493u32, include_str!("../../../limen-core/tests/golden/product-comparison__structured-extraction@1__ample.json")),
    ("product-comparison", "hierarchical-summary@1", "ample", 1493u32, include_str!("../../../limen-core/tests/golden/product-comparison__hierarchical-summary@1__ample.json")),
    ("product-comparison", "retrieval-ranking@1", "ample", 1493u32, include_str!("../../../limen-core/tests/golden/product-comparison__retrieval-ranking@1__ample.json")),
    ("requirements-architecture-review", "full-input-truncation@1", "zero", 0u32, include_str!("../../../limen-core/tests/golden/requirements-architecture-review__full-input-truncation@1__zero.json")),
    ("requirements-architecture-review", "recency@1", "zero", 0u32, include_str!("../../../limen-core/tests/golden/requirements-architecture-review__recency@1__zero.json")),
    ("requirements-architecture-review", "structured-extraction@1", "zero", 0u32, include_str!("../../../limen-core/tests/golden/requirements-architecture-review__structured-extraction@1__zero.json")),
    ("requirements-architecture-review", "hierarchical-summary@1", "zero", 0u32, include_str!("../../../limen-core/tests/golden/requirements-architecture-review__hierarchical-summary@1__zero.json")),
    ("requirements-architecture-review", "retrieval-ranking@1", "zero", 0u32, include_str!("../../../limen-core/tests/golden/requirements-architecture-review__retrieval-ranking@1__zero.json")),
    ("requirements-architecture-review", "full-input-truncation@1", "tight", 163u32, include_str!("../../../limen-core/tests/golden/requirements-architecture-review__full-input-truncation@1__tight.json")),
    ("requirements-architecture-review", "recency@1", "tight", 163u32, include_str!("../../../limen-core/tests/golden/requirements-architecture-review__recency@1__tight.json")),
    ("requirements-architecture-review", "structured-extraction@1", "tight", 163u32, include_str!("../../../limen-core/tests/golden/requirements-architecture-review__structured-extraction@1__tight.json")),
    ("requirements-architecture-review", "hierarchical-summary@1", "tight", 163u32, include_str!("../../../limen-core/tests/golden/requirements-architecture-review__hierarchical-summary@1__tight.json")),
    ("requirements-architecture-review", "retrieval-ranking@1", "tight", 163u32, include_str!("../../../limen-core/tests/golden/requirements-architecture-review__retrieval-ranking@1__tight.json")),
    ("requirements-architecture-review", "full-input-truncation@1", "exact_boundary", 308u32, include_str!("../../../limen-core/tests/golden/requirements-architecture-review__full-input-truncation@1__exact_boundary.json")),
    ("requirements-architecture-review", "recency@1", "exact_boundary", 308u32, include_str!("../../../limen-core/tests/golden/requirements-architecture-review__recency@1__exact_boundary.json")),
    ("requirements-architecture-review", "structured-extraction@1", "exact_boundary", 308u32, include_str!("../../../limen-core/tests/golden/requirements-architecture-review__structured-extraction@1__exact_boundary.json")),
    ("requirements-architecture-review", "hierarchical-summary@1", "exact_boundary", 308u32, include_str!("../../../limen-core/tests/golden/requirements-architecture-review__hierarchical-summary@1__exact_boundary.json")),
    ("requirements-architecture-review", "retrieval-ranking@1", "exact_boundary", 308u32, include_str!("../../../limen-core/tests/golden/requirements-architecture-review__retrieval-ranking@1__exact_boundary.json")),
    ("requirements-architecture-review", "full-input-truncation@1", "representative", 326u32, include_str!("../../../limen-core/tests/golden/requirements-architecture-review__full-input-truncation@1__representative.json")),
    ("requirements-architecture-review", "recency@1", "representative", 326u32, include_str!("../../../limen-core/tests/golden/requirements-architecture-review__recency@1__representative.json")),
    ("requirements-architecture-review", "structured-extraction@1", "representative", 326u32, include_str!("../../../limen-core/tests/golden/requirements-architecture-review__structured-extraction@1__representative.json")),
    ("requirements-architecture-review", "hierarchical-summary@1", "representative", 326u32, include_str!("../../../limen-core/tests/golden/requirements-architecture-review__hierarchical-summary@1__representative.json")),
    ("requirements-architecture-review", "retrieval-ranking@1", "representative", 326u32, include_str!("../../../limen-core/tests/golden/requirements-architecture-review__retrieval-ranking@1__representative.json")),
    ("requirements-architecture-review", "full-input-truncation@1", "ample", 1653u32, include_str!("../../../limen-core/tests/golden/requirements-architecture-review__full-input-truncation@1__ample.json")),
    ("requirements-architecture-review", "recency@1", "ample", 1653u32, include_str!("../../../limen-core/tests/golden/requirements-architecture-review__recency@1__ample.json")),
    ("requirements-architecture-review", "structured-extraction@1", "ample", 1653u32, include_str!("../../../limen-core/tests/golden/requirements-architecture-review__structured-extraction@1__ample.json")),
    ("requirements-architecture-review", "hierarchical-summary@1", "ample", 1653u32, include_str!("../../../limen-core/tests/golden/requirements-architecture-review__hierarchical-summary@1__ample.json")),
    ("requirements-architecture-review", "retrieval-ranking@1", "ample", 1653u32, include_str!("../../../limen-core/tests/golden/requirements-architecture-review__retrieval-ranking@1__ample.json")),
];
