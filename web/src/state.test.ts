import { describe, expect, it } from "vitest";
import { clampBudget, defaultBudgetFromFullInputTokens, TrialCache, trialCacheKey } from "./state";
import type { TrialOutcome } from "./types";

describe("trialCacheKey", () => {
  it("joins the three parts with a distinct separator", () => {
    expect(
      trialCacheKey({
        scenarioId: "incident-investigation",
        strategyId: "recency@1",
        requestedTokens: 200,
      }),
    ).toBe("incident-investigation::recency@1::200");
  });

  it("never conflates two distinct combinations", () => {
    const a = trialCacheKey({ scenarioId: "s1", strategyId: "a", requestedTokens: 100 });
    const b = trialCacheKey({ scenarioId: "s1", strategyId: "a", requestedTokens: 101 });
    const c = trialCacheKey({ scenarioId: "s2", strategyId: "a", requestedTokens: 100 });
    const d = trialCacheKey({ scenarioId: "s1", strategyId: "b", requestedTokens: 100 });
    const keys = new Set([a, b, c, d]);
    expect(keys.size).toBe(4);
  });
});

function fakeOutcome(raw: string): TrialOutcome {
  return {
    raw,
    parsed: JSON.parse(`{"marker": ${JSON.stringify(raw)}}`),
  } as unknown as TrialOutcome;
}

describe("TrialCache", () => {
  it("returns undefined for a miss and the exact stored object for a hit", () => {
    const cache = new TrialCache();
    const parts = { scenarioId: "s1", strategyId: "recency@1", requestedTokens: 50 };
    expect(cache.get(parts)).toBeUndefined();
    expect(cache.has(parts)).toBe(false);

    const outcome = fakeOutcome("raw-json-1");
    cache.set(parts, outcome);

    expect(cache.has(parts)).toBe(true);
    expect(cache.get(parts)).toBe(outcome); // exact same object, not a re-derived copy
    expect(cache.size).toBe(1);
  });

  it("keeps distinct entries for distinct keys and clear() empties it", () => {
    const cache = new TrialCache();
    cache.set({ scenarioId: "s1", strategyId: "a", requestedTokens: 1 }, fakeOutcome("one"));
    cache.set({ scenarioId: "s1", strategyId: "b", requestedTokens: 1 }, fakeOutcome("two"));
    expect(cache.size).toBe(2);

    cache.clear();
    expect(cache.size).toBe(0);
    expect(cache.get({ scenarioId: "s1", strategyId: "a", requestedTokens: 1 })).toBeUndefined();
  });
});

describe("defaultBudgetFromFullInputTokens", () => {
  it("returns the 50% midpoint, rounded", () => {
    expect(defaultBudgetFromFullInputTokens(1000)).toBe(500);
    expect(defaultBudgetFromFullInputTokens(1001)).toBe(501); // rounds to nearest
  });

  it("never returns less than 1, even for a tiny or zero full-input count", () => {
    expect(defaultBudgetFromFullInputTokens(0)).toBe(1);
    expect(defaultBudgetFromFullInputTokens(1)).toBe(1);
  });
});

describe("clampBudget", () => {
  const bounds = { min: 0, max: 100 };

  it("passes through an in-range integer unchanged", () => {
    expect(clampBudget(50, bounds)).toBe(50);
  });

  it("clamps above max down to max", () => {
    expect(clampBudget(9999, bounds)).toBe(100);
  });

  it("clamps below min up to min", () => {
    expect(clampBudget(-5, bounds)).toBe(0);
  });

  it("floors a fractional value", () => {
    expect(clampBudget(50.9, bounds)).toBe(50);
  });

  it("treats NaN as the minimum rather than propagating it", () => {
    expect(clampBudget(Number.NaN, bounds)).toBe(bounds.min);
  });

  it("treats +/-Infinity as clamped to bounds, not propagated", () => {
    expect(clampBudget(Number.POSITIVE_INFINITY, bounds)).toBe(bounds.min);
    expect(clampBudget(Number.NEGATIVE_INFINITY, bounds)).toBe(bounds.min);
  });

  it("respects a non-zero min bound", () => {
    expect(clampBudget(5, { min: 10, max: 100 })).toBe(10);
  });
});
