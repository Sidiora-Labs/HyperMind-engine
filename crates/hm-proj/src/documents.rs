#![allow(clippy::missing_errors_doc)]

use crate::generation::{decode, encode, should_apply, staged_marker_key, verify_frame};
use crate::runs::RunsProjection;
use crate::store::{Mutation, ProjectionId, ProjectionStore, ReadSnapshot};
use hm_core::{Error, ErrorCode};
use hm_ledger::frame::{EventKind as LedgerEventKind, Frame};
use hm_schema::events::{
    DocumentChunk, DocumentChunked, DocumentExtracted, DocumentIngested, EventEnvelope,
    EventPayload,
};
use serde::{Deserialize, Serialize};

const DOCUMENT_PREFIX: u8 = b'D';
const EXTRACTION_PREFIX: u8 = b'X';
const CHUNK_PREFIX: u8 = b'H';

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DocumentRecord {
    pub document_id: Vec<u8>,
    pub name: String,
    pub media_type: String,
    pub content_digest: Vec<u8>,
    pub byte_length: u64,
    pub ingest_lsn: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExtractionRecord {
    pub document_id: Vec<u8>,
    pub source_lsn: u64,
    pub loader_id: String,
    pub extraction_version: u16,
    pub text: Vec<u8>,
    pub text_digest: Vec<u8>,
    pub partial_reason: String,
    pub failed_units: Vec<u32>,
    pub run_id: Vec<u8>,
    pub generation: u64,
    pub event_lsn: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChunkRecord {
    pub document_id: Vec<u8>,
    pub chunk_id: Vec<u8>,
    pub content_hash: Vec<u8>,
    pub occurrence: u32,
    pub ordinal: u32,
    pub byte_start: u32,
    pub byte_end: u32,
    pub cut: u8,
    pub change: u8,
    pub page_number: u32,
    pub row_index: u32,
    pub column_start: u32,
    pub column_end: u32,
    pub token_estimate: u32,
    pub run_id: Vec<u8>,
    pub generation: u64,
    pub event_lsn: u64,
}

pub struct DocumentsProjection;

impl DocumentsProjection {
    pub fn apply_event(store: &ProjectionStore, frame: &Frame) -> Result<(), Error> {
        let snapshot = store.begin_snapshot()?;
        if !should_apply(&snapshot, ProjectionId::Documents, frame)? {
            return Ok(());
        }
        let mut mutations = Vec::new();
        match frame.header.kind {
            LedgerEventKind::DocumentIngested => {
                let verified = verify_frame(frame)?;
                let EventPayload::DocumentIngested(ingested) = &verified.envelope.payload else {
                    return Err(Error::new(ErrorCode::InvalidKind).at_lsn(frame.header.lsn));
                };
                let record = document_record(frame, ingested)?;
                mutations.push(Mutation::put(
                    document_key(&record.document_id)?,
                    encode(&record)?,
                ));
            }
            LedgerEventKind::DocumentExtracted => {
                let verified = verify_frame(frame)?;
                let EventPayload::DocumentExtracted(extracted) = &verified.envelope.payload else {
                    return Err(Error::new(ErrorCode::InvalidKind).at_lsn(frame.header.lsn));
                };
                let run_id = run_id(&verified.envelope)?;
                let generation = RunsProjection::generation_for_run(&snapshot, run_id)?;
                let record = extraction_record(frame, extracted, run_id, generation);
                mutations.push(Mutation::put(
                    extraction_key(generation, &record.document_id)?,
                    encode(&record)?,
                ));
                mutations.push(Mutation::put(
                    staged_marker_key(frame.header.lsn.get()),
                    [1],
                ));
            }
            LedgerEventKind::DocumentChunked => {
                let verified = verify_frame(frame)?;
                let EventPayload::DocumentChunked(chunked) = &verified.envelope.payload else {
                    return Err(Error::new(ErrorCode::InvalidKind).at_lsn(frame.header.lsn));
                };
                let run_id = run_id(&verified.envelope)?;
                let generation = RunsProjection::generation_for_run(&snapshot, run_id)?;
                for chunk in &chunked.chunks {
                    let record = chunk_record(frame, chunked, chunk, run_id, generation);
                    mutations.push(Mutation::put(
                        chunk_key(generation, &record.document_id, record.ordinal)?,
                        encode(&record)?,
                    ));
                }
                mutations.push(Mutation::put(
                    staged_marker_key(frame.header.lsn.get()),
                    [1],
                ));
            }
            _ => {}
        }
        drop(snapshot);
        store.apply(ProjectionId::Documents, frame.header.lsn, &mutations)
    }

    pub fn document(
        snapshot: &ReadSnapshot<'_>,
        document_id: &[u8],
    ) -> Result<Option<DocumentRecord>, Error> {
        snapshot
            .get(ProjectionId::Documents, &document_key(document_id)?)?
            .map(|bytes| decode(&bytes))
            .transpose()
    }

    pub fn extraction(
        snapshot: &ReadSnapshot<'_>,
        generation: u64,
        document_id: &[u8],
    ) -> Result<Option<ExtractionRecord>, Error> {
        if !RunsProjection::is_readable(snapshot, generation)? {
            return Err(Error::new(ErrorCode::OperationUnavailable));
        }
        for candidate in RunsProjection::lineage(snapshot, generation)? {
            if let Some(bytes) = snapshot.get(
                ProjectionId::Documents,
                &extraction_key(candidate, document_id)?,
            )? {
                return decode(&bytes).map(Some);
            }
        }
        Ok(None)
    }

    pub fn chunks(
        snapshot: &ReadSnapshot<'_>,
        generation: u64,
        document_id: &[u8],
    ) -> Result<Vec<ChunkRecord>, Error> {
        if !RunsProjection::is_readable(snapshot, generation)? {
            return Err(Error::new(ErrorCode::OperationUnavailable));
        }
        for candidate in RunsProjection::lineage(snapshot, generation)? {
            let prefix = chunk_prefix(candidate, document_id)?;
            let items = snapshot.scan_prefix(ProjectionId::Documents, &prefix, usize::MAX)?;
            if items.is_empty() {
                continue;
            }
            let mut output = Vec::with_capacity(items.len());
            for item in items {
                output.push(decode(&item.value)?);
            }
            return Ok(output);
        }
        Ok(Vec::new())
    }
}

fn document_record(frame: &Frame, ingested: &DocumentIngested) -> Result<DocumentRecord, Error> {
    Ok(DocumentRecord {
        document_id: ingested.document_id.clone(),
        name: ingested.name.clone(),
        media_type: ingested.media_type.clone(),
        content_digest: ingested.content_digest.clone(),
        byte_length: u64::try_from(ingested.content.len())
            .map_err(|_| Error::new(ErrorCode::InvalidLength))?,
        ingest_lsn: frame.header.lsn.get(),
    })
}

fn extraction_record(
    frame: &Frame,
    extracted: &DocumentExtracted,
    run_id: &[u8],
    generation: u64,
) -> ExtractionRecord {
    ExtractionRecord {
        document_id: extracted.document_id.clone(),
        source_lsn: extracted.source_lsn,
        loader_id: extracted.loader_id.clone(),
        extraction_version: extracted.extraction_version,
        text: extracted.text.clone(),
        text_digest: extracted.text_digest.clone(),
        partial_reason: extracted.partial_reason.clone().unwrap_or_default(),
        failed_units: extracted.failed_units.clone().unwrap_or_default(),
        run_id: run_id.to_vec(),
        generation,
        event_lsn: frame.header.lsn.get(),
    }
}

fn chunk_record(
    frame: &Frame,
    chunked: &DocumentChunked,
    chunk: &DocumentChunk,
    run_id: &[u8],
    generation: u64,
) -> ChunkRecord {
    ChunkRecord {
        document_id: chunked.document_id.clone(),
        chunk_id: chunk.chunk_id.clone(),
        content_hash: chunk.content_hash.clone(),
        occurrence: chunk.occurrence,
        ordinal: chunk.ordinal,
        byte_start: chunk.byte_start,
        byte_end: chunk.byte_end,
        cut: chunk.cut as u8,
        change: chunk.change as u8,
        page_number: chunk.page_number,
        row_index: chunk.row_index,
        column_start: chunk.column_start,
        column_end: chunk.column_end,
        token_estimate: chunk.token_estimate,
        run_id: run_id.to_vec(),
        generation,
        event_lsn: frame.header.lsn.get(),
    }
}

fn run_id(envelope: &EventEnvelope) -> Result<&[u8], Error> {
    envelope
        .run_id
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| Error::new(ErrorCode::SchemaInvalid))
}

fn document_key(document_id: &[u8]) -> Result<Vec<u8>, Error> {
    let mut key = vec![DOCUMENT_PREFIX];
    append_id(&mut key, document_id)?;
    Ok(key)
}

fn extraction_key(generation: u64, document_id: &[u8]) -> Result<Vec<u8>, Error> {
    let mut key = Vec::with_capacity(document_id.len() + 11);
    key.push(EXTRACTION_PREFIX);
    key.extend_from_slice(&generation.to_be_bytes());
    append_id(&mut key, document_id)?;
    Ok(key)
}

fn chunk_prefix(generation: u64, document_id: &[u8]) -> Result<Vec<u8>, Error> {
    let mut key = Vec::with_capacity(document_id.len() + 11);
    key.push(CHUNK_PREFIX);
    key.extend_from_slice(&generation.to_be_bytes());
    append_id(&mut key, document_id)?;
    Ok(key)
}

fn chunk_key(generation: u64, document_id: &[u8], ordinal: u32) -> Result<Vec<u8>, Error> {
    let mut key = chunk_prefix(generation, document_id)?;
    key.extend_from_slice(&ordinal.to_be_bytes());
    Ok(key)
}

fn append_id(key: &mut Vec<u8>, id: &[u8]) -> Result<(), Error> {
    let length = u16::try_from(id.len()).map_err(|_| Error::new(ErrorCode::InvalidLength))?;
    if length == 0 {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    key.extend_from_slice(&length.to_be_bytes());
    key.extend_from_slice(id);
    Ok(())
}
