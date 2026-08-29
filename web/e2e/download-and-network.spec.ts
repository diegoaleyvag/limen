import { readFileSync } from "node:fs";
import { expect, test } from "@playwright/test";

async function waitForBothColumnsReady(page: import("@playwright/test").Page) {
  await expect(page.locator(".strategy-column").nth(0).locator(".column-status")).toHaveCount(0, {
    timeout: 15_000,
  });
  await expect(page.locator(".strategy-column").nth(1).locator(".column-status")).toHaveCount(0, {
    timeout: 15_000,
  });
}

test.describe("byte-exact downloads", () => {
  test("the downloaded file's bytes exactly match the raw WASM run_trial output, not a re-serialized copy", async ({
    page,
  }) => {
    await page.goto("/");
    await waitForBothColumnsReady(page);

    const columnA = page.locator(".strategy-column").nth(0);
    const downloadButton = columnA.locator("button", { hasText: "Download trial result" });
    await expect(downloadButton).toBeVisible();

    const [download] = await Promise.all([page.waitForEvent("download"), downloadButton.click()]);

    const path = await download.path();
    expect(path).not.toBeNull();
    const downloadedBytes = readFileSync(path as string);
    const downloadedText = downloadedBytes.toString("utf-8");

    // Read back the exact raw string the app's in-memory `TrialOutcome` holds for column A --
    // the same string `run_trial` returned across the WASM boundary, never re-parsed/re-stringified.
    const expectedRaw = await page.evaluate(() => window.__limenTestHooks__?.getLastRaw("a"));
    expect(expectedRaw).not.toBeNull();

    expect(downloadedText).toBe(expectedRaw);
    // Also compare raw byte length, in case any lossy string transcoding happened anywhere.
    expect(downloadedBytes.byteLength).toBe(Buffer.byteLength(expectedRaw as string, "utf-8"));

    // Sanity: it must be real, valid, canonical (compact, single-line) TrialResult JSON.
    expect(() => JSON.parse(downloadedText)).not.toThrow();
    expect(downloadedText.includes("\n")).toBe(false);
    const parsed = JSON.parse(downloadedText);
    expect(typeof parsed.result_digest).toBe("string");
    expect(parsed.result_digest.startsWith("sha256:")).toBe(true);
  });

  test("the downloaded filename encodes scenario, strategy, and requested tokens", async ({
    page,
  }) => {
    await page.goto("/");
    await waitForBothColumnsReady(page);

    const scenarioId = await page.locator("#scenario-select").inputValue();
    const strategyId = await page.locator("#strategy-select-a").inputValue();
    const requestedTokens = await page.locator("#budget-number").inputValue();

    const downloadButton = page
      .locator(".strategy-column")
      .nth(0)
      .locator("button", { hasText: "Download trial result" });
    const [download] = await Promise.all([page.waitForEvent("download"), downloadButton.click()]);

    expect(download.suggestedFilename()).toBe(
      `limen-trial-${scenarioId}-${strategyId}-${requestedTokens}.json`,
    );
  });
});

test.describe("rejection of external requests (offline claim)", () => {
  test("every network request during a full interaction session stays same-origin", async ({
    page,
    baseURL,
  }) => {
    const requestedUrls: string[] = [];
    page.on("request", (request) => {
      requestedUrls.push(request.url());
    });

    await page.goto("/");
    await waitForBothColumnsReady(page);

    // Exercise a realistic full session: change scenario, change both strategies, move the
    // budget, and download a result -- the same set of interactions the other e2e specs cover
    // individually, combined here specifically to maximize the surface checked for stray
    // external requests.
    const scenarioSelect = page.locator("#scenario-select");
    const scenarioOptions = await scenarioSelect
      .locator("option")
      .evaluateAll((nodes) => nodes.map((n) => (n as HTMLOptionElement).value));
    await scenarioSelect.selectOption(scenarioOptions[1] ?? scenarioOptions[0]);
    await waitForBothColumnsReady(page);

    await page.locator("#strategy-select-a").focus();
    await page.keyboard.type("Recency");
    await waitForBothColumnsReady(page);

    await page.locator("#strategy-select-b").focus();
    await page.keyboard.type("Structured");
    await waitForBothColumnsReady(page);

    const range = page.locator("#budget-range");
    await range.focus();
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("ArrowRight");
    await waitForBothColumnsReady(page);

    const downloadButton = page
      .locator(".strategy-column")
      .nth(0)
      .locator("button", { hasText: "Download trial result" });
    await Promise.all([page.waitForEvent("download"), downloadButton.click()]);

    expect(requestedUrls.length).toBeGreaterThan(0);

    const baseOrigin = new URL(baseURL ?? "http://127.0.0.1:4173").origin;
    const offendingRequests = requestedUrls.filter((url) => {
      if (url.startsWith("data:") || url.startsWith("blob:") || url.startsWith("about:")) {
        return false;
      }
      try {
        return new URL(url).origin !== baseOrigin;
      } catch {
        return false;
      }
    });

    expect(
      offendingRequests,
      `unexpected non-same-origin requests: ${offendingRequests.join(", ")}`,
    ).toEqual([]);
  });
});
