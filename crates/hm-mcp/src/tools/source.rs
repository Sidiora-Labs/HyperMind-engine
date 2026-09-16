#![allow(clippy::missing_errors_doc)]

use crate::Envelope;
use base64::Engine as _;
use hm_core::{ConversationId, Error, ErrorCode};
use hm_cortex::connectors::{MAXIMUM_DELIVERY_BYTES, retry_delay_ns, retry_exhausted};
use hm_ledger::frame::EventKind;
use hm_schema::event::{CURRENT_SCHEMA_VERSION, MAXIMUM_IDENTIFIER_BYTES, encode_event_envelope};
use hm_schema::events::{
    Authority, ConnectorState, EventEnvelope, EventPayload, ProviderFrame, Retention, Sensitivity,
    SourceDeliveryAccepted, SourceDeliverySettled, SourceDeliveryState, SourceSignatureScheme,
};
use hm_serve::actor::{ActorEngine, IncomingEvent, SourceSignatureRequest};
use rmcp::schemars;
use serde::Deserialize;
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

const MAXIMUM_CONNECTOR_SCAN: usize = 256;
const MAXIMUM_DELIVERY_SCAN: usize = 256;
const SIGNATURE_HEX_BYTES: usize = 64;
const CONNECTOR_ID_HEX_BYTES: usize = 32;
const DELIVERY_CONVERSATION: &str = "source-deliveries";

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct SourceDeliveryInput {
    pub connector_id: String,
    pub delivery_id: String,
    pub event_name: String,
    pub credential_version: u32,
    pub signed_at_ns: i64,
    pub signature: String,
    pub body_base64: String,
    #[serde(default)]
    pub conversation: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SourceOutcome {
    Applied,
    Failed,
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct SourceSettlementInput {
    pub connector_id: String,
    pub delivery_id: String,
    pub attempt: u32,
    pub outcome: SourceOutcome,
    #[serde(default)]
    pub detail: Option<String>,
}

pub async fn deliver(actor: &ActorEngine, input: SourceDeliveryInput) -> Result<Envelope, Error> {
    let connector_id = parse_connector_id(&input.connector_id)?;
    if !bounded_identifier(&input.delivery_id)
        || input.event_name.is_empty()
        || input.credential_version == 0
        || input.signed_at_ns <= 0
    {
        return Err(invalid());
    }
    let signature = parse_signature(&input.signature)?;
    let body = decode_body(&input.body_base64)?;
    let held = bound_connector(actor, &connector_id).await?;
    if let Some(prior) = held_delivery(actor, connector_id, input.delivery_id.as_bytes()).await? {
        return Ok(duplicate_envelope(actor, &input, &prior));
    }
    let verified = match actor
        .verify_source_delivery(SourceSignatureRequest {
            connector_id,
            provider: held.provider.clone(),
            credential_version: input.credential_version,
            delivery_id: input.delivery_id.clone().into_bytes(),
            event_name: input.event_name.clone(),
            signed_at_ns: input.signed_at_ns,
            body: body.clone(),
            signature,
        })
        .await
    {
        Ok(value) => value,
        Err(error) if error.code == ErrorCode::SignatureInvalid => {
            return Ok(not_dispatched(error));
        }
        Err(error) => return Err(error),
    };
    let conversation = ConversationId::derive(
        input
            .conversation
            .as_deref()
            .filter(|value| !value.is_empty())
            .unwrap_or(DELIVERY_CONVERSATION),
    );
    let accepted = EventPayload::SourceDeliveryAccepted(Box::new(SourceDeliveryAccepted {
        connector_id: connector_id.to_vec(),
        delivery_id: input.delivery_id.clone().into_bytes(),
        signature_scheme: SourceSignatureScheme::HmacSha256V0,
        credential_version: input.credential_version,
        signed_at_ns: input.signed_at_ns,
        body_digest: verified.body_digest.to_vec(),
        body_bytes: verified.body_bytes,
        event_name: input.event_name.clone(),
    }));
    let frame = EventPayload::ProviderFrame(Box::new(ProviderFrame {
        provider: held.provider,
        api_content: body,
    }));
    let outcome = actor
        .append(vec![
            IncomingEvent {
                kind: EventKind::SourceDeliveryAccepted,
                conversation,
                payload: external_payload(accepted),
            },
            IncomingEvent {
                kind: EventKind::ProviderFrame,
                conversation,
                payload: external_payload(frame),
            },
        ])
        .await?;
    let accepted_lsn = outcome.first_lsn.get();
    let body_lsn = outcome.last_lsn.get();
    let mut envelope = Envelope::empty();
    envelope.items.push(json!({
        "connector_id": hex(&connector_id),
        "delivery_id": input.delivery_id,
        "event_name": input.event_name,
        "state": delivery_state_name(u8::from(SourceDeliveryState::Accepted)),
        "accepted_lsn": accepted_lsn,
        "body_lsn": body_lsn,
        "body_digest": hex(&verified.body_digest),
        "body_bytes": verified.body_bytes,
        "duplicate": false,
    }));
    envelope.provenance.extend([
        format!("hm://{}/lsn/{accepted_lsn}", actor.actor()),
        format!("hm://{}/lsn/{body_lsn}", actor.actor()),
    ]);
    Ok(envelope)
}

pub async fn settle(actor: &ActorEngine, input: SourceSettlementInput) -> Result<Envelope, Error> {
    let connector_id = parse_connector_id(&input.connector_id)?;
    if !bounded_identifier(&input.delivery_id) || input.attempt == 0 {
        return Err(invalid());
    }
    if input
        .detail
        .as_ref()
        .is_some_and(|detail| detail.is_empty() || detail.len() > MAXIMUM_IDENTIFIER_BYTES)
    {
        return Err(invalid());
    }
    let prior = held_delivery(actor, connector_id, input.delivery_id.as_bytes())
        .await?
        .ok_or_else(|| Error::new(ErrorCode::OrderingViolation))?;
    let state = if retry_exhausted(input.attempt) {
        SourceDeliveryState::Abandoned
    } else {
        match input.outcome {
            SourceOutcome::Applied => SourceDeliveryState::Applied,
            SourceOutcome::Failed => SourceDeliveryState::Failed,
        }
    };
    let next_attempt_at_ns = if state == SourceDeliveryState::Failed {
        wall_time_ns()?
            .checked_add(retry_delay_ns(input.attempt))
            .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?
    } else {
        0
    };
    let detail = input
        .detail
        .filter(|detail| !detail.is_empty())
        .unwrap_or_else(|| delivery_state_name(u8::from(state)).to_owned());
    let settled = EventPayload::SourceDeliverySettled(Box::new(SourceDeliverySettled {
        connector_id: connector_id.to_vec(),
        delivery_id: input.delivery_id.clone().into_bytes(),
        accepted_lsn: prior.accepted_lsn,
        attempt: input.attempt,
        state,
        next_attempt_at_ns,
        detail: detail.clone(),
    }));
    let outcome = actor
        .append(vec![IncomingEvent {
            kind: EventKind::SourceDeliverySettled,
            conversation: ConversationId::derive(DELIVERY_CONVERSATION),
            payload: encode_event_envelope(&EventEnvelope {
                schema_version: CURRENT_SCHEMA_VERSION,
                payload: settled,
                connection_id: None,
                client_seq: 0,
                client_event_index: 0,
                client_event_count: 1,
                origin_actor: 0,
                run_id: None,
                model_provenance: None,
                authority: Authority::RuntimeFact,
                retention: Retention::Durable,
                sensitivity: Sensitivity::Personal,
                event_time_ns: 0,
            }),
        }])
        .await?;
    let settled_lsn = outcome.first_lsn.get();
    let uri = format!("hm://{}/lsn/{settled_lsn}", actor.actor());
    let mut envelope = Envelope::empty();
    envelope.items.push(json!({
        "connector_id": hex(&connector_id),
        "delivery_id": input.delivery_id,
        "attempt": input.attempt,
        "state": delivery_state_name(u8::from(state)),
        "next_attempt_at_ns": next_attempt_at_ns,
        "detail": detail,
        "accepted_lsn": prior.accepted_lsn,
        "settled_lsn": settled_lsn,
        "uri": uri.clone(),
    }));
    envelope.provenance.push(uri);
    Ok(envelope)
}

pub(crate) const fn delivery_state_name(state: u8) -> &'static str {
    match state {
        0 => "accepted",
        1 => "applied",
        2 => "failed",
        3 => "abandoned",
        _ => "unknown",
    }
}

struct HeldConnector {
    provider: String,
}

struct HeldDelivery {
    accepted_lsn: u64,
    body_digest: Vec<u8>,
    body_bytes: u64,
    event_name: String,
    attempt: u32,
    state: u8,
}

async fn bound_connector(
    actor: &ActorEngine,
    connector_id: &[u8; 16],
) -> Result<HeldConnector, Error> {
    actor
        .connectors(MAXIMUM_CONNECTOR_SCAN)
        .await?
        .into_iter()
        .find(|record| {
            record.connector_id == connector_id.as_slice()
                && record.state == u8::from(ConnectorState::Bound)
        })
        .map(|record| HeldConnector {
            provider: record.provider,
        })
        .ok_or_else(|| Error::new(ErrorCode::CapabilityDenied))
}

async fn held_delivery(
    actor: &ActorEngine,
    connector_id: [u8; 16],
    delivery_id: &[u8],
) -> Result<Option<HeldDelivery>, Error> {
    Ok(actor
        .source_deliveries(connector_id, MAXIMUM_DELIVERY_SCAN)
        .await?
        .into_iter()
        .find(|record| record.delivery_id == delivery_id)
        .map(|record| HeldDelivery {
            accepted_lsn: record.accepted_lsn,
            body_digest: record.body_digest,
            body_bytes: record.body_bytes,
            event_name: record.event_name,
            attempt: record.attempt,
            state: record.state,
        }))
}

fn duplicate_envelope(
    actor: &ActorEngine,
    input: &SourceDeliveryInput,
    prior: &HeldDelivery,
) -> Envelope {
    let uri = format!("hm://{}/lsn/{}", actor.actor(), prior.accepted_lsn);
    let mut envelope = Envelope::empty();
    envelope.items.push(json!({
        "connector_id": input.connector_id,
        "delivery_id": input.delivery_id,
        "event_name": prior.event_name,
        "state": delivery_state_name(prior.state),
        "attempt": prior.attempt,
        "accepted_lsn": prior.accepted_lsn,
        "body_digest": hex(&prior.body_digest),
        "body_bytes": prior.body_bytes,
        "duplicate": true,
    }));
    envelope.provenance.push(uri);
    envelope
}

fn not_dispatched(error: Error) -> Envelope {
    let mut envelope = Envelope::error(error, true);
    envelope.effect_state = Some("not_dispatched".to_owned());
    envelope
}

fn external_payload(payload: EventPayload) -> Vec<u8> {
    encode_event_envelope(&EventEnvelope {
        schema_version: CURRENT_SCHEMA_VERSION,
        payload,
        connection_id: None,
        client_seq: 0,
        client_event_index: 0,
        client_event_count: 1,
        origin_actor: 0,
        run_id: None,
        model_provenance: None,
        authority: Authority::ExternalObserved,
        retention: Retention::Durable,
        sensitivity: Sensitivity::Personal,
        event_time_ns: 0,
    })
}

fn wall_time_ns() -> Result<i64, Error> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|elapsed| i64::try_from(elapsed.as_nanos()).ok())
        .ok_or_else(|| Error::new(ErrorCode::OperationUnavailable))
}

fn invalid() -> Error {
    Error::new(ErrorCode::InvalidArgument)
}

fn bounded_identifier(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAXIMUM_IDENTIFIER_BYTES
}

fn decode_body(encoded: &str) -> Result<Vec<u8>, Error> {
    let body = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| invalid())?;
    if body.is_empty() || body.len() > MAXIMUM_DELIVERY_BYTES {
        return Err(invalid());
    }
    Ok(body)
}

fn parse_signature(encoded: &str) -> Result<Vec<u8>, Error> {
    if encoded.len() != SIGNATURE_HEX_BYTES {
        return Err(invalid());
    }
    decode_hex(encoded.as_bytes())
}

fn parse_connector_id(encoded: &str) -> Result<[u8; 16], Error> {
    if encoded.len() != CONNECTOR_ID_HEX_BYTES {
        return Err(invalid());
    }
    let decoded = decode_hex(encoded.as_bytes())?;
    <[u8; 16]>::try_from(decoded.as_slice()).map_err(|_| invalid())
}

fn decode_hex(source: &[u8]) -> Result<Vec<u8>, Error> {
    let mut decoded = Vec::with_capacity(source.len() / 2);
    for pair in source.chunks_exact(2) {
        let high = hex_digit(pair[0]).ok_or_else(invalid)?;
        let low = hex_digit(pair[1]).ok_or_else(invalid)?;
        decoded.push((high << 4) | low);
    }
    Ok(decoded)
}

const fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(
        String::with_capacity(bytes.len() * 2),
        |mut output, byte| {
            let _ = write!(output, "{byte:02x}");
            output
        },
    )
}
