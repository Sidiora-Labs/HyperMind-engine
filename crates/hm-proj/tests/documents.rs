#![forbid(unsafe_code)]
#![allow(clippy::too_many_lines)]

use hm_core::{ActorId, ConversationId, ErrorCode, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_proj::documents::{ChunkRecord, DocumentsProjection};
use hm_proj::generation::GenerationProjection;
use hm_proj::rebuild::rebuild_projection_stream;
use hm_proj::runs::{RunsProjection, StagedProjection};
use hm_proj::store::{ProjectionId, ProjectionStore};
use hm_schema::event::encode_event_envelope;
use hm_schema::events::{
    Authority, ConsolidationBudget, ConsolidationClosed, ConsolidationOpened,
    ConsolidationPhaseName, ConsolidationRetracted, DocumentChange, DocumentChunk, DocumentChunked,
    DocumentCut, DocumentExtracted, DocumentIngested, DocumentPageSpan, EventEnvelope,
    EventPayload, Retention, Sensitivity,
};
use std::collections::BTreeMap;
use tempfile::tempdir;

const DOCUMENT_ID: [u8; 32] = [0x2a; 32];
const CONTENT: &[u8] = b"alpha beta\ngamma delta\n";
const FIRST_TEXT: &[u8] = b"alpha beta\ngamma delta\n";
const SECOND_TEXT: &[u8] = b"alpha beta\ngamma delta\nomega\n";

fn frame(lsn: u64, kind: EventKind, run_id: Option<&[u8]>, payload: EventPayload) -> Frame {
    let authority = if kind == EventKind::DocumentIngested {
        Authority::ExternalObserved
    } else {
        Authority::DerivedInference
    };
    Frame {
        header: FrameHeader {
            lsn: LSN::new(lsn),
            kind,
            wall_timestamp_ns: UtcNanos::new(i64::try_from(lsn).unwrap() * 1_000),
            actor: ActorId::new(11),
            conversation: ConversationId::new([0x5c; 16]),
        },
        sealed_payload: encode_event_envelope(&EventEnvelope {
            schema_version: 2,
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
            event_time_ns: i64::try_from(lsn).unwrap() * 1_000,
        }),
    }
}

fn ingested(lsn: u64) -> Frame {
    frame(
        lsn,
        EventKind::DocumentIngested,
        None,
        EventPayload::DocumentIngested(Box::new(DocumentIngested {
            document_id: DOCUMENT_ID.to_vec(),
            name: "handbook.md".to_owned(),
            media_type: "text/markdown".to_owned(),
            content: CONTENT.to_vec(),
            content_digest: vec![0x11; 32],
        })),
    )
}

fn extracted(lsn: u64, run_id: &[u8], source_lsn: u64, text: &[u8], digest: u8) -> Frame {
    frame(
        lsn,
        EventKind::DocumentExtracted,
        Some(run_id),
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

fn chunked(lsn: u64, run_id: &[u8], source_lsn: u64, chunks: Vec<DocumentChunk>) -> Frame {
    frame(
        lsn,
        EventKind::DocumentChunked,
        Some(run_id),
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

fn opened(lsn: u64, run_id: &[u8], generation: u64, parent: u64) -> Frame {
    frame(
        lsn,
        EventKind::ConsolidationOpened,
        Some(run_id),
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

fn closed(lsn: u64, run_id: &[u8], generation: u64, parent: u64, records: u64) -> Frame {
    frame(
        lsn,
        EventKind::ConsolidationClosed,
        Some(run_id),
        EventPayload::ConsolidationClosed(Box::new(ConsolidationClosed {
            generation,
            expected_active_generation: parent,
            derived_records: records,
            dropped_candidates: 0,
            llm_calls: 0,
            input_tokens: 0,
            output_tokens: 0,
            cost_microusd: 0,
        })),
    )
}

fn retracted(lsn: u64, run_id: &[u8], previous_generation: u64) -> Frame {
    frame(
        lsn,
        EventKind::ConsolidationRetracted,
        Some(run_id),
        EventPayload::ConsolidationRetracted(Box::new(ConsolidationRetracted {
            target_run_id: run_id.to_vec(),
            previous_generation,
            reason: "superseded extraction".to_owned(),
        })),
    )
}

fn summary(chunks: &[ChunkRecord]) -> Vec<(u32, Vec<u8>, u32, u32, u8)> {
    chunks
        .iter()
        .map(|record| {
            (
                record.ordinal,
                record.chunk_id.clone(),
                record.byte_start,
                record.byte_end,
                record.change,
            )
        })
        .collect()
}

fn first_revision(run_id: &[u8]) -> Vec<Frame> {
    vec![
        ingested(1),
        opened(2, run_id, 1, 0),
        extracted(3, run_id, 1, FIRST_TEXT, 0x21),
        chunked(4, run_id, 3, first_chunks()),
        closed(5, run_id, 1, 0, 2),
    ]
}

#[test]
fn documents_projection_is_registered() {
    assert_eq!(ProjectionId::COUNT, 21);
    assert_eq!(ProjectionId::Documents.name(), "documents");
}

#[test]
fn ingested_documents_are_readable_without_a_generation() {
    let directory = tempdir().unwrap();
    let store = ProjectionStore::open(directory.path(), 32 * 1024 * 1024).unwrap();
    DocumentsProjection::apply_event(&store, &ingested(1)).unwrap();

    let snapshot = store.begin_snapshot().unwrap();
    let record = DocumentsProjection::document(&snapshot, &DOCUMENT_ID)
        .unwrap()
        .expect("catalogue record");
    assert_eq!(record.name, "handbook.md");
    assert_eq!(record.media_type, "text/markdown");
    assert_eq!(record.content_digest, vec![0x11; 32]);
    assert_eq!(record.byte_length, u64::try_from(CONTENT.len()).unwrap());
    assert_eq!(record.ingest_lsn, 1);
    assert_eq!(
        DocumentsProjection::document(&snapshot, &[0x33; 32]).unwrap(),
        None
    );
}

#[test]
fn staged_document_records_are_invisible_until_publish() {
    let directory = tempdir().unwrap();
    let store = ProjectionStore::open(directory.path(), 32 * 1024 * 1024).unwrap();
    let run_id = b"document-run-one";
    let frames = first_revision(run_id);
    for event in &frames[..4] {
        GenerationProjection::apply_event(&store, event).unwrap();
    }

    let staged = store.begin_snapshot().unwrap();
    assert_eq!(
        DocumentsProjection::chunks(&staged, 1, &DOCUMENT_ID)
            .expect_err("open generation is unreadable")
            .code,
        ErrorCode::OperationUnavailable
    );
    assert_eq!(
        DocumentsProjection::extraction(&staged, 1, &DOCUMENT_ID)
            .expect_err("open generation is unreadable")
            .code,
        ErrorCode::OperationUnavailable
    );
    let run = RunsProjection::run(&staged, run_id).unwrap().unwrap();
    assert_eq!(
        run.staged,
        BTreeMap::from([
            (3, StagedProjection::Documents),
            (4, StagedProjection::Documents)
        ])
    );
    assert_eq!(RunsProjection::active_generation(&staged).unwrap(), 0);
    drop(staged);

    GenerationProjection::apply_event(&store, &frames[4]).unwrap();
    let snapshot = store.begin_snapshot().unwrap();
    assert_eq!(RunsProjection::active_generation(&snapshot).unwrap(), 1);
    let extraction = DocumentsProjection::extraction(&snapshot, 1, &DOCUMENT_ID)
        .unwrap()
        .expect("published extraction");
    assert_eq!(extraction.text, FIRST_TEXT);
    assert_eq!(extraction.loader_id, "text");
    assert_eq!(extraction.source_lsn, 1);
    assert_eq!(extraction.generation, 1);
    assert_eq!(extraction.run_id, run_id);
    assert_eq!(extraction.partial_reason, "");
    assert!(extraction.failed_units.is_empty());

    let chunks = DocumentsProjection::chunks(&snapshot, 1, &DOCUMENT_ID).unwrap();
    assert_eq!(
        summary(&chunks),
        vec![
            (0, vec![0x60; 32], 0, 11, DocumentChange::Added as u8),
            (1, vec![0x61; 32], 11, 23, DocumentChange::Added as u8),
        ]
    );
    assert!(chunks.iter().all(|record| record.generation == 1));
    assert_eq!(chunks[0].event_lsn, 4);
    assert_eq!(chunks[1].token_estimate, 2);
}

#[test]
fn publish_refuses_a_mismatched_derived_count() {
    let directory = tempdir().unwrap();
    let store = ProjectionStore::open(directory.path(), 32 * 1024 * 1024).unwrap();
    let run_id = b"document-run-mismatch";
    let frames = first_revision(run_id);
    for event in &frames[..4] {
        GenerationProjection::apply_event(&store, event).unwrap();
    }
    assert_eq!(
        GenerationProjection::apply_event(&store, &closed(5, run_id, 1, 0, 1))
            .expect_err("staged count is two")
            .code,
        ErrorCode::IdempotencyConflict
    );
    let snapshot = store.begin_snapshot().unwrap();
    assert_eq!(RunsProjection::active_generation(&snapshot).unwrap(), 0);
    assert_eq!(
        DocumentsProjection::chunks(&snapshot, 1, &DOCUMENT_ID)
            .expect_err("run is still open")
            .code,
        ErrorCode::OperationUnavailable
    );
}

#[test]
fn retraction_falls_back_to_the_parent_chunk_set() {
    let directory = tempdir().unwrap();
    let store = ProjectionStore::open(directory.path(), 32 * 1024 * 1024).unwrap();
    let run_one = b"document-run-one";
    let run_two = b"document-run-two";
    let mut frames = first_revision(run_one);
    frames.extend([
        opened(6, run_two, 2, 1),
        extracted(7, run_two, 1, SECOND_TEXT, 0x22),
        chunked(8, run_two, 7, second_chunks()),
        closed(9, run_two, 2, 1, 2),
    ]);
    for event in &frames {
        GenerationProjection::apply_event(&store, event).unwrap();
    }

    let snapshot = store.begin_snapshot().unwrap();
    assert_eq!(RunsProjection::active_generation(&snapshot).unwrap(), 2);
    let revised = DocumentsProjection::chunks(&snapshot, 2, &DOCUMENT_ID).unwrap();
    assert_eq!(revised.len(), 3);
    assert_eq!(
        summary(&revised),
        vec![
            (0, vec![0x80; 32], 0, 11, DocumentChange::Retained as u8),
            (1, vec![0x81; 32], 11, 23, DocumentChange::Replaced as u8),
            (2, vec![0x82; 32], 23, 29, DocumentChange::Added as u8),
        ]
    );
    assert_eq!(
        DocumentsProjection::extraction(&snapshot, 2, &DOCUMENT_ID)
            .unwrap()
            .unwrap()
            .text,
        SECOND_TEXT
    );
    drop(snapshot);

    GenerationProjection::apply_event(&store, &retracted(10, run_two, 1)).unwrap();
    let snapshot = store.begin_snapshot().unwrap();
    assert_eq!(RunsProjection::active_generation(&snapshot).unwrap(), 1);
    let parent = DocumentsProjection::chunks(&snapshot, 1, &DOCUMENT_ID).unwrap();
    assert_eq!(
        summary(&parent),
        vec![
            (0, vec![0x60; 32], 0, 11, DocumentChange::Added as u8),
            (1, vec![0x61; 32], 11, 23, DocumentChange::Added as u8),
        ]
    );
    let resolved = DocumentsProjection::chunks(&snapshot, 2, &DOCUMENT_ID).unwrap();
    assert_eq!(summary(&resolved), summary(&parent));
    assert_eq!(
        DocumentsProjection::extraction(&snapshot, 2, &DOCUMENT_ID)
            .unwrap()
            .unwrap()
            .text,
        FIRST_TEXT
    );
    assert_eq!(
        DocumentsProjection::document(&snapshot, &DOCUMENT_ID)
            .unwrap()
            .unwrap()
            .ingest_lsn,
        1
    );
}

#[test]
fn replay_is_idempotent_and_gaps_are_refused() {
    let directory = tempdir().unwrap();
    let store = ProjectionStore::open(directory.path(), 32 * 1024 * 1024).unwrap();
    let run_id = b"document-run-replay";
    let frames = first_revision(run_id);
    for event in &frames {
        GenerationProjection::apply_event(&store, event).unwrap();
    }
    let before = store
        .begin_snapshot()
        .unwrap()
        .canonical_dump(ProjectionId::Documents)
        .unwrap();

    DocumentsProjection::apply_event(&store, &frames[4]).unwrap();
    let snapshot = store.begin_snapshot().unwrap();
    assert_eq!(
        snapshot.canonical_dump(ProjectionId::Documents).unwrap(),
        before
    );
    assert_eq!(
        snapshot.checkpoint(ProjectionId::Documents).unwrap().get(),
        5
    );
    drop(snapshot);

    assert_eq!(
        DocumentsProjection::apply_event(&store, &ingested(7))
            .expect_err("lsn six is missing")
            .code,
        ErrorCode::ProjectionCheckpoint
    );
}

#[test]
fn reset_rebuild_is_byte_identical() {
    let directory = tempdir().unwrap();
    let store = ProjectionStore::open(directory.path(), 64 * 1024 * 1024).unwrap();
    let run_one = b"document-run-one";
    let run_two = b"document-run-two";
    let mut frames = first_revision(run_one);
    frames.extend([
        opened(6, run_two, 2, 1),
        extracted(7, run_two, 1, SECOND_TEXT, 0x22),
        chunked(8, run_two, 7, second_chunks()),
        closed(9, run_two, 2, 1, 2),
    ]);
    let progress = rebuild_projection_stream(&store, &frames, false, usize::MAX).unwrap();
    assert!(progress.complete);
    let first = store
        .begin_snapshot()
        .unwrap()
        .canonical_dump(ProjectionId::Documents)
        .unwrap();

    rebuild_projection_stream(&store, &frames, true, usize::MAX).unwrap();
    let snapshot = store.begin_snapshot().unwrap();
    assert_eq!(
        snapshot.canonical_dump(ProjectionId::Documents).unwrap(),
        first
    );
    assert_eq!(
        DocumentsProjection::chunks(&snapshot, 2, &DOCUMENT_ID)
            .unwrap()
            .len(),
        3
    );
}
