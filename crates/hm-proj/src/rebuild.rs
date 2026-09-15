#![allow(clippy::missing_errors_doc)]

use crate::bindings::BindingsProjection;
use crate::intent::IntentFrameProjection;
use crate::ledger::WorkLedgerProjection;
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
        store.reset(ProjectionId::IntentFrame)?;
        store.reset(ProjectionId::WorkLedger)?;
        store.reset(ProjectionId::Bindings)?;
    }
    let snapshot = store.begin_snapshot()?;
    let mut timeline_checkpoint = snapshot.checkpoint(ProjectionId::ConversationHeads)?.get();
    let mut lexical_checkpoint = snapshot.checkpoint(ProjectionId::Bm25)?.get();
    let mut intent_checkpoint = snapshot.checkpoint(ProjectionId::IntentFrame)?.get();
    let mut ledger_checkpoint = snapshot.checkpoint(ProjectionId::WorkLedger)?.get();
    let mut bindings_checkpoint = snapshot.checkpoint(ProjectionId::Bindings)?.get();
    drop(snapshot);
    let minimum = [
        timeline_checkpoint,
        lexical_checkpoint,
        intent_checkpoint,
        ledger_checkpoint,
        bindings_checkpoint,
    ]
    .into_iter()
    .min()
    .unwrap_or(0);
    if [
        timeline_checkpoint,
        lexical_checkpoint,
        intent_checkpoint,
        ledger_checkpoint,
        bindings_checkpoint,
    ]
    .into_iter()
    .any(|checkpoint| checkpoint > frames.len() as u64)
    {
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
        if intent_checkpoint < expected_lsn {
            IntentFrameProjection::apply_event(store, frame)?;
            intent_checkpoint = expected_lsn;
        }
        if ledger_checkpoint < expected_lsn {
            WorkLedgerProjection::apply_event(store, frame)?;
            ledger_checkpoint = expected_lsn;
        }
        if bindings_checkpoint < expected_lsn {
            BindingsProjection::apply_event(store, frame)?;
            bindings_checkpoint = expected_lsn;
        }
        applied_frames += 1;
    }
    let applied_lsn = [
        timeline_checkpoint,
        lexical_checkpoint,
        intent_checkpoint,
        ledger_checkpoint,
        bindings_checkpoint,
    ]
    .into_iter()
    .min()
    .unwrap_or(0);
    Ok(RebuildProgress {
        applied_lsn: LSN::new(applied_lsn),
        applied_frames,
        complete: timeline_checkpoint == frames.len() as u64
            && lexical_checkpoint == frames.len() as u64
            && intent_checkpoint == frames.len() as u64
            && ledger_checkpoint == frames.len() as u64
            && bindings_checkpoint == frames.len() as u64,
    })
}
