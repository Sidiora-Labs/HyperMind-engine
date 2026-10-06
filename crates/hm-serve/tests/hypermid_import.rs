use hm_context::{
    temporal::IdentityRegistry,
    types::{Scope, digest_bytes},
};
use hm_core::ActorId;
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    hypermid_import::{ImportBundle, ImportEntry, import_batch},
};
fn config(p: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: p.to_owned(),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}
#[tokio::test]
async fn durable_resume_preserves_sources_and_rejects_corruption_and_scope() {
    let dir = tempfile::tempdir().unwrap();
    let scope = Scope {
        owner_id: "owner".into(),
        project_id: "project".into(),
        workspace_id: None,
    };
    let mut registry = IdentityRegistry::new();
    registry.bind(scope.clone(), 7, vec![]).unwrap();
    let rows = [
        ("record", "record", serde_json::json!({"record_id":"record","kind":"note","category":"general","status":"active","current_revision":2,"current_revision_digest":"revision-two","importance":0.8,"confidence":0.9,"created_at_ms":1,"observed_from_ms":1})),
        ("revision:1", "revision", serde_json::json!({"record_id":"record","revision":1,"revision_digest":"revision-one","content":"retained evidence one","content_digest":digest_bytes(b"retained evidence one"),"authored_at_ms":1,"immutable_anchor":false})),
        ("revision:2", "revision", serde_json::json!({"record_id":"record","revision":2,"revision_digest":"revision-two","parent_revision_digest":"revision-one","content":"retained evidence two","content_digest":digest_bytes(b"retained evidence two"),"authored_at_ms":2,"immutable_anchor":false})),
    ];
    let entries = rows.into_iter().map(|(source_id,kind,payload)| ImportEntry { source_id:source_id.into(),kind:kind.into(),digest:digest_bytes(&serde_json::to_vec(&payload).unwrap()),payload }).collect();
    let mut bundle = ImportBundle {
        version: 1,
        import_id: "export-1".into(),
        scope,
        entries,
        context_sources: vec![],
        digest: String::new(),
    };
    bundle.digest = bundle.computed_digest().unwrap();
    let engine = ActorEngine::open(config(dir.path())).await.unwrap();
    let first = import_batch(&engine, &registry, &bundle, 1).await.unwrap();
    assert_eq!(first.accepted, 1);
    assert!(!first.complete);
    engine.shutdown().await.unwrap();
    let engine = ActorEngine::open(config(dir.path())).await.unwrap();
    let done = import_batch(&engine, &registry, &bundle, 10).await.unwrap();
    assert!(done.complete);
    assert_eq!(done.accepted, 3);
    assert_eq!(
        hm_serve::hypermid_import::list_import_receipts(&engine, &bundle.scope, 10)
            .await
            .unwrap(),
        vec![done.clone()]
    );
    let count = engine.stats().await.unwrap().log_events;
    assert_eq!(
        done,
        import_batch(&engine, &registry, &bundle, 10).await.unwrap()
    );
    assert_eq!(count, engine.stats().await.unwrap().log_events);
    let native = hm_serve::context_memory::rebuild(&engine,&bundle.scope).await.unwrap();
    assert_eq!(native.read(&bundle.scope,"record",0).unwrap().unwrap().content,"retained evidence two");
    let mut corrupt = bundle.clone();
    corrupt.entries[1].payload["content"] = serde_json::json!("changed");
    assert!(import_batch(&engine, &registry, &corrupt, 1).await.is_err());
    corrupt.entries[1].digest =
        digest_bytes(&serde_json::to_vec(&corrupt.entries[1].payload).unwrap());
    corrupt.digest = corrupt.computed_digest().unwrap();
    assert!(import_batch(&engine, &registry, &corrupt, 1).await.is_err());
    assert!(
        import_batch(&engine, &IdentityRegistry::new(), &bundle, 1)
            .await
            .is_err()
    );
    engine.shutdown().await.unwrap();
}

#[test]
fn context_export_verifies_binding_and_payload_digest() {
    use hm_serve::hypermid_import::decode_context_export;
    let binding = serde_json::json!({"scope":{"owner_id":"o","project_id":"p"},"session_id":"session","cursor":{"epoch":1,"sequence":1}});
    let payload = serde_json::json!({"scope":binding["scope"],"session_id":"session","cursor":{"epoch":1,"sequence":1},"item_id":"item","source_event_id":"event","source_digest":"a".repeat(64)});
    let entries = serde_json::json!([{"entry_id":"entry","kind":"source_reference","content_digest":digest_bytes(&serde_json::to_vec(&payload).unwrap()),"byte_length":serde_json::to_vec(&payload).unwrap().len()}]);
    let mut export = serde_json::json!({"binding":binding,"manifest":{"manifest_id":"export","schema_version":1,"scope":binding["scope"],"session_id":"session","cursor":binding["cursor"],"entries":entries,"entries_digest":digest_bytes(&serde_json::to_vec(&entries).unwrap()),"session_binding_digest":digest_bytes(&serde_json::to_vec(&binding).unwrap())},"records":[{"entry_id":"entry","kind":"source_reference","payload":payload}]});
    let bundle = decode_context_export(&serde_json::to_vec(&export).unwrap()).unwrap();
    assert_eq!(bundle.entries[0].payload["source_event_id"], "event");
    export["records"][0]["payload"]["source_event_id"] = serde_json::json!("other");
    assert!(decode_context_export(&serde_json::to_vec(&export).unwrap()).is_err());
}

#[test]
fn memory_jsonl_preserves_revision_and_refuses_changed_digest() {
    use hm_serve::hypermid_import::decode_memory_export;
    let payload = serde_json::json!({"record_id":"record-1","revision":2,"content":"knowledge","revision_digest":"b".repeat(64),"parent_revision_digest":"a".repeat(64)});
    let entry = serde_json::json!({"item_key":"revision:record-1:00000000000000000002","kind":"revision","payload":payload,"item_digest":digest_bytes(&serde_json::to_vec(&payload).unwrap())});
    let encoded = serde_json::to_vec(&entry).unwrap();
    let mut stream = b"hypermid.memory.export.v1\0".to_vec();
    stream.extend_from_slice(&(encoded.len() as u64).to_be_bytes());
    stream.extend_from_slice(&encoded);
    let manifest = serde_json::json!({"export_id":"knowledge-export","schema_version":2,"scope":{"owner_id":"owner","project_id":"project"},"record_count":0,"item_count":1,"stream_digest":digest_bytes(&stream)});
    let jsonl = format!(
        "{}\n{}\n",
        serde_json::json!({"kind":"manifest","manifest":manifest}),
        serde_json::json!({"kind":"entry","entry":entry})
    );
    let bundle = decode_memory_export(jsonl.as_bytes()).unwrap();
    assert_eq!(bundle.entries[0].payload["revision"], 2);
    let mut corrupt = serde_json::json!({"manifest":manifest,"entries":[entry]});
    corrupt["entries"][0]["payload"]["content"] = serde_json::json!("different");
    assert!(decode_memory_export(&serde_json::to_vec(&corrupt).unwrap()).is_err());
}
