import { el } from "../dom";
import { presentContradictionOutcome } from "../format";
import type { ContradictionResult } from "../types";

const OUTCOME_CSS_CLASS: Record<ContradictionResult["outcome"], string> = {
  all_retained: "status-included",
  split: "status-partial",
  partial_within_retained: "status-partial",
  none_retained: "status-dropped",
};

/**
 * Explicit contradiction call-outs: one entry per contradiction group, showing the purely
 * structural outcome (`all_retained` / `split` / `partial_within_retained` / `none_retained`),
 * specifically flagging `split` as "split across the budget boundary" -- meaning at least one
 * dropped member of the group was dropped because the budget ran out, while another member
 * survived (never implying the two sit at adjacent positions or a single geometric cutoff point;
 * `retrieval-ranking@1`'s non-contiguous bin-packing can produce `split` with no such adjacency
 * at all). Never a truth judgement about which competing claim was correct.
 */
export function createContradictionsSection(
  contradictions: ContradictionResult[],
): HTMLElement | null {
  if (contradictions.length === 0) return null;

  const entries = contradictions.map((result) => {
    const cssClass = OUTCOME_CSS_CLASS[result.outcome];
    const label = presentContradictionOutcome(result.outcome);
    return el("li", { class: "contradiction-item" }, [
      el("span", { class: "contradiction-group-id" }, [result.group_id]),
      el("span", { class: `badge ${cssClass}` }, [label]),
      result.outcome === "split"
        ? el("p", { class: "contradiction-note" }, [
            "At least one competing claim in this group was dropped because the budget ran out, while another survived selection -- the outcome was genuinely decided by the budget cutoff, not by which claim was correct.",
          ])
        : null,
    ]);
  });

  return el("div", { class: "contradictions-section" }, [
    el("h3", { class: "subsection-title" }, ["Contradiction groups"]),
    el(
      "ol",
      { class: "contradictions-list", attrs: { "aria-label": "Contradiction group outcomes" } },
      entries,
    ),
  ]);
}
