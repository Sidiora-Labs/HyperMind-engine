use hm_context::{Authority, Scope, development::*, digest_bytes, maintenance::Usage};
use hm_core::{ActorId, ConversationId};
use hm_ledger::frame::EventKind;
use hm_schema::{
    event::{CURRENT_SCHEMA_VERSION, encode_event_envelope},
    events::{EventEnvelope, EventPayload, Retention, Sensitivity, UserMsg},
};
use hm_serve::{
    actor::{ActorConfig, ActorEngine, IncomingEvent},
    context_memory::{
        self, MemoryCommand, MemoryGrant, MemoryRecord, MemoryRequest, MemorySource, Provenance,
        RecordKind, RecordStatus,
    },
    development_admission,
};
use std::collections::{BTreeMap, BTreeSet};
fn owner() -> Scope {
    Scope {
        owner_id: "owner".into(),
        project_id: "development".into(),
        workspace_id: None,
    }
}
fn worker() -> Scope {
    Scope {
        owner_id: "worker".into(),
        project_id: "development".into(),
        workspace_id: None,
    }
}
fn config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.into(),
        actor: ActorId::new(31),
        user: [4; 16],
        kek: [9; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}
async fn command(actor: &ActorEngine, id: &str, command: MemoryCommand) {
    context_memory::execute(
        actor,
        &owner(),
        &owner(),
        MemoryRequest {
            version: 1,
            scope: owner(),
            request_id: id.into(),
            command,
        },
    )
    .await
    .unwrap();
}
fn capability(id: &str, kind: DevelopmentKind) -> WorkerCapability {
    WorkerCapability {
        id: id.into(),
        scope: owner(),
        principal: worker(),
        worker_id: "worker-runtime".into(),
        revision: 1,
        revoked: false,
        allowed_kinds: BTreeSet::from([kind]),
        source_ids: BTreeSet::from(["evidence".into()]),
        record_ids: BTreeSet::from(["existing".into()]),
        new_record_ids: BTreeSet::from(["new-one".into(), "new-two".into()]),
        session_id: "session".into(),
        conversation: "conversation".into(),
        lease: DevelopmentLease {
            id: format!("lease-{id}"),
            attempt: 1,
            expires_at_ns: i64::MAX,
        },
        budget: DevelopmentBudget {
            reserved_tokens: 12000,
            max_input_bytes: 65536,
            max_output_bytes: 65536,
            max_mutations: 8,
        },
    }
}
fn record(id: &str, content: &str) -> MemoryRecord {
    let mut record = MemoryRecord::new(id, RecordKind::Note, content, 100);
    record.provenance = vec![Provenance {
        source_id: "evidence".into(),
        source_digest: digest_bytes(b"the vault access code is 4821."),
        span_start: 0,
        span_end: 30,
        quoted_digest: digest_bytes(b"the vault access code is 4821."),
    }];
    record
}
fn generated(id: &str) -> DevelopmentRecord {
    let mut record = record(id, "the vault access code is 4821.");
    record.authority = Authority::DerivedInference;
    serde_json::from_value(serde_json::to_value(record).unwrap()).unwrap()
}
async fn setup(actor: &ActorEngine, kind: DevelopmentKind) {
    command(
        actor,
        "source",
        MemoryCommand::Source {
            source: MemorySource {
                id: "evidence".into(),
                digest: digest_bytes(b"the vault access code is 4821."),
                content: b"the vault access code is 4821.".to_vec(),
                locator: "local:source".into(),
                occurred_at_ns: None,
                recorded_at_ns: 100,
                tombstoned: false,
            },
        },
    )
    .await;
    command(
        actor,
        "existing",
        MemoryCommand::Create {
            record: record("existing", "the vault access code is 4821."),
        },
    )
    .await;
    command(
        actor,
        "grant",
        MemoryCommand::SetGrant {
            grant: MemoryGrant {
                principal_digest: None,
                id: "read-grant".into(),
                principal: worker(),
                record_ids: BTreeSet::from(["existing".into()]),
                categories: BTreeSet::new(),
                read: true,
                expires_at_ns: None,
                revoked: false,
                revision: 1,
                record_revisions: BTreeMap::new(),
            },
        },
    )
    .await;
    command(
        actor,
        "capability",
        MemoryCommand::RegisterWorker {
            capability: capability("cap", kind),
        },
    )
    .await;
}
async fn snapshot(actor: &ActorEngine, cap: &str, records: bool) -> EvidenceSnapshot {
    development_admission::snapshot(
        actor,
        &owner(),
        &worker(),
        "worker-runtime",
        SnapshotRequest {
            capability_id: cap.into(),
            source_ids: BTreeSet::from(["evidence".into()]),
            record_ids: if records {
                BTreeSet::from(["existing".into()])
            } else {
                BTreeSet::new()
            },
        },
    )
    .await
    .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires a configured actual structured generation provider"]
async fn actual_curation_classification_conflict_and_restart() {
    use hm_cortex::development_curation::{CurationAction, CurationRequest};
    use hm_serve::development_curation::{self, ApplyBoundary};
    let endpoint =
        std::env::var("HM_DEVELOPMENT_ENDPOINT").expect("configured generation endpoint");
    let model = std::env::var("HM_DEVELOPMENT_MODEL").expect("configured generation model");
    let provider = tokio::task::spawn_blocking(move || {
        std::sync::Arc::new(
            hm_llm::ollama::Ollama::new(
                hm_llm::ProviderConfig {
                    endpoint,
                    model,
                    api_key: None,
                    tier: hm_llm::ModelTier::Economy,
                    pricing: Default::default(),
                },
                hm_llm::HttpTransport::default(),
            )
            .unwrap(),
        ) as std::sync::Arc<dyn hm_llm::LlmProvider>
    })
    .await
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(dir.path())).await.unwrap();
    setup(&actor, DevelopmentKind::Curation).await;
    let evidence = snapshot(&actor, "cap", true).await;
    let plan = development_curation::infer(
        evidence.clone(),
        provider.clone(),
        CurationRequest {
            plan_id: "curation-real".into(),
            actions: vec![CurationAction::Reword {
                id: "existing".into(),
            }],
        },
    )
    .await
    .unwrap();
    assert!(matches!(plan.usage,Usage::Known(n) if n>0));
    let boundary = ApplyBoundary {
        session_id: "session".into(),
        generation: None,
        materialization_digest: None,
    };
    let before = actor.stats().await.unwrap().log_events;
    development_curation::apply(
        &actor,
        &owner(),
        &worker(),
        "worker-runtime",
        plan.clone(),
        &boundary,
    )
    .await
    .unwrap();
    assert_eq!(actor.stats().await.unwrap().log_events, before + 1);
    let state = context_memory::rebuild(&actor, &owner()).await.unwrap();
    assert_eq!(state.records["existing"].revision, 2);
    assert_eq!(
        state.records["existing"].metadata["classification"]["shareability"],
        "private"
    );
    assert!(state.visible_records(&worker(), 100).unwrap().is_empty());
    let mut stale = plan.clone();
    stale.id = "stale-curation".into();
    assert!(
        development_curation::apply(
            &actor,
            &owner(),
            &worker(),
            "worker-runtime",
            stale,
            &boundary
        )
        .await
        .is_err()
    );
    command(
        &actor,
        "refresh-grant",
        MemoryCommand::SetGrant {
            grant: MemoryGrant {
                principal_digest: None,
                id: "read-grant".into(),
                principal: worker(),
                record_ids: BTreeSet::from(["existing".into()]),
                categories: BTreeSet::new(),
                read: true,
                expires_at_ns: None,
                revoked: false,
                revision: 2,
                record_revisions: BTreeMap::new(),
            },
        },
    )
    .await;
    command(
        &actor,
        "classification-cap",
        MemoryCommand::RegisterWorker {
            capability: capability("classification-cap", DevelopmentKind::Curation),
        },
    )
    .await;
    let classification = development_curation::infer(
        snapshot(&actor, "classification-cap", true).await,
        provider.clone(),
        CurationRequest {
            plan_id: "classify-real".into(),
            actions: vec![CurationAction::Classify {
                id: "existing".into(),
            }],
        },
    )
    .await
    .unwrap();
    let content = state.records["existing"].content.clone();
    development_curation::apply(
        &actor,
        &owner(),
        &worker(),
        "worker-runtime",
        classification,
        &boundary,
    )
    .await
    .unwrap();
    let mut pinned = record("protected", "the vault access code is 4821.");
    pinned.pinned = true;
    pinned.contradictions = vec!["existing".into()];
    command(
        &actor,
        "protected",
        MemoryCommand::Create { record: pinned },
    )
    .await;
    let mut cap = capability("merge-cap", DevelopmentKind::Curation);
    cap.record_ids.insert("protected".into());
    command(
        &actor,
        "merge-cap",
        MemoryCommand::RegisterWorker { capability: cap },
    )
    .await;
    command(
        &actor,
        "merge-grant",
        MemoryCommand::SetGrant {
            grant: MemoryGrant {
                principal_digest: None,
                id: "merge-grant".into(),
                principal: worker(),
                record_ids: BTreeSet::from(["existing".into(), "protected".into()]),
                categories: BTreeSet::new(),
                read: true,
                expires_at_ns: None,
                revoked: false,
                revision: 1,
                record_revisions: BTreeMap::new(),
            },
        },
    )
    .await;
    let merge_snapshot = development_admission::snapshot(
        &actor,
        &owner(),
        &worker(),
        "worker-runtime",
        SnapshotRequest {
            capability_id: "merge-cap".into(),
            source_ids: BTreeSet::from(["evidence".into()]),
            record_ids: BTreeSet::from(["existing".into(), "protected".into()]),
        },
    )
    .await
    .unwrap();
    let protected_request = CurationRequest {
        plan_id: "archive-pin".into(),
        actions: vec![CurationAction::Archive {
            id: "protected".into(),
            duplicate_of: "existing".into(),
        }],
    };
    assert!(
        hm_cortex::development_curation::curate(
            merge_snapshot.clone(),
            provider.as_ref(),
            protected_request
        )
        .is_err()
    );
    let merged = development_curation::infer(
        merge_snapshot,
        provider,
        CurationRequest {
            plan_id: "merge-real".into(),
            actions: vec![CurationAction::Merge {
                id: "new-one".into(),
                parents: vec!["existing".into(), "protected".into()],
                archive_parents: true,
            }],
        },
    )
    .await
    .unwrap();
    development_curation::apply(
        &actor,
        &owner(),
        &worker(),
        "worker-runtime",
        merged,
        &boundary,
    )
    .await
    .unwrap();
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(dir.path())).await.unwrap();
    let state = context_memory::rebuild(&actor, &owner()).await.unwrap();
    assert_eq!(state.records["existing"].content, content);
    assert_eq!(state.records["existing"].revision, 4);
    assert!(state.records["protected"].pinned);
    assert_eq!(state.records["protected"].status, RecordStatus::Active);
    assert!(
        state.records["new-one"]
            .lineage
            .iter()
            .any(|l| l.parent_record_id == "protected" && l.relation == "merged_from")
    );
    assert!(
        state.records["new-one"]
            .lineage
            .iter()
            .any(|l| l.parent_record_id == "existing" && l.relation == "merged_from")
    );
    assert_eq!(state.records["new-one"].provenance, state.records["protected"].provenance);
    assert!(
        state.records["new-one"]
            .contradictions
            .contains(&"existing".to_string())
    );
    assert_eq!(state.development_receipts.len(), 3);
    assert!(
        !state
            .visible_records(&worker(), 100)
            .unwrap()
            .iter()
            .any(|r| r.id == "existing" || r.id == "new-one")
    );
    actor.shutdown().await.unwrap();
}
