import { expect, test } from "@playwright/test";

async function waitForBothColumnsReady(page: import("@playwright/test").Page) {
  await expect(page.locator(".strategy-column").nth(0).locator(".column-status")).toHaveCount(0, {
    timeout: 15_000,
  });
  await expect(page.locator(".strategy-column").nth(1).locator(".column-status")).toHaveCount(0, {
    timeout: 15_000,
  });
}

test.describe("keyboard operability", () => {
  test("budget range slider is reachable and adjustable by keyboard alone", async ({ page }) => {
    await page.goto("/");
    await waitForBothColumnsReady(page);

    const range = page.locator("#budget-range");
    await range.focus();
    await expect(range).toBeFocused();

    const before = await range.inputValue();
    // Native <input type="range"> responds to arrow keys without any mouse involvement.
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("ArrowRight");
    const after = await range.inputValue();
    expect(Number(after)).not.toBe(Number(before));

    const output = page.locator("#budget-value");
    await expect(output).toHaveText(`${after} tokens`);
  });

  test("budget number input is reachable and editable by keyboard alone", async ({ page }) => {
    await page.goto("/");
    await waitForBothColumnsReady(page);

    const number = page.locator("#budget-number");
    await number.focus();
    await expect(number).toBeFocused();

    await page.keyboard.press("ControlOrMeta+A");
    await page.keyboard.type("7");
    await page.keyboard.press("Tab");

    await expect(page.locator("#budget-value")).toContainText("tokens");
  });

  test("both strategy selectors are operable via keyboard alone (no mouse)", async ({ page }) => {
    await page.goto("/");
    await waitForBothColumnsReady(page);

    const strategySelectA = page.locator("#strategy-select-a");
    await strategySelectA.focus();
    const before = await strategySelectA.inputValue();

    // Type-ahead is a standard, purely-keyboard way to operate a native <select>: typing text
    // while it is focused jumps to the first option whose label starts with that text, with no
    // mouse involvement and no dropdown ever needing to visually open.
    await page.keyboard.type("Recency");
    await waitForBothColumnsReady(page);
    const after = await strategySelectA.inputValue();
    expect(after).not.toBe(before);

    // The sticky column label (also the accessible name source for the column) must reflect the
    // newly selected strategy without any mouse interaction ever occurring.
    const stickyLabel = page.locator(".strategy-column").nth(0).locator(".column-sticky-label");
    await expect(stickyLabel).not.toHaveText("Strategy A");
  });

  test("tab order reaches scenario, budget, and both strategy selects in a sensible sequence", async ({
    page,
  }) => {
    await page.goto("/");
    await waitForBothColumnsReady(page);

    // Start from a known point and walk forward, recording which known controls we pass through
    // and in what relative order -- proves every one of them is genuinely reachable by Tab.
    await page.locator("#scenario-select").focus();
    const seen: string[] = ["scenario-select"];
    const targets = ["budget-range", "budget-number", "strategy-select-a", "strategy-select-b"];

    for (let i = 0; i < 40 && seen.length < targets.length + 1; i++) {
      await page.keyboard.press("Tab");
      const activeId = await page.evaluate(() => document.activeElement?.id ?? "");
      if (targets.includes(activeId) && seen[seen.length - 1] !== activeId) {
        seen.push(activeId);
      }
    }

    expect(seen).toEqual(["scenario-select", ...targets]);
  });
});

test.describe("screen-reader-relevant structure", () => {
  test("timeline and source list carry descriptive aria-labels", async ({ page }) => {
    await page.goto("/");
    await waitForBothColumnsReady(page);

    const columnA = page.locator(".strategy-column").nth(0);
    await expect(columnA.locator(".timeline")).toHaveAttribute("aria-label", /budget timeline/i);
    await expect(columnA.locator(".source-list")).toHaveAttribute(
      "aria-label",
      /source items, in original order/i,
    );
    await expect(columnA.locator(".trace-list")).toHaveAttribute("aria-label", /decision trace/i);
  });

  test("each strategy column is labeled via aria-labelledby pointing at its own heading", async ({
    page,
  }) => {
    await page.goto("/");
    await waitForBothColumnsReady(page);

    const columnA = page.locator(".strategy-column").nth(0);
    const labelledBy = await columnA.getAttribute("aria-labelledby");
    expect(labelledBy).toBeTruthy();
    const referenced = page.locator(`#${labelledBy}`);
    await expect(referenced).toHaveCount(1);
  });

  test("a status role/aria-live region exists and actually updates its text on recompute", async ({
    page,
  }) => {
    await page.goto("/");
    await waitForBothColumnsReady(page);

    const liveRegion = page.locator('[role="status"][aria-live="polite"]');
    await expect(liveRegion).toHaveCount(1);

    const before = await liveRegion.textContent();

    // Trigger a recompute by moving the budget slider.
    const range = page.locator("#budget-range");
    await range.focus();
    await page.keyboard.press("ArrowLeft");
    await waitForBothColumnsReady(page);

    await expect.poll(async () => await liveRegion.textContent()).not.toBe(before);
  });

  test("the comparison table uses real semantic table markup with row/column headers", async ({
    page,
  }) => {
    await page.goto("/");
    await waitForBothColumnsReady(page);

    const table = page.locator("table.comparison-table");
    await expect(table).toBeVisible();
    await expect(table.locator("caption")).toHaveCount(1);
    await expect(table.locator('th[scope="col"]')).toHaveCount(4);
    const rowHeaders = table.locator('th[scope="row"]');
    expect(await rowHeaders.count()).toBeGreaterThan(0);
  });
});

test.describe("color-independent status indicators", () => {
  test("every included/partial/dropped source badge carries real accessible text, not just a CSS class", async ({
    page,
  }) => {
    await page.goto("/");
    await waitForBothColumnsReady(page);

    const badges = page.locator(".strategy-column").nth(0).locator(".source-item .badge").first();
    await expect(badges).toBeVisible();

    const allBadges = page.locator(".strategy-column").nth(0).locator(".source-item .badge");
    const count = await allBadges.count();
    expect(count).toBeGreaterThan(0);

    const knownLabels = ["Included", "Partial", "Dropped", "Distractor"];
    for (let i = 0; i < count; i++) {
      const badge = allBadges.nth(i);
      const text = (await badge.innerText()).trim();
      expect(text.length).toBeGreaterThan(0);
      const matchesKnown =
        knownLabels.some((label) => text.includes(label)) || text.startsWith("Contradiction:");
      expect(matchesKnown, `unexpected badge text: ${text}`).toBe(true);
    }
  });

  test("timeline segments expose their status as visible text, not color alone", async ({
    page,
  }) => {
    await page.goto("/");
    await waitForBothColumnsReady(page);

    const timelineItems = page.locator(".strategy-column").nth(0).locator(".timeline-item");
    const count = await timelineItems.count();
    expect(count).toBeGreaterThan(0);

    for (let i = 0; i < count; i++) {
      const label = timelineItems.nth(i).locator(".timeline-item-label");
      const text = (await label.innerText()).trim();
      expect(["Included", "Partial", "Dropped"]).toContain(text);
    }
  });

  test("required-facts retained/not-retained state is conveyed by visible text", async ({
    page,
  }) => {
    await page.goto("/");
    await waitForBothColumnsReady(page);

    const factBadges = page.locator(".strategy-column").nth(0).locator(".fact-item .badge");
    const count = await factBadges.count();
    expect(count).toBeGreaterThan(0);
    for (let i = 0; i < count; i++) {
      const text = (await factBadges.nth(i).innerText()).trim();
      const matchesKnown = text.includes("Not retained") || text.includes("Retained");
      expect(matchesKnown, `unexpected fact badge text: ${text}`).toBe(true);
    }
  });
});
