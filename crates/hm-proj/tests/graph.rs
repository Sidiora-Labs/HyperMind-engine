#![forbid(unsafe_code)]
#![allow(clippy::too_many_lines)]

use hm_core::{ActorId, ConversationId, ErrorCode, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_proj::generation::GenerationProjection;
use hm_proj::graph::{EdgeCitation, GraphProjection};
use hm_proj::store::{ProjectionId, ProjectionStore};
use hm_schema::event::encode_event_envelope;
use hm_schema::events::{
    Authority, ConsolidationBudget, ConsolidationClosed, ConsolidationOpened, EdgeAsserted,
    EdgeRetracted, EventEnvelope, EventPayload, ModelProvenance, PromptVersion, ProvenanceRange,
    Retention, Sensitivity,
};
use tempfile::tempdir;

fn conversation() -> ConversationId {
    let mut bytes = [0_u8; 16];
    bytes[0] = 1;
    ConversationId::new(bytes)
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
            conversation: conversation(),
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

fn model(call: u8) -> ModelProvenance {
    ModelProvenance {
        model_id: "fixture-model".to_owned(),
        prompt_id: "connect-long-context".to_owned(),
        prompt_version: 1,
        temperature: 0.0,
        call_id: Some(vec![call]),
        input_tokens: 10,
        output_tokens: 2,
        cache_read_tokens: 0,
        cache_write_tokens: 0,
        cost_microusd: 5,
    }
}

fn citations(lsn: u64) -> Vec<ProvenanceRange> {
    vec![ProvenanceRange {
        first_lsn: lsn,
        last_lsn: lsn,
        byte_start: 0,
        byte_end: 16,
    }]
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
                prompt_id: "connect-long-context".to_owned(),
                version: 1,
                model_id: "fixture-model".to_owned(),
            }],
            budget: Box::new(ConsolidationBudget {
                max_llm_calls: 10,
                max_tokens: 10_000,
                max_microusd: 1_000,
                max_wall_ms: 30_000,
            }),
            source_first_lsn: 0,
            source_last_lsn: 0,
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
            input_tokens: records * 10,
            output_tokens: records * 2,
            cost_microusd: records * 5,
        })),
        None,
    )
}

fn asserted(lsn: u64, run_id: &[u8], edge_id: &[u8], relation: &str, valid_to_ns: i64) -> Frame {
    frame(
        lsn,
        EventKind::EdgeAsserted,
        run_id,
        EventPayload::EdgeAsserted(Box::new(EdgeAsserted {
            edge_id: edge_id.to_vec(),
            source_id: b"memory-source".to_vec(),
            target_id: b"memory-target".to_vec(),
            relation: relation.to_owned(),
            weight_micros: 500_000,
            valid_from_ns: 1,
            valid_to_ns,
            citations: citations(lsn),
        })),
        Some(model(u8::try_from(lsn).unwrap())),
    )
}

fn retracted(lsn: u64, run_id: &[u8], edge_id: &[u8]) -> Frame {
    frame(
        lsn,
        EventKind::EdgeRetracted,
        run_id,
        EventPayload::EdgeRetracted(Box::new(EdgeRetracted {
            edge_id: edge_id.to_vec(),
            citations: citations(lsn),
        })),
        Some(model(u8::try_from(lsn).unwrap())),
    )
}

fn apply(store: &ProjectionStore, frames: &[Frame]) {
    for event in frames {
        GenerationProjection::apply_event(store, event).unwrap();
    }
}

#[test]
fn edges_are_readable_by_event_lsn_and_listed_in_lineage_order() {
    let directory = tempdir().unwrap();
    let store = ProjectionStore::open(directory.path(), 32 * 1024 * 1024).unwrap();
    let run_one = b"graph-run-one";
    apply(
        &store,
        &[
            opened(1, run_one, 1, 0),
            asserted(2, run_one, b"edge-a", "supports", 0),
            asserted(3, run_one, b"edge-b", "contradicts", 0),
            asserted(4, run_one, b"edge-c", "refines", 0),
            closed(5, run_one, 1, 0, 3),
        ],
    );

    let snapshot = store.begin_snapshot().unwrap();
    for (lsn, edge_id, relation) in [
        (2_u64, b"edge-a".as_slice(), "supports"),
        (3, b"edge-b".as_slice(), "contradicts"),
        (4, b"edge-c".as_slice(), "refines"),
    ] {
        let record = GraphProjection::edge_at(&snapshot, lsn).unwrap().unwrap();
        assert_eq!(record.edge_id, edge_id);
        assert_eq!(record.relation, relation);
        assert_eq!(record.event_lsn, lsn);
        assert_eq!(record.generation, 1);
        assert_eq!(
            record.citations,
            vec![EdgeCitation {
                first_lsn: lsn,
                last_lsn: lsn,
                byte_start: 0,
                byte_end: 16,
            }]
        );
    }
    assert_eq!(GraphProjection::edge_at(&snapshot, 99).unwrap(), None);

    let listed = GraphProjection::list_edges(&snapshot, 1, 10_000, 10).unwrap();
    assert_eq!(
        listed
            .iter()
            .map(|record| record.event_lsn)
            .collect::<Vec<_>>(),
        vec![2, 3, 4]
    );
    let truncated = GraphProjection::list_edges(&snapshot, 1, 10_000, 2).unwrap();
    assert_eq!(
        truncated
            .iter()
            .map(|record| record.event_lsn)
            .collect::<Vec<_>>(),
        vec![2, 3]
    );
    drop(snapshot);

    let run_two = b"graph-run-two";
    apply(
        &store,
        &[
            opened(6, run_two, 2, 1),
            asserted(7, run_two, b"edge-a", "reinforces", 0),
            closed(8, run_two, 2, 1, 1),
        ],
    );

    let snapshot = store.begin_snapshot().unwrap();
    let listed = GraphProjection::list_edges(&snapshot, 2, 10_000, 10).unwrap();
    assert_eq!(
        listed
            .iter()
            .map(|record| record.event_lsn)
            .collect::<Vec<_>>(),
        vec![3, 4, 7]
    );
    let newest = listed
        .iter()
        .find(|record| record.edge_id == b"edge-a")
        .unwrap();
    assert_eq!(newest.relation, "reinforces");
    assert_eq!(newest.generation, 2);
}

#[test]
fn retracted_and_expired_edges_are_excluded_from_the_relationship_listing() {
    let directory = tempdir().unwrap();
    let store = ProjectionStore::open(directory.path(), 32 * 1024 * 1024).unwrap();
    let run_one = b"graph-visibility-one";
    apply(
        &store,
        &[
            opened(1, run_one, 1, 0),
            asserted(2, run_one, b"edge-a", "supports", 0),
            asserted(3, run_one, b"edge-b", "contradicts", 0),
            asserted(4, run_one, b"edge-c", "refines", 500),
            closed(5, run_one, 1, 0, 3),
        ],
    );
    let run_two = b"graph-visibility-two";
    apply(
        &store,
        &[
            opened(6, run_two, 2, 1),
            retracted(7, run_two, b"edge-b"),
            closed(8, run_two, 2, 1, 1),
        ],
    );

    let snapshot = store.begin_snapshot().unwrap();
    let listed = GraphProjection::list_edges(&snapshot, 2, 10_000, 10).unwrap();
    assert_eq!(
        listed
            .iter()
            .map(|record| record.edge_id.clone())
            .collect::<Vec<_>>(),
        vec![b"edge-a".to_vec()]
    );
    let retracted_record = GraphProjection::edge_at(&snapshot, 3).unwrap().unwrap();
    assert_eq!(retracted_record.edge_id, b"edge-b");
    let expired_record = GraphProjection::edge_at(&snapshot, 4).unwrap().unwrap();
    assert_eq!(expired_record.valid_to_ns, 500);
    assert_eq!(
        GraphProjection::list_edges(&snapshot, 2, 400, 10)
            .unwrap()
            .iter()
            .map(|record| record.edge_id.clone())
            .collect::<Vec<_>>(),
        vec![b"edge-a".to_vec(), b"edge-c".to_vec()]
    );
}

#[test]
fn relationship_reads_reject_zero_bounds_and_unreadable_generations() {
    let directory = tempdir().unwrap();
    let store = ProjectionStore::open(directory.path(), 32 * 1024 * 1024).unwrap();
    let run_one = b"graph-bounds-one";
    apply(
        &store,
        &[
            opened(1, run_one, 1, 0),
            asserted(2, run_one, b"edge-a", "supports", 0),
            closed(3, run_one, 1, 0, 1),
        ],
    );
    let run_two = b"graph-bounds-two";
    apply(
        &store,
        &[
            opened(4, run_two, 2, 1),
            asserted(5, run_two, b"edge-b", "contradicts", 0),
        ],
    );

    let snapshot = store.begin_snapshot().unwrap();
    let before = snapshot.canonical_dump(ProjectionId::Graph).unwrap();
    assert_eq!(
        GraphProjection::edge_at(&snapshot, 0).unwrap_err().code,
        ErrorCode::InvalidArgument
    );
    assert_eq!(
        GraphProjection::list_edges(&snapshot, 1, 10_000, 0)
            .unwrap_err()
            .code,
        ErrorCode::InvalidArgument
    );
    assert_eq!(
        GraphProjection::list_edges(&snapshot, 2, 10_000, 10)
            .unwrap_err()
            .code,
        ErrorCode::InvalidArgument
    );
    assert_eq!(
        GraphProjection::list_edges(&snapshot, 1, 10_000, 10)
            .unwrap()
            .len(),
        1
    );
    assert!(GraphProjection::edge_at(&snapshot, 2).unwrap().is_some());
    let after = snapshot.canonical_dump(ProjectionId::Graph).unwrap();
    assert_eq!(before, after);
}
