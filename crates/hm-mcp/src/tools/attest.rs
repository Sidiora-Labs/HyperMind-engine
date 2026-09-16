#![allow(clippy::missing_errors_doc)]

use crate::Envelope;
use hm_core::{Error, ErrorCode};
use hm_ledger::idempotency::ConnectionId;
use hm_schema::events::AttestationDisposition;
use hm_schema::wire::Attest;
use hm_serve::actor::ActorEngine;
use rmcp::schemars;
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AttestDisposition {
    Used,
    Ignored,
    Helpful,
    Harmful,
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct AttestInput {
    pub provenance: Vec<String>,
    pub disposition: AttestDisposition,
    pub idempotency_key: String,
}

pub async fn run(actor: &ActorEngine, input: AttestInput) -> Result<Envelope, Error> {
    if input.provenance.is_empty() || input.idempotency_key.is_empty() {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let targets = input
        .provenance
        .iter()
        .map(|uri| parse_lsn(actor, uri))
        .collect::<Result<BTreeSet<_>, _>>()?;
    let disposition = match input.disposition {
        AttestDisposition::Used => AttestationDisposition::Used,
        AttestDisposition::Ignored => AttestationDisposition::Ignored,
        AttestDisposition::Helpful => AttestationDisposition::Helpful,
        AttestDisposition::Harmful => AttestationDisposition::Harmful,
    };
    let mut request = Attest {
        client_seq: 1,
        ..Attest::default()
    };
    let values = Some(targets.iter().copied().collect());
    match disposition {
        AttestationDisposition::Used => request.used = values,
        AttestationDisposition::Ignored => request.ignored = values,
        AttestationDisposition::Helpful => request.helpful = values,
        AttestationDisposition::Harmful => request.harmful = values,
    }
    let digest = blake3::hash(input.idempotency_key.as_bytes());
    let mut connection = [0; 16];
    connection.copy_from_slice(&digest.as_bytes()[..16]);
    let connection: ConnectionId = connection;
    let outcome = hm_serve::requests::attest::write(actor, connection, request).await?;
    let mut envelope = Envelope::empty();
    envelope.items.push(json!({
        "disposition": disposition_name(disposition),
        "targets": targets,
        "first_lsn": outcome.first_lsn.get(),
        "last_lsn": outcome.last_lsn.get(),
        "duplicate": outcome.duplicate,
        "manifest_used": matches!(disposition, AttestationDisposition::Used | AttestationDisposition::Helpful)
            .then(|| targets.iter().copied().collect::<Vec<_>>()),
    }));
    for lsn in outcome.first_lsn.get()..=outcome.last_lsn.get() {
        envelope
            .provenance
            .push(format!("hm://{}/lsn/{lsn}", actor.actor()));
    }
    Ok(envelope)
}

fn parse_lsn(actor: &ActorEngine, uri: &str) -> Result<u64, Error> {
    let path = uri
        .strip_prefix("hm://")
        .and_then(|value| value.split('?').next())
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
    let segments = path.split('/').collect::<Vec<_>>();
    segments
        .first()
        .and_then(|value| value.parse::<u16>().ok())
        .filter(|value| *value == actor.actor().get())
        .ok_or_else(|| Error::new(ErrorCode::CapabilityDenied))?;
    segments
        .last()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value != 0)
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))
}

const fn disposition_name(value: AttestationDisposition) -> &'static str {
    match value {
        AttestationDisposition::Used => "used",
        AttestationDisposition::Ignored => "ignored",
        AttestationDisposition::Helpful => "helpful",
        AttestationDisposition::Harmful => "harmful",
    }
}
