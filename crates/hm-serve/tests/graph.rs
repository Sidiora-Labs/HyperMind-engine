use hm_core::{ActorId, ConversationId, ErrorCode};
use hm_ledger::frame::EventKind;
use hm_proj::memories::MemoryRecord;
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    Authority, ConsolidationBudget, ConsolidationClosed, ConsolidationOpened,
    ConsolidationPhaseName, EdgeAsserted, EventEnvelope, EventPayload, MemoryMinted,
    ModelProvenance, PromptVersion, ProvenanceRange, Retention, Sensitivity,
};
use hm_serve::actor::{ActorConfig, ActorEngine, IncomingEvent};

const RUN: &[u8] = b"repository-graph/read";
const ANCHOR: &[u8] = b"node-service";
const ENDPOINT: &[u8] = b"node-handler";
const EDGE: &[u8] = b"edge-service-contains-handler";

fn config(path: &std::path::Path, actor: u16) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join(actor.to_string()),
        actor: ActorId::new(actor),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

fn provenance() -> ModelProvenance {
    ModelProvenance {
        model_id: "repository-graph-extract".to_owned(),
        prompt_id: "repository-graph-extract/v1".to_owned(),
        prompt_version: 1,
        temperature: 0.0,
        call_id: Some(vec![9; 32]),
        input_tokens: 8,
        output_tokens: 4,
        cache_read_tokens: 0,
        cache_write_tokens: 0,
        cost_microusd: 0,
    }
}

fn citation() -> ProvenanceRange {
    ProvenanceRange {
        first_lsn: 1,
        last_lsn: 1,
        byte_start: 0,
        byte_end: 16,
    }
}

fn event(
    kind: EventKind,
    payload: EventPayload,
    model_provenance: Option<ModelProvenance>,
) -> IncomingEvent {
    IncomingEvent {
        kind,
        conversation: ConversationId::derive("repository:atlas"),
        payload: encode_event_envelope(&EventEnvelope {
            schema_version: CURRENT_SCHEMA_VERSION,
            payload,
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 0,
            origin_actor: 0,
            run_id: Some(RUN.to_vec()),
            model_provenance: model_provenance.map(Box::new),
            authority: Authority::DerivedInference,
            retention: Retention::Durable,
            sensitivity: Sensitivity::Personal,
            event_time_ns: 0,
        }),
    }
}

fn minted(memory_id: &[u8], name: &str) -> IncomingEvent {
    event(
        EventKind::MemoryMinted,
        EventPayload::MemoryMinted(Box::new(MemoryMinted {
            memory_id: memory_id.to_vec(),
            name: name.to_owned(),
            definition: format!("{name} of the atlas repository").into_bytes(),
            tags: vec!["file".to_owned()],
            salience_micros: 500_000,
            citations: vec![citation()],
        })),
        Some(provenance()),
    )
}

fn published_generation() -> Vec<IncomingEvent> {
    vec![
        event(
            EventKind::ConsolidationOpened,
            EventPayload::ConsolidationOpened(Box::new(ConsolidationOpened {
                scope_digest: vec![3; 32],
                cadence_key: "repository:atlas".to_owned(),
                generation: 1,
                expected_active_generation: 0,
                phases: vec![ConsolidationPhaseName::Publish],
                prompts: vec![PromptVersion {
                    prompt_id: "repository-graph-extract/v1".to_owned(),
                    version: 1,
                    model_id: "repository-graph-extract".to_owned(),
                }],
                budget: Box::new(ConsolidationBudget {
                    max_llm_calls: 1,
                    max_tokens: 1_000,
                    max_microusd: 1_000,
                    max_wall_ms: 1_000,
                }),
                source_first_lsn: 0,
                source_last_lsn: 0,
            })),
            None,
        ),
        minted(ANCHOR, "service"),
        minted(ENDPOINT, "handler"),
        event(
            EventKind::EdgeAsserted,
            EventPayload::EdgeAsserted(Box::new(EdgeAsserted {
                edge_id: EDGE.to_vec(),
                source_id: ANCHOR.to_vec(),
                target_id: ENDPOINT.to_vec(),
                relation: "contains".to_owned(),
                weight_micros: 250_000,
                valid_from_ns: 0,
                valid_to_ns: 0,
                citations: vec![citation()],
            })),
            Some(provenance()),
        ),
        event(
            EventKind::ConsolidationClosed,
            EventPayload::ConsolidationClosed(Box::new(ConsolidationClosed {
                generation: 1,
                expected_active_generation: 0,
                derived_records: 3,
                dropped_candidates: 0,
                llm_calls: 1,
                input_tokens: 24,
                output_tokens: 12,
                cost_microusd: 0,
            })),
            None,
        ),
    ]
}

fn named<'a>(records: &'a [MemoryRecord], name: &str) -> &'a MemoryRecord {
    records
        .iter()
        .find(|record| record.name == name)
        .expect("memory record")
}

async fn assert_published_reads(engine: &ActorEngine) {
    let anchor = engine
        .graph_neighbourhood(ANCHOR.to_vec(), 0, 16)
        .await
        .unwrap();
    assert_eq!(anchor.generation, 1);
    assert_eq!(anchor.node.as_ref().unwrap().name, "service");
    assert_eq!(anchor.neighbours.len(), 1);
    assert!(anchor.neighbours[0].outgoing);
    assert_eq!(anchor.neighbours[0].edge.relation, "contains");
    assert_eq!(anchor.neighbours[0].edge.source_id, ANCHOR);
    assert_eq!(
        anchor.neighbours[0].endpoint.as_ref().unwrap().name,
        "handler"
    );

    let endpoint = engine
        .graph_neighbourhood(ENDPOINT.to_vec(), 0, 16)
        .await
        .unwrap();
    assert_eq!(endpoint.generation, 1);
    assert_eq!(endpoint.node.as_ref().unwrap().name, "handler");
    assert_eq!(endpoint.neighbours.len(), 1);
    assert!(!endpoint.neighbours[0].outgoing);
    assert_eq!(endpoint.neighbours[0].edge.edge_id, EDGE);
    assert_eq!(
        endpoint.neighbours[0].endpoint.as_ref().unwrap().name,
        "service"
    );

    let unknown = engine
        .graph_neighbourhood(b"node-absent".to_vec(), 0, 16)
        .await
        .unwrap();
    assert_eq!(unknown.generation, 1);
    assert!(unknown.node.is_none());
    assert!(unknown.neighbours.is_empty());

    let records = engine.memories(16).await.unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(named(&records, "service").memory_id, ANCHOR);
    assert_eq!(named(&records, "service").version_lsn, 2);
    assert_eq!(named(&records, "handler").memory_id, ENDPOINT);
    assert_eq!(named(&records, "handler").version_lsn, 3);
}

#[tokio::test]
async fn graph_neighbourhood_reads_a_published_generation() {
    let temporary = tempfile::tempdir().unwrap();
    let engine = ActorEngine::open(config(temporary.path(), 5))
        .await
        .unwrap();
    let committed = engine.append(published_generation()).await.unwrap();
    assert_eq!(committed.last_lsn.get(), 5);
    assert_published_reads(&engine).await;
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn graph_reads_are_bounded_and_survive_restart() {
    let temporary = tempfile::tempdir().unwrap();
    let engine = ActorEngine::open(config(temporary.path(), 6))
        .await
        .unwrap();

    assert_eq!(
        engine
            .graph_neighbourhood(Vec::new(), 0, 16)
            .await
            .unwrap_err()
            .code,
        ErrorCode::InvalidArgument
    );
    assert_eq!(
        engine
            .graph_neighbourhood(ANCHOR.to_vec(), 0, 0)
            .await
            .unwrap_err()
            .code,
        ErrorCode::InvalidArgument
    );
    assert_eq!(
        engine
            .graph_neighbourhood(ANCHOR.to_vec(), 0, 257)
            .await
            .unwrap_err()
            .code,
        ErrorCode::InvalidArgument
    );
    assert_eq!(
        engine.memories(0).await.unwrap_err().code,
        ErrorCode::InvalidArgument
    );
    assert_eq!(
        engine.memories(257).await.unwrap_err().code,
        ErrorCode::InvalidArgument
    );

    let empty = engine
        .graph_neighbourhood(ANCHOR.to_vec(), 0, 16)
        .await
        .unwrap();
    assert_eq!(empty.generation, 0);
    assert!(empty.node.is_none());
    assert!(empty.neighbours.is_empty());
    assert!(engine.memories(16).await.unwrap().is_empty());

    engine.append(published_generation()).await.unwrap();
    assert_published_reads(&engine).await;
    engine.shutdown().await.unwrap();

    let reopened = ActorEngine::open(config(temporary.path(), 6))
        .await
        .unwrap();
    assert_published_reads(&reopened).await;
    reopened.shutdown().await.unwrap();
}
