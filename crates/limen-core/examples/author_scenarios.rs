//! Authors the three real scenario datasets under `scenarios/v1/` as Rust data, then writes them
//! out as pretty-printed JSON.
//!
//! Run from anywhere with: `cargo run -p limen-core --example author_scenarios`
//!
//! Every [`crate::model::EvidenceSpan`] below is derived programmatically from the source item's
//! own `text` via [`str::find`] (see the [`evidence`] helper) rather than hand-counted, so a typo
//! in prose text can never silently desynchronize from its byte offsets. `content_digest` is
//! computed via [`canonical::digest_with_field_blanked`], the same helper `validate_manifest`
//! uses to check it, so the freshly written files always validate cleanly. See
//! `docs/SCENARIO_AUTHORING.md` for the full authoring methodology this file follows.
//!
//! All scenario content (names, companies, products, incidents, requirements) is invented and
//! synthetic; see the content-safety note in `docs/SCENARIO_AUTHORING.md`.

use std::collections::BTreeSet;
use std::path::PathBuf;

use limen_core::canonical;
use limen_core::model::{
    CanonicalValue, ContextItem, ContradictionGroup, EvidenceSpan, ExpectedFact, FactComponent,
    ScenarioAnnotations, ScenarioManifest,
};

// ---------------------------------------------------------------------------------------------
// Small construction helpers shared by all three scenarios.
// ---------------------------------------------------------------------------------------------

/// Builds one [`ContextItem`].
fn item(source_id: &str, order_index: u32, section_label: &str, text: &str) -> ContextItem {
    ContextItem {
        source_id: source_id.to_string(),
        order_index,
        section_label: section_label.to_string(),
        text: text.to_string(),
    }
}

/// Finds `needle` inside the `text` of the item identified by `source_id` and returns the
/// corresponding [`EvidenceSpan`]. Panics with a precise, debuggable message if the source_id is
/// unknown or the needle is not present -- this is the one place a typo in prose text or an
/// evidence needle would surface, at `cargo run` time, rather than as a silently-wrong byte span.
fn evidence(items: &[ContextItem], source_id: &str, needle: &str) -> EvidenceSpan {
    let found = items
        .iter()
        .find(|candidate| candidate.source_id == source_id);
    let Some(source_item) = found else {
        panic!("author_scenarios: no item with source_id {source_id:?}");
    };
    let Some(start) = source_item.text.find(needle) else {
        panic!(
            "author_scenarios: needle {needle:?} not found in source_id {source_id:?} (text: {:?})",
            source_item.text
        );
    };
    EvidenceSpan {
        source_id: source_id.to_string(),
        byte_start: start as u32,
        byte_end: (start + needle.len()) as u32,
    }
}

fn number(normalized: &str, unit: Option<&str>) -> CanonicalValue {
    CanonicalValue::Number {
        normalized: normalized.to_string(),
        unit: unit.map(|s| s.to_string()),
    }
}

fn date(normalized: &str) -> CanonicalValue {
    CanonicalValue::Date {
        normalized: normalized.to_string(),
    }
}

fn text_value(normalized: &str) -> CanonicalValue {
    CanonicalValue::Text {
        normalized: normalized.to_string(),
    }
}

fn component(
    component_id: &str,
    evidence: Vec<EvidenceSpan>,
    canonical_value: Option<CanonicalValue>,
    required_qualifiers: &[&str],
) -> FactComponent {
    FactComponent {
        component_id: component_id.to_string(),
        evidence,
        canonical_value,
        required_qualifiers: required_qualifiers.iter().map(|s| s.to_string()).collect(),
    }
}

fn fact(
    fact_id: &str,
    statement: &str,
    why_it_matters: &str,
    components: Vec<FactComponent>,
    expected_citation_source_ids: &[&str],
) -> ExpectedFact {
    ExpectedFact {
        fact_id: fact_id.to_string(),
        statement: statement.to_string(),
        why_it_matters: why_it_matters.to_string(),
        components,
        expected_citation_source_ids: expected_citation_source_ids
            .iter()
            .map(|s| s.to_string())
            .collect(),
    }
}

fn group(group_id: &str, members: Vec<EvidenceSpan>) -> ContradictionGroup {
    ContradictionGroup {
        group_id: group_id.to_string(),
        members,
    }
}

fn distractors(source_ids: &[&str]) -> BTreeSet<String> {
    source_ids.iter().map(|s| s.to_string()).collect()
}

/// Assembles the final [`ScenarioManifest`] and fills in a freshly computed `content_digest`, so
/// every manifest this file produces always passes `validate_manifest`'s digest-freshness check.
fn finalize(
    scenario_id: &str,
    title: &str,
    task_query: &str,
    items: Vec<ContextItem>,
    required_facts: Vec<ExpectedFact>,
    distractor_source_ids: BTreeSet<String>,
    contradiction_groups: Vec<ContradictionGroup>,
) -> ScenarioManifest {
    let mut manifest = ScenarioManifest {
        schema_version: "1.0.0".to_string(),
        scenario_id: scenario_id.to_string(),
        scenario_version: "1.0.0".to_string(),
        title: title.to_string(),
        task_query: task_query.to_string(),
        items,
        annotations: ScenarioAnnotations {
            required_facts,
            distractor_source_ids,
            contradiction_groups,
        },
        content_digest: String::new(),
    };
    manifest.content_digest = canonical::digest_with_field_blanked(&manifest, "content_digest")
        .expect("scenario manifest is plain data and must always canonicalize");
    manifest
}

// ---------------------------------------------------------------------------------------------
// Scenario 1: incident-investigation
// ---------------------------------------------------------------------------------------------

fn build_incident_investigation() -> ScenarioManifest {
    let items = vec![
        item(
            "alert-1",
            0,
            "monitoring_alert",
            "ALERT INC-4471 2025-03-11T14:02:00Z service=orders-api metric=p99_latency_ms value=180 threshold=150 severity=warning",
        ),
        item(
            "alert-2",
            1,
            "monitoring_alert",
            "ALERT INC-4471 2025-03-11T14:07:00Z service=orders-api metric=p99_latency_ms value=420 threshold=150 severity=critical error_rate_pct=2.1",
        ),
        item(
            "chat-1",
            2,
            "chat_message",
            "Priya Nandan: paging in on INC-4471, orders-api p99 just hit 420ms and error rate is 2.1 percent, declaring this critical now.",
        ),
        item(
            "chat-9",
            3,
            "chat_message",
            "Alex Chen: heads up, the payments-api TLS certificate renews automatically on 2025-04-02, no action needed from on-call.",
        ),
        item(
            "chat-2",
            4,
            "chat_message",
            "Sam Rivera: the checkout-service deploy at 13:55 UTC is almost certainly the cause, let's roll it back first.",
        ),
        item(
            "chat-3",
            5,
            "chat_message",
            "Jordan Blake: might also be the CDN, they had a wobble last week. Worth checking their status page before we touch anything else.",
        ),
        item(
            "log-1",
            6,
            "log_line",
            "2025-03-11T14:09:41Z orders-api WARN payments-db-primary connection_pool_wait_ms=310 active_connections=200 max_connections=200",
        ),
        item(
            "chat-4",
            7,
            "chat_message",
            "Priya Nandan: rolled checkout-service back to v2.2.9 at 14:15 UTC. Latency is still climbing, so the deploy does not look like the cause.",
        ),
        item(
            "log-2",
            8,
            "log_line",
            "2025-03-11T14:18:22Z orders-api ERROR payments-db-primary connection_timeout_ms=5000 pool_exhausted=true active_connections=200",
        ),
        item(
            "chat-5",
            9,
            "chat_message",
            "Sam Rivera: rollback finished but I still think the deploy is related somehow, the timing lined up too well to be a coincidence.",
        ),
        item(
            "chat-6",
            10,
            "chat_message",
            "Jordan Blake: checked the CDN provider status page, no incidents logged for 2025-03-11. Ruling out the CDN.",
        ),
        item(
            "log-3",
            11,
            "log_line",
            "2025-03-11T14:44:05Z orders-api INFO migration_job=migrate-2091 status=killed triggered_by=priya.nandan",
        ),
        item(
            "chat-7",
            12,
            "chat_message",
            "Priya Nandan: found it, migrate-2091 was a schema migration originally scheduled for the 2025-03-10 maintenance window. It ran late and held a lock on payments-db-primary, exhausting the connection pool. Killed the job at 14:45 UTC.",
        ),
        item(
            "chat-8",
            13,
            "chat_message",
            "Priya Nandan: connection pool recovered right after, p99 latency is back to 95ms baseline as of 14:52 UTC. Standing down from critical.",
        ),
        item(
            "log-4",
            14,
            "log_line",
            "2025-03-11T14:53:10Z orders-api INFO p99_latency_ms=95 error_rate_pct=0.1 status=stable",
        ),
        item(
            "note-1",
            15,
            "postmortem_note",
            "Draft postmortem for INC-4471, prepared 2025-03-12. Root cause: a stuck schema migration (migrate-2091) held a lock on payments-db-primary and exhausted its connection pool, not the checkout-service deploy. Incident was declared critical at 14:07 UTC on 2025-03-11 and resolved at 14:52 UTC the same day, a duration of 45 minutes. Baseline throughput for orders-api is 640 requests/sec; peak error rate during the incident reached 2.1 percent.",
        ),
    ];

    let required_facts = vec![
        fact(
            "f-onset-latency",
            "orders-api p99 latency reached 420ms when the incident was declared critical.",
            "Establishes the peak severity of the incident in the primary metric the team pages on; without this number a reader cannot judge how bad the incident actually got.",
            vec![component(
                "c-onset-latency-420",
                vec![evidence(&items, "alert-2", "420")],
                Some(number("420", Some("ms"))),
                &[],
            )],
            &["alert-2"],
        ),
        fact(
            "f-onset-error-rate",
            "The error rate for orders-api reached 2.1 percent at the peak of the incident.",
            "Error rate alongside latency is needed to judge whether requests were merely slow or actually failing outright, which changes the customer-facing severity assessment.",
            vec![component(
                "c-error-rate-2-1",
                vec![evidence(&items, "alert-2", "2.1")],
                Some(number("2.1", Some("pct"))),
                &[],
            )],
            &["alert-2"],
        ),
        fact(
            "f-recovery-latency",
            "Latency recovered to a 95ms baseline by the end of the incident.",
            "Confirms the incident actually ended with a return to normal performance, rather than merely being downgraded in severity while still degraded.",
            vec![component(
                "c-recovery-95",
                vec![evidence(&items, "chat-8", "95ms"), evidence(&items, "log-4", "95")],
                Some(number("95", Some("ms"))),
                &[],
            )],
            &["chat-8", "log-4"],
        ),
        fact(
            "f-deploy-ruled-out",
            "The on-call engineer determined the checkout-service deploy was not the cause of the incident after rolling it back.",
            "Prevents a reader from wrongly concluding the deploy was the root cause just because it was the first suspect; preserving the negation is essential to not misattributing the incident.",
            vec![component(
                "c-deploy-not-cause",
                vec![evidence(&items, "chat-4", "the deploy does not look like the cause")],
                Some(text_value("deploy")),
                &["not"],
            )],
            &["chat-4"],
        ),
        fact(
            "f-root-cause-migration-job",
            "The root cause was the stuck schema migration job migrate-2091, which exhausted the payments-db-primary connection pool.",
            "This is the actual root cause the whole investigation was searching for; a strategy that keeps only the job id or only the exhaustion mechanism, but not both, cannot support a complete causal explanation.",
            vec![
                component(
                    "c-root-cause-job-id",
                    vec![
                        evidence(&items, "log-3", "migrate-2091"),
                        evidence(&items, "chat-7", "migrate-2091"),
                    ],
                    Some(text_value("migrate-2091")),
                    &[],
                ),
                component(
                    "c-root-cause-pool-exhaustion",
                    vec![evidence(&items, "log-2", "pool_exhausted")],
                    Some(text_value("pool_exhausted")),
                    &[],
                ),
            ],
            &["log-3", "chat-7", "log-2"],
        ),
        fact(
            "f-incident-duration",
            "The incident window ran from 14:07 UTC to 14:52 UTC on 2025-03-11, a 45-minute duration.",
            "Both the start and end times are needed to compute how long the incident actually lasted; keeping only one endpoint makes the duration unrecoverable.",
            vec![
                component(
                    "c-incident-start",
                    vec![
                        evidence(&items, "alert-2", "14:07"),
                        evidence(&items, "note-1", "14:07"),
                    ],
                    Some(text_value("14:07")),
                    &[],
                ),
                component(
                    "c-incident-end",
                    vec![
                        evidence(&items, "chat-8", "14:52"),
                        evidence(&items, "note-1", "14:52"),
                    ],
                    Some(text_value("14:52")),
                    &[],
                ),
            ],
            &["alert-2", "chat-8", "note-1"],
        ),
        fact(
            "f-migration-schedule-date",
            "The migration job had originally been scheduled for the 2025-03-10 maintenance window, a day before it actually ran.",
            "Explains why a routine migration turned dangerous: it ran outside its intended low-traffic window, which is the key lesson for preventing a repeat.",
            vec![component(
                "c-migration-scheduled-date",
                vec![evidence(&items, "chat-7", "2025-03-10")],
                Some(date("2025-03-10")),
                &[],
            )],
            &["chat-7"],
        ),
        fact(
            "f-postmortem-date",
            "The draft postmortem was prepared on 2025-03-12, one day after the incident.",
            "Indicates how fresh the documented account is relative to the incident, which affects how much the write-up should be trusted over live chat reconstructions.",
            vec![component(
                "c-postmortem-authored-date",
                vec![evidence(&items, "note-1", "2025-03-12")],
                Some(date("2025-03-12")),
                &[],
            )],
            &["note-1"],
        ),
        fact(
            "f-incident-date",
            "The incident occurred on 2025-03-11.",
            "Anchors the entire timeline to one calendar date, distinguishing it from the unrelated prior-week CDN issue Jordan initially considered.",
            vec![component(
                "c-incident-calendar-date",
                vec![
                    evidence(&items, "alert-1", "2025-03-11"),
                    evidence(&items, "alert-2", "2025-03-11"),
                ],
                Some(date("2025-03-11")),
                &[],
            )],
            &["alert-1"],
        ),
        fact(
            "f-baseline-throughput",
            "orders-api normally sustains a baseline throughput of 640 requests/sec.",
            "Gives the reader a reference point for normal operation, which is necessary context for judging how abnormal the incident's latency and error-rate numbers actually were.",
            vec![component(
                "c-baseline-throughput-640",
                vec![evidence(&items, "note-1", "640 requests/sec")],
                Some(number("640", Some("requests/sec"))),
                &[],
            )],
            &["note-1"],
        ),
    ];

    let contradiction_groups = vec![group(
        "g-incident-root-cause-dispute",
        vec![
            evidence(
                &items,
                "chat-2",
                "the checkout-service deploy at 13:55 UTC is almost certainly the cause",
            ),
            evidence(&items, "chat-4", "the deploy does not look like the cause"),
            evidence(
                &items,
                "chat-5",
                "I still think the deploy is related somehow",
            ),
        ],
    )];

    finalize(
        "incident-investigation",
        "Checkout Latency Incident INC-4471",
        "What caused the incident, how severe was it, and how was it resolved?",
        items,
        required_facts,
        distractors(&["chat-3", "chat-9"]),
        contradiction_groups,
    )
}

// ---------------------------------------------------------------------------------------------
// Scenario 2: product-comparison
// ---------------------------------------------------------------------------------------------

fn build_product_comparison() -> ScenarioManifest {
    let items = vec![
        item(
            "spec-corsair",
            0,
            "spec_sheet",
            "Corsair Gateway by Wrenfield Software. Pricing: $18 per seat per month, 10 seat minimum. Uptime SLA: 99.95 percent. Deployment: self-hosted or managed cloud.",
        ),
        item(
            "spec-halcyon",
            1,
            "spec_sheet",
            "Halcyon Gateway by Briarcliff Systems. Pricing: $24 per seat per month, 5 seat minimum. Uptime SLA: 99.9 percent. Deployment: managed cloud only.",
        ),
        item(
            "spec-corsair-legacy",
            2,
            "spec_sheet",
            "Corsair Gateway legacy notice: the original IP-allowlist-only access control module was discontinued in v3.0.0 and superseded by the policy-based access control module.",
        ),
        item(
            "market-corsair",
            3,
            "marketing_copy",
            "Corsair Gateway marketing page: 'Adaptive rate limiting ships today, fully available in the current release.'",
        ),
        item(
            "changelog-corsair",
            4,
            "changelog_entry",
            "Corsair Gateway changelog v3.4.0, 2025-04-18: adaptive rate limiting is in closed beta, targeted for general availability in v3.6.0.",
        ),
        item(
            "changelog-halcyon",
            5,
            "changelog_entry",
            "Halcyon Gateway changelog v5.1.0, 2025-04-30: fixed an issue where the audit-log export could silently drop entries under high load.",
        ),
        item(
            "bench-report",
            6,
            "benchmark_report",
            "Synthetic gateway benchmark, 2025-05-04. Corsair Gateway sustained 12400 requests/sec at p99 latency of 38ms. Halcyon Gateway sustained 9800 requests/sec at p99 latency of 61ms. Both tested against the same 64-node cluster.",
        ),
        item(
            "quote-halcyon-1",
            7,
            "customer_quote",
            "Customer note from Meadowvale Retail: 'Halcyon's managed-cloud-only deployment meant we could not keep our EU data on-prem, which was a blocker for us.'",
        ),
        item(
            "eval-note-1",
            8,
            "eval_note",
            "Internal evaluation note, 2025-05-06: our compliance requirement mandates on-premises deployment capability for the payments module; Halcyon Gateway does not offer self-hosted deployment.",
        ),
        item(
            "eval-note-2",
            9,
            "eval_note",
            "Internal evaluation note: at our expected load of 10000 requests/sec, Corsair Gateway's benchmarked 12400 requests/sec headroom looks safer than Halcyon's 9800 requests/sec, which is below our target.",
        ),
        item(
            "market-halcyon",
            10,
            "marketing_copy",
            "Halcyon Gateway marketing page: 'Best-in-class 99.99 percent uptime SLA across all plans.'",
        ),
        item(
            "quote-corsair-2",
            11,
            "customer_quote",
            "Customer note from Basalt Freight: 'We pay $18 per seat with Corsair's 10 seat minimum, so our 12-engineer team costs $216 a month.'",
        ),
        item(
            "quote-corsair-1",
            12,
            "customer_quote",
            "Customer note from Ferrow Logistics: 'Corsair's static rate limiting has been reliable for us for eight months.'",
        ),
        item(
            "eval-note-3",
            13,
            "eval_note",
            "Internal evaluation note: we briefly considered a third product, Ironclad Gateway, but its vendor was unresponsive to our sales inquiries and it was dropped from consideration before benchmarking.",
        ),
        item(
            "eval-note-4",
            14,
            "eval_note",
            "Internal evaluation note, final recommendation: choose Corsair Gateway for the payments module due to on-premises support and throughput headroom; revisit Halcyon if the on-premises requirement is later dropped.",
        ),
    ];

    let required_facts = vec![
        fact(
            "f-corsair-pricing",
            "Corsair Gateway costs $18 per seat per month with a 10 seat minimum.",
            "Both the per-seat rate and the minimum seat count are required to compute the actual monthly cost for a team, which directly drives the budget comparison between the two products.",
            vec![
                component(
                    "c-corsair-price-18",
                    vec![
                        evidence(&items, "spec-corsair", "$18 per seat per month"),
                        evidence(&items, "quote-corsair-2", "$18 per seat"),
                    ],
                    Some(number("18", Some("per seat"))),
                    &["$"],
                ),
                component(
                    "c-corsair-seat-min-10",
                    vec![
                        evidence(&items, "spec-corsair", "10 seat minimum"),
                        evidence(&items, "quote-corsair-2", "10 seat minimum"),
                    ],
                    Some(number("10", Some("seat minimum"))),
                    &[],
                ),
            ],
            &["spec-corsair", "quote-corsair-2"],
        ),
        fact(
            "f-halcyon-pricing",
            "Halcyon Gateway costs $24 per seat per month with a 5 seat minimum.",
            "Establishes Halcyon's higher per-seat rate and lower seat minimum, both of which factor into how its total cost compares to Corsair's for a given team size.",
            vec![
                component(
                    "c-halcyon-price-24",
                    vec![evidence(&items, "spec-halcyon", "$24 per seat per month")],
                    Some(number("24", Some("per seat"))),
                    &["$"],
                ),
                component(
                    "c-halcyon-seat-min-5",
                    vec![evidence(&items, "spec-halcyon", "5 seat minimum")],
                    Some(number("5", Some("seat minimum"))),
                    &[],
                ),
            ],
            &["spec-halcyon"],
        ),
        fact(
            "f-benchmark-throughput",
            "The benchmark measured Corsair Gateway at 12400 requests/sec and Halcyon Gateway at 9800 requests/sec under the same conditions.",
            "The comparison is only meaningful with both numbers together; keeping just one product's throughput number loses the relative headroom analysis the evaluation note relies on.",
            vec![
                component(
                    "c-corsair-throughput-12400",
                    vec![
                        evidence(&items, "bench-report", "12400 requests/sec"),
                        evidence(&items, "eval-note-2", "12400 requests/sec"),
                    ],
                    Some(number("12400", Some("requests/sec"))),
                    &[],
                ),
                component(
                    "c-halcyon-throughput-9800",
                    vec![
                        evidence(&items, "bench-report", "9800 requests/sec"),
                        evidence(&items, "eval-note-2", "9800 requests/sec"),
                    ],
                    Some(number("9800", Some("requests/sec"))),
                    &[],
                ),
            ],
            &["bench-report", "eval-note-2"],
        ),
        fact(
            "f-corsair-p99-latency",
            "Corsair Gateway's p99 latency in the benchmark was 38ms.",
            "p99 latency reveals tail-end performance that average throughput numbers can hide, and 38ms is the number the team would actually experience under load.",
            vec![component(
                "c-corsair-latency-38",
                vec![evidence(&items, "bench-report", "38ms")],
                Some(number("38", Some("ms"))),
                &[],
            )],
            &["bench-report"],
        ),
        fact(
            "f-halcyon-p99-latency",
            "Halcyon Gateway's p99 latency in the benchmark was 61ms.",
            "Shows Halcyon's tail latency is substantially higher than Corsair's, which matters for latency-sensitive checkout traffic even if average-case performance looks acceptable.",
            vec![component(
                "c-halcyon-latency-61",
                vec![evidence(&items, "bench-report", "61ms")],
                Some(number("61", Some("ms"))),
                &[],
            )],
            &["bench-report"],
        ),
        fact(
            "f-halcyon-uptime-per-spec",
            "Halcyon Gateway's spec sheet lists a 99.9 percent uptime SLA.",
            "This is the contractually documented SLA figure, distinct from the higher figure the marketing page separately claims, and is the number that would actually govern a support contract.",
            vec![component(
                "c-halcyon-uptime-spec-99-9",
                vec![evidence(&items, "spec-halcyon", "99.9 percent")],
                Some(number("99.9", Some("percent"))),
                &[],
            )],
            &["spec-halcyon"],
        ),
        fact(
            "f-halcyon-no-onprem",
            "Halcyon Gateway does not offer a self-hosted deployment option, which conflicts with the payments module's compliance requirement.",
            "This is a hard disqualifying constraint for the payments use case, not merely a preference, so losing this fact could lead to recommending a product that cannot legally be used for that workload.",
            vec![component(
                "c-halcyon-no-self-hosted",
                vec![evidence(&items, "eval-note-1", "does not offer self-hosted deployment")],
                Some(text_value("self-hosted")),
                &["not"],
            )],
            &["eval-note-1", "quote-halcyon-1"],
        ),
        fact(
            "f-benchmark-date",
            "The gateway benchmark was run on 2025-05-04.",
            "Dates the performance numbers so a reader can judge whether they reflect the currently shipping versions of each product rather than stale results.",
            vec![component(
                "c-benchmark-date",
                vec![evidence(&items, "bench-report", "2025-05-04")],
                Some(date("2025-05-04")),
                &[],
            )],
            &["bench-report"],
        ),
        fact(
            "f-adaptive-rate-limiting-still-beta",
            "As of the changelog entry dated 2025-04-18, Corsair Gateway's adaptive rate limiting is still in closed beta, not generally available.",
            "Both the status and the date matter together: without the date, a reader cannot tell whether this beta status is still current or long superseded, which is exactly the ambiguity the marketing page's contradictory claim depends on.",
            vec![
                component(
                    "c-adaptive-rate-limiting-beta-status",
                    vec![evidence(&items, "changelog-corsair", "closed beta")],
                    Some(text_value("closed beta")),
                    &[],
                ),
                component(
                    "c-adaptive-rate-limiting-changelog-date",
                    vec![evidence(&items, "changelog-corsair", "2025-04-18")],
                    Some(date("2025-04-18")),
                    &[],
                ),
            ],
            &["changelog-corsair", "market-corsair"],
        ),
        fact(
            "f-compliance-requirement-date",
            "The internal evaluation note recording the on-premises compliance requirement is dated 2025-05-06.",
            "Establishes that the compliance constraint was documented after the benchmark and marketing claims were already known, showing it was evaluated with full information rather than being an earlier, possibly-outdated requirement.",
            vec![component(
                "c-eval-note-1-date",
                vec![evidence(&items, "eval-note-1", "2025-05-06")],
                Some(date("2025-05-06")),
                &[],
            )],
            &["eval-note-1"],
        ),
    ];

    let contradiction_groups = vec![
        group(
            "g-adaptive-rate-limiting-availability",
            vec![
                evidence(
                    &items,
                    "market-corsair",
                    "ships today, fully available in the current release",
                ),
                evidence(&items, "changelog-corsair", "is in closed beta"),
            ],
        ),
        group(
            "g-halcyon-uptime-claim",
            vec![
                evidence(&items, "spec-halcyon", "99.9 percent"),
                evidence(&items, "market-halcyon", "99.99 percent"),
            ],
        ),
    ];

    finalize(
        "product-comparison",
        "API Gateway Product Comparison: Corsair vs Halcyon",
        "Which product should we choose given our requirements, and what are the tradeoffs?",
        items,
        required_facts,
        distractors(&["spec-corsair-legacy", "eval-note-3"]),
        contradiction_groups,
    )
}

// ---------------------------------------------------------------------------------------------
// Scenario 3: requirements-architecture-review
// ---------------------------------------------------------------------------------------------

fn build_requirements_architecture_review() -> ScenarioManifest {
    let items = vec![
        item(
            "req-1",
            0,
            "requirements_excerpt",
            "Requirements excerpt REQ-118, v1: the Inventory Lookup Service must sustain 500 requests/sec at p99 latency under 200ms. Multi-region read replication is mandatory for compliance with the Data Residency Directive.",
        ),
        item(
            "req-2",
            1,
            "requirements_excerpt",
            "Requirements excerpt REQ-119: infrastructure budget for the Inventory Lookup Service is capped at $4000 per month, inclusive of caching infrastructure.",
        ),
        item(
            "meeting-4",
            2,
            "meeting_notes",
            "Meeting notes, 2025-05-20. Attendees: Priyanka Suresh, Casey Whitfield. Mobile app redesign kickoff scheduled for next quarter; design review cadence will follow the same weekly Tuesday slot used for the Inventory Lookup Service reviews.",
        ),
        item(
            "meeting-1",
            3,
            "meeting_notes",
            "Design review meeting notes, 2025-06-02. Attendees: Morgan Ellis, Priyanka Suresh, Lena Ferraro. Discussed two caching options: a single-node in-memory cache, and a distributed cache cluster. Lena to draft an architecture decision record.",
        ),
        item(
            "adr-draft-1",
            4,
            "architecture_decision_draft",
            "Architecture decision draft ADR-07 by Lena Ferraro, 2025-06-05. Initial recommendation: adopt the single-node in-memory cache for simplicity and lower cost.",
        ),
        item(
            "meeting-2",
            5,
            "meeting_notes",
            "Design review meeting notes, 2025-06-09. Attendees: Morgan Ellis, Lena Ferraro, Devon Ashworth. Load testing showed the single-node in-memory cache cannot sustain 500 requests/sec once multi-region replication is added; the team reversed course toward a distributed cache cluster.",
        ),
        item(
            "adr-draft-2",
            6,
            "architecture_decision_draft",
            "Architecture decision draft ADR-07, revision 2, by Lena Ferraro, 2025-06-10. Final recommendation: adopt a distributed cache cluster using a cache-aside pattern across three regions, to satisfy both the 500 requests/sec throughput requirement and the Data Residency Directive's replication mandate.",
        ),
        item(
            "email-1",
            7,
            "stakeholder_email",
            "Email from Casey Whitfield to Morgan Ellis, 2025-06-11: the multi-region replication requirement is no longer needed for launch; the compliance deadline was pushed out and we should simplify the design to cut cost.",
        ),
        item(
            "email-2",
            8,
            "stakeholder_email",
            "Email from Devon Ashworth to Morgan Ellis, 2025-06-12: multi-region read replication remains mandatory under the Data Residency Directive for any service launching after 2025-09-01; this has not changed regardless of the launch date.",
        ),
        item(
            "risk-1",
            9,
            "risk_register",
            "Risk register entry RISK-22: likelihood medium, impact high. If the distributed cache cluster is not deployed across all three required regions before 2025-09-01, the service risks non-compliance with the Data Residency Directive.",
        ),
        item(
            "budget-note-1",
            10,
            "stakeholder_email",
            "Email from Casey Whitfield to Finance, 2025-06-13: please also renew the office snack budget for Q3, separate line item from infrastructure.",
        ),
        item(
            "risk-2",
            11,
            "risk_register",
            "Risk register entry RISK-23: likelihood low, impact medium. Cross-region network latency between cache nodes could add up to 15ms of replication lag under peak load.",
        ),
        item(
            "meeting-3",
            12,
            "meeting_notes",
            "Design review meeting notes, 2025-06-16. Attendees: Morgan Ellis, Priyanka Suresh, Devon Ashworth, Lena Ferraro. Confirmed the Data Residency Directive requirement stands; Casey's cost-cutting proposal was declined. Distributed cache cluster with three-region replication remains the plan.",
        ),
        item(
            "email-3",
            13,
            "stakeholder_email",
            "Email from Priyanka Suresh to the design review list, 2025-06-17: benchmark estimates for the distributed cache cluster show p99 latency of 165ms at 500 requests/sec, under our 200ms requirement.",
        ),
        item(
            "adr-final",
            14,
            "architecture_decision_draft",
            "Architecture decision record ADR-07, approved 2025-06-18. Approved by Morgan Ellis and Devon Ashworth. Final architecture: distributed cache cluster, cache-aside pattern, three-region replication, estimated infrastructure cost $3600 per month, within the $4000 budget cap.",
        ),
    ];

    let required_facts = vec![
        fact(
            "f-throughput-requirement",
            "The Inventory Lookup Service must sustain 500 requests/sec.",
            "This is the primary non-functional requirement that rules out the single-node in-memory cache once multi-region replication is added, driving the final architecture choice.",
            vec![component(
                "c-throughput-500",
                vec![
                    evidence(&items, "req-1", "500 requests/sec"),
                    evidence(&items, "meeting-2", "500 requests/sec"),
                ],
                Some(number("500", Some("requests/sec"))),
                &[],
            )],
            &["req-1"],
        ),
        fact(
            "f-latency-requirement",
            "The service must keep p99 latency under 200ms.",
            "Sets the tail-latency ceiling that the final benchmark estimate must be checked against; without it, the 165ms benchmark number has no threshold to be judged against.",
            vec![component(
                "c-latency-threshold-200",
                vec![evidence(&items, "req-1", "200ms")],
                Some(number("200", Some("ms"))),
                &[],
            )],
            &["req-1"],
        ),
        fact(
            "f-budget-cap",
            "Infrastructure budget for the service is capped at $4000 per month.",
            "This is a hard financial constraint the chosen architecture must fit within, which is what makes the final $3600 per month estimate a meaningful pass rather than an arbitrary number.",
            vec![component(
                "c-budget-cap-4000",
                vec![evidence(&items, "req-2", "$4000 per month")],
                Some(number("4000", Some("$"))),
                &[],
            )],
            &["req-2"],
        ),
        fact(
            "f-compliance-deadline",
            "Multi-region read replication is mandatory under the Data Residency Directive for any service launching after 2025-09-01.",
            "This is the firm compliance deadline that makes the replication requirement non-negotiable, directly countering the proposal to drop it for cost savings.",
            vec![component(
                "c-compliance-deadline-date",
                vec![
                    evidence(&items, "email-2", "2025-09-01"),
                    evidence(&items, "risk-1", "2025-09-01"),
                ],
                Some(date("2025-09-01")),
                &[],
            )],
            &["email-2", "risk-1"],
        ),
        fact(
            "f-final-architecture-approved",
            "The distributed cache cluster architecture (ADR-07) was formally approved on 2025-06-18.",
            "The design went through an earlier reversed recommendation, so only the combination of the final decision content and its formal approval confirms this is the architecture that actually shipped, not another draft that might again be reconsidered.",
            vec![
                component(
                    "c-final-decision-content",
                    vec![evidence(
                        &items,
                        "adr-draft-2",
                        "distributed cache cluster using a cache-aside pattern across three regions",
                    )],
                    Some(text_value("distributed cache cluster")),
                    &[],
                ),
                component(
                    "c-final-decision-approved",
                    vec![evidence(&items, "adr-final", "approved 2025-06-18")],
                    Some(date("2025-06-18")),
                    &[],
                ),
            ],
            &["adr-draft-2", "adr-final"],
        ),
        fact(
            "f-benchmark-latency-actual",
            "Benchmark estimates for the distributed cache cluster show p99 latency of 165ms at 500 requests/sec.",
            "Confirms the final architecture actually meets the 200ms latency requirement with margin, closing the loop between the requirement and the delivered design rather than leaving it an open question.",
            vec![component(
                "c-benchmark-latency-165",
                vec![evidence(&items, "email-3", "165ms")],
                Some(number("165", Some("ms"))),
                &[],
            )],
            &["email-3"],
        ),
        fact(
            "f-single-node-cache-insufficient",
            "Load testing showed the single-node in-memory cache could not sustain 500 requests/sec once multi-region replication was added, so the team reversed its initial recommendation.",
            "Explains why the initially simpler, cheaper option was abandoned; without this, a reader might wrongly resurrect the single-node recommendation as a valid cost-saving alternative.",
            vec![component(
                "c-single-node-cache-fails-throughput",
                vec![evidence(
                    &items,
                    "meeting-2",
                    "the single-node in-memory cache cannot sustain 500 requests/sec",
                )],
                Some(text_value("cannot sustain")),
                &["not"],
            )],
            &["meeting-2", "adr-draft-1"],
        ),
        fact(
            "f-requirement-reconfirmed",
            "At the 2025-06-16 design review, the Data Residency Directive replication requirement was reconfirmed and the cost-cutting proposal to drop it was declined.",
            "Documents how the disagreement over the requirement was actually resolved by the review process, which matters for anyone trying to understand why the final architecture still includes three-region replication despite the earlier proposal to simplify it.",
            vec![component(
                "c-requirement-reconfirmed-date",
                vec![evidence(&items, "meeting-3", "2025-06-16")],
                Some(date("2025-06-16")),
                &[],
            )],
            &["meeting-3"],
        ),
        fact(
            "f-final-infra-cost",
            "The approved architecture's estimated infrastructure cost is $3600 per month, within the $4000 budget cap.",
            "Shows the final architecture actually satisfies the hard budget constraint rather than merely being technically superior, which is necessary to judge the decision as fully compliant with all stated requirements.",
            vec![component(
                "c-final-cost-3600",
                vec![evidence(&items, "adr-final", "$3600 per month")],
                Some(number("3600", Some("$"))),
                &[],
            )],
            &["adr-final"],
        ),
        fact(
            "f-risk-rating",
            "Risk RISK-22 (missing the multi-region deployment deadline) is rated likelihood medium and impact high.",
            "Both the likelihood and impact ratings are needed together to judge the overall severity of the compliance risk; keeping only one axis would misrepresent how the team actually prioritized it.",
            vec![
                component(
                    "c-risk-22-likelihood-medium",
                    vec![evidence(&items, "risk-1", "likelihood medium")],
                    Some(text_value("medium")),
                    &[],
                ),
                component(
                    "c-risk-22-impact-high",
                    vec![evidence(&items, "risk-1", "impact high")],
                    Some(text_value("high")),
                    &[],
                ),
            ],
            &["risk-1"],
        ),
    ];

    let contradiction_groups = vec![group(
        "g-replication-requirement-dispute",
        vec![
            evidence(
                &items,
                "email-1",
                "the multi-region replication requirement is no longer needed for launch",
            ),
            evidence(
                &items,
                "email-2",
                "multi-region read replication remains mandatory under the Data Residency Directive for any service launching after 2025-09-01",
            ),
        ],
    )];

    finalize(
        "requirements-architecture-review",
        "Inventory Lookup Service Caching Architecture Review",
        "What does the system need to satisfy, and what architecture decision best fits those constraints?",
        items,
        required_facts,
        distractors(&["budget-note-1", "meeting-4"]),
        contradiction_groups,
    )
}

// ---------------------------------------------------------------------------------------------

fn scenarios_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR is this crate's own directory (crates/limen-core); the shared
    // `scenarios/` directory lives two levels up, at the workspace root.
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scenarios/v1")
}

fn write_manifest(manifest: &ScenarioManifest, file_name: &str) -> std::io::Result<()> {
    let dir = scenarios_dir();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(file_name);
    let json = serde_json::to_string_pretty(manifest).expect("plain data always serializes");
    std::fs::write(&path, format!("{json}\n"))?;
    println!("wrote {}", path.display());
    Ok(())
}

fn main() -> std::io::Result<()> {
    write_manifest(
        &build_incident_investigation(),
        "incident-investigation.json",
    )?;
    write_manifest(&build_product_comparison(), "product-comparison.json")?;
    write_manifest(
        &build_requirements_architecture_review(),
        "requirements-architecture-review.json",
    )?;
    Ok(())
}
