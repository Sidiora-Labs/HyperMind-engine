#![allow(clippy::missing_errors_doc)]

use crate::Envelope;
use base64::Engine as _;
use hm_core::{ConversationId, Error, ErrorCode, LSN};
use hm_ledger::frame::EventKind;
use hm_schema::event::{
    self, Boundary, CURRENT_SCHEMA_VERSION, EventHistory, encode_event_envelope,
};
use hm_schema::events::{
    Attestation, AttestationDisposition, Authority, EventEnvelope, EventPayload, Retention,
    Sensitivity,
};
use hm_serve::actor::{ActorEngine, IncomingEvent};
use hm_serve::config::{CapabilityToken, capability_equal};
use rmcp::schemars;
use serde::Deserialize;
use serde_json::json;

#[derive(Clone, Copy, Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ForgetAction {
    Fade,
    RetractRun,
    CryptoShred,
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct ForgetInput {
    pub action: ForgetAction,
    #[serde(default)]
    pub lsn: Option<u64>,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub admin_token: Option<String>,
}

pub async fn run(
    actor: &ActorEngine,
    admin_token: Option<&CapabilityToken>,
    input: ForgetInput,
) -> Result<Envelope, Error> {
    match input.action {
        ForgetAction::Fade => {
            let lsn = input
                .lsn
                .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
            fade(actor, vec![checked_lsn(lsn)?], "fade").await
        }
        ForgetAction::RetractRun => {
            let run_id = input
                .run_id
                .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
            if run_id.is_empty() {
                return Err(Error::new(ErrorCode::InvalidArgument));
            }
            let frames = actor.frames_since(LSN::new(0), None, usize::MAX).await?;
            let mut history = RunHistory::default();
            let mut targets = Vec::new();
            for frame in frames {
                let kind = event::EventKind::try_from(frame.header.kind as u8)
                    .map_err(|()| Error::new(ErrorCode::InvalidKind))?;
                let verified = event::verify_event_with_history(
                    &frame.sealed_payload,
                    kind,
                    Boundary::Disk,
                    &history,
                )?;
                history
                    .records
                    .insert(frame.header.lsn, (kind, verified.envelope.authority));
                if verified.envelope.run_id.as_deref() == Some(run_id.as_bytes()) {
                    targets.push(frame.header.lsn);
                }
            }
            if targets.is_empty() {
                return Err(Error::new(ErrorCode::InvalidArgument));
            }
            fade(actor, targets, "retract_run").await
        }
        ForgetAction::CryptoShred => {
            let given = input
                .admin_token
                .ok_or_else(|| Error::new(ErrorCode::CapabilityDenied))?;
            let expected = admin_token.ok_or_else(|| Error::new(ErrorCode::CapabilityDenied))?;
            let decoded = decode_token(&given)?;
            if !capability_equal(expected, &decoded) {
                return Err(Error::new(ErrorCode::CapabilityDenied));
            }
            let receipt = actor.crypto_delete().await?;
            let mut envelope = Envelope::empty();
            envelope.items.push(json!({
                "action": "crypto_shred",
                "actor": actor.actor().get(),
                "receipt_base64": base64::engine::general_purpose::STANDARD.encode(receipt),
            }));
            Ok(envelope)
        }
    }
}

#[derive(Default)]
struct RunHistory {
    records: std::collections::BTreeMap<LSN, (event::EventKind, Authority)>,
}

impl EventHistory for RunHistory {
    fn kind_at(&self, lsn: LSN) -> Option<event::EventKind> {
        self.records.get(&lsn).map(|record| record.0)
    }

    fn authority_at(&self, lsn: LSN) -> Option<Authority> {
        self.records.get(&lsn).map(|record| record.1)
    }
}

async fn fade(actor: &ActorEngine, targets: Vec<LSN>, action: &str) -> Result<Envelope, Error> {
    actor.guard_tripwires(targets.clone()).await?;
    let conversation = ConversationId::derive("hypermind.forget");
    let mut first_lsn = None;
    let mut last_lsn = None;
    for chunk in targets.chunks(hm_schema::protocol::MAXIMUM_BATCH_EVENTS) {
        let events = chunk
            .iter()
            .map(|target| IncomingEvent {
                kind: EventKind::Attestation,
                conversation,
                payload: encode_event_envelope(&EventEnvelope {
                    schema_version: CURRENT_SCHEMA_VERSION,
                    payload: EventPayload::Attestation(Box::new(Attestation {
                        target_lsn: target.get(),
                        disposition: AttestationDisposition::Ignored,
                    })),
                    connection_id: None,
                    client_seq: 0,
                    client_event_index: 0,
                    client_event_count: 0,
                    origin_actor: actor.actor().get(),
                    run_id: None,
                    model_provenance: None,
                    authority: Authority::RuntimeFact,
                    retention: Retention::Durable,
                    sensitivity: Sensitivity::Personal,
                    event_time_ns: 0,
                }),
            })
            .collect();
        let outcome = actor.append(events).await?;
        first_lsn.get_or_insert(outcome.first_lsn);
        last_lsn = Some(outcome.last_lsn);
    }
    let mut envelope = Envelope::empty();
    envelope.items.push(json!({
        "action": action,
        "targets": targets.iter().map(|lsn| lsn.get()).collect::<Vec<_>>(),
        "first_lsn": first_lsn.map_or(0, LSN::get),
        "last_lsn": last_lsn.map_or(0, LSN::get),
    }));
    Ok(envelope)
}

fn checked_lsn(value: u64) -> Result<LSN, Error> {
    if value == 0 {
        Err(Error::new(ErrorCode::InvalidArgument))
    } else {
        Ok(LSN::new(value))
    }
}

fn decode_token(encoded: &str) -> Result<CapabilityToken, Error> {
    if encoded.len() != 64 || !encoded.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(Error::new(ErrorCode::CapabilityDenied));
    }
    let mut token = [0; 32];
    for (index, byte) in token.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&encoded[index * 2..index * 2 + 2], 16)
            .map_err(|_| Error::new(ErrorCode::CapabilityDenied))?;
    }
    Ok(token)
}
