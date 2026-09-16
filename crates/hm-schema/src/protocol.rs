#![allow(clippy::missing_errors_doc)]

use crate::event::{EventKind, MAXIMUM_EVENT_BYTES};
use crate::wire::{
    Activate, Append, AsOf, Attest, Checkpoint, Event as EventPush, Hello, LatestCheckpoint,
    Recall, Request, RequestPayload, Subscribe, Transcript, WireEnvelope, WireEnvelopeRef,
    WirePayload,
};
use hm_core::{Error, ErrorCode};
use planus::ReadAsRoot;

pub const COMPATIBLE_PROTOCOL_VERSION: u16 = 2;
pub const CURRENT_PROTOCOL_VERSION: u16 = 3;
pub const MAXIMUM_PROTOCOL_PAYLOAD_BYTES: usize = 17 * 1024 * 1024;
pub const MAXIMUM_BATCH_EVENTS: usize = 256;
pub const MAXIMUM_QUERY_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, PartialEq)]
pub struct VerifiedRequest {
    pub proto_version: u16,
    pub request: Request,
}

#[must_use]
pub fn encode_wire_envelope(envelope: &WireEnvelope) -> Vec<u8> {
    let mut builder = planus::Builder::new();
    let encoded = builder.finish(envelope, None);
    finish_with_identifier(encoded, *b"NCPR")
}

pub fn verify_wire_envelope(encoded: &[u8]) -> Result<WireEnvelope, Error> {
    if encoded.is_empty() || encoded.len() > MAXIMUM_PROTOCOL_PAYLOAD_BYTES {
        return Err(Error::new(ErrorCode::ProtocolInvalid));
    }
    require_identifier(encoded)?;
    let envelope_ref = WireEnvelopeRef::read_as_root(encoded)
        .map_err(|_| Error::new(ErrorCode::ProtocolInvalid))?;
    let envelope =
        WireEnvelope::try_from(envelope_ref).map_err(|_| Error::new(ErrorCode::ProtocolInvalid))?;
    if !matches!(
        envelope.proto_version,
        COMPATIBLE_PROTOCOL_VERSION | CURRENT_PROTOCOL_VERSION
    ) {
        return Err(Error::new(ErrorCode::ProtocolVersion));
    }
    if let WirePayload::Hello(hello) = &envelope.payload {
        validate_hello(hello)?;
    }
    if let WirePayload::Event(event) = &envelope.payload {
        validate_event_push(event)?;
    }
    Ok(envelope)
}

pub fn verify_request(encoded: &[u8]) -> Result<VerifiedRequest, Error> {
    let envelope = verify_wire_envelope(encoded)?;
    let WirePayload::Request(request) = envelope.payload else {
        return Err(Error::new(ErrorCode::ProtocolInvalid));
    };
    validate_request(&request)?;
    Ok(VerifiedRequest {
        proto_version: envelope.proto_version,
        request: *request,
    })
}

pub fn validate_request(request: &Request) -> Result<(), Error> {
    if request.request_id == 0 {
        return Err(Error::new(ErrorCode::ProtocolInvalid));
    }
    match &request.payload {
        RequestPayload::Append(value) => validate_append(value),
        RequestPayload::Activate(value) => validate_activate(value),
        RequestPayload::Transcript(value) => validate_transcript(value),
        RequestPayload::Recall(value) => validate_recall(value),
        RequestPayload::AsOf(value) => validate_asof(value),
        RequestPayload::Checkpoint(value) => validate_checkpoint(value),
        RequestPayload::LatestCheckpoint(value) => validate_latest_checkpoint(value),
        RequestPayload::Attest(value) => validate_attest(value),
        RequestPayload::Subscribe(value) => validate_subscribe(value),
        RequestPayload::ToolRequest(value) => {
            if matches!(
                value.verb.as_str(),
                "remember"
                    | "recall"
                    | "activate"
                    | "believe"
                    | "retract"
                    | "dispute"
                    | "intend"
                    | "bind"
                    | "predict"
                    | "outcome"
                    | "attest"
                    | "consolidate"
                    | "inspect"
                    | "forget"
            ) && !value.arguments_json.is_empty()
                && value.arguments_json.len() <= MAXIMUM_QUERY_BYTES
                && std::str::from_utf8(&value.arguments_json).is_ok()
            {
                Ok(())
            } else {
                Err(Error::new(ErrorCode::ProtocolInvalid))
            }
        }
        RequestPayload::Health(_) | RequestPayload::LatencyHistograms(_) => Ok(()),
        RequestPayload::Stats(value) if value.actor != 0 => Ok(()),
        RequestPayload::Stats(_) => Err(Error::new(ErrorCode::ProtocolInvalid)),
        RequestPayload::VerifyStatus(value) if value.actor != 0 => Ok(()),
        RequestPayload::RebuildProjection(value) if value.actor != 0 && !value.name.is_empty() => {
            Ok(())
        }
        RequestPayload::CryptoDelete(value) if value.actor != 0 => Ok(()),
        _ => Err(Error::new(ErrorCode::OperationUnavailable)),
    }
}

fn require_identifier(encoded: &[u8]) -> Result<(), Error> {
    if encoded.get(4..8) == Some(b"NCPR".as_slice()) {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::ProtocolInvalid))
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

fn validate_hello(hello: &Hello) -> Result<(), Error> {
    if !matches!(
        hello.proto_version,
        COMPATIBLE_PROTOCOL_VERSION | CURRENT_PROTOCOL_VERSION
    ) {
        return Err(Error::new(ErrorCode::ProtocolVersion));
    }
    if hello.connection_id.len() != 16 || hello.capability_token.is_empty() {
        return Err(Error::new(ErrorCode::ProtocolInvalid));
    }
    Ok(())
}

fn validate_append(append: &Append) -> Result<(), Error> {
    if append.client_seq == 0
        || append.events.is_empty()
        || append.events.len() > MAXIMUM_BATCH_EVENTS
    {
        return Err(Error::new(ErrorCode::ProtocolInvalid));
    }
    for event in &append.events {
        let kind =
            EventKind::try_from(event.kind).map_err(|()| Error::new(ErrorCode::ProtocolInvalid))?;
        if !kind.is_wave_five()
            || event.conversation.len() != 16
            || event.payload.is_empty()
            || event.payload.len() > MAXIMUM_EVENT_BYTES
        {
            return Err(Error::new(ErrorCode::ProtocolInvalid));
        }
    }
    Ok(())
}

fn validate_activate(request: &Activate) -> Result<(), Error> {
    if request.conversation.len() != 16
        || request.query.len() > MAXIMUM_QUERY_BYTES
        || request
            .turn_text
            .as_ref()
            .is_some_and(|value| value.len() > MAXIMUM_QUERY_BYTES)
        || request.budget_tokens == 0
        || request.budget_tokens > u64::from(u32::MAX)
        || request.temporal_to_ns < request.temporal_from_ns
        || request
            .token_weights
            .as_ref()
            .is_none_or(|weights| weights.len() != 256)
    {
        return Err(Error::new(ErrorCode::ProtocolInvalid));
    }
    match (
        request.query_embedding.as_ref(),
        request.query_binary_prefilter.as_ref(),
    ) {
        (None, None) => Ok(()),
        (Some(embedding), Some(prefilter))
            if !embedding.is_empty()
                && embedding.len() <= 65_536
                && prefilter.len() == embedding.len().div_ceil(8) =>
        {
            Ok(())
        }
        _ => Err(Error::new(ErrorCode::ProtocolInvalid)),
    }
}

fn validate_transcript(request: &Transcript) -> Result<(), Error> {
    if request.conversation.len() == 16 && (1..=16_384).contains(&request.limit) {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::ProtocolInvalid))
    }
}

fn validate_recall(request: &Recall) -> Result<(), Error> {
    if request.query.len() <= MAXIMUM_QUERY_BYTES
        && (1..=4096).contains(&request.limit)
        && request.level <= 3
        && request.end_ns >= request.start_ns
    {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::ProtocolInvalid))
    }
}

fn validate_asof(request: &AsOf) -> Result<(), Error> {
    let valid_axis = request.valid_time_ns != 0;
    let known_axis = request.known_lsn != 0;
    if request.belief_type <= 4
        && !request.canonical_identity.is_empty()
        && request.canonical_identity.len() <= 4096
        && request.transaction_lsn == 0
        && valid_axis != known_axis
    {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::ProtocolInvalid))
    }
}

fn validate_checkpoint(request: &Checkpoint) -> Result<(), Error> {
    if request.turn_id.is_empty()
        || request.turn_id.len() > 4096
        || request.blob.is_empty()
        || request.blob.len() > MAXIMUM_EVENT_BYTES
        || request.client_seq == 0
    {
        Err(Error::new(ErrorCode::ProtocolInvalid))
    } else {
        Ok(())
    }
}

fn validate_latest_checkpoint(request: &LatestCheckpoint) -> Result<(), Error> {
    if request.turn_id.is_empty() || request.turn_id.len() > 4096 {
        Err(Error::new(ErrorCode::ProtocolInvalid))
    } else {
        Ok(())
    }
}

fn validate_subscribe(request: &Subscribe) -> Result<(), Error> {
    if request
        .conversation
        .as_ref()
        .is_some_and(|conversation| conversation.len() != 16)
    {
        Err(Error::new(ErrorCode::ProtocolInvalid))
    } else {
        Ok(())
    }
}

fn validate_attest(request: &Attest) -> Result<(), Error> {
    let groups = [
        &request.used,
        &request.ignored,
        &request.helpful,
        &request.harmful,
    ];
    let count: usize = groups
        .iter()
        .map(|group| group.as_ref().map_or(0, Vec::len))
        .sum();
    if request.client_seq == 0 || count == 0 || count > MAXIMUM_BATCH_EVENTS {
        return Err(Error::new(ErrorCode::ProtocolInvalid));
    }
    let mut targets = std::collections::BTreeSet::new();
    for target in groups
        .into_iter()
        .flat_map(|group| group.as_deref().unwrap_or_default())
    {
        if *target == 0 || !targets.insert(*target) {
            return Err(Error::new(ErrorCode::ProtocolInvalid));
        }
    }
    Ok(())
}

fn validate_event_push(event: &EventPush) -> Result<(), Error> {
    let kind =
        EventKind::try_from(event.kind).map_err(|()| Error::new(ErrorCode::ProtocolInvalid))?;
    if event.subscription_id == 0
        || event.lsn == 0
        || !kind.is_wave_seven()
        || event.actor == 0
        || event.conversation.len() != 16
        || event.payload.is_empty()
        || event.payload.len() > MAXIMUM_EVENT_BYTES
    {
        Err(Error::new(ErrorCode::ProtocolInvalid))
    } else {
        Ok(())
    }
}
