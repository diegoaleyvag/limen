import { el } from "../dom";
import { clampBudget } from "../state";

export interface BudgetControlHandle {
  element: HTMLElement;
  setBounds: (min: number, max: number) => void;
  setValue: (value: number) => void;
  getValue: () => number;
}

/**
 * One shared budget control: a native `<input type="range">` paired with a native
 * `<input type="number">`, kept in sync in both directions. Shared across both comparison
 * columns (there is exactly one instance of this control on the page), both keyboard-operable,
 * both grouped under one visible label plus a `role="group"` for screen readers.
 */
export function createBudgetControl(
  initialValue: number,
  initialMax: number,
  onChange: (value: number) => void,
): BudgetControlHandle {
  let min = 0;
  let max = initialMax;

  const valueDisplay = el("output", { id: "budget-value", class: "budget-value" }, [
    `${initialValue} tokens`,
  ]);

  const range = el("input", {
    type: "range",
    id: "budget-range",
    class: "control-range",
    min: String(min),
    max: String(max),
    step: "1",
    value: String(initialValue),
    attrs: {
      "aria-label": "Token budget, slider",
      "aria-describedby": "budget-value",
    },
  });

  const number = el("input", {
    type: "number",
    id: "budget-number",
    class: "control-number",
    min: String(min),
    max: String(max),
    step: "1",
    value: String(initialValue),
    attrs: {
      "aria-label": "Token budget, exact value",
      "aria-describedby": "budget-value",
    },
  });

  const commit = (raw: number): void => {
    const clamped = clampBudget(raw, { min, max });
    range.value = String(clamped);
    number.value = String(clamped);
    valueDisplay.textContent = `${clamped} tokens`;
    onChange(clamped);
  };

  range.addEventListener("input", () => commit(Number(range.value)));
  number.addEventListener("input", () => commit(Number(number.value)));

  const wrapper = el("div", { class: "budget-control" }, [
    el("span", { class: "field-label", id: "budget-label" }, [
      "Token budget (shared across both columns)",
    ]),
    el(
      "div",
      {
        class: "budget-control-row",
        attrs: { role: "group", "aria-labelledby": "budget-label" },
      },
      [range, number, valueDisplay],
    ),
  ]);

  return {
    element: wrapper,
    setBounds: (newMin, newMax) => {
      min = newMin;
      max = newMax;
      range.min = String(min);
      range.max = String(max);
      number.min = String(min);
      number.max = String(max);
    },
    setValue: (value) => commit(value),
    getValue: () => Number(range.value),
  };
}
