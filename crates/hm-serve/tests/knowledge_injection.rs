use hm_context::{Scope, knowledge_injection::VisibleEvidence};
use hm_core::ActorId;
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_memory::{
        self, MemoryCommand, MemoryGrant, MemoryRecord, MemoryRequest, RecordKind, RecordStatus,
    },
    knowledge_injection as knowledge,
};
use std::collections::{BTreeMap, BTreeSet};
fn scope() -> Scope {
    Scope {
        owner_id: "knowledge-owner".into(),
        project_id: "project".into(),
        workspace_id: None,
    }
}
fn guest() -> Scope {
    Scope {
        owner_id: "knowledge-reader".into(),
        ..scope()
    }
}
fn config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.into(),
        actor: ActorId::new(17),
        user: [3; 16],
        kek: [4; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}
async fn command(actor: &ActorEngine, id: &str, command: MemoryCommand) {
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
#[tokio::test]
async fn real_native_categories_budget_grants_observed_use_and_restart() {
    let root = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(root.path())).await.unwrap();
    for (id, category, pinned, importance, time) in [
        ("pinned", "work", true, 1, 100),
        ("recent", "work", false, 900_000, 300),
        ("older", "work", false, 900_000, 200),
        ("private", "personal", false, 1_000_000, 400),
    ] {
        let mut record = MemoryRecord::new(
            id,
            RecordKind::Fact,
            format!("Exact native knowledge {id}"),
            time,
        );
        record.category = category.into();
        record.pinned = pinned;
        record.importance = importance;
        command(&actor, id, MemoryCommand::Create { record }).await;
    }
    command(
        &actor,
        "archive",
        MemoryCommand::SetStatus {
            id: "older".into(),
            status: RecordStatus::Archived,
            expected_revision: 1,
        },
    )
    .await;
    let state = context_memory::rebuild(&actor, &scope()).await.unwrap();
    let revisions: BTreeMap<_, _> = state
        .records
        .values()
        .filter(|r| r.category == "work")
        .map(|r| (r.id.clone(), r.revision_digest.clone()))
        .collect();
    command(
        &actor,
        "share",
        MemoryCommand::SetGrant {
            grant: MemoryGrant {
                principal_digest: None,
                id: "share-work".into(),
                principal: guest(),
                record_ids: BTreeSet::new(),
                categories: BTreeSet::from(["work".into()]),
                read: true,
                expires_at_ns: None,
                revoked: false,
                revision: 1,
                record_revisions: revisions,
            },
        },
    )
    .await;
    let before = actor.stats().await.unwrap().applied.last_lsn;
    let all = knowledge::select(
        &actor,
        &scope(),
        &guest(),
        "second-session",
        7,
        10_000,
        &VisibleEvidence::default(),
        "gpt-4o",
    )
    .await
    .unwrap();
    assert_eq!(
        all.blocks.iter().map(|b| b.id.as_str()).collect::<Vec<_>>(),
        vec!["pinned", "recent"]
    );
    assert_eq!(actor.stats().await.unwrap().applied.last_lsn, before);
    assert_eq!(
        knowledge::select(
            &actor,
            &scope(),
            &guest(),
            "second-session",
            7,
            10_000,
            &VisibleEvidence::default(),
            "gpt-4o"
        )
        .await
        .unwrap(),
        all
    );
    assert!(knowledge::validate_plan(&actor, &all, 8).await.is_err());
    let trimmed = knowledge::select(
        &actor,
        &scope(),
        &guest(),
        "second-session",
        7,
        all.blocks[0].tokens,
        &VisibleEvidence::default(),
        "gpt-4o",
    )
    .await
    .unwrap();
    assert_eq!(trimmed.blocks.len(), 1);
    assert_eq!(trimmed.blocks[0].id, "pinned");
    let visible = VisibleEvidence {
        records: vec![all.selected[0].clone()],
        source_spans: vec![],
    };
    let suppressed = knowledge::select(
        &actor,
        &scope(),
        &guest(),
        "second-session",
        7,
        10_000,
        &visible,
        "gpt-4o",
    )
    .await
    .unwrap();
    assert_eq!(suppressed.blocks[0].id, "recent");
    let mut wrong = visible.clone();
    wrong.records[0].revision_digest = "old".into();
    assert_eq!(
        knowledge::select(
            &actor,
            &scope(),
            &guest(),
            "second-session",
            7,
            10_000,
            &wrong,
            "gpt-4o"
        )
        .await
        .unwrap()
        .blocks
        .len(),
        2
    );
    assert_eq!(
        knowledge::exact_recall(&actor, &scope(), &guest(), "older")
            .await
            .unwrap()
            .unwrap()
            .status,
        RecordStatus::Archived
    );
    assert!(
        knowledge::exact_recall(&actor, &scope(), &guest(), "private")
            .await
            .is_err()
    );
    let receipt = knowledge::observe_use(
        &actor,
        &all,
        7,
        "turn-one",
        &BTreeSet::from(["pinned".into()]),
    )
    .await
    .unwrap();
    assert!(!receipt.replayed);
    knowledge::validate_plan(&actor, &all, 7).await.unwrap();
    assert!(
        knowledge::observe_use(
            &actor,
            &all,
            7,
            "turn-one",
            &BTreeSet::from(["pinned".into()])
        )
        .await
        .unwrap()
        .replayed
    );
    let snapshot = knowledge::snapshot(&actor, &scope(), &guest(), "third-session", 9)
        .await
        .unwrap();
    assert_eq!(
        snapshot
            .records
            .iter()
            .find(|r| r.id == "pinned")
            .unwrap()
            .observed_uses,
        1
    );
    assert_eq!(
        snapshot
            .records
            .iter()
            .find(|r| r.id == "pinned")
            .unwrap()
            .importance,
        1
    );
    let fresh = knowledge::select(
        &actor,
        &scope(),
        &guest(),
        "third-session",
        9,
        10_000,
        &VisibleEvidence::default(),
        "gpt-4o",
    )
    .await
    .unwrap();
    command(
        &actor,
        "revoke",
        MemoryCommand::RevokeGrant {
            id: "share-work".into(),
        },
    )
    .await;
    assert!(knowledge::validate_plan(&actor, &fresh, 9).await.is_err());
    assert!(
        knowledge::observe_use(
            &actor,
            &fresh,
            9,
            "revoked-turn",
            &BTreeSet::from(["pinned".into()])
        )
        .await
        .is_err()
    );
    assert!(
        knowledge::exact_recall(&actor, &scope(), &guest(), "pinned")
            .await
            .is_err()
    );
    command(
        &actor,
        "delete",
        MemoryCommand::Tombstone {
            id: "recent".into(),
            expected_revision: 1,
        },
    )
    .await;
    assert!(
        knowledge::exact_recall(&actor, &scope(), &scope(), "recent")
            .await
            .unwrap()
            .is_none()
    );
    actor.shutdown().await.unwrap();
    let reopened = ActorEngine::open(config(root.path())).await.unwrap();
    let restored = knowledge::snapshot(&reopened, &scope(), &scope(), "fourth-session", 10)
        .await
        .unwrap();
    assert_eq!(
        restored
            .records
            .iter()
            .find(|r| r.id == "pinned")
            .unwrap()
            .observed_uses,
        1
    );
    assert!(
        !restored
            .records
            .iter()
            .any(|r| r.id == "recent" || r.id == "older")
    );
    assert_eq!(
        context_memory::rebuild(&reopened, &scope())
            .await
            .unwrap()
            .records["pinned"]
            .importance,
        1
    );
    reopened.shutdown().await.unwrap();
}

#[tokio::test]
async fn native_visible_quote_suppression_requires_exact_original_evidence() {
    let root = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(root.path())).await.unwrap();
    let content = b"Measured pressure 24 kPa";
    let digest = hm_context::digest_bytes(content);
    command(
        &actor,
        "original",
        MemoryCommand::Source {
            source: context_memory::MemorySource {
                id: "source".into(),
                digest: digest.clone(),
                content: content.to_vec(),
                locator: "measurement:source".into(),
                occurred_at_ns: None,
                recorded_at_ns: 100,
                tombstoned: false,
            },
        },
    )
    .await;
    for (id, text) in [
        ("verbatim", "Measured pressure 24 kPa"),
        ("inference", "Pressure appears stable"),
    ] {
        let mut record = MemoryRecord::new(id, RecordKind::Fact, text, 101);
        record.provenance = vec![context_memory::Provenance {
            source_id: "source".into(),
            source_digest: digest.clone(),
            span_start: 0,
            span_end: content.len() as u64,
            quoted_digest: digest.clone(),
        }];
        if id == "inference" {
            record.authority = hm_context::Authority::DerivedInference;
        }
        command(&actor, id, MemoryCommand::Create { record }).await;
    }
    let span = hm_context::SourceSpan {
        source_id: "source".into(),
        source_digest: digest,
        byte_start: 0,
        byte_end: content.len() as u64,
    };
    let visible = VisibleEvidence {
        records: vec![],
        source_spans: vec![span.clone()],
    };
    let plan = knowledge::select(
        &actor,
        &scope(),
        &scope(),
        "session",
        1,
        10_000,
        &visible,
        "gpt-4o",
    )
    .await
    .unwrap();
    assert_eq!(
        plan.blocks
            .iter()
            .map(|b| b.id.as_str())
            .collect::<Vec<_>>(),
        vec!["inference"]
    );
    let mut partial = visible;
    partial.source_spans[0].byte_end -= 1;
    assert_eq!(
        knowledge::select(
            &actor,
            &scope(),
            &scope(),
            "session",
            1,
            10_000,
            &partial,
            "gpt-4o"
        )
        .await
        .unwrap()
        .blocks
        .len(),
        2
    );
    assert!(
        knowledge::exact_recall(&actor, &scope(), &scope(), "verbatim")
            .await
            .unwrap()
            .is_some()
    );
    command(
        &actor,
        "source-delete",
        MemoryCommand::TombstoneSource {
            id: "source".into(),
        },
    )
    .await;
    assert!(knowledge::validate_plan(&actor, &plan, 1).await.is_err());
    assert!(
        knowledge::exact_recall(&actor, &scope(), &scope(), "verbatim")
            .await
            .unwrap()
            .is_none()
    );
    actor.shutdown().await.unwrap();
}
