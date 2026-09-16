#![forbid(unsafe_code)]
#![allow(clippy::too_many_lines)]

use hm_compose::bundle::RetrievalLane;
use hm_compose::fusion::{LaneRanking, RankedCandidate, fuse};
use hm_compose::lanes::graph::{GraphSeed, MAXIMUM_VISITED_NODES, search};
use hm_core::{ActorId, ConversationId, ErrorCode, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_mcp::{
    AttestDisposition, AttestInput, ConsolidateAction, ConsolidateBudget, ConsolidateInput,
    ConsolidateMode, McpServer, RememberInput, RememberKind,
};
use hm_proj::generation::GenerationProjection;
use hm_proj::store::ProjectionStore;
use hm_schema::event::{Boundary, encode_event_envelope, verify_event};
use hm_schema::events::{
    AttestationDisposition as StoredDisposition, Authority, ConsolidationBudget,
    ConsolidationClosed, ConsolidationOpened, ConsolidationPhaseName, EdgeAsserted, EventEnvelope,
    EventPayload, ModelProvenance, PromptVersion, ProvenanceRange, Retention, Sensitivity,
};
use hm_serve::actor::{ActorConfig, ActorEngine};
use std::collections::BTreeSet;

fn actor_config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("7"),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 64 * 1024 * 1024,
    }
}

fn consolidate(action: ConsolidateAction) -> ConsolidateInput {
    ConsolidateInput {
        action,
        mode: None,
        scope: None,
        cadence_key: None,
        budget: None,
        run_id: None,
        reason: None,
    }
}

#[tokio::test]
async fn attest_and_consolidate_complete_real_ledger_lifecycles() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());
    let remembered = server
        .remember_envelope(RememberInput {
            conversation: "sleep-lifecycle".to_owned(),
            content: "deployment evidence".to_owned(),
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
    let source = remembered.items[0]["first_lsn"].as_u64().unwrap();
    let provenance = vec![format!("hm://7/lsn/{source}")];
    for (index, disposition) in [
        AttestDisposition::Used,
        AttestDisposition::Ignored,
        AttestDisposition::Helpful,
        AttestDisposition::Harmful,
    ]
    .into_iter()
    .enumerate()
    {
        let result = server
            .attest_envelope(AttestInput {
                provenance: provenance.clone(),
                disposition,
                idempotency_key: format!("attestation-{index}"),
            })
            .await;
        assert!(result.ok, "{result:?}");
        assert_eq!(result.items[0]["targets"], serde_json::json!([source]));
        assert_eq!(
            result.items[0]["manifest_used"].is_array(),
            matches!(
                disposition,
                AttestDisposition::Used | AttestDisposition::Helpful
            )
        );
    }
    let duplicate = server
        .attest_envelope(AttestInput {
            provenance: provenance.clone(),
            disposition: AttestDisposition::Helpful,
            idempotency_key: "attestation-2".to_owned(),
        })
        .await;
    assert!(duplicate.ok);
    assert_eq!(duplicate.items[0]["duplicate"], true);

    let frames = actor.frames_since(LSN::new(source), None, 4).await.unwrap();
    let dispositions = frames
        .iter()
        .map(|frame| {
            let verified = verify_event(
                &frame.sealed_payload,
                hm_schema::event::EventKind::Attestation,
                Boundary::Disk,
            )
            .unwrap();
            let EventPayload::Attestation(value) = verified.envelope.payload else {
                panic!("expected attestation")
            };
            value.disposition
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        dispositions,
        BTreeSet::from([
            StoredDisposition::Used,
            StoredDisposition::Ignored,
            StoredDisposition::Helpful,
            StoredDisposition::Harmful,
        ])
    );

    let mut run_ids = Vec::new();
    for (index, mode) in [
        ConsolidateMode::Nrem,
        ConsolidateMode::Rem,
        ConsolidateMode::Both,
    ]
    .into_iter()
    .enumerate()
    {
        let input = ConsolidateInput {
            action: ConsolidateAction::Run,
            mode: Some(mode),
            scope: Some("sleep-lifecycle".to_owned()),
            cadence_key: Some(format!("night-{index}")),
            budget: Some(ConsolidateBudget {
                max_llm_calls: 8,
                max_tokens: 16_000,
                max_microusd: 50_000,
                max_wall_ms: 30_000,
            }),
            run_id: None,
            reason: None,
        };
        let result = server.consolidate_envelope(input.clone()).await;
        assert!(result.ok, "{result:?}");
        assert_eq!(result.items[0]["status"], "published");
        assert_eq!(result.items[0]["generation"], (index + 1) as u64);
        let replay = server.consolidate_envelope(input).await;
        assert_eq!(replay.items[0]["run_id"], result.items[0]["run_id"]);
        assert_eq!(replay.items[0]["first_lsn"], result.items[0]["first_lsn"]);
        assert_eq!(replay.items[0]["duplicate"], true);
        run_ids.push(result.items[0]["run_id"].as_str().unwrap().to_owned());
    }
    let listed = server
        .consolidate_envelope(consolidate(ConsolidateAction::List))
        .await;
    assert!(listed.ok);
    assert_eq!(listed.items.len(), 3);
    assert!(listed.items.iter().all(|item| item["cost"].is_object()));
    assert!(listed.items.iter().all(|item| item["stats"].is_object()));

    let retracted = server
        .consolidate_envelope(ConsolidateInput {
            action: ConsolidateAction::Retract,
            run_id: run_ids.last().cloned(),
            reason: Some("operator rollback".to_owned()),
            ..consolidate(ConsolidateAction::Retract)
        })
        .await;
    assert!(retracted.ok, "{retracted:?}");
    assert_eq!(retracted.items[0]["active_generation"], 2);
    let listed = server
        .consolidate_envelope(consolidate(ConsolidateAction::List))
        .await;
    assert_eq!(
        listed
            .items
            .iter()
            .find(|item| item["run_id"] == run_ids[2])
            .unwrap()["status"],
        "retracted"
    );

    drop(server);
    actor.shutdown().await.unwrap();
}

#[test]
fn graph_lane_obeys_hops_validity_query_cap_and_fuses() {
    let temporary = tempfile::tempdir().unwrap();
    let store = ProjectionStore::open(temporary.path(), 64 * 1024 * 1024).unwrap();
    let run = b"graph-run";
    let frames = [
        opened(1, run),
        edge(2, run, b"a", b"b", "deployment dependency", 0),
        edge(3, run, b"b", b"c", "deployment dependency", 0),
        edge(4, run, b"b", b"d", "unrelated topic", 0),
        edge(5, run, b"a", b"expired", "deployment dependency", 50),
        closed(6, run),
    ];
    for frame in &frames {
        GenerationProjection::apply_event(&store, frame).unwrap();
    }
    let snapshot = store.begin_snapshot().unwrap();
    let ranking = search(
        &snapshot,
        1,
        &[GraphSeed {
            node_id: b"a".to_vec(),
            source_lsn: LSN::new(1),
        }],
        "deployment dependency",
        100,
        2,
        10,
    )
    .unwrap();
    assert_eq!(
        ranking
            .candidates
            .iter()
            .map(|candidate| candidate.canonical_id.as_slice())
            .collect::<Vec<_>>(),
        vec![b"b".as_slice(), b"c".as_slice()]
    );
    let lexical = LaneRanking {
        lane: RetrievalLane::Lexical,
        weight_q16: 1 << 16,
        candidates: vec![RankedCandidate::neutral(b"c".to_vec(), LSN::new(3))],
    };
    let fused = fuse(
        ActorId::new(7),
        ConversationId::derive("graph-test"),
        &[ranking, lexical],
        10,
    )
    .unwrap();
    let joined = fused
        .iter()
        .find(|candidate| candidate.canonical_id == b"c")
        .unwrap();
    assert_eq!(joined.lane_ranks.len(), 2);

    let oversized = (0..=MAXIMUM_VISITED_NODES)
        .map(|index| GraphSeed {
            node_id: index.to_le_bytes().to_vec(),
            source_lsn: LSN::new(1),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        search(&snapshot, 1, &oversized, "deployment", 100, 1, 10)
            .unwrap_err()
            .code,
        ErrorCode::CapacityExceeded
    );
}

fn opened(lsn: u64, run: &[u8]) -> Frame {
    frame(
        lsn,
        EventKind::ConsolidationOpened,
        run,
        EventPayload::ConsolidationOpened(Box::new(ConsolidationOpened {
            scope_digest: vec![1; 32],
            cadence_key: "graph-night".to_owned(),
            generation: 1,
            expected_active_generation: 0,
            phases: vec![ConsolidationPhaseName::Nrem],
            prompts: vec![PromptVersion {
                prompt_id: "merge-cluster".to_owned(),
                version: 1,
                model_id: "fixture-model".to_owned(),
            }],
            budget: Box::new(ConsolidationBudget {
                max_llm_calls: 10,
                max_tokens: 10_000,
                max_microusd: 10_000,
                max_wall_ms: 30_000,
            }),
            source_first_lsn: 0,
            source_last_lsn: 0,
        })),
        None,
    )
}

fn edge(lsn: u64, run: &[u8], source: &[u8], target: &[u8], relation: &str, until: i64) -> Frame {
    frame(
        lsn,
        EventKind::EdgeAsserted,
        run,
        EventPayload::EdgeAsserted(Box::new(EdgeAsserted {
            edge_id: format!("edge-{lsn}").into_bytes(),
            source_id: source.to_vec(),
            target_id: target.to_vec(),
            relation: relation.to_owned(),
            weight_micros: 500_000,
            valid_from_ns: 1,
            valid_to_ns: until,
            citations: vec![ProvenanceRange {
                first_lsn: 1,
                last_lsn: 1,
                byte_start: 0,
                byte_end: 1,
            }],
        })),
        Some(ModelProvenance {
            model_id: "fixture-model".to_owned(),
            prompt_id: "connect-long-context".to_owned(),
            prompt_version: 1,
            temperature: 0.0,
            call_id: Some(vec![u8::try_from(lsn).unwrap()]),
            input_tokens: 10,
            output_tokens: 2,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            cost_microusd: 5,
        }),
    )
}

fn closed(lsn: u64, run: &[u8]) -> Frame {
    frame(
        lsn,
        EventKind::ConsolidationClosed,
        run,
        EventPayload::ConsolidationClosed(Box::new(ConsolidationClosed {
            generation: 1,
            expected_active_generation: 0,
            derived_records: 4,
            dropped_candidates: 0,
            llm_calls: 1,
            input_tokens: 10,
            output_tokens: 2,
            cost_microusd: 5,
        })),
        None,
    )
}

fn frame(
    lsn: u64,
    kind: EventKind,
    run: &[u8],
    payload: EventPayload,
    model_provenance: Option<ModelProvenance>,
) -> Frame {
    Frame {
        header: FrameHeader {
            lsn: LSN::new(lsn),
            kind,
            wall_timestamp_ns: UtcNanos::new(i64::try_from(lsn).unwrap()),
            actor: ActorId::new(7),
            conversation: ConversationId::derive("graph-test"),
        },
        sealed_payload: encode_event_envelope(&EventEnvelope {
            schema_version: 2,
            payload,
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 0,
            origin_actor: 0,
            run_id: Some(run.to_vec()),
            model_provenance: model_provenance.map(Box::new),
            authority: Authority::DerivedInference,
            retention: Retention::Durable,
            sensitivity: Sensitivity::Personal,
            event_time_ns: i64::try_from(lsn).unwrap(),
        }),
    }
}
