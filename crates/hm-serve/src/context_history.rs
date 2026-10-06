use crate::actor::{ActorEngine, IncomingEvent};
use hm_context::{history::{SourceHistory, SourceRelation}, Authority, ContextError, Cursor, MessagePart, MessageRole, Scope, SourceMessage, SourceSpan, CONTRACT_VERSION, digest_bytes, validate_id};
use hm_core::{ConversationId, LSN};
use hm_ledger::frame::{EventKind, Frame};
use hm_schema::{event::{self, CURRENT_SCHEMA_VERSION}, events::{EventEnvelope, EventPayload, ProviderFrame, Retention, Sensitivity}};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const HISTORY_PROVIDER: &str = "hypermind/context-history/v1";
const MAX_RECORD_BYTES: usize = 512 * 1024;

#[derive(Debug)]
pub enum HistoryError { Context(ContextError), Ledger(hm_core::Error) }
impl std::fmt::Display for HistoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { match self { Self::Context(e) => e.fmt(f), Self::Ledger(e) => e.fmt(f) } }
}
impl std::error::Error for HistoryError {}
impl From<ContextError> for HistoryError { fn from(e: ContextError) -> Self { Self::Context(e) } }
impl From<serde_json::Error> for HistoryError { fn from(e: serde_json::Error) -> Self { Self::Context(e.into()) } }
impl From<hm_core::Error> for HistoryError { fn from(e: hm_core::Error) -> Self { Self::Ledger(e) } }

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceIngestion {
    pub version: u32,
    pub scope: Scope,
    pub session_id: String,
    pub conversation: String,
    pub message: SourceMessage,
    pub original_bytes: Vec<u8>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelationIngestion {
    pub version: u32,
    pub scope: Scope,
    pub session_id: String,
    pub conversation: String,
    pub relation: SourceRelation,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForkRequest {
    pub version: u32,
    pub scope: Scope,
    pub parent_session_id: String,
    pub parent_conversation: String,
    pub child_session_id: String,
    pub child_conversation: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryReceipt {
    pub version: u32,
    pub scope: Scope,
    pub session_id: String,
    pub cursor: Cursor,
    pub last_lsn: u64,
    pub replayed: bool,
    pub source_span: Option<SourceSpan>,
}
#[derive(Clone, Debug)]
pub struct LedgerHistory { pub history: SourceHistory, pub source_uris: Vec<String>, pub unsupported_parts: Vec<UnsupportedPart> }

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UnsupportedPart { pub source_id: String, pub ledger_reference: String, pub reason: String }

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Operation {
    Source { message: SourceMessage, original_bytes: Vec<u8> },
    Relation { relation: SourceRelation },
    Fork { request: ForkRequest, frozen_history: Vec<u8>, source_uris: Vec<String>, unsupported_parts: Vec<UnsupportedPart> },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record { version: u32, scope: Scope, session_id: String, conversation: String, operation: Operation }

fn validate_binding(version: u32, scope: &Scope, session: &str, conversation: &str) -> Result<(), ContextError> {
    if version != CONTRACT_VERSION { return Err(ContextError::Invalid("unsupported source history version".into())); }
    scope.validate()?; validate_id(session)?; validate_id(conversation)
}
fn trusted(expected: &Scope, actual: &Scope) -> Result<(), ContextError> { if expected == actual { Ok(()) } else { Err(ContextError::ScopeMismatch) } }

pub async fn replay(actor: &ActorEngine, scope: &Scope, session_id: &str, conversation: &str) -> Result<LedgerHistory, HistoryError> {
    validate_binding(CONTRACT_VERSION, scope, session_id, conversation)?;
    let mut output = LedgerHistory { history: SourceHistory::new(scope.clone(), session_id)?, source_uris: Vec::new(), unsupported_parts: Vec::new() };
    let wanted = ConversationId::derive(conversation);
    let mut ignored = BTreeMap::new();
    for frame in actor.frames_since(LSN::new(0), None, usize::MAX).await? {
        let verified = actor.verified_event(frame.header.lsn).await?;
        if let EventPayload::Attestation(attestation) = &verified.envelope.payload {
            if attestation.disposition == hm_schema::events::AttestationDisposition::Ignored {
                ignored.entry(format!("hm://{}/lsn/{}", actor.actor(), attestation.target_lsn)).or_insert(frame.header.lsn.get());
                apply_ignored(&mut output, &ignored)?;
            }
        }
        if let EventPayload::ProviderFrame(provider) = &verified.envelope.payload {
            if provider.provider == HISTORY_PROVIDER {
                let record: Record = serde_json::from_slice(&provider.api_content)?;
                validate_binding(record.version, &record.scope, &record.session_id, &record.conversation)?;
                if frame.header.conversation != ConversationId::derive(&record.conversation) { return Err(ContextError::Conflict.into()); }
                if record.session_id == session_id || frame.header.conversation == wanted {
                    trusted(scope, &record.scope)?;
                    if record.session_id != session_id || record.conversation != conversation { return Err(ContextError::Conflict.into()); }
                    apply(&mut output, &record, actor.actor().get(), frame.header.lsn.get())?;
                    apply_ignored(&mut output, &ignored)?;
                }
                continue;
            }
            if provider.provider == "hypermind/context-session/v1" {
                let binding: serde_json::Value = serde_json::from_slice(&provider.api_content)?;
                if binding["request"]["session_id"].as_str() == Some(session_id) || binding["conversation"].as_str() == Some(conversation) {
                    let bound_scope: Scope = serde_json::from_value(binding["request"]["scope"].clone())?;
                    trusted(scope, &bound_scope)?;
                    if binding["conversation"].as_str() != Some(conversation) || binding["request"]["session_id"].as_str() != Some(session_id) { return Err(ContextError::Conflict.into()); }
                }
            }
        }
        if frame.header.conversation != wanted { continue; }
        if let Some((source, bytes)) = native_source(&frame, &verified.envelope, &output.history, &mut output.unsupported_parts)? {
            output.history.ingest(source, bytes)?;
            output.source_uris.push(format!("hm://{}/lsn/{}", actor.actor(), frame.header.lsn.get()));
            apply_ignored(&mut output, &ignored)?;
        }
    }
    apply_ignored(&mut output, &ignored)?;
    Ok(output)
}

fn apply_ignored(output: &mut LedgerHistory, ignored: &BTreeMap<String, u64>) -> Result<(), HistoryError> {
    let removed: Vec<_> = output.history.messages().into_iter().zip(&output.source_uris)
        .filter_map(|(message, uri)| ignored.get(uri).map(|lsn| (message.id.clone(), *lsn)))
        .collect();
    let mut visible: BTreeSet<_> = output.history.visible_messages().into_iter().map(|m| m.id.clone()).collect();
    for (source_id, lsn) in removed {
        if visible.remove(&source_id) {
            output.history.relate(SourceRelation::Tombstone { id: format!("native-forget:{lsn}:{}", digest_bytes(source_id.as_bytes())), source_id })?;
        }
    }
    output.unsupported_parts.retain(|part| visible.contains(&part.source_id));
    Ok(())
}

impl LedgerHistory {
    pub fn visible_source_uris(&self) -> Vec<String> {
        let visible: BTreeSet<_> = self.history.visible_messages().into_iter().map(|m| m.id.as_str()).collect();
        self.history.messages().into_iter().zip(&self.source_uris).filter(|(message, _)| visible.contains(message.id.as_str())).map(|(_, uri)| uri.clone()).collect()
    }
}

fn apply(output: &mut LedgerHistory, record: &Record, actor: u16, lsn: u64) -> Result<(), HistoryError> {
    match &record.operation {
        Operation::Source { message, original_bytes } => {
            let receipt = output.history.ingest(message.clone(), original_bytes.clone())?;
            if !receipt.replayed { output.source_uris.push(format!("hm://{actor}/lsn/{lsn}")); }
        }
        Operation::Relation { relation } => { output.history.relate(relation.clone())?; }
        Operation::Fork { request, frozen_history, source_uris, unsupported_parts } => {
            validate_binding(request.version, &request.scope, &request.parent_session_id, &request.parent_conversation)?;
            if request.scope != record.scope || request.child_session_id != record.session_id || request.child_conversation != record.conversation || request.parent_conversation == request.child_conversation || !output.history.messages().is_empty() || !output.history.relations().is_empty() { return Err(ContextError::Conflict.into()); }
            let restored = SourceHistory::restore_canonical(frozen_history)?;
            if restored.scope() != &record.scope || restored.session_id() != record.session_id || restored.parent().is_none_or(|parent| parent.session_id != request.parent_session_id) || source_uris.len() != restored.messages().len() { return Err(ContextError::Conflict.into()); }
            output.history = restored;
            output.source_uris = source_uris.clone();
            output.unsupported_parts = unsupported_parts.clone();
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct PreparedSource {
    expected_tail: LSN,
    actor: u16,
    record: Record,
    receipt: HistoryReceipt,
}

pub async fn prepare_ingest(actor: &ActorEngine, trusted_scope: &Scope, request: &SourceIngestion) -> Result<PreparedSource, HistoryError> {
    trusted(trusted_scope, &request.scope)?;
    validate_binding(request.version, &request.scope, &request.session_id, &request.conversation)?;
    let allowed_authority = match request.message.role {
        MessageRole::User => matches!(request.message.authority, Authority::UserAsserted | Authority::ExternalObserved | Authority::DerivedInference),
        MessageRole::Assistant => matches!(request.message.authority, Authority::AssistantGenerated | Authority::DerivedInference),
        MessageRole::Tool => matches!(request.message.authority, Authority::ToolObserved | Authority::ExternalObserved | Authority::DerivedInference),
    };
    if !allowed_authority { return Err(ContextError::Invalid("source authority does not match external message role".into()).into()); }
    let expected_tail = actor.stats().await?.applied.last_lsn;
    let mut state = replay(actor, &request.scope, &request.session_id, &request.conversation).await?;
    if request.message.id.starts_with("lsn:") { return Err(ContextError::Invalid("reserved native source identity".into()).into()); }
    let receipt = state.history.ingest(request.message.clone(), request.original_bytes.clone())?;
    let record = Record { version: request.version, scope: request.scope.clone(), session_id: request.session_id.clone(), conversation: request.conversation.clone(), operation: Operation::Source { message: request.message.clone(), original_bytes: request.original_bytes.clone() } };
    Ok(PreparedSource { expected_tail, actor: actor.actor().get(), record, receipt: HistoryReceipt { version: CONTRACT_VERSION, scope: request.scope.clone(), session_id: request.session_id.clone(), cursor: receipt.cursor, last_lsn: 0, replayed: false, source_span: Some(state.history.source_span(&request.message.id)?) } })
}

async fn persist_prepared(actor: &ActorEngine, trusted_scope: &Scope, prepared: &PreparedSource) -> Result<HistoryReceipt, HistoryError> {
    trusted(trusted_scope, &prepared.record.scope)?;
    if actor.actor().get() != prepared.actor { return Err(ContextError::ScopeMismatch.into()); }
    let (lsn, replayed) = append(actor, &prepared.record, prepared.expected_tail).await?;
    let mut receipt = prepared.receipt.clone();
    receipt.last_lsn = lsn;
    receipt.replayed = replayed;
    if replayed {
        let mut current = replay(actor, &prepared.record.scope, &prepared.record.session_id, &prepared.record.conversation).await?;
        if let Operation::Source { message, original_bytes } = &prepared.record.operation {
            receipt.cursor = current.history.ingest(message.clone(), original_bytes.clone())?.cursor;
        }
    }
    Ok(receipt)
}

pub async fn commit_ingest(actor: &ActorEngine, trusted_scope: &Scope, prepared: &PreparedSource) -> Result<HistoryReceipt, HistoryError> {
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    persist_prepared(actor, trusted_scope, prepared).await
}

pub async fn ingest(actor: &ActorEngine, trusted_scope: &Scope, request: &SourceIngestion) -> Result<HistoryReceipt, HistoryError> {
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let prepared = prepare_ingest(actor, trusted_scope, request).await?;
    persist_prepared(actor, trusted_scope, &prepared).await
}

pub async fn relate(actor: &ActorEngine, trusted_scope: &Scope, request: &RelationIngestion) -> Result<HistoryReceipt, HistoryError> {
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    trusted(trusted_scope, &request.scope)?;
    validate_binding(request.version, &request.scope, &request.session_id, &request.conversation)?;
    let expected_tail = actor.stats().await?.applied.last_lsn;
    let mut state = replay(actor, &request.scope, &request.session_id, &request.conversation).await?;
    let receipt = state.history.relate(request.relation.clone())?;
    let record = Record { version: request.version, scope: request.scope.clone(), session_id: request.session_id.clone(), conversation: request.conversation.clone(), operation: Operation::Relation { relation: request.relation.clone() } };
    let (lsn, replayed) = append(actor, &record, expected_tail).await?;
    Ok(HistoryReceipt { version: CONTRACT_VERSION, scope: request.scope.clone(), session_id: request.session_id.clone(), cursor: receipt.cursor, last_lsn: lsn, replayed, source_span: None })
}

pub async fn fork(actor: &ActorEngine, trusted_scope: &Scope, request: &ForkRequest) -> Result<HistoryReceipt, HistoryError> {
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    trusted(trusted_scope, &request.scope)?;
    validate_binding(request.version, &request.scope, &request.parent_session_id, &request.parent_conversation)?;
    validate_binding(request.version, &request.scope, &request.child_session_id, &request.child_conversation)?;
    if request.parent_session_id == request.child_session_id || request.parent_conversation == request.child_conversation { return Err(ContextError::Conflict.into()); }
    let expected_tail = actor.stats().await?.applied.last_lsn;
    let child = replay(actor, &request.scope, &request.child_session_id, &request.child_conversation).await?;
    if let Some(parent) = child.history.parent() {
        if parent.session_id != request.parent_session_id { return Err(ContextError::Conflict.into()); }
        let records = actor.frames_since(LSN::new(0), Some(ConversationId::derive(&request.child_conversation)), usize::MAX).await?;
        for frame in records {
            if frame.header.kind != EventKind::ProviderFrame { continue; }
            let event = actor.verified_event(frame.header.lsn).await?;
            if let EventPayload::ProviderFrame(p) = event.envelope.payload {
                if p.provider == HISTORY_PROVIDER {
                    let record: Record = serde_json::from_slice(&p.api_content)?;
                    if let Operation::Fork { request: existing, .. } = record.operation {
                        if &existing != request { return Err(ContextError::Conflict.into()); }
                        return Ok(HistoryReceipt { version: CONTRACT_VERSION, scope: request.scope.clone(), session_id: request.child_session_id.clone(), cursor: parent.cursor, last_lsn: frame.header.lsn.get(), replayed: true, source_span: None });
                    }
                }
            }
        }
        return Err(ContextError::Conflict.into());
    }
    if !child.history.messages().is_empty() || !child.history.relations().is_empty() { return Err(ContextError::Conflict.into()); }
    let parent = replay(actor, &request.scope, &request.parent_session_id, &request.parent_conversation).await?;
    let frozen = parent.history.freeze_child(&request.child_session_id)?;
    let record = Record { version: request.version, scope: request.scope.clone(), session_id: request.child_session_id.clone(), conversation: request.child_conversation.clone(), operation: Operation::Fork { request: request.clone(), frozen_history: frozen.export_canonical()?, source_uris: parent.source_uris, unsupported_parts: parent.unsupported_parts } };
    let (lsn, replayed) = append(actor, &record, expected_tail).await?;
    Ok(HistoryReceipt { version: CONTRACT_VERSION, scope: request.scope.clone(), session_id: request.child_session_id.clone(), cursor: frozen.cursor(), last_lsn: lsn, replayed, source_span: None })
}

pub async fn recover(actor: &ActorEngine, trusted_scope: &Scope, session_id: &str, conversation: &str, span: &SourceSpan) -> Result<Vec<u8>, HistoryError> {
    replay(actor, trusted_scope, session_id, conversation).await?.history.recover(trusted_scope, span).map_err(Into::into)
}

async fn append(actor: &ActorEngine, record: &Record, expected_tail: LSN) -> Result<(u64, bool), HistoryError> {
    let content = serde_json::to_vec(record)?;
    if content.len() > MAX_RECORD_BYTES { return Err(ContextError::Capacity.into()); }
    for frame in actor.frames_since(LSN::new(0), Some(ConversationId::derive(&record.conversation)), usize::MAX).await? {
        if frame.header.kind != EventKind::ProviderFrame { continue; }
        let verified = actor.verified_event(frame.header.lsn).await?;
        if let EventPayload::ProviderFrame(provider) = verified.envelope.payload {
            if provider.provider != HISTORY_PROVIDER { continue; }
            let existing: Record = serde_json::from_slice(&provider.api_content)?;
            if existing.scope == record.scope && existing.session_id == record.session_id && operation_id(&existing.operation) == operation_id(&record.operation) {
                if existing != *record { return Err(ContextError::Conflict.into()); }
                return Ok((frame.header.lsn.get(), true));
            }
        }
    }
    let payload = event::encode_event_envelope(&EventEnvelope {
        schema_version: CURRENT_SCHEMA_VERSION,
        payload: EventPayload::ProviderFrame(Box::new(ProviderFrame { provider: HISTORY_PROVIDER.into(), api_content: content })),
        connection_id: None, client_seq: 0, client_event_index: 0, client_event_count: 1,
        origin_actor: 0, run_id: None, model_provenance: None,
        authority: hm_schema::events::Authority::RuntimeFact,
        retention: Retention::Durable, sensitivity: Sensitivity::Personal, event_time_ns: 0,
    });
    let outcome = actor.append_if_tail(expected_tail, vec![IncomingEvent { kind: EventKind::ProviderFrame, conversation: ConversationId::derive(&record.conversation), payload }]).await?;
    Ok((outcome.last_lsn.get(), outcome.duplicate))
}

fn operation_id(operation: &Operation) -> (&'static str, &str) {
    match operation {
        Operation::Source { message, .. } => ("source", &message.id),
        Operation::Relation { relation } => match relation {
            SourceRelation::Edit { id, .. } | SourceRelation::Regenerate { id, .. } | SourceRelation::Tombstone { id, .. } => ("relation", id),
        },
        Operation::Fork { .. } => ("fork", "fork"),
    }
}

fn hex(bytes: &[u8]) -> String { bytes.iter().map(|byte| format!("{byte:02x}")).collect() }
fn opaque(bytes: &[u8], reference: String, media_type: &str) -> MessagePart { MessagePart::Opaque { media_type: media_type.into(), reference, digest: digest_bytes(bytes) } }
fn text_part(bytes: &[u8], reference: String) -> MessagePart {
    match String::from_utf8(bytes.to_vec()) { Ok(text) => MessagePart::Text { text }, Err(_) => opaque(bytes, reference, "application/octet-stream") }
}
fn native_source(frame: &Frame, envelope: &EventEnvelope, history: &SourceHistory, unsupported: &mut Vec<UnsupportedPart>) -> Result<Option<(SourceMessage, Vec<u8>)>, HistoryError> {
    let id = format!("lsn:{}", frame.header.lsn.get());
    let uri = format!("hm://{}/lsn/{}", frame.header.actor.get(), frame.header.lsn.get());
    let (role, parts, bytes) = match &envelope.payload {
        EventPayload::UserMsg(message) => {
            if envelope.authority == hm_schema::events::Authority::ExternalObserved && serde_json::from_slice::<serde_json::Value>(&message.content).is_ok_and(|value| value["format"] == "hypermid_import_v1" && value.get("entry").is_some() && value.get("scope").is_some()) { return Ok(None); }
            (MessageRole::User, vec![text_part(&message.content, uri.clone())], message.content.clone())
        }
        EventPayload::DeliveredMsg(message) => (MessageRole::Assistant, vec![text_part(&message.content, uri.clone())], message.content.clone()),
        EventPayload::ToolCall(call) => {
            let arguments = std::str::from_utf8(&call.arguments).ok().filter(|text| serde_json::from_str::<serde_json::Value>(text).is_ok_and(|v| v.is_object()));
            let mut parts = vec![opaque(&frame.sealed_payload, uri.clone(), "application/vnd.hypermind.event")];
            if let Some(arguments) = arguments.filter(|_| validate_id(&call.tool_name).is_ok() && validate_id(&format!("bytes:{}", hex(&call.call_id))).is_ok()) {
                parts.insert(0, MessagePart::ToolCall { call_id: format!("bytes:{}", hex(&call.call_id)), name: call.tool_name.clone(), arguments: arguments.into() });
            } else {
                unsupported.push(UnsupportedPart { source_id: id.clone(), ledger_reference: uri.clone(), reason: "Native tool arguments, name or call identity cannot be rendered by the source-part contract".into() });
            }
            (MessageRole::Assistant, parts, frame.sealed_payload.clone())
        }
        EventPayload::ToolResult(result) => {
            let mut parts = vec![opaque(&frame.sealed_payload, uri.clone(), "application/vnd.hypermind.event")];
            if let Some(content) = String::from_utf8(result.result.clone()).ok().filter(|_| validate_id(&format!("bytes:{}", hex(&result.call_id))).is_ok()) {
                parts.insert(0, MessagePart::ToolResult { call_id: format!("bytes:{}", hex(&result.call_id)), content, failed: result.status != hm_schema::events::ResultStatus::Ok });
            } else {
                unsupported.push(UnsupportedPart { source_id: id.clone(), ledger_reference: uri.clone(), reason: "Native binary tool result or oversized call identity cannot be rendered by the source-part contract".into() });
            }
            (MessageRole::Tool, parts, frame.sealed_payload.clone())
        }
        _ => return Ok(None),
    };
    let ordinal = history.messages().last().map_or(Ok(1), |m| m.ordinal.checked_add(1).ok_or(ContextError::Capacity))?;
    let authority = match envelope.authority {
        hm_schema::events::Authority::UserAsserted => Authority::UserAsserted,
        hm_schema::events::Authority::ExternalObserved => Authority::ExternalObserved,
        hm_schema::events::Authority::ToolObserved => Authority::ToolObserved,
        hm_schema::events::Authority::RuntimeFact => Authority::RuntimeFact,
        hm_schema::events::Authority::AssistantGenerated => Authority::AssistantGenerated,
        hm_schema::events::Authority::DerivedInference => Authority::DerivedInference,
    };
    let mut message = SourceMessage { id, ordinal, role, parts, occurred_at_ns: (envelope.event_time_ns != 0).then_some(envelope.event_time_ns), recorded_at_ns: frame.header.wall_timestamp_ns.get(), authority, source_digest: String::new() };
    message.source_digest = message.computed_digest()?;
    Ok(Some((message, bytes)))
}
