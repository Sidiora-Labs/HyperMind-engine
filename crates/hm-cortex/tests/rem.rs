#![forbid(unsafe_code)]
#![allow(clippy::items_after_statements, clippy::too_many_lines)]

use chrono::DateTime;
use hm_cortex::budget::{BudgetDimension, BudgetTracker, BudgetUsage};
use hm_cortex::citations::{FrozenCandidate, SourceKind};
use hm_cortex::fsrs::{FsrsData, FsrsState, initial_stability, retrievability, schedule_next};
use hm_cortex::rem::r#abstract::{
    AbstractDrop, AbstractError, AbstractOptions, ExistingAbstract, abstract_request, synthesize,
};
use hm_cortex::rem::connect::{ConnectMemory, connect, connect_request};
use hm_cortex::rem::hindsight::{
    ConcernSource, ConcernSourceKind, HindsightDecision, HindsightMemory, audit, hindsight_request,
};
use hm_cortex::review::{
    AttestationSignal, ContradictedOutcome, FadePolicy, ProtectionSet, ReviewCandidate,
    counterexample_protection, fading_event, schedule_reviews,
};
use hm_cortex::run::{
    Cadence, CadenceError, PersistedPhase, PhaseMachine, due, retraction_event, run_id,
};
use hm_llm::openai_compat::OpenAiCompatible;
use hm_llm::{
    ModelTier, Pricing, ProviderConfig, RecordedTransport, StructuredRequest, Usage, WireFixture,
    WireRequest, WireResponse,
};
use hm_schema::events::{
    Authority, ConsolidationBudget, ConsolidationPhaseName, ConsolidationPhaseState, Retention,
    ReviewRating,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

fn source(lsn: u64, conversation: u8, root: u8, content: &str) -> FrozenCandidate {
    let mut conversation_id = [0; 16];
    conversation_id[0] = conversation;
    let mut source_root = [0; 32];
    source_root[0] = root;
    FrozenCandidate {
        lsn,
        conversation: conversation_id,
        source_root,
        content: content.as_bytes().to_vec(),
        kind: SourceKind::Declarative,
        authority: Authority::UserAsserted,
    }
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

fn fixture(request: &StructuredRequest, output: &Value) -> WireFixture {
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
                "max_tokens": request.maximum_output_tokens,
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
        response: WireResponse {
            status: 200,
            body: json!({
                "choices": [{"message": {"content": output.to_string()}}],
                "usage": {
                    "prompt_tokens": 100,
                    "completion_tokens": 20,
                    "prompt_tokens_details": {"cached_tokens": 10}
                }
            }),
        },
    }
}

#[test]
fn connect_uses_one_long_context_call_and_evidence_count_weights() {
    let left = ConnectMemory {
        memory_id: vec![1],
        cluster_ordinal: 1,
        name: "Token rotation".to_owned(),
        definition: b"Refresh tokens rotate after use.".to_vec(),
        entities: BTreeSet::from(["Auth".to_owned()]),
        evidence: vec![source(10, 1, 1, "Refresh tokens rotate after use.")],
        faded: false,
    };
    let right = ConnectMemory {
        memory_id: vec![2],
        cluster_ordinal: 9,
        name: "Failure audit".to_owned(),
        definition: b"Auth failures are logged in ops.".to_vec(),
        entities: BTreeSet::from(["Auth".to_owned()]),
        evidence: vec![source(11, 2, 2, "Auth failures are logged in ops.")],
        faded: false,
    };
    let faded = ConnectMemory {
        memory_id: vec![3],
        cluster_ordinal: 1,
        name: "Old".to_owned(),
        definition: b"Old memory.".to_vec(),
        entities: BTreeSet::from(["Auth".to_owned()]),
        evidence: vec![source(12, 3, 3, "Old memory.")],
        faded: true,
    };
    let memories = [left, right, faded];
    let request = connect_request(&memories, &[(0, 1)]).unwrap();
    let provider = recorded_provider(vec![fixture(
        &request,
        &json!({
            "edges": [{
                "source": "01",
                "target": "02",
                "relation": "records rotation failures",
                "evidence_lsns": [10, 11],
                "valid_from_ns": 100,
                "valid_to_ns": 0
            }]
        }),
    )]);
    let report = connect(&provider, b"run", &memories).unwrap();
    assert_eq!(report.llm_calls, 1);
    assert_eq!(report.edges.len(), 1);
    assert_eq!(report.edges[0].event.weight_micros, 500_000);
    assert_eq!(report.edges[0].event.citations.len(), 2);
    assert_eq!(report.edges[0].model_provenance.prompt_version, 1);
    assert!(!request.prompt.contains("id=03"));
    assert_eq!(provider.transport().remaining(), 0);
}

#[test]
fn abstraction_enforces_grounding_and_novelty_against_store_and_run() {
    let sources = [
        source(20, 1, 1, "Refresh tokens rotate after every use."),
        source(21, 2, 2, "Rotation failures are recorded in the ops log."),
    ];
    let output = json!({
        "name": "Audited token rotation",
        "definition": "Token rotation limits credential reuse and records rotation failures in the ops log.",
        "citations": [
            {"lsn": 20, "byte_start": 0, "byte_end": 38, "quote": "tokens rotate"},
            {"lsn": 21, "byte_start": 0, "byte_end": 46, "quote": "Rotation failures"}
        ]
    });
    let request = abstract_request("auth", &sources).unwrap();
    let provider = recorded_provider(vec![fixture(&request, &output)]);
    let decision = synthesize(
        &provider,
        b"run",
        "auth",
        &sources,
        &[],
        &[],
        AbstractOptions {
            maximum_similarity_micros: 800_000,
            minimum_grounding_micros: 100_000,
        },
    )
    .unwrap();
    assert_eq!(decision.model_provenance.prompt_version, 2);
    assert_eq!(decision.citations.len(), 2);
    assert!(decision.quality.accepted);

    let provider = recorded_provider(vec![fixture(&request, &output)]);
    let duplicate = synthesize(
        &provider,
        b"run",
        "auth",
        &sources,
        &[ExistingAbstract {
            definition: String::from_utf8(decision.definition.clone()).unwrap(),
            faded: false,
        }],
        &[],
        AbstractOptions {
            maximum_similarity_micros: 800_000,
            minimum_grounding_micros: 100_000,
        },
    );
    assert!(matches!(
        duplicate,
        Err(AbstractError::Drop(AbstractDrop::Duplicate {
            similarity_micros: 1_000_000
        }))
    ));
}

#[test]
fn hindsight_defaults_to_no_change_and_declines_lossy_rewrites() {
    let memory = HindsightMemory {
        memory_id: vec![5],
        name: "Gemini questions".to_owned(),
        candidate: source(
            29,
            1,
            1,
            "I asked Gemini to generate 5 questions and quote \"be honest\".",
        ),
        faded: false,
    };
    let sources = [ConcernSource {
        kind: ConcernSourceKind::Neighbour,
        candidate: source(30, 1, 1, "The neighbour says the questions were difficult."),
    }];
    let request = hindsight_request(&memory, &sources).unwrap();
    let provider = recorded_provider(vec![fixture(
        &request,
        &json!({"action": "no_change", "concern": null, "definition": null, "citations": []}),
    )]);
    assert_eq!(
        audit(&provider, b"run", &memory, &sources).unwrap(),
        HindsightDecision::NoChange
    );

    let provider = recorded_provider(vec![fixture(
        &request,
        &json!({
            "action": "revise",
            "concern": "The neighbour says the questions were difficult.",
            "definition": "A user asked Gemini for difficult questions.",
            "citations": [{"lsn": 30, "byte_start": 0, "byte_end": 47, "quote": "questions were difficult"}]
        }),
    )]);
    let decision = audit(&provider, b"run", &memory, &sources).unwrap();
    let HindsightDecision::Declined(reasons) = decision else {
        panic!("lossy rewrite was not declined");
    };
    assert!(
        reasons
            .iter()
            .any(|reason| reason.contains("dropped numbers"))
    );
    assert!(
        reasons
            .iter()
            .any(|reason| reason.contains("dropped quotations"))
    );
    assert!(reasons.iter().any(|reason| reason.contains("first person")));

    let provider = recorded_provider(vec![fixture(
        &request,
        &json!({
            "action": "revise",
            "concern": "The neighbour says the questions were difficult.",
            "definition": "I asked Gemini to generate 5 questions and quote \"be honest\"; the neighbour says the questions were difficult.",
            "citations": [
                {"lsn": 30, "byte_start": 0, "byte_end": 48, "quote": "questions were difficult"},
                {"lsn": 29, "byte_start": 0, "byte_end": 61, "quote": "be honest"}
            ]
        }),
    )]);
    let revision = audit(&provider, b"run", &memory, &sources).unwrap();
    let HindsightDecision::Revised(revision) = revision else {
        panic!("grounded revision was declined");
    };
    assert_eq!(revision.run_id, b"run");
    assert_eq!(revision.citations.len(), 2);
    assert_eq!(revision.authority, Authority::DerivedInference);
    assert_eq!(revision.model_provenance.prompt_version, 3);

    let faded = HindsightMemory {
        faded: true,
        ..memory
    };
    let unused = recorded_provider(Vec::new());
    assert_eq!(
        audit(&unused, b"run", &faded, &sources).unwrap(),
        HindsightDecision::NoChange
    );
}

#[test]
fn fsrs6_vectors_review_order_protection_and_fading_are_preserved() {
    assert!((initial_stability(ReviewRating::Again) - 0.4072).abs() < 1e-9);
    assert!((initial_stability(ReviewRating::Easy) - 15.4722).abs() < 1e-9);
    let fresh = FsrsData::default();
    let learned = schedule_next(fresh, ReviewRating::Good, 0.0);
    assert!((learned.stability - 3.1262).abs() < 1e-9);
    assert_eq!(learned.state, FsrsState::Learning);
    assert_eq!(learned.interval_days, 3);
    assert!((retrievability(3.1262, 3.1262) - 0.9).abs() < 1e-9);

    let protected_lsn = 100;
    let protection =
        ProtectionSet::from_sources([protected_lsn], [], [], [], &[vec![200], vec![201]], 1);
    let base = ReviewCandidate {
        memory_id: vec![1],
        retention: Retention::CurrentState,
        provenance_lsns: BTreeSet::from([1]),
        fsrs: FsrsData {
            stability: 0.1,
            difficulty: 7.0,
            repetitions: 2,
            lapses: 0,
            state: FsrsState::Review,
            last_review_ns: Some(1),
        },
        source_lsn: 1,
        signals: vec![AttestationSignal::Used],
        retrieval_rank: Some(10),
        contradiction_involved: false,
        dream_access_count: 99,
        faded: false,
    };
    let contradicted = ReviewCandidate {
        memory_id: vec![2],
        contradiction_involved: true,
        ..base.clone()
    };
    let protected = ReviewCandidate {
        memory_id: vec![3],
        provenance_lsns: BTreeSet::from([protected_lsn]),
        ..base.clone()
    };
    let do_not_store = ReviewCandidate {
        memory_id: vec![4],
        retention: Retention::DoNotStore,
        ..base.clone()
    };
    let decisions = schedule_reviews(
        &[base.clone(), contradicted, protected, do_not_store],
        &protection,
        1_000,
    );
    assert_eq!(decisions.len(), 2);
    assert_eq!(decisions[0].event.memory_id, vec![2]);
    assert_eq!(decisions[0].event.rating, ReviewRating::Again);
    assert_eq!(decisions[1].event.rating, ReviewRating::Good);

    let now = 40 * 86_400_000_000_000_i64;
    assert!(
        fading_event(
            &base,
            &protection,
            now,
            FadePolicy {
                maximum_retrievability_micros: 900_000
            }
        )
        .is_some()
    );
    let durable = ReviewCandidate {
        retention: Retention::Durable,
        ..base
    };
    assert!(
        fading_event(
            &durable,
            &protection,
            now,
            FadePolicy {
                maximum_retrievability_micros: 900_000
            }
        )
        .is_none()
    );

    let counterexamples = counterexample_protection(
        &[
            ContradictedOutcome {
                domain: "auth".to_owned(),
                observation_lsn: 1,
                observed_at_ns: 1,
            },
            ContradictedOutcome {
                domain: "auth".to_owned(),
                observation_lsn: 2,
                observed_at_ns: 2,
            },
            ContradictedOutcome {
                domain: "deploy".to_owned(),
                observation_lsn: 3,
                observed_at_ns: 1,
            },
        ],
        1,
    );
    assert_eq!(counterexamples, BTreeSet::from([2, 3]));
}

#[test]
fn phase_machine_resumes_budget_abort_is_clean_and_cadence_is_timezone_aware() {
    let id = run_id(b"actor scope", "daily", 7);
    assert_eq!(id, run_id(b"actor scope", "daily", 7));
    assert_ne!(id, run_id(b"actor scope", "daily", 8));
    let mut machine = PhaseMachine::resume(
        id.to_vec(),
        vec![
            ConsolidationPhaseName::Connect,
            ConsolidationPhaseName::Abstract,
        ],
        Vec::<PersistedPhase>::new(),
    );
    let connect = machine.next().unwrap();
    assert!(!connect.resumed);
    let started = machine.event(
        &connect,
        ConsolidationPhaseState::Started,
        Some(b"cursor-1".to_vec()),
        BudgetUsage::default(),
        0,
    );
    machine.record(&connect, &started);
    let resumed = machine.next().unwrap();
    assert!(resumed.resumed);
    assert_eq!(resumed.cursor, Some(b"cursor-1".to_vec()));
    let completed = machine.event(
        &resumed,
        ConsolidationPhaseState::Completed,
        None,
        BudgetUsage::default(),
        0,
    );
    assert_ne!(started.attempt_prefix, completed.attempt_prefix);
    assert_eq!(
        &started.attempt_prefix[..12],
        &completed.attempt_prefix[..12]
    );
    machine.record(&resumed, &completed);
    assert_eq!(
        machine.next().unwrap().phase,
        ConsolidationPhaseName::Abstract
    );

    let mut budget = BudgetTracker::new(
        ConsolidationBudget {
            max_llm_calls: 1,
            max_tokens: 120,
            max_microusd: 140,
            max_wall_ms: 10,
        },
        100,
    );
    budget
        .admit_call(
            Usage {
                input_tokens: 100,
                output_tokens: 20,
                cost_microusd: 140,
                ..Usage::default()
            },
            110,
        )
        .unwrap();
    let before = budget.usage();
    let error = budget
        .admit_call(
            Usage {
                input_tokens: 1,
                ..Usage::default()
            },
            110,
        )
        .unwrap_err();
    assert_eq!(error.dimension, BudgetDimension::LlmCalls);
    assert_eq!(budget.usage(), before);

    let before_dst = nanos("2026-03-29T00:59:00Z");
    let after_dst = nanos("2026-03-29T01:01:00Z");
    let cadence = Cadence {
        every_days: 1,
        local_hour: 2,
        local_minute: 30,
    };
    assert!(!due(None, before_dst, "Europe/Berlin", cadence).unwrap());
    assert!(due(None, after_dst, "Europe/Berlin", cadence).unwrap());
    assert_eq!(
        due(None, after_dst, "Mars/Olympus", cadence),
        Err(CadenceError::UnknownTimezone)
    );

    let retracted = retraction_event(id.to_vec(), 6, "operator rollback");
    assert_eq!(retracted.target_run_id, id);
    assert_eq!(retracted.previous_generation, 6);
}

fn nanos(value: &str) -> i64 {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .timestamp_nanos_opt()
        .unwrap()
}
