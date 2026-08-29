import { expect, test } from "@playwright/test";

async function waitForBothColumnsReady(page: import("@playwright/test").Page) {
  await expect(page.locator(".strategy-column").nth(0).locator(".column-status")).toHaveCount(0, {
    timeout: 15_000,
  });
  await expect(page.locator(".strategy-column").nth(1).locator(".column-status")).toHaveCount(0, {
    timeout: 15_000,
  });
}

test.describe("mobile stacking", () => {
  test.use({ viewport: { width: 390, height: 844 } }); // narrower than the 56rem (896px) breakpoint

  test("column A renders above column B, with both sticky strategy labels visible", async ({
    page,
  }) => {
    await page.goto("/");
    await waitForBothColumnsReady(page);

    const columnA = page.locator(".strategy-column").nth(0);
    const columnB = page.locator(".strategy-column").nth(1);

    const boxA = await columnA.boundingBox();
    const boxB = await columnB.boundingBox();
    if (boxA === null || boxB === null) {
      throw new Error("expected both columns to have a bounding box");
    }
    // "Above" means A's top edge is strictly less than B's top edge (single-column stack).
    expect(boxA.y).toBeLessThan(boxB.y);

    const labelA = columnA.locator(".column-sticky-label");
    const labelB = columnB.locator(".column-sticky-label");
    await expect(labelA).toBeVisible();
    await expect(labelB).toBeVisible();
    await expect(labelA).toContainText("Strategy A");
    await expect(labelB).toContainText("Strategy B");
  });

  test("the columns grid collapses to a single track at narrow width", async ({ page }) => {
    await page.goto("/");
    await waitForBothColumnsReady(page);

    const gridTemplateColumns = await page
      .locator(".columns-grid")
      .evaluate((el) => getComputedStyle(el).gridTemplateColumns);
    // A single-track grid reports exactly one column width, not "Npx Mpx" (two or more tracks).
    const trackCount = gridTemplateColumns.trim().split(/\s+/).length;
    expect(trackCount).toBe(1);
  });
});

test.describe("desktop layout (contrast case)", () => {
  test.use({ viewport: { width: 1280, height: 900 } });

  test("columns render side by side (two grid tracks) at a wide viewport", async ({ page }) => {
    await page.goto("/");
    await waitForBothColumnsReady(page);

    const gridTemplateColumns = await page
      .locator(".columns-grid")
      .evaluate((el) => getComputedStyle(el).gridTemplateColumns);
    const trackCount = gridTemplateColumns.trim().split(/\s+/).length;
    expect(trackCount).toBe(2);
  });
});

test.describe("reduced motion", () => {
  test("no transition duration is applied to interactive elements when reduced motion is requested", async ({
    page,
  }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto("/");
    await waitForBothColumnsReady(page);

    // The button's `transition` declaration lives entirely inside a
    // `@media (prefers-reduced-motion: no-preference)` guard, so under `reduce` there is nothing
    // to override: the computed transition-duration must be 0s.
    const button = page.locator("button.button").first();
    await expect(button).toBeVisible();
    const transitionDuration = await button.evaluate(
      (el) => getComputedStyle(el).transitionDuration,
    );
    for (const duration of transitionDuration.split(",")) {
      expect(duration.trim()).toBe("0s");
    }
  });

  test("interacting with controls under reduced motion still recomputes correctly (no broken behavior)", async ({
    page,
  }) => {
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.goto("/");
    await waitForBothColumnsReady(page);

    const range = page.locator("#budget-range");
    const before = await range.inputValue();
    await range.focus();
    await page.keyboard.press("ArrowRight");
    await waitForBothColumnsReady(page);
    const after = await range.inputValue();
    expect(after).not.toBe(before);

    // A real, non-empty trial result must still be present -- reduced motion must never break
    // functional recomputation.
    const sourceItems = page.locator(".strategy-column").nth(0).locator(".source-item");
    expect(await sourceItems.count()).toBeGreaterThan(0);
  });
});
