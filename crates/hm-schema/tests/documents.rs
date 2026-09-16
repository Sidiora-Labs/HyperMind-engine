#![forbid(unsafe_code)]

use hm_core::ErrorCode;
use hm_schema::event::{Boundary, EventKind, encode_event_envelope, verify_event};
use hm_schema::events::{
    Authority, ConsolidationBudget, ConsolidationOpened, ConsolidationPhaseName, DocumentChange,
    DocumentChunk, DocumentChunked, DocumentCut, DocumentExtracted, DocumentIngested,
    DocumentPageSpan, EventEnvelope, EventPayload, Retention, Sensitivity,
};

fn event_envelope(payload: EventPayload, schema_version: u16) -> EventEnvelope {
    EventEnvelope {
        schema_version,
        payload,
        connection_id: None,
        client_seq: 0,
        client_event_index: 0,
        client_event_count: 0,
        origin_actor: 3,
        run_id: None,
        model_provenance: None,
        authority: Authority::ExternalObserved,
        retention: Retention::Durable,
        sensitivity: Sensitivity::Personal,
        event_time_ns: 0,
    }
}

fn encode_event(envelope: &EventEnvelope) -> Vec<u8> {
    encode_event_envelope(envelope)
}

fn document_ingested() -> DocumentIngested {
    DocumentIngested {
        document_id: vec![0x11; 32],
        name: "handbook.md".to_owned(),
        media_type: "text/markdown".to_owned(),
        content: b"Onboarding starts on Monday.".to_vec(),
        content_digest: vec![0x22; 32],
    }
}

fn document_extracted() -> DocumentExtracted {
    DocumentExtracted {
        document_id: vec![0x11; 32],
        source_lsn: 41,
        loader_id: "text".to_owned(),
        extraction_version: 1,
        text: b"Onboarding starts on Monday.".to_vec(),
        text_digest: vec![0x33; 32],
        page_spans: vec![DocumentPageSpan {
            page_number: 1,
            byte_start: 0,
            byte_end: 28,
        }],
        partial_reason: None,
        failed_units: None,
    }
}

fn chunk(ordinal: u32, byte_start: u32, byte_end: u32) -> DocumentChunk {
    DocumentChunk {
        chunk_id: vec![u8::try_from(ordinal).expect("ordinal fits") + 1; 32],
        content_hash: vec![0x44; 32],
        occurrence: 0,
        ordinal,
        byte_start,
        byte_end,
        cut: DocumentCut::ParagraphEnd,
        change: DocumentChange::Added,
        page_number: 1,
        row_index: 0,
        column_start: 0,
        column_end: 0,
        token_estimate: 4,
    }
}

fn document_chunked() -> DocumentChunked {
    DocumentChunked {
        document_id: vec![0x11; 32],
        source_lsn: 42,
        extraction_version: 1,
        chunker_id: "paragraph".to_owned(),
        token_budget: 256,
        previous_chunked_lsn: 0,
        chunks: vec![chunk(0, 0, 12), chunk(1, 12, 28)],
    }
}

fn derived_envelope(payload: EventPayload) -> EventEnvelope {
    let mut envelope = event_envelope(payload, 2);
    envelope.authority = Authority::DerivedInference;
    envelope.run_id = Some(b"document-run-1".to_vec());
    envelope
}

fn rejects(envelope: &EventEnvelope, kind: EventKind) {
    let error = verify_event(&encode_event(envelope), kind, Boundary::Socket)
        .expect_err("payload must be rejected");
    assert_eq!(error.code, ErrorCode::SchemaInvalid);
}

#[test]
fn document_event_kinds_round_trip_and_are_wave_seven() {
    let cases = [
        (45_u8, EventKind::DocumentIngested, false),
        (46_u8, EventKind::DocumentExtracted, true),
        (47_u8, EventKind::DocumentChunked, true),
    ];
    for (raw, expected, needs_run_id) in cases {
        let kind = EventKind::try_from(raw).expect("document kind decodes");
        assert_eq!(kind, expected);
        assert_eq!(kind as u8, raw);
        assert!(kind.is_wave_seven());
        assert_eq!(kind.requires_run_id(), needs_run_id);
        assert!(!kind.is_llm_derived());
    }
    assert!(EventKind::try_from(48).is_err());
}

#[test]
fn document_ingested_requires_identity_and_digest() {
    let envelope = event_envelope(
        EventPayload::DocumentIngested(Box::new(document_ingested())),
        2,
    );
    let encoded = encode_event(&envelope);
    for boundary in [Boundary::Socket, Boundary::Disk] {
        let verified = verify_event(&encoded, EventKind::DocumentIngested, boundary)
            .expect("well formed ingestion verifies");
        assert_eq!(verified.kind, EventKind::DocumentIngested);
        assert_eq!(verified.boundary, boundary);
    }

    let mut short_digest = document_ingested();
    short_digest.content_digest = vec![0x22; 31];
    rejects(
        &event_envelope(EventPayload::DocumentIngested(Box::new(short_digest)), 2),
        EventKind::DocumentIngested,
    );

    let mut short_identity = document_ingested();
    short_identity.document_id = vec![0x11; 16];
    rejects(
        &event_envelope(EventPayload::DocumentIngested(Box::new(short_identity)), 2),
        EventKind::DocumentIngested,
    );

    let mut empty_content = document_ingested();
    empty_content.content = Vec::new();
    rejects(
        &event_envelope(EventPayload::DocumentIngested(Box::new(empty_content)), 2),
        EventKind::DocumentIngested,
    );

    let mut empty_media_type = document_ingested();
    empty_media_type.media_type = String::new();
    rejects(
        &event_envelope(
            EventPayload::DocumentIngested(Box::new(empty_media_type)),
            2,
        ),
        EventKind::DocumentIngested,
    );

    let mut empty_name = document_ingested();
    empty_name.name = String::new();
    rejects(
        &event_envelope(EventPayload::DocumentIngested(Box::new(empty_name)), 2),
        EventKind::DocumentIngested,
    );
}

#[test]
fn document_extracted_requires_run_id_and_coherent_partial_fields() {
    let envelope = derived_envelope(EventPayload::DocumentExtracted(Box::new(
        document_extracted(),
    )));
    let encoded = encode_event(&envelope);
    for boundary in [Boundary::Socket, Boundary::Disk] {
        verify_event(&encoded, EventKind::DocumentExtracted, boundary)
            .expect("well formed extraction verifies");
    }

    let mut partial = document_extracted();
    partial.partial_reason = Some("two pages could not be decoded".to_owned());
    partial.failed_units = Some(vec![3, 4]);
    verify_event(
        &encode_event(&derived_envelope(EventPayload::DocumentExtracted(
            Box::new(partial),
        ))),
        EventKind::DocumentExtracted,
        Boundary::Socket,
    )
    .expect("declared partial extraction verifies");

    let mut missing_run_id = derived_envelope(EventPayload::DocumentExtracted(Box::new(
        document_extracted(),
    )));
    missing_run_id.run_id = None;
    rejects(&missing_run_id, EventKind::DocumentExtracted);

    let mut zero_source = document_extracted();
    zero_source.source_lsn = 0;
    rejects(
        &derived_envelope(EventPayload::DocumentExtracted(Box::new(zero_source))),
        EventKind::DocumentExtracted,
    );

    let mut zero_version = document_extracted();
    zero_version.extraction_version = 0;
    rejects(
        &derived_envelope(EventPayload::DocumentExtracted(Box::new(zero_version))),
        EventKind::DocumentExtracted,
    );

    let mut empty_loader = document_extracted();
    empty_loader.loader_id = String::new();
    rejects(
        &derived_envelope(EventPayload::DocumentExtracted(Box::new(empty_loader))),
        EventKind::DocumentExtracted,
    );

    let mut empty_text = document_extracted();
    empty_text.text = Vec::new();
    rejects(
        &derived_envelope(EventPayload::DocumentExtracted(Box::new(empty_text))),
        EventKind::DocumentExtracted,
    );

    let mut short_digest = document_extracted();
    short_digest.text_digest = vec![0x33; 31];
    rejects(
        &derived_envelope(EventPayload::DocumentExtracted(Box::new(short_digest))),
        EventKind::DocumentExtracted,
    );

    let mut units_without_reason = document_extracted();
    units_without_reason.failed_units = Some(vec![7]);
    rejects(
        &derived_envelope(EventPayload::DocumentExtracted(Box::new(
            units_without_reason,
        ))),
        EventKind::DocumentExtracted,
    );

    let mut reason_without_units = document_extracted();
    reason_without_units.partial_reason = Some("a page failed".to_owned());
    rejects(
        &derived_envelope(EventPayload::DocumentExtracted(Box::new(
            reason_without_units,
        ))),
        EventKind::DocumentExtracted,
    );
}

#[test]
fn document_chunked_chunks_must_tile_and_be_ordinally_dense() {
    let envelope = derived_envelope(EventPayload::DocumentChunked(Box::new(document_chunked())));
    let encoded = encode_event(&envelope);
    for boundary in [Boundary::Socket, Boundary::Disk] {
        verify_event(&encoded, EventKind::DocumentChunked, boundary)
            .expect("tiled chunk list verifies");
    }

    let mut missing_run_id =
        derived_envelope(EventPayload::DocumentChunked(Box::new(document_chunked())));
    missing_run_id.run_id = None;
    rejects(&missing_run_id, EventKind::DocumentChunked);

    let mut skipped_ordinal = document_chunked();
    skipped_ordinal.chunks[1].ordinal = 2;
    rejects(
        &derived_envelope(EventPayload::DocumentChunked(Box::new(skipped_ordinal))),
        EventKind::DocumentChunked,
    );

    let mut gapped = document_chunked();
    gapped.chunks[1].byte_start = 13;
    rejects(
        &derived_envelope(EventPayload::DocumentChunked(Box::new(gapped))),
        EventKind::DocumentChunked,
    );

    let mut late_start = document_chunked();
    late_start.chunks[0].byte_start = 1;
    rejects(
        &derived_envelope(EventPayload::DocumentChunked(Box::new(late_start))),
        EventKind::DocumentChunked,
    );

    let mut empty_range = document_chunked();
    empty_range.chunks[1].byte_end = 12;
    rejects(
        &derived_envelope(EventPayload::DocumentChunked(Box::new(empty_range))),
        EventKind::DocumentChunked,
    );

    let mut short_chunk_id = document_chunked();
    short_chunk_id.chunks[0].chunk_id = vec![0x01; 31];
    rejects(
        &derived_envelope(EventPayload::DocumentChunked(Box::new(short_chunk_id))),
        EventKind::DocumentChunked,
    );

    let mut short_content_hash = document_chunked();
    short_content_hash.chunks[1].content_hash = vec![0x44; 16];
    rejects(
        &derived_envelope(EventPayload::DocumentChunked(Box::new(short_content_hash))),
        EventKind::DocumentChunked,
    );

    let mut no_chunks = document_chunked();
    no_chunks.chunks.clear();
    rejects(
        &derived_envelope(EventPayload::DocumentChunked(Box::new(no_chunks))),
        EventKind::DocumentChunked,
    );

    let mut no_budget = document_chunked();
    no_budget.token_budget = 0;
    rejects(
        &derived_envelope(EventPayload::DocumentChunked(Box::new(no_budget))),
        EventKind::DocumentChunked,
    );

    let mut empty_chunker = document_chunked();
    empty_chunker.chunker_id = String::new();
    rejects(
        &derived_envelope(EventPayload::DocumentChunked(Box::new(empty_chunker))),
        EventKind::DocumentChunked,
    );

    let mut zero_source = document_chunked();
    zero_source.source_lsn = 0;
    rejects(
        &derived_envelope(EventPayload::DocumentChunked(Box::new(zero_source))),
        EventKind::DocumentChunked,
    );

    let mut zero_version = document_chunked();
    zero_version.extraction_version = 0;
    rejects(
        &derived_envelope(EventPayload::DocumentChunked(Box::new(zero_version))),
        EventKind::DocumentChunked,
    );
}

#[test]
fn extract_only_runs_may_declare_no_prompts() {
    let extract = consolidation_opened(vec![ConsolidationPhaseName::Extract]);
    verify_event(
        &encode_event(&derived_envelope(EventPayload::ConsolidationOpened(
            Box::new(extract),
        ))),
        EventKind::ConsolidationOpened,
        Boundary::Socket,
    )
    .expect("an extract-only run needs no prompts");

    let nrem = consolidation_opened(vec![ConsolidationPhaseName::Nrem]);
    rejects(
        &derived_envelope(EventPayload::ConsolidationOpened(Box::new(nrem))),
        EventKind::ConsolidationOpened,
    );

    let mixed = consolidation_opened(vec![
        ConsolidationPhaseName::Extract,
        ConsolidationPhaseName::Connect,
    ]);
    rejects(
        &derived_envelope(EventPayload::ConsolidationOpened(Box::new(mixed))),
        EventKind::ConsolidationOpened,
    );
}

fn consolidation_opened(phases: Vec<ConsolidationPhaseName>) -> ConsolidationOpened {
    ConsolidationOpened {
        scope_digest: vec![0x55; 32],
        cadence_key: "document:handbook.md".to_owned(),
        generation: 9,
        expected_active_generation: 8,
        phases,
        prompts: Vec::new(),
        budget: Box::new(ConsolidationBudget {
            max_llm_calls: 1,
            max_tokens: 1_000,
            max_microusd: 1_000,
            max_wall_ms: 10_000,
        }),
    }
}
