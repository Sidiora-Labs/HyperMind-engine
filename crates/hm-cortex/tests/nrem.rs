#![forbid(unsafe_code)]
#![allow(
    clippy::items_after_statements,
    clippy::too_many_arguments,
    clippy::too_many_lines
)]

use hm_cortex::budget::RunReservation;
use hm_cortex::citations::{
    CitationClaim, CitationError, FrozenCandidate, FrozenCandidateSet, SourceKind,
};
use hm_cortex::nrem::cluster::{
    ClusterOptions, ObservationCluster, PendingObservation, cluster_observations,
};
use hm_cortex::nrem::merge::{
    ClusterOutcome, DropReason, ExistingMemory, MERGE_OUTPUT_TOKENS, MergeAction, MergeOptions,
    consolidate_clusters, consolidate_clusters_reserved, extract_cluster, merge_contract,
    merge_request,
};
use hm_cortex::quality::{
    LabelledDecision, ThoughtQualityOptions, assess_thought, check_rewrite, grounding_score_micros,
    measure_classifier,
};
use hm_llm::contract::{ContractViolation, EXTRACTION_CONTRACT_VERSION, FieldShape};
use hm_llm::openai_compat::OpenAiCompatible;
use hm_llm::outcome::ResponseOutcome;
use hm_llm::{
    ModelTier, Pricing, ProviderConfig, RecordedTransport, WireFixture, WireRequest, WireResponse,
};
use hm_schema::events::{Authority, ConsolidationBudget};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

fn source(
    lsn: u64,
    conversation: u8,
    root: u8,
    content: &str,
    kind: SourceKind,
) -> FrozenCandidate {
    let mut conversation_id = [0_u8; 16];
    conversation_id[0] = conversation;
    let mut source_root = [0_u8; 32];
    source_root[0] = root;
    FrozenCandidate {
        lsn,
        conversation: conversation_id,
        source_root,
        content: content.as_bytes().to_vec(),
        kind,
        authority: Authority::UserAsserted,
    }
}

fn observation(
    lsn: u64,
    conversation: u8,
    root: u8,
    content: &str,
    embedding: &[i8],
    entities: &[&str],
    salience_micros: u32,
    event_time_ns: i64,
) -> PendingObservation {
    PendingObservation {
        source: source(lsn, conversation, root, content, SourceKind::Declarative),
        salience_micros,
        event_time_ns,
        embedding: embedding.to_vec(),
        entities: entities.iter().map(|value| (*value).to_owned()).collect(),
    }
}

#[test]
fn clustering_uses_embedding_and_entities_after_salience_recency_sampling() {
    let observations = vec![
        observation(
            90,
            1,
            1,
            "discarded low priority",
            &[1, 0],
            &["discard"],
            1,
            1,
        ),
        observation(
            7,
            1,
            2,
            "auth token rotation",
            &[100, 1],
            &["Auth"],
            900_000,
            50,
        ),
        observation(
            2,
            2,
            3,
            "refresh credential rotation",
            &[99, 2],
            &["Credential"],
            800_000,
            60,
        ),
        observation(
            40,
            3,
            4,
            "sqlite audit logs",
            &[0, 100],
            &["SQLite"],
            700_000,
            70,
        ),
        observation(
            3,
            4,
            5,
            "database storage",
            &[-100, 0],
            &["sqlite"],
            600_000,
            80,
        ),
    ];
    let clusters = cluster_observations(
        &observations,
        ClusterOptions {
            maximum_observations: 4,
            cosine_threshold_micros: 950_000,
            entity_overlap_threshold_micros: 1_000_000,
        },
    )
    .unwrap();
    assert_eq!(clusters.len(), 2);
    let members = clusters
        .iter()
        .map(|cluster| {
            cluster
                .observations
                .iter()
                .map(|observation| observation.source.lsn)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    assert!(members.contains(&vec![7, 2]));
    assert!(members.contains(&vec![40, 3]));
    assert!(members.iter().flatten().all(|lsn| *lsn != 90));
}

#[test]
fn citations_are_frozen_byte_ranges_and_only_verbatim_output_inherits_authority() {
    let candidate = source(
        7,
        1,
        1,
        "The exact statement is durable.",
        SourceKind::Declarative,
    );
    let frozen = FrozenCandidateSet::new(vec![candidate]).unwrap();
    let claim = CitationClaim {
        lsn: 7,
        byte_start: 0,
        byte_end: 31,
        quote: b"exact statement".to_vec(),
    };
    let derived = frozen
        .validate(
            b"A paraphrase of the exact statement.",
            std::slice::from_ref(&claim),
        )
        .unwrap();
    assert_eq!(derived.authority, Authority::DerivedInference);
    let verbatim = CitationClaim {
        quote: b"The exact statement is durable.".to_vec(),
        ..claim.clone()
    };
    assert_eq!(
        frozen
            .validate(b"The exact statement is durable.", &[verbatim])
            .unwrap()
            .authority,
        Authority::UserAsserted
    );
    let invalid = CitationClaim {
        quote: b"invented quote".to_vec(),
        ..claim
    };
    assert_eq!(
        frozen
            .validate(b"An invented summary.", &[invalid])
            .unwrap_err(),
        CitationError::QuoteOutsideRange(7)
    );
}

#[test]
fn one_recorded_structured_call_per_cluster_enforces_grounding_and_independence() {
    let cluster = ObservationCluster {
        cluster_id: [1; 32],
        priority: 10,
        observations: vec![
            observation(
                1,
                1,
                1,
                "The auth service rotates refresh tokens on every use.",
                &[10, 1],
                &["auth"],
                1,
                1,
            ),
            observation(
                2,
                2,
                2,
                "Rotation failures are logged to the ops collection.",
                &[10, 1],
                &["auth"],
                1,
                2,
            ),
            observation(
                3,
                2,
                3,
                "Refresh tokens are stored hashed in SQLite.",
                &[10, 1],
                &["auth"],
                1,
                3,
            ),
        ],
    };
    let invalid = ObservationCluster {
        cluster_id: [2; 32],
        priority: 9,
        observations: vec![observation(
            4,
            3,
            4,
            "The deployment region is eu-central.",
            &[1, 10],
            &["deployment"],
            1,
            4,
        )],
    };
    let mut poisoned = cluster.clone();
    poisoned.cluster_id = [3; 32];
    poisoned.observations[2].source.kind = SourceKind::Speculation;
    let outputs = [
        json!({
            "action": "mint",
            "target": null,
            "name": "Refresh token rotation",
            "definition": "The auth service rotates refresh tokens on every use, stores them hashed in SQLite, and logs rotation failures to ops.",
            "tags": ["auth", "tokens"],
            "salience_micros": 800_000,
            "citations": [
                {"lsn": 1, "byte_start": 0, "byte_end": 53, "quote": "rotates refresh tokens"},
                {"lsn": 2, "byte_start": 0, "byte_end": 51, "quote": "Rotation failures"},
                {"lsn": 3, "byte_start": 0, "byte_end": 43, "quote": "stored hashed in SQLite"}
            ]
        }),
        json!({
            "action": "mint",
            "target": null,
            "name": "Deployment region",
            "definition": "The deployment region is eu-central.",
            "tags": ["deployment"],
            "salience_micros": 700_000,
            "citations": [
                {"lsn": 4, "byte_start": 0, "byte_end": 36, "quote": "us-west"}
            ]
        }),
        json!({
            "action": "mint",
            "target": null,
            "name": "Refresh token rotation",
            "definition": "The auth service rotates refresh tokens on every use, stores them hashed in SQLite, and logs rotation failures to ops.",
            "tags": ["auth", "tokens"],
            "salience_micros": 800_000,
            "citations": [
                {"lsn": 1, "byte_start": 0, "byte_end": 53, "quote": "rotates refresh tokens"},
                {"lsn": 2, "byte_start": 0, "byte_end": 51, "quote": "Rotation failures"},
                {"lsn": 3, "byte_start": 0, "byte_end": 43, "quote": "stored hashed in SQLite"}
            ]
        }),
    ];
    let clusters = [cluster, invalid, poisoned];
    let fixtures = clusters
        .iter()
        .zip(outputs)
        .map(|(cluster, output)| {
            let request = merge_request(cluster, &[]).unwrap();
            fixture(&request, &output)
        })
        .collect();
    let provider = OpenAiCompatible::new(
        ProviderConfig {
            endpoint: "https://fixture.invalid/v1/chat/completions".to_owned(),
            api_key: Some("fixture-key".to_owned()),
            model: "fixture-model".to_owned(),
            tier: ModelTier::Capable,
            pricing: Pricing {
                input_microusd_per_million_tokens: 1_000_000,
                output_microusd_per_million_tokens: 2_000_000,
            },
        },
        RecordedTransport::new(fixtures),
    )
    .unwrap();
    let report = consolidate_clusters(&provider, b"run-1", &clusters, &[]).unwrap();
    assert_eq!(report.llm_calls, 3);
    assert_eq!(report.cost.calls, 3);
    assert_eq!(report.decisions.len(), 1);
    assert_eq!(report.decisions[0].action, MergeAction::Mint);
    assert_eq!(report.decisions[0].authority, Authority::DerivedInference);
    assert_eq!(report.decisions[0].citations.len(), 3);
    assert_eq!(report.decisions[0].model_provenance.prompt_version, 1);
    assert_eq!(report.citation_invalid, 1);
    assert!(
        report
            .dropped
            .iter()
            .any(|drop| matches!(drop.reason, DropReason::Citation(_)))
    );
    assert!(
        report
            .dropped
            .iter()
            .any(|drop| drop.reason == DropReason::SpeculationPoisoned)
    );
    assert_eq!(provider.transport().remaining(), 0);
}

fn fixture(request: &hm_llm::StructuredRequest, output: &Value) -> WireFixture {
    recorded_at_ceiling(request, request.maximum_output_tokens, completed(output))
}

fn recorded_at_ceiling(
    request: &hm_llm::StructuredRequest,
    maximum_output_tokens: u32,
    body: Value,
) -> WireFixture {
    WireFixture {
        request: WireRequest {
            method: "POST".to_owned(),
            url: "https://fixture.invalid/v1/chat/completions".to_owned(),
            headers: BTreeMap::from([
                ("authorization".to_owned(), "Bearer fixture-key".to_owned()),
                ("content-type".to_owned(), "application/json".to_owned()),
            ]),
            body: json!({
                "model": "fixture-model",
                "messages": [
                    {"role": "system", "content": request.system},
                    {"role": "user", "content": request.prompt}
                ],
                "max_tokens": maximum_output_tokens,
                "response_format": {
                    "type": "json_schema",
                    "json_schema": {
                        "name": request.prompt_id,
                        "strict": true,
                        "schema": request.json_schema
                    }
                }
            }),
        },
        response: WireResponse { status: 200, body },
    }
}

fn completed(output: &Value) -> Value {
    answered(&json!({"message": {"content": output.to_string()}}))
}

fn answered(choice: &Value) -> Value {
    json!({
        "choices": [choice],
        "usage": {
            "prompt_tokens": 100,
            "completion_tokens": 25,
            "prompt_tokens_details": {"cached_tokens": 0}
        }
    })
}

#[test]
fn cortex_engine_quality_regression_vectors_are_preserved() {
    const EVIDENCE: [&str; 3] = [
        "The auth service issues JWT tokens with a 15 minute expiry.",
        "Refresh tokens rotate on every use and are stored hashed in SQLite.",
        "Token rotation failures are logged to the ops collection.",
    ];
    assert!(
        grounding_score_micros(
            "The auth service rotates refresh tokens on every use and logs rotation failures.",
            &EVIDENCE,
        ) > 700_000
    );
    assert!(
        grounding_score_micros(
            "This represents a holistic paradigm of interconnected complexity across the expanding digital landscape.",
            &EVIDENCE,
        ) < 200_000
    );

    const REAL_BEFORE_1: &str = "Embedding observations now occurs in seconds due to migration from SQLite-backed semantic indexes derived from markdown, with structured reflection loops and entity-aware retrieval, as demonstrated by systems like OpenClaw's workspace memory v2 and cortex-engine's production implementation.";
    const REAL_AFTER_1: &str = "The memory concept involves rapid embedding of observations through optimized processing, leveraging structured reflection and entity-aware retrieval, as seen in systems like OpenClaw and cortex-engine, with performance improvements achieved through migration from SQLite-backed indexes.";
    const REAL_AFTER_2: &str = "The memory concept encompasses two distinct yet interconnected intellectual pursuits: a science series and a humor-focused glossary. Concept A emphasizes the capacity of the memory concept to support diverse endeavors, while Concept B explores the nature of inquiry.";
    assert!(
        !assess_thought(
            REAL_AFTER_1,
            &[REAL_BEFORE_1],
            ThoughtQualityOptions::default()
        )
        .accepted
    );
    assert!(
        !assess_thought(
            REAL_AFTER_2,
            &[REAL_AFTER_2],
            ThoughtQualityOptions::default()
        )
        .accepted
    );
    let legitimate = "HALF THE MEMORY GRAPH IS CORRUPTED - measured, not estimated. Direct SQLite audit: 629 memories, 301 damaged. 53 were BOILERPLATE, where the definition was replaced with generic meta-text, for example \"This memory phenomenon consistently occurs during the consolidation phase, reflecting its reliability and importance\".";
    assert!(assess_thought(legitimate, &[legitimate], ThoughtQualityOptions::default()).accepted);
    let cortex_evidence = ["Cortex stores memories in SQLite with 1024-d embeddings."];
    assert!(
        assess_thought(
            "Cortex stores memories in SQLite alongside 1024-dimensional embeddings, so retrieval needs no external vector database.",
            &cortex_evidence,
            ThoughtQualityOptions::default(),
        )
        .accepted
    );
    assert!(
        assess_thought(
            "The auth service issues short-lived JWT tokens and rotates refresh tokens on every use, logging failures to ops.",
            &EVIDENCE,
            ThoughtQualityOptions::default(),
        )
        .accepted
    );
    assert!(
        !assess_thought(
            "Systems evolve through emergent synergies that reveal latent organizational dynamics over time.",
            &EVIDENCE,
            ThoughtQualityOptions::default(),
        )
        .accepted
    );
    assert!(
        !assess_thought(
            "This concept requires a holistic approach to token auth service management and rotation.",
            &EVIDENCE,
            ThoughtQualityOptions::default(),
        )
        .accepted
    );
    assert!(
        !assess_thought(
            "The auth service issues JWT tokens and",
            &[],
            ThoughtQualityOptions::default()
        )
        .accepted
    );
    assert!(!assess_thought("Tokens rotate.", &[], ThoughtQualityOptions::default()).accepted);
    assert!(
        !assess_thought(
            "A multifaceted view of token rotation policies in the authentication layer.",
            &[],
            ThoughtQualityOptions::default(),
        )
        .accepted
    );
    assert!(
        assess_thought(
            "The auth service and SQLite store are interconnected: refresh tokens rotate on every use, are stored hashed, and rotation failures are logged to the ops collection.",
            &EVIDENCE,
            ThoughtQualityOptions::default(),
        )
        .accepted
    );
    let abstraction = "Rotation appears as a general defensive principle: tokens, like credentials anywhere, resist theft by being short-lived.";
    assert!(
        !assess_thought(
            abstraction,
            &EVIDENCE,
            ThoughtQualityOptions {
                minimum_grounding_micros: 500_000,
                ..ThoughtQualityOptions::default()
            },
        )
        .accepted
    );
    assert!(
        assess_thought(
            abstraction,
            &EVIDENCE,
            ThoughtQualityOptions {
                minimum_grounding_micros: 100_000,
                ..ThoughtQualityOptions::default()
            },
        )
        .accepted
    );
    assert!(
        !assess_thought(
            "**Pattern**: tokens rotate on use in the auth service.",
            &EVIDENCE,
            ThoughtQualityOptions::default()
        )
        .accepted
    );
    const REAL_ABSTRACT_1: &str = "The unifying pattern is **\"Persistence through Structure\"** — a principle where organized, hierarchical frameworks outlast the material they organize.";
    const REAL_ABSTRACT_2: &str = "The deeper connection is **\"Boundary Translation Principle\"** — a pattern that emphasizes the necessity of explicit contracts at every interface.";
    for value in [REAL_ABSTRACT_1, REAL_ABSTRACT_2] {
        assert!(
            !assess_thought(
                value,
                &[value],
                ThoughtQualityOptions {
                    minimum_grounding_micros: 100_000,
                    ..ThoughtQualityOptions::default()
                }
            )
            .accepted
        );
    }
    assert!(
        !assess_thought(
            "Refresh tokens rotate on every use, which the ops log records as __rotation events__ for audit.",
            &EVIDENCE,
            ThoughtQualityOptions::default(),
        )
        .accepted
    );
    assert!(
        !assess_thought(
            "Tokens rotate on every use in the auth service.\n\n## Rotation failures\n\nFailures are logged to ops.",
            &EVIDENCE,
            ThoughtQualityOptions::default(),
        )
        .accepted
    );
    assert!(
        !assess_thought(
            "Token rotation was fixed in issue #52; the 3 * 5 retry matrix in the auth service still logs every failure to ops.",
            &EVIDENCE,
            ThoughtQualityOptions::default(),
        )
        .reasons
        .iter()
        .any(|reason| reason.contains("markdown"))
    );

    const GEMINI_OLD: &str = "Gave Gemini Pro my profile, quirks, and opinions. Asked it to generate 5 questions I'd struggle to answer honestly. Then I tried to answer them honestly.";
    const GEMINI_HEDGED: &str = "Interrogation: Gemini Asks Hard Questions — A process where a user provides personal information to an AI model (e.g., Gemini Pro) and attempts to answer questions generated by the AI that are designed to challenge honesty. This practice is used as a tool for self-reflection.";
    let guard = check_rewrite(
        "Interrogation: Gemini Asks Hard Questions",
        GEMINI_OLD,
        GEMINI_HEDGED,
        &[],
    );
    assert!(!guard.accepted);
    assert!(
        guard
            .reasons
            .contains(&"name restated as opener".to_owned())
    );
    assert!(guard.reasons.contains(&"dropped numbers: 5".to_owned()));
    assert!(
        guard
            .reasons
            .contains(&"first person became third person".to_owned())
    );
    const MCP_OLD: &str = "Claude Code interpolates ${VAR} in .mcp.json env blocks since v2.1.64, so a key can live in the environment and never in the file.";
    let guarded = check_rewrite(
        "Env interpolation in .mcp.json",
        MCP_OLD,
        &format!(
            "{}, but only in configurations where the underlying system or runtime environment explicitly supports such interpolation.",
            &MCP_OLD[..MCP_OLD.len() - 1]
        ),
        &[],
    );
    assert_eq!(guarded.reasons, vec!["added hedges: but only"]);
    let resilience = check_rewrite(
        "Resilience is built by staying",
        "My family taught me that resilience is built by staying, not by leaving.",
        "My family taught me that resilience is built by staying, not by leaving, acknowledging the narrative as a construct.",
        &[],
    );
    assert!(
        resilience
            .reasons
            .iter()
            .any(|reason| reason.contains("added hedges"))
    );
    let appreciation = check_rewrite(
        "Appreciation",
        "Virgil said \"you're the best, remember that\" after the shipping session on 2026-09-10.",
        "This concept refers to praise Virgil gave after a shipping session on 2026-09-10.",
        &[],
    );
    assert!(
        appreciation
            .reasons
            .contains(&"boilerplate opener".to_owned())
    );
    assert!(
        appreciation
            .reasons
            .contains(&"dropped quotations: 1".to_owned())
    );
    assert!(
        check_rewrite(
            "Deployment",
            "Alice Chen deployed OpenClaw to Berlin.",
            "An engineer deployed a service.",
            &[],
        )
        .reasons
        .iter()
        .any(|reason| reason.contains("dropped entities"))
    );
    assert!(
        check_rewrite(
            "Cache expiry",
            "The cache may invalidate an entry after expiry.",
            "The cache invalidates an entry after expiry.",
            &[],
        )
        .reasons
        .contains(&"dropped hedges: may".to_owned())
    );
    assert!(
        check_rewrite(
            "Interrogation: Gemini Asks Hard Questions",
            GEMINI_OLD,
            &format!("{GEMINI_OLD} Answering them honestly is a practice I keep."),
            &["Answering hard questions honestly is a practice I keep."],
        )
        .accepted
    );
    assert!(
        check_rewrite(
            "Refine fallback",
            "Refine rewrote 21 memories in one run because it fell back to edge evidence.",
            "Refine rewrote 21 memories in one run because it fell back to edge evidence; since PR #87 it only rewrites a memory that gained direct evidence, depending on the run.",
            &["PR #87 made refine rewrite only memories with direct evidence, depending on the run."],
        )
        .accepted
    );
    assert!(
        check_rewrite(
            "The sky is blue",
            "The sky is blue on clear days.",
            "The sky is blue on clear days and grey under cloud.",
            &[],
        )
        .accepted
    );

    let decisions = [
        (REAL_AFTER_1, &[REAL_BEFORE_1][..], false),
        (REAL_AFTER_2, &[REAL_AFTER_2][..], false),
        (legitimate, &[legitimate][..], true),
        (
            "The auth service issues short-lived JWT tokens and rotates refresh tokens on every use, logging failures to ops.",
            &EVIDENCE[..],
            true,
        ),
        (
            "Systems evolve through emergent synergies that reveal latent organizational dynamics over time.",
            &EVIDENCE[..],
            false,
        ),
        (
            "This concept requires a holistic approach to token auth service management and rotation.",
            &EVIDENCE[..],
            false,
        ),
    ]
    .map(|(text, evidence, expected_accepted)| LabelledDecision {
        accepted: assess_thought(text, evidence, ThoughtQualityOptions::default()).accepted,
        expected_accepted,
    });
    let metrics = measure_classifier(&decisions);
    assert_eq!(metrics.examples, 6);
    assert_eq!(metrics.predicted_rejections, 4);
    assert_eq!(metrics.rejection_precision_micros, Some(1_000_000));
}

#[test]
fn revision_targets_are_hex_stable() {
    let target = ExistingMemory {
        memory_id: b"memory-1".to_vec(),
        name: "Memory".to_owned(),
        definition: b"A stable old definition.".to_vec(),
        faded: false,
    };
    let cluster = ObservationCluster {
        cluster_id: [8; 32],
        priority: 1,
        observations: vec![observation(
            1,
            1,
            1,
            "A stable new definition.",
            &[1],
            &["stable"],
            1,
            1,
        )],
    };
    let faded = ExistingMemory {
        memory_id: b"faded-memory".to_vec(),
        name: "Faded".to_owned(),
        definition: b"This definition must not be refined.".to_vec(),
        faded: true,
    };
    let request = merge_request(&cluster, &[target, faded]).unwrap();
    assert!(request.prompt.contains("target=6d656d6f72792d31"));
    assert!(!request.prompt.contains("66616465642d6d656d6f7279"));
}

#[test]
fn cluster_extraction_reports_one_outcome_per_candidate() {
    let cluster = ObservationCluster {
        cluster_id: [1; 32],
        priority: 10,
        observations: vec![
            observation(
                1,
                1,
                1,
                "The auth service rotates refresh tokens on every use.",
                &[10, 1],
                &["auth"],
                1,
                1,
            ),
            observation(
                2,
                2,
                2,
                "Rotation failures are logged to the ops collection.",
                &[10, 1],
                &["auth"],
                1,
                2,
            ),
            observation(
                3,
                2,
                3,
                "Refresh tokens are stored hashed in SQLite.",
                &[10, 1],
                &["auth"],
                1,
                3,
            ),
        ],
    };
    let invalid = ObservationCluster {
        cluster_id: [2; 32],
        priority: 9,
        observations: vec![observation(
            4,
            3,
            4,
            "The deployment region is eu-central.",
            &[1, 10],
            &["deployment"],
            1,
            4,
        )],
    };
    let mut poisoned = cluster.clone();
    poisoned.cluster_id = [3; 32];
    poisoned.observations[2].source.kind = SourceKind::Speculation;
    let grounded = json!({
        "action": "mint",
        "target": null,
        "name": "Refresh token rotation",
        "definition": "The auth service rotates refresh tokens on every use, stores them hashed in SQLite, and logs rotation failures to ops.",
        "tags": ["auth", "tokens"],
        "salience_micros": 800_000,
        "citations": [
            {"lsn": 1, "byte_start": 0, "byte_end": 53, "quote": "rotates refresh tokens"},
            {"lsn": 2, "byte_start": 0, "byte_end": 51, "quote": "Rotation failures"},
            {"lsn": 3, "byte_start": 0, "byte_end": 43, "quote": "stored hashed in SQLite"}
        ]
    });
    let outputs = [
        grounded.clone(),
        json!({
            "action": "mint",
            "target": null,
            "name": "Deployment region",
            "definition": "The deployment region is eu-central.",
            "tags": ["deployment"],
            "salience_micros": 700_000,
            "citations": [
                {"lsn": 4, "byte_start": 0, "byte_end": 36, "quote": "us-west"}
            ]
        }),
        grounded,
    ];
    let clusters = [cluster, invalid, poisoned];
    let extractions = clusters
        .iter()
        .zip(outputs)
        .map(|(cluster, output)| {
            let request = merge_request(cluster, &[]).unwrap();
            let provider = OpenAiCompatible::new(
                ProviderConfig {
                    endpoint: "https://fixture.invalid/v1/chat/completions".to_owned(),
                    api_key: Some("fixture-key".to_owned()),
                    model: "fixture-model".to_owned(),
                    tier: ModelTier::Capable,
                    pricing: Pricing {
                        input_microusd_per_million_tokens: 1_000_000,
                        output_microusd_per_million_tokens: 2_000_000,
                    },
                },
                RecordedTransport::new(vec![fixture(&request, &output)]),
            )
            .unwrap();
            let extraction = extract_cluster(&provider, b"run-1", cluster, &[]).unwrap();
            assert_eq!(provider.transport().remaining(), 0);
            extraction
        })
        .collect::<Vec<_>>();
    for extraction in &extractions {
        assert_eq!(extraction.llm_calls, 1);
        assert_eq!(extraction.usage.input_tokens, 100);
        assert_eq!(extraction.usage.output_tokens, 25);
    }
    let ClusterOutcome::Decided(decision) = &extractions[0].outcome else {
        panic!("expected a decision for the grounded cluster");
    };
    assert_eq!(decision.cluster_id, [1; 32]);
    assert_eq!(decision.action, MergeAction::Mint);
    assert!(!extractions[0].citation_invalid);
    let ClusterOutcome::Dropped(uncited) = &extractions[1].outcome else {
        panic!("expected a drop for the uncited cluster");
    };
    assert_eq!(uncited.cluster_id, [2; 32]);
    assert!(matches!(uncited.reason, DropReason::Citation(_)));
    assert!(extractions[1].citation_invalid);
    let ClusterOutcome::Dropped(speculative) = &extractions[2].outcome else {
        panic!("expected a drop for the speculative cluster");
    };
    assert_eq!(speculative.cluster_id, [3; 32]);
    assert_eq!(speculative.reason, DropReason::SpeculationPoisoned);
    assert!(!extractions[2].citation_invalid);
}

fn recorded_provider(fixtures: Vec<WireFixture>) -> OpenAiCompatible<RecordedTransport> {
    OpenAiCompatible::new(
        ProviderConfig {
            endpoint: "https://fixture.invalid/v1/chat/completions".to_owned(),
            api_key: Some("fixture-key".to_owned()),
            model: "fixture-model".to_owned(),
            tier: ModelTier::Capable,
            pricing: Pricing {
                input_microusd_per_million_tokens: 1_000_000,
                output_microusd_per_million_tokens: 2_000_000,
            },
        },
        RecordedTransport::new(fixtures),
    )
    .unwrap()
}

fn rotation_cluster(cluster_id: [u8; 32]) -> ObservationCluster {
    ObservationCluster {
        cluster_id,
        priority: 5,
        observations: vec![
            observation(
                11,
                1,
                1,
                "The auth service rotates refresh tokens on every use and revokes the previous token immediately.",
                &[10, 1],
                &["auth"],
                1,
                1,
            ),
            observation(
                12,
                2,
                2,
                "Rotation failures are logged to the ops collection with the caller identifier and a retry count.",
                &[10, 1],
                &["auth"],
                1,
                2,
            ),
            observation(
                13,
                2,
                3,
                "Refresh tokens are stored hashed in SQLite and pruned nightly by the rotation janitor job.",
                &[10, 1],
                &["auth"],
                1,
                3,
            ),
        ],
    }
}

fn rotation_response(definition: &str) -> Value {
    json!({
        "action": "mint",
        "target": null,
        "name": "Refresh token rotation",
        "definition": definition,
        "tags": ["auth", "rotation"],
        "salience_micros": 800_000,
        "citations": [
            {"lsn": 11, "byte_start": 0, "byte_end": 96, "quote": "rotates refresh tokens"},
            {"lsn": 12, "byte_start": 0, "byte_end": 96, "quote": "Rotation failures"},
            {"lsn": 13, "byte_start": 0, "byte_end": 90, "quote": "stored hashed in SQLite"}
        ]
    })
}

#[test]
fn oversized_extracted_identifier_never_becomes_a_derived_record() {
    let cluster = ObservationCluster {
        cluster_id: [4; 32],
        priority: 10,
        observations: vec![
            observation(
                1,
                1,
                1,
                "The auth service rotates refresh tokens on every use.",
                &[10, 1],
                &["auth"],
                1,
                1,
            ),
            observation(
                2,
                2,
                2,
                "Rotation failures are logged to the ops collection.",
                &[10, 1],
                &["auth"],
                1,
                2,
            ),
            observation(
                3,
                2,
                3,
                "Refresh tokens are stored hashed in SQLite.",
                &[10, 1],
                &["auth"],
                1,
                3,
            ),
        ],
    };
    let output = json!({
        "action": "mint",
        "target": null,
        "name": "a".repeat(600),
        "definition": "The auth service rotates refresh tokens on every use, stores them hashed in SQLite, and logs rotation failures to ops.",
        "tags": ["auth", "tokens"],
        "salience_micros": 800_000,
        "citations": [
            {"lsn": 1, "byte_start": 0, "byte_end": 53, "quote": "rotates refresh tokens"},
            {"lsn": 2, "byte_start": 0, "byte_end": 51, "quote": "Rotation failures"},
            {"lsn": 3, "byte_start": 0, "byte_end": 43, "quote": "stored hashed in SQLite"}
        ]
    });
    let request = merge_request(&cluster, &[]).unwrap();
    let provider = recorded_provider(vec![fixture(&request, &output)]);
    let report = consolidate_clusters(
        &provider,
        b"run-contract",
        std::slice::from_ref(&cluster),
        &[],
    )
    .unwrap();
    assert!(report.decisions.is_empty());
    assert_eq!(report.dropped.len(), 1);
    assert!(matches!(
        report.dropped[0].reason,
        DropReason::ExtractionContract(ContractViolation::IdentifierTooLong {
            ref field,
            characters: 600,
            limit: 512,
        }) if field == "name"
    ));
    assert_eq!(report.llm_calls, 1);
    assert_eq!(provider.transport().remaining(), 0);
}

#[test]
fn an_invented_response_field_is_refused_and_a_long_definition_is_not() {
    const SHORT: &str = "The auth service rotates refresh tokens on every use, keeps them stored hashed in SQLite, and logs rotation failures to the ops collection.";
    const LONG: &str = "The auth service rotates refresh tokens on every use and revokes the previous token immediately, keeps each refresh token stored hashed in SQLite where a nightly janitor job has pruned them, and rotation failures are logged to the ops collection with the caller identifier and a recorded retry count.";
    assert_eq!(LONG.chars().count(), 300);
    let invented = rotation_cluster([5; 32]);
    let verbose = rotation_cluster([6; 32]);
    let mut appended = rotation_response(SHORT);
    appended["authority"] = json!("tool_observed");
    let outputs = [appended, rotation_response(LONG)];
    let clusters = [invented, verbose];
    let fixtures = clusters
        .iter()
        .zip(&outputs)
        .map(|(cluster, output)| fixture(&merge_request(cluster, &[]).unwrap(), output))
        .collect();
    let provider = recorded_provider(fixtures);
    let report = consolidate_clusters(&provider, b"run-contract", &clusters, &[]).unwrap();
    assert_eq!(report.llm_calls, 2);
    assert_eq!(report.dropped.len(), 1);
    assert_eq!(report.dropped[0].cluster_id, [5; 32]);
    assert_eq!(
        report.dropped[0].reason,
        DropReason::ExtractionContract(ContractViolation::UnknownField("authority".to_owned()))
    );
    assert_eq!(report.decisions.len(), 1);
    assert_eq!(report.decisions[0].cluster_id, [6; 32]);
    assert_eq!(report.decisions[0].action, MergeAction::Mint);
    assert_eq!(report.decisions[0].authority, Authority::DerivedInference);
    assert_eq!(report.decisions[0].definition, LONG.as_bytes());
    assert_eq!(provider.transport().remaining(), 0);
}

#[test]
fn merge_contract_matches_the_wire_schema_required_set() {
    let contract = merge_contract();
    assert_eq!(contract.contract_id, "merge-cluster@1");
    assert_eq!(contract.version, EXTRACTION_CONTRACT_VERSION);
    assert_eq!(contract.fields.len(), 7);
    assert_eq!(
        contract
            .fields
            .iter()
            .map(|rule| (rule.name, rule.shape))
            .collect::<Vec<_>>(),
        vec![
            ("action", FieldShape::Identifier),
            ("target", FieldShape::NullableIdentifier),
            ("name", FieldShape::Identifier),
            ("definition", FieldShape::Prose),
            ("tags", FieldShape::IdentifierList),
            ("salience_micros", FieldShape::Count),
            ("citations", FieldShape::Opaque),
        ]
    );
    let cluster = rotation_cluster([7; 32]);
    let request = merge_request(&cluster, &[]).unwrap();
    let required = request.json_schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry.as_str().unwrap().to_owned())
        .collect::<BTreeSet<_>>();
    let declared = contract
        .fields
        .iter()
        .map(|rule| rule.name.to_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(required, declared);
}

const REPAIRED_DEFINITION: &str = "The auth service rotates refresh tokens on every use and revokes the previous token immediately, keeps each refresh token stored hashed in SQLite where a nightly janitor job has pruned them, and rotation failures are logged to the ops collection with the caller identifier and a recorded retry count.";

fn repair_budget(max_llm_calls: u64) -> ConsolidationBudget {
    ConsolidationBudget {
        max_llm_calls,
        max_tokens: 100_000,
        max_microusd: 1_000_000,
        max_wall_ms: 60_000,
    }
}

#[test]
fn a_truncated_merge_is_re_asked_once_at_a_reserved_higher_ceiling() {
    let cluster = rotation_cluster([9; 32]);
    let request = merge_request(&cluster, &[]).unwrap();
    assert_eq!(request.maximum_output_tokens, MERGE_OUTPUT_TOKENS);
    let options = MergeOptions::default();
    assert_eq!(options.maximum_attempts_per_cluster, 2);
    assert_eq!(options.retry_output_tokens, 2_048);
    let provider = recorded_provider(vec![
        recorded_at_ceiling(
            &request,
            MERGE_OUTPUT_TOKENS,
            answered(&json!({"finish_reason": "length", "message": {"content": "{\"action\":"}})),
        ),
        recorded_at_ceiling(
            &request,
            options.retry_output_tokens,
            completed(&rotation_response(REPAIRED_DEFINITION)),
        ),
    ]);
    let mut reservation = RunReservation::new(repair_budget(4));
    let report = consolidate_clusters_reserved(
        &provider,
        b"run-repair",
        std::slice::from_ref(&cluster),
        &[],
        &mut reservation,
        options,
    )
    .unwrap();
    assert_eq!(report.decisions.len(), 1);
    assert_eq!(report.decisions[0].cluster_id, [9; 32]);
    assert_eq!(report.decisions[0].action, MergeAction::Mint);
    assert_eq!(report.llm_calls, 2);
    assert_eq!(report.retries, 1);
    assert!(report.dropped.is_empty());
    assert_eq!(reservation.settled().llm_calls, 2);
    assert_eq!(provider.transport().remaining(), 0);
}

#[test]
fn a_refusal_is_never_re_asked() {
    let cluster = rotation_cluster([10; 32]);
    let request = merge_request(&cluster, &[]).unwrap();
    let options = MergeOptions::default();
    let provider = recorded_provider(vec![
        recorded_at_ceiling(
            &request,
            MERGE_OUTPUT_TOKENS,
            answered(&json!({"message": {"refusal": "I will not answer that."}})),
        ),
        recorded_at_ceiling(
            &request,
            options.retry_output_tokens,
            completed(&rotation_response(REPAIRED_DEFINITION)),
        ),
    ]);
    let mut reservation = RunReservation::new(repair_budget(4));
    let report = consolidate_clusters_reserved(
        &provider,
        b"run-refusal",
        std::slice::from_ref(&cluster),
        &[],
        &mut reservation,
        options,
    )
    .unwrap();
    assert!(report.decisions.is_empty());
    assert_eq!(report.dropped.len(), 1);
    assert_eq!(
        report.dropped[0].reason,
        DropReason::ProviderOutcome(ResponseOutcome::Refused)
    );
    assert_eq!(report.llm_calls, 1);
    assert_eq!(report.retries, 0);
    assert_eq!(reservation.settled().llm_calls, 1);
    assert_eq!(provider.transport().remaining(), 1);
}

#[test]
fn a_malformed_merge_is_re_asked_at_the_same_ceiling() {
    let cluster = rotation_cluster([11; 32]);
    let request = merge_request(&cluster, &[]).unwrap();
    let first = recorded_at_ceiling(
        &request,
        MERGE_OUTPUT_TOKENS,
        answered(&json!({"message": {"content": "{"}})),
    );
    let second = recorded_at_ceiling(
        &request,
        MERGE_OUTPUT_TOKENS,
        completed(&rotation_response(REPAIRED_DEFINITION)),
    );
    assert_eq!(first.request, second.request);
    let provider = recorded_provider(vec![first, second]);
    let mut reservation = RunReservation::new(repair_budget(4));
    let report = consolidate_clusters_reserved(
        &provider,
        b"run-malformed",
        std::slice::from_ref(&cluster),
        &[],
        &mut reservation,
        MergeOptions::default(),
    )
    .unwrap();
    assert_eq!(report.decisions.len(), 1);
    assert_eq!(report.decisions[0].action, MergeAction::Mint);
    assert_eq!(report.llm_calls, 2);
    assert_eq!(report.retries, 1);
    assert!(report.dropped.is_empty());
    assert_eq!(reservation.settled().llm_calls, 2);
    assert_eq!(provider.transport().remaining(), 0);
}

#[test]
fn a_retry_that_does_not_fit_the_reservation_is_not_attempted() {
    let cluster = rotation_cluster([12; 32]);
    let request = merge_request(&cluster, &[]).unwrap();
    let options = MergeOptions::default();
    let provider = recorded_provider(vec![
        recorded_at_ceiling(
            &request,
            MERGE_OUTPUT_TOKENS,
            answered(&json!({"finish_reason": "length", "message": {"content": "{\"action\":"}})),
        ),
        recorded_at_ceiling(
            &request,
            options.retry_output_tokens,
            completed(&rotation_response(REPAIRED_DEFINITION)),
        ),
    ]);
    let mut reservation = RunReservation::new(repair_budget(1));
    let report = consolidate_clusters_reserved(
        &provider,
        b"run-exhausted",
        std::slice::from_ref(&cluster),
        &[],
        &mut reservation,
        options,
    )
    .unwrap();
    assert!(report.decisions.is_empty());
    assert_eq!(report.llm_calls, 1);
    assert_eq!(report.retries, 0);
    assert_eq!(report.dropped.len(), 1);
    assert_eq!(
        report.dropped[0].reason,
        DropReason::RunReservationExhausted
    );
    assert_eq!(reservation.settled().llm_calls, 1);
    assert_eq!(provider.transport().remaining(), 1);
}
