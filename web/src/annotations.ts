// Pure lookup-index helpers built from data the engine already returned (a `ScenarioManifest`'s
// `annotations`). These only aggregate/index existing engine output for rendering (e.g. "which
// contradiction groups mention this source_id"); they never decide selection, scoring, or
// retention themselves, so they do not duplicate `limen-core`'s logic.
import type { ScenarioAnnotations } from "./types";

/** Maps `source_id` to the list of contradiction `group_id`s it is a member of. */
export function contradictionGroupsBySourceId(
  annotations: ScenarioAnnotations,
): Map<string, string[]> {
  const map = new Map<string, string[]>();
  for (const group of annotations.contradiction_groups) {
    for (const member of group.members) {
      const list = map.get(member.source_id) ?? [];
      list.push(group.group_id);
      map.set(member.source_id, list);
    }
  }
  return map;
}

/** Maps `source_id` to the list of required-fact ids that either cite it
 * (`expected_citation_source_ids`) or evidence it (any component's `evidence`). */
export function requiredFactsBySourceId(annotations: ScenarioAnnotations): Map<string, string[]> {
  const map = new Map<string, string[]>();
  for (const fact of annotations.required_facts) {
    const sourceIds = new Set<string>(fact.expected_citation_source_ids);
    for (const component of fact.components) {
      for (const evidence of component.evidence) sourceIds.add(evidence.source_id);
    }
    for (const sourceId of sourceIds) {
      const list = map.get(sourceId) ?? [];
      list.push(fact.fact_id);
      map.set(sourceId, list);
    }
  }
  return map;
}

export function isDistractor(annotations: ScenarioAnnotations, sourceId: string): boolean {
  return annotations.distractor_source_ids.includes(sourceId);
}
