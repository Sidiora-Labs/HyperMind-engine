#![allow(clippy::missing_errors_doc)]

use crate::lexical::LexicalProjection;
use crate::store::{ProjectionId, ProjectionStore};
use crate::timeline::apply_timeline;
use hm_core::{Error, ErrorCode, LSN};
use hm_ledger::frame::Frame;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RebuildProgress {
    pub applied_lsn: LSN,
    pub applied_frames: usize,
    pub complete: bool,
}

pub fn rebuild_projection_stream(
    store: &ProjectionStore,
    frames: &[Frame],
    reset: bool,
    maximum_frames: usize,
) -> Result<RebuildProgress, Error> {
    if reset {
        store.reset(ProjectionId::ConversationHeads)?;
        store.reset(ProjectionId::Bm25)?;
    }
    let snapshot = store.begin_snapshot()?;
    let mut timeline_checkpoint = snapshot.checkpoint(ProjectionId::ConversationHeads)?.get();
    let mut lexical_checkpoint = snapshot.checkpoint(ProjectionId::Bm25)?.get();
    drop(snapshot);
    let minimum = timeline_checkpoint.min(lexical_checkpoint);
    if timeline_checkpoint > frames.len() as u64 || lexical_checkpoint > frames.len() as u64 {
        return Err(Error::new(ErrorCode::ProjectionCheckpoint).at_lsn(LSN::new(minimum)));
    }
    let mut applied_frames = 0_usize;
    let start = usize::try_from(minimum).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
    for frame in frames.iter().skip(start) {
        if applied_frames == maximum_frames {
            break;
        }
        let expected_lsn = applied_frames as u64 + minimum + 1;
        if frame.header.lsn.get() != expected_lsn {
            return Err(Error::new(ErrorCode::SequenceViolation).at_lsn(frame.header.lsn));
        }
        if timeline_checkpoint < expected_lsn {
            apply_timeline(store, frame)?;
            timeline_checkpoint = expected_lsn;
        }
        if lexical_checkpoint < expected_lsn {
            LexicalProjection::apply_event(store, frame)?;
            lexical_checkpoint = expected_lsn;
        }
        applied_frames += 1;
    }
    let applied_lsn = timeline_checkpoint.min(lexical_checkpoint);
    Ok(RebuildProgress {
        applied_lsn: LSN::new(applied_lsn),
        applied_frames,
        complete: timeline_checkpoint == frames.len() as u64
            && lexical_checkpoint == frames.len() as u64,
    })
}
