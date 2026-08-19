// Thin typed wrapper around the `wasm-pack --target web` output at `./pkg/`. This is the *only*
// place `web/src` imports the generated bindings: everything else in the app calls the functions
// exported from this module. No tokenization, selection, validation, metrics, or digesting logic
// is duplicated here or anywhere else in this package; every one of those concerns is computed
// exactly once, in Rust, and this module only parses the JSON strings the engine returns.

import type {
  EngineErrorPayload,
  EngineResult,
  ScenarioManifest,
  ScenarioSummary,
  StrategyDescriptor,
  TrialOutcome,
  TrialResult,
} from "../types";
import initWasm, {
  engine_version,
  get_scenario_detail,
  list_scenarios,
  list_strategies,
  run_trial,
} from "./pkg/limen_wasm.js";

let initPromise: Promise<void> | null = null;

/** Instantiates the WASM module exactly once, no matter how many callers await this. Every other
 * export in this module assumes this has already resolved. */
export function ensureEngineReady(): Promise<void> {
  if (!initPromise) {
    initPromise = initWasm().then(() => undefined);
  }
  return initPromise;
}

/** Parses a value thrown across the WASM boundary back into a typed `EngineErrorPayload`. Every
 * `Err` this engine ever produces is a JSON string shaped like `{"error": "...", "detail": ...}`
 * (see `crates/limen-wasm/src/lib.rs`'s module docs), so this should always succeed for a real
 * engine error; the fallback branch exists only so a genuinely unexpected thrown value still
 * becomes a well-typed, displayable error instead of an uncaught exception reaching the UI layer. */
function parseEngineError(thrown: unknown): EngineErrorPayload {
  if (typeof thrown === "string") {
    try {
      const parsed: unknown = JSON.parse(thrown);
      if (
        parsed !== null &&
        typeof parsed === "object" &&
        "error" in parsed &&
        typeof parsed.error === "string" &&
        "detail" in parsed
      ) {
        return parsed as EngineErrorPayload;
      }
    } catch {
      // Falls through to the generic fallback below.
    }
  }
  return {
    error: "canonicalization_failed",
    detail: `unrecognized error payload from the engine: ${String(thrown)}`,
  };
}

export function getEngineVersion(): string {
  return engine_version();
}

export function listScenarios(): ScenarioSummary[] {
  return JSON.parse(list_scenarios()) as ScenarioSummary[];
}

export function listStrategies(): StrategyDescriptor[] {
  return JSON.parse(list_strategies()) as StrategyDescriptor[];
}

/** The full manifest (items and evaluator-side `annotations`) for one embedded scenario, parsed
 * from a *copy* of the engine's JSON string. Never used to reconstruct a downloadable artifact. */
export function getScenarioDetail(scenarioId: string): EngineResult<ScenarioManifest> {
  try {
    const raw = get_scenario_detail(scenarioId);
    return { ok: true, value: JSON.parse(raw) as ScenarioManifest };
  } catch (thrown) {
    return { ok: false, error: parseEngineError(thrown) };
  }
}

/** Runs one trial. `value.raw` is the untouched string the engine returned: the only thing that
 * may ever be written to a download. `value.parsed` is a parsed *copy*, for rendering only. */
export function runTrial(
  scenarioId: string,
  strategyId: string,
  requestedTokens: number,
): EngineResult<TrialOutcome> {
  try {
    const raw = run_trial(scenarioId, strategyId, requestedTokens);
    const parsed = JSON.parse(raw) as TrialResult;
    return { ok: true, value: { raw, parsed } };
  } catch (thrown) {
    return { ok: false, error: parseEngineError(thrown) };
  }
}
