// TypeScript mirrors of the exact JSON shapes `limen-core` serializes (see
// `crates/limen-core/src/model.rs`, `metrics.rs`, `result.rs`, `error.rs`, and `catalog.rs`/
// `strategy/mod.rs`'s display-metadata additions). This file defines *shapes only*: no
// tokenization, selection, validation, metrics, or digesting logic is ever duplicated here. The
// Rust engine is the only place any of that is computed; this file just lets TypeScript describe
// what comes back over the WASM boundary.
//
// Every `Option<T>` field on the Rust side serializes as an explicit JSON `null` (serde's default,
// never an omitted key), so the corresponding field below is typed `T | null`, never optional.

export type SelectionStatus = "included" | "partial" | "dropped";

export interface ScenarioSummary {
  scenario_id: string;
  scenario_version: string;
  title: string;
  task_query: string;
  item_count: number;
}

export interface StrategyDescriptor {
  strategy_id: string;
  label: string;
  summary: string;
}

export interface ContextItem {
  source_id: string;
  order_index: number;
  section_label: string;
  text: string;
}

export interface EvidenceSpan {
  source_id: string;
  byte_start: number;
  byte_end: number;
}

export type CanonicalValue =
  | { number: { normalized: string; unit: string | null } }
  | { date: { normalized: string } }
  | { text: { normalized: string } };

export interface FactComponent {
  component_id: string;
  evidence: EvidenceSpan[];
  canonical_value: CanonicalValue | null;
  required_qualifiers: string[];
}

export interface ExpectedFact {
  fact_id: string;
  statement: string;
  why_it_matters: string;
  components: FactComponent[];
  expected_citation_source_ids: string[];
}

export interface ContradictionGroup {
  group_id: string;
  members: EvidenceSpan[];
}

export interface ScenarioAnnotations {
  required_facts: ExpectedFact[];
  distractor_source_ids: string[];
  contradiction_groups: ContradictionGroup[];
}

export interface ScenarioManifest {
  schema_version: string;
  scenario_id: string;
  scenario_version: string;
  title: string;
  task_query: string;
  items: ContextItem[];
  annotations: ScenarioAnnotations;
  content_digest: string;
}

export interface ItemSelectionRecord {
  source_id: string;
  order_index: number;
  status: SelectionStatus;
  included_byte_start: number | null;
  included_byte_end: number | null;
  output_text: string | null;
}

export interface TraceStep {
  step_index: number;
  source_id: string;
  action: string;
  score: number | null;
  detail: string;
}

export interface BudgetUsage {
  requested_tokens: number;
  used_tokens: number;
  remaining_tokens: number;
}

export interface SelectionOutput {
  strategy_id: string;
  budget: BudgetUsage;
  selection: ItemSelectionRecord[];
  trace: TraceStep[];
}

export interface FactRecallDetail {
  fact_id: string;
  retained: boolean;
  missing_components: string[];
}

export interface FactRecall {
  retained: number;
  required: number;
  per_fact: FactRecallDetail[];
}

export type ContradictionOutcome =
  | "all_retained"
  | "split"
  | "partial_within_retained"
  | "none_retained";

export interface ContradictionResult {
  group_id: string;
  outcome: ContradictionOutcome;
}

export interface NumericDateAccuracy {
  exact: number;
  checked: number;
  mismatches: string[];
}

export interface CitationRetention {
  retained: number;
  expected: number;
  per_fact: Array<[string, boolean]>;
}

export type DistortionIndicator =
  | { evidence_clipped: { fact_id: string } }
  | { qualifier_dropped: { fact_id: string; qualifier: string } }
  | { provenance_reordered: { source_id: string } }
  | { contradiction_split: { group_id: string } }
  | { distractor_retained: { source_id: string } };

export interface Metrics {
  fact_recall: FactRecall;
  contradictions: ContradictionResult[];
  numeric_date_accuracy: NumericDateAccuracy;
  citation_retention: CitationRetention;
  budget: BudgetUsage;
  distortion_indicators: DistortionIndicator[];
}

export interface TrialResult {
  schema_version: string;
  engine_version: string;
  tokenizer_id: string;
  scenario_id: string;
  scenario_version: string;
  scenario_content_digest: string;
  strategy_id: string;
  selection: SelectionOutput;
  metrics: Metrics;
  result_digest: string;
}

/** Mirrors `EngineError`'s `#[serde(tag = "error", content = "detail")]` representation. */
export type EngineErrorPayload =
  | { error: "unknown_strategy_version"; detail: string }
  | { error: "unknown_scenario_id"; detail: string }
  | { error: "budget_below_minimum"; detail: null }
  | { error: "validation_failed"; detail: unknown[] }
  | { error: "canonicalization_failed"; detail: string }
  | {
      error: "used_tokens_exceed_requested";
      detail: { requested_tokens: number; used_tokens: number };
    };

/** A trial, including both the parsed view (for rendering) and the untouched raw string the
 * engine returned (the only thing ever written to a download). */
export interface TrialOutcome {
  raw: string;
  parsed: TrialResult;
}

export type EngineResult<T> = { ok: true; value: T } | { ok: false; error: EngineErrorPayload };
