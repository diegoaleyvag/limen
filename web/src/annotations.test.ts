import { describe, expect, it } from "vitest";
import {
  contradictionGroupsBySourceId,
  isDistractor,
  requiredFactsBySourceId,
} from "./annotations";
import type { ScenarioAnnotations } from "./types";

function annotations(): ScenarioAnnotations {
  return {
    required_facts: [
      {
        fact_id: "f1",
        statement: "s",
        why_it_matters: "w",
        expected_citation_source_ids: ["log-1"],
        components: [
          {
            component_id: "c1",
            evidence: [{ source_id: "log-2", byte_start: 0, byte_end: 3 }],
            canonical_value: null,
            required_qualifiers: [],
          },
        ],
      },
    ],
    distractor_source_ids: ["note-1"],
    contradiction_groups: [
      {
        group_id: "g1",
        members: [
          { source_id: "chat-1", byte_start: 0, byte_end: 5 },
          { source_id: "chat-2", byte_start: 0, byte_end: 5 },
        ],
      },
    ],
  };
}

describe("contradictionGroupsBySourceId", () => {
  it("maps each member source_id to the group(s) it belongs to", () => {
    const map = contradictionGroupsBySourceId(annotations());
    expect(map.get("chat-1")).toEqual(["g1"]);
    expect(map.get("chat-2")).toEqual(["g1"]);
    expect(map.get("log-1")).toBeUndefined();
  });
});

describe("requiredFactsBySourceId", () => {
  it("maps a citation source and an evidence source to the same fact_id", () => {
    const map = requiredFactsBySourceId(annotations());
    expect(map.get("log-1")).toEqual(["f1"]);
    expect(map.get("log-2")).toEqual(["f1"]);
  });

  it("returns undefined for a source not referenced by any fact", () => {
    const map = requiredFactsBySourceId(annotations());
    expect(map.get("note-1")).toBeUndefined();
  });

  it("never duplicates a fact_id when a source is both cited and evidenced", () => {
    const both: ScenarioAnnotations = {
      required_facts: [
        {
          fact_id: "f2",
          statement: "s",
          why_it_matters: "w",
          expected_citation_source_ids: ["dual"],
          components: [
            {
              component_id: "c1",
              evidence: [{ source_id: "dual", byte_start: 0, byte_end: 1 }],
              canonical_value: null,
              required_qualifiers: [],
            },
          ],
        },
      ],
      distractor_source_ids: [],
      contradiction_groups: [],
    };
    expect(requiredFactsBySourceId(both).get("dual")).toEqual(["f2"]);
  });
});

describe("isDistractor", () => {
  it("returns true only for a listed distractor source_id", () => {
    const data = annotations();
    expect(isDistractor(data, "note-1")).toBe(true);
    expect(isDistractor(data, "log-1")).toBe(false);
  });
});
