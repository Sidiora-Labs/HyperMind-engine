#![allow(clippy::missing_errors_doc)]

use super::surfaces::{self, Availability, Surface};
use crate::Envelope;
use hm_core::{Error, ErrorCode, LSN};
use hm_index::vocabulary::{DEFAULT_SIMILARITY_THRESHOLD_Q16, Q16_ONE};
use hm_schema::event::{self, Boundary, EventHistory};
use hm_schema::events::{AttentionDecision, Authority, EventPayload, VocabularyCategory};
use hm_serve::actor::ActorEngine;
use hm_serve::config::CapabilityToken;
use rmcp::schemars;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;

const MAXIMUM_VOCABULARY_ITEMS: usize = 256;
const MAXIMUM_ALIAS_PROPOSALS: usize = 64;
const REVIEW_WARNING: &str = "An alias proposal changes nothing; it is accepted only by importing a new vocabulary version that declares the alias.";

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
    admin_token: Option<&CapabilityToken>,
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
    if uri == format!("hm://{}/vocabulary", actor.actor()) {
        let records = actor.vocabularies(MAXIMUM_VOCABULARY_ITEMS).await?;
        envelope.items[0]["vocabulary"] = json!(
            records
                .iter()
                .map(|record| json!({
                    "vocabulary_id": String::from_utf8_lossy(&record.vocabulary_id),
                    "version": record.version,
                    "source_uri": record.source_uri,
                    "source_media_type": record.source_media_type,
                    "source_digest": hex(&record.source_digest),
                    "term_count": record.term_count,
                    "ignored_triples": record.ignored_triples,
                    "event_lsn": record.event_lsn,
                }))
                .collect::<Vec<_>>()
        );
        envelope.provenance.extend(
            records
                .iter()
                .map(|record| format!("hm://{}/lsn/{}", actor.actor(), record.event_lsn)),
        );
        envelope.warnings.push(REVIEW_WARNING.to_owned());
        return Ok(envelope);
    }
    let aliases_path = format!("hm://{}/vocabulary/aliases", actor.actor());
    if uri.split('?').next() == Some(aliases_path.as_str()) {
        let query = parse_alias_query(&uri)?;
        let proposals = actor
            .alias_proposals(query.name, query.threshold_q16, MAXIMUM_ALIAS_PROPOSALS)
            .await?;
        envelope.items[0]["alias_proposals"] = json!(
            proposals
                .iter()
                .map(|proposal| json!({
                    "vocabulary_id": String::from_utf8_lossy(&proposal.vocabulary_id),
                    "version": proposal.version,
                    "term_id": proposal.term_id,
                    "canonical_name": proposal.canonical_name,
                    "category": category_name(proposal.category),
                    "matched_candidate": proposal.matched_candidate,
                    "similarity_q16": proposal.similarity_q16,
                    "threshold_q16": query.threshold_q16,
                    "accepted": false,
                }))
                .collect::<Vec<_>>()
        );
        envelope.warnings.push(REVIEW_WARNING.to_owned());
        return Ok(envelope);
    }
    if uri == format!("hm://{}/access", actor.actor()) {
        let access = super::access::run(actor, admin_token);
        envelope.items.extend(access.items);
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
    if uri.starts_with(&format!("hm://{}/evidence/", actor.actor())) {
        let evidence = super::evidence::run(actor, parse_lsn(&uri)?).await?;
        envelope.items.extend(evidence.items);
        envelope.provenance.extend(evidence.provenance);
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
        history.record(frame.header.lsn, kind, verified.envelope.authority);
    }
    let first = parse_lsn(&uri)?;
    let mut pending = vec![first];
    let mut visited = BTreeSet::new();
    let mut edges: Vec<Value> = Vec::new();
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
        let hops = references(&verified.envelope.payload);
        for (target, relation) in &hops {
            edges.push(json!({
                "from": lsn.get(),
                "to": target.get(),
                "relation": relation,
            }));
        }
        pending.extend(hops.into_iter().rev().map(|(target, _)| target));
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
    envelope.items[0]["evidence_path"] = json!({
        "label": "evidence_path",
        "root_lsn": first.get(),
        "edges": edges,
        "visited": visited.len(),
    });
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
pub(crate) struct InspectHistory {
    records: std::collections::BTreeMap<LSN, (event::EventKind, Authority)>,
}

impl InspectHistory {
    pub(crate) fn record(&mut self, lsn: LSN, kind: event::EventKind, authority: Authority) {
        self.records.insert(lsn, (kind, authority));
    }
}

impl EventHistory for InspectHistory {
    fn kind_at(&self, lsn: LSN) -> Option<event::EventKind> {
        self.records.get(&lsn).map(|record| record.0)
    }

    fn authority_at(&self, lsn: LSN) -> Option<Authority> {
        self.records.get(&lsn).map(|record| record.1)
    }
}

struct AliasQuery {
    name: String,
    threshold_q16: u32,
}

fn parse_alias_query(uri: &str) -> Result<AliasQuery, Error> {
    let query = uri.split_once('?').map_or("", |(_, query)| query);
    let mut name: Option<String> = None;
    let mut threshold: Option<u32> = None;
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (key, value) = pair
            .split_once('=')
            .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
        match key {
            "name" if name.is_none() => name = Some(decode_component(value)?),
            "threshold_q16" if threshold.is_none() => {
                threshold = Some(
                    value
                        .parse::<u32>()
                        .map_err(|_| Error::new(ErrorCode::InvalidArgument))?,
                );
            }
            _ => return Err(Error::new(ErrorCode::InvalidArgument)),
        }
    }
    let name = name
        .filter(|value| !value.is_empty())
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
    let threshold_q16 = threshold.unwrap_or(DEFAULT_SIMILARITY_THRESHOLD_Q16);
    if threshold_q16 == 0 || threshold_q16 > Q16_ONE {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    Ok(AliasQuery {
        name,
        threshold_q16,
    })
}

fn decode_component(value: &str) -> Result<String, Error> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                decoded.push(b' ');
                index += 1;
            }
            b'%' => {
                let digits = value
                    .get(index + 1..index + 3)
                    .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
                decoded.push(
                    u8::from_str_radix(digits, 16)
                        .map_err(|_| Error::new(ErrorCode::InvalidArgument))?,
                );
                index += 3;
            }
            byte => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(decoded).map_err(|_| Error::new(ErrorCode::InvalidArgument))
}

const fn category_name(category: VocabularyCategory) -> &'static str {
    match category {
        VocabularyCategory::EntityType => "entity_type",
        VocabularyCategory::EntityInstance => "entity_instance",
        VocabularyCategory::Relation => "relation",
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

fn references(payload: &EventPayload) -> Vec<(LSN, &'static str)> {
    let (raw, relation) = match payload {
        EventPayload::ToolResult(value) => (vec![value.tool_call_lsn], "tool_call_lsn"),
        EventPayload::Effect(value) => (vec![value.tool_call_lsn], "tool_call_lsn"),
        EventPayload::Outcome(value) => (
            value.evidence_lsns.clone().unwrap_or_default(),
            "evidence_lsns",
        ),
        EventPayload::LoopClosed(value) => (
            value.evidence_lsns.clone().unwrap_or_default(),
            "evidence_lsns",
        ),
        EventPayload::Binding(value) => (vec![value.evidence_lsn], "evidence_lsn"),
        EventPayload::Attestation(value) => (vec![value.target_lsn], "target_lsn"),
        EventPayload::OutcomeObserved(value) => {
            (value.observation_lsns.clone(), "observation_lsns")
        }
        _ => (Vec::new(), ""),
    };
    raw.into_iter()
        .filter(|value| *value != 0)
        .map(|value| (LSN::new(value), relation))
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
