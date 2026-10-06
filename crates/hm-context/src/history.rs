use crate::types::{digest_bytes, validate_id, ContextError, Cursor, MessagePart, MessageRole, Scope, SourceMessage, SourceSpan, CONTRACT_VERSION};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourceRelation {
    Edit { id: String, original_id: String, replacement_id: String },
    Regenerate { id: String, original_id: String, replacement_id: String },
    Tombstone { id: String, source_id: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IngestReceipt { pub cursor: Cursor, pub replayed: bool }

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenParent { pub session_id: String, pub cursor: Cursor, pub digest: String }

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Event {
    Source { message: SourceMessage, original_bytes: Vec<u8> },
    Relation { relation: SourceRelation },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot { version: u32, scope: Scope, session_id: String, parent: Option<FrozenParent>, events: Vec<Event>, digest: String }

#[derive(Clone, Debug)]
pub struct SourceHistory {
    scope: Scope,
    session_id: String,
    parent: Option<FrozenParent>,
    events: Vec<Event>,
    sources: BTreeMap<String, usize>,
    relations: BTreeMap<String, usize>,
    hidden: BTreeSet<String>,
}

impl SourceHistory {
    pub fn new(scope: Scope, session_id: impl Into<String>) -> Result<Self, ContextError> {
        scope.validate()?;
        let session_id = session_id.into();
        validate_id(&session_id)?;
        Ok(Self { scope, session_id, parent: None, events: Vec::new(), sources: BTreeMap::new(), relations: BTreeMap::new(), hidden: BTreeSet::new() })
    }

    pub fn scope(&self) -> &Scope { &self.scope }
    pub fn session_id(&self) -> &str { &self.session_id }
    pub fn parent(&self) -> Option<&FrozenParent> { self.parent.as_ref() }
    pub fn cursor(&self) -> Cursor { Cursor { epoch: 1, sequence: self.events.len() as u64 } }
    pub fn messages(&self) -> Vec<&SourceMessage> {
        self.events.iter().filter_map(|event| match event { Event::Source { message, .. } => Some(message), _ => None }).collect()
    }
    pub fn relations(&self) -> Vec<&SourceRelation> {
        self.events.iter().filter_map(|event| match event { Event::Relation { relation } => Some(relation), _ => None }).collect()
    }
    pub fn visible_messages(&self) -> Vec<&SourceMessage> {
        self.messages().into_iter().filter(|message| !self.hidden.contains(&message.id)).collect()
    }

    pub fn ingest(&mut self, message: SourceMessage, original_bytes: Vec<u8>) -> Result<IngestReceipt, ContextError> {
        message.validate()?;
        validate_parts(&message)?;
        if let Some(index) = self.sources.get(&message.id) {
            if self.events[*index] != (Event::Source { message, original_bytes }) { return Err(ContextError::Conflict); }
            return Ok(IngestReceipt { cursor: Cursor { epoch: 1, sequence: *index as u64 + 1 }, replayed: true });
        }
        if self.relations.contains_key(&message.id) { return Err(ContextError::Conflict); }
        if self.messages().last().is_some_and(|last| message.ordinal <= last.ordinal) { return Err(ContextError::Invalid("source ordinals must increase".into())); }
        self.cursor().advance()?;
        self.sources.insert(message.id.clone(), self.events.len());
        self.events.push(Event::Source { message, original_bytes });
        Ok(IngestReceipt { cursor: self.cursor(), replayed: false })
    }

    pub fn relate(&mut self, relation: SourceRelation) -> Result<IngestReceipt, ContextError> {
        let (id, original, replacement) = relation_fields(&relation);
        validate_id(id)?;
        validate_id(original)?;
        if let Some(index) = self.relations.get(id) {
            if self.events[*index] != (Event::Relation { relation }) { return Err(ContextError::Conflict); }
            return Ok(IngestReceipt { cursor: Cursor { epoch: 1, sequence: *index as u64 + 1 }, replayed: true });
        }
        if self.sources.contains_key(id) || !self.sources.contains_key(original) || self.hidden.contains(original) { return Err(ContextError::Conflict); }
        if let Some(replacement) = replacement {
            validate_id(replacement)?;
            let Some(replacement_index) = self.sources.get(replacement) else { return Err(ContextError::Invalid("replacement source missing".into())); };
            if replacement == original || replacement_index <= &self.sources[original] || self.hidden.contains(replacement) { return Err(ContextError::Conflict); }
            let old = self.message(original)?;
            let new = self.message(replacement)?;
            if old.role != new.role || matches!(relation, SourceRelation::Regenerate { .. }) && old.role != MessageRole::Assistant { return Err(ContextError::Invalid("invalid revision roles".into())); }
        }
        self.cursor().advance()?;
        self.hidden.insert(original.to_owned());
        self.relations.insert(id.to_owned(), self.events.len());
        self.events.push(Event::Relation { relation });
        Ok(IngestReceipt { cursor: self.cursor(), replayed: false })
    }

    pub fn message(&self, id: &str) -> Result<&SourceMessage, ContextError> {
        let index = self.sources.get(id).ok_or_else(|| ContextError::Invalid("source missing".into()))?;
        match &self.events[*index] { Event::Source { message, .. } => Ok(message), _ => Err(ContextError::Conflict) }
    }

    pub fn recover(&self, scope: &Scope, span: &SourceSpan) -> Result<Vec<u8>, ContextError> {
        if scope != &self.scope { return Err(ContextError::ScopeMismatch); }
        let index = self.sources.get(&span.source_id).ok_or_else(|| ContextError::Invalid("source missing".into()))?;
        let Event::Source { message, original_bytes } = &self.events[*index] else { return Err(ContextError::Conflict); };
        if span.source_digest != message.source_digest { return Err(ContextError::Stale); }
        let start = usize::try_from(span.byte_start).map_err(|_| ContextError::Capacity)?;
        let end = usize::try_from(span.byte_end).map_err(|_| ContextError::Capacity)?;
        if start > end || end > original_bytes.len() { return Err(ContextError::Invalid("source byte range out of bounds".into())); }
        Ok(original_bytes[start..end].to_vec())
    }

    pub fn source_span(&self, id: &str) -> Result<SourceSpan, ContextError> {
        let message = self.message(id)?;
        let Event::Source { original_bytes, .. } = &self.events[self.sources[id]] else { return Err(ContextError::Conflict); };
        Ok(SourceSpan { source_id: id.into(), source_digest: message.source_digest.clone(), byte_start: 0, byte_end: original_bytes.len() as u64 })
    }

    pub fn freeze_child(&self, child_session_id: impl Into<String>) -> Result<Self, ContextError> {
        let child_session_id = child_session_id.into();
        validate_id(&child_session_id)?;
        if child_session_id == self.session_id { return Err(ContextError::Conflict); }
        let mut child = self.clone();
        child.session_id = child_session_id;
        child.parent = Some(FrozenParent { session_id: self.session_id.clone(), cursor: self.cursor(), digest: digest_bytes(&self.export_canonical()?) });
        Ok(child)
    }

    pub fn export_canonical(&self) -> Result<Vec<u8>, ContextError> {
        let mut snapshot = Snapshot { version: CONTRACT_VERSION, scope: self.scope.clone(), session_id: self.session_id.clone(), parent: self.parent.clone(), events: self.events.clone(), digest: String::new() };
        snapshot.digest = digest_bytes(&serde_json::to_vec(&snapshot)?);
        Ok(serde_json::to_vec(&snapshot)?)
    }

    pub fn restore_canonical(bytes: &[u8]) -> Result<Self, ContextError> {
        let mut snapshot: Snapshot = serde_json::from_slice(bytes)?;
        if snapshot.version != CONTRACT_VERSION { return Err(ContextError::Invalid("unsupported history version".into())); }
        let digest = std::mem::take(&mut snapshot.digest);
        if digest_bytes(&serde_json::to_vec(&snapshot)?) != digest { return Err(ContextError::Invalid("history digest mismatch".into())); }
        if let Some(parent) = &snapshot.parent {
            validate_id(&parent.session_id)?;
            parent.cursor.validate()?;
            if parent.session_id == snapshot.session_id || parent.cursor.sequence > snapshot.events.len() as u64 || parent.digest.len() != 64 || !parent.digest.bytes().all(|b| b.is_ascii_hexdigit()) { return Err(ContextError::Invalid("invalid frozen parent".into())); }
        }
        let mut history = Self::new(snapshot.scope, snapshot.session_id)?;
        history.parent = snapshot.parent;
        for event in snapshot.events {
            let receipt = match event { Event::Source { message, original_bytes } => history.ingest(message, original_bytes)?, Event::Relation { relation } => history.relate(relation)? };
            if receipt.replayed { return Err(ContextError::Invalid("duplicate checkpoint event".into())); }
        }
        if history.export_canonical()? != bytes { return Err(ContextError::Invalid("noncanonical history checkpoint".into())); }
        Ok(history)
    }

    pub fn restore_checkpoint(&mut self, bytes: &[u8]) -> Result<(), ContextError> {
        let restored = Self::restore_canonical(bytes)?;
        if restored.scope != self.scope || restored.session_id != self.session_id { return Err(ContextError::ScopeMismatch); }
        *self = restored;
        Ok(())
    }
}

fn relation_fields(relation: &SourceRelation) -> (&str, &str, Option<&str>) {
    match relation {
        SourceRelation::Edit { id, original_id, replacement_id } | SourceRelation::Regenerate { id, original_id, replacement_id } => (id, original_id, Some(replacement_id)),
        SourceRelation::Tombstone { id, source_id } => (id, source_id, None),
    }
}

fn validate_parts(message: &SourceMessage) -> Result<(), ContextError> {
    for part in &message.parts {
        match part {
            MessagePart::Text { .. } => {}
            MessagePart::ToolCall { call_id, name, arguments } => {
                validate_id(call_id)?; validate_id(name)?;
                if message.role != MessageRole::Assistant || !serde_json::from_str::<serde_json::Value>(arguments)?.is_object() { return Err(ContextError::Invalid("invalid tool call".into())); }
            }
            MessagePart::ToolResult { call_id, .. } => {
                validate_id(call_id)?;
                if message.role != MessageRole::Tool { return Err(ContextError::Invalid("invalid tool result role".into())); }
            }
            MessagePart::Opaque { media_type, reference, digest } => {
                if media_type.is_empty() || reference.is_empty() || digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) { return Err(ContextError::Invalid("invalid opaque source".into())); }
            }
        }
    }
    Ok(())
}
