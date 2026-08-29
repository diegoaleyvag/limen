import { el, replaceChildren } from "../dom";
import type { ScenarioSummary } from "../types";

export interface ScenarioSelectorHandle {
  element: HTMLElement;
}

/** A native `<select>` listing the three real scenarios by title, plus the task query displayed
 * once a scenario is selected. */
export function createScenarioSelector(
  scenarios: ScenarioSummary[],
  initialScenarioId: string,
  onChange: (scenarioId: string) => void,
): ScenarioSelectorHandle {
  const taskQueryEl = el("p", { class: "task-query", id: "task-query-display" }, []);

  const setTaskQuery = (query: string): void => {
    replaceChildren(taskQueryEl, [
      el("span", { class: "field-label" }, ["Task query: "]),
      `\u201c${query}\u201d`,
    ]);
  };

  const select = el(
    "select",
    {
      id: "scenario-select",
      class: "control-select",
      onchange: () => {
        onChange(select.value);
        const chosen = scenarios.find((s) => s.scenario_id === select.value);
        if (chosen) setTaskQuery(chosen.task_query);
      },
    },
    scenarios.map((s) =>
      el("option", { value: s.scenario_id, selected: s.scenario_id === initialScenarioId }, [
        `${s.title} (${s.item_count} items)`,
      ]),
    ),
  );

  const wrapper = el("div", { class: "scenario-selector" }, [
    el("label", { class: "field", htmlFor: "scenario-select" }, [
      el("span", { class: "field-label" }, ["Scenario"]),
      select,
    ]),
    taskQueryEl,
  ]);

  const initial = scenarios.find((s) => s.scenario_id === initialScenarioId);
  if (initial) setTaskQuery(initial.task_query);

  return { element: wrapper };
}
