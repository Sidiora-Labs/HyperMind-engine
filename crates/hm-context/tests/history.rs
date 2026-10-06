use hm_context::history::{SourceHistory, SourceRelation};
use hm_context::{Authority, ContextError, MessagePart, MessageRole, Scope, SourceMessage};

fn scope() -> Scope { Scope { owner_id: "owner".into(), project_id: "project".into(), workspace_id: Some("workspace".into()) } }
fn message(id: &str, ordinal: u64, role: MessageRole, text: &str) -> SourceMessage {
    let mut message = SourceMessage { id: id.into(), ordinal, role, parts: vec![MessagePart::Text { text: text.into() }], occurred_at_ns: None, recorded_at_ns: 100, authority: Authority::UserAsserted, source_digest: String::new() };
    message.source_digest = message.computed_digest().unwrap();
    message
}

#[test]
fn immutable_replay_order_and_exact_scoped_bytes() {
    let mut history = SourceHistory::new(scope(), "session").unwrap();
    let original = message("first", 9, MessageRole::User, "hello");
    let bytes = b" original\r\n\0\xff".to_vec();
    assert!(!history.ingest(original.clone(), bytes.clone()).unwrap().replayed);
    assert!(history.ingest(original.clone(), bytes.clone()).unwrap().replayed);
    assert!(matches!(history.ingest(original, b"changed".to_vec()), Err(ContextError::Conflict)));
    assert!(history.ingest(message("earlier", 8, MessageRole::User, "bad"), vec![]).is_err());
    let span = history.source_span("first").unwrap();
    assert_eq!(history.recover(&scope(), &span).unwrap(), bytes);
    let mut other = scope(); other.owner_id = "other".into();
    assert!(matches!(history.recover(&other, &span), Err(ContextError::ScopeMismatch)));
    let mut stale = span.clone(); stale.source_digest = "bad".into();
    assert!(matches!(history.recover(&scope(), &stale), Err(ContextError::Stale)));
    let mut range = span; range.byte_end += 1;
    assert!(history.recover(&scope(), &range).is_err());
    assert_eq!(history.cursor().sequence, 1);
}

#[test]
fn revisions_tombstones_and_forks_preserve_originals() {
    let mut history = SourceHistory::new(scope(), "parent").unwrap();
    history.ingest(message("one", 1, MessageRole::Assistant, "old"), b"old".to_vec()).unwrap();
    let child = history.freeze_child("child").unwrap();
    history.ingest(message("two", 2, MessageRole::Assistant, "new"), b"new".to_vec()).unwrap();
    let relation = SourceRelation::Regenerate { id: "regenerate".into(), original_id: "one".into(), replacement_id: "two".into() };
    history.relate(relation.clone()).unwrap();
    assert!(history.relate(relation).unwrap().replayed);
    assert_eq!(history.visible_messages()[0].id, "two");
    assert_eq!(child.visible_messages()[0].id, "one");
    assert_eq!(child.cursor().sequence, 1);
    assert_eq!(child.parent().unwrap().cursor.sequence, 1);
    assert_eq!(history.recover(&scope(), &history.source_span("one").unwrap()).unwrap(), b"old");
    history.relate(SourceRelation::Tombstone { id: "delete".into(), source_id: "two".into() }).unwrap();
    assert!(history.visible_messages().is_empty());
    assert_eq!(history.messages().len(), 2);
    let checkpoint = history.export_canonical().unwrap();
    let restored = SourceHistory::restore_canonical(&checkpoint).unwrap();
    assert_eq!(restored.export_canonical().unwrap(), checkpoint);
    assert!(restored.visible_messages().is_empty());
    assert_eq!(restored.recover(&scope(), &restored.source_span("two").unwrap()).unwrap(), b"new");
    let child_export = child.export_canonical().unwrap();
    assert_eq!(SourceHistory::restore_canonical(&child_export).unwrap().parent(), child.parent());
}

#[test]
fn restore_is_atomic_and_corruption_and_invalid_ingestion_fail_closed() {
    let mut history = SourceHistory::new(scope(), "session").unwrap();
    history.ingest(message("one", 1, MessageRole::User, "a"), vec![1]).unwrap();
    let before = history.export_canonical().unwrap();
    let mut corrupted: serde_json::Value = serde_json::from_slice(&before).unwrap();
    corrupted["events"][0]["original_bytes"] = serde_json::json!([2]);
    assert!(history.restore_checkpoint(&serde_json::to_vec(&corrupted).unwrap()).is_err());
    assert_eq!(history.export_canonical().unwrap(), before);
    let foreign = SourceHistory::new(scope(), "other").unwrap().export_canonical().unwrap();
    assert!(matches!(history.restore_checkpoint(&foreign), Err(ContextError::ScopeMismatch)));
    let mut invalid = message("invalid", 2, MessageRole::User, "text"); invalid.source_digest.clear();
    assert!(history.ingest(invalid, vec![]).is_err());
    assert!(history.relate(SourceRelation::Edit { id: "edit".into(), original_id: "one".into(), replacement_id: "missing".into() }).is_err());
    history.ingest(message("two", 2, MessageRole::User, "b"), vec![2]).unwrap();
    assert!(history.relate(SourceRelation::Regenerate { id: "regen".into(), original_id: "one".into(), replacement_id: "two".into() }).is_err());
    history.relate(SourceRelation::Edit { id: "edit".into(), original_id: "one".into(), replacement_id: "two".into() }).unwrap();
    assert_eq!(history.visible_messages()[0].id, "two");
}
