use hm_context::Scope;
use hm_core::ActorId;
use hm_mcp::{ActivateInput, InspectInput, McpServer, RememberInput, RememberKind};
use hm_serve::actor::{ActorConfig, ActorEngine};
use serde_json::json;
fn config(root: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: root.join("1"),
        actor: ActorId::new(1),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}
fn scope() -> Scope {
    Scope {
        owner_id: "owner".into(),
        project_id: "project".into(),
        workspace_id: None,
    }
}
#[tokio::test]
async fn durable_session_context_uses_real_history_and_rejects_foreign_scope() {
    let directory = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    let server = McpServer::new(actor.clone()).with_context_scope(scope());
    let result = server
        .remember_envelope(RememberInput {
            context: None,
            conversation: "session".into(),
            content: "Original durable conversation".into(),
            kind: RememberKind::User,
            chunk_bytes: None,
            anchor: None,
            retention: None,
            sensitivity: None,
            vocabulary: None,
            source: None,
            derive: None,
            source_delivery: None,
            source_settlement: None,
            document: None,
            source_sync: None,
        })
        .await;
    assert!(result.ok, "{result:?}");
    let request = json!({"version":1,"scope":scope(),"session_id":"session","budget":{"context_tokens":4096,"reserved_output_tokens":256,"required_tokens":0},"generation":1});
    let output = server
        .activate_envelope(ActivateInput {
            conversation: "session".into(),
            query: String::new(),
            turn_text: String::new(),
            budget_tokens: 4096,
            context: Some(request.clone()),
        })
        .await;
    assert!(output.ok, "{output:?}");
    assert_eq!(output.items[0]["history"]["message_count"], 1);
    assert_eq!(
        output.items[0]["messages"][0]["parts"][0]["text"],
        "Original durable conversation"
    );
    let mut foreign = request;
    foreign["scope"]["owner_id"] = json!("foreign");
    let refused = server
        .activate_envelope(ActivateInput {
            conversation: "session".into(),
            query: String::new(),
            turn_text: String::new(),
            budget_tokens: 4096,
            context: Some(foreign),
        })
        .await;
    assert!(!refused.ok);
    let digest = output.items[0]["report"]["digest"].clone();
    drop(server);
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    let server = McpServer::new(actor).with_context_scope(scope());
    let inspected = server
        .inspect_envelope(InspectInput {
            uri: Some("hm://1/context/session".into()),
            ..Default::default()
        })
        .await;
    assert!(inspected.ok, "{inspected:?}");
    assert_eq!(inspected.items[0]["report"]["digest"], digest);
    let sessions = server
        .inspect_envelope(InspectInput {
            uri: Some("hm://1/context".into()),
            ..Default::default()
        })
        .await;
    assert_eq!(sessions.items[0]["sessions"][0]["scope"], json!(scope()));
}

async fn context_operation(
    server: &McpServer,
    conversation: &str,
    operation: serde_json::Value,
) -> hm_mcp::Envelope {
    server
        .remember_envelope(
            serde_json::from_value(
                json!({"conversation":conversation,"content":"","kind":"user","context":operation}),
            )
            .unwrap(),
        )
        .await
}
fn source(id: &str, ordinal: u64, text: &str) -> hm_context::SourceMessage {
    let mut message = hm_context::SourceMessage {
        id: id.into(),
        ordinal,
        role: hm_context::MessageRole::User,
        parts: vec![hm_context::MessagePart::Text { text: text.into() }],
        occurred_at_ns: Some(100),
        recorded_at_ns: 200,
        authority: hm_context::Authority::UserAsserted,
        source_digest: String::new(),
    };
    message.source_digest = message.computed_digest().unwrap();
    message
}
async fn activate_context(
    server: &McpServer,
    session: &str,
    conversation: &str,
) -> hm_mcp::Envelope {
    server.activate_envelope(serde_json::from_value(json!({"conversation":conversation,"budget_tokens":4096,"context":{"version":1,"scope":scope(),"session_id":session,"budget":{"context_tokens":4096,"reserved_output_tokens":256,"required_tokens":0}}})).unwrap()).await
}
async fn job(server: &McpServer, id: &str, action: serde_json::Value) -> hm_mcp::Envelope {
    context_operation(server,"session",json!({"operation":"job","request":{"version":1,"scope":scope(),"request_id":id,"action":action}})).await
}
#[tokio::test]
async fn real_sources_jobs_reductions_revocations_forks_and_recovery_share_ledger() {
    let directory = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    let server = McpServer::new(actor.clone()).with_context_scope(scope());
    for (id, ordinal, text) in [
        (
            "first",
            1,
            "The earlier message discussed a stable context boundary and immutable originals.",
        ),
        ("tail", 2, "Keep the current user request verbatim."),
    ] {
        let receipt=context_operation(&server,"session",json!({"operation":"source","request":{"version":1,"scope":scope(),"session_id":"session","conversation":"session","message":source(id,ordinal,text),"original_bytes":text.as_bytes()}})).await;
        assert!(receipt.ok, "{receipt:?}");
    }
    let initial = activate_context(&server, "session", "session").await;
    assert!(initial.ok, "{initial:?}");
    let native_history = hm_serve::context_history::replay(&actor, &scope(), "session", "session")
        .await
        .unwrap();
    let first = native_history.history.message("first").unwrap().clone();
    let span = native_history.history.source_span("first").unwrap();
    let chunks = hm_context::historian::select_chunks_with_spans(
        &[first],
        &[span.clone()],
        hm_context::historian::ChunkLimits {
            max_messages: 8,
            max_bytes: 8192,
        },
    )
    .unwrap();
    let chunk = &chunks[0];
    let queued=job(&server,"queue",json!({"action":"historian_enqueue","session_id":"session","cursor":native_history.history.cursor(),"policy_revision":1,"chunk":chunk,"now_ms":0,"reservation":128})).await;
    assert!(queued.ok, "{queued:?}");
    let claimed = job(
        &server,
        "claim",
        json!({"action":"historian_claim","worker":"worker","now_ms":0,"lease_ms":60_000}),
    )
    .await;
    assert!(claimed.ok, "{claimed:?}");
    let claim = claimed.items[0]["result"].clone();
    assert!(claim.is_object(), "{claimed:?}");
    let tiers: Vec<_> = [
        "Earlier message: stable context and immutable originals.",
        "Stable context; immutable originals.",
        "Immutable originals.",
        "Originals.",
    ]
    .into_iter()
    .map(|text| json!({"text":text,"coverage":[span.clone()]}))
    .collect();
    let completed=job(&server,"complete",json!({"action":"historian_complete","claim":claim,"result":{"source_digest":chunk.digest,"tiers":tiers},"now_ms":0,"usage":{"Known":12}})).await;
    assert!(completed.ok, "{completed:?}");
    let reduced = activate_context(&server, "session", "session").await;
    assert!(reduced.ok, "{reduced:?}");
    assert!(
        reduced.items[0]["report"]["blocks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|b| b["id"].as_str().is_some_and(|s| s.starts_with("summary:")))
    );
    assert!(
        !reduced.items[0]["report"]["included"]
            .as_array()
            .unwrap()
            .contains(&json!("first"))
    );
    let recovered = server
        .inspect_envelope(InspectInput {
            uri: Some("hm://1/context/session/source/first".into()),
            ..Default::default()
        })
        .await;
    assert!(recovered.ok, "{recovered:?}");
    assert_eq!(
        recovered.items[0]["original_bytes"],
        json!(
            b"The earlier message discussed a stable context boundary and immutable originals."
                .to_vec()
        )
    );
    let cancelled = job(
        &server,
        "cancel",
        json!({"action":"historian_cancel","id":claim["job"]["id"]}),
    )
    .await;
    assert!(cancelled.ok, "{cancelled:?}");
    let restored = activate_context(&server, "session", "session").await;
    assert!(restored.ok, "{restored:?}");
    assert!(
        restored.items[0]["report"]["included"]
            .as_array()
            .unwrap()
            .contains(&json!("first"))
    );
    assert!(
        !restored.items[0]["report"]["blocks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|b| b["id"].as_str().is_some_and(|s| s.starts_with("summary:")))
    );
    let created=job(&server,"note",json!({"action":"notes","command":{"Create":{"id":"live-note","kind":"Anchor","revision":1,"text":"Current required note","parents":[],"contradictions":[],"expires_at_ns":null,"predicate":"True","tombstoned":false}}})).await;
    assert!(created.ok, "{created:?}");
    let with_note = activate_context(&server, "session", "session").await;
    assert!(with_note.ok, "{with_note:?}");
    assert!(
        with_note.items[0]["report"]["blocks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|b| b["text"] == "Current required note")
    );
    let deleted = job(
        &server,
        "delete-note",
        json!({"action":"notes","command":{"Tombstone":{"id":"live-note","expected_revision":1}}}),
    )
    .await;
    assert!(deleted.ok, "{deleted:?}");
    let without_note = activate_context(&server, "session", "session").await;
    assert!(without_note.ok, "{without_note:?}");
    assert!(
        !without_note.items[0]["report"]["blocks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|b| b["text"] == "Current required note")
    );
    let child=context_operation(&server,"child-conversation",json!({"operation":"fork","request":{"version":1,"scope":scope(),"parent_session_id":"session","parent_conversation":"session","child_session_id":"child","child_conversation":"child-conversation"}})).await;
    assert!(child.ok, "{child:?}");
    let child_state = activate_context(&server, "child", "child-conversation").await;
    assert!(child_state.ok, "{child_state:?}");
    assert_eq!(
        child_state.items[0]["history"]["parent"]["session_id"],
        "session"
    );
    let correction=context_operation(&server,"session",json!({"operation":"source","request":{"version":1,"scope":scope(),"session_id":"session","conversation":"session","message":source("corrected",3,"Corrected earlier message"),"original_bytes":b"Corrected earlier message".to_vec()}})).await;
    assert!(correction.ok, "{correction:?}");
    let edit=context_operation(&server,"session",json!({"operation":"relation","request":{"version":1,"scope":scope(),"session_id":"session","conversation":"session","relation":{"kind":"edit","id":"edit-first","original_id":"first","replacement_id":"corrected"}}})).await;
    assert!(edit.ok, "{edit:?}");
    let edited = activate_context(&server, "session", "session").await;
    assert!(edited.ok, "{edited:?}");
    assert!(
        !edited.items[0]["report"]["included"]
            .as_array()
            .unwrap()
            .contains(&json!("first"))
    );
    let child_history = server
        .inspect_envelope(InspectInput {
            uri: Some("hm://1/context/child/history".into()),
            ..Default::default()
        })
        .await;
    assert!(child_history.ok, "{child_history:?}");
    assert_eq!(
        child_history.items[0]["messages"].as_array().unwrap().len(),
        2
    );
    let digest = edited.items[0]["report"]["digest"].clone();
    let generation = edited.items[0]["cache"]["generation"].clone();
    drop(server);
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    let server = McpServer::new(actor).with_context_scope(scope());
    let replay = server
        .inspect_envelope(InspectInput {
            uri: Some("hm://1/context/session".into()),
            ..Default::default()
        })
        .await;
    assert!(replay.ok, "{replay:?}");
    assert_eq!(replay.items[0]["report"]["digest"], digest);
    assert_eq!(replay.items[0]["cache"]["generation"], generation);
    assert_eq!(replay.items[0]["cache"]["replayed"], true);
}

#[tokio::test]
async fn native_memory_and_registered_retrieval_affect_context_and_recovery() {
    use hm_serve::context_memory::{MemoryRecord, MemorySource, Provenance, RecordKind};
    let directory = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    let server = McpServer::new(actor.clone()).with_context_scope(scope());
    let text = b"Original note evidence".to_vec();
    let digest = hm_context::digest_bytes(&text);
    let source = MemorySource {
        id: "note-origin".into(),
        digest: digest.clone(),
        content: text.clone(),
        locator: "local:note-origin".into(),
        occurred_at_ns: None,
        recorded_at_ns: 200,
        tombstoned: false,
    };
    let admitted=context_operation(&server,"session",json!({"operation":"memory","request":{"version":1,"scope":scope(),"request_id":"source","command":{"kind":"source","source":source}}})).await;
    assert!(admitted.ok, "{admitted:?}");
    let mut record = MemoryRecord::new(
        "live-memory",
        RecordKind::ConditionalNote,
        "Current scoped note",
        200,
    );
    record.predicate = Some(hm_context::notes::Predicate::Equals {
        key: "project_id".into(),
        value: "project".into(),
    });
    record.provenance.push(Provenance {
        source_id: "note-origin".into(),
        source_digest: digest.clone(),
        span_start: 0,
        span_end: text.len() as u64,
        quoted_digest: digest,
    });
    let created=context_operation(&server,"session",json!({"operation":"memory","request":{"version":1,"scope":scope(),"request_id":"create","command":{"kind":"create","record":record}}})).await;
    assert!(created.ok, "{created:?}");
    let with_note = activate_context(&server, "session", "session").await;
    assert!(with_note.ok, "{with_note:?}");
    assert!(
        with_note.items[0]["report"]["blocks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|b| b["text"] == "Current scoped note")
    );
    assert!(with_note.items[0]["coverage"]["utc_offset_seconds"].is_null());
    let recovered = server
        .inspect_envelope(InspectInput {
            uri: Some("hm://1/context-memory/live-memory/source/note-origin".into()),
            ..Default::default()
        })
        .await;
    assert!(recovered.ok, "{recovered:?}");
    assert_eq!(recovered.items[0]["source"]["content"], json!(text));
    record.revision = 2;
    record.content = "Revised scoped note".into();
    record.revision_digest.clear();
    let revised=context_operation(&server,"session",json!({"operation":"memory","request":{"version":1,"scope":scope(),"request_id":"revise","command":{"kind":"revise","record":record,"expected_revision":1}}})).await;
    assert!(revised.ok, "{revised:?}");
    let with_revision = activate_context(&server, "session", "session").await;
    assert!(with_revision.ok, "{with_revision:?}");
    assert!(
        with_revision.items[0]["report"]["blocks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|b| b["text"] == "Revised scoped note")
    );
    assert!(
        with_revision.items[0]["cache"]["generation"].as_u64()
            > with_note.items[0]["cache"]["generation"].as_u64()
    );
    let invalid=context_operation(&server,"session",json!({"operation":"memory","request":{"version":1,"scope":scope(),"request_id":"stale","command":{"kind":"revise","record":record,"expected_revision":1}}})).await;
    assert!(!invalid.ok);
    assert_eq!(invalid.items[0]["error"], "kSequenceViolation");
    let removed=context_operation(&server,"session",json!({"operation":"memory","request":{"version":1,"scope":scope(),"request_id":"source-gone","command":{"kind":"tombstone_source","id":"note-origin"}}})).await;
    assert!(removed.ok, "{removed:?}");
    let without_note = activate_context(&server, "session", "session").await;
    assert!(without_note.ok, "{without_note:?}");
    assert!(
        !without_note.items[0]["report"]["blocks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|b| b["text"] == "Revised scoped note")
    );
    let hidden = server
        .inspect_envelope(InspectInput {
            uri: Some("hm://1/context-memory/live-memory/source/note-origin".into()),
            ..Default::default()
        })
        .await;
    assert!(!hidden.ok);
    let document = "quartz lexical registered document";
    let document_digest = hm_context::digest_bytes(document.as_bytes());
    let registered=context_operation(&server,"session",json!({"operation":"retrieval_source","expected_revision":0,"source":{"scope":scope(),"kind":"document","id":"document-1","revision":1,"text":document,"content_digest":document_digest,"authority":"external_observed","provenance":[{"source_id":"document-1","source_digest":document_digest,"byte_start":0,"byte_end":document.len()}],"occurred_at_ns":null,"recorded_at_ns":"200","expires_at_ns":null,"tombstoned":false}})).await;
    assert!(registered.ok, "{registered:?}");
    let query = json!({"conversation":"session","query":"quartz","budget_tokens":4096,"context":{"version":1,"scope":scope(),"session_id":"session","utc_offset_seconds":3600,"budget":{"context_tokens":4096,"reserved_output_tokens":256,"required_tokens":0}}});
    let evidence = server
        .activate_envelope(serde_json::from_value(query.clone()).unwrap())
        .await;
    assert!(evidence.ok, "{evidence:?}");
    assert!(
        evidence.items[0]["report"]["included"]
            .as_array()
            .unwrap()
            .contains(&json!("document-1"))
    );
    assert_eq!(evidence.items[0]["coverage"]["utc_offset_seconds"], 3600);
    assert!(evidence.gaps.iter().any(|g| {
        g.as_str()
            .is_some_and(|s| s.contains("Semantic retrieval unavailable"))
    }));
    let tombstoned=context_operation(&server,"session",json!({"operation":"retrieval_tombstone","kind":"document","id":"document-1","expected_revision":1})).await;
    assert!(tombstoned.ok, "{tombstoned:?}");
    let final_state = server
        .activate_envelope(serde_json::from_value(query).unwrap())
        .await;
    assert!(final_state.ok, "{final_state:?}");
    assert!(
        !final_state.items[0]["report"]["included"]
            .as_array()
            .unwrap()
            .contains(&json!("document-1"))
    );
    let off = context_operation(
        &server,
        "session",
        json!({"operation":"embedding","enabled":false,"expected_revision":0}),
    )
    .await;
    assert!(off.ok, "{off:?}");
    let backfill = context_operation(
        &server,
        "session",
        json!({"operation":"backfill","maximum_items":1,"maximum_bytes":4096}),
    )
    .await;
    assert!(backfill.ok, "{backfill:?}");
    assert!(backfill.items[0]["unavailable"].is_string());
    assert_eq!(
        backfill.items[0]["usage"],
        "unknown: embedding provider does not report accounting"
    );
    drop(server);
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    let server = McpServer::new(actor).with_context_scope(scope());
    let state = server
        .inspect_envelope(InspectInput {
            uri: Some("hm://1/context-memory".into()),
            ..Default::default()
        })
        .await;
    assert!(state.ok, "{state:?}");
    assert!(state.items[0]["records"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn native_memory_jsonl_export_reopens_restores_and_refuses_invalid_artifacts() {
    use hm_serve::context_memory::{
        MemoryExport, MemoryRecord, MemorySource, Provenance, RecordKind,
    };
    let directory = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    let server = McpServer::new(actor.clone()).with_context_scope(scope());
    let original = b"Portable original evidence\nwith exact bytes".to_vec();
    let digest = hm_context::digest_bytes(&original);
    let source = MemorySource {
        id: "portable-source".into(),
        digest: digest.clone(),
        content: original.clone(),
        locator: "local:portable-source".into(),
        occurred_at_ns: None,
        recorded_at_ns: 200,
        tombstoned: false,
    };
    let admitted = context_operation(&server, "portable", json!({"operation":"memory","request":{"version":1,"scope":scope(),"request_id":"source","command":{"kind":"source","source":source}}})).await;
    assert!(admitted.ok, "{admitted:?}");
    let mut record = MemoryRecord::new(
        "portable-note",
        RecordKind::Note,
        "Portable native note",
        200,
    );
    record.metadata = serde_json::from_str(
        r#"{"large_integer":18446744073709551615,"decimal":1.25,"nullable":null}"#,
    )
    .unwrap();
    record.provenance.push(Provenance {
        source_id: "portable-source".into(),
        source_digest: digest.clone(),
        span_start: 0,
        span_end: original.len() as u64,
        quoted_digest: digest,
    });
    let created = context_operation(&server, "portable", json!({"operation":"memory","request":{"version":1,"scope":scope(),"request_id":"create","command":{"kind":"create","record":record}}})).await;
    assert!(created.ok, "{created:?}");
    let context = activate_context(&server, "portable", "portable").await;
    assert!(context.ok, "{context:?}");
    let tail = actor.stats().await.unwrap().applied.last_lsn;
    let exported = server
        .inspect_envelope(InspectInput {
            uri: Some("hm://1/context-memory-export".into()),
            ..Default::default()
        })
        .await;
    assert!(exported.ok, "{exported:?}");
    assert_eq!(actor.stats().await.unwrap().applied.last_lsn, tail);
    let metadata = &exported.items[0];
    let bytes: Vec<u8> = serde_json::from_value(metadata["bytes"].clone()).unwrap();
    let export = MemoryExport::from_jsonl(&bytes).unwrap();
    assert_eq!(metadata["scope"], json!(scope()));
    assert_eq!(metadata["media_type"], "application/x-ndjson");
    assert_eq!(metadata["restore_max_bytes"], 512 * 1024);
    assert_eq!(metadata["byte_count"], bytes.len());
    assert_eq!(
        metadata["artifact_digest"],
        hm_context::digest_bytes(&bytes)
    );
    assert_eq!(metadata["export_digest"], export.digest);
    assert_eq!(metadata["cursor"], export.cursor);
    assert_eq!(export.events.len(), 2);
    assert_eq!(export.events[0]["command"]["kind"], "source");
    assert_eq!(export.events[1]["command"]["kind"], "create");
    drop(server);
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    let server = McpServer::new(actor.clone()).with_context_scope(scope());
    let reopened = server
        .inspect_envelope(InspectInput {
            uri: Some("hm://1/context-memory-export".into()),
            ..Default::default()
        })
        .await;
    assert!(reopened.ok, "{reopened:?}");
    assert_eq!(reopened.items[0], *metadata);
    drop(server);
    actor.shutdown().await.unwrap();

    let target_directory = tempfile::tempdir().unwrap();
    let target = ActorEngine::open(config(target_directory.path()))
        .await
        .unwrap();
    let server = McpServer::new(target.clone()).with_context_scope(scope());
    let restored = context_operation(&server, "portable", json!({"operation":"memory","request":{"version":1,"scope":scope(),"request_id":"restore","command":{"kind":"restore_jsonl","jsonl":bytes,"artifact_digest":hm_context::digest_bytes(&bytes)}}})).await;
    assert!(restored.ok, "{restored:?}");
    let recovered = server
        .inspect_envelope(InspectInput {
            uri: Some("hm://1/context-memory/portable-note/source/portable-source".into()),
            ..Default::default()
        })
        .await;
    assert!(recovered.ok, "{recovered:?}");
    assert_eq!(recovered.items[0]["source"]["content"], json!(original));
    let read = server
        .inspect_envelope(InspectInput {
            uri: Some("hm://1/context-memory/portable-note".into()),
            ..Default::default()
        })
        .await;
    assert!(read.ok, "{read:?}");
    assert_eq!(read.items[0]["record"]["content"], record.content);
    assert_eq!(
        read.items[0]["record"]["authority"],
        json!(record.authority)
    );
    assert_eq!(
        read.items[0]["record"]["provenance"],
        json!(record.provenance)
    );
    let tail = target.stats().await.unwrap().applied.last_lsn;
    let mut corrupt = bytes.clone();
    corrupt[0] ^= 1;
    let refused = context_operation(&server, "portable", json!({"operation":"memory","request":{"version":1,"scope":scope(),"request_id":"corrupt","command":{"kind":"restore_jsonl","jsonl":corrupt,"artifact_digest":hm_context::digest_bytes(&bytes)}}})).await;
    assert!(!refused.ok);
    let mut foreign = export.clone();
    foreign.scope.project_id = "foreign".into();
    foreign.digest.clear();
    foreign.digest = hm_context::digest_bytes(&serde_json::to_vec(&foreign).unwrap());
    foreign.validate().unwrap();
    let foreign_bytes = foreign.to_jsonl().unwrap();
    let refused = context_operation(&server, "portable", json!({"operation":"memory","request":{"version":1,"scope":scope(),"request_id":"foreign","command":{"kind":"restore_jsonl","jsonl":foreign_bytes,"artifact_digest":hm_context::digest_bytes(&foreign_bytes)}}})).await;
    assert!(!refused.ok);
    assert_eq!(target.stats().await.unwrap().applied.last_lsn, tail);
    let disabled = McpServer::new(target.clone());
    let refused = disabled
        .inspect_envelope(InspectInput {
            uri: Some("hm://1/context-memory-export".into()),
            ..Default::default()
        })
        .await;
    assert!(!refused.ok);
    drop(disabled);
    drop(server);
    target.shutdown().await.unwrap();
    let target = ActorEngine::open(config(target_directory.path()))
        .await
        .unwrap();
    let server = McpServer::new(target.clone()).with_context_scope(scope());
    let recovered = server
        .inspect_envelope(InspectInput {
            uri: Some("hm://1/context-memory/portable-note/source/portable-source".into()),
            ..Default::default()
        })
        .await;
    assert!(recovered.ok, "{recovered:?}");
    assert_eq!(recovered.items[0]["source"]["content"], json!(original));
    drop(server);
    target.shutdown().await.unwrap();
}
