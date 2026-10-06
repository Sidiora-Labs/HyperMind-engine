use hm_context::{Scope, digest_bytes, temporal::IdentityRegistry};
use hm_core::{ActorId, LSN};
use hm_ledger::{
    frame,
    segment::{SegmentLog, SegmentLogOptions},
};
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_memory::{
        self, MemoryCommand, MemoryRecord, MemoryRequest, MemorySource, Provenance, RecordKind,
    },
    hypermid_import::{ImportBundle, ImportEntry, import_batch},
    retention::{self, RetentionPolicy},
};
use std::{fs, path::Path};
fn scope() -> Scope {
    Scope {
        owner_id: "source-owner".into(),
        project_id: "project".into(),
        workspace_id: None,
    }
}
fn config(root: &Path) -> ActorConfig {
    ActorConfig {
        actor_directory: root.to_owned(),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}
fn source(id: &str, text: &str) -> MemorySource {
    MemorySource {
        id: id.into(),
        digest: digest_bytes(text.as_bytes()),
        content: text.as_bytes().to_vec(),
        locator: format!("instrument://{id}"),
        occurred_at_ns: Some(10),
        recorded_at_ns: 11,
        tombstoned: false,
    }
}
fn record(id: &str, text: &str, source: &MemorySource) -> MemoryRecord {
    let mut r = MemoryRecord::new(id, RecordKind::Note, text, 12);
    r.provenance = vec![Provenance {
        source_id: source.id.clone(),
        source_digest: source.digest.clone(),
        span_start: 0,
        span_end: source.content.len() as u64,
        quoted_digest: source.digest.clone(),
    }];
    r
}
async fn command(
    actor: &ActorEngine,
    id: &str,
    c: MemoryCommand,
) -> Result<context_memory::MemoryReceipt, context_memory::MemoryError> {
    context_memory::execute(
        actor,
        &scope(),
        &scope(),
        MemoryRequest {
            version: 1,
            scope: scope(),
            request_id: id.into(),
            command: c,
        },
    )
    .await
}
async fn write(actor: &ActorEngine, id: &str, c: MemoryCommand) {
    command(actor, id, c).await.unwrap();
}
async fn plan(
    actor: &ActorEngine,
) -> Result<retention::RetentionPlan, context_memory::MemoryError> {
    retention::plan(
        actor,
        &scope(),
        &scope(),
        vec!["target".into()],
        actor.stats().await.unwrap().applied.last_lsn.get(),
        RetentionPolicy {
            revision: 1,
            tombstone_grace_ns: 0,
        },
    )
    .await
}
fn files(root: &Path) -> Vec<Vec<u8>> {
    fs::read_dir(root)
        .unwrap()
        .flat_map(|e| {
            let e = e.unwrap();
            if e.file_type().unwrap().is_dir() {
                files(&e.path())
            } else {
                vec![fs::read(e.path()).unwrap()]
            }
        })
        .collect()
}
#[tokio::test]
async fn dedicated_original_sources_and_every_provenance_revision_are_physically_erased() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let actor = ActorEngine::open(cfg.clone()).await.unwrap();
    let original = source(
        "original-reading",
        "raw original instrument observation 18 C\r\n",
    );
    let correction = source(
        "corrected-reading",
        "raw corrected instrument observation 17 C\r\n",
    );
    let keep = source(
        "retained-reading",
        "retained unrelated instrument observation\n",
    );
    for s in [&original, &correction, &keep] {
        write(
            &actor,
            &format!("source-{}", s.id),
            MemoryCommand::Source { source: s.clone() },
        )
        .await;
    }
    let first = record("target", "initial attributed conclusion", &original);
    write(
        &actor,
        "create-target",
        MemoryCommand::Create {
            record: first.clone(),
        },
    )
    .await;
    let retained = record("retained", "retained attributed conclusion", &keep);
    write(
        &actor,
        "create-retained",
        MemoryCommand::Create { record: retained },
    )
    .await;
    let mut revision = record("target", "corrected attributed conclusion", &correction);
    revision.revision = 2;
    write(
        &actor,
        "revise-target",
        MemoryCommand::Revise {
            record: revision,
            expected_revision: 1,
        },
    )
    .await;
    write(
        &actor,
        "withdraw-target",
        MemoryCommand::Tombstone {
            id: "target".into(),
            expected_revision: 2,
        },
    )
    .await;
    let before = actor.stats().await.unwrap().applied.last_lsn;
    assert!(plan(&actor).await.is_err());
    assert_eq!(actor.stats().await.unwrap().applied.last_lsn, before);
    for s in [&original, &correction] {
        write(
            &actor,
            &format!("withdraw-{}", s.id),
            MemoryCommand::TombstoneSource { id: s.id.clone() },
        )
        .await;
    }
    let baseline = context_memory::rebuild(&actor, &scope()).await.unwrap();
    let original_frames =
        SegmentLog::open(dir.path(), ActorId::new(7), SegmentLogOptions::default())
            .unwrap()
            .read_all()
            .unwrap();
    let root = actor.verification_status().await.unwrap().root;
    let plan = plan(&actor).await.unwrap();
    assert_eq!(
        plan.source_ids,
        vec!["corrected-reading", "original-reading"]
    );
    assert_eq!(plan.source_copies.len(), 7);
    assert_eq!(
        plan.source_copies
            .iter()
            .filter(|c| c.kind == "raw_source")
            .count(),
        2
    );
    assert_eq!(
        plan.source_copies
            .iter()
            .filter(|c| c.kind == "record_revision")
            .count(),
        2
    );
    for s in [&original, &correction] {
        assert!(
            plan.source_copies
                .iter()
                .any(|c| c.id == s.id && c.digest == s.digest)
        );
    }
    let raw_copies: Vec<_> = plan
        .source_copies
        .iter()
        .filter(|c| c.kind == "raw_source")
        .collect();
    assert_ne!(raw_copies[0].digest, raw_copies[1].digest);
    for copy in &raw_copies {
        let native = if copy.id == original.id {
            &original
        } else {
            &correction
        };
        assert_eq!(copy.digest, digest_bytes(&native.content));
        assert_eq!(copy.digest_kind, "source_content_sha256");
    }
    let revision_copies: Vec<_> = plan
        .source_copies
        .iter()
        .filter(|c| c.kind == "record_revision")
        .collect();
    assert_ne!(revision_copies[0].digest, revision_copies[1].digest);
    for (copy, revision) in revision_copies.iter().zip([1, 2]) {
        assert_eq!(
            copy.digest,
            baseline.revisions[&("target".into(), revision)].revision_digest
        );
        assert_eq!(copy.digest_kind, "record_revision_sha256");
    }
    let status_copy = plan
        .source_copies
        .iter()
        .find(|c| c.kind == "record_status")
        .unwrap();
    assert_eq!(
        status_copy.digest,
        baseline.revisions[&("target".into(), 3)].revision_digest
    );
    let affected = plan.affected_lsns.clone();
    let approval = retention::approve(&scope(), plan.clone()).unwrap();
    let receipt = retention::execute(&actor, &scope(), approval.clone())
        .await
        .unwrap();
    assert_eq!(receipt.source_ids, plan.source_ids);
    assert_eq!(receipt.source_copies, plan.source_copies);
    assert_eq!(receipt.redacted_frames, 7);
    assert!(
        retention::execute(&actor, &scope(), approval.clone())
            .await
            .unwrap()
            .replayed
    );
    assert!(!dir.path().join("log").exists());
    for bytes in files(dir.path()) {
        for f in original_frames
            .iter()
            .filter(|f| affected.contains(&f.header.lsn.get()))
        {
            assert!(
                !bytes
                    .windows(f.sealed_payload.len())
                    .any(|b| b == f.sealed_payload)
            );
        }
    }
    let redacted = SegmentLog::open(dir.path(), ActorId::new(7), SegmentLogOptions::default())
        .unwrap()
        .read_all()
        .unwrap();
    for (old, new) in original_frames.iter().zip(redacted.iter()) {
        if !affected.contains(&old.header.lsn.get()) {
            assert_eq!(frame::encode(old).unwrap(), frame::encode(new).unwrap());
        }
    }
    let state = context_memory::rebuild(&actor, &scope()).await.unwrap();
    assert!(!state.records.contains_key("target"));
    for s in [&original, &correction] {
        assert!(!state.sources.contains_key(&s.id));
        assert_eq!(state.purged_sources[&s.id], s.digest);
        assert!(
            command(
                &actor,
                &format!("resurrect-{}", s.id),
                MemoryCommand::Source { source: s.clone() }
            )
            .await
            .is_err()
        );
    }
    assert_eq!(state.sources[&keep.id], baseline.sources[&keep.id]);
    assert_eq!(state.records["retained"], baseline.records["retained"]);
    assert_eq!(actor.verification_status().await.unwrap().root, root);
    for f in actor.frames_since(LSN::new(0), None, 100).await.unwrap() {
        for s in [&original, &correction] {
            let raw_array = serde_json::to_vec(&s.content).unwrap();
            assert!(
                !f.sealed_payload
                    .windows(raw_array.len())
                    .any(|b| b == raw_array)
            );
            assert!(
                !f.sealed_payload
                    .windows(s.locator.len())
                    .any(|b| b == s.locator.as_bytes())
            );
        }
    }
    assert!(
        command(
            &actor,
            "forged-skeleton",
            MemoryCommand::PurgedSource {
                id: "retained-reading".into(),
                source_digest: keep.digest.clone(),
                original_request_digest: "forged".into()
            }
        )
        .await
        .is_err()
    );
    let payload = serde_json::json!({"source_id":original.id,"source_digest":original.digest,"captured_content":String::from_utf8(original.content.clone()).unwrap(),"locator":original.locator,"created_at_ms":1,"observed_at_ms":1});
    let entry = ImportEntry {
        source_id: "source:original-reading".into(),
        kind: "source".into(),
        digest: digest_bytes(&serde_json::to_vec(&payload).unwrap()),
        payload,
    };
    let mut bundle = ImportBundle {
        version: 1,
        import_id: "resurrection-import".into(),
        scope: scope(),
        entries: vec![entry],
        digest: String::new(),
        context_sources: vec![],
    };
    bundle.digest = bundle.computed_digest().unwrap();
    let mut registry = IdentityRegistry::new();
    registry.bind(scope(), 7, vec![]).unwrap();
    let tail = actor.stats().await.unwrap().applied.last_lsn;
    assert!(import_batch(&actor, &registry, &bundle, 1).await.is_err());
    assert_eq!(actor.stats().await.unwrap().applied.last_lsn, tail);
    actor.shutdown().await.unwrap();
    let reopened = ActorEngine::open(cfg).await.unwrap();
    let state = context_memory::rebuild(&reopened, &scope()).await.unwrap();
    assert_eq!(state.sources[&keep.id], baseline.sources[&keep.id]);
    assert_eq!(state.records["retained"], baseline.records["retained"]);
    assert_eq!(state.purged_sources.len(), 2);
    assert_eq!(reopened.verification_status().await.unwrap().root, root);
    assert!(
        retention::execute(&reopened, &scope(), approval)
            .await
            .unwrap()
            .replayed
    );
    reopened.shutdown().await.unwrap();
}
#[tokio::test]
async fn unrelated_historical_citation_protects_withdrawn_original_source() {
    let dir = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(dir.path())).await.unwrap();
    let shared = source("shared", "shared original evidence");
    let later = source("later", "replacement evidence");
    for s in [&shared, &later] {
        write(&actor, &s.id, MemoryCommand::Source { source: s.clone() }).await;
    }
    write(
        &actor,
        "target",
        MemoryCommand::Create {
            record: record("target", "target evidence", &shared),
        },
    )
    .await;
    write(
        &actor,
        "other",
        MemoryCommand::Create {
            record: record("other", "historical competing evidence", &shared),
        },
    )
    .await;
    let mut revised = record("other", "later competing evidence", &later);
    revised.revision = 2;
    write(
        &actor,
        "revise-other",
        MemoryCommand::Revise {
            record: revised,
            expected_revision: 1,
        },
    )
    .await;
    write(
        &actor,
        "tombstone",
        MemoryCommand::Tombstone {
            id: "target".into(),
            expected_revision: 1,
        },
    )
    .await;
    write(
        &actor,
        "withdraw-shared",
        MemoryCommand::TombstoneSource {
            id: shared.id.clone(),
        },
    )
    .await;
    let tail = actor.stats().await.unwrap().applied.last_lsn;
    let error = plan(&actor).await.unwrap_err();
    assert!(error.to_string().contains("historical citation"));
    assert_eq!(actor.stats().await.unwrap().applied.last_lsn, tail);
    assert_eq!(
        context_memory::rebuild(&actor, &scope())
            .await
            .unwrap()
            .sources[&shared.id]
            .content,
        shared.content
    );
    actor.shutdown().await.unwrap();
}
