import { el } from "../dom";
import { presentDistortionIndicator, presentFraction } from "../format";
import type { Metrics } from "../types";

/**
 * The metrics summary: fact recall (retained/required, denominator always visible), numeric/date
 * accuracy, citation retention, budget used/remaining, and the distortion indicator list. A
 * native `<dl>` of label/value pairs: every value is the exact figure the engine computed, never
 * a re-derived approximation.
 */
export function createMetricsSummary(metrics: Metrics): HTMLElement {
  const { fact_recall, numeric_date_accuracy, citation_retention, budget, distortion_indicators } =
    metrics;

  const dl = el("dl", { class: "metrics-summary" }, [
    el("dt", {}, ["Fact recall"]),
    el("dd", {}, [
      presentFraction(fact_recall.retained, fact_recall.required),
      " required facts retained",
    ]),

    el("dt", {}, ["Numeric / date accuracy"]),
    el("dd", {}, [
      presentFraction(numeric_date_accuracy.exact, numeric_date_accuracy.checked),
      " checked values exactly retained",
      numeric_date_accuracy.mismatches.length > 0
        ? el("span", { class: "metrics-note" }, [
            ` (not exactly retained -- omitted, or present but altered/incomplete: ${numeric_date_accuracy.mismatches.join(", ")})`,
          ])
        : null,
    ]),

    el("dt", {}, ["Citation retention"]),
    el("dd", {}, [
      presentFraction(citation_retention.retained, citation_retention.expected),
      " facts with all expected citations retained",
    ]),

    el("dt", {}, ["Budget"]),
    el("dd", {}, [
      `${budget.used_tokens} / ${budget.requested_tokens} tokens used, ${budget.remaining_tokens} remaining`,
    ]),
  ]);

  const distortionSection =
    distortion_indicators.length > 0
      ? el("div", { class: "distortion-section" }, [
          el("h4", { class: "subsection-title" }, [
            `Distortion indicators (${distortion_indicators.length})`,
          ]),
          el(
            "ul",
            { class: "distortion-list" },
            distortion_indicators.map((indicator) =>
              el("li", {}, [presentDistortionIndicator(indicator)]),
            ),
          ),
        ])
      : el("p", { class: "distortion-none" }, ["No distortion indicators for this trial."]);

  return el("div", { class: "metrics-section" }, [
    el("h3", { class: "subsection-title" }, ["Metrics"]),
    dl,
    distortionSection,
  ]);
}
