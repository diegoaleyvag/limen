import { describe, expect, it } from "vitest";
import {
  presentAction,
  presentCanonicalValue,
  presentContradictionOutcome,
  presentDistortionIndicator,
  presentFraction,
  presentStatus,
} from "./format";

describe("presentStatus", () => {
  it("pairs every status with a distinct glyph and a text label (never color-only)", () => {
    const included = presentStatus("included");
    const partial = presentStatus("partial");
    const dropped = presentStatus("dropped");

    for (const presentation of [included, partial, dropped]) {
      expect(presentation.glyph.length).toBeGreaterThan(0);
      expect(presentation.label.length).toBeGreaterThan(0);
      expect(presentation.cssClass.length).toBeGreaterThan(0);
    }
    const glyphs = new Set([included.glyph, partial.glyph, dropped.glyph]);
    const labels = new Set([included.label, partial.label, dropped.label]);
    expect(glyphs.size).toBe(3);
    expect(labels.size).toBe(3);
  });
});

describe("presentAction", () => {
  it("maps every known trace action code to its exact human-readable label", () => {
    expect(presentAction("included_full")).toBe("Included (full)");
    expect(presentAction("dropped_over_budget")).toBe("Dropped (over budget)");
    expect(presentAction("dropped_too_old")).toBe("Dropped (too old)");
    expect(presentAction("extracted_included")).toBe("Extracted (included)");
    expect(presentAction("templated_included")).toBe("Templated (included)");
    expect(presentAction("dropped_below_budget")).toBe("Dropped (below budget)");
  });

  it("degrades gracefully (title-cased, spaced) for an unknown code", () => {
    expect(presentAction("some_future_action")).toBe("Some future action");
  });
});

describe("presentContradictionOutcome", () => {
  it("maps every outcome to distinct, non-empty text", () => {
    const outcomes = ["all_retained", "split", "partial_within_retained", "none_retained"] as const;
    const labels = outcomes.map(presentContradictionOutcome);
    expect(new Set(labels).size).toBe(4);
    for (const label of labels) expect(label.length).toBeGreaterThan(0);
  });
});

describe("presentDistortionIndicator", () => {
  it("names the affected fact for evidence_clipped", () => {
    expect(presentDistortionIndicator({ evidence_clipped: { fact_id: "f1" } })).toContain('"f1"');
  });

  it("names both the qualifier and the fact for qualifier_dropped", () => {
    const text = presentDistortionIndicator({
      qualifier_dropped: { fact_id: "f1", qualifier: "not" },
    });
    expect(text).toContain('"not"');
    expect(text).toContain('"f1"');
  });

  it("names the source for provenance_reordered", () => {
    expect(presentDistortionIndicator({ provenance_reordered: { source_id: "src-1" } })).toContain(
      '"src-1"',
    );
  });

  it("names the group for contradiction_split", () => {
    expect(presentDistortionIndicator({ contradiction_split: { group_id: "g1" } })).toContain(
      '"g1"',
    );
  });

  it("names the source for distractor_retained", () => {
    expect(presentDistortionIndicator({ distractor_retained: { source_id: "noise-1" } })).toContain(
      '"noise-1"',
    );
  });
});

describe("presentFraction", () => {
  it("always shows both numerator and denominator, never a bare number", () => {
    expect(presentFraction(3, 10)).toBe("3 / 10");
    expect(presentFraction(0, 0)).toBe("0 / 0");
  });
});

describe("presentCanonicalValue", () => {
  it("includes the unit for a number with a unit", () => {
    expect(presentCanonicalValue({ number: { normalized: "350", unit: "ms" } })).toBe("350 ms");
  });

  it("omits the unit when absent", () => {
    expect(presentCanonicalValue({ number: { normalized: "42", unit: null } })).toBe("42");
  });

  it("renders a date's normalized form", () => {
    expect(presentCanonicalValue({ date: { normalized: "2024-01-05" } })).toBe("2024-01-05");
  });

  it("renders free text's normalized form", () => {
    expect(presentCanonicalValue({ text: { normalized: "deploy" } })).toBe("deploy");
  });
});
