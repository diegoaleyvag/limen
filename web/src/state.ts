// Pure, DOM-free and WASM-free state helpers: the trial result cache and small budget-arithmetic
// helpers. Kept dependency-free on purpose so it is directly unit-testable with Vitest without a
// jsdom environment or a real WASM instance.
import type { TrialOutcome } from "./types";

export interface TrialCacheKeyParts {
  scenarioId: string;
  strategyId: string;
  requestedTokens: number;
}

/** Builds the cache key for one (scenario_id, strategy_id, requested_tokens) combination. Exported
 * on its own (not just as a private detail of `TrialCache`) so tests can assert on the exact key
 * shape and so two different combinations are provably never conflated. */
export function trialCacheKey({
  scenarioId,
  strategyId,
  requestedTokens,
}: TrialCacheKeyParts): string {
  return `${scenarioId}::${strategyId}::${requestedTokens}`;
}

/**
 * Client-side cache of computed trial results, keyed by `(scenario_id, strategy_id,
 * requested_tokens)`, so re-selecting a previously-seen combination is instant rather than
 * re-invoking the WASM engine. A cache hit returns the exact same `TrialOutcome` object
 * (`raw`/`parsed`) that was stored, never a re-derived copy.
 */
export class TrialCache {
  #entries = new Map<string, TrialOutcome>();

  get(parts: TrialCacheKeyParts): TrialOutcome | undefined {
    return this.#entries.get(trialCacheKey(parts));
  }

  set(parts: TrialCacheKeyParts, outcome: TrialOutcome): void {
    this.#entries.set(trialCacheKey(parts), outcome);
  }

  has(parts: TrialCacheKeyParts): boolean {
    return this.#entries.has(trialCacheKey(parts));
  }

  get size(): number {
    return this.#entries.size;
  }

  clear(): void {
    this.#entries.clear();
  }
}

/** A very large probe budget passed to one `full-input-truncation@1` call per scenario, used only
 * to measure that scenario's full concatenated token count (nothing this large will ever be
 * truncated by any of the three embedded scenarios). */
export const FULL_INPUT_PROBE_TOKENS = 1_000_000;

/** Given a scenario's full concatenated token count (as measured by the probe above), returns a
 * default requested-token budget at the midpoint of the product brief's 40 to 60 percent band, so
 * the initial view demonstrates a real selection tradeoff rather than a trivial full-fit or
 * fully-empty result. */
export function defaultBudgetFromFullInputTokens(fullInputTokenCount: number): number {
  return Math.max(1, Math.round(fullInputTokenCount * 0.5));
}

export interface BudgetBounds {
  min: number;
  max: number;
}

/** Clamps a requested budget into `[bounds.min, bounds.max]`, flooring to a whole token count
 * (`requested_tokens` is a `u32` on the Rust side) and treating `NaN` (e.g. a cleared number
 * input) as the minimum rather than propagating an invalid value into a WASM call. */
export function clampBudget(value: number, bounds: BudgetBounds): number {
  if (!Number.isFinite(value)) return bounds.min;
  const rounded = Math.floor(value);
  return Math.min(Math.max(rounded, bounds.min), bounds.max);
}
