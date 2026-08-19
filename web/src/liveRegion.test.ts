import { describe, expect, it } from "vitest";
import { createLiveRegion } from "./liveRegion";

describe("createLiveRegion", () => {
  it("renders an aria-live polite status region", () => {
    const region = createLiveRegion();
    expect(region.element.getAttribute("aria-live")).toBe("polite");
    expect(region.element.getAttribute("role")).toBe("status");
  });

  it("sets textContent to the announced message", () => {
    const region = createLiveRegion();
    region.announce("Scenario changed to Incident Investigation.");
    expect(region.element.textContent).toBe("Scenario changed to Incident Investigation.");
  });

  it("toggles a trailing non-breaking space when the same message repeats, so a real text mutation always occurs", () => {
    const region = createLiveRegion();
    region.announce("Result updated.");
    const first = region.element.textContent;
    region.announce("Result updated.");
    const second = region.element.textContent;

    expect(first).toBe("Result updated.");
    expect(second).not.toBe(first); // must actually change so aria-live fires again
    expect(second).toBe("Result updated.\u00a0");
  });

  it("does not append a trailing space when consecutive messages differ", () => {
    const region = createLiveRegion();
    region.announce("First message.");
    region.announce("Second message.");
    expect(region.element.textContent).toBe("Second message.");
  });
});
