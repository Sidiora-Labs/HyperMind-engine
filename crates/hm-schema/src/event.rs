#![allow(clippy::missing_errors_doc)]

use crate::events::{
    AttestationDisposition, EventEnvelope, EventEnvelopeRef, EventPayload, ToolResult,
};
use hm_core::{Error, ErrorCode, LSN};
use planus::ReadAsRoot;

pub const CURRENT_SCHEMA_VERSION: u16 = 2;
pub const MAXIMUM_EVENT_BYTES: usize = 16 * 1024 * 1024;
pub const MAXIMUM_IDENTIFIER_BYTES: usize = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Boundary {
    Disk,
    Socket,
    Import,
}

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
}

impl EventKind {
    #[must_use]
    pub const fn is_wave_one(self) -> bool {
        matches!(
            self,
            Self::UserMsg
                | Self::DeliveredMsg
                | Self::ToolCall
                | Self::ToolResult
                | Self::Reasoning
                | Self::Attestation
        )
    }
}

impl TryFrom<u8> for EventKind {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::UserMsg),
            2 => Ok(Self::DeliveredMsg),
            3 => Ok(Self::ToolCall),
            4 => Ok(Self::ToolResult),
            5 => Ok(Self::Reasoning),
            6 => Ok(Self::ProviderFrame),
            7 => Ok(Self::MediaRef),
            8 => Ok(Self::Effect),
            9 => Ok(Self::Approval),
            10 => Ok(Self::Outcome),
            11 => Ok(Self::Checkpoint),
            12 => Ok(Self::Supervisor),
            13 => Ok(Self::Recovery),
            14 => Ok(Self::IntentSet),
            15 => Ok(Self::LoopOpened),
            16 => Ok(Self::LoopClosed),
            17 => Ok(Self::Assertion),
            18 => Ok(Self::Consolidation),
            19 => Ok(Self::Embedding),
            20 => Ok(Self::Retract),
            21 => Ok(Self::Attestation),
            _ => Err(()),
        }
    }
}

pub trait EventHistory {
    fn kind_at(&self, lsn: LSN) -> Option<EventKind>;
}

impl<F> EventHistory for F
where
    F: Fn(LSN) -> Option<EventKind>,
{
    fn kind_at(&self, lsn: LSN) -> Option<EventKind> {
        self(lsn)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct VerifiedEvent {
    pub envelope: EventEnvelope,
    pub kind: EventKind,
    pub boundary: Boundary,
}

#[must_use]
pub fn encode_event_envelope(envelope: &EventEnvelope) -> Vec<u8> {
    let mut builder = planus::Builder::new();
    let encoded = builder.finish(envelope, None);
    finish_with_identifier(encoded, *b"NCEV")
}

pub fn verify_event(
    encoded: &[u8],
    expected_kind: EventKind,
    boundary: Boundary,
) -> Result<VerifiedEvent, Error> {
    verify_event_with_history(encoded, expected_kind, boundary, &|_| None)
}

pub fn verify_event_with_history(
    encoded: &[u8],
    expected_kind: EventKind,
    boundary: Boundary,
    history: &impl EventHistory,
) -> Result<VerifiedEvent, Error> {
    if encoded.is_empty() || encoded.len() > MAXIMUM_EVENT_BYTES {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    require_identifier(encoded, *b"NCEV", ErrorCode::SchemaInvalid)?;
    let envelope_ref = EventEnvelopeRef::read_as_root(encoded)
        .map_err(|_| Error::new(ErrorCode::SchemaInvalid))?;
    let envelope =
        EventEnvelope::try_from(envelope_ref).map_err(|_| Error::new(ErrorCode::SchemaInvalid))?;
    if envelope.schema_version == 0 || envelope.schema_version > CURRENT_SCHEMA_VERSION {
        return Err(Error::new(ErrorCode::SchemaVersion));
    }
    if !expected_kind.is_wave_one() || payload_kind(&envelope.payload) != expected_kind {
        return Err(Error::new(ErrorCode::ForbiddenKind));
    }
    validate_envelope(&envelope)?;
    validate_payload(&envelope.payload, history)?;
    Ok(VerifiedEvent {
        envelope,
        kind: expected_kind,
        boundary,
    })
}

fn require_identifier(encoded: &[u8], identifier: [u8; 4], code: ErrorCode) -> Result<(), Error> {
    if encoded.get(4..8) == Some(identifier.as_slice()) {
        Ok(())
    } else {
        Err(Error::new(code))
    }
}

fn finish_with_identifier(encoded: &[u8], identifier: [u8; 4]) -> Vec<u8> {
    let root_offset = u32::from_le_bytes(encoded[..4].try_into().expect("Planus root offset"));
    let mut output = Vec::with_capacity(encoded.len() + identifier.len());
    output.extend_from_slice(&(root_offset + 4).to_le_bytes());
    output.extend_from_slice(&identifier);
    output.extend_from_slice(&encoded[4..]);
    output
}

fn validate_envelope(envelope: &EventEnvelope) -> Result<(), Error> {
    if envelope
        .connection_id
        .as_ref()
        .is_some_and(|value| value.len() != 16)
        || envelope
            .run_id
            .as_ref()
            .is_some_and(|value| value.is_empty() || value.len() > MAXIMUM_IDENTIFIER_BYTES)
        || (envelope.client_event_count == 0 && envelope.client_event_index != 0)
        || (envelope.client_event_count != 0
            && envelope.client_event_index >= envelope.client_event_count)
    {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    if let Some(model) = &envelope.model_provenance
        && (model.model_id.is_empty() || model.prompt_id.is_empty())
    {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    Ok(())
}

fn validate_payload(payload: &EventPayload, history: &impl EventHistory) -> Result<(), Error> {
    match payload {
        EventPayload::UserMsg(_) | EventPayload::DeliveredMsg(_) | EventPayload::Reasoning(_) => {
            Ok(())
        }
        EventPayload::ToolCall(value) => {
            if bounded_identifier(&value.call_id) && !value.tool_name.is_empty() {
                Ok(())
            } else {
                Err(Error::new(ErrorCode::SchemaInvalid))
            }
        }
        EventPayload::ToolResult(value) => validate_tool_result(value, history),
        EventPayload::Attestation(value) => {
            if value.target_lsn != 0
                && matches!(
                    value.disposition,
                    AttestationDisposition::Used | AttestationDisposition::Ignored
                )
            {
                Ok(())
            } else {
                Err(Error::new(ErrorCode::SchemaInvalid))
            }
        }
        _ => Err(Error::new(ErrorCode::ForbiddenKind)),
    }
}

fn validate_tool_result(value: &ToolResult, history: &impl EventHistory) -> Result<(), Error> {
    if !bounded_identifier(&value.call_id) || value.tool_call_lsn == 0 {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    let referenced_lsn = LSN::new(value.tool_call_lsn);
    if history.kind_at(referenced_lsn) != Some(EventKind::ToolCall) {
        return Err(Error::new(ErrorCode::OrderingViolation).at_lsn(referenced_lsn));
    }
    Ok(())
}

fn bounded_identifier(value: &[u8]) -> bool {
    !value.is_empty() && value.len() <= MAXIMUM_IDENTIFIER_BYTES
}

fn payload_kind(payload: &EventPayload) -> EventKind {
    match payload {
        EventPayload::UserMsg(_) => EventKind::UserMsg,
        EventPayload::DeliveredMsg(_) => EventKind::DeliveredMsg,
        EventPayload::ToolCall(_) => EventKind::ToolCall,
        EventPayload::ToolResult(_) => EventKind::ToolResult,
        EventPayload::Reasoning(_) => EventKind::Reasoning,
        EventPayload::ProviderFrame(_) => EventKind::ProviderFrame,
        EventPayload::MediaRef(_) => EventKind::MediaRef,
        EventPayload::Effect(_) => EventKind::Effect,
        EventPayload::Approval(_) => EventKind::Approval,
        EventPayload::Outcome(_) => EventKind::Outcome,
        EventPayload::Checkpoint(_) => EventKind::Checkpoint,
        EventPayload::Supervisor(_) => EventKind::Supervisor,
        EventPayload::Recovery(_) => EventKind::Recovery,
        EventPayload::IntentSet(_) => EventKind::IntentSet,
        EventPayload::LoopOpened(_) => EventKind::LoopOpened,
        EventPayload::LoopClosed(_) => EventKind::LoopClosed,
        EventPayload::Assertion(_) => EventKind::Assertion,
        EventPayload::Consolidation(_) => EventKind::Consolidation,
        EventPayload::Embedding(_) => EventKind::Embedding,
        EventPayload::Retract(_) => EventKind::Retract,
        EventPayload::Attestation(_) => EventKind::Attestation,
    }
}
