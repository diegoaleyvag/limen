import { el } from "../dom";
import { presentAction } from "../format";
import type { TraceStep } from "../types";

/**
 * The expandable decision trace: a native `<details>`/`<summary>` disclosure (no reinvented
 * accordion) revealing the ordered trace, one row per step, showing action/score/detail exactly
 * as the strategy recorded it, in its own processing order (which may differ from `order_index`
 * order; see `docs/STRATEGIES.md`).
 */
export function createDecisionTrace(trace: TraceStep[]): HTMLElement {
  const rows = trace.map((step) =>
    el("li", { class: "trace-step" }, [
      el("span", { class: "trace-step-index" }, [`${step.step_index}.`]),
      el("span", { class: "trace-step-source" }, [step.source_id]),
      el("span", { class: "trace-step-action" }, [presentAction(step.action)]),
      el("span", { class: "trace-step-score" }, [
        step.score === null ? "(not score-based)" : `score ${step.score}`,
      ]),
      el("p", { class: "trace-step-detail" }, [step.detail]),
    ]),
  );

  return el("details", { class: "decision-trace" }, [
    el("summary", {}, [`Decision trace (${trace.length} steps)`]),
    el(
      "ol",
      { class: "trace-list", attrs: { "aria-label": "Decision trace, in processing order" } },
      rows,
    ),
  ]);
}
