import { el, replaceChildren } from "../dom";
import { downloadTrialResult } from "../download";
import type {
  EngineErrorPayload,
  ScenarioManifest,
  StrategyDescriptor,
  TrialOutcome,
} from "../types";
import { createContradictionsSection } from "./contradictions";
import { createDecisionTrace } from "./decisionTrace";
import { createFactsList } from "./factsList";
import { createMetricsSummary } from "./metricsSummary";
import { createSourceList } from "./sourceList";
import { createTimeline } from "./timeline";

export interface StrategyColumnHandle {
  element: HTMLElement;
  getStrategyId: () => string;
  showLoading: () => void;
  showError: (error: EngineErrorPayload) => void;
  showResult: (manifest: ScenarioManifest, outcome: TrialOutcome) => void;
}

/**
 * One independent comparison column: its own strategy `<select>` (listing all five strategies),
 * a persistent/sticky strategy-name label (visible at narrow widths so scroll position never
 * costs a user their sense of which column they are reading), and the full per-column result
 * view (budget timeline, source items, required facts, contradiction call-outs, metrics summary,
 * decision trace, download button). Reading the shared scenario + shared budget happens in the
 * caller (`main.ts`); this module only renders whatever it is given.
 */
export function createStrategyColumn(
  columnId: "a" | "b",
  columnLetter: "A" | "B",
  strategies: StrategyDescriptor[],
  initialStrategyId: string,
  onStrategyChange: (strategyId: string) => void,
): StrategyColumnHandle {
  const selectId = `strategy-select-${columnId}`;
  const titleId = `strategy-column-title-${columnId}`;

  const stickyLabel = el("h2", { class: "column-sticky-label", id: titleId }, [
    `Strategy ${columnLetter}`,
  ]);

  const select = el(
    "select",
    {
      id: selectId,
      class: "control-select",
      onchange: () => {
        onStrategyChange(select.value);
        updateHeaderText();
      },
    },
    strategies.map((s) =>
      el("option", { value: s.strategy_id, selected: s.strategy_id === initialStrategyId }, [
        s.label,
      ]),
    ),
  );

  const summaryEl = el("p", { class: "strategy-summary" }, []);

  function updateHeaderText(): void {
    const current = strategies.find((s) => s.strategy_id === select.value);
    summaryEl.textContent = current?.summary ?? "";
    stickyLabel.textContent = current
      ? `Strategy ${columnLetter}: ${current.label}`
      : `Strategy ${columnLetter}`;
  }
  updateHeaderText();

  const body = el("div", { class: "column-body" }, []);

  const element = el(
    "section",
    { class: "strategy-column", attrs: { "aria-labelledby": titleId } },
    [
      stickyLabel,
      el("label", { class: "field", htmlFor: selectId }, [
        el("span", { class: "field-label" }, [`Strategy ${columnLetter}`]),
        select,
      ]),
      summaryEl,
      body,
    ],
  );

  const showLoading = (): void => {
    replaceChildren(body, [el("p", { class: "column-status" }, ["Running..."])]);
  };

  const showError = (error: EngineErrorPayload): void => {
    const detailText = typeof error.detail === "string" ? `: ${error.detail}` : "";
    replaceChildren(body, [
      el("div", { class: "column-error", attrs: { role: "alert" } }, [
        el("span", { class: "badge status-danger" }, ["Error"]),
        el("p", {}, [`${error.error}${detailText}`]),
      ]),
    ]);
  };

  const showResult = (manifest: ScenarioManifest, outcome: TrialOutcome): void => {
    const { parsed, raw } = outcome;

    const downloadButton = el(
      "button",
      {
        type: "button",
        class: "button button-primary",
        onclick: () => {
          downloadTrialResult(
            manifest.scenario_id,
            parsed.strategy_id,
            parsed.metrics.budget.requested_tokens,
            raw,
          );
        },
      },
      ["Download trial result (.json)"],
    );

    replaceChildren(body, [
      el("h3", { class: "subsection-title" }, ["Budget timeline"]),
      createTimeline(manifest.items, parsed.selection.selection),
      el("details", { class: "evidence-disclosure" }, [
        el("summary", {}, ["Evidence: source items"]),
        createSourceList(manifest.items, parsed.selection.selection, manifest.annotations),
      ]),
      el("details", { class: "evidence-disclosure" }, [
        el("summary", {}, [
          `Required facts: ${parsed.metrics.fact_recall.retained} / ${parsed.metrics.fact_recall.required} retained`,
        ]),
        createFactsList(manifest.annotations, parsed.metrics.fact_recall),
      ]),
      createContradictionsSection(parsed.metrics.contradictions),
      createMetricsSummary(parsed.metrics),
      createDecisionTrace(parsed.selection.trace),
      el("div", { class: "download-row" }, [downloadButton]),
    ]);
  };

  return {
    element,
    getStrategyId: () => select.value,
    showLoading,
    showError,
    showResult,
  };
}
