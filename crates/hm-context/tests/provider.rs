use hm_context::provider::*;
use hm_context::*;
use hm_context::history::SourceHistory;
use std::collections::BTreeMap;
struct ByteVocabulary;
impl TokenCounter for ByteVocabulary {
    fn count(&self, bytes: &[u8]) -> Result<u64, ContextError> { Ok(bytes.len() as u64) }
}
fn message(id: &str, ordinal: u64, role: MessageRole, parts: Vec<MessagePart>) -> SourceMessage {
    let mut m = SourceMessage { id: id.into(), ordinal, role, parts, occurred_at_ns: None, recorded_at_ns: 1, authority: Authority::ToolObserved, source_digest: String::new() };
    m.source_digest = m.computed_digest().unwrap(); m
}
fn request<'a>(messages: &'a [SourceMessage], source_spans: &'a BTreeMap<String, SourceSpan>, blocks: &'a [ContextBlock], required: &'a [String], tokens: u64) -> RenderRequest<'a> {
    RenderRequest { scope: Scope { owner_id: "owner".into(), project_id: "project".into(), workspace_id: None }, session_id: "session".into(), cursor: Cursor::default(), generation: 1, messages, source_spans, blocks, required_message_ids: required, budget: TokenBudget { context_tokens: tokens, reserved_output_tokens: 1, required_tokens: 0 }, profile: CapabilityProfile::default() }
}
#[test]
fn repeated_ids_atomic_admission() {
    let mut messages = Vec::new();
    for n in 0..2 {
        messages.push(message(&format!("a{n}"), n * 2, MessageRole::Assistant, vec![MessagePart::ToolCall { call_id: "same".into(), name: "read".into(), arguments: "{}".into() }]));
        messages.push(message(&format!("t{n}"), n * 2 + 1, MessageRole::Tool, vec![MessagePart::ToolResult { call_id: "same".into(), content: "ok".into(), failed: false }]));
    }
    let (_, spans) = history(&messages);
    let full = render_context(request(&messages, &spans, &[], &[], 10000), &ByteVocabulary).unwrap();
    let id = |m: &RenderedMessage| match &m.parts[0] { MessagePart::ToolCall { call_id, .. } | MessagePart::ToolResult { call_id, .. } => call_id.clone(), _ => panic!() };
    assert_eq!(id(&full.messages[0]), id(&full.messages[1]));
    assert_ne!(id(&full.messages[0]), id(&full.messages[2]));
    let limited = render_context(request(&messages, &spans, &[], &[], full.report.token_count), &ByteVocabulary).unwrap();
    assert_eq!(limited.messages.len(), 2); assert_eq!(limited.report.omitted.len(), 2);
}
#[test]
fn required_opaque_overflow_and_unsupported() {
    let messages = vec![message("image", 0, MessageRole::User, vec![MessagePart::Opaque { media_type: "image/png".into(), reference: "asset:one".into(), digest: digest_bytes(b"image-content") }])];
    let (history, spans) = history(&messages);
    let required = vec!["image".into()];
    assert!(matches!(render_context(request(&messages, &spans, &[], &required, 10), &ByteVocabulary), Err(RenderError::RequiredOverflow { .. })));
    assert!(matches!(render_context(request(&messages, &spans, &[], &required, 10000), &UnsupportedTokenizer), Err(RenderError::TokenizerUnsupported(_))));
    let result = render_context(request(&messages, &spans, &[], &required, 10000), &ByteVocabulary).unwrap();
    assert_eq!(result.messages[0].parts, messages[0].parts);
    assert_eq!(history.recover(history.scope(), &result.messages[0].provenance[0]).unwrap(), original("image"));
    let mut req = request(&messages, &spans, &[], &required, 10000); req.profile.opaque = false;
    assert!(matches!(render_context(req, &ByteVocabulary), Err(RenderError::Unsupported(_))));
}
#[test]
fn evidence_authority_and_canonical_digest() {
    let blocks = vec![ContextBlock { id: "memory".into(), text: "evidence".into(), authority: Authority::DerivedInference, provenance: vec![], tokens: u64::MAX, required: true }];
    let result = render_context(request(&[], &BTreeMap::new(), &blocks, &[], 10000), &ByteVocabulary).unwrap();
    assert_eq!(result.messages[0].role, MessageRole::User);
    assert!(result.report.token_count < 10000);
    let mut canonical = result.report.clone(); canonical.digest.clear();
    assert_eq!(result.report.digest, digest_bytes(&serde_json::to_vec(&canonical).unwrap()));
}
#[test]
fn orphan_tool_rejected() {
    let messages = vec![message("tool", 0, MessageRole::Tool, vec![MessagePart::ToolResult { call_id: "missing".into(), content: "x".into(), failed: false }])];
    let (_, spans) = history(&messages);
    assert!(matches!(render_context(request(&messages, &spans, &[], &[], 10000), &ByteVocabulary), Err(RenderError::ToolAdjacency(_))));
}

fn original(id: &str) -> Vec<u8> { format!("host envelope\r\n{id}\0").into_bytes() }
fn history(messages: &[SourceMessage]) -> (SourceHistory, BTreeMap<String, SourceSpan>) {
    let mut history = SourceHistory::new(Scope { owner_id: "owner".into(), project_id: "project".into(), workspace_id: None }, "session").unwrap();
    let mut spans = BTreeMap::new();
    for source in messages {
        history.ingest(source.clone(), original(&source.id)).unwrap();
        spans.insert(source.id.clone(), history.source_span(&source.id).unwrap());
    }
    (history, spans)
}
#[test]
fn source_spans_are_required_and_bound_before_token_accounting() {
    let messages = vec![message("source", 0, MessageRole::User, vec![MessagePart::Text { text: "parsed".into() }])];
    let (_, mut spans) = history(&messages);
    let required = vec!["source".into()];
    let full = render_context(request(&messages, &spans, &[], &required, 10000), &ByteVocabulary).unwrap();
    assert_eq!(full.report.token_count, serde_json::to_vec(&full.messages).unwrap().len() as u64);
    spans.get_mut("source").unwrap().source_digest = "incorrect".into();
    assert!(render_context(request(&messages, &spans, &[], &required, 10000), &ByteVocabulary).is_err());
    assert!(render_context(request(&messages, &BTreeMap::new(), &[], &required, 10000), &ByteVocabulary).is_err());
}
