#![allow(clippy::missing_errors_doc)]

use crate::generation::{decode, encode, should_apply, staged_marker_key, verify_frame};
use crate::runs::RunsProjection;
use crate::store::{Mutation, ProjectionId, ProjectionStore, ReadSnapshot};
use hm_core::{Error, ErrorCode};
use hm_ledger::frame::{EventKind as LedgerEventKind, Frame};
use hm_schema::events::{Authority, EventEnvelope, EventPayload, ProvenanceRange};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const HEAD_PREFIX: u8 = b'H';
const VERSION_PREFIX: u8 = b'V';
const FADE_PREFIX: u8 = b'F';

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MemoryCitation {
    pub first_lsn: u64,
    pub last_lsn: u64,
    pub byte_start: u32,
    pub byte_end: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MemoryModelProvenance {
    pub model_id: String,
    pub prompt_id: String,
    pub prompt_version: u16,
    pub call_id: Vec<u8>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub cost_microusd: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MemoryRecord {
    pub memory_id: Vec<u8>,
    pub name: String,
    pub definition: Vec<u8>,
    pub tags: Vec<String>,
    pub salience_micros: u32,
    pub citations: Vec<MemoryCitation>,
    pub authority: Authority,
    pub run_id: Vec<u8>,
    pub generation: u64,
    pub version_lsn: u64,
    pub previous_lsn: u64,
    pub merged_from: Vec<Vec<u8>>,
    pub model_provenance: MemoryModelProvenance,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct FadeRecord {
    generation: u64,
    event_lsn: u64,
    reason: u8,
    evidence_lsns: Vec<u64>,
}

pub struct MemoryProjection;

impl MemoryProjection {
    #[allow(clippy::too_many_lines)]
    pub fn apply_event(store: &ProjectionStore, frame: &Frame) -> Result<(), Error> {
        let snapshot = store.begin_snapshot()?;
        if !should_apply(&snapshot, ProjectionId::Memories, frame)? {
            return Ok(());
        }
        let mut mutations = Vec::new();
        match frame.header.kind {
            LedgerEventKind::MemoryMinted => {
                let verified = verify_frame(frame)?;
                let EventPayload::MemoryMinted(memory) = &verified.envelope.payload else {
                    return Err(Error::new(ErrorCode::InvalidKind).at_lsn(frame.header.lsn));
                };
                ensure_new_id(&snapshot, &memory.memory_id, frame)?;
                let generation = generation(&snapshot, &verified.envelope)?;
                let record = MemoryRecord {
                    memory_id: memory.memory_id.clone(),
                    name: memory.name.clone(),
                    definition: memory.definition.clone(),
                    tags: memory.tags.clone(),
                    salience_micros: memory.salience_micros,
                    citations: citations(&memory.citations),
                    authority: verified.envelope.authority,
                    run_id: run_id(&verified.envelope)?.to_vec(),
                    generation,
                    version_lsn: frame.header.lsn.get(),
                    previous_lsn: 0,
                    merged_from: Vec::new(),
                    model_provenance: model_provenance(&verified.envelope)?,
                };
                put_record(&record, &mut mutations)?;
            }
            LedgerEventKind::MemoryRevised => {
                let verified = verify_frame(frame)?;
                let EventPayload::MemoryRevised(memory) = &verified.envelope.payload else {
                    return Err(Error::new(ErrorCode::InvalidKind).at_lsn(frame.header.lsn));
                };
                let generation = generation(&snapshot, &verified.envelope)?;
                let previous = read_version(&snapshot, &memory.memory_id, memory.previous_lsn)?
                    .ok_or_else(|| {
                        Error::new(ErrorCode::OrderingViolation).at_lsn(frame.header.lsn)
                    })?;
                if current_for_staging(&snapshot, generation, &memory.memory_id)?
                    .is_none_or(|current| current.version_lsn != previous.version_lsn)
                {
                    return Err(Error::new(ErrorCode::IdempotencyConflict).at_lsn(frame.header.lsn));
                }
                let record = MemoryRecord {
                    memory_id: memory.memory_id.clone(),
                    name: memory.name.clone(),
                    definition: memory.definition.clone(),
                    tags: memory.tags.clone(),
                    salience_micros: memory.salience_micros,
                    citations: citations(&memory.citations),
                    authority: verified.envelope.authority,
                    run_id: run_id(&verified.envelope)?.to_vec(),
                    generation,
                    version_lsn: frame.header.lsn.get(),
                    previous_lsn: previous.version_lsn,
                    merged_from: Vec::new(),
                    model_provenance: model_provenance(&verified.envelope)?,
                };
                put_record(&record, &mut mutations)?;
            }
            LedgerEventKind::MemoryMerged => {
                let verified = verify_frame(frame)?;
                let EventPayload::MemoryMerged(memory) = &verified.envelope.payload else {
                    return Err(Error::new(ErrorCode::InvalidKind).at_lsn(frame.header.lsn));
                };
                ensure_new_id(&snapshot, &memory.memory_id, frame)?;
                let generation = generation(&snapshot, &verified.envelope)?;
                let mut merged_from = Vec::with_capacity(memory.merged_memory_ids.len());
                for source in &memory.merged_memory_ids {
                    if current_for_staging(&snapshot, generation, &source.value)?.is_none() {
                        return Err(
                            Error::new(ErrorCode::OrderingViolation).at_lsn(frame.header.lsn)
                        );
                    }
                    merged_from.push(source.value.clone());
                }
                let record = MemoryRecord {
                    memory_id: memory.memory_id.clone(),
                    name: memory.name.clone(),
                    definition: memory.definition.clone(),
                    tags: memory.tags.clone(),
                    salience_micros: memory.salience_micros,
                    citations: citations(&memory.citations),
                    authority: verified.envelope.authority,
                    run_id: run_id(&verified.envelope)?.to_vec(),
                    generation,
                    version_lsn: frame.header.lsn.get(),
                    previous_lsn: 0,
                    merged_from,
                    model_provenance: model_provenance(&verified.envelope)?,
                };
                put_record(&record, &mut mutations)?;
            }
            LedgerEventKind::MemoryFaded => {
                let verified = verify_frame(frame)?;
                let EventPayload::MemoryFaded(faded) = &verified.envelope.payload else {
                    return Err(Error::new(ErrorCode::InvalidKind).at_lsn(frame.header.lsn));
                };
                let generation = generation(&snapshot, &verified.envelope)?;
                if current_for_staging(&snapshot, generation, &faded.memory_id)?.is_none() {
                    return Err(Error::new(ErrorCode::OrderingViolation).at_lsn(frame.header.lsn));
                }
                let fade = FadeRecord {
                    generation,
                    event_lsn: frame.header.lsn.get(),
                    reason: faded.reason as u8,
                    evidence_lsns: faded.evidence_lsns.clone().unwrap_or_default(),
                };
                mutations.push(Mutation::put(
                    fade_key(&faded.memory_id, frame.header.lsn.get())?,
                    encode(&fade)?,
                ));
            }
            _ => {}
        }
        if matches!(
            frame.header.kind,
            LedgerEventKind::MemoryMinted
                | LedgerEventKind::MemoryRevised
                | LedgerEventKind::MemoryMerged
                | LedgerEventKind::MemoryFaded
        ) {
            mutations.push(Mutation::put(
                staged_marker_key(frame.header.lsn.get()),
                [1],
            ));
        }
        drop(snapshot);
        store.apply(ProjectionId::Memories, frame.header.lsn, &mutations)
    }

    pub fn get_visible(
        snapshot: &ReadSnapshot<'_>,
        generation: u64,
        memory_id: &[u8],
    ) -> Result<Option<MemoryRecord>, Error> {
        if !RunsProjection::is_readable(snapshot, generation)? {
            return Err(Error::new(ErrorCode::OperationUnavailable));
        }
        if is_faded(snapshot, memory_id)? {
            return Ok(None);
        }
        for candidate in RunsProjection::lineage(snapshot, generation)? {
            if let Some(bytes) =
                snapshot.get(ProjectionId::Memories, &head_key(candidate, memory_id)?)?
            {
                return decode(&bytes).map(Some);
            }
        }
        Ok(None)
    }

    pub fn list_visible(
        snapshot: &ReadSnapshot<'_>,
        generation: u64,
        limit: usize,
    ) -> Result<Vec<MemoryRecord>, Error> {
        if limit == 0 || !RunsProjection::is_readable(snapshot, generation)? {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let mut seen = BTreeSet::new();
        let mut output = Vec::new();
        for candidate in RunsProjection::lineage(snapshot, generation)? {
            let prefix = head_generation_prefix(candidate);
            for item in snapshot.scan_prefix(ProjectionId::Memories, &prefix, limit)? {
                let record: MemoryRecord = decode(&item.value)?;
                if seen.insert(record.memory_id.clone()) && !is_faded(snapshot, &record.memory_id)?
                {
                    output.push(record);
                    if output.len() == limit {
                        return Ok(output);
                    }
                }
            }
        }
        Ok(output)
    }
}

fn put_record(record: &MemoryRecord, mutations: &mut Vec<Mutation>) -> Result<(), Error> {
    let encoded = encode(record)?;
    mutations.push(Mutation::put(
        head_key(record.generation, &record.memory_id)?,
        encoded.clone(),
    ));
    mutations.push(Mutation::put(
        version_key(&record.memory_id, record.version_lsn)?,
        encoded,
    ));
    Ok(())
}

fn ensure_new_id(
    snapshot: &ReadSnapshot<'_>,
    memory_id: &[u8],
    frame: &Frame,
) -> Result<(), Error> {
    if latest_version(snapshot, memory_id)?.is_some() {
        Err(Error::new(ErrorCode::AlreadyExists).at_lsn(frame.header.lsn))
    } else {
        Ok(())
    }
}

fn latest_version(
    snapshot: &ReadSnapshot<'_>,
    memory_id: &[u8],
) -> Result<Option<MemoryRecord>, Error> {
    let prefix = id_prefix(VERSION_PREFIX, memory_id)?;
    snapshot
        .scan_prefix_reverse(ProjectionId::Memories, &prefix, 1)?
        .into_iter()
        .next()
        .map(|item| decode(&item.value))
        .transpose()
}

fn read_version(
    snapshot: &ReadSnapshot<'_>,
    memory_id: &[u8],
    lsn: u64,
) -> Result<Option<MemoryRecord>, Error> {
    snapshot
        .get(ProjectionId::Memories, &version_key(memory_id, lsn)?)?
        .map(|bytes| decode(&bytes))
        .transpose()
}

fn current_for_staging(
    snapshot: &ReadSnapshot<'_>,
    generation: u64,
    memory_id: &[u8],
) -> Result<Option<MemoryRecord>, Error> {
    for candidate in RunsProjection::lineage_for_staging(snapshot, generation)? {
        if let Some(bytes) =
            snapshot.get(ProjectionId::Memories, &head_key(candidate, memory_id)?)?
        {
            return decode(&bytes).map(Some);
        }
    }
    Ok(None)
}

fn is_faded(snapshot: &ReadSnapshot<'_>, memory_id: &[u8]) -> Result<bool, Error> {
    let prefix = id_prefix(FADE_PREFIX, memory_id)?;
    for item in snapshot.scan_prefix_reverse(ProjectionId::Memories, &prefix, usize::MAX)? {
        let fade: FadeRecord = decode(&item.value)?;
        if RunsProjection::is_published(snapshot, fade.generation)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn generation(snapshot: &ReadSnapshot<'_>, envelope: &EventEnvelope) -> Result<u64, Error> {
    RunsProjection::generation_for_run(snapshot, run_id(envelope)?)
}

fn run_id(envelope: &EventEnvelope) -> Result<&[u8], Error> {
    envelope
        .run_id
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| Error::new(ErrorCode::SchemaInvalid))
}

fn model_provenance(envelope: &EventEnvelope) -> Result<MemoryModelProvenance, Error> {
    let model = envelope
        .model_provenance
        .as_deref()
        .ok_or_else(|| Error::new(ErrorCode::SchemaInvalid))?;
    Ok(MemoryModelProvenance {
        model_id: model.model_id.clone(),
        prompt_id: model.prompt_id.clone(),
        prompt_version: model.prompt_version,
        call_id: model.call_id.clone().unwrap_or_default(),
        input_tokens: model.input_tokens,
        output_tokens: model.output_tokens,
        cache_read_tokens: model.cache_read_tokens,
        cache_write_tokens: model.cache_write_tokens,
        cost_microusd: model.cost_microusd,
    })
}

fn citations(values: &[ProvenanceRange]) -> Vec<MemoryCitation> {
    values
        .iter()
        .map(|value| MemoryCitation {
            first_lsn: value.first_lsn,
            last_lsn: value.last_lsn,
            byte_start: value.byte_start,
            byte_end: value.byte_end,
        })
        .collect()
}

fn head_generation_prefix(generation: u64) -> [u8; 9] {
    let mut key = [0_u8; 9];
    key[0] = HEAD_PREFIX;
    key[1..].copy_from_slice(&generation.to_be_bytes());
    key
}

fn head_key(generation: u64, memory_id: &[u8]) -> Result<Vec<u8>, Error> {
    let mut key = head_generation_prefix(generation).to_vec();
    append_id(&mut key, memory_id)?;
    Ok(key)
}

fn version_key(memory_id: &[u8], lsn: u64) -> Result<Vec<u8>, Error> {
    let mut key = id_prefix(VERSION_PREFIX, memory_id)?;
    key.extend_from_slice(&lsn.to_be_bytes());
    Ok(key)
}

fn fade_key(memory_id: &[u8], lsn: u64) -> Result<Vec<u8>, Error> {
    let mut key = id_prefix(FADE_PREFIX, memory_id)?;
    key.extend_from_slice(&lsn.to_be_bytes());
    Ok(key)
}

fn id_prefix(prefix: u8, memory_id: &[u8]) -> Result<Vec<u8>, Error> {
    let mut key = vec![prefix];
    append_id(&mut key, memory_id)?;
    Ok(key)
}

fn append_id(key: &mut Vec<u8>, memory_id: &[u8]) -> Result<(), Error> {
    let length =
        u16::try_from(memory_id.len()).map_err(|_| Error::new(ErrorCode::InvalidLength))?;
    if length == 0 {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    key.extend_from_slice(&length.to_be_bytes());
    key.extend_from_slice(memory_id);
    Ok(())
}
