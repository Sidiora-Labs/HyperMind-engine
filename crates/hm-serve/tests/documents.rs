use hm_core::{ActorId, ConversationId};
use hm_ledger::frame::EventKind;
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    Authority, ConsolidationBudget, ConsolidationClosed, ConsolidationOpened,
    ConsolidationPhaseName, ConsolidationRetracted, DocumentChange, DocumentChunk, DocumentChunked,
    DocumentCut, DocumentExtracted, DocumentIngested, DocumentPageSpan, EventEnvelope,
    EventPayload, Retention, Sensitivity,
};
use hm_serve::actor::{ActorConfig, ActorEngine, DocumentState, IncomingEvent};

const DOCUMENT_ID: [u8; 32] = [0x2a; 32];
const CONTENT: &[u8] = b"alpha beta\ngamma delta\n";
const FIRST_TEXT: &[u8] = b"alpha beta\ngamma delta\n";
const SECOND_TEXT: &[u8] = b"alpha beta\ngamma delta\nomega\n";
const FIRST_RUN: &[u8] = b"document-extract/one";
const SECOND_RUN: &[u8] = b"document-extract/two";

fn config(path: &std::path::Path, actor: u16) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join(actor.to_string()),
        actor: ActorId::new(actor),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

fn event(
    kind: EventKind,
    run_id: Option<&[u8]>,
    authority: Authority,
    payload: EventPayload,
) -> IncomingEvent {
    IncomingEvent {
        kind,
        conversation: ConversationId::derive("documents:handbook"),
        payload: encode_event_envelope(&EventEnvelope {
            schema_version: CURRENT_SCHEMA_VERSION,
            payload,
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 0,
            origin_actor: 0,
            run_id: run_id.map(<[u8]>::to_vec),
            model_provenance: None,
            authority,
            retention: Retention::Durable,
            sensitivity: Sensitivity::Personal,
            event_time_ns: 0,
        }),
    }
}

fn ingested() -> IncomingEvent {
    event(
        EventKind::DocumentIngested,
        None,
        Authority::ExternalObserved,
        EventPayload::DocumentIngested(Box::new(DocumentIngested {
            document_id: DOCUMENT_ID.to_vec(),
            name: "handbook.md".to_owned(),
            media_type: "text/markdown".to_owned(),
            content: CONTENT.to_vec(),
            content_digest: vec![0x11; 32],
        })),
    )
}

fn opened(run_id: &[u8], generation: u64, parent: u64) -> IncomingEvent {
    event(
        EventKind::ConsolidationOpened,
        Some(run_id),
        Authority::DerivedInference,
        EventPayload::ConsolidationOpened(Box::new(ConsolidationOpened {
            scope_digest: vec![u8::try_from(generation).unwrap(); 32],
            cadence_key: format!("document-{generation}"),
            generation,
            expected_active_generation: parent,
            phases: vec![ConsolidationPhaseName::Extract],
            prompts: Vec::new(),
            budget: Box::new(ConsolidationBudget {
                max_llm_calls: 1,
                max_tokens: 1_000,
                max_microusd: 100,
                max_wall_ms: 10_000,
            }),
            source_first_lsn: 0,
            source_last_lsn: 0,
        })),
    )
}

fn extracted(run_id: &[u8], source_lsn: u64, text: &[u8], digest: u8) -> IncomingEvent {
    event(
        EventKind::DocumentExtracted,
        Some(run_id),
        Authority::DerivedInference,
        EventPayload::DocumentExtracted(Box::new(DocumentExtracted {
            document_id: DOCUMENT_ID.to_vec(),
            source_lsn,
            loader_id: "text".to_owned(),
            extraction_version: 1,
            text: text.to_vec(),
            text_digest: vec![digest; 32],
            page_spans: vec![DocumentPageSpan {
                page_number: 1,
                byte_start: 0,
                byte_end: u32::try_from(text.len()).unwrap(),
            }],
            partial_reason: None,
            failed_units: None,
        })),
    )
}

fn chunk(
    ordinal: u32,
    byte_start: u32,
    byte_end: u32,
    tag: u8,
    change: DocumentChange,
) -> DocumentChunk {
    DocumentChunk {
        chunk_id: vec![tag + u8::try_from(ordinal).unwrap(); 32],
        content_hash: vec![tag + 0x40 + u8::try_from(ordinal).unwrap(); 32],
        occurrence: 0,
        ordinal,
        byte_start,
        byte_end,
        cut: DocumentCut::ParagraphEnd,
        change,
        page_number: 1,
        row_index: 0,
        column_start: 0,
        column_end: 0,
        token_estimate: 2,
    }
}

fn chunked(run_id: &[u8], source_lsn: u64, chunks: Vec<DocumentChunk>) -> IncomingEvent {
    event(
        EventKind::DocumentChunked,
        Some(run_id),
        Authority::DerivedInference,
        EventPayload::DocumentChunked(Box::new(DocumentChunked {
            document_id: DOCUMENT_ID.to_vec(),
            source_lsn,
            extraction_version: 1,
            chunker_id: "paragraph".to_owned(),
            token_budget: 64,
            previous_chunked_lsn: 0,
            chunks,
        })),
    )
}

fn closed(run_id: &[u8], generation: u64, parent: u64) -> IncomingEvent {
    event(
        EventKind::ConsolidationClosed,
        Some(run_id),
        Authority::DerivedInference,
        EventPayload::ConsolidationClosed(Box::new(ConsolidationClosed {
            generation,
            expected_active_generation: parent,
            derived_records: 2,
            dropped_candidates: 0,
            llm_calls: 0,
            input_tokens: 0,
            output_tokens: 0,
            cost_microusd: 0,
        })),
    )
}

fn retracted(run_id: &[u8], previous_generation: u64) -> IncomingEvent {
    event(
        EventKind::ConsolidationRetracted,
        Some(run_id),
        Authority::DerivedInference,
        EventPayload::ConsolidationRetracted(Box::new(ConsolidationRetracted {
            target_run_id: run_id.to_vec(),
            previous_generation,
            reason: "superseded extraction".to_owned(),
        })),
    )
}

fn first_chunks() -> Vec<DocumentChunk> {
    vec![
        chunk(0, 0, 11, 0x60, DocumentChange::Added),
        chunk(1, 11, 23, 0x60, DocumentChange::Added),
    ]
}

fn second_chunks() -> Vec<DocumentChunk> {
    vec![
        chunk(0, 0, 11, 0x80, DocumentChange::Retained),
        chunk(1, 11, 23, 0x80, DocumentChange::Replaced),
        chunk(2, 23, 29, 0x80, DocumentChange::Added),
    ]
}

fn summary(state: &DocumentState) -> Vec<(u32, Vec<u8>, u32, u32)> {
    state
        .chunks
        .iter()
        .map(|record| {
            (
                record.ordinal,
                record.chunk_id.clone(),
                record.byte_start,
                record.byte_end,
            )
        })
        .collect()
}

fn expected(chunks: &[DocumentChunk]) -> Vec<(u32, Vec<u8>, u32, u32)> {
    chunks
        .iter()
        .map(|chunk| {
            (
                chunk.ordinal,
                chunk.chunk_id.clone(),
                chunk.byte_start,
                chunk.byte_end,
            )
        })
        .collect()
}

async fn state(engine: &ActorEngine) -> DocumentState {
    engine
        .document_state(DOCUMENT_ID.to_vec())
        .await
        .unwrap()
        .expect("document state")
}

#[tokio::test]
async fn document_state_is_none_for_an_unknown_document() {
    let temporary = tempfile::tempdir().unwrap();
    let engine = ActorEngine::open(config(temporary.path(), 21))
        .await
        .unwrap();

    assert_eq!(engine.document_state(vec![0x33; 32]).await.unwrap(), None);

    engine.append(vec![ingested()]).await.unwrap();
    assert_eq!(engine.document_state(vec![0x33; 32]).await.unwrap(), None);

    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn document_state_reports_the_catalogue_before_any_chunk_run() {
    let temporary = tempfile::tempdir().unwrap();
    let engine = ActorEngine::open(config(temporary.path(), 22))
        .await
        .unwrap();
    engine.append(vec![ingested()]).await.unwrap();

    let current = state(&engine).await;
    assert_eq!(current.generation, 0);
    assert_eq!(current.document.document_id, DOCUMENT_ID);
    assert_eq!(current.document.name, "handbook.md");
    assert_eq!(current.document.media_type, "text/markdown");
    assert_eq!(current.document.content_digest, vec![0x11; 32]);
    assert_eq!(
        current.document.byte_length,
        u64::try_from(CONTENT.len()).unwrap()
    );
    assert_eq!(current.document.ingest_lsn, 1);
    assert_eq!(current.extraction, None);
    assert!(current.chunks.is_empty());

    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn document_state_hides_a_staged_generation() {
    let temporary = tempfile::tempdir().unwrap();
    let engine = ActorEngine::open(config(temporary.path(), 23))
        .await
        .unwrap();
    let committed = engine
        .append(vec![
            ingested(),
            opened(FIRST_RUN, 1, 0),
            extracted(FIRST_RUN, 1, FIRST_TEXT, 0x21),
            chunked(FIRST_RUN, 3, first_chunks()),
        ])
        .await
        .unwrap();
    assert_eq!(committed.last_lsn.get(), 4);

    let current = state(&engine).await;
    assert_eq!(current.generation, 0);
    assert_eq!(current.document.ingest_lsn, 1);
    assert_eq!(current.extraction, None);
    assert!(current.chunks.is_empty());

    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn document_state_returns_the_published_chunk_set() {
    let temporary = tempfile::tempdir().unwrap();
    let engine = ActorEngine::open(config(temporary.path(), 24))
        .await
        .unwrap();
    engine
        .append(vec![
            ingested(),
            opened(FIRST_RUN, 1, 0),
            extracted(FIRST_RUN, 1, FIRST_TEXT, 0x21),
            chunked(FIRST_RUN, 3, first_chunks()),
            closed(FIRST_RUN, 1, 0),
        ])
        .await
        .unwrap();

    let current = state(&engine).await;
    assert_eq!(current.generation, 1);
    let extraction = current.extraction.as_ref().expect("extraction record");
    assert_eq!(extraction.loader_id, "text");
    assert_eq!(extraction.extraction_version, 1);
    assert_eq!(extraction.text, FIRST_TEXT);
    assert_eq!(extraction.text_digest, vec![0x21; 32]);
    assert_eq!(extraction.source_lsn, 1);
    assert_eq!(extraction.generation, 1);
    assert_eq!(extraction.run_id, FIRST_RUN);
    assert!(extraction.partial_reason.is_empty());
    assert!(extraction.failed_units.is_empty());
    assert_eq!(summary(&current), expected(&first_chunks()));
    assert!(
        current
            .chunks
            .iter()
            .all(|record| record.generation == 1 && record.document_id == DOCUMENT_ID)
    );

    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn document_state_falls_back_after_retraction() {
    let temporary = tempfile::tempdir().unwrap();
    let engine = ActorEngine::open(config(temporary.path(), 25))
        .await
        .unwrap();
    engine
        .append(vec![
            ingested(),
            opened(FIRST_RUN, 1, 0),
            extracted(FIRST_RUN, 1, FIRST_TEXT, 0x21),
            chunked(FIRST_RUN, 3, first_chunks()),
            closed(FIRST_RUN, 1, 0),
        ])
        .await
        .unwrap();
    engine
        .append(vec![
            opened(SECOND_RUN, 2, 1),
            extracted(SECOND_RUN, 1, SECOND_TEXT, 0x22),
            chunked(SECOND_RUN, 7, second_chunks()),
            closed(SECOND_RUN, 2, 1),
        ])
        .await
        .unwrap();

    let published = state(&engine).await;
    assert_eq!(published.generation, 2);
    assert_eq!(
        published.extraction.as_ref().unwrap().text_digest,
        vec![0x22; 32]
    );
    assert_eq!(summary(&published), expected(&second_chunks()));

    engine.append(vec![retracted(SECOND_RUN, 1)]).await.unwrap();

    let restored = state(&engine).await;
    assert_eq!(restored.generation, 1);
    assert_eq!(
        restored.extraction.as_ref().unwrap().text_digest,
        vec![0x21; 32]
    );
    assert_eq!(summary(&restored), expected(&first_chunks()));

    engine.shutdown().await.unwrap();
}
