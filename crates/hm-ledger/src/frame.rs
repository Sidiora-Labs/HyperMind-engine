#![allow(clippy::missing_errors_doc)]

use hm_core::{ActorId, ConversationId, Error, ErrorCode, LSN, UtcNanos};

pub const FRAME_HEADER_SIZE: usize = 43;
pub const MAXIMUM_FRAME_BYTES: usize = 16 * 1024 * 1024;

const LENGTH_OFFSET: usize = 0;
const CHECKSUM_OFFSET: usize = 4;
const LSN_OFFSET: usize = 8;
const KIND_OFFSET: usize = 16;
const WALL_TIMESTAMP_OFFSET: usize = 17;
const ACTOR_OFFSET: usize = 25;
const CONVERSATION_OFFSET: usize = 27;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum EventKind {
    UserMsg = 1,
    DeliveredMsg = 2,
    ToolCall = 3,
    ToolResult = 4,
    Reasoning = 5,
    ProviderFrame = 6,
    MediaRef = 7,
    Effect = 8,
    Approval = 9,
    Outcome = 10,
    Checkpoint = 11,
    Supervisor = 12,
    Recovery = 13,
    IntentSet = 14,
    LoopOpened = 15,
    LoopClosed = 16,
    Assertion = 17,
    Consolidation = 18,
    Embedding = 19,
    Retract = 20,
    Attestation = 21,
    Binding = 22,
    ProposedAssertion = 23,
    MemoryMinted = 24,
    MemoryRevised = 25,
    MemoryMerged = 26,
    MemoryFaded = 27,
    EdgeAsserted = 28,
    EdgeRetracted = 29,
    ConsolidationOpened = 30,
    ConsolidationPhase = 31,
    ConsolidationClosed = 32,
    ConsolidationRetracted = 33,
    Reviewed = 34,
    IntentionSet = 35,
    IntentionFired = 36,
    AttentionDecided = 37,
    IntentionCancelled = 38,
    Predicted = 39,
    OutcomeObserved = 40,
    ProcedureMined = 41,
    ProcedureRevised = 42,
    ProcedureAdopted = 43,
    VocabularyImported = 44,
    DocumentIngested = 45,
    DocumentExtracted = 46,
    DocumentChunked = 47,
    SourceConnectorBound = 48,
    SourceDeliveryAccepted = 49,
    SourceDeliverySettled = 50,
    SourceRevisionObserved = 51,
    ProcedureImported = 52,
    ProcedureImprovementProposed = 53,
}

impl TryFrom<u8> for EventKind {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        const KINDS: [EventKind; 53] = [
            EventKind::UserMsg,
            EventKind::DeliveredMsg,
            EventKind::ToolCall,
            EventKind::ToolResult,
            EventKind::Reasoning,
            EventKind::ProviderFrame,
            EventKind::MediaRef,
            EventKind::Effect,
            EventKind::Approval,
            EventKind::Outcome,
            EventKind::Checkpoint,
            EventKind::Supervisor,
            EventKind::Recovery,
            EventKind::IntentSet,
            EventKind::LoopOpened,
            EventKind::LoopClosed,
            EventKind::Assertion,
            EventKind::Consolidation,
            EventKind::Embedding,
            EventKind::Retract,
            EventKind::Attestation,
            EventKind::Binding,
            EventKind::ProposedAssertion,
            EventKind::MemoryMinted,
            EventKind::MemoryRevised,
            EventKind::MemoryMerged,
            EventKind::MemoryFaded,
            EventKind::EdgeAsserted,
            EventKind::EdgeRetracted,
            EventKind::ConsolidationOpened,
            EventKind::ConsolidationPhase,
            EventKind::ConsolidationClosed,
            EventKind::ConsolidationRetracted,
            EventKind::Reviewed,
            EventKind::IntentionSet,
            EventKind::IntentionFired,
            EventKind::AttentionDecided,
            EventKind::IntentionCancelled,
            EventKind::Predicted,
            EventKind::OutcomeObserved,
            EventKind::ProcedureMined,
            EventKind::ProcedureRevised,
            EventKind::ProcedureAdopted,
            EventKind::VocabularyImported,
            EventKind::DocumentIngested,
            EventKind::DocumentExtracted,
            EventKind::DocumentChunked,
            EventKind::SourceConnectorBound,
            EventKind::SourceDeliveryAccepted,
            EventKind::SourceDeliverySettled,
            EventKind::SourceRevisionObserved,
            EventKind::ProcedureImported,
            EventKind::ProcedureImprovementProposed,
        ];
        let index = usize::from(value.saturating_sub(1));
        if value == 0 {
            return Err(Error::new(ErrorCode::InvalidKind));
        }
        KINDS
            .get(index)
            .copied()
            .ok_or_else(|| Error::new(ErrorCode::InvalidKind))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameHeader {
    pub lsn: LSN,
    pub kind: EventKind,
    pub wall_timestamp_ns: UtcNanos,
    pub actor: ActorId,
    pub conversation: ConversationId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frame {
    pub header: FrameHeader,
    pub sealed_payload: Vec<u8>,
}

#[must_use]
pub fn encoded_frame_length(prefix: &[u8]) -> Option<usize> {
    let bytes: [u8; 4] = prefix.get(..4)?.try_into().ok()?;
    Some(u32::from_le_bytes(bytes) as usize)
}

pub fn encode(frame: &Frame) -> Result<Vec<u8>, Error> {
    if frame.header.lsn.get() == 0 {
        return Err(Error::new(ErrorCode::SequenceViolation));
    }
    let total = FRAME_HEADER_SIZE
        .checked_add(frame.sealed_payload.len())
        .ok_or_else(|| Error::new(ErrorCode::InvalidLength))?;
    if total > MAXIMUM_FRAME_BYTES || total > u32::MAX as usize {
        return Err(Error::new(ErrorCode::InvalidLength));
    }

    let total_u32 = u32::try_from(total).map_err(|_| Error::new(ErrorCode::InvalidLength))?;
    let mut encoded = vec![0_u8; total];
    encoded[LENGTH_OFFSET..LENGTH_OFFSET + 4].copy_from_slice(&total_u32.to_le_bytes());
    encoded[LSN_OFFSET..LSN_OFFSET + 8].copy_from_slice(&frame.header.lsn.get().to_le_bytes());
    encoded[KIND_OFFSET] = frame.header.kind as u8;
    encoded[WALL_TIMESTAMP_OFFSET..WALL_TIMESTAMP_OFFSET + 8]
        .copy_from_slice(&frame.header.wall_timestamp_ns.get().to_le_bytes());
    encoded[ACTOR_OFFSET..ACTOR_OFFSET + 2]
        .copy_from_slice(&frame.header.actor.get().to_le_bytes());
    encoded[CONVERSATION_OFFSET..FRAME_HEADER_SIZE]
        .copy_from_slice(frame.header.conversation.as_bytes());
    encoded[FRAME_HEADER_SIZE..].copy_from_slice(&frame.sealed_payload);
    let checksum = crc32c::crc32c(&encoded[LSN_OFFSET..]);
    encoded[CHECKSUM_OFFSET..CHECKSUM_OFFSET + 4].copy_from_slice(&checksum.to_le_bytes());
    Ok(encoded)
}

pub fn decode(encoded: &[u8]) -> Result<Frame, Error> {
    if encoded.len() < FRAME_HEADER_SIZE {
        return Err(Error::new(ErrorCode::Truncated));
    }
    let declared = encoded_frame_length(encoded).ok_or_else(|| Error::new(ErrorCode::Truncated))?;
    if !(FRAME_HEADER_SIZE..=MAXIMUM_FRAME_BYTES).contains(&declared) {
        return Err(Error::new(ErrorCode::InvalidLength));
    }
    if declared != encoded.len() {
        return Err(Error::new(if declared > encoded.len() {
            ErrorCode::Truncated
        } else {
            ErrorCode::InvalidLength
        }));
    }

    let expected = u32::from_le_bytes(copy_array(encoded, CHECKSUM_OFFSET)?);
    let actual = crc32c::crc32c(&encoded[LSN_OFFSET..]);
    if expected != actual {
        return Err(Error::new(ErrorCode::ChecksumMismatch));
    }
    let lsn = u64::from_le_bytes(copy_array(encoded, LSN_OFFSET)?);
    if lsn == 0 {
        return Err(Error::new(ErrorCode::SequenceViolation));
    }
    let kind = EventKind::try_from(encoded[KIND_OFFSET])?;
    let wall_timestamp_ns = i64::from_le_bytes(copy_array(encoded, WALL_TIMESTAMP_OFFSET)?);
    let actor = u16::from_le_bytes(copy_array(encoded, ACTOR_OFFSET)?);
    let conversation = ConversationId::new(copy_array(encoded, CONVERSATION_OFFSET)?);

    Ok(Frame {
        header: FrameHeader {
            lsn: LSN::new(lsn),
            kind,
            wall_timestamp_ns: UtcNanos::new(wall_timestamp_ns),
            actor: ActorId::new(actor),
            conversation,
        },
        sealed_payload: encoded[FRAME_HEADER_SIZE..].to_vec(),
    })
}

fn copy_array<const N: usize>(bytes: &[u8], offset: usize) -> Result<[u8; N], Error> {
    bytes
        .get(offset..offset + N)
        .ok_or_else(|| Error::new(ErrorCode::Truncated))?
        .try_into()
        .map_err(|_| Error::new(ErrorCode::Truncated))
}
