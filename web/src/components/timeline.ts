import { utf8ByteLength } from "../bytes";
import { el } from "../dom";
import { presentStatus } from "../format";
import type { ContextItem, ItemSelectionRecord } from "../types";

/**
 * The order_index of the last retained (`included`/`partial`) item, provided at least one item
 * with a *higher* order_index was dropped (i.e. there is a genuine trailing run of drops after
 * it). Returns `null` when nothing was dropped after the furthest-reaching retained item (an
 * ample budget, or a bin-packing strategy whose furthest-reaching retained item happens to be the
 * scenario's last item) -- in both cases there is no honest "everything after this point was cut"
 * boundary to draw. Computed purely from `order_index`/`status` fields the engine already
 * returned; this is aggregation, not a re-derivation of any strategy's own cutoff logic.
 */
export function computeBudgetBoundaryOrderIndex(selection: ItemSelectionRecord[]): number | null {
  const retained = selection.filter((r) => r.status !== "dropped").map((r) => r.order_index);
  const dropped = selection.filter((r) => r.status === "dropped").map((r) => r.order_index);
  if (retained.length === 0 || dropped.length === 0) return null;
  const lastRetained = Math.max(...retained);
  return dropped.some((idx) => idx > lastRetained) ? lastRetained : null;
}

/**
 * A budget timeline: a semantic `<ol>`, one `<li>` per source item in `order_index` order, sized
 * proportionally to that item's own UTF-8 byte length (a visual proxy for its token share; real
 * token counts are never recomputed in TypeScript, only the Rust tokenizer ever counts tokens),
 * with a distinct boundary marker where the budget was exhausted. Every status is conveyed by a
 * glyph and a word before color.
 */
export function createTimeline(
  items: ContextItem[],
  selection: ItemSelectionRecord[],
): HTMLElement {
  const itemsByOrderIndex = new Map(items.map((item) => [item.order_index, item]));
  const recordsByOrderIndex = new Map(selection.map((record) => [record.order_index, record]));
  const orderIndices = [...itemsByOrderIndex.keys()].sort((a, b) => a - b);

  const weights = new Map(
    orderIndices.map((idx): [number, number] => {
      const item = itemsByOrderIndex.get(idx);
      return [idx, item ? Math.max(1, utf8ByteLength(item.text)) : 1];
    }),
  );
  const totalWeight = [...weights.values()].reduce((sum, w) => sum + w, 0) || 1;
  const boundary = computeBudgetBoundaryOrderIndex(selection);

  const listItems = orderIndices.flatMap((idx) => {
    const item = itemsByOrderIndex.get(idx);
    const record = recordsByOrderIndex.get(idx);
    if (!item || !record) return [];

    const presentation = presentStatus(record.status);
    const weightPercent = ((weights.get(idx) ?? 1) / totalWeight) * 100;

    const segment = el(
      "li",
      {
        class: `timeline-item ${presentation.cssClass}`,
        attrs: {
          title: `${item.section_label} #${item.order_index} (${item.source_id}): ${presentation.label}`,
        },
      },
      [
        el("span", { class: "timeline-item-glyph", attrs: { "aria-hidden": "true" } }, [
          presentation.glyph,
        ]),
        el("span", { class: "visually-hidden" }, [`${item.source_id}, ${item.section_label}, `]),
        el("span", { class: "timeline-item-label" }, [presentation.label]),
      ],
    );
    segment.style.setProperty("--timeline-weight", `${weightPercent}%`);

    if (boundary === idx) {
      const marker = el("li", { class: "timeline-boundary", attrs: { role: "presentation" } }, [
        el("span", { class: "timeline-boundary-marker", attrs: { "aria-hidden": "true" } }, []),
        el("span", { class: "visually-hidden" }, [
          "Budget boundary: everything after this point was dropped.",
        ]),
      ]);
      return [segment, marker];
    }
    return [segment];
  });

  const list = el(
    "ol",
    {
      class: "timeline",
      attrs: { "aria-label": "Budget timeline, one entry per source item in original order" },
    },
    listItems,
  );

  return el("div", { class: "timeline-wrapper" }, [list]);
}
