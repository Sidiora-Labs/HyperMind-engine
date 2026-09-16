#![allow(clippy::missing_errors_doc)]

use crate::store::ReadSnapshot;
use crate::timeline::{read_conversation_record, scan_latest_conversation_record_of_kind};
use hm_core::{ConversationId, Error, ErrorCode, LSN};
use hm_ledger::frame::{EventKind, Frame};
use hm_schema::event::{self, Boundary, EventHistory};
use hm_schema::events::{Authority, EventPayload};

pub const MAXIMUM_TURN_BYTES: usize = 4096;
pub const LATEST_CHECKPOINT_SCAN: usize = 65_536;
const TURN_DOMAIN: &[u8] = b"neocortex-turn-v1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckpointRead {
    pub lsn: LSN,
    pub blob: Vec<u8>,
}

#[must_use]
pub fn turn_conversation(turn_id: &[u8]) -> ConversationId {
    let mut hasher = blake3::Hasher::new();
    hasher.update(TURN_DOMAIN);
    hasher.update(turn_id);
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
    ConversationId::new(bytes)
}

pub fn encode_checkpoint_cursor(turn_id: &[u8], blob: &[u8]) -> Result<Vec<u8>, Error> {
    if turn_id.is_empty() || turn_id.len() > MAXIMUM_TURN_BYTES || blob.is_empty() {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let length = u16::try_from(turn_id.len()).map_err(|_| Error::new(ErrorCode::InvalidLength))?;
    let mut cursor = Vec::with_capacity(2 + turn_id.len() + blob.len());
    cursor.extend_from_slice(&length.to_le_bytes());
    cursor.extend_from_slice(turn_id);
    cursor.extend_from_slice(blob);
    Ok(cursor)
}

pub fn decode_checkpoint_cursor<'a>(cursor: &'a [u8], turn_id: &[u8]) -> Result<&'a [u8], Error> {
    if cursor.len() < 2 {
        return Err(Error::new(ErrorCode::InvalidLength));
    }
    let length = usize::from(u16::from_le_bytes([cursor[0], cursor[1]]));
    if length == 0 || length > cursor.len() - 2 {
        return Err(Error::new(ErrorCode::InvalidLength));
    }
    if cursor.get(2..2 + length) != Some(turn_id) {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    Ok(&cursor[2 + length..])
}

pub fn latest_checkpoint(
    snapshot: &ReadSnapshot<'_>,
    turn_id: &[u8],
) -> Result<Option<CheckpointRead>, Error> {
    if turn_id.is_empty() || turn_id.len() > MAXIMUM_TURN_BYTES {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let conversation = turn_conversation(turn_id);
    let mut before_lsn = LSN::new(u64::MAX);
    let mut remaining = LATEST_CHECKPOINT_SCAN;
    while remaining > 0 {
        let scan = scan_latest_conversation_record_of_kind(
            snapshot,
            conversation,
            EventKind::Checkpoint,
            before_lsn,
            remaining,
        )?;
        remaining -= scan.scanned.min(remaining);
        if let Some(record) = scan.record {
            let verified = event::verify_event(
                &record.payload,
                event::EventKind::Checkpoint,
                Boundary::Disk,
            )
            .map_err(|error| error.at_lsn(record.lsn))?;
            let EventPayload::Checkpoint(checkpoint) = verified.envelope.payload else {
                return Err(Error::new(ErrorCode::InvariantViolation).at_lsn(record.lsn));
            };
            if let Ok(blob) = decode_checkpoint_cursor(&checkpoint.cursor, turn_id) {
                return Ok(Some(CheckpointRead {
                    lsn: record.lsn,
                    blob: blob.to_vec(),
                }));
            }
            before_lsn = record.lsn;
        } else if scan.next_before_lsn.get() == 0 {
            return Ok(None);
        } else {
            before_lsn = scan.next_before_lsn;
        }
    }
    Err(Error::new(ErrorCode::CapacityExceeded))
}

pub(crate) struct ProjectionHistory<'snapshot, 'environment> {
    snapshot: &'snapshot ReadSnapshot<'environment>,
}

impl<'snapshot, 'environment> ProjectionHistory<'snapshot, 'environment> {
    pub(crate) const fn new(snapshot: &'snapshot ReadSnapshot<'environment>) -> Self {
        Self { snapshot }
    }
}

impl EventHistory for ProjectionHistory<'_, '_> {
    fn kind_at(&self, lsn: LSN) -> Option<event::EventKind> {
        if let Some(record) = read_conversation_record(self.snapshot, lsn).ok().flatten() {
            return schema_kind(record.kind);
        }
        let mut key = [0_u8; 9];
        key[0] = b'N';
        key[1..].copy_from_slice(&lsn.get().to_be_bytes());
        self.snapshot
            .get(crate::store::ProjectionId::WorkLedger, &key)
            .ok()
            .flatten()
            .map(|_| event::EventKind::ToolCall)
    }

    fn authority_at(&self, lsn: LSN) -> Option<Authority> {
        let record = read_conversation_record(self.snapshot, lsn).ok()??;
        let kind = schema_kind(record.kind)?;
        event::verify_event_with_history(&record.payload, kind, Boundary::Disk, self)
            .ok()
            .map(|verified| verified.envelope.authority)
    }
}

pub(crate) fn verify_frame(
    snapshot: &ReadSnapshot<'_>,
    frame: &Frame,
) -> Result<hm_schema::events::EventEnvelope, Error> {
    let kind = schema_kind(frame.header.kind)
        .ok_or_else(|| Error::new(ErrorCode::ForbiddenKind).at_lsn(frame.header.lsn))?;
    event::verify_event_with_history(
        &frame.sealed_payload,
        kind,
        Boundary::Disk,
        &ProjectionHistory::new(snapshot),
    )
    .map(|verified| verified.envelope)
    .map_err(|error| error.at_lsn(frame.header.lsn))
}

pub(crate) const fn schema_kind(kind: EventKind) -> Option<event::EventKind> {
    Some(match kind {
        EventKind::UserMsg => event::EventKind::UserMsg,
        EventKind::DeliveredMsg => event::EventKind::DeliveredMsg,
        EventKind::ToolCall => event::EventKind::ToolCall,
        EventKind::ToolResult => event::EventKind::ToolResult,
        EventKind::Reasoning => event::EventKind::Reasoning,
        EventKind::ProviderFrame => event::EventKind::ProviderFrame,
        EventKind::MediaRef => event::EventKind::MediaRef,
        EventKind::Effect => event::EventKind::Effect,
        EventKind::Approval => event::EventKind::Approval,
        EventKind::Outcome => event::EventKind::Outcome,
        EventKind::Checkpoint => event::EventKind::Checkpoint,
        EventKind::Supervisor => event::EventKind::Supervisor,
        EventKind::Recovery => event::EventKind::Recovery,
        EventKind::IntentSet => event::EventKind::IntentSet,
        EventKind::LoopOpened => event::EventKind::LoopOpened,
        EventKind::LoopClosed => event::EventKind::LoopClosed,
        EventKind::Attestation => event::EventKind::Attestation,
        EventKind::Binding => event::EventKind::Binding,
        EventKind::ProposedAssertion => event::EventKind::ProposedAssertion,
        EventKind::MemoryMinted => event::EventKind::MemoryMinted,
        EventKind::MemoryRevised => event::EventKind::MemoryRevised,
        EventKind::MemoryMerged => event::EventKind::MemoryMerged,
        EventKind::MemoryFaded => event::EventKind::MemoryFaded,
        EventKind::EdgeAsserted => event::EventKind::EdgeAsserted,
        EventKind::EdgeRetracted => event::EventKind::EdgeRetracted,
        EventKind::ConsolidationOpened => event::EventKind::ConsolidationOpened,
        EventKind::ConsolidationPhase => event::EventKind::ConsolidationPhase,
        EventKind::ConsolidationClosed => event::EventKind::ConsolidationClosed,
        EventKind::ConsolidationRetracted => event::EventKind::ConsolidationRetracted,
        EventKind::Reviewed => event::EventKind::Reviewed,
        EventKind::IntentionSet => event::EventKind::IntentionSet,
        EventKind::IntentionFired => event::EventKind::IntentionFired,
        EventKind::AttentionDecided => event::EventKind::AttentionDecided,
        EventKind::IntentionCancelled => event::EventKind::IntentionCancelled,
        EventKind::Predicted => event::EventKind::Predicted,
        EventKind::OutcomeObserved => event::EventKind::OutcomeObserved,
        EventKind::ProcedureMined => event::EventKind::ProcedureMined,
        EventKind::ProcedureRevised => event::EventKind::ProcedureRevised,
        EventKind::ProcedureAdopted => event::EventKind::ProcedureAdopted,
        EventKind::VocabularyImported => event::EventKind::VocabularyImported,
        EventKind::DocumentIngested => event::EventKind::DocumentIngested,
        EventKind::DocumentExtracted => event::EventKind::DocumentExtracted,
        EventKind::DocumentChunked => event::EventKind::DocumentChunked,
        EventKind::SourceConnectorBound => event::EventKind::SourceConnectorBound,
        EventKind::SourceDeliveryAccepted => event::EventKind::SourceDeliveryAccepted,
        EventKind::SourceDeliverySettled => event::EventKind::SourceDeliverySettled,
        EventKind::SourceRevisionObserved => event::EventKind::SourceRevisionObserved,
        EventKind::Assertion
        | EventKind::Consolidation
        | EventKind::Embedding
        | EventKind::Retract => return None,
    })
}
