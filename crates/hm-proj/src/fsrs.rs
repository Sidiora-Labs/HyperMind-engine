#![allow(clippy::missing_errors_doc)]

use crate::generation::{decode, encode, should_apply, staged_marker_key, verify_frame};
use crate::runs::RunsProjection;
use crate::store::{Mutation, ProjectionId, ProjectionStore, ReadSnapshot};
use hm_core::{Error, ErrorCode};
use hm_ledger::frame::{EventKind as LedgerEventKind, Frame};
use hm_schema::events::{EventEnvelope, EventPayload, ReviewRating};
use serde::{Deserialize, Serialize};

const HEAD_PREFIX: u8 = b'H';
const VERSION_PREFIX: u8 = b'V';

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FsrsState {
    pub memory_id: Vec<u8>,
    pub rating: ReviewRating,
    pub source_lsn: u64,
    pub reviewed_at_ns: i64,
    pub stability_millis: u64,
    pub difficulty_micros: u32,
    pub due_at_ns: i64,
    pub run_id: Vec<u8>,
    pub generation: u64,
    pub event_lsn: u64,
}

pub struct FsrsProjection;

impl FsrsProjection {
    pub fn apply_event(store: &ProjectionStore, frame: &Frame) -> Result<(), Error> {
        let snapshot = store.begin_snapshot()?;
        if !should_apply(&snapshot, ProjectionId::Fsrs, frame)? {
            return Ok(());
        }
        let mut mutations = Vec::new();
        if frame.header.kind == LedgerEventKind::Reviewed {
            let verified = verify_frame(frame)?;
            let EventPayload::Reviewed(reviewed) = &verified.envelope.payload else {
                return Err(Error::new(ErrorCode::InvalidKind).at_lsn(frame.header.lsn));
            };
            let run_id = run_id(&verified.envelope)?;
            let state = FsrsState {
                memory_id: reviewed.memory_id.clone(),
                rating: reviewed.rating,
                source_lsn: reviewed.source_lsn,
                reviewed_at_ns: reviewed.reviewed_at_ns,
                stability_millis: reviewed.stability_millis,
                difficulty_micros: reviewed.difficulty_micros,
                due_at_ns: reviewed.due_at_ns,
                run_id: run_id.to_vec(),
                generation: RunsProjection::generation_for_run(&snapshot, run_id)?,
                event_lsn: frame.header.lsn.get(),
            };
            let bytes = encode(&state)?;
            mutations.push(Mutation::put(
                head_key(state.generation, &state.memory_id)?,
                bytes.clone(),
            ));
            mutations.push(Mutation::put(
                version_key(&state.memory_id, state.event_lsn)?,
                bytes,
            ));
            mutations.push(Mutation::put(
                staged_marker_key(frame.header.lsn.get()),
                [1],
            ));
        }
        drop(snapshot);
        store.apply(ProjectionId::Fsrs, frame.header.lsn, &mutations)
    }

    pub fn get_visible(
        snapshot: &ReadSnapshot<'_>,
        generation: u64,
        memory_id: &[u8],
    ) -> Result<Option<FsrsState>, Error> {
        if !RunsProjection::is_readable(snapshot, generation)? {
            return Err(Error::new(ErrorCode::OperationUnavailable));
        }
        for candidate in RunsProjection::lineage(snapshot, generation)? {
            if let Some(bytes) =
                snapshot.get(ProjectionId::Fsrs, &head_key(candidate, memory_id)?)?
            {
                return decode(&bytes).map(Some);
            }
        }
        Ok(None)
    }
}

fn run_id(envelope: &EventEnvelope) -> Result<&[u8], Error> {
    envelope
        .run_id
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| Error::new(ErrorCode::SchemaInvalid))
}

fn head_key(generation: u64, memory_id: &[u8]) -> Result<Vec<u8>, Error> {
    let mut key = Vec::with_capacity(memory_id.len() + 11);
    key.push(HEAD_PREFIX);
    key.extend_from_slice(&generation.to_be_bytes());
    append_id(&mut key, memory_id)?;
    Ok(key)
}

fn version_key(memory_id: &[u8], lsn: u64) -> Result<Vec<u8>, Error> {
    let mut key = vec![VERSION_PREFIX];
    append_id(&mut key, memory_id)?;
    key.extend_from_slice(&lsn.to_be_bytes());
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
