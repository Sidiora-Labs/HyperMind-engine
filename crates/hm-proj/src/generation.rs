#![allow(clippy::missing_errors_doc)]

use crate::documents::DocumentsProjection;
use crate::fsrs::FsrsProjection;
use crate::graph::GraphProjection;
use crate::memories::MemoryProjection;
use crate::runs::RunsProjection;
use crate::store::{ProjectionId, ProjectionStore, ReadSnapshot};
use hm_core::{Error, ErrorCode};
use hm_ledger::frame::Frame;
use hm_schema::event::{self, Boundary, EventKind, VerifiedEvent};
use serde::Serialize;
use serde::de::DeserializeOwned;

pub struct GenerationProjection;

impl GenerationProjection {
    pub fn apply_event(store: &ProjectionStore, frame: &Frame) -> Result<(), Error> {
        MemoryProjection::apply_event(store, frame)?;
        GraphProjection::apply_event(store, frame)?;
        FsrsProjection::apply_event(store, frame)?;
        DocumentsProjection::apply_event(store, frame)?;
        RunsProjection::apply_event(store, frame)
    }

    pub fn active_generation(snapshot: &ReadSnapshot<'_>) -> Result<u64, Error> {
        RunsProjection::active_generation(snapshot)
    }
}

pub(crate) fn verify_frame(frame: &Frame) -> Result<VerifiedEvent, Error> {
    let kind = EventKind::try_from(frame.header.kind as u8)
        .map_err(|()| Error::new(ErrorCode::InvalidKind).at_lsn(frame.header.lsn))?;
    event::verify_event(&frame.sealed_payload, kind, Boundary::Disk)
}

pub(crate) fn should_apply(
    snapshot: &ReadSnapshot<'_>,
    projection: ProjectionId,
    frame: &Frame,
) -> Result<bool, Error> {
    let checkpoint = snapshot.checkpoint(projection)?.get();
    let lsn = frame.header.lsn.get();
    if checkpoint == lsn {
        return Ok(false);
    }
    if checkpoint.checked_add(1) != Some(lsn) {
        return Err(Error::new(ErrorCode::ProjectionCheckpoint)
            .at_lsn(frame.header.lsn)
            .at_offset(checkpoint));
    }
    Ok(true)
}

pub(crate) fn staged_marker_key(lsn: u64) -> [u8; 9] {
    let mut key = [0_u8; 9];
    key[0] = b'S';
    key[1..].copy_from_slice(&lsn.to_be_bytes());
    key
}

pub(crate) fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, Error> {
    bincode::serialize(value).map_err(|_| Error::new(ErrorCode::InvariantViolation))
}

pub(crate) fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, Error> {
    bincode::deserialize(bytes).map_err(|_| Error::new(ErrorCode::InvariantViolation))
}
