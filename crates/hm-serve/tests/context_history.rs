use hm_context::{history::SourceRelation, Authority, MessagePart, MessageRole, Scope, SourceMessage};
use hm_core::{ActorId, ConversationId};
use hm_schema::{event::{self, CURRENT_SCHEMA_VERSION}, events::{EventEnvelope, EventPayload, Retention, Sensitivity, UserMsg, ToolCall, ToolResult, ResultStatus}};
use hm_serve::{actor::{ActorConfig, ActorEngine, IncomingEvent}, context_history::{self, SourceIngestion, RelationIngestion, ForkRequest}};

fn config(path: &std::path::Path) -> ActorConfig { ActorConfig { actor_directory: path.into(), actor: ActorId::new(17), user: [3;16], kek: [4;32], projection_map_bytes: 16*1024*1024 } }
fn scope() -> Scope { Scope { owner_id: "owner".into(), project_id: "project".into(), workspace_id: Some("workspace".into()) } }
fn request(id: &str, ordinal: u64, text: &str) -> SourceIngestion {
    let mut message = SourceMessage { id: id.into(), ordinal, role: MessageRole::User, parts: vec![MessagePart::Text { text: text.into() }, MessagePart::Opaque { media_type: "image/png".into(), reference: "artifact:image".into(), digest: "a".repeat(64) }], occurred_at_ns: Some(1_800_000_000_123_456_789), recorded_at_ns: 1_800_000_000_987_654_321, authority: Authority::UserAsserted, source_digest: String::new() };
    message.source_digest = message.computed_digest().unwrap();
    SourceIngestion { version: 1, scope: scope(), session_id: "session".into(), conversation: "conversation".into(), message, original_bytes: b"original\r\n\xff\0".to_vec() }
}
fn native(payload: EventPayload, kind: hm_ledger::frame::EventKind, conversation: &str) -> IncomingEvent {
    IncomingEvent { kind, conversation: ConversationId::derive(conversation), payload: event::encode_event_envelope(&EventEnvelope { schema_version: CURRENT_SCHEMA_VERSION, payload, connection_id: None, client_seq: 0, client_event_index: 0, client_event_count: 1, origin_actor: 0, run_id: None, model_provenance: None, authority: hm_schema::events::Authority::ExternalObserved, retention: Retention::Durable, sensitivity: Sensitivity::Personal, event_time_ns: 1_800_000_000_000_000_111 }) }
}

#[tokio::test]
async fn source_relations_frozen_fork_and_exact_recovery_survive_restart() {
    let dir = tempfile::tempdir().unwrap();
    let engine = ActorEngine::open(config(dir.path())).await.unwrap();
    let first = request("source-one", 10, "original");
    let receipt = context_history::ingest(&engine, &scope(), &first).await.unwrap();
    let span = receipt.source_span.clone().unwrap();
    let count = engine.stats().await.unwrap().log_events;
    let duplicate = context_history::ingest(&engine, &scope(), &first).await.unwrap();
    assert!(duplicate.replayed);
    assert_eq!(duplicate.last_lsn, receipt.last_lsn);
    assert_eq!(engine.stats().await.unwrap().log_events, count);
    let second = request("source-two", 11, "correction");
    context_history::ingest(&engine, &scope(), &second).await.unwrap();
    context_history::relate(&engine, &scope(), &RelationIngestion { version:1, scope:scope(), session_id:"session".into(), conversation:"conversation".into(), relation: SourceRelation::Edit { id:"edit-one".into(), original_id:"source-one".into(), replacement_id:"source-two".into() } }).await.unwrap();
    let fork = ForkRequest { version:1, scope:scope(), parent_session_id:"session".into(), parent_conversation:"conversation".into(), child_session_id:"child".into(), child_conversation:"child-room".into() };
    context_history::fork(&engine, &scope(), &fork).await.unwrap();
    context_history::relate(&engine, &scope(), &RelationIngestion { version:1, scope:scope(), session_id:"session".into(), conversation:"conversation".into(), relation:SourceRelation::Tombstone { id:"delete-two".into(), source_id:"source-two".into() } }).await.unwrap();
    engine.shutdown().await.unwrap();
    let engine = ActorEngine::open(config(dir.path())).await.unwrap();
    let state = context_history::replay(&engine, &scope(), "session", "conversation").await.unwrap();
    assert!(state.history.visible_messages().is_empty());
    assert_eq!(state.history.message("source-one").unwrap(), &first.message);
    assert_eq!(context_history::recover(&engine, &scope(), "session", "conversation", &span).await.unwrap(), first.original_bytes);
    let child = context_history::replay(&engine, &scope(), "child", "child-room").await.unwrap();
    assert_eq!(child.history.visible_messages()[0].id, "source-two");
    assert!(context_history::fork(&engine, &scope(), &fork).await.unwrap().replayed);
    let mut other = scope(); other.project_id = "other-project".into();
    assert!(context_history::recover(&engine, &other, "session", "conversation", &span).await.is_err());
    assert!(context_history::ingest(&engine, &other, &first).await.is_err());
    let count = engine.stats().await.unwrap().log_events;
    let mut corrupt = first.clone(); corrupt.original_bytes.push(9);
    assert!(context_history::ingest(&engine, &scope(), &corrupt).await.is_err());
    let mut corrupt = first.clone(); corrupt.message.source_digest.clear();
    assert!(context_history::ingest(&engine, &scope(), &corrupt).await.is_err());
    let mut privileged = first.clone(); privileged.message.id = "privileged-source".into(); privileged.message.ordinal = 12; privileged.message.authority = Authority::RuntimeFact;
    privileged.message.source_digest = privileged.message.computed_digest().unwrap();
    assert!(context_history::ingest(&engine, &scope(), &privileged).await.is_err());
    let mut wrong_role = first.clone(); wrong_role.message.id = "wrong-role-source".into(); wrong_role.message.ordinal = 12; wrong_role.message.authority = Authority::AssistantGenerated;
    wrong_role.message.source_digest = wrong_role.message.computed_digest().unwrap();
    assert!(context_history::prepare_ingest(&engine, &scope(), &wrong_role).await.is_err());
    assert_eq!(engine.stats().await.unwrap().log_events, count);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn native_tools_keep_parts_raw_envelopes_time_and_binary_diagnostics() {
    let dir = tempfile::tempdir().unwrap();
    let engine = ActorEngine::open(config(dir.path())).await.unwrap();
    engine.append(vec![native(EventPayload::UserMsg(Box::new(UserMsg {content:b"hello".to_vec()})), hm_ledger::frame::EventKind::UserMsg, "native-room")]).await.unwrap();
    let call = native(EventPayload::ToolCall(Box::new(ToolCall { call_id:vec![0,255,1], tool_name:"lookup".into(), arguments:b"{\"key\":1}".to_vec() })), hm_ledger::frame::EventKind::ToolCall, "native-room");
    let original_call = call.payload.clone();
    let call_lsn = engine.append(vec![call]).await.unwrap().last_lsn.get();
    engine.append(vec![native(EventPayload::ToolResult(Box::new(ToolResult { call_id:vec![0,255,1], tool_call_lsn:call_lsn, status:ResultStatus::OutcomeUnknown, result:b"uncertain".to_vec() })), hm_ledger::frame::EventKind::ToolResult, "native-room")]).await.unwrap();
    let state = context_history::replay(&engine, &scope(), "native-session", "native-room").await.unwrap();
    let messages = state.history.messages();
    assert_eq!(messages.len(), 3);
    assert!(matches!(&messages[1].parts[0], MessagePart::ToolCall {call_id,name,arguments} if call_id == "bytes:00ff01" && name == "lookup" && arguments == "{\"key\":1}"));
    assert!(matches!(&messages[2].parts[0], MessagePart::ToolResult {call_id,content,failed} if call_id == "bytes:00ff01" && content == "uncertain" && *failed));
    assert_eq!(messages[1].occurred_at_ns, Some(1_800_000_000_000_000_111));
    let span = state.history.source_span(&format!("lsn:{call_lsn}")).unwrap();
    assert_eq!(state.history.recover(&scope(), &span).unwrap(), original_call);
    let bad_call = native(EventPayload::ToolCall(Box::new(ToolCall {call_id:b"binary".to_vec(), tool_name:"binary_tool".into(), arguments:vec![255,0]})), hm_ledger::frame::EventKind::ToolCall, "native-room");
    let original_binary = bad_call.payload.clone();
    let bad_lsn = engine.append(vec![bad_call]).await.unwrap().last_lsn.get();
    engine.append(vec![native(EventPayload::ToolResult(Box::new(ToolResult {call_id:b"binary".to_vec(),tool_call_lsn:bad_lsn,status:ResultStatus::Ok,result:vec![255,0]})),hm_ledger::frame::EventKind::ToolResult,"native-room")]).await.unwrap();
    let old = engine.append(vec![native(EventPayload::DeliveredMsg(Box::new(hm_schema::events::DeliveredMsg { content:b"draft".to_vec() })),hm_ledger::frame::EventKind::DeliveredMsg,"native-room")]).await.unwrap().last_lsn.get();
    let new = engine.append(vec![native(EventPayload::DeliveredMsg(Box::new(hm_schema::events::DeliveredMsg { content:b"regenerated".to_vec() })),hm_ledger::frame::EventKind::DeliveredMsg,"native-room")]).await.unwrap().last_lsn.get();
    context_history::relate(&engine,&scope(),&RelationIngestion {version:1,scope:scope(),session_id:"native-session".into(),conversation:"native-room".into(),relation:SourceRelation::Regenerate {id:"native-regeneration".into(),original_id:format!("lsn:{old}"),replacement_id:format!("lsn:{new}")}}).await.unwrap();
    engine.shutdown().await.unwrap();
    let engine = ActorEngine::open(config(dir.path())).await.unwrap();
    let state = context_history::replay(&engine, &scope(), "native-session", "native-room").await.unwrap();
    assert!(!state.history.visible_messages().iter().any(|m| m.id == format!("lsn:{old}")));
    assert_eq!(state.history.recover(&scope(), &state.history.source_span(&format!("lsn:{old}")).unwrap()).unwrap(), b"draft");
    assert_eq!(state.unsupported_parts.len(), 2);
    let binary = state.history.message(&format!("lsn:{bad_lsn}")).unwrap();
    assert!(matches!(&binary.parts[..], [MessagePart::Opaque {..}]));
    assert_eq!(state.history.recover(&scope(), &state.history.source_span(&binary.id).unwrap()).unwrap(), original_binary);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn concurrent_replay_is_idempotent_and_imported_knowledge_is_not_an_utterance() {
    let dir = tempfile::tempdir().unwrap();
    let engine = ActorEngine::open(config(dir.path())).await.unwrap();
    let request = request("same-source", 1, "same");
    let trusted_scope = scope();
    let (a,b) = tokio::join!(context_history::ingest(&engine,&trusted_scope,&request),context_history::ingest(&engine,&trusted_scope,&request));
    let a = a.unwrap(); let b = b.unwrap();
    assert_ne!(a.replayed, b.replayed);
    assert_eq!(engine.stats().await.unwrap().log_events, 1);
    let content = serde_json::to_vec(&serde_json::json!({"format":"hypermid_import_v1","scope":scope(),"import_id":"knowledge","entry":{"kind":"revision","payload":{"content":"imported evidence"}}})).unwrap();
    engine.append(vec![native(EventPayload::UserMsg(Box::new(UserMsg {content})),hm_ledger::frame::EventKind::UserMsg,"knowledge-room")]).await.unwrap();
    let state = context_history::replay(&engine,&scope(),"knowledge-session","knowledge-room").await.unwrap();
    assert!(state.history.messages().is_empty());
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn native_writer_between_preparation_and_admission_is_refused_without_lost_sources() {
    let dir = tempfile::tempdir().unwrap();
    let engine = ActorEngine::open(config(dir.path())).await.unwrap();
    let request = request("raced-source", 2, "immutable input");
    let prepared = context_history::prepare_ingest(&engine, &scope(), &request).await.unwrap();
    engine.append(vec![native(EventPayload::UserMsg(Box::new(UserMsg {content:b"concurrent native input".to_vec()})),hm_ledger::frame::EventKind::UserMsg,"conversation")]).await.unwrap();
    let before = engine.stats().await.unwrap().log_events;
    match context_history::commit_ingest(&engine, &scope(), &prepared).await {
        Err(context_history::HistoryError::Ledger(error)) => assert_eq!(error.code, hm_core::ErrorCode::SequenceViolation),
        other => panic!("expected atomic tail refusal, got {other:?}"),
    }
    assert_eq!(engine.stats().await.unwrap().log_events, before);
    let state = context_history::replay(&engine, &scope(), "session", "conversation").await.unwrap();
    assert_eq!(state.history.messages().len(), 1);
    assert!(state.history.message("raced-source").is_err());
    let accepted = context_history::ingest(&engine, &scope(), &request).await.unwrap();
    let prepared_retry = context_history::prepare_ingest(&engine, &scope(), &request).await.unwrap();
    engine.append(vec![native(EventPayload::UserMsg(Box::new(UserMsg {content:b"new native tail".to_vec()})),hm_ledger::frame::EventKind::UserMsg,"conversation")]).await.unwrap();
    let duplicate = context_history::commit_ingest(&engine, &scope(), &prepared_retry).await.unwrap();
    assert!(duplicate.replayed);
    assert_eq!(duplicate.last_lsn, accepted.last_lsn);
    assert_eq!(duplicate.cursor, accepted.cursor);
    let total = engine.stats().await.unwrap().log_events;
    engine.shutdown().await.unwrap();
    let engine = ActorEngine::open(config(dir.path())).await.unwrap();
    let duplicate = context_history::ingest(&engine, &scope(), &request).await.unwrap();
    assert!(duplicate.replayed);
    assert_eq!(engine.stats().await.unwrap().log_events, total);
    let state = context_history::replay(&engine, &scope(), "session", "conversation").await.unwrap();
    assert_eq!(state.history.messages().len(), 3);
    assert_eq!(context_history::recover(&engine, &scope(), "session", "conversation", &accepted.source_span.unwrap()).await.unwrap(), request.original_bytes);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn native_forget_retracts_sources_and_frozen_children_without_destroying_recovery() {
    let dir = tempfile::tempdir().unwrap();
    let engine = ActorEngine::open(config(dir.path())).await.unwrap();
    let native_lsn = engine.append(vec![native(EventPayload::UserMsg(Box::new(UserMsg {content:b"native source".to_vec()})),hm_ledger::frame::EventKind::UserMsg,"conversation")]).await.unwrap().last_lsn.get();
    let mut delivered = native(EventPayload::DeliveredMsg(Box::new(hm_schema::events::DeliveredMsg {content:b"run source".to_vec()})),hm_ledger::frame::EventKind::DeliveredMsg,"conversation");
    let mut envelope = event::verify_event(&delivered.payload,event::EventKind::DeliveredMsg,event::Boundary::Disk).unwrap().envelope;
    envelope.run_id = Some(b"retracted-run".to_vec());
    delivered.payload = event::encode_event_envelope(&envelope);
    let run_lsn = engine.append(vec![delivered]).await.unwrap().last_lsn.get();
    let request = request("versioned-source", 3, "versioned source");
    let source_receipt = context_history::ingest(&engine,&scope(),&request).await.unwrap();
    let fork = ForkRequest {version:1,scope:scope(),parent_session_id:"session".into(),parent_conversation:"conversation".into(),child_session_id:"child".into(),child_conversation:"child-room".into()};
    context_history::fork(&engine,&scope(),&fork).await.unwrap();
    let before = context_history::replay(&engine,&scope(),"child","child-room").await.unwrap();
    assert_eq!(before.history.visible_messages().len(),3);
    let digest = hm_context::digest_bytes(&before.history.export_canonical().unwrap());
    let mcp = hm_mcp::McpServer::new(engine.clone());
    for target in [native_lsn,source_receipt.last_lsn] {
        let result = mcp.forget_envelope(hm_mcp::ForgetInput {action:hm_mcp::ForgetAction::Fade,lsn:Some(target),run_id:None,admin_token:None}).await;
        assert!(result.ok, "{result:?}");
    }
    let result = mcp.forget_envelope(hm_mcp::ForgetInput {action:hm_mcp::ForgetAction::RetractRun,lsn:None,run_id:Some("retracted-run".into()),admin_token:None}).await;
    assert!(result.ok, "{result:?}");
    let child = context_history::replay(&engine,&scope(),"child","child-room").await.unwrap();
    assert!(child.history.visible_messages().is_empty());
    assert!(child.visible_source_uris().is_empty());
    assert_ne!(hm_context::digest_bytes(&child.history.export_canonical().unwrap()),digest);
    assert_eq!(child.history.recover(&scope(),&child.history.source_span(&format!("lsn:{native_lsn}")).unwrap()).unwrap(),b"native source");
    assert_eq!(child.history.recover(&scope(),&child.history.source_span(&format!("lsn:{run_lsn}")).unwrap()).unwrap(),b"run source");
    engine.shutdown().await.unwrap();
    let engine = ActorEngine::open(config(dir.path())).await.unwrap();
    for (session,conversation) in [("session","conversation"),("child","child-room")] {
        let history = context_history::replay(&engine,&scope(),session,conversation).await.unwrap();
        assert!(history.history.visible_messages().is_empty());
        assert_eq!(context_history::recover(&engine,&scope(),session,conversation,&source_receipt.source_span.clone().unwrap()).await.unwrap(),request.original_bytes);
    }
    let duplicate = context_history::ingest(&engine,&scope(),&request).await.unwrap();
    assert!(duplicate.replayed);
    assert!(context_history::replay(&engine,&scope(),"session","conversation").await.unwrap().history.visible_messages().is_empty());
    let mut fresh = request.clone();
    fresh.message.id = "post-forget-source".into(); fresh.message.ordinal = 4;
    fresh.message.source_digest = fresh.message.computed_digest().unwrap();
    let fresh_receipt = context_history::ingest(&engine,&scope(),&fresh).await.unwrap();
    engine.shutdown().await.unwrap();
    let engine = ActorEngine::open(config(dir.path())).await.unwrap();
    let retry = context_history::ingest(&engine,&scope(),&fresh).await.unwrap();
    assert!(retry.replayed);
    assert_eq!(retry.cursor,fresh_receipt.cursor);
    assert_eq!(context_history::replay(&engine,&scope(),"session","conversation").await.unwrap().history.visible_messages()[0].id,"post-forget-source");
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn versioned_source_after_knowledge_import_reaches_real_remember_boundary() {
    let dir = tempfile::tempdir().unwrap();
    let engine = ActorEngine::open(config(dir.path())).await.unwrap();
    let trusted = Scope { owner_id:"journey-owner".into(), project_id:"journey-project".into(), workspace_id:Some("workspace".into()) };
    let server = hm_mcp::McpServer::new(engine.clone()).with_context_scope(trusted.clone());
    let entries = (1..=2).map(|revision| {
        let payload = serde_json::json!({"record_id":"pressure-record","revision":revision,"content":format!("Observed pressure revision {revision}"),"recorded_at_ns":"1791288000123456789","provenance":["pressure-sensor"]});
        hm_serve::hypermid_import::ImportEntry { source_id:format!("pressure-revision-{revision}"),kind:"revision".into(),digest:hm_context::digest_bytes(&serde_json::to_vec(&payload).unwrap()),payload }
    }).collect();
    let mut bundle = hm_serve::hypermid_import::ImportBundle {version:1,import_id:"pressure-knowledge-export".into(),scope:trusted.clone(),entries,digest:String::new()};
    bundle.digest = bundle.computed_digest().unwrap();
    for max_entries in [1,128] {
        let input: hm_mcp::RememberInput = serde_json::from_value(serde_json::json!({"conversation":"pressure-conversation","kind":"user","context":{"operation":"import","request":bundle,"max_entries":max_entries}})).unwrap();
        let result = server.remember_envelope(input).await;
        assert!(result.ok,"{result:?}");
    }
    let text = "Measured pressure is 20 kPa. ".repeat(90);
    let mut message = SourceMessage {id:"reading-20".into(),ordinal:1,role:MessageRole::User,parts:vec![MessagePart::Text{text:text.clone()}],occurred_at_ns:Some(1_791_288_000_123_456_790),recorded_at_ns:1_791_288_000_123_456_790,authority:Authority::UserAsserted,source_digest:String::new()};
    message.source_digest = message.computed_digest().unwrap();
    let source = SourceIngestion {version:1,scope:trusted.clone(),session_id:"pressure-session".into(),conversation:"pressure-conversation".into(),message,original_bytes:format!("{{ \"id\": \"reading-20\", \"text\": {} }}\r\n",serde_json::to_string(&text).unwrap()).into_bytes()};
    let wire = serde_json::json!({"conversation":"pressure-conversation","kind":"user","context":{"operation":"source","request":source}});
    let source_from_wire: SourceIngestion = serde_json::from_value(wire["context"]["request"].clone()).unwrap();
    source_from_wire.message.validate().unwrap();
    context_history::prepare_ingest(&engine,&trusted,&source_from_wire).await.unwrap();
    let result = server.remember_envelope(serde_json::from_value(wire).unwrap()).await;
    assert!(result.ok,"{result:?}");
    let recovered = context_history::recover(&engine,&trusted,"pressure-session","pressure-conversation",&serde_json::from_value::<context_history::HistoryReceipt>(result.items[0].clone()).unwrap().source_span.unwrap()).await.unwrap();
    assert_eq!(recovered,source.original_bytes);
    engine.shutdown().await.unwrap();
}
