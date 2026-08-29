import "./style.css";

import { createBudgetControl } from "./components/budgetControl";
import { createComparisonTable } from "./components/comparisonTable";
import { createScenarioSelector } from "./components/scenarioSelector";
import { createStrategyColumn, type StrategyColumnHandle } from "./components/strategyColumn";
import { el, replaceChildren } from "./dom";
import { createLiveRegion } from "./liveRegion";
import { defaultBudgetFromFullInputTokens, FULL_INPUT_PROBE_TOKENS, TrialCache } from "./state";
import { installTestHooks } from "./testHooks";
import type { EngineErrorPayload, ScenarioManifest, TrialOutcome } from "./types";
import {
  ensureEngineReady,
  getEngineVersion,
  getScenarioDetail,
  listScenarios,
  listStrategies,
  runTrial,
} from "./wasm/engine";

const FULL_INPUT_TRUNCATION_ID = "full-input-truncation@1";

async function main(): Promise<void> {
  const appRoot = document.getElementById("app");
  if (!appRoot) throw new Error("missing #app root element");

  await ensureEngineReady();

  const scenarios = listScenarios();
  const strategies = listStrategies();
  const firstScenario = scenarios[0];
  const firstStrategy = strategies[0];
  if (!firstScenario || !firstStrategy) {
    replaceChildren(appRoot, [
      el("p", { class: "column-error", attrs: { role: "alert" } }, [
        "The engine returned no scenarios or strategies.",
      ]),
    ]);
    return;
  }

  const cache = new TrialCache();
  const liveRegion = createLiveRegion();

  let currentManifest: ScenarioManifest | null = null;
  let requestedTokens = 0;
  let latestOutcomeA: TrialOutcome | null = null;
  let latestOutcomeB: TrialOutcome | null = null;

  installTestHooks((column) => (column === "a" ? latestOutcomeA : latestOutcomeB));

  const strategyLabel = (strategyId: string): string =>
    strategies.find((s) => s.strategy_id === strategyId)?.label ?? strategyId;

  const comparisonContainer = el("div", { class: "comparison-container" }, []);

  function updateComparisonTable(): void {
    if (latestOutcomeA && latestOutcomeB) {
      replaceChildren(comparisonContainer, [
        createComparisonTable(
          `A: ${strategyLabel(latestOutcomeA.parsed.strategy_id)}`,
          latestOutcomeA.parsed.metrics,
          `B: ${strategyLabel(latestOutcomeB.parsed.strategy_id)}`,
          latestOutcomeB.parsed.metrics,
        ),
      ]);
    } else {
      replaceChildren(comparisonContainer, [
        el("p", { class: "comparison-placeholder" }, [
          "The comparison table appears once both columns have a result.",
        ]),
      ]);
    }
  }

  function refreshColumn(columnId: "a" | "b"): void {
    if (!currentManifest) return;
    const column = columnId === "a" ? columnA : columnB;
    const strategyId = column.getStrategyId();
    const scenarioId = currentManifest.scenario_id;
    const manifest = currentManifest;

    const cacheParts = { scenarioId, strategyId, requestedTokens };
    const cached = cache.get(cacheParts);
    if (cached) {
      column.showResult(manifest, cached);
      if (columnId === "a") latestOutcomeA = cached;
      else latestOutcomeB = cached;
      liveRegion.announce(
        `Strategy ${columnId.toUpperCase()} result updated from cache: ${strategyLabel(strategyId)}.`,
      );
      updateComparisonTable();
      return;
    }

    column.showLoading();
    const result = runTrial(scenarioId, strategyId, requestedTokens);
    if (result.ok) {
      cache.set(cacheParts, result.value);
      column.showResult(manifest, result.value);
      if (columnId === "a") latestOutcomeA = result.value;
      else latestOutcomeB = result.value;
      liveRegion.announce(
        `Strategy ${columnId.toUpperCase()} result updated: ${strategyLabel(strategyId)}.`,
      );
    } else {
      column.showError(result.error);
      if (columnId === "a") latestOutcomeA = null;
      else latestOutcomeB = null;
      liveRegion.announce(
        `Strategy ${columnId.toUpperCase()} failed: ${describeError(result.error)}.`,
      );
    }
    updateComparisonTable();
  }

  const scenarioSelector = createScenarioSelector(
    scenarios,
    firstScenario.scenario_id,
    (scenarioId) => {
      loadScenario(scenarioId);
    },
  );

  const budgetControl = createBudgetControl(0, 1, (value) => {
    requestedTokens = value;
    refreshColumn("a");
    refreshColumn("b");
  });

  const secondStrategy = strategies[Math.min(1, strategies.length - 1)] ?? firstStrategy;
  const initialStrategyA = firstStrategy.strategy_id;
  const initialStrategyB = secondStrategy.strategy_id;

  const columnA: StrategyColumnHandle = createStrategyColumn(
    "a",
    "A",
    strategies,
    initialStrategyA,
    () => refreshColumn("a"),
  );
  const columnB: StrategyColumnHandle = createStrategyColumn(
    "b",
    "B",
    strategies,
    initialStrategyB,
    () => refreshColumn("b"),
  );

  function loadScenario(scenarioId: string): void {
    const detail = getScenarioDetail(scenarioId);
    if (!detail.ok) {
      currentManifest = null;
      liveRegion.announce(`Failed to load scenario: ${describeError(detail.error)}.`);
      replaceChildren(comparisonContainer, [
        el("p", { class: "column-error", attrs: { role: "alert" } }, [
          `Failed to load scenario: ${describeError(detail.error)}`,
        ]),
      ]);
      return;
    }
    currentManifest = detail.value;

    // Measure the scenario's full concatenated token count via one full-input-truncation@1 run
    // at a very large budget, so nothing is truncated; this becomes the slider's ceiling, and the
    // default budget sits at the 40-60% midpoint of it (see `state.ts`).
    const probe = runTrial(scenarioId, FULL_INPUT_TRUNCATION_ID, FULL_INPUT_PROBE_TOKENS);
    const fullInputTokens = probe.ok
      ? probe.value.parsed.metrics.budget.used_tokens
      : FULL_INPUT_PROBE_TOKENS;
    const defaultTokens = defaultBudgetFromFullInputTokens(fullInputTokens);

    budgetControl.setBounds(0, fullInputTokens);
    budgetControl.setValue(defaultTokens); // triggers onChange -> refreshColumn("a") and ("b")

    liveRegion.announce(
      `Scenario changed to ${detail.value.title}. Budget defaulted to ${defaultTokens} of ${fullInputTokens} tokens.`,
    );
  }

  const footer = el("footer", { class: "app-footer" }, [
    `Engine version ${getEngineVersion()}, offline, no network calls.`,
  ]);

  replaceChildren(appRoot, [
    el("div", { class: "app-shell" }, [
      el("header", { class: "app-header" }, [
        el("h1", {}, ["Limen"]),
        el("p", { class: "app-tagline" }, [
          "A deterministic lab for comparing context-selection strategies under fixed token budgets.",
        ]),
      ]),
      el("div", { class: "non-claims-banner", attrs: { role: "note" } }, [
        el("p", {}, [
          "Limen does not claim to benchmark real model quality. It isolates the context-construction decision so its tradeoffs are visible and reproducible.",
        ]),
        el("p", {}, [
          "Metrics evaluate selected context against annotated expected facts; they do not claim downstream LLM answer accuracy.",
        ]),
      ]),
      el(
        "section",
        { class: "controls-panel", attrs: { "aria-label": "Scenario and budget controls" } },
        [scenarioSelector.element, budgetControl.element],
      ),
      el("div", { class: "columns-grid" }, [columnA.element, columnB.element]),
      el("section", { class: "comparison-section" }, [
        el("h2", { class: "section-title" }, ["Comparison"]),
        comparisonContainer,
      ]),
      footer,
      liveRegion.element,
    ]),
  ]);

  loadScenario(firstScenario.scenario_id);
}

function describeError(error: EngineErrorPayload): string {
  return typeof error.detail === "string" ? `${error.error}: ${error.detail}` : error.error;
}

main().catch((err: unknown) => {
  const appRoot = document.getElementById("app");
  const message = err instanceof Error ? err.message : String(err);
  if (appRoot) {
    replaceChildren(appRoot, [
      el("p", { class: "column-error", attrs: { role: "alert" } }, [
        `Limen failed to start: ${message}`,
      ]),
    ]);
  }
  console.error(err);
});
