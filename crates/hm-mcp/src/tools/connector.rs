#![allow(clippy::missing_errors_doc)]

use crate::Envelope;
use base64::Engine as _;
use hm_core::{ConversationId, Error, ErrorCode};
use hm_ledger::frame::EventKind;
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    Authority, ConnectorState, EventEnvelope, EventPayload, Retention, Sensitivity,
    SourceConnectorBound, SourceSignatureScheme,
};
use hm_serve::actor::{ActorEngine, IncomingEvent};
use rmcp::schemars;
use serde::Deserialize;
use serde_json::json;

const DEFAULT_CONSENT_TTL_NS: i64 = 600_000_000_000;
const MAXIMUM_CONNECTOR_SCAN: usize = 256;
const MAXIMUM_SECRET_BYTES: usize = 4096;
const CONNECTOR_ID_HEX_BYTES: usize = 32;
const CONNECTOR_CONVERSATION: &str = "source-connectors";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorAction {
    Consent,
    Bind,
    Rotate,
    Revoke,
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct ConnectorInput {
    pub action: ConnectorAction,
    pub provider: String,
    pub connector_id: String,
    #[serde(default)]
    pub external_account: Option<String>,
    #[serde(default)]
    pub consent_state: Option<String>,
    #[serde(default)]
    pub consent_ttl_ns: Option<i64>,
    #[serde(default)]
    pub credential_version: Option<u32>,
    #[serde(default)]
    pub signing_secret: Option<String>,
    #[serde(default)]
    pub scopes: Option<Vec<String>>,
}

pub async fn run(actor: &ActorEngine, input: ConnectorInput) -> Result<Envelope, Error> {
    if input.provider.is_empty() {
        return Err(invalid());
    }
    let connector_id = parse_connector_id(&input.connector_id)?;
    match input.action {
        ConnectorAction::Consent => consent(actor, input, connector_id).await,
        ConnectorAction::Bind => bind(actor, input, connector_id).await,
        ConnectorAction::Rotate => rotate(actor, input, connector_id).await,
        ConnectorAction::Revoke => revoke(actor, input, connector_id).await,
    }
}

async fn consent(
    actor: &ActorEngine,
    input: ConnectorInput,
    connector_id: [u8; 16],
) -> Result<Envelope, Error> {
    let ttl_ns = consent_ttl(input.consent_ttl_ns)?;
    let mint = actor
        .mint_consent_state(input.provider.clone(), connector_id, ttl_ns)
        .await?;
    let mut envelope = Envelope::empty();
    envelope.items.push(json!({
        "action": "consent",
        "provider": input.provider,
        "connector_id": hex(&connector_id),
        "consent_state": mint.state,
        "consent_nonce": hex(&mint.nonce),
        "expires_at_ns": mint.expires_at_ns,
    }));
    Ok(envelope)
}

async fn bind(
    actor: &ActorEngine,
    input: ConnectorInput,
    connector_id: [u8; 16],
) -> Result<Envelope, Error> {
    let external_account = required_text(input.external_account)?;
    let state = required_text(input.consent_state)?;
    let credential_version = required_version(input.credential_version)?;
    let secret = decode_secret(input.signing_secret.as_deref())?;
    let scopes = checked_scopes(input.scopes.ok_or_else(invalid)?)?;
    let grant = actor
        .redeem_consent_state(input.provider.clone(), connector_id, state)
        .await?;
    actor
        .store_connector_credential(
            input.provider.clone(),
            connector_id,
            credential_version,
            secret,
        )
        .await?;
    append(
        actor,
        "bind",
        SourceConnectorBound {
            connector_id: connector_id.to_vec(),
            provider: input.provider,
            external_account,
            consent_nonce: grant.nonce.to_vec(),
            consent_expires_at_ns: grant.expires_at_ns,
            credential_version,
            signature_scheme: SourceSignatureScheme::HmacSha256V0,
            scopes,
            state: ConnectorState::Bound,
        },
    )
    .await
}

async fn rotate(
    actor: &ActorEngine,
    input: ConnectorInput,
    connector_id: [u8; 16],
) -> Result<Envelope, Error> {
    let credential_version = required_version(input.credential_version)?;
    let secret = decode_secret(input.signing_secret.as_deref())?;
    let ttl_ns = consent_ttl(input.consent_ttl_ns)?;
    let held = bound_connector(actor, &connector_id).await?;
    let scopes = match input.scopes {
        Some(values) => checked_scopes(values)?,
        None => held.scopes,
    };
    let mint = actor
        .mint_consent_state(input.provider.clone(), connector_id, ttl_ns)
        .await?;
    actor
        .store_connector_credential(
            input.provider.clone(),
            connector_id,
            credential_version,
            secret,
        )
        .await?;
    append(
        actor,
        "rotate",
        SourceConnectorBound {
            connector_id: connector_id.to_vec(),
            provider: input.provider,
            external_account: held.external_account,
            consent_nonce: mint.nonce.to_vec(),
            consent_expires_at_ns: mint.expires_at_ns,
            credential_version,
            signature_scheme: SourceSignatureScheme::HmacSha256V0,
            scopes,
            state: ConnectorState::Bound,
        },
    )
    .await
}

async fn revoke(
    actor: &ActorEngine,
    input: ConnectorInput,
    connector_id: [u8; 16],
) -> Result<Envelope, Error> {
    let ttl_ns = consent_ttl(input.consent_ttl_ns)?;
    let held = bound_connector(actor, &connector_id).await?;
    let mint = actor
        .mint_consent_state(input.provider.clone(), connector_id, ttl_ns)
        .await?;
    append(
        actor,
        "revoke",
        SourceConnectorBound {
            connector_id: connector_id.to_vec(),
            provider: input.provider,
            external_account: held.external_account,
            consent_nonce: mint.nonce.to_vec(),
            consent_expires_at_ns: mint.expires_at_ns,
            credential_version: held.credential_version,
            signature_scheme: SourceSignatureScheme::HmacSha256V0,
            scopes: held.scopes,
            state: ConnectorState::Revoked,
        },
    )
    .await
}

async fn append(
    actor: &ActorEngine,
    action: &str,
    record: SourceConnectorBound,
) -> Result<Envelope, Error> {
    let mut item = json!({
        "action": action,
        "provider": &record.provider,
        "connector_id": hex(&record.connector_id),
        "external_account": &record.external_account,
        "credential_version": record.credential_version,
        "signature_scheme": signature_scheme_name(u8::from(record.signature_scheme)),
        "scopes": &record.scopes,
        "state": connector_state_name(u8::from(record.state)),
    });
    let outcome = actor
        .append(vec![IncomingEvent {
            kind: EventKind::SourceConnectorBound,
            conversation: ConversationId::derive(CONNECTOR_CONVERSATION),
            payload: encode_event_envelope(&EventEnvelope {
                schema_version: CURRENT_SCHEMA_VERSION,
                payload: EventPayload::SourceConnectorBound(Box::new(record)),
                connection_id: None,
                client_seq: 0,
                client_event_index: 0,
                client_event_count: 1,
                origin_actor: 0,
                run_id: None,
                model_provenance: None,
                authority: Authority::UserAsserted,
                retention: Retention::Durable,
                sensitivity: Sensitivity::Personal,
                event_time_ns: 0,
            }),
        }])
        .await?;
    let uri = format!("hm://{}/lsn/{}", actor.actor(), outcome.first_lsn.get());
    let mut envelope = Envelope::empty();
    item["lsn"] = json!(outcome.first_lsn.get());
    item["uri"] = json!(uri);
    envelope.items.push(item);
    envelope.provenance.push(uri);
    Ok(envelope)
}

struct HeldConnector {
    external_account: String,
    credential_version: u32,
    scopes: Vec<String>,
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
            external_account: record.external_account,
            credential_version: record.credential_version,
            scopes: record.scopes,
        })
        .ok_or_else(|| Error::new(ErrorCode::CapabilityDenied))
}

pub(crate) const fn connector_state_name(state: u8) -> &'static str {
    match state {
        0 => "bound",
        1 => "revoked",
        _ => "unknown",
    }
}

pub(crate) const fn signature_scheme_name(scheme: u8) -> &'static str {
    match scheme {
        0 => "hmac_sha256_v0",
        _ => "unknown",
    }
}

fn invalid() -> Error {
    Error::new(ErrorCode::InvalidArgument)
}

fn consent_ttl(value: Option<i64>) -> Result<i64, Error> {
    let ttl_ns = value.unwrap_or(DEFAULT_CONSENT_TTL_NS);
    if ttl_ns <= 0 {
        return Err(invalid());
    }
    Ok(ttl_ns)
}

fn required_text(value: Option<String>) -> Result<String, Error> {
    value.filter(|text| !text.is_empty()).ok_or_else(invalid)
}

fn required_version(value: Option<u32>) -> Result<u32, Error> {
    value.filter(|version| *version != 0).ok_or_else(invalid)
}

fn decode_secret(value: Option<&str>) -> Result<Vec<u8>, Error> {
    let encoded = value.ok_or_else(invalid)?;
    let secret = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| invalid())?;
    if secret.is_empty() || secret.len() > MAXIMUM_SECRET_BYTES {
        return Err(invalid());
    }
    Ok(secret)
}

fn checked_scopes(values: Vec<String>) -> Result<Vec<String>, Error> {
    if values.is_empty() || values.iter().any(String::is_empty) {
        return Err(invalid());
    }
    Ok(values)
}

fn parse_connector_id(encoded: &str) -> Result<[u8; 16], Error> {
    let source = encoded.as_bytes();
    if source.len() != CONNECTOR_ID_HEX_BYTES {
        return Err(invalid());
    }
    let mut connector_id = [0_u8; 16];
    for (index, byte) in connector_id.iter_mut().enumerate() {
        let high = hex_digit(source[index * 2]).ok_or_else(invalid)?;
        let low = hex_digit(source[index * 2 + 1]).ok_or_else(invalid)?;
        *byte = (high << 4) | low;
    }
    Ok(connector_id)
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
