#![allow(clippy::missing_errors_doc)]

use crate::generation::{decode, encode, should_apply, staged_marker_key, verify_frame};
use crate::runs::RunsProjection;
use crate::store::{Mutation, ProjectionId, ProjectionStore, ReadSnapshot};
use hm_core::{Error, ErrorCode};
use hm_ledger::frame::{EventKind as LedgerEventKind, Frame};
use hm_schema::events::{EventEnvelope, EventPayload, ProvenanceRange};
use roaring::RoaringTreemap;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::Cursor;

const EDGE_PREFIX: u8 = b'E';
const HEAD_PREFIX: u8 = b'H';
const ADJACENCY_PREFIX: u8 = b'A';
const RETRACT_PREFIX: u8 = b'R';

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EdgeCitation {
    pub first_lsn: u64,
    pub last_lsn: u64,
    pub byte_start: u32,
    pub byte_end: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EdgeRecord {
    pub edge_id: Vec<u8>,
    pub source_id: Vec<u8>,
    pub target_id: Vec<u8>,
    pub relation: String,
    pub weight_micros: u32,
    pub valid_from_ns: i64,
    pub valid_to_ns: i64,
    pub citations: Vec<EdgeCitation>,
    pub run_id: Vec<u8>,
    pub generation: u64,
    pub event_lsn: u64,
    pub model_id: String,
    pub prompt_id: String,
    pub prompt_version: u16,
    pub call_id: Vec<u8>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_microusd: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct EdgeRetraction {
    generation: u64,
    event_lsn: u64,
    citations: Vec<EdgeCitation>,
}

pub struct GraphProjection;

impl GraphProjection {
    pub fn apply_event(store: &ProjectionStore, frame: &Frame) -> Result<(), Error> {
        let snapshot = store.begin_snapshot()?;
        if !should_apply(&snapshot, ProjectionId::Graph, frame)? {
            return Ok(());
        }
        let mut mutations = Vec::new();
        match frame.header.kind {
            LedgerEventKind::EdgeAsserted => {
                let verified = verify_frame(frame)?;
                let EventPayload::EdgeAsserted(edge) = &verified.envelope.payload else {
                    return Err(Error::new(ErrorCode::InvalidKind).at_lsn(frame.header.lsn));
                };
                let generation = generation(&snapshot, &verified.envelope)?;
                let model = verified
                    .envelope
                    .model_provenance
                    .as_deref()
                    .ok_or_else(|| Error::new(ErrorCode::SchemaInvalid))?;
                let record = EdgeRecord {
                    edge_id: edge.edge_id.clone(),
                    source_id: edge.source_id.clone(),
                    target_id: edge.target_id.clone(),
                    relation: edge.relation.clone(),
                    weight_micros: edge.weight_micros,
                    valid_from_ns: edge.valid_from_ns,
                    valid_to_ns: edge.valid_to_ns,
                    citations: citations(&edge.citations),
                    run_id: run_id(&verified.envelope)?.to_vec(),
                    generation,
                    event_lsn: frame.header.lsn.get(),
                    model_id: model.model_id.clone(),
                    prompt_id: model.prompt_id.clone(),
                    prompt_version: model.prompt_version,
                    call_id: model.call_id.clone().unwrap_or_default(),
                    input_tokens: model.input_tokens,
                    output_tokens: model.output_tokens,
                    cost_microusd: model.cost_microusd,
                };
                mutations.push(Mutation::put(edge_key(record.event_lsn), encode(&record)?));
                mutations.push(Mutation::put(
                    head_key(generation, &record.edge_id)?,
                    record.event_lsn.to_le_bytes(),
                ));
                for node in [&record.source_id, &record.target_id] {
                    let key = adjacency_key(generation, node)?;
                    let mut adjacency = decode_bitmap(snapshot.get(ProjectionId::Graph, &key)?)?;
                    adjacency.insert(record.event_lsn);
                    mutations.push(Mutation::put(key, encode_bitmap(&adjacency)?));
                }
            }
            LedgerEventKind::EdgeRetracted => {
                let verified = verify_frame(frame)?;
                let EventPayload::EdgeRetracted(edge) = &verified.envelope.payload else {
                    return Err(Error::new(ErrorCode::InvalidKind).at_lsn(frame.header.lsn));
                };
                if latest_edge(&snapshot, &edge.edge_id)?.is_none() {
                    return Err(Error::new(ErrorCode::OrderingViolation).at_lsn(frame.header.lsn));
                }
                let retraction = EdgeRetraction {
                    generation: generation(&snapshot, &verified.envelope)?,
                    event_lsn: frame.header.lsn.get(),
                    citations: citations(&edge.citations),
                };
                mutations.push(Mutation::put(
                    retraction_key(&edge.edge_id, frame.header.lsn.get())?,
                    encode(&retraction)?,
                ));
            }
            _ => {}
        }
        if matches!(
            frame.header.kind,
            LedgerEventKind::EdgeAsserted | LedgerEventKind::EdgeRetracted
        ) {
            mutations.push(Mutation::put(
                staged_marker_key(frame.header.lsn.get()),
                [1],
            ));
        }
        drop(snapshot);
        store.apply(ProjectionId::Graph, frame.header.lsn, &mutations)
    }

    pub fn neighbours(
        snapshot: &ReadSnapshot<'_>,
        generation: u64,
        node_id: &[u8],
        valid_at_ns: i64,
        limit: usize,
    ) -> Result<Vec<EdgeRecord>, Error> {
        if node_id.is_empty() || limit == 0 || !RunsProjection::is_readable(snapshot, generation)? {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let mut seen = BTreeSet::new();
        let mut output = Vec::new();
        for candidate in RunsProjection::lineage(snapshot, generation)? {
            let adjacency = decode_bitmap(
                snapshot.get(ProjectionId::Graph, &adjacency_key(candidate, node_id)?)?,
            )?;
            for lsn in &adjacency {
                let record = read_edge(snapshot, lsn)?;
                if seen.insert(record.edge_id.clone())
                    && valid_at(&record, valid_at_ns)
                    && !is_retracted(snapshot, &record.edge_id)?
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

fn latest_edge(snapshot: &ReadSnapshot<'_>, edge_id: &[u8]) -> Result<Option<EdgeRecord>, Error> {
    let prefix = head_id_suffix(edge_id)?;
    for item in snapshot.scan_prefix(ProjectionId::Graph, &[HEAD_PREFIX], usize::MAX)? {
        if item.key.ends_with(&prefix) {
            return decode_lsn(&item.value)
                .and_then(|lsn| read_edge(snapshot, lsn))
                .map(Some);
        }
    }
    Ok(None)
}

fn read_edge(snapshot: &ReadSnapshot<'_>, lsn: u64) -> Result<EdgeRecord, Error> {
    snapshot
        .get(ProjectionId::Graph, &edge_key(lsn))?
        .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))
        .and_then(|bytes| decode(&bytes))
}

fn is_retracted(snapshot: &ReadSnapshot<'_>, edge_id: &[u8]) -> Result<bool, Error> {
    let prefix = id_prefix(RETRACT_PREFIX, edge_id)?;
    for item in snapshot.scan_prefix_reverse(ProjectionId::Graph, &prefix, usize::MAX)? {
        let retraction: EdgeRetraction = decode(&item.value)?;
        if RunsProjection::is_published(snapshot, retraction.generation)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn valid_at(record: &EdgeRecord, time_ns: i64) -> bool {
    record.valid_from_ns <= time_ns && (record.valid_to_ns == 0 || time_ns <= record.valid_to_ns)
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

fn citations(values: &[ProvenanceRange]) -> Vec<EdgeCitation> {
    values
        .iter()
        .map(|value| EdgeCitation {
            first_lsn: value.first_lsn,
            last_lsn: value.last_lsn,
            byte_start: value.byte_start,
            byte_end: value.byte_end,
        })
        .collect()
}

fn edge_key(lsn: u64) -> [u8; 9] {
    let mut key = [0_u8; 9];
    key[0] = EDGE_PREFIX;
    key[1..].copy_from_slice(&lsn.to_be_bytes());
    key
}

fn head_key(generation: u64, edge_id: &[u8]) -> Result<Vec<u8>, Error> {
    let mut key = Vec::with_capacity(edge_id.len() + 11);
    key.push(HEAD_PREFIX);
    key.extend_from_slice(&generation.to_be_bytes());
    append_id(&mut key, edge_id)?;
    Ok(key)
}

fn adjacency_key(generation: u64, node_id: &[u8]) -> Result<Vec<u8>, Error> {
    let mut key = Vec::with_capacity(node_id.len() + 11);
    key.push(ADJACENCY_PREFIX);
    key.extend_from_slice(&generation.to_be_bytes());
    append_id(&mut key, node_id)?;
    Ok(key)
}

fn retraction_key(edge_id: &[u8], lsn: u64) -> Result<Vec<u8>, Error> {
    let mut key = id_prefix(RETRACT_PREFIX, edge_id)?;
    key.extend_from_slice(&lsn.to_be_bytes());
    Ok(key)
}

fn id_prefix(prefix: u8, id: &[u8]) -> Result<Vec<u8>, Error> {
    let mut key = vec![prefix];
    append_id(&mut key, id)?;
    Ok(key)
}

fn head_id_suffix(edge_id: &[u8]) -> Result<Vec<u8>, Error> {
    let mut suffix = Vec::new();
    append_id(&mut suffix, edge_id)?;
    Ok(suffix)
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

fn encode_bitmap(bitmap: &RoaringTreemap) -> Result<Vec<u8>, Error> {
    let mut bytes = Vec::new();
    bitmap
        .serialize_into(&mut bytes)
        .map_err(|_| Error::new(ErrorCode::InvariantViolation))?;
    Ok(bytes)
}

fn decode_bitmap(bytes: Option<Vec<u8>>) -> Result<RoaringTreemap, Error> {
    bytes.map_or_else(
        || Ok(RoaringTreemap::new()),
        |bytes| {
            RoaringTreemap::deserialize_from(&mut Cursor::new(bytes))
                .map_err(|_| Error::new(ErrorCode::InvariantViolation))
        },
    )
}

fn decode_lsn(bytes: &[u8]) -> Result<u64, Error> {
    bytes
        .try_into()
        .map(u64::from_le_bytes)
        .map_err(|_| Error::new(ErrorCode::InvariantViolation))
}
