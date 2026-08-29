import { el } from "../dom";
import type { Metrics } from "../types";

interface ComparisonRow {
  label: string;
  valueA: string;
  valueB: string;
  delta: string;
}

function signedDelta(a: number, b: number): string {
  const delta = a - b;
  if (delta === 0) return "0";
  return delta > 0 ? `+${delta}` : `${delta}`;
}

function buildRows(metricsA: Metrics, metricsB: Metrics): ComparisonRow[] {
  return [
    {
      label: "Fact recall (retained / required)",
      valueA: `${metricsA.fact_recall.retained} / ${metricsA.fact_recall.required}`,
      valueB: `${metricsB.fact_recall.retained} / ${metricsB.fact_recall.required}`,
      delta: signedDelta(metricsA.fact_recall.retained, metricsB.fact_recall.retained),
    },
    {
      label: "Numeric / date accuracy (exact / checked)",
      valueA: `${metricsA.numeric_date_accuracy.exact} / ${metricsA.numeric_date_accuracy.checked}`,
      valueB: `${metricsB.numeric_date_accuracy.exact} / ${metricsB.numeric_date_accuracy.checked}`,
      delta: signedDelta(
        metricsA.numeric_date_accuracy.exact,
        metricsB.numeric_date_accuracy.exact,
      ),
    },
    {
      label: "Citation retention (retained / expected)",
      valueA: `${metricsA.citation_retention.retained} / ${metricsA.citation_retention.expected}`,
      valueB: `${metricsB.citation_retention.retained} / ${metricsB.citation_retention.expected}`,
      delta: signedDelta(
        metricsA.citation_retention.retained,
        metricsB.citation_retention.retained,
      ),
    },
    {
      label: "Budget used (tokens)",
      valueA: `${metricsA.budget.used_tokens}`,
      valueB: `${metricsB.budget.used_tokens}`,
      delta: signedDelta(metricsA.budget.used_tokens, metricsB.budget.used_tokens),
    },
    {
      label: "Budget remaining (tokens)",
      valueA: `${metricsA.budget.remaining_tokens}`,
      valueB: `${metricsB.budget.remaining_tokens}`,
      delta: signedDelta(metricsA.budget.remaining_tokens, metricsB.budget.remaining_tokens),
    },
    {
      label: "Distortion indicators (count)",
      valueA: `${metricsA.distortion_indicators.length}`,
      valueB: `${metricsB.distortion_indicators.length}`,
      delta: signedDelta(
        metricsA.distortion_indicators.length,
        metricsB.distortion_indicators.length,
      ),
    },
  ];
}

/**
 * The delta/comparison view between columns A and B: a real `<table>`, rows are metrics, columns
 * are Strategy A / Strategy B, and a fourth column shows the difference. Serves "metric changes
 * between strategies" directly.
 */
export function createComparisonTable(
  labelA: string,
  metricsA: Metrics,
  labelB: string,
  metricsB: Metrics,
): HTMLElement {
  const rows = buildRows(metricsA, metricsB);

  return el("table", { class: "comparison-table" }, [
    el("caption", {}, ["Metric changes between strategy A and strategy B"]),
    el("thead", {}, [
      el("tr", {}, [
        el("th", { attrs: { scope: "col" } }, ["Metric"]),
        el("th", { attrs: { scope: "col" } }, [labelA]),
        el("th", { attrs: { scope: "col" } }, [labelB]),
        el("th", { attrs: { scope: "col" } }, ["Difference (A minus B)"]),
      ]),
    ]),
    el(
      "tbody",
      {},
      rows.map((row) =>
        el("tr", {}, [
          el("th", { attrs: { scope: "row" } }, [row.label]),
          el("td", { class: "num-cell" }, [row.valueA]),
          el("td", { class: "num-cell" }, [row.valueB]),
          el("td", { class: "num-cell delta-cell" }, [row.delta]),
        ]),
      ),
    ),
  ]);
}
