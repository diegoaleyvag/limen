// Presentation-only formatting helpers: mapping the engine's stable string codes (statuses, trace
// action codes, contradiction outcomes, distortion indicator variants) to a short glyph plus a
// human-readable word or sentence. Every mapping pairs an icon/glyph with text; nothing here ever
// conveys a status through color alone (color is applied by CSS classes derived from these same
// codes, layered underneath the glyph+text, never replacing it).
import type { ContradictionOutcome, DistortionIndicator, SelectionStatus } from "./types";

export interface StatusPresentation {
  glyph: string;
  label: string;
  cssClass: string;
}

const STATUS_PRESENTATION: Record<SelectionStatus, StatusPresentation> = {
  included: { glyph: "\u2713", label: "Included", cssClass: "status-included" },
  partial: { glyph: "\u25D1", label: "Partial", cssClass: "status-partial" },
  dropped: { glyph: "\u2715", label: "Dropped", cssClass: "status-dropped" },
};

export function presentStatus(status: SelectionStatus): StatusPresentation {
  return STATUS_PRESENTATION[status];
}

const ACTION_LABELS: Record<string, string> = {
  included_full: "Included (full)",
  included_partial: "Included (partial)",
  dropped_over_budget: "Dropped (over budget)",
  dropped_too_old: "Dropped (too old)",
  extracted_included: "Extracted (included)",
  extracted_partial: "Extracted (partial)",
  dropped_no_extractable_content: "Dropped (no extractable content)",
  templated_included: "Templated (included)",
  dropped_below_budget: "Dropped (below budget)",
};

/** Humanizes a trace `action` code. Falls back to a generic underscore-to-space conversion for
 * any code not in the fixed table above, so a future strategy addition degrades gracefully
 * instead of rendering a raw `snake_case` token. */
export function presentAction(action: string): string {
  const known = ACTION_LABELS[action];
  if (known) return known;
  const spaced = action.replace(/_/g, " ");
  return spaced.charAt(0).toUpperCase() + spaced.slice(1);
}

const CONTRADICTION_OUTCOME_LABELS: Record<ContradictionOutcome, string> = {
  all_retained: "All retained",
  split: "Split across the budget boundary",
  partial_within_retained: "Partial (within retained)",
  none_retained: "None retained",
};

export function presentContradictionOutcome(outcome: ContradictionOutcome): string {
  return CONTRADICTION_OUTCOME_LABELS[outcome];
}

/** One line of human-readable text per distortion indicator, naming the affected fact/source/group
 * explicitly (never just "something was distorted"). */
export function presentDistortionIndicator(indicator: DistortionIndicator): string {
  if ("evidence_clipped" in indicator) {
    return `Evidence clipped for required fact "${indicator.evidence_clipped.fact_id}".`;
  }
  if ("qualifier_dropped" in indicator) {
    return `Qualifier "${indicator.qualifier_dropped.qualifier}" dropped from required fact "${indicator.qualifier_dropped.fact_id}".`;
  }
  if ("provenance_reordered" in indicator) {
    return `Provenance reordered for source "${indicator.provenance_reordered.source_id}".`;
  }
  if ("contradiction_split" in indicator) {
    return `Contradiction group "${indicator.contradiction_split.group_id}" split across the budget boundary.`;
  }
  return `Distractor source "${indicator.distractor_retained.source_id}" retained.`;
}

/** Formats a fraction as "retained / required" with the denominator always visible, per the
 * product's non-negotiable "every number carries its denominator" principle. */
export function presentFraction(numerator: number, denominator: number): string {
  return `${numerator} / ${denominator}`;
}

export function presentCanonicalValue(
  value:
    | { number: { normalized: string; unit: string | null } }
    | { date: { normalized: string } }
    | { text: { normalized: string } },
): string {
  if ("number" in value) {
    return value.number.unit
      ? `${value.number.normalized} ${value.number.unit}`
      : value.number.normalized;
  }
  if ("date" in value) {
    return value.date.normalized;
  }
  return value.text.normalized;
}
