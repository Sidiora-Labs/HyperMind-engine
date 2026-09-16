#![forbid(unsafe_code)]

use hm_core::{ActorId, ErrorCode, LSN};
use hm_cortex::adjudicate::SUPERSESSION_PROMPT;
use hm_cortex::nli::NliModel;
use hm_llm::openai_compat::OpenAiCompatible;
use hm_llm::{
    ModelTier, Pricing, ProviderConfig, RecordedTransport, WireFixture, WireRequest, WireResponse,
};
use hm_mcp::{
    ActivateInput, BeliefClaimInput, BeliefTypeInput, BelieveInput, ClaimInput, DisputeInput,
    DisputeRuntime, McpServer, ProvenanceInput, RememberInput, RememberKind, RetractInput,
};
use hm_proj::beliefs::BeliefAsOf;
use hm_schema::events::{Assertion, AssertionClaim, BeliefType, ProvenanceRange};
use hm_serve::actor::{ActorConfig, ActorEngine};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

fn actor_config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("7"),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 32 * 1024 * 1024,
    }
}

fn provenance(lsn: u64, byte_end: u32) -> Vec<ProvenanceInput> {
    vec![ProvenanceInput {
        first_lsn: lsn,
        last_lsn: lsn,
        byte_start: 0,
        byte_end,
    }]
}

fn claim(
    id: &str,
    identity: &str,
    value: &str,
    valid_from_ns: i64,
    valid_to_ns: i64,
    evidence_lsn: u64,
) -> BeliefClaimInput {
    BeliefClaimInput {
        belief_id: id.to_owned(),
        belief_type: BeliefTypeInput::Fact,
        canonical_identity: identity.to_owned(),
        value: value.to_owned(),
        valid_from_ns,
        valid_to_ns,
        provenance: provenance(evidence_lsn, u32::try_from(value.len()).unwrap()),
        conflict_domain: Some("deployment:region".to_owned()),
        claim: ClaimInput::Affirmative,
    }
}

async fn evidence(server: &McpServer, conversation: &str) -> u64 {
    let result = server
        .remember_envelope(RememberInput {
            conversation: conversation.to_owned(),
            content: "observed deployment configuration".to_owned(),
            kind: RememberKind::User,
            chunk_bytes: None,
            anchor: None,
            retention: None,
            sensitivity: None,
            vocabulary: None,
            source: None,
            derive: None,
            source_delivery: None,
            source_settlement: None,
            document: None,
        })
        .await;
    assert!(result.ok);
    result.items[0]["first_lsn"].as_u64().unwrap()
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn believe_asof_retract_and_protected_proposal_use_real_ledger_projections() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());
    let evidence_lsn = evidence(&server, "belief-lifecycle").await;

    let first = server
        .believe_envelope(BelieveInput {
            conversation: "belief-lifecycle".to_owned(),
            belief: claim(
                "region-v1",
                "deployment:active",
                "Europe",
                1,
                100,
                evidence_lsn,
            ),
            run_id: None,
        })
        .await;
    assert!(first.ok);
    let first_lsn = first.items[0]["lsn"].as_u64().unwrap();
    let second = server
        .believe_envelope(BelieveInput {
            conversation: "belief-lifecycle".to_owned(),
            belief: claim(
                "region-v2",
                "deployment:active",
                "America",
                101,
                0,
                evidence_lsn,
            ),
            run_id: None,
        })
        .await;
    assert!(second.ok);
    let second_lsn = second.items[0]["lsn"].as_u64().unwrap();

    let valid = actor
        .as_of(
            BeliefType::Fact,
            "deployment:active".to_owned(),
            BeliefAsOf::ValidAt(50),
        )
        .await
        .unwrap()
        .record
        .unwrap();
    let known = actor
        .as_of(
            BeliefType::Fact,
            "deployment:active".to_owned(),
            BeliefAsOf::KnownAt(LSN::new(second_lsn)),
        )
        .await
        .unwrap()
        .record
        .unwrap();
    assert_eq!(valid.value, b"Europe");
    assert_eq!(known.value, b"America");

    let rejected = server
        .believe_envelope(BelieveInput {
            conversation: "belief-lifecycle".to_owned(),
            belief: BeliefClaimInput {
                belief_id: "run-name".to_owned(),
                belief_type: BeliefTypeInput::Identity,
                canonical_identity: "self:name".to_owned(),
                value: "HyperMind".to_owned(),
                valid_from_ns: 0,
                valid_to_ns: 0,
                provenance: provenance(evidence_lsn, 9),
                conflict_domain: Some("self".to_owned()),
                claim: ClaimInput::Affirmative,
            },
            run_id: Some("run-5".to_owned()),
        })
        .await;
    assert!(!rejected.ok);
    assert_eq!(
        rejected.items[0]["error"],
        ErrorCode::ProtectedTypeWrite.as_str()
    );
    assert_eq!(rejected.effect_state.as_deref(), Some("rejected"));
    let activated = server
        .activate_envelope(ActivateInput {
            conversation: "belief-lifecycle".to_owned(),
            query: "identity".to_owned(),
            turn_text: String::new(),
            budget_tokens: 4096,
        })
        .await;
    assert!(activated.ok);
    assert!(activated.items.iter().any(|item| {
        item["tier"] == "conflicts"
            && item["uri"]
                .as_str()
                .is_some_and(|uri| uri.contains("src=proposal"))
    }));

    let retracted = server
        .retract_envelope(RetractInput {
            conversation: "belief-lifecycle".to_owned(),
            belief_id: "region-v2".to_owned(),
            provenance: provenance(evidence_lsn, 8),
        })
        .await;
    assert!(retracted.ok);
    assert_eq!(retracted.items[0]["tombstoned"], true);
    assert!(
        actor
            .as_of(
                BeliefType::Fact,
                "deployment:active".to_owned(),
                BeliefAsOf::KnownAt(LSN::new(second_lsn + 2)),
            )
            .await
            .unwrap()
            .record
            .is_none()
    );
    assert_eq!(
        actor
            .as_of(
                BeliefType::Fact,
                "deployment:active".to_owned(),
                BeliefAsOf::KnownAt(LSN::new(first_lsn)),
            )
            .await
            .unwrap()
            .record
            .unwrap()
            .value,
        b"Europe"
    );

    drop(server);
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn negative_existence_rejection_is_pre_dispatch() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());
    let evidence_lsn = evidence(&server, "negative").await;
    let mut negative = claim(
        "missing",
        "deployment:missing",
        "No deployment exists",
        0,
        0,
        evidence_lsn,
    );
    negative.claim = ClaimInput::NegativeExistence;
    let result = server
        .believe_envelope(BelieveInput {
            conversation: "negative".to_owned(),
            belief: negative,
            run_id: None,
        })
        .await;
    assert!(!result.ok);
    assert_eq!(
        result.items[0]["error"],
        ErrorCode::NegativeExistenceUncorroborated.as_str()
    );
    assert_eq!(result.effect_state.as_deref(), Some("rejected"));
    assert_eq!(actor.stats().await.unwrap().log_events, 1);
    drop(server);
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn dispute_uses_real_bidirectional_nli_and_recorded_provider_then_surfaces_conflict() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let evidence_server = McpServer::new(actor.clone());
    let evidence_lsn = evidence(&evidence_server, "dispute").await;
    let existing = claim(
        "europe",
        "deployment:region:europe",
        "The deployment region is Europe.",
        10,
        0,
        evidence_lsn,
    );
    let incoming = claim(
        "america",
        "deployment:region:america",
        "The deployment region is America.",
        20,
        0,
        evidence_lsn,
    );
    let asserted = evidence_server
        .believe_envelope(BelieveInput {
            conversation: "dispute".to_owned(),
            belief: existing.clone(),
            run_id: None,
        })
        .await;
    assert!(asserted.ok);
    drop(evidence_server);

    let provider = recorded_provider(&existing, &incoming);
    let cache = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/hm-models");
    let runtime = DisputeRuntime::new(
        Arc::new(NliModel::download(&cache).unwrap()),
        Some(Arc::new(provider)),
        ModelTier::Capable,
    );
    let server = McpServer::new(actor.clone()).with_dispute_runtime(runtime);
    let report = server
        .dispute_envelope(DisputeInput {
            conversation: "dispute".to_owned(),
            existing,
            incoming,
        })
        .await;
    assert!(report.ok);
    assert_eq!(report.items[0]["verdict"], "genuine");
    assert_eq!(report.items[0]["suggested_action"], "surface_conflict");
    assert_eq!(
        report.items[0]["resulting_events"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let snapshot = actor
        .as_of(
            BeliefType::Fact,
            "deployment:region:america".to_owned(),
            BeliefAsOf::KnownAt(LSN::new(actor.stats().await.unwrap().log_events)),
        )
        .await
        .unwrap()
        .record
        .unwrap();
    assert!(snapshot.conflict_edges.iter().any(|edge| {
        edge.other_canonical_identity == "deployment:region:europe" && edge.obligated_surfacing
    }));
    drop(server);
    actor.shutdown().await.unwrap();
}

fn recorded_provider(
    existing: &BeliefClaimInput,
    incoming: &BeliefClaimInput,
) -> OpenAiCompatible<RecordedTransport> {
    let schema = json!({
        "type": "object",
        "properties": {
            "supersedes": {"type": "boolean"},
            "reason": {"type": "string"}
        },
        "required": ["supersedes", "reason"],
        "additionalProperties": false
    });
    let existing = assertion(existing);
    let incoming = assertion(incoming);
    let prompt = format!(
        "existing_identity: {}\nexisting_valid: {}..{}\nexisting_value: {}\n\nincoming_identity: {}\nincoming_valid: {}..{}\nincoming_value: {}",
        existing.canonical_identity,
        existing.valid_from_ns,
        existing.valid_to_ns,
        std::str::from_utf8(&existing.value).unwrap(),
        incoming.canonical_identity,
        incoming.valid_from_ns,
        incoming.valid_to_ns,
        std::str::from_utf8(&incoming.value).unwrap(),
    );
    let fixture = WireFixture {
        request: WireRequest {
            method: "POST".to_owned(),
            url: "https://fixture.invalid/v1/chat/completions".to_owned(),
            headers: BTreeMap::from([
                ("authorization".to_owned(), "Bearer fixture-key".to_owned()),
                ("content-type".to_owned(), "application/json".to_owned()),
            ]),
            body: json!({
                "model": "fixture-capable",
                "messages": [
                    {"role": "system", "content": SUPERSESSION_PROMPT},
                    {"role": "user", "content": prompt}
                ],
                "max_tokens": 256,
                "response_format": {
                    "type": "json_schema",
                    "json_schema": {
                        "name": "supersession_1",
                        "strict": true,
                        "schema": schema
                    }
                }
            }),
        },
        response: WireResponse {
            status: 200,
            body: json!({
                "choices": [{"message": {"content": serde_json::to_string(&json!({
                    "supersedes": false,
                    "reason": "claims remain in conflict"
                })).unwrap()}}],
                "usage": {"prompt_tokens": 30, "completion_tokens": 12}
            }),
        },
    };
    OpenAiCompatible::new(
        ProviderConfig {
            endpoint: "https://fixture.invalid/v1/chat/completions".to_owned(),
            api_key: Some("fixture-key".to_owned()),
            model: "fixture-capable".to_owned(),
            tier: ModelTier::Capable,
            pricing: Pricing::default(),
        },
        RecordedTransport::new(vec![fixture]),
    )
    .unwrap()
}

fn assertion(input: &BeliefClaimInput) -> Assertion {
    Assertion {
        belief_id: input.belief_id.as_bytes().to_vec(),
        belief_type: BeliefType::Fact,
        canonical_identity: input.canonical_identity.clone(),
        value: input.value.as_bytes().to_vec(),
        valid_from_ns: input.valid_from_ns,
        valid_to_ns: input.valid_to_ns,
        provenance: input
            .provenance
            .iter()
            .map(|range| ProvenanceRange {
                first_lsn: range.first_lsn,
                last_lsn: range.last_lsn,
                byte_start: range.byte_start,
                byte_end: range.byte_end,
            })
            .collect(),
        conflict_domain: input.conflict_domain.clone(),
        claim: AssertionClaim::Affirmative,
    }
}
