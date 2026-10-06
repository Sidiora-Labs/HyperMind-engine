use crate::types::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct CapabilityProfile {
    pub user: bool,
    pub assistant: bool,
    pub tool: bool,
    pub text: bool,
    pub tool_calls: bool,
    pub tool_results: bool,
    pub opaque: bool,
}
impl Default for CapabilityProfile {
    fn default() -> Self { Self { user: true, assistant: true, tool: true, text: true, tool_calls: true, tool_results: true, opaque: true } }
}

pub trait TokenCounter {
    fn count(&self, serialized_envelope: &[u8]) -> Result<u64, ContextError>;
}
impl<F> TokenCounter for F where F: Fn(&[u8]) -> Result<u64, ContextError> {
    fn count(&self, bytes: &[u8]) -> Result<u64, ContextError> { self(bytes) }
}
pub struct UnsupportedTokenizer;
impl TokenCounter for UnsupportedTokenizer {
    fn count(&self, _: &[u8]) -> Result<u64, ContextError> { Err(ContextError::Unavailable("exact tokenizer unsupported".into())) }
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error(transparent)] Context(#[from] ContextError),
    #[error("required context needs {required} tokens, budget allows {available}")] RequiredOverflow { required: u64, available: u64 },
    #[error("provider does not support {0}")] Unsupported(String),
    #[error("tool adjacency violation: {0}")] ToolAdjacency(String),
    #[error("tokenizer unsupported or unavailable: {0}")] TokenizerUnsupported(String),
}

pub struct RenderRequest<'a> {
    pub scope: Scope,
    pub session_id: String,
    pub cursor: Cursor,
    pub generation: u64,
    pub messages: &'a [SourceMessage],
    pub source_spans: &'a BTreeMap<String, SourceSpan>,
    pub blocks: &'a [ContextBlock],
    pub required_message_ids: &'a [String],
    pub budget: TokenBudget,
    pub profile: CapabilityProfile,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RenderedMessage {
    pub id: String,
    pub role: MessageRole,
    pub parts: Vec<MessagePart>,
    pub authority: Authority,
    pub provenance: Vec<SourceSpan>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RenderedContext {
    pub messages: Vec<RenderedMessage>,
    pub report: ContextReport,
    pub digest: String,
}
struct Unit { messages: Vec<RenderedMessage>, ids: Vec<String>, required: bool, block: Option<ContextBlock> }

fn supports(message: &RenderedMessage, p: CapabilityProfile) -> Result<(), RenderError> {
    let role = match message.role { MessageRole::User => p.user, MessageRole::Assistant => p.assistant, MessageRole::Tool => p.tool };
    if !role { return Err(RenderError::Unsupported(format!("role {:?}", message.role))); }
    for part in &message.parts {
        let supported = match part { MessagePart::Text { .. } => p.text, MessagePart::ToolCall { .. } => p.tool_calls, MessagePart::ToolResult { .. } => p.tool_results, MessagePart::Opaque { .. } => p.opaque };
        if !supported { return Err(RenderError::Unsupported("message part".into())); }
    }
    Ok(())
}
fn count(messages: &[RenderedMessage], counter: &dyn TokenCounter) -> Result<u64, RenderError> {
    let bytes = serde_json::to_vec(messages).map_err(ContextError::from)?;
    counter.count(&bytes).map_err(|e| RenderError::TokenizerUnsupported(e.to_string()))
}

pub fn render_context(request: RenderRequest<'_>, counter: &dyn TokenCounter) -> Result<RenderedContext, RenderError> {
    request.scope.validate()?;
    validate_id(&request.session_id)?;
    request.cursor.validate()?;
    let available = request.budget.available()?;
    let required: BTreeSet<_> = request.required_message_ids.iter().cloned().collect();
    let mut seen = BTreeSet::new();
    let mut units: Vec<Unit> = Vec::new();
    let mut pending: BTreeMap<String, String> = BTreeMap::new();
    let mut last_ordinal = None;
    for source in request.messages {
        source.validate()?;
        if !seen.insert(source.id.clone()) || last_ordinal.is_some_and(|n| n >= source.ordinal) { return Err(ContextError::Invalid("duplicate identity or unordered source".into()).into()); }
        last_ordinal = Some(source.ordinal);
        if !pending.is_empty() && source.role != MessageRole::Tool { return Err(RenderError::ToolAdjacency(source.id.clone())); }
        let span = request.source_spans.get(&source.id)
            .ok_or_else(|| ContextError::Invalid("missing original source span".into()))?;
        if span.source_id != source.id || span.source_digest != source.source_digest || span.byte_start > span.byte_end {
            return Err(ContextError::Invalid("invalid original source span".into()).into());
        }
        let mut rendered = RenderedMessage { id: source.id.clone(), role: source.role, parts: source.parts.clone(), authority: source.authority, provenance: vec![span.clone()] };
        for (index, part) in rendered.parts.iter_mut().enumerate() {
            match part {
                MessagePart::ToolCall { call_id, .. } => {
                    if source.role != MessageRole::Assistant || pending.contains_key(call_id) { return Err(RenderError::ToolAdjacency(source.id.clone())); }
                    let qualified = format!("{}:{}:{}", source.id.len(), source.id, index);
                    pending.insert(call_id.clone(), qualified.clone());
                    *call_id = qualified;
                }
                MessagePart::ToolResult { call_id, .. } => {
                    if source.role != MessageRole::Tool { return Err(RenderError::ToolAdjacency(source.id.clone())); }
                    *call_id = pending.remove(call_id).ok_or_else(|| RenderError::ToolAdjacency(source.id.clone()))?;
                }
                _ => {}
            }
        }
        if source.role == MessageRole::Tool {
            if !source.parts.iter().any(|p| matches!(p, MessagePart::ToolResult { .. })) { return Err(RenderError::ToolAdjacency(source.id.clone())); }
            let unit = units.last_mut().ok_or_else(|| RenderError::ToolAdjacency(source.id.clone()))?;
            unit.required |= required.contains(&source.id);
            unit.ids.push(source.id.clone()); unit.messages.push(rendered);
        } else {
            units.push(Unit { messages: vec![rendered], ids: vec![source.id.clone()], required: required.contains(&source.id), block: None });
        }
    }
    if !pending.is_empty() { return Err(RenderError::ToolAdjacency("missing tool result".into())); }
    if !required.is_subset(&seen) { return Err(ContextError::Invalid("unknown required source".into()).into()); }
    for block in request.blocks {
        validate_id(&block.id)?;
        if !seen.insert(block.id.clone()) { return Err(ContextError::Invalid("duplicate block identity".into()).into()); }
        let label = serde_json::to_string(&(block.authority, &block.provenance)).map_err(ContextError::from)?;
        units.push(Unit { messages: vec![RenderedMessage { id: block.id.clone(), role: MessageRole::User, parts: vec![MessagePart::Text { text: format!("Context evidence {label}\n{}", block.text) }], authority: block.authority, provenance: block.provenance.clone() }], ids: vec![block.id.clone()], required: block.required, block: Some(block.clone()) });
    }
    let mut selected = BTreeSet::new();
    let mut omitted = Vec::new();
    for (index, unit) in units.iter().enumerate() {
        let support = unit.messages.iter().try_for_each(|m| supports(m, request.profile));
        if unit.required { support?; selected.insert(index); }
        else if support.is_err() { omitted.extend(unit.ids.iter().map(|id| Omission { id: id.clone(), reason: "provider capability unsupported".into() })); }
    }
    let assemble = |selected: &BTreeSet<usize>| -> Vec<RenderedMessage> { selected.iter().flat_map(|i| units[*i].messages.clone()).collect() };
    let required_count = count(&assemble(&selected), counter)?;
    if required_count > available { return Err(RenderError::RequiredOverflow { required: required_count, available }); }
    for (index, unit) in units.iter().enumerate() {
        if unit.required || unit.messages.iter().any(|m| supports(m, request.profile).is_err()) { continue; }
        selected.insert(index);
        if count(&assemble(&selected), counter)? > available {
            selected.remove(&index);
            omitted.extend(unit.ids.iter().map(|id| Omission { id: id.clone(), reason: "token budget".into() }));
        }
    }
    let messages = assemble(&selected);
    let token_count = count(&messages, counter)?;
    let mut report = ContextReport { version: CONTRACT_VERSION, scope: request.scope, session_id: request.session_id, cursor: request.cursor, generation: request.generation, blocks: selected.iter().filter_map(|i| units[*i].block.clone()).collect(), included: selected.iter().flat_map(|i| units[*i].ids.clone()).collect(), omitted, gaps: Vec::new(), token_count, digest: String::new() };
    report.digest = digest_bytes(&serde_json::to_vec(&report).map_err(ContextError::from)?);
    let digest = digest_bytes(&serde_json::to_vec(&(&messages, &report)).map_err(ContextError::from)?);
    Ok(RenderedContext { messages, report, digest })
}
