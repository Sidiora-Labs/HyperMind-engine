#![forbid(unsafe_code)]
#![allow(clippy::too_many_lines)]

use hm_core::{ActorId, ConversationId, ErrorCode, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_proj::fsrs::FsrsProjection;
use hm_proj::generation::GenerationProjection;
use hm_proj::graph::GraphProjection;
use hm_proj::lease::LeaseManager;
use hm_proj::memories::MemoryProjection;
use hm_proj::rebuild::rebuild_projection_stream;
use hm_proj::runs::{RunStatus, RunsProjection};
use hm_proj::store::{ProjectionId, ProjectionStore};
use hm_schema::event::encode_event_envelope;
use hm_schema::events::{
    Authority, ConsolidationBudget, ConsolidationClosed, ConsolidationOpened,
    ConsolidationRetracted, EdgeAsserted, EventEnvelope, EventPayload, MemoryFadeReason,
    MemoryFaded, MemoryMinted, MemoryRevised, ModelProvenance, PromptVersion, ProvenanceRange,
    Retention, ReviewRating, Reviewed, Sensitivity,
};
use tempfile::tempdir;

fn citation() -> Vec<ProvenanceRange> {
    vec![ProvenanceRange {
        first_lsn: 1,
        last_lsn: 1,
        byte_start: 0,
        byte_end: 8,
    }]
}

fn model(call: u8) -> ModelProvenance {
    ModelProvenance {
        model_id: "fixture-model".to_owned(),
        prompt_id: "merge-cluster".to_owned(),
        prompt_version: 1,
        temperature: 0.0,
        call_id: Some(vec![call]),
        input_tokens: 20,
        output_tokens: 5,
        cache_read_tokens: 0,
        cache_write_tokens: 0,
        cost_microusd: 7,
    }
}

fn frame(
    lsn: u64,
    kind: EventKind,
    run_id: &[u8],
    payload: EventPayload,
    model_provenance: Option<ModelProvenance>,
) -> Frame {
    Frame {
        header: FrameHeader {
            lsn: LSN::new(lsn),
            kind,
            wall_timestamp_ns: UtcNanos::new(i64::try_from(lsn).unwrap() * 1_000),
            actor: ActorId::new(7),
            conversation: ConversationId::new([0x61; 16]),
        },
        sealed_payload: encode_event_envelope(&EventEnvelope {
            schema_version: 2,
            payload,
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 0,
            origin_actor: 0,
            run_id: Some(run_id.to_vec()),
            model_provenance: model_provenance.map(Box::new),
            authority: Authority::DerivedInference,
            retention: Retention::Durable,
            sensitivity: Sensitivity::Personal,
            event_time_ns: i64::try_from(lsn).unwrap() * 1_000,
        }),
    }
}

fn opened(lsn: u64, run_id: &[u8], generation: u64, parent: u64) -> Frame {
    frame(
        lsn,
        EventKind::ConsolidationOpened,
        run_id,
        EventPayload::ConsolidationOpened(Box::new(ConsolidationOpened {
            scope_digest: vec![u8::try_from(generation).unwrap(); 32],
            cadence_key: format!("generation-{generation}"),
            generation,
            expected_active_generation: parent,
            phases: vec![hm_schema::events::ConsolidationPhaseName::Nrem],
            prompts: vec![PromptVersion {
                prompt_id: "merge-cluster".to_owned(),
                version: 1,
                model_id: "fixture-model".to_owned(),
            }],
            budget: Box::new(ConsolidationBudget {
                max_llm_calls: 10,
                max_tokens: 10_000,
                max_microusd: 1_000,
                max_wall_ms: 30_000,
            }),
        })),
        None,
    )
}

fn closed(lsn: u64, run_id: &[u8], generation: u64, parent: u64, records: u64) -> Frame {
    frame(
        lsn,
        EventKind::ConsolidationClosed,
        run_id,
        EventPayload::ConsolidationClosed(Box::new(ConsolidationClosed {
            generation,
            expected_active_generation: parent,
            derived_records: records,
            dropped_candidates: 0,
            llm_calls: records,
            input_tokens: records * 20,
            output_tokens: records * 5,
            cost_microusd: records * 7,
        })),
        None,
    )
}

fn minted(lsn: u64, run_id: &[u8], definition: &[u8]) -> Frame {
    frame(
        lsn,
        EventKind::MemoryMinted,
        run_id,
        EventPayload::MemoryMinted(Box::new(MemoryMinted {
            memory_id: b"memory-1".to_vec(),
            name: "Deployment".to_owned(),
            definition: definition.to_vec(),
            tags: vec!["deployment".to_owned()],
            salience_micros: 800_000,
            citations: citation(),
        })),
        Some(model(1)),
    )
}

#[test]
fn generations_publish_atomically_lease_distinct_views_and_retract_overrides_leases() {
    let directory = tempdir().unwrap();
    let store = ProjectionStore::open(directory.path(), 128 * 1024 * 1024).unwrap();
    let run_one = b"run-one";
    let first = [
        opened(1, run_one, 1, 0),
        minted(2, run_one, b"deployed in eu-central"),
        frame(
            3,
            EventKind::EdgeAsserted,
            run_one,
            EventPayload::EdgeAsserted(Box::new(EdgeAsserted {
                edge_id: b"edge-1".to_vec(),
                source_id: b"memory-1".to_vec(),
                target_id: b"memory-2".to_vec(),
                relation: "supports".to_owned(),
                weight_micros: 500_000,
                valid_from_ns: 1,
                valid_to_ns: 0,
                citations: citation(),
            })),
            Some(model(2)),
        ),
        frame(
            4,
            EventKind::Reviewed,
            run_one,
            EventPayload::Reviewed(Box::new(Reviewed {
                memory_id: b"memory-1".to_vec(),
                rating: ReviewRating::Good,
                source_lsn: 2,
                reviewed_at_ns: 4_000,
                stability_millis: 86_400_000,
                difficulty_micros: 400_000,
                due_at_ns: 8_000,
            })),
            None,
        ),
        closed(5, run_one, 1, 0, 3),
    ];
    for event in &first[..2] {
        GenerationProjection::apply_event(&store, event).unwrap();
    }
    let staged = store.begin_snapshot().unwrap();
    assert_eq!(GenerationProjection::active_generation(&staged).unwrap(), 0);
    assert_eq!(
        MemoryProjection::get_visible(&staged, 0, b"memory-1").unwrap(),
        None
    );
    drop(staged);
    for event in &first[2..] {
        GenerationProjection::apply_event(&store, event).unwrap();
    }

    let leases = LeaseManager::default();
    let snapshot = store.begin_snapshot().unwrap();
    assert_eq!(
        GenerationProjection::active_generation(&snapshot).unwrap(),
        1
    );
    let lease_one = leases.lease(&snapshot, 1, 10_000, 60).unwrap();
    assert_eq!(
        MemoryProjection::get_visible(&snapshot, lease_one.generation, b"memory-1")
            .unwrap()
            .unwrap()
            .definition,
        b"deployed in eu-central"
    );
    assert_eq!(
        GraphProjection::neighbours(&snapshot, 1, b"memory-1", 5_000, 10)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        FsrsProjection::get_visible(&snapshot, 1, b"memory-1")
            .unwrap()
            .unwrap()
            .rating,
        ReviewRating::Good
    );
    drop(snapshot);

    let run_two = b"run-two";
    let second = [
        opened(6, run_two, 2, 1),
        frame(
            7,
            EventKind::MemoryRevised,
            run_two,
            EventPayload::MemoryRevised(Box::new(MemoryRevised {
                memory_id: b"memory-1".to_vec(),
                previous_lsn: 2,
                name: "Deployment".to_owned(),
                definition: b"deployed in us-east".to_vec(),
                tags: vec!["deployment".to_owned()],
                salience_micros: 900_000,
                citations: citation(),
            })),
            Some(model(3)),
        ),
        closed(8, run_two, 2, 1, 1),
    ];
    for event in &second {
        GenerationProjection::apply_event(&store, event).unwrap();
    }
    GenerationProjection::apply_event(&store, &second[2]).unwrap();

    let snapshot = store.begin_snapshot().unwrap();
    let lease_two = leases.lease(&snapshot, 2, 20_000, 60).unwrap();
    assert_eq!(
        MemoryProjection::get_visible(&snapshot, lease_one.generation, b"memory-1")
            .unwrap()
            .unwrap()
            .definition,
        b"deployed in eu-central"
    );
    assert_eq!(
        MemoryProjection::get_visible(&snapshot, lease_two.generation, b"memory-1")
            .unwrap()
            .unwrap()
            .definition,
        b"deployed in us-east"
    );
    let published = RunsProjection::run(&snapshot, run_two).unwrap().unwrap();
    assert_eq!(published.status, RunStatus::Published);
    assert_eq!(published.published_lsn, 8);
    assert_eq!(published.staged.len(), 1);
    drop(snapshot);

    let run_three = b"run-three";
    for event in [
        opened(9, run_three, 3, 2),
        frame(
            10,
            EventKind::MemoryFaded,
            run_three,
            EventPayload::MemoryFaded(Box::new(MemoryFaded {
                memory_id: b"memory-1".to_vec(),
                reason: MemoryFadeReason::LowRetrievability,
                evidence_lsns: Some(vec![4]),
            })),
            None,
        ),
        closed(11, run_three, 3, 2, 1),
    ] {
        GenerationProjection::apply_event(&store, &event).unwrap();
    }
    let snapshot = store.begin_snapshot().unwrap();
    assert!(
        MemoryProjection::get_visible(&snapshot, lease_one.generation, b"memory-1")
            .unwrap()
            .is_none()
    );
    assert!(
        MemoryProjection::get_visible(&snapshot, lease_two.generation, b"memory-1")
            .unwrap()
            .is_none()
    );
    drop(snapshot);

    for event in [
        frame(
            12,
            EventKind::ConsolidationRetracted,
            run_three,
            EventPayload::ConsolidationRetracted(Box::new(ConsolidationRetracted {
                target_run_id: run_three.to_vec(),
                previous_generation: 2,
                reason: "rollback fading".to_owned(),
            })),
            None,
        ),
        frame(
            13,
            EventKind::ConsolidationRetracted,
            run_two,
            EventPayload::ConsolidationRetracted(Box::new(ConsolidationRetracted {
                target_run_id: run_two.to_vec(),
                previous_generation: 1,
                reason: "rollback revision".to_owned(),
            })),
            None,
        ),
    ] {
        GenerationProjection::apply_event(&store, &event).unwrap();
    }
    let snapshot = store.begin_snapshot().unwrap();
    assert_eq!(
        GenerationProjection::active_generation(&snapshot).unwrap(),
        1
    );
    assert_eq!(
        MemoryProjection::get_visible(&snapshot, lease_two.generation, b"memory-1")
            .unwrap()
            .unwrap()
            .definition,
        b"deployed in eu-central"
    );
    assert_eq!(
        leases.resolve(lease_one.lease_id, 30_000).unwrap(),
        lease_one
    );
    assert_eq!(
        leases.resolve(lease_two.lease_id, 30_000).unwrap(),
        lease_two
    );
}

#[test]
fn publish_refuses_missing_projection_and_resumes_after_partial_apply() {
    let directory = tempdir().unwrap();
    let store = ProjectionStore::open(directory.path(), 64 * 1024 * 1024).unwrap();
    let run_id = b"partial-run";
    let open = opened(1, run_id, 1, 0);
    GenerationProjection::apply_event(&store, &open).unwrap();
    let edge = frame(
        2,
        EventKind::EdgeAsserted,
        run_id,
        EventPayload::EdgeAsserted(Box::new(EdgeAsserted {
            edge_id: b"edge-partial".to_vec(),
            source_id: b"one".to_vec(),
            target_id: b"two".to_vec(),
            relation: "related".to_owned(),
            weight_micros: 500_000,
            valid_from_ns: 1,
            valid_to_ns: 0,
            citations: citation(),
        })),
        Some(model(9)),
    );
    MemoryProjection::apply_event(&store, &edge).unwrap();
    FsrsProjection::apply_event(&store, &edge).unwrap();
    RunsProjection::apply_event(&store, &edge).unwrap();

    let close = closed(3, run_id, 1, 0, 1);
    MemoryProjection::apply_event(&store, &close).unwrap();
    FsrsProjection::apply_event(&store, &close).unwrap();
    assert_eq!(
        RunsProjection::apply_event(&store, &close)
            .expect_err("graph marker is absent")
            .code,
        ErrorCode::ProjectionCheckpoint
    );
    assert_eq!(
        GenerationProjection::active_generation(&store.begin_snapshot().unwrap()).unwrap(),
        0
    );

    GraphProjection::apply_event(&store, &edge).unwrap();
    GraphProjection::apply_event(&store, &close).unwrap();
    RunsProjection::apply_event(&store, &close).unwrap();
    let snapshot = store.begin_snapshot().unwrap();
    assert_eq!(
        GenerationProjection::active_generation(&snapshot).unwrap(),
        1
    );
    assert_eq!(
        GraphProjection::neighbours(&snapshot, 1, b"one", 5, 10)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn generation_projections_rebuild_byte_identically() {
    let directory = tempdir().unwrap();
    let store = ProjectionStore::open(directory.path(), 64 * 1024 * 1024).unwrap();
    let run_id = b"rebuild-run";
    let frames = vec![
        opened(1, run_id, 1, 0),
        minted(2, run_id, b"canonical definition"),
        closed(3, run_id, 1, 0, 1),
    ];
    rebuild_projection_stream(&store, &frames, true, usize::MAX).unwrap();
    let first = {
        let snapshot = store.begin_snapshot().unwrap();
        [
            snapshot.canonical_dump(ProjectionId::Memories).unwrap(),
            snapshot.canonical_dump(ProjectionId::Graph).unwrap(),
            snapshot.canonical_dump(ProjectionId::Fsrs).unwrap(),
            snapshot.canonical_dump(ProjectionId::Runs).unwrap(),
        ]
    };
    rebuild_projection_stream(&store, &frames, true, usize::MAX).unwrap();
    let snapshot = store.begin_snapshot().unwrap();
    let second = [
        snapshot.canonical_dump(ProjectionId::Memories).unwrap(),
        snapshot.canonical_dump(ProjectionId::Graph).unwrap(),
        snapshot.canonical_dump(ProjectionId::Fsrs).unwrap(),
        snapshot.canonical_dump(ProjectionId::Runs).unwrap(),
    ];
    assert_eq!(first, second);
}
