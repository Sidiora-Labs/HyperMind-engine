use hm_context::{Authority, Scope, digest_bytes, temporal::IdentityRegistry};
use hm_core::ActorId;
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_memory::{
        self, MemoryCommand, MemoryGrant, MemoryRecord, MemoryRequest, MemorySource, Provenance,
        RecordKind,
    },
    hypermid_import::{ImportBundle, ImportEntry, import_batch},
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
fn scope() -> Scope {
    Scope {
        owner_id: "owner".into(),
        project_id: "project".into(),
        workspace_id: None,
    }
}
fn config(p: &std::path::Path, actor: u16) -> ActorConfig {
    ActorConfig {
        actor_directory: p.to_owned(),
        actor: ActorId::new(actor),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}
async fn command(
    actor: &ActorEngine,
    id: &str,
    command: MemoryCommand,
) -> Result<context_memory::MemoryReceipt, context_memory::MemoryError> {
    context_memory::execute(
        actor,
        &scope(),
        &scope(),
        MemoryRequest {
            version: 1,
            scope: scope(),
            request_id: id.into(),
            command,
        },
    )
    .await
}
#[tokio::test]
async fn native_mutations_are_atomic_revision_fenced_and_restore_from_ledger() {
    let dir = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(&dir.path().join("a"), 7))
        .await
        .unwrap();
    let bytes = b"original chronological evidence".to_vec();
    let digest = digest_bytes(&bytes);
    command(
        &actor,
        "source",
        MemoryCommand::Source {
            source: MemorySource {
                id: "source".into(),
                digest: digest.clone(),
                content: bytes.clone(),
                locator: "file://evidence".into(),
                occurred_at_ns: Some(10),
                recorded_at_ns: 20,
                tombstoned: false,
            },
        },
    )
    .await
    .unwrap();
    let mut note = MemoryRecord::new("note", RecordKind::Note, "conclusion", 30);
    note.authority = Authority::DerivedInference;
    note.provenance = vec![Provenance {
        source_id: "source".into(),
        source_digest: digest.clone(),
        span_start: 0,
        span_end: bytes.len() as u64,
        quoted_digest: digest,
    }];
    let first = command(
        &actor,
        "create",
        MemoryCommand::Create {
            record: note.clone(),
        },
    )
    .await
    .unwrap();
    assert!(
        command(
            &actor,
            "create",
            MemoryCommand::Create {
                record: note.clone()
            }
        )
        .await
        .unwrap()
        .replayed
    );
    let foreign = Scope {
        owner_id: "reader".into(),
        project_id: "other".into(),
        workspace_id: None,
    };
    let grant = MemoryGrant {
        principal_digest: None,
        id: "read".into(),
        principal: foreign.clone(),
        record_ids: BTreeSet::from(["note".into()]),
        categories: BTreeSet::new(),
        read: true,
        expires_at_ns: None,
        revoked: false,
        revision: 1,
        record_revisions: BTreeMap::new(),
    };
    command(&actor, "grant", MemoryCommand::SetGrant { grant })
        .await
        .unwrap();
    assert_eq!(
        context_memory::rebuild(&actor, &scope())
            .await
            .unwrap()
            .read(&foreign, "note", 100)
            .unwrap()
            .unwrap()
            .content,
        "conclusion"
    );
    note.content = "corrected conclusion".into();
    note.revision = 2;
    note.revision_digest.clear();
    let other = note.clone();
    let (a, b) = tokio::join!(
        command(
            &actor,
            "revision-a",
            MemoryCommand::Revise {
                record: note,
                expected_revision: 1
            }
        ),
        command(
            &actor,
            "revision-b",
            MemoryCommand::Revise {
                record: other,
                expected_revision: 1
            }
        )
    );
    assert!(a.is_ok() ^ b.is_ok());
    let state = context_memory::rebuild(&actor, &scope()).await.unwrap();
    assert_eq!(state.revisions.len(), 2);
    assert!(state.read(&foreign, "note", 100).is_err());
    assert_eq!(state.sources["source"].content, bytes);
    command(
        &actor,
        "delete",
        MemoryCommand::Tombstone {
            id: "note".into(),
            expected_revision: 2,
        },
    )
    .await
    .unwrap();
    assert!(
        context_memory::rebuild(&actor, &scope())
            .await
            .unwrap()
            .visible_records(&scope(), 100)
            .unwrap()
            .is_empty()
    );
    let exported = context_memory::export(&actor, &scope(), &scope())
        .await
        .unwrap();
    let count = actor.stats().await.unwrap().log_events;
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(&dir.path().join("a"), 7))
        .await
        .unwrap();
    assert_eq!(actor.stats().await.unwrap().log_events, count);
    assert!(
        context_memory::rebuild(&actor, &scope())
            .await
            .unwrap()
            .read(&scope(), "note", 100)
            .unwrap()
            .is_none()
    );
    assert!(first.last_lsn > 0);
    let restored = ActorEngine::open(config(&dir.path().join("b"), 8))
        .await
        .unwrap();
    command(
        &restored,
        "restore",
        MemoryCommand::RestoreExport { export: exported },
    )
    .await
    .unwrap();
    let state = context_memory::rebuild(&restored, &scope()).await.unwrap();
    assert!(state.read(&scope(), "note", 100).unwrap().is_none());
    assert_eq!(state.sources["source"].content, bytes);
    actor.shutdown().await.unwrap();
    restored.shutdown().await.unwrap();
}
#[tokio::test]
async fn chronological_anchors_cannot_be_rewritten_and_forged_spans_commit_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(dir.path(), 7)).await.unwrap();
    let mut anchor = MemoryRecord::new("anchor", RecordKind::Anchor, "observation", 20);
    anchor.occurred_at_ns = Some(10);
    command(
        &actor,
        "anchor",
        MemoryCommand::Create {
            record: anchor.clone(),
        },
    )
    .await
    .unwrap();
    anchor.revision = 2;
    anchor.content = "rewritten".into();
    let count = actor.stats().await.unwrap().log_events;
    assert!(
        command(
            &actor,
            "rewrite",
            MemoryCommand::Revise {
                record: anchor,
                expected_revision: 1
            }
        )
        .await
        .is_err()
    );
    let mut n = MemoryRecord::new("forged", RecordKind::Note, "invalid", 20);
    n.provenance.push(Provenance {
        source_id: "missing".into(),
        source_digest: "a".repeat(64),
        span_start: 0,
        span_end: 1,
        quoted_digest: "a".repeat(64),
    });
    assert!(
        command(&actor, "forged", MemoryCommand::Create { record: n })
            .await
            .is_err()
    );
    assert_eq!(count, actor.stats().await.unwrap().log_events);
    actor.shutdown().await.unwrap();
}
fn entry(id: &str, kind: &str, payload: Value) -> ImportEntry {
    ImportEntry {
        source_id: id.into(),
        kind: kind.into(),
        digest: digest_bytes(&serde_json::to_vec(&payload).unwrap()),
        payload,
    }
}
fn bundle() -> ImportBundle {
    let content = "original evidence";
    let d = digest_bytes(content.as_bytes());
    let record = json!({"record_id":"record","kind":"note","category":"architecture","status":"active","current_revision":2,"current_revision_digest":"rev-2","importance":0.8,"confidence":0.9,"observed_from_ms":1,"expires_at_ms":null,"retention_until_ms":100,"created_at_ms":2});
    let mut entries = vec![
        entry("scope", "scope", json!({"scope":scope()})),
        entry("record", "record", record),
        entry(
            "revision:1",
            "revision",
            json!({"record_id":"record","revision":1,"revision_digest":"rev-1","parent_revision_digest":null,"content":"initial conclusion","content_digest":digest_bytes(b"initial conclusion"),"authored_at_ms":2,"immutable_anchor":false}),
        ),
        entry(
            "revision:2",
            "revision",
            json!({"record_id":"record","revision":2,"revision_digest":"rev-2","parent_revision_digest":"rev-1","content":"corrected conclusion","content_digest":digest_bytes(b"corrected conclusion"),"authored_at_ms":3,"immutable_anchor":false}),
        ),
        entry(
            "source",
            "source",
            json!({"source_id":"source","source_digest":d,"captured_content":content,"locator":"file://evidence","observed_at_ms":1,"created_at_ms":2}),
        ),
        entry(
            "provenance",
            "provenance",
            json!({"record_id":"record","revision":2,"source_id":"source","span_start":0,"span_end":content.len(),"quoted_digest":d}),
        ),
        entry(
            "verification",
            "verification",
            json!({"event_id":"verify","record_id":"record","revision_digest":"rev-2","state":"supported","evidence_source_id":"source","confidence":0.95,"created_at_ms":4}),
        ),
        entry(
            "mutation",
            "mutation",
            json!({"event_id":"update","epoch":1,"sequence":2,"operation":"update","record_id":"record","previous_revision_digest":"rev-1","result_revision_digest":"rev-2","actor_scope_digest":"opaque-owner-digest","created_at_ms":3}),
        ),
        entry(
            "summary-detail",
            "summary_detail",
            json!({"record_id":"summary","summary_level":1,"input_set_digest":"original"}),
        ),
        entry(
            "episode-detail",
            "episode_detail",
            json!({"record_id":"episode","observed_from_ms":1,"observed_to_ms":2,"participants":["owner"],"episode_type":"change"}),
        ),
    ];
    entries.push(entry("parent","record",json!({"record_id":"parent","kind":"anchor","category":"evidence","status":"active","current_revision":1,"current_revision_digest":"parent-revision","importance":1.0,"confidence":1.0,"created_at_ms":1,"observed_from_ms":1})));
    entries.push(entry("parent-revision","revision",json!({"record_id":"parent","revision":1,"revision_digest":"parent-revision","content":content,"content_digest":d,"authored_at_ms":1,"immutable_anchor":true})));
    entries.push(entry("lineage","lineage",json!({"child_record_id":"record","child_revision":2,"parent_record_id":"parent","parent_revision_digest":"parent-revision","relation":"derived_from","created_at_ms":3})));
    for (id, kind, content) in [
        ("summary", "summary", "overview"),
        ("episode", "episode", "chronological episode"),
        ("conditional", "smart_note", "deferred note"),
    ] {
        let revision = format!("{id}-revision");
        entries.push(entry(id,"record",json!({"record_id":id,"kind":kind,"category":"general","status":"active","current_revision":1,"current_revision_digest":revision,"importance":0.5,"confidence":0.8,"created_at_ms":1,"observed_from_ms":1})));
        entries.push(entry(&format!("{id}:revision"),"revision",json!({"record_id":id,"revision":1,"revision_digest":revision,"content":content,"content_digest":digest_bytes(content.as_bytes()),"authored_at_ms":1,"immutable_anchor":false})));
    }
    entries.push(entry("smart-detail","smart_note_detail",json!({"record_id":"conditional","predicate":{"operator":"all","clauses":[{"field":"event.kind","comparison":"eq","value":"build"}]},"predicate_digest":"original-predicate-digest","last_result":null})));
    entries.push(entry("grant","grant",json!({"grant_id":"sharing","grantee_scope_digest":"not-an-owner","operations":["read"],"categories":["architecture"],"expires_at_ms":10,"revoked_at_ms":null,"revision":1})));
    let mut b = ImportBundle {
        version: 1,
        import_id: "export".into(),
        scope: scope(),
        entries,
        digest: String::new(),
    };
    b.digest = b.computed_digest().unwrap();
    b
}
#[tokio::test]
async fn migration_stages_all_rows_then_publishes_native_revisions_idempotently() {
    let dir = tempfile::tempdir().unwrap();
    let mut registry = IdentityRegistry::new();
    registry.bind(scope(), 7, vec![]).unwrap();
    let bundle = bundle();
    let actor = ActorEngine::open(config(dir.path(), 7)).await.unwrap();
    let partial = import_batch(&actor, &registry, &bundle, 3).await.unwrap();
    assert!(!partial.complete);
    assert!(
        context_memory::rebuild(&actor, &scope())
            .await
            .unwrap()
            .records
            .is_empty()
    );
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(dir.path(), 7)).await.unwrap();
    let done = import_batch(&actor, &registry, &bundle, 100).await.unwrap();
    assert!(done.complete);
    let state = context_memory::rebuild(&actor, &scope()).await.unwrap();
    assert_eq!(state.records["record"].content, "corrected conclusion");
    assert_eq!(state.records["record"].provenance[0].source_id, "source");
    assert_eq!(
        state.records["record"].lineage[0].parent_record_id,
        "parent"
    );
    assert_eq!(
        state.revisions[&("record".into(), 1)].content,
        "initial conclusion"
    );
    assert_eq!(state.origins.len(), bundle.entries.len());
    assert_eq!(state.verifications.len(), 1);
    assert_eq!(state.audits.len(), 1);
    assert_eq!(state.grants.len(), 1);
    assert!(
        state.records["conditional"]
            .smart_condition
            .as_ref()
            .unwrap()
            .evaluate(&BTreeMap::from([("event.kind".into(), "build".into())]))
            .unwrap()
    );
    assert_eq!(state.records["episode"].occurred_at_ns, Some(1_000_000));
    let count = actor.stats().await.unwrap().log_events;
    assert_eq!(
        done,
        import_batch(&actor, &registry, &bundle, 100).await.unwrap()
    );
    assert_eq!(count, actor.stats().await.unwrap().log_events);
    command(
        &actor,
        "imported-delete",
        MemoryCommand::Tombstone {
            id: "record".into(),
            expected_revision: 2,
        },
    )
    .await
    .unwrap();
    assert!(
        context_memory::rebuild(&actor, &scope())
            .await
            .unwrap()
            .read(&scope(), "record", 10_000_000)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        done,
        import_batch(&actor, &registry, &bundle, 100).await.unwrap()
    );
    assert!(
        context_memory::rebuild(&actor, &scope())
            .await
            .unwrap()
            .read(&scope(), "record", 10_000_000)
            .unwrap()
            .is_none()
    );
    actor.shutdown().await.unwrap();
}
