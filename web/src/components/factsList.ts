import { el } from "../dom";
import { presentFraction } from "../format";
import type { FactRecall, ScenarioAnnotations } from "../types";

/**
 * The required-facts list: each fact's `statement` and `why_it_matters` alongside its
 * retained/not-retained state (with any missing components named), for teaching purposes. This
 * is evaluator-side annotation context, legitimately shown to the human user (see
 * `crates/limen-wasm/src/lib.rs`'s `get_scenario_detail` doc comment); it is never shown to a
 * `SelectionStrategy` itself.
 */
export function createFactsList(
  annotations: ScenarioAnnotations,
  factRecall: FactRecall,
): HTMLElement {
  const detailByFactId = new Map(factRecall.per_fact.map((detail) => [detail.fact_id, detail]));

  const entries = annotations.required_facts.map((fact) => {
    const detail = detailByFactId.get(fact.fact_id);
    const retained = detail?.retained ?? false;
    const statusClass = retained ? "status-included" : "status-dropped";
    const statusGlyph = retained ? "\u2713" : "\u2715";
    const statusLabel = retained ? "Retained" : "Not retained";

    return el("li", { class: "fact-item" }, [
      el("div", { class: "fact-item-header" }, [
        el("span", { class: `badge ${statusClass}` }, [
          el("span", { attrs: { "aria-hidden": "true" } }, [statusGlyph]),
          ` ${statusLabel}`,
        ]),
        el("span", { class: "fact-statement" }, [fact.statement]),
      ]),
      el("p", { class: "fact-why-it-matters" }, [
        el("span", { class: "field-label" }, ["Why it matters: "]),
        fact.why_it_matters,
      ]),
      detail && detail.missing_components.length > 0
        ? el("p", { class: "fact-missing-components" }, [
            el("span", { class: "field-label" }, ["Missing components: "]),
            detail.missing_components.join(", "),
          ])
        : null,
    ]);
  });

  return el("div", { class: "facts-section" }, [
    el("h3", { class: "subsection-title" }, [
      `Required facts (${presentFraction(factRecall.retained, factRecall.required)} retained)`,
    ]),
    el("ol", { class: "facts-list", attrs: { "aria-label": "Required facts" } }, entries),
  ]);
}
