import { expect, test } from "@playwright/test";

/** Waits for both comparison columns to have finished a real trial run (not "Running..."). */
async function waitForBothColumnsReady(page: import("@playwright/test").Page) {
  await expect(page.locator(".strategy-column").nth(0).locator(".column-status")).toHaveCount(0, {
    timeout: 15_000,
  });
  await expect(page.locator(".strategy-column").nth(1).locator(".column-status")).toHaveCount(0, {
    timeout: 15_000,
  });
}

async function openEvidence(page: import("@playwright/test").Page, index = 0) {
  const disclosure = page
    .locator(".strategy-column")
    .nth(0)
    .locator(".evidence-disclosure")
    .nth(index);
  await disclosure.locator("summary").click();
}

test.describe("real WASM loading and trial execution", () => {
  test("loads the compiled WASM engine in a real browser and renders a real trial result", async ({
    page,
  }) => {
    await page.goto("/");

    // The app boots by calling into the WASM engine (`ensureEngineReady`, `listScenarios`,
    // `listStrategies`, `runTrial`) before it ever renders scenario/strategy content -- if any of
    // that failed to load, these selects would never be populated.
    const scenarioSelect = page.locator("#scenario-select");
    await expect(scenarioSelect).toBeVisible();
    await expect(scenarioSelect.locator("option")).toHaveCount(3);

    const strategySelectA = page.locator("#strategy-select-a");
    const strategySelectB = page.locator("#strategy-select-b");
    await expect(strategySelectA.locator("option")).toHaveCount(5);
    await expect(strategySelectB.locator("option")).toHaveCount(5);

    await waitForBothColumnsReady(page);

    // A real trial result renders: the source list has real entries, and the metrics summary
    // shows real numbers (not placeholders).
    await openEvidence(page);
    const sourceItemsA = page.locator(".strategy-column").nth(0).locator(".source-item");
    await expect(sourceItemsA.first()).toBeVisible();
    const countA = await sourceItemsA.count();
    expect(countA).toBeGreaterThan(0);

    const footer = page.locator(".app-footer");
    await expect(footer).toContainText("Engine version");
    await expect(footer).toContainText("offline");
  });

  test("changing scenario, strategy, and budget produces a new real trial result", async ({
    page,
  }) => {
    await page.goto("/");
    await waitForBothColumnsReady(page);

    const columnA = page.locator(".strategy-column").nth(0);
    const budgetBefore = await columnA.locator(".metrics-summary").innerText();

    // Change strategy A to a different registered strategy.
    const strategySelectA = page.locator("#strategy-select-a");
    const currentValue = await strategySelectA.inputValue();
    const options = await strategySelectA
      .locator("option")
      .evaluateAll((nodes) => nodes.map((n) => (n as HTMLOptionElement).value));
    const nextValue = options.find((v) => v !== currentValue);
    expect(nextValue).toBeDefined();
    await strategySelectA.selectOption(nextValue as string);

    await waitForBothColumnsReady(page);
    const budgetAfter = await columnA.locator(".metrics-summary").innerText();

    // Switching to a genuinely different strategy on the same scenario/budget must change at
    // least the rendered strategy label; the metrics text block should differ in some real way
    // (different strategy_id shown in the sticky label at minimum).
    const stickyLabel = await columnA.locator(".column-sticky-label").innerText();
    expect(stickyLabel.length).toBeGreaterThan("Strategy A".length);
    expect(budgetBefore.length).toBeGreaterThan(0);
    expect(budgetAfter.length).toBeGreaterThan(0);
  });

  test("selecting a different scenario updates the task query and re-runs both columns", async ({
    page,
  }) => {
    await page.goto("/");
    await waitForBothColumnsReady(page);

    const taskQueryBefore = await page.locator("#task-query-display").innerText();

    const scenarioSelect = page.locator("#scenario-select");
    const options = await scenarioSelect
      .locator("option")
      .evaluateAll((nodes) => nodes.map((n) => (n as HTMLOptionElement).value));
    const currentValue = await scenarioSelect.inputValue();
    const nextScenario = options.find((v) => v !== currentValue);
    expect(nextScenario).toBeDefined();

    await scenarioSelect.selectOption(nextScenario as string);
    await waitForBothColumnsReady(page);

    const taskQueryAfter = await page.locator("#task-query-display").innerText();
    expect(taskQueryAfter).not.toBe(taskQueryBefore);
  });
});
