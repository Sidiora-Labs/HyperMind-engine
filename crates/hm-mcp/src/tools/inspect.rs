#![allow(clippy::missing_errors_doc)]

use crate::Envelope;
use hm_core::{Error, ErrorCode, LSN};
use hm_schema::event::{self, Boundary, EventHistory};
use hm_schema::events::{Authority, EventPayload};
use hm_serve::actor::ActorEngine;
use rmcp::schemars;
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, Deserialize, schemars::JsonSchema)]
pub struct InspectInput {
    #[serde(default)]
    pub uri: Option<String>,
}

pub async fn run(actor: &ActorEngine, input: InspectInput) -> Result<Envelope, Error> {
    let stats = actor.stats().await?;
    let verification = actor.verification_status().await?;
    let mut envelope = Envelope::empty();
    envelope.items.push(json!({
        "actor": stats.actor.get(),
        "log_events": stats.log_events,
        "log_bytes": stats.log_bytes,
        "applied_lsn": stats.applied.last_lsn.get(),
        "applied_digest": hex(&stats.applied.rolling_digest),
        "verification": {
            "verified": verification.verified,
            "root": hex(&verification.root),
            "leaf_count": verification.leaf_count,
            "last_checkpoint_lsn": verification.last_checkpoint_lsn.get(),
        },
        "projections": stats.projections.iter().map(|projection| json!({
            "name": projection.name,
            "applied_lsn": projection.applied_lsn.get(),
        })).collect::<Vec<_>>(),
    }));
    let Some(uri) = input.uri else {
        return Ok(envelope);
    };
    let all_frames = actor.frames_since(LSN::new(0), None, usize::MAX).await?;
    let mut history = InspectHistory::default();
    for frame in &all_frames {
        let kind = event::EventKind::try_from(frame.header.kind as u8)
            .map_err(|()| Error::new(ErrorCode::InvalidKind).at_lsn(frame.header.lsn))?;
        let verified = event::verify_event_with_history(
            &frame.sealed_payload,
            kind,
            Boundary::Disk,
            &history,
        )?;
        history
            .records
            .insert(frame.header.lsn, (kind, verified.envelope.authority));
    }
    let first = parse_lsn(&uri)?;
    let mut pending = vec![first];
    let mut visited = BTreeSet::new();
    while let Some(lsn) = pending.pop() {
        if !visited.insert(lsn) {
            continue;
        }
        if visited.len() > 256 {
            return Err(Error::new(ErrorCode::CapacityExceeded));
        }
        let frame = actor
            .frames_since(LSN::new(lsn.get() - 1), None, 1)
            .await?
            .into_iter()
            .next()
            .filter(|frame| frame.header.lsn == lsn)
            .ok_or_else(|| Error::new(ErrorCode::InvalidArgument).at_lsn(lsn))?;
        let kind = event::EventKind::try_from(frame.header.kind as u8)
            .map_err(|()| Error::new(ErrorCode::InvalidKind).at_lsn(lsn))?;
        let verified = event::verify_event_with_history(
            &frame.sealed_payload,
            kind,
            Boundary::Disk,
            &history,
        )?;
        pending.extend(references(&verified.envelope.payload).into_iter().rev());
        let integrity = actor.integrity_at(lsn).await?;
        let item_uri = format!("hm://{}/lsn/{}", actor.actor(), lsn.get());
        envelope.items.push(json!({
            "uri": item_uri,
            "lsn": lsn.get(),
            "kind": format!("{kind:?}").to_lowercase(),
            "authority": authority_name(verified.envelope.authority),
            "integrity": {
                "leaf_hash": hex(&integrity.leaf_hash),
                "root_at_lsn": hex(&integrity.root),
                "checkpoint_lsn": integrity.checkpoint_lsn.get(),
            },
        }));
        envelope.provenance.push(item_uri);
    }
    Ok(envelope)
}

#[derive(Default)]
struct InspectHistory {
    records: std::collections::BTreeMap<LSN, (event::EventKind, Authority)>,
}

impl EventHistory for InspectHistory {
    fn kind_at(&self, lsn: LSN) -> Option<event::EventKind> {
        self.records.get(&lsn).map(|record| record.0)
    }

    fn authority_at(&self, lsn: LSN) -> Option<Authority> {
        self.records.get(&lsn).map(|record| record.1)
    }
}

fn parse_lsn(uri: &str) -> Result<LSN, Error> {
    if !uri.starts_with("hm://") {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let path = uri.split('?').next().unwrap_or(uri);
    let raw = path
        .rsplit('/')
        .next()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value != 0)
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
    Ok(LSN::new(raw))
}

fn references(payload: &EventPayload) -> Vec<LSN> {
    let raw = match payload {
        EventPayload::ToolResult(value) => vec![value.tool_call_lsn],
        EventPayload::Effect(value) => vec![value.tool_call_lsn],
        EventPayload::Outcome(value) => value.evidence_lsns.clone().unwrap_or_default(),
        EventPayload::LoopClosed(value) => value.evidence_lsns.clone().unwrap_or_default(),
        EventPayload::Binding(value) => vec![value.evidence_lsn],
        EventPayload::Attestation(value) => vec![value.target_lsn],
        _ => Vec::new(),
    };
    raw.into_iter()
        .filter(|value| *value != 0)
        .map(LSN::new)
        .collect()
}

const fn authority_name(authority: Authority) -> &'static str {
    match authority {
        Authority::UserAsserted => "user_asserted",
        Authority::ExternalObserved => "external_observed",
        Authority::ToolObserved => "tool_observed",
        Authority::RuntimeFact => "runtime_fact",
        Authority::AssistantGenerated => "assistant_generated",
        Authority::DerivedInference => "derived_inference",
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
