#![allow(clippy::missing_errors_doc)]

use crate::Envelope;
use crate::tools::consolidate::{RunStatus, hex, incoming, read_history};
use crate::tools::remember::RememberDocument;
use base64::Engine as _;
use hm_core::{ConversationId, Error, ErrorCode};
use hm_cortex::budget::BudgetUsage;
use hm_cortex::run::{PhaseMachine, run_id};
use hm_docs::chunk::{
    ChunkCut, ChunkSpan, MAXIMUM_TOKEN_BUDGET, WordEstimator, chunk_paragraphs, chunk_table,
};
use hm_docs::formats::table::TableLoader;
use hm_docs::identity::document_identity;
use hm_docs::loader::{
    DocumentLoader, Extraction, LoaderId, LoaderRegistry, MAXIMUM_DOCUMENT_BYTES,
};
use hm_docs::plan::{
    ChangeKind, ChangePlan, StoredChunk, plan_initial, plan_revision, validate_plan,
};
use hm_ledger::frame::EventKind;
use hm_schema::event::{CURRENT_SCHEMA_VERSION, MAXIMUM_IDENTIFIER_BYTES, encode_event_envelope};
use hm_schema::events::{
    Authority, ConsolidationBudget, ConsolidationClosed, ConsolidationOpened,
    ConsolidationPhaseName, ConsolidationPhaseState, DocumentChange, DocumentChunk,
    DocumentChunked, DocumentCut, DocumentExtracted, DocumentIngested, DocumentPageSpan,
    EventEnvelope, EventPayload, Retention, Sensitivity,
};
use hm_serve::actor::{ActorEngine, IncomingEvent};
use serde_json::{Value, json};

const DEFAULT_TOKEN_BUDGET: u32 = 512;
const DERIVED_RECORDS: u64 = 2;
const PARAGRAPH_CHUNKER: &str = "paragraph";
const TABLE_CHUNKER: &str = "row";
const SCOPE_DOMAIN: &[u8] = b"hypermind.document-scope.v1\0";
const REVISION_DOMAIN: &[u8] = b"hypermind.document-revision.v1\0";

#[allow(clippy::too_many_lines)]
pub async fn run(
    actor: &ActorEngine,
    conversation: ConversationId,
    input: &RememberDocument,
    retention: Retention,
    sensitivity: Sensitivity,
) -> Result<Envelope, Error> {
    if retention == Retention::DoNotStore {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let name = input.name.trim();
    let media_type = input.media_type.trim();
    if !bounded(name) || !bounded(media_type) {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    guard_encoded_size(input.content_base64.len())?;
    let content = base64::engine::general_purpose::STANDARD
        .decode(input.content_base64.as_bytes())
        .map_err(|_| Error::new(ErrorCode::SchemaInvalid))?;
    if content.len() > MAXIMUM_DOCUMENT_BYTES {
        return Err(Error::new(ErrorCode::CapacityExceeded));
    }
    if content.is_empty() {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }

    let registry = LoaderRegistry::standard();
    let loader = match input
        .loader
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(value) => loader_by_id(value)?,
        None => registry.select(media_type, name)?,
    };
    let extraction = registry.extract(loader, &content)?;
    if extraction.text.is_empty() {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    let budget = input.token_budget.unwrap_or(DEFAULT_TOKEN_BUDGET);
    if budget == 0 || budget > MAXIMUM_TOKEN_BUDGET {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }

    let content_digest = blake3::hash(&content);
    let document_id = document_handle(name);
    let state = actor.document_state(document_id.to_vec()).await?;
    let mut previous_text: Option<String> = None;
    let mut stored: Vec<StoredChunk> = Vec::new();
    let mut previous_chunked_lsn = 0_u64;
    let mut ingest_lsn = 0_u64;
    if let Some(current) = state.as_ref() {
        if current.document.content_digest.as_slice() == content_digest.as_bytes().as_slice() {
            ingest_lsn = current.document.ingest_lsn;
        }
        if let Some(record) = current.extraction.as_ref()
            && !current.chunks.is_empty()
        {
            previous_text = Some(
                String::from_utf8(record.text.clone())
                    .map_err(|_| Error::new(ErrorCode::SchemaInvalid))?,
            );
            previous_chunked_lsn = current.chunks.first().map_or(0, |chunk| chunk.event_lsn);
            for chunk in &current.chunks {
                stored.push(StoredChunk {
                    chunk_id: identity(&chunk.chunk_id)?,
                    content_hash: identity(&chunk.content_hash)?,
                    occurrence: chunk.occurrence,
                    ordinal: chunk.ordinal,
                    byte_start: offset(chunk.byte_start)?,
                    byte_end: offset(chunk.byte_end)?,
                    cut: cut_of(chunk.cut)?,
                    page_number: chunk.page_number,
                    row_index: chunk.row_index,
                    column_start: chunk.column_start,
                    column_end: chunk.column_end,
                    token_estimate: chunk.token_estimate,
                });
            }
        }
    }

    let plan = match previous_text.as_deref() {
        Some(old_text) => plan_revision(
            document_id,
            old_text,
            &stored,
            &extraction.text,
            &|region: &str| region_spans(loader, region, budget),
        )?,
        None => plan_initial(
            document_id,
            &extraction.text,
            &document_spans(&extraction, budget)?,
        )?,
    };
    validate_plan(&plan, &stored, &extraction.text)?;

    let mut envelope = preview(&document_id, &extraction, &plan);
    if let Some(partial) = extraction.partial.as_ref() {
        envelope.gaps.push(json!({
            "kind": "partial_extraction",
            "loader": loader.as_str(),
            "reason": partial.reason,
            "failed_units": partial.failed_units,
        }));
        envelope.warnings.push(
            "The document was stored with part of its content unextracted; the unread units are named in the gap."
                .to_owned(),
        );
    }
    if input.plan_only {
        envelope.items[0]["published"] = json!(false);
        return Ok(envelope);
    }

    let text_digest = blake3::hash(extraction.text.as_bytes());
    let scope_digest = scope_of(&document_id);
    let cadence_key = cadence_of(&document_id, text_digest.as_bytes());
    let history = read_history(actor).await?;
    if let Some((id, existing)) = history.runs.iter().find(|(_, run)| {
        run.status == RunStatus::Published
            && run.scope_digest == scope_digest
            && run.cadence_key == cadence_key
    }) {
        return Ok(publish_report(
            envelope,
            id,
            existing.generation,
            ingest_lsn,
            existing.last_lsn,
        ));
    }

    if ingest_lsn == 0 {
        let retained = actor
            .append(vec![ingested_event(
                conversation,
                &document_id,
                name,
                media_type,
                &content,
                retention,
                sensitivity,
            )])
            .await?;
        ingest_lsn = retained.first_lsn.get();
    }

    let generation = history.maximum_generation.saturating_add(1).max(1);
    let parent = history.active_generation;
    let id = run_id(&scope_digest, &cadence_key, generation).to_vec();
    let opened = ConsolidationOpened {
        scope_digest: scope_digest.to_vec(),
        cadence_key,
        generation,
        expected_active_generation: parent,
        phases: vec![ConsolidationPhaseName::Extract],
        prompts: Vec::new(),
        budget: Box::new(ConsolidationBudget {
            max_llm_calls: 1,
            max_tokens: 1,
            max_microusd: 1,
            max_wall_ms: 1,
        }),
        source_first_lsn: ingest_lsn,
        source_last_lsn: ingest_lsn,
    };
    let mut machine = PhaseMachine::resume(id.clone(), opened.phases.clone(), []);
    let work = machine
        .next()
        .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))?;
    let started = machine.event(
        &work,
        ConsolidationPhaseState::Started,
        None,
        BudgetUsage::default(),
        0,
    );
    machine.record(&work, &started);
    let completed = machine.event(
        &work,
        ConsolidationPhaseState::Completed,
        None,
        BudgetUsage::default(),
        0,
    );
    let extracted = extracted_payload(&document_id, &extraction, ingest_lsn, text_digest)?;
    let chunked = chunked_payload(
        &document_id,
        &extraction,
        &plan,
        ingest_lsn,
        budget,
        previous_chunked_lsn,
    )?;
    let outcome = actor
        .append(vec![
            incoming(
                actor,
                &id,
                EventKind::ConsolidationOpened,
                EventPayload::ConsolidationOpened(Box::new(opened)),
            ),
            incoming(
                actor,
                &id,
                EventKind::ConsolidationPhase,
                EventPayload::ConsolidationPhase(Box::new(started)),
            ),
            staged(
                actor,
                conversation,
                &id,
                EventKind::DocumentExtracted,
                EventPayload::DocumentExtracted(Box::new(extracted)),
                retention,
                sensitivity,
            ),
            staged(
                actor,
                conversation,
                &id,
                EventKind::DocumentChunked,
                EventPayload::DocumentChunked(Box::new(chunked)),
                retention,
                sensitivity,
            ),
            incoming(
                actor,
                &id,
                EventKind::ConsolidationPhase,
                EventPayload::ConsolidationPhase(Box::new(completed)),
            ),
            incoming(
                actor,
                &id,
                EventKind::ConsolidationClosed,
                EventPayload::ConsolidationClosed(Box::new(ConsolidationClosed {
                    generation,
                    expected_active_generation: parent,
                    derived_records: DERIVED_RECORDS,
                    dropped_candidates: 0,
                    llm_calls: 0,
                    input_tokens: 0,
                    output_tokens: 0,
                    cost_microusd: 0,
                })),
            ),
        ])
        .await?;

    Ok(publish_report(
        envelope,
        &id,
        generation,
        ingest_lsn,
        outcome.last_lsn.get(),
    ))
}

fn preview(document_id: &[u8; 32], extraction: &Extraction, plan: &ChangePlan) -> Envelope {
    let mut envelope = Envelope::empty();
    envelope.items.push(json!({
        "document_id": hex(document_id),
        "loader": extraction.loader.as_str(),
        "extraction_version": extraction.extraction_version,
        "published": false,
        "generation": Value::Null,
        "run_id": Value::Null,
        "first_lsn": 0,
        "last_lsn": 0,
        "plan": {
            "regions": plan.regions,
            "retained": plan.retained,
            "moved": plan.moved,
            "replaced": plan.replaced,
            "added": plan.added,
        },
        "chunks": plan
            .chunks
            .iter()
            .map(|chunk| json!({
                "chunk_id": hex(&chunk.chunk_id),
                "ordinal": chunk.ordinal,
                "change": change_name(chunk.change),
                "byte_start": chunk.byte_start,
                "byte_end": chunk.byte_end,
                "cut": cut_name(chunk.cut),
                "page": chunk.page_number,
                "row": chunk.row_index,
                "tokens": chunk.token_estimate,
            }))
            .collect::<Vec<_>>(),
    }));
    envelope
}

fn publish_report(
    mut envelope: Envelope,
    id: &[u8],
    generation: u64,
    first_lsn: u64,
    last_lsn: u64,
) -> Envelope {
    envelope.items[0]["published"] = json!(true);
    envelope.items[0]["generation"] = json!(generation);
    envelope.items[0]["run_id"] = json!(hex(id));
    envelope.items[0]["first_lsn"] = json!(first_lsn);
    envelope.items[0]["last_lsn"] = json!(last_lsn);
    envelope
}

fn extracted_payload(
    document_id: &[u8; 32],
    extraction: &Extraction,
    source_lsn: u64,
    text_digest: blake3::Hash,
) -> Result<DocumentExtracted, Error> {
    let mut page_spans = Vec::with_capacity(extraction.page_spans.len());
    for page in &extraction.page_spans {
        page_spans.push(DocumentPageSpan {
            page_number: page.page_number,
            byte_start: span(page.byte_start)?,
            byte_end: span(page.byte_end)?,
        });
    }
    Ok(DocumentExtracted {
        document_id: document_id.to_vec(),
        source_lsn,
        loader_id: extraction.loader.as_str().to_owned(),
        extraction_version: extraction.extraction_version,
        text: extraction.text.as_bytes().to_vec(),
        text_digest: text_digest.as_bytes().to_vec(),
        page_spans,
        partial_reason: extraction
            .partial
            .as_ref()
            .map(|partial| partial.reason.clone()),
        failed_units: extraction
            .partial
            .as_ref()
            .map(|partial| partial.failed_units.clone()),
    })
}

fn chunked_payload(
    document_id: &[u8; 32],
    extraction: &Extraction,
    plan: &ChangePlan,
    source_lsn: u64,
    token_budget: u32,
    previous_chunked_lsn: u64,
) -> Result<DocumentChunked, Error> {
    let mut chunks = Vec::with_capacity(plan.chunks.len());
    for chunk in &plan.chunks {
        chunks.push(DocumentChunk {
            chunk_id: chunk.chunk_id.to_vec(),
            content_hash: chunk.content_hash.to_vec(),
            occurrence: chunk.occurrence,
            ordinal: chunk.ordinal,
            byte_start: span(chunk.byte_start)?,
            byte_end: span(chunk.byte_end)?,
            cut: wire_cut(chunk.cut),
            change: wire_change(chunk.change),
            page_number: chunk.page_number,
            row_index: chunk.row_index,
            column_start: chunk.column_start,
            column_end: chunk.column_end,
            token_estimate: chunk.token_estimate,
        });
    }
    Ok(DocumentChunked {
        document_id: document_id.to_vec(),
        source_lsn,
        extraction_version: extraction.extraction_version,
        chunker_id: chunker_of(extraction).to_owned(),
        token_budget,
        previous_chunked_lsn,
        chunks,
    })
}

fn ingested_event(
    conversation: ConversationId,
    document_id: &[u8; 32],
    name: &str,
    media_type: &str,
    content: &[u8],
    retention: Retention,
    sensitivity: Sensitivity,
) -> IncomingEvent {
    IncomingEvent {
        kind: EventKind::DocumentIngested,
        conversation,
        payload: encode_event_envelope(&EventEnvelope {
            schema_version: CURRENT_SCHEMA_VERSION,
            payload: EventPayload::DocumentIngested(Box::new(DocumentIngested {
                document_id: document_id.to_vec(),
                name: name.to_owned(),
                media_type: media_type.to_owned(),
                content: content.to_vec(),
                content_digest: blake3::hash(content).as_bytes().to_vec(),
            })),
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 0,
            origin_actor: 0,
            run_id: None,
            model_provenance: None,
            authority: Authority::ExternalObserved,
            retention,
            sensitivity,
            event_time_ns: 0,
        }),
    }
}

fn staged(
    actor: &ActorEngine,
    conversation: ConversationId,
    id: &[u8],
    kind: EventKind,
    payload: EventPayload,
    retention: Retention,
    sensitivity: Sensitivity,
) -> IncomingEvent {
    IncomingEvent {
        kind,
        conversation,
        payload: encode_event_envelope(&EventEnvelope {
            schema_version: CURRENT_SCHEMA_VERSION,
            payload,
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 0,
            origin_actor: actor.actor().get(),
            run_id: Some(id.to_vec()),
            model_provenance: None,
            authority: Authority::DerivedInference,
            retention,
            sensitivity,
            event_time_ns: 0,
        }),
    }
}

fn document_spans(extraction: &Extraction, budget: u32) -> Result<Vec<ChunkSpan>, Error> {
    match extraction.table.as_ref() {
        Some(layout) => chunk_table(&extraction.text, layout, budget, &WordEstimator),
        None => chunk_paragraphs(
            &extraction.text,
            budget,
            &WordEstimator,
            &extraction.page_spans,
        ),
    }
}

fn region_spans(loader: LoaderId, region: &str, budget: u32) -> Result<Vec<ChunkSpan>, Error> {
    if loader == LoaderId::Table {
        let extracted = TableLoader.extract(region.as_bytes())?;
        let layout = extracted
            .table
            .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))?;
        return chunk_table(region, &layout, budget, &WordEstimator);
    }
    chunk_paragraphs(region, budget, &WordEstimator, &[])
}

const fn chunker_of(extraction: &Extraction) -> &'static str {
    if extraction.table.is_some() {
        TABLE_CHUNKER
    } else {
        PARAGRAPH_CHUNKER
    }
}

fn loader_by_id(value: &str) -> Result<LoaderId, Error> {
    match value {
        "text" => Ok(LoaderId::Text),
        "table" => Ok(LoaderId::Table),
        "pdf" => Ok(LoaderId::Pdf),
        "mail" => Ok(LoaderId::Mail),
        _ => Err(Error::new(ErrorCode::OperationUnavailable)),
    }
}

fn document_handle(name: &str) -> [u8; 32] {
    document_identity(name, &[])
}

fn scope_of(document_id: &[u8; 32]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(SCOPE_DOMAIN);
    hasher.update(document_id);
    *hasher.finalize().as_bytes()
}

fn cadence_of(document_id: &[u8; 32], text_digest: &[u8; 32]) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(REVISION_DOMAIN);
    hasher.update(document_id);
    hasher.update(text_digest);
    format!("document:{}", hex(hasher.finalize().as_bytes()))
}

fn guard_encoded_size(encoded: usize) -> Result<(), Error> {
    if encoded / 4 * 3 > MAXIMUM_DOCUMENT_BYTES {
        return Err(Error::new(ErrorCode::CapacityExceeded));
    }
    Ok(())
}

fn bounded(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAXIMUM_IDENTIFIER_BYTES
}

fn identity(value: &[u8]) -> Result<[u8; 32], Error> {
    value
        .try_into()
        .map_err(|_| Error::new(ErrorCode::SchemaInvalid))
}

fn offset(value: u32) -> Result<usize, Error> {
    usize::try_from(value).map_err(|_| Error::new(ErrorCode::CapacityExceeded))
}

fn span(value: usize) -> Result<u32, Error> {
    u32::try_from(value).map_err(|_| Error::new(ErrorCode::CapacityExceeded))
}

fn cut_of(value: u8) -> Result<ChunkCut, Error> {
    match value {
        0 => Ok(ChunkCut::ParagraphEnd),
        1 => Ok(ChunkCut::ParagraphCut),
        2 => Ok(ChunkCut::RowEnd),
        3 => Ok(ChunkCut::RowCut),
        _ => Err(Error::new(ErrorCode::SchemaInvalid)),
    }
}

const fn wire_cut(cut: ChunkCut) -> DocumentCut {
    match cut {
        ChunkCut::ParagraphEnd => DocumentCut::ParagraphEnd,
        ChunkCut::ParagraphCut => DocumentCut::ParagraphCut,
        ChunkCut::RowEnd => DocumentCut::RowEnd,
        ChunkCut::RowCut => DocumentCut::RowCut,
    }
}

const fn wire_change(change: ChangeKind) -> DocumentChange {
    match change {
        ChangeKind::Retained => DocumentChange::Retained,
        ChangeKind::Moved => DocumentChange::Moved,
        ChangeKind::Replaced => DocumentChange::Replaced,
        ChangeKind::Added => DocumentChange::Added,
    }
}

const fn cut_name(cut: ChunkCut) -> &'static str {
    match cut {
        ChunkCut::ParagraphEnd => "paragraph_end",
        ChunkCut::ParagraphCut => "paragraph_cut",
        ChunkCut::RowEnd => "row_end",
        ChunkCut::RowCut => "row_cut",
    }
}

const fn change_name(change: ChangeKind) -> &'static str {
    match change {
        ChangeKind::Retained => "retained",
        ChangeKind::Moved => "moved",
        ChangeKind::Replaced => "replaced",
        ChangeKind::Added => "added",
    }
}
