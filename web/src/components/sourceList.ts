import {
  contradictionGroupsBySourceId,
  isDistractor,
  requiredFactsBySourceId,
} from "../annotations";
import { sliceByUtf8ByteRange } from "../bytes";
import { el } from "../dom";
import { presentStatus } from "../format";
import type { ContextItem, ItemSelectionRecord, ScenarioAnnotations } from "../types";

function renderSourceText(item: ContextItem, record: ItemSelectionRecord): HTMLElement {
  if (record.status === "dropped") {
    return el("p", { class: "source-text source-text-dropped" }, [item.text]);
  }
  if (record.status === "included") {
    return el("p", { class: "source-text" }, [item.text]);
  }
  // Partial: distinguish the retained provenance range from the rest of the original text.
  const start = record.included_byte_start ?? 0;
  const end = record.included_byte_end ?? 0;
  const before = sliceByUtf8ByteRange(item.text, 0, start);
  const retained = sliceByUtf8ByteRange(item.text, start, end);
  const after = sliceByUtf8ByteRange(item.text, end, Number.MAX_SAFE_INTEGER);
  return el("p", { class: "source-text" }, [
    before ? el("span", { class: "source-text-clipped" }, [before]) : null,
    el("mark", { class: "source-text-retained" }, [retained]),
    after ? el("span", { class: "source-text-clipped" }, [after]) : null,
  ]);
}

/**
 * The source item list: one entry per item in `order_index` order, showing `section_label`,
 * status, and (for transform strategies) the `output_text` actually produced, plus
 * evaluator-side context for teaching purposes: distractor membership and contradiction-group
 * membership are marked visibly, and cross-references to required facts this item evidences or
 * is cited by are listed.
 */
export function createSourceList(
  items: ContextItem[],
  selection: ItemSelectionRecord[],
  annotations: ScenarioAnnotations,
): HTMLElement {
  const recordsBySourceId = new Map(selection.map((r) => [r.source_id, r]));
  const groupsBySourceId = contradictionGroupsBySourceId(annotations);
  const factsBySourceId = requiredFactsBySourceId(annotations);
  const sortedItems = [...items].sort((a, b) => a.order_index - b.order_index);

  const entries = sortedItems.map((item) => {
    const record = recordsBySourceId.get(item.source_id);
    if (!record) return null;
    const presentation = presentStatus(record.status);
    const badges: (HTMLElement | null)[] = [
      el("span", { class: `badge ${presentation.cssClass}` }, [
        el("span", { attrs: { "aria-hidden": "true" } }, [presentation.glyph]),
        ` ${presentation.label}`,
      ]),
    ];

    if (isDistractor(annotations, item.source_id)) {
      badges.push(el("span", { class: "badge badge-distractor" }, ["Distractor"]));
    }
    for (const groupId of groupsBySourceId.get(item.source_id) ?? []) {
      badges.push(
        el("span", { class: "badge badge-contradiction" }, [`Contradiction: ${groupId}`]),
      );
    }

    const factRefs = factsBySourceId.get(item.source_id) ?? [];

    return el("li", { class: "source-item" }, [
      el("div", { class: "source-item-header" }, [
        el("span", { class: "source-item-position" }, [`#${item.order_index}`]),
        el("span", { class: "source-item-label" }, [item.section_label]),
        el("span", { class: "source-item-id" }, [item.source_id]),
        el("div", { class: "badge-row" }, badges),
      ]),
      renderSourceText(item, record),
      record.output_text
        ? el("div", { class: "source-output" }, [
            el("span", { class: "field-label" }, ["Output text produced:"]),
            el("p", { class: "source-text source-output-text" }, [record.output_text]),
          ])
        : null,
      factRefs.length > 0
        ? el("p", { class: "source-fact-refs" }, [
            el("span", { class: "field-label" }, ["Evidence/citation for: "]),
            factRefs.join(", "),
          ])
        : null,
    ]);
  });

  return el(
    "ol",
    { class: "source-list", attrs: { "aria-label": "Source items, in original order" } },
    entries,
  );
}
