import { describe, expect, it, vi } from "vitest";
import { createBudgetControl } from "./budgetControl";

function fireInput(node: HTMLInputElement, value: string): void {
  node.value = value;
  node.dispatchEvent(new Event("input", { bubbles: true }));
}

describe("createBudgetControl", () => {
  it("renders a labeled group containing a range slider, a number input, and a value display", () => {
    const control = createBudgetControl(50, 100, () => {});
    const group = control.element.querySelector('[role="group"]');
    expect(group).not.toBeNull();

    const range = control.element.querySelector<HTMLInputElement>('input[type="range"]');
    const number = control.element.querySelector<HTMLInputElement>('input[type="number"]');
    const output = control.element.querySelector("output");

    expect(range).not.toBeNull();
    expect(number).not.toBeNull();
    expect(output?.textContent).toBe("50 tokens");
  });

  it("both the range and number inputs carry an aria-label (keyboard/screen-reader operable)", () => {
    const control = createBudgetControl(0, 100, () => {});
    const range = control.element.querySelector<HTMLInputElement>('input[type="range"]');
    const number = control.element.querySelector<HTMLInputElement>('input[type="number"]');
    expect(range?.getAttribute("aria-label")).toBeTruthy();
    expect(number?.getAttribute("aria-label")).toBeTruthy();
  });

  it("moving the range input calls onChange with the clamped value and syncs the number input", () => {
    const onChange = vi.fn();
    const control = createBudgetControl(10, 100, onChange);
    const range = control.element.querySelector<HTMLInputElement>('input[type="range"]');
    const number = control.element.querySelector<HTMLInputElement>('input[type="number"]');
    expect(range).not.toBeNull();
    expect(number).not.toBeNull();

    fireInput(range as HTMLInputElement, "75");

    expect(onChange).toHaveBeenCalledWith(75);
    expect(number?.value).toBe("75");
    expect(control.element.querySelector("output")?.textContent).toBe("75 tokens");
  });

  it("typing in the number input calls onChange and syncs the range input", () => {
    const onChange = vi.fn();
    const control = createBudgetControl(10, 100, onChange);
    const number = control.element.querySelector<HTMLInputElement>('input[type="number"]');
    const range = control.element.querySelector<HTMLInputElement>('input[type="range"]');

    fireInput(number as HTMLInputElement, "42");

    expect(onChange).toHaveBeenCalledWith(42);
    expect(range?.value).toBe("42");
  });

  it("clamps an out-of-bounds typed value before calling onChange", () => {
    const onChange = vi.fn();
    const control = createBudgetControl(10, 100, onChange);
    const number = control.element.querySelector<HTMLInputElement>("input[type='number']");

    fireInput(number as HTMLInputElement, "99999");

    expect(onChange).toHaveBeenCalledWith(100);
    expect(number?.value).toBe("100");
  });

  it("setBounds updates min/max on both inputs", () => {
    const control = createBudgetControl(10, 100, () => {});
    control.setBounds(0, 500);
    const range = control.element.querySelector<HTMLInputElement>('input[type="range"]');
    const number = control.element.querySelector<HTMLInputElement>('input[type="number"]');
    expect(range?.max).toBe("500");
    expect(number?.max).toBe("500");
  });

  it("setValue programmatically commits a value and getValue reflects it", () => {
    const onChange = vi.fn();
    const control = createBudgetControl(10, 100, onChange);
    control.setValue(33);
    expect(onChange).toHaveBeenCalledWith(33);
    expect(control.getValue()).toBe(33);
  });
});
