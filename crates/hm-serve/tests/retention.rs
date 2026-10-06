use hm_context::Scope;
use hm_core::{ActorId, LSN};
use hm_ledger::{
    frame,
    segment::{SegmentLog, SegmentLogOptions},
};
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_memory::{self, MemoryCommand, MemoryRecord, MemoryRequest, RecordKind},
    retention::{self, RetentionPolicy},
};
use std::{fs, path::Path};
fn scope() -> Scope {
    Scope {
        owner_id: "retention-owner".into(),
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
async fn write(actor: &ActorEngine, id: &str, command: MemoryCommand) {
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
    .unwrap();
}
fn files(root: &Path) -> Vec<Vec<u8>> {
    let mut out = vec![];
    for e in fs::read_dir(root).unwrap() {
        let e = e.unwrap();
        if e.file_type().unwrap().is_dir() {
            out.extend(files(&e.path()))
        } else {
            out.push(fs::read(e.path()).unwrap())
        }
    }
    out
}
#[tokio::test]
async fn certified_native_purge_removes_managed_bytes_preserves_roots_and_recovers() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(dir.path());
    let actor = ActorEngine::open(cfg.clone()).await.unwrap();
    let first = MemoryRecord::new("erase-me", RecordKind::Note, "secret-observation-first", 1);
    write(
        &actor,
        "first",
        MemoryCommand::Create {
            record: first.clone(),
        },
    )
    .await;
    let keep = MemoryRecord::new(
        "keep-me",
        RecordKind::Anchor,
        "untouched exact anchor evidence",
        2,
    );
    write(
        &actor,
        "keep",
        MemoryCommand::Create {
            record: keep.clone(),
        },
    )
    .await;
    let mut second = first.clone();
    second.revision = 2;
    second.content = "secret-observation-correction".into();
    second.revision_digest = second.computed_revision_digest().unwrap();
    write(
        &actor,
        "revise",
        MemoryCommand::Revise {
            record: second,
            expected_revision: 1,
        },
    )
    .await;
    write(
        &actor,
        "tombstone",
        MemoryCommand::Tombstone {
            id: first.id.clone(),
            expected_revision: 2,
        },
    )
    .await;
    let tail = actor.stats().await.unwrap().applied.last_lsn.get();
    let original = SegmentLog::open(dir.path(), ActorId::new(7), SegmentLogOptions::default())
        .unwrap()
        .read_all()
        .unwrap();
    let unrelated = frame::encode(&original[1]).unwrap();
    let sensitive: Vec<_> = original
        .iter()
        .filter(|f| f.header.lsn.get() != 2)
        .map(|f| f.sealed_payload.clone())
        .collect();
    let policy = RetentionPolicy {
        revision: 1,
        tombstone_grace_ns: 0,
    };
    let plan = retention::plan(
        &actor,
        &scope(),
        &scope(),
        vec![first.id.clone()],
        tail,
        policy.clone(),
    )
    .await
    .unwrap();
    let visitor = Scope {
        owner_id: "visitor".into(),
        ..scope()
    };
    assert!(retention::approve(&visitor, plan.clone()).is_err());
    let mut forged = retention::approve(&scope(), plan.clone()).unwrap();
    forged.plan.record_ids.push("keep-me".into());
    assert!(retention::execute(&actor, &scope(), forged).await.is_err());
    let expired = RetentionPolicy {
        revision: 1,
        tombstone_grace_ns: i64::MAX,
    };
    assert!(
        retention::plan(
            &actor,
            &scope(),
            &scope(),
            vec![first.id.clone()],
            tail,
            expired
        )
        .await
        .is_err()
    );
    write(
        &actor,
        "tail-fence",
        MemoryCommand::Create {
            record: MemoryRecord::new("extra", RecordKind::Fact, "unrelated later evidence", 3),
        },
    )
    .await;
    assert!(
        retention::execute(
            &actor,
            &scope(),
            retention::approve(&scope(), plan).unwrap()
        )
        .await
        .is_err()
    );
    let tail = actor.stats().await.unwrap().applied.last_lsn.get();
    let root = actor.verification_status().await.unwrap().root;
    let plan = retention::plan(
        &actor,
        &scope(),
        &scope(),
        vec![first.id.clone()],
        tail,
        policy,
    )
    .await
    .unwrap();
    let approval = retention::approve(&scope(), plan.clone()).unwrap();
    let keep_before = context_memory::rebuild(&actor, &scope())
        .await
        .unwrap()
        .records["keep-me"]
        .clone();
    let receipt = retention::execute(&actor, &scope(), approval.clone())
        .await
        .unwrap();
    assert!(receipt.managed_cleanup_complete);
    assert_eq!(receipt.redacted_frames, 3);
    assert!(receipt.storage_semantics.contains("no crypto"));
    assert!(
        retention::execute(&actor, &scope(), approval.clone())
            .await
            .unwrap()
            .replayed
    );
    assert!(!dir.path().join("log").exists());
    assert!(!dir.path().join("projections").exists());
    for bytes in files(dir.path()) {
        for secret in &sensitive {
            assert!(!bytes.windows(secret.len()).any(|w| w == secret));
        }
    }
    let stored = SegmentLog::open(dir.path(), ActorId::new(7), SegmentLogOptions::default())
        .unwrap()
        .read_all()
        .unwrap();
    assert_eq!(frame::encode(&stored[1]).unwrap(), unrelated);
    assert_eq!(actor.verification_status().await.unwrap().root, root);
    let native = context_memory::rebuild(&actor, &scope()).await.unwrap();
    assert!(!native.records.contains_key("erase-me"));
    assert_eq!(native.purged_records["erase-me"].len(), 3);
    assert_eq!(native.records["keep-me"], keep_before);
    for frame in actor.frames_since(LSN::new(0), None, 100).await.unwrap() {
        let bytes = frame.sealed_payload;
        assert!(
            !bytes
                .windows(b"secret-observation".len())
                .any(|w| w == b"secret-observation")
        );
    }
    assert!(
        context_memory::export(&actor, &scope(), &scope())
            .await
            .is_err()
    );
    assert!(
        context_memory::execute(
            &actor,
            &scope(),
            &scope(),
            MemoryRequest {
                version: 1,
                scope: scope(),
                request_id: "resurrection".into(),
                command: MemoryCommand::Create {
                    record: first.clone()
                }
            }
        )
        .await
        .is_err()
    );
    let active = hm_ledger::retention::active_directory(dir.path()).unwrap();
    let certificate = fs::read(active.join("REDACTIONS")).unwrap();
    actor.shutdown().await.unwrap();
    fs::create_dir(dir.path().join("log")).unwrap();
    fs::write(dir.path().join("log/old-copy.seg"), &sensitive[0]).unwrap();
    let reopened = ActorEngine::open(cfg.clone()).await.unwrap();
    assert!(!dir.path().join("log").exists());
    assert_eq!(reopened.verification_status().await.unwrap().root, root);
    assert!(
        context_memory::rebuild(&reopened, &scope())
            .await
            .unwrap()
            .purged_records
            .contains_key("erase-me")
    );
    reopened.shutdown().await.unwrap();
    let mut corrupt = certificate.clone();
    let last = corrupt.len() - 1;
    corrupt[last] ^= 1;
    fs::write(active.join("REDACTIONS"), corrupt).unwrap();
    assert!(ActorEngine::open(cfg.clone()).await.is_err());
    fs::write(active.join("REDACTIONS"), certificate).unwrap();
    let segments = fs::read_dir(active.join("log"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|e| e == "seg"))
        .unwrap();
    let original_segment = fs::read(&segments).unwrap();
    let mut corruption = original_segment.clone();
    let length = frame::encoded_frame_length(&corruption).unwrap();
    let mut altered = frame::decode(&corruption[..length]).unwrap();
    altered.sealed_payload[40] ^= 1;
    corruption[..length].copy_from_slice(&frame::encode(&altered).unwrap());
    fs::write(&segments, corruption).unwrap();
    assert!(ActorEngine::open(cfg.clone()).await.is_err());
    fs::write(&segments, original_segment).unwrap();
    let reopened = ActorEngine::open(cfg).await.unwrap();
    assert_eq!(reopened.verification_status().await.unwrap().root, root);
    assert_eq!(
        context_memory::rebuild(&reopened, &scope())
            .await
            .unwrap()
            .records["keep-me"]
            .content,
        keep.content
    );
    reopened.shutdown().await.unwrap();
}
