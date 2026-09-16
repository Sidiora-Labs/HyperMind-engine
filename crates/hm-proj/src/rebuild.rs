#![allow(clippy::missing_errors_doc)]

use crate::attention::AttentionProjection;
use crate::attestations::AttestationsProjection;
use crate::beliefs::BeliefProjection;
use crate::bindings::BindingsProjection;
use crate::entities::EntityProjection;
use crate::generation::GenerationProjection;
use crate::intent::IntentFrameProjection;
use crate::intentions::IntentionsProjection;
use crate::ladder::TemporalLadder;
use crate::ledger::WorkLedgerProjection;
use crate::lexical::LexicalProjection;
use crate::predictions::PredictionsProjection;
use crate::procedures::ProceduresProjection;
use crate::store::{ProjectionId, ProjectionStore};
use crate::timeline::apply_timeline;
use crate::vocabulary::VocabularyProjection;
use hm_core::{Error, ErrorCode, LSN};
use hm_ledger::frame::Frame;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RebuildProgress {
    pub applied_lsn: LSN,
    pub applied_frames: usize,
    pub complete: bool,
}

#[allow(clippy::too_many_lines)]
pub fn rebuild_projection_stream(
    store: &ProjectionStore,
    frames: &[Frame],
    reset: bool,
    maximum_frames: usize,
) -> Result<RebuildProgress, Error> {
    if reset {
        reset_all(store)?;
    }
    let snapshot = store.begin_snapshot()?;
    let mut timeline_checkpoint = snapshot.checkpoint(ProjectionId::ConversationHeads)?.get();
    let mut lexical_checkpoint = snapshot.checkpoint(ProjectionId::Bm25)?.get();
    let mut intent_checkpoint = snapshot.checkpoint(ProjectionId::IntentFrame)?.get();
    let mut ledger_checkpoint = snapshot.checkpoint(ProjectionId::WorkLedger)?.get();
    let mut bindings_checkpoint = snapshot.checkpoint(ProjectionId::Bindings)?.get();
    let mut entity_checkpoint = snapshot.checkpoint(ProjectionId::EntityIndex)?.get();
    let mut belief_checkpoint = snapshot.checkpoint(ProjectionId::BeliefStore)?.get();
    let mut ladder_checkpoint = snapshot.checkpoint(ProjectionId::TemporalLadder)?.get();
    let mut memories_checkpoint = snapshot.checkpoint(ProjectionId::Memories)?.get();
    let mut graph_checkpoint = snapshot.checkpoint(ProjectionId::Graph)?.get();
    let mut fsrs_checkpoint = snapshot.checkpoint(ProjectionId::Fsrs)?.get();
    let mut runs_checkpoint = snapshot.checkpoint(ProjectionId::Runs)?.get();
    let mut intentions_checkpoint = snapshot.checkpoint(ProjectionId::Intentions)?.get();
    let mut attention_checkpoint = snapshot.checkpoint(ProjectionId::AttentionHistory)?.get();
    let mut predictions_checkpoint = snapshot.checkpoint(ProjectionId::Predictions)?.get();
    let mut procedures_checkpoint = snapshot.checkpoint(ProjectionId::Procedures)?.get();
    let mut attestations_checkpoint = snapshot.checkpoint(ProjectionId::Attestations)?.get();
    let mut vocabulary_checkpoint = snapshot.checkpoint(ProjectionId::Vocabulary)?.get();
    drop(snapshot);
    let checkpoints = [
        timeline_checkpoint,
        lexical_checkpoint,
        intent_checkpoint,
        ledger_checkpoint,
        bindings_checkpoint,
        entity_checkpoint,
        belief_checkpoint,
        ladder_checkpoint,
        memories_checkpoint,
        graph_checkpoint,
        fsrs_checkpoint,
        runs_checkpoint,
        intentions_checkpoint,
        attention_checkpoint,
        predictions_checkpoint,
        procedures_checkpoint,
        attestations_checkpoint,
        vocabulary_checkpoint,
    ];
    let minimum = checkpoints.into_iter().min().unwrap_or(0);
    if checkpoints
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
        if entity_checkpoint < expected_lsn {
            EntityProjection::apply_event(store, frame)?;
            entity_checkpoint = expected_lsn;
        }
        if belief_checkpoint < expected_lsn {
            BeliefProjection::apply_event(store, frame)?;
            belief_checkpoint = expected_lsn;
        }
        if ladder_checkpoint < expected_lsn {
            TemporalLadder::apply_event(store, frame)?;
            ladder_checkpoint = expected_lsn;
        }
        if [
            memories_checkpoint,
            graph_checkpoint,
            fsrs_checkpoint,
            runs_checkpoint,
        ]
        .into_iter()
        .any(|checkpoint| checkpoint < expected_lsn)
        {
            GenerationProjection::apply_event(store, frame)?;
            memories_checkpoint = expected_lsn;
            graph_checkpoint = expected_lsn;
            fsrs_checkpoint = expected_lsn;
            runs_checkpoint = expected_lsn;
        }
        if intentions_checkpoint < expected_lsn {
            IntentionsProjection::apply_event(store, frame)?;
            intentions_checkpoint = expected_lsn;
        }
        if attention_checkpoint < expected_lsn {
            AttentionProjection::apply_event(store, frame)?;
            attention_checkpoint = expected_lsn;
        }
        if predictions_checkpoint < expected_lsn {
            PredictionsProjection::apply_event(store, frame)?;
            predictions_checkpoint = expected_lsn;
        }
        if procedures_checkpoint < expected_lsn {
            ProceduresProjection::apply_event(store, frame)?;
            procedures_checkpoint = expected_lsn;
        }
        if attestations_checkpoint < expected_lsn {
            AttestationsProjection::apply_event(store, frame)?;
            attestations_checkpoint = expected_lsn;
        }
        if vocabulary_checkpoint < expected_lsn {
            VocabularyProjection::apply_event(store, frame)?;
            vocabulary_checkpoint = expected_lsn;
        }
        applied_frames += 1;
    }
    let checkpoints = [
        timeline_checkpoint,
        lexical_checkpoint,
        intent_checkpoint,
        ledger_checkpoint,
        bindings_checkpoint,
        entity_checkpoint,
        belief_checkpoint,
        ladder_checkpoint,
        memories_checkpoint,
        graph_checkpoint,
        fsrs_checkpoint,
        runs_checkpoint,
        intentions_checkpoint,
        attention_checkpoint,
        predictions_checkpoint,
        procedures_checkpoint,
        attestations_checkpoint,
        vocabulary_checkpoint,
    ];
    let applied_lsn = checkpoints.into_iter().min().unwrap_or(0);
    Ok(RebuildProgress {
        applied_lsn: LSN::new(applied_lsn),
        applied_frames,
        complete: checkpoints
            .into_iter()
            .all(|checkpoint| checkpoint == frames.len() as u64),
    })
}

fn reset_all(store: &ProjectionStore) -> Result<(), Error> {
    for projection in [
        ProjectionId::ConversationHeads,
        ProjectionId::Bm25,
        ProjectionId::IntentFrame,
        ProjectionId::WorkLedger,
        ProjectionId::Bindings,
        ProjectionId::EntityIndex,
        ProjectionId::BeliefStore,
        ProjectionId::TemporalLadder,
        ProjectionId::Memories,
        ProjectionId::Graph,
        ProjectionId::Fsrs,
        ProjectionId::Runs,
        ProjectionId::Intentions,
        ProjectionId::AttentionHistory,
        ProjectionId::Predictions,
        ProjectionId::Procedures,
        ProjectionId::Attestations,
        ProjectionId::Vocabulary,
    ] {
        store.reset(projection)?;
    }
    Ok(())
}
