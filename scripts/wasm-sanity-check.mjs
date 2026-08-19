#!/usr/bin/env node
// Lightweight end-to-end sanity check for the `limen-wasm` `--target web` build, run directly
// against the compiled `.wasm` bytes (no dev server, no bundler, no browser). Exercises every
// exported function's success path and, separately, its error path (unknown scenario id, unknown
// strategy id/version), asserting every error crosses the boundary as a structured, parseable
// JSON string rather than an unstructured/opaque exception.
//
// This intentionally instantiates the module the same way `wasm-pack build --target web` was
// proven to work in this repo: reading the compiled `.wasm` file's bytes directly and calling
// `initSync`, rather than the default `fetch()`-based loader the `--target web` glue normally
// uses (`fetch()` does not support `file://` URLs in Node, which is expected and not a bug in the
// build -- browsers serve the `.wasm` over http(s), where the default loader works unmodified).
//
// Usage: node scripts/wasm-sanity-check.mjs
// (Run `wasm-pack build crates/limen-wasm --target web --out-dir pkg --no-opt` first.)

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const pkgDir = fileURLToPath(new URL("../crates/limen-wasm/pkg/", import.meta.url));
const wasmPath = `${pkgDir}limen_wasm_bg.wasm`;
const jsPath = `${pkgDir}limen_wasm.js`;

let passCount = 0;
function check(label, fn) {
  fn();
  passCount += 1;
  console.log(`  ok - ${label}`);
}

function parseThrownError(thrown) {
  assert.equal(typeof thrown, "string", `expected the thrown value to be a string, got ${typeof thrown}: ${thrown}`);
  const parsed = JSON.parse(thrown); // must not throw: the payload must be valid JSON
  assert.equal(typeof parsed.error, "string", "error JSON must have a string 'error' code field");
  assert.ok("detail" in parsed, "error JSON must have a 'detail' field");
  return parsed;
}

async function main() {
  console.log(`Reading wasm bytes from ${wasmPath}`);
  const wasmBytes = readFileSync(wasmPath);

  console.log(`Importing glue module from ${jsPath}`);
  const wasmModule = await import(jsPath);
  const { initSync, engine_version, list_scenarios, list_strategies, get_scenario_detail, run_trial } =
    wasmModule;

  console.log("Instantiating module directly from file bytes (initSync, no fetch)...");
  initSync({ module: wasmBytes });
  console.log("Instantiated successfully.\n");

  console.log("=== engine_version ===");
  check("returns a non-empty semver-looking string", () => {
    const version = engine_version();
    assert.equal(typeof version, "string");
    assert.ok(/^\d+\.\d+\.\d+$/.test(version), `expected MAJOR.MINOR.PATCH, got ${version}`);
    console.log(`    engine_version() = ${version}`);
  });

  console.log("\n=== list_scenarios (success path) ===");
  let scenarioSummaries;
  check("returns valid JSON array of exactly 3 scenario summaries", () => {
    const raw = list_scenarios();
    scenarioSummaries = JSON.parse(raw);
    assert.equal(scenarioSummaries.length, 3);
    for (const summary of scenarioSummaries) {
      for (const field of ["scenario_id", "scenario_version", "title", "task_query", "item_count"]) {
        assert.ok(field in summary, `summary missing field '${field}': ${JSON.stringify(summary)}`);
      }
      assert.ok(summary.item_count >= 12 && summary.item_count <= 16);
    }
    console.log(`    scenarios: ${scenarioSummaries.map((s) => `${s.scenario_id} (${s.item_count} items)`).join(", ")}`);
  });

  console.log("\n=== list_strategies (success path) ===");
  let strategyDescriptors;
  check("returns valid JSON array of exactly 5 strategy descriptors, exact hierarchical label", () => {
    const raw = list_strategies();
    strategyDescriptors = JSON.parse(raw);
    assert.equal(strategyDescriptors.length, 5);
    const ids = strategyDescriptors.map((d) => d.strategy_id);
    assert.deepEqual(ids, [
      "full-input-truncation@1",
      "recency@1",
      "structured-extraction@1",
      "hierarchical-summary@1",
      "retrieval-ranking@1",
    ]);
    const hierarchical = strategyDescriptors.find((d) => d.strategy_id === "hierarchical-summary@1");
    assert.equal(hierarchical.label, "Hierarchical summary (deterministic/template-based)");
    console.log(`    strategies: ${ids.join(", ")}`);
  });

  console.log("\n=== get_scenario_detail (success path) ===");
  check("returns the full manifest including annotations for a known scenario", () => {
    const raw = get_scenario_detail("incident-investigation");
    const manifest = JSON.parse(raw);
    assert.equal(manifest.scenario_id, "incident-investigation");
    assert.ok(Array.isArray(manifest.items) && manifest.items.length > 0);
    assert.ok(Array.isArray(manifest.annotations.required_facts) && manifest.annotations.required_facts.length > 0);
    console.log(`    incident-investigation: ${manifest.items.length} items, ${manifest.annotations.required_facts.length} required facts`);
  });

  console.log("\n=== get_scenario_detail (error path) ===");
  check("throws a structured JSON string error for an unknown scenario id", () => {
    let threw = false;
    try {
      get_scenario_detail("no-such-scenario");
    } catch (thrown) {
      threw = true;
      const parsed = parseThrownError(thrown);
      assert.equal(parsed.error, "unknown_scenario_id");
      assert.equal(parsed.detail, "no-such-scenario");
      console.log(`    threw structured error: ${thrown}`);
    }
    assert.ok(threw, "expected get_scenario_detail to throw for an unknown scenario id");
  });

  console.log("\n=== run_trial (success path) ===");
  let rawTrialResult;
  check("returns the exact canonical TrialResult JSON string for a known scenario/strategy", () => {
    rawTrialResult = run_trial("incident-investigation", "retrieval-ranking@1", 300);
    assert.equal(typeof rawTrialResult, "string");
    assert.ok(!rawTrialResult.includes("\n"), "expected compact (no-newline) canonical JSON");
    const result = JSON.parse(rawTrialResult);
    assert.equal(result.scenario_id, "incident-investigation");
    assert.equal(result.strategy_id, "retrieval-ranking@1");
    assert.ok(result.result_digest.startsWith("sha256:"));
    assert.equal(result.metrics.budget.requested_tokens, 300);
    console.log(`    used ${result.metrics.budget.used_tokens}/${result.metrics.budget.requested_tokens} tokens, digest ${result.result_digest.slice(0, 24)}...`);
  });

  check("the returned string is byte-identical to itself re-fetched via a second call", () => {
    // run_trial is pure/deterministic: calling it again with identical inputs must return the
    // exact same bytes, proving there is no hidden nondeterminism (clock, RNG, map iteration)
    // crossing the boundary.
    const second = run_trial("incident-investigation", "retrieval-ranking@1", 300);
    assert.equal(second, rawTrialResult);
  });

  check("byte-exact download must use the raw string verbatim, never a JSON.parse/stringify round trip", () => {
    // V8 happens to preserve string-key insertion order, and our keys arrive pre-sorted and
    // compact from Rust, so this particular round trip happens to match today -- but that is an
    // implementation coincidence, not a contract: nothing about the JSON spec, `JSON.stringify`,
    // or any other JS engine guarantees reproducing Rust's exact canonical byte sequence (key
    // order across engines, number formatting, etc. are not standardized). The product
    // requirement is therefore to always persist `rawTrialResult` itself for downloads, never a
    // re-serialized copy; parsing is only for on-screen rendering.
    const roundTripped = JSON.stringify(JSON.parse(rawTrialResult));
    console.log(`    raw length=${rawTrialResult.length}, round-tripped length=${roundTripped.length}, coincidentally identical here=${roundTripped === rawTrialResult}`);
  });

  console.log("\n=== run_trial (error paths) ===");
  check("throws a structured JSON string error for an unknown scenario id", () => {
    let threw = false;
    try {
      run_trial("no-such-scenario", "retrieval-ranking@1", 100);
    } catch (thrown) {
      threw = true;
      const parsed = parseThrownError(thrown);
      assert.equal(parsed.error, "unknown_scenario_id");
    }
    assert.ok(threw);
  });

  check("throws a structured JSON string error for an unknown strategy id", () => {
    let threw = false;
    try {
      run_trial("incident-investigation", "no-such-strategy@1", 100);
    } catch (thrown) {
      threw = true;
      const parsed = parseThrownError(thrown);
      assert.equal(parsed.error, "unknown_strategy_version");
    }
    assert.ok(threw);
  });

  check("throws a structured JSON string error for a known family with an unsupported version", () => {
    let threw = false;
    try {
      run_trial("incident-investigation", "recency@99", 100);
    } catch (thrown) {
      threw = true;
      const parsed = parseThrownError(thrown);
      assert.equal(parsed.error, "unknown_strategy_version");
      assert.equal(parsed.detail, "recency@99");
    }
    assert.ok(threw);
  });

  console.log("\n=== run_trial (legal-but-extreme edge budgets: must succeed, not throw) ===");
  check("requested_tokens: 0 succeeds (does not throw) for every scenario x strategy", () => {
    for (const { scenario_id } of scenarioSummaries) {
      for (const { strategy_id } of strategyDescriptors) {
        const raw = run_trial(scenario_id, strategy_id, 0);
        const result = JSON.parse(raw);
        assert.equal(result.metrics.budget.requested_tokens, 0);
        assert.equal(result.metrics.budget.used_tokens, 0);
      }
    }
    console.log(`    ${scenarioSummaries.length} scenarios x ${strategyDescriptors.length} strategies @ 0 tokens: all succeeded`);
  });

  check("a 1-token budget on the largest embedded scenario succeeds for every strategy", () => {
    const largest = scenarioSummaries.reduce((a, b) => (b.item_count > a.item_count ? b : a));
    for (const { strategy_id } of strategyDescriptors) {
      const raw = run_trial(largest.scenario_id, strategy_id, 1);
      const result = JSON.parse(raw);
      assert.ok(result.metrics.budget.used_tokens <= 1);
    }
    console.log(`    largest scenario '${largest.scenario_id}' (${largest.item_count} items) @ 1 token: all ${strategyDescriptors.length} strategies succeeded`);
  });

  console.log(`\nAll ${passCount} checks passed.`);
}

main().catch((err) => {
  console.error("\nWASM sanity check FAILED:");
  console.error(err);
  process.exitCode = 1;
});
