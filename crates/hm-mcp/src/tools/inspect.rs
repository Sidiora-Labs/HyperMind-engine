#![allow(clippy::missing_errors_doc)]

use super::surfaces::{self, Availability, Surface};
use crate::Envelope;
use hm_core::{Error, ErrorCode, LSN};
use hm_schema::event::{self, Boundary, EventHistory};
use hm_schema::events::{AttentionDecision, Authority, EventPayload};
use hm_serve::actor::ActorEngine;
use rmcp::schemars;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InspectMode {
    #[default]
    Status,
    Discover,
}

#[derive(Clone, Debug, Default, Deserialize, schemars::JsonSchema)]
pub struct InspectInput {
    #[serde(default)]
    pub uri: Option<String>,
    #[serde(default)]
    pub mode: InspectMode,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[allow(clippy::too_many_lines)]
pub async fn run(
    actor: &ActorEngine,
    availability: Availability,
    input: InspectInput,
) -> Result<Envelope, Error> {
    if matches!(input.mode, InspectMode::Discover) {
        return Ok(discover(actor, availability, &input));
    }
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
    if uri == format!("hm://{}/attention", actor.actor()) {
        let history = actor.attention_history(256).await?;
        envelope.items[0]["attention"] = json!(
            history
                .iter()
                .map(|record| json!({
                    "lsn": record.lsn,
                    "intention_id": String::from_utf8_lossy(&record.intention_id),
                    "wake_id": hex(&record.wake_id),
                    "decision": attention_name(record.decision),
                    "reason": record.reason,
                }))
                .collect::<Vec<_>>()
        );
        envelope.provenance.extend(
            history
                .iter()
                .map(|record| format!("hm://{}/lsn/{}", actor.actor(), record.lsn)),
        );
        return Ok(envelope);
    }
    if uri == format!("hm://{}/calibration", actor.actor()) {
        let counters = actor.calibration().await?;
        envelope.items[0]["calibration"] = json!(
            counters
                .iter()
                .map(|(kind, counts)| json!({
                    "predicate_kind": super::predict::kind_name(*kind),
                    "supported": counts.supported,
                    "contradicted": counts.contradicted,
                    "pending": counts.pending,
                    "unresolvable": counts.unresolvable,
                    "not_executed": counts.not_executed,
                }))
                .collect::<Vec<_>>()
        );
        return Ok(envelope);
    }
    if uri == format!("hm://{}/sources", actor.actor()) {
        let sources = super::sources::index(actor).await?;
        envelope.items.extend(sources.items);
        envelope.provenance.extend(sources.provenance);
        return Ok(envelope);
    }
    if let Some(conversation_hex) = uri.strip_prefix(&format!("hm://{}/sources/", actor.actor())) {
        let source = super::sources::detail(actor, conversation_hex).await?;
        envelope.items.extend(source.items);
        envelope.provenance.extend(source.provenance);
        return Ok(envelope);
    }
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

fn discover(actor: &ActorEngine, availability: Availability, input: &InspectInput) -> Envelope {
    let limit = input
        .limit
        .unwrap_or(surfaces::DEFAULT_DISCOVERY_LIMIT)
        .clamp(1, surfaces::MAXIMUM_DISCOVERY_LIMIT);
    let matched = surfaces::search(input.query.as_deref(), surfaces::MAXIMUM_DISCOVERY_LIMIT);
    let mut envelope = Envelope::empty();
    for surface in matched.iter().copied().take(limit) {
        envelope.items.push(discovery_item(surface, availability));
    }
    envelope.health = json!({
        "actor": actor.actor().get(),
        "advertised_tools": surfaces::ADVERTISED_TOOLS,
        "discoverable_surfaces": surfaces::SURFACES.len(),
        "matched": matched.len(),
        "returned": envelope.items.len(),
        "truncated": envelope.items.len() < matched.len(),
    });
    envelope
        .warnings
        .push("discovery_is_not_authorization".to_owned());
    envelope
}

fn discovery_item(surface: &Surface, availability: Availability) -> Value {
    let available = availability.satisfies(surface.requirement);
    json!({
        "verb": surface.verb,
        "surface": format!("{}.{}", surface.verb, surface.surface),
        "summary": surface.summary,
        "arguments": surface.arguments,
        "mutation": surface.mutation,
        "requires": surface.requirement.as_str(),
        "available": available,
        "unavailable_reason": if available {
            Value::Null
        } else {
            json!(surface.requirement.unavailable_reason())
        },
    })
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
        EventPayload::OutcomeObserved(value) => value.observation_lsns.clone(),
        _ => Vec::new(),
    };
    raw.into_iter()
        .filter(|value| *value != 0)
        .map(LSN::new)
        .collect()
}

const fn attention_name(value: AttentionDecision) -> &'static str {
    match value {
        AttentionDecision::Ignore => "ignore",
        AttentionDecision::Remember => "remember",
        AttentionDecision::Batch => "batch",
        AttentionDecision::Schedule => "schedule",
        AttentionDecision::AskUser => "ask_user",
        AttentionDecision::StartWork => "start_work",
        AttentionDecision::Notify => "notify",
    }
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
