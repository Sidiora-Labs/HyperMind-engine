#![allow(clippy::missing_errors_doc)]

use crate::Envelope;
use hm_core::{ConversationId, Error, ErrorCode, LSN};
use hm_index::entity_rules::{self, EntityKind};
use hm_ledger::frame::{EventKind, Frame};
use hm_serve::actor::ActorEngine;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const MAXIMUM_SOURCE_FRAMES: usize = 65_536;
pub(crate) const MAXIMUM_SOURCE_ENTITIES: usize = 64;
pub(crate) const MAXIMUM_ENTITY_LSNS: usize = 32;
pub(crate) const MAXIMUM_SOURCE_CONVERSATIONS: usize = 256;
pub(crate) const MAXIMUM_SOURCE_CITATIONS: usize = 256;

pub(crate) const EXTRACTOR: &str = "entity_rules";

pub(crate) async fn index(actor: &ActorEngine) -> Result<Envelope, Error> {
    let frames = load_frames(actor).await?;
    let mut groups: BTreeMap<ConversationId, Group> = BTreeMap::new();
    for frame in &frames {
        groups
            .entry(frame.header.conversation)
            .or_default()
            .observe(frame);
    }
    let mut ordered: Vec<(ConversationId, Group)> = groups.into_iter().collect();
    ordered.sort_by_key(|(conversation, group)| (group.first_lsn, *conversation));
    let truncated = ordered.len() > MAXIMUM_SOURCE_CONVERSATIONS;
    ordered.truncate(MAXIMUM_SOURCE_CONVERSATIONS);
    let mut envelope = Envelope::empty();
    for (conversation, group) in ordered {
        envelope.items.push(json!({
            "surface": "sources",
            "conversation": conversation.to_string(),
            "records": group.records,
            "first_lsn": group.first_lsn,
            "last_lsn": group.last_lsn,
            "last_wall_timestamp_ns": group.last_wall_timestamp_ns,
            "kinds": group.kinds,
            "truncated": truncated,
        }));
        envelope
            .provenance
            .push(format!("hm://{}/lsn/{}", actor.actor(), group.last_lsn));
    }
    Ok(envelope)
}

#[allow(clippy::too_many_lines)]
pub(crate) async fn detail(actor: &ActorEngine, conversation_hex: &str) -> Result<Envelope, Error> {
    let conversation = parse_conversation(conversation_hex)?;
    let frames = load_frames(actor).await?;
    let mut messages: BTreeMap<ConversationId, Vec<(u64, String)>> = BTreeMap::new();
    let mut records = 0_u64;
    let mut first_lsn = 0_u64;
    let mut last_lsn = 0_u64;
    for frame in &frames {
        if frame.header.conversation == conversation {
            let lsn = frame.header.lsn.get();
            if records == 0 || lsn < first_lsn {
                first_lsn = lsn;
            }
            if lsn > last_lsn {
                last_lsn = lsn;
            }
            records += 1;
        }
        if !matches!(
            frame.header.kind,
            EventKind::UserMsg | EventKind::DeliveredMsg
        ) {
            continue;
        }
        let (content, _) = crate::event_content(frame.header.kind, &frame.sealed_payload)?;
        if content.is_empty() {
            continue;
        }
        messages
            .entry(frame.header.conversation)
            .or_default()
            .push((frame.header.lsn.get(), content));
    }

    let mut appearances: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (other, other_messages) in &messages {
        let hex = other.to_string();
        for entity in entity_rules::extract_entities(&joined(other_messages)) {
            appearances
                .entry(entity.canonical)
                .or_default()
                .insert(hex.clone());
        }
    }

    let own = messages.get(&conversation).cloned().unwrap_or_default();
    let extracted = entity_rules::extract_entities(&joined(&own));
    let mut contributions: BTreeMap<(EntityKind, String), Vec<u64>> = BTreeMap::new();
    for (lsn, content) in &own {
        for entity in entity_rules::extract_entities(content) {
            contributions
                .entry((entity.kind, entity.canonical))
                .or_default()
                .push(*lsn);
        }
    }

    let mut truncated = extracted.len() > MAXIMUM_SOURCE_ENTITIES;
    let mut entities = Vec::new();
    let mut shared = Vec::new();
    let mut shared_seen = BTreeSet::new();
    for entity in extracted.iter().take(MAXIMUM_SOURCE_ENTITIES) {
        let mut lsns = contributions
            .get(&(entity.kind, entity.canonical.clone()))
            .cloned()
            .unwrap_or_default();
        lsns.dedup();
        if lsns.len() > MAXIMUM_ENTITY_LSNS {
            lsns.truncate(MAXIMUM_ENTITY_LSNS);
            truncated = true;
        }
        entities.push(json!({
            "canonical": entity.canonical,
            "kind": entity_kind_name(entity.kind),
            "aliases": entity.aliases,
            "lsns": lsns,
        }));
        let Some(conversations) = appearances.get(&entity.canonical) else {
            continue;
        };
        if conversations.len() < 2 || !shared_seen.insert(entity.canonical.clone()) {
            continue;
        }
        shared.push(json!({
            "canonical": entity.canonical,
            "conversations": conversations.iter().collect::<Vec<_>>(),
        }));
    }

    if own.len() > MAXIMUM_SOURCE_CITATIONS {
        truncated = true;
    }
    let citations = own
        .iter()
        .take(MAXIMUM_SOURCE_CITATIONS)
        .map(|(lsn, _)| format!("hm://{}/{conversation}/{lsn}", actor.actor()))
        .collect::<Vec<_>>();

    let mut envelope = Envelope::empty();
    envelope.provenance.extend(citations.iter().cloned());
    envelope.items.push(json!({
        "surface": "source_detail",
        "conversation": conversation.to_string(),
        "records": records,
        "first_lsn": first_lsn,
        "last_lsn": last_lsn,
        "extractor": EXTRACTOR,
        "entities": entities,
        "shared_entities": shared,
        "citations": citations,
        "truncated": truncated,
    }));
    Ok(envelope)
}

fn parse_conversation(value: &str) -> Result<ConversationId, Error> {
    if value.len() != 32
        || !value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let mut bytes = [0_u8; 16];
    for (index, slot) in bytes.iter_mut().enumerate() {
        let pair = value
            .get(index * 2..index * 2 + 2)
            .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
        *slot = u8::from_str_radix(pair, 16).map_err(|_| Error::new(ErrorCode::InvalidArgument))?;
    }
    Ok(ConversationId::new(bytes))
}

async fn load_frames(actor: &ActorEngine) -> Result<Vec<Frame>, Error> {
    let frames = actor
        .frames_since(LSN::new(0), None, MAXIMUM_SOURCE_FRAMES + 1)
        .await?;
    if frames.len() > MAXIMUM_SOURCE_FRAMES {
        return Err(Error::new(ErrorCode::CapacityExceeded));
    }
    Ok(frames)
}

fn joined(messages: &[(u64, String)]) -> String {
    messages
        .iter()
        .map(|(_, content)| content.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Default)]
struct Group {
    records: u64,
    first_lsn: u64,
    last_lsn: u64,
    last_wall_timestamp_ns: i64,
    kinds: BTreeMap<String, u64>,
}

impl Group {
    fn observe(&mut self, frame: &Frame) {
        let lsn = frame.header.lsn.get();
        if self.records == 0 || lsn < self.first_lsn {
            self.first_lsn = lsn;
        }
        if lsn >= self.last_lsn {
            self.last_lsn = lsn;
            self.last_wall_timestamp_ns = frame.header.wall_timestamp_ns.get();
        }
        self.records += 1;
        *self
            .kinds
            .entry(format!("{:?}", frame.header.kind).to_lowercase())
            .or_default() += 1;
    }
}

const fn entity_kind_name(kind: EntityKind) -> &'static str {
    match kind {
        EntityKind::Url => "url",
        EntityKind::Domain => "domain",
        EntityKind::Path => "path",
        EntityKind::HexId => "hex_id",
        EntityKind::StructuredId => "structured_id",
        EntityKind::ProperNoun => "proper_noun",
    }
}
