#![forbid(unsafe_code)]

use hm_compose::lanes::graph::{GraphSeed, search};
use hm_core::{ActorId, ConversationId, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_proj::generation::GenerationProjection;
use hm_proj::store::ProjectionStore;
use hm_proj::vocabulary::VocabularyProjection;
use hm_schema::event::encode_event_envelope;
use hm_schema::events::{
    Authority, ConsolidationBudget, ConsolidationClosed, ConsolidationOpened,
    ConsolidationPhaseName, EdgeAsserted, EventEnvelope, EventPayload, ModelProvenance,
    PromptVersion, ProvenanceRange, Retention, Sensitivity, VocabularyCategory, VocabularyImported,
    VocabularyTerm,
};

const RUN: &[u8] = b"vocabulary-run";

fn frame(
    lsn: u64,
    kind: EventKind,
    payload: EventPayload,
    authority: Authority,
    run: Option<&[u8]>,
    model_provenance: Option<ModelProvenance>,
) -> Frame {
    Frame {
        header: FrameHeader {
            lsn: LSN::new(lsn),
            kind,
            wall_timestamp_ns: UtcNanos::new(i64::try_from(lsn).unwrap()),
            actor: ActorId::new(7),
            conversation: ConversationId::derive("vocabulary-graph"),
        },
        sealed_payload: encode_event_envelope(&EventEnvelope {
            schema_version: 2,
            payload,
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 0,
            origin_actor: 0,
            run_id: run.map(<[u8]>::to_vec),
            model_provenance: model_provenance.map(Box::new),
            authority,
            retention: Retention::Durable,
            sensitivity: Sensitivity::Personal,
            event_time_ns: i64::try_from(lsn).unwrap(),
        }),
    }
}

fn opened(lsn: u64) -> Frame {
    frame(
        lsn,
        EventKind::ConsolidationOpened,
        EventPayload::ConsolidationOpened(Box::new(ConsolidationOpened {
            scope_digest: vec![1; 32],
            cadence_key: "vocabulary-night".to_owned(),
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
        Authority::DerivedInference,
        Some(RUN),
        None,
    )
}

fn edge(lsn: u64, source: &[u8], target: &[u8], relation: &str) -> Frame {
    frame(
        lsn,
        EventKind::EdgeAsserted,
        EventPayload::EdgeAsserted(Box::new(EdgeAsserted {
            edge_id: format!("edge-{lsn}").into_bytes(),
            source_id: source.to_vec(),
            target_id: target.to_vec(),
            relation: relation.to_owned(),
            weight_micros: 500_000,
            valid_from_ns: 1,
            valid_to_ns: 0,
            citations: vec![ProvenanceRange {
                first_lsn: 1,
                last_lsn: 1,
                byte_start: 0,
                byte_end: 1,
            }],
        })),
        Authority::DerivedInference,
        Some(RUN),
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

fn closed(lsn: u64) -> Frame {
    frame(
        lsn,
        EventKind::ConsolidationClosed,
        EventPayload::ConsolidationClosed(Box::new(ConsolidationClosed {
            generation: 1,
            expected_active_generation: 0,
            derived_records: 1,
            dropped_candidates: 0,
            llm_calls: 1,
            input_tokens: 10,
            output_tokens: 2,
            cost_microusd: 5,
        })),
        Authority::DerivedInference,
        Some(RUN),
        None,
    )
}

fn imported(lsn: u64) -> Frame {
    frame(
        lsn,
        EventKind::VocabularyImported,
        EventPayload::VocabularyImported(Box::new(VocabularyImported {
            vocabulary_id: b"fleet-vocabulary".to_vec(),
            version: 1,
            source_uri: "file:///vocabularies/fleet.nt".to_owned(),
            source_media_type: "application/n-triples".to_owned(),
            source_digest: vec![0x5a; 32],
            terms: vec![VocabularyTerm {
                term_id: "hm:vocab/works_for".to_owned(),
                canonical_name: "works_for".to_owned(),
                category: VocabularyCategory::Relation,
                parent_term_id: None,
                aliases: Some(vec!["employs".to_owned()]),
            }],
            ignored_triples: 0,
        })),
        Authority::UserAsserted,
        None,
        None,
    )
}

fn apply(store: &ProjectionStore, frames: &[Frame]) {
    for frame in frames {
        GenerationProjection::apply_event(store, frame).unwrap();
        VocabularyProjection::apply_event(store, frame).unwrap();
    }
}

fn seeds() -> Vec<GraphSeed> {
    vec![GraphSeed {
        node_id: b"a".to_vec(),
        source_lsn: LSN::new(1),
    }]
}

fn reached(store: &ProjectionStore, query: &str) -> Vec<Vec<u8>> {
    let snapshot = store.begin_snapshot().unwrap();
    search(&snapshot, 1, &seeds(), query, 100, 1, 10)
        .unwrap()
        .candidates
        .into_iter()
        .map(|candidate| candidate.canonical_id)
        .collect()
}

fn graph_frames() -> Vec<Frame> {
    vec![opened(1), edge(2, b"a", b"b", "works_for"), closed(3)]
}

#[test]
fn graph_lane_follows_reviewed_relation_aliases() {
    let temporary = tempfile::tempdir().unwrap();
    let store = ProjectionStore::open(temporary.path(), 32 * 1024 * 1024).unwrap();
    let mut frames = graph_frames();
    frames.push(imported(4));
    apply(&store, &frames);

    assert_eq!(reached(&store, "employs"), vec![b"b".to_vec()]);
    assert_eq!(reached(&store, "works_for"), vec![b"b".to_vec()]);
}

#[test]
fn graph_lane_ignores_unreviewed_near_misses() {
    let temporary = tempfile::tempdir().unwrap();
    let store = ProjectionStore::open(temporary.path(), 32 * 1024 * 1024).unwrap();
    let mut frames = graph_frames();
    frames.push(imported(4));
    apply(&store, &frames);

    assert!(reached(&store, "employ").is_empty());
}

#[test]
fn graph_lane_without_vocabulary_keeps_literal_gating() {
    let temporary = tempfile::tempdir().unwrap();
    let store = ProjectionStore::open(temporary.path(), 32 * 1024 * 1024).unwrap();
    apply(&store, &graph_frames());

    assert!(reached(&store, "employs").is_empty());
    assert_eq!(reached(&store, "works for"), vec![b"b".to_vec()]);
}
