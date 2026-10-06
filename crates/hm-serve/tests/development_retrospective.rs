use hm_context::{
    development::*, history::SourceRelation, Authority, MessagePart, MessageRole, Scope,
    SourceMessage,
};
use hm_core::ActorId;
use hm_cortex::development_retrospective::{
    RetrospectiveProposal, CHECKPOINT_CATEGORY, LESSON_CATEGORY,
};
use hm_llm::{ollama::Ollama, HttpTransport, LlmProvider, ModelTier, Pricing, ProviderConfig};
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_history::{self, RelationIngestion, SourceIngestion},
    context_memory::{self, MemoryCommand, MemoryGrant, MemoryRequest, RecordStatus},
    development_retrospective::{
        self as retrospective, RetrospectiveOutcome, RetrospectiveRequest,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
fn scope() -> Scope {
    Scope {
        owner_id: "owner".into(),
        project_id: "retrospective".into(),
        workspace_id: None,
    }
}
fn worker() -> Scope {
    Scope {
        owner_id: "worker".into(),
        project_id: "retrospective".into(),
        workspace_id: None,
    }
}
fn config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.into(),
        actor: ActorId::new(49),
        user: [5; 16],
        kek: [6; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}
fn message(id: &str, ordinal: u64, text: &str) -> SourceMessage {
    let mut message = SourceMessage {
        id: id.into(),
        ordinal,
        role: MessageRole::User,
        parts: vec![MessagePart::Text { text: text.into() }],
        occurred_at_ns: None,
        recorded_at_ns: 100 + ordinal as i64,
        authority: Authority::UserAsserted,
        source_digest: String::new(),
    };
    message.source_digest = message.computed_digest().unwrap();
    message
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
async fn capability(
    actor: &ActorEngine,
    id: &str,
    records: BTreeSet<String>,
    new_ids: BTreeSet<String>,
) {
    if !records.is_empty() {
        command(
            actor,
            &format!("grant-{id}"),
            MemoryCommand::SetGrant {
                grant: MemoryGrant {
                    principal_digest: None,
                    id: format!("read-{id}"),
                    principal: worker(),
                    record_ids: records.clone(),
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
    }
    command(
        actor,
        id,
        MemoryCommand::RegisterWorker {
            capability: WorkerCapability {
                id: id.into(),
                scope: scope(),
                principal: worker(),
                worker_id: "retrospective-worker".into(),
                revision: 1,
                revoked: false,
                allowed_kinds: BTreeSet::from([DevelopmentKind::Retrospective]),
                source_ids: BTreeSet::from(["correction".into()]),
                record_ids: records,
                new_record_ids: new_ids,
                session_id: "session".into(),
                conversation: "conversation".into(),
                lease: DevelopmentLease {
                    id: format!("lease-{id}"),
                    attempt: 1,
                    expires_at_ns: i64::MAX,
                },
                budget: DevelopmentBudget {
                    reserved_tokens: 16384,
                    max_input_bytes: 65536,
                    max_output_bytes: 65536,
                    max_mutations: 8,
                },
            },
        },
    )
    .await;
}
fn request(
    cap: &str,
    plan: &str,
    records: BTreeSet<String>,
    lesson_ids: Vec<String>,
) -> RetrospectiveRequest {
    RetrospectiveRequest {
        plan_id: plan.into(),
        checkpoint_id: "checkpoint".into(),
        lesson_ids,
        snapshot: SnapshotRequest {
            capability_id: cap.into(),
            source_ids: BTreeSet::from(["correction".into()]),
            record_ids: records,
        },
    }
}
fn provider() -> Arc<dyn LlmProvider> {
    Arc::new(
        Ollama::new(
            ProviderConfig {
                endpoint: std::env::var("HM_TEST_OLLAMA_ENDPOINT")
                    .expect("set HM_TEST_OLLAMA_ENDPOINT to the configured local generation endpoint"),
                api_key: None,
                model: std::env::var("HM_TEST_OLLAMA_MODEL")
                    .unwrap_or_else(|_| "qwen2.5:3b".into()),
                tier: ModelTier::Economy,
                pricing: Pricing::default(),
            },
            HttpTransport::default(),
        )
        .unwrap(),
    )
}
async fn rebound_plan(
    actor: &ActorEngine,
    mut plan: DevelopmentPlan,
    capability: &str,
    plan_id: &str,
    lesson_id: &str,
    checkpoint_id: &str,
) -> DevelopmentPlan {
    plan.id = plan_id.into();
    plan.evidence = hm_serve::development_admission::snapshot(
        actor,
        &scope(),
        &worker(),
        "retrospective-worker",
        SnapshotRequest {
            capability_id: capability.into(),
            source_ids: BTreeSet::from(["correction".into()]),
            record_ids: BTreeSet::new(),
        },
    )
    .await
    .unwrap();
    for mutation in &mut plan.mutations {
        if let PlannedKnowledgeMutation::Create { record } = mutation {
            record.id = if record.category == CHECKPOINT_CATEGORY {
                checkpoint_id
            } else {
                lesson_id
            }
            .into();
        }
    }
    plan
}
#[tokio::test]
async fn real_user_corrections_create_grounded_lessons_and_atomic_watermark_with_restart_fences() {
    let dir = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(dir.path())).await.unwrap();
    let corrected="Correction: never force-push a shared branch. Create a reviewed pull request and wait for approval before merging.";
    for source in [
        message(
            "original",
            1,
            "Use a force push to publish a shared branch.",
        ),
        message("correction", 2, corrected),
    ] {
        let original_bytes = match &source.parts[0] {
            MessagePart::Text { text } => text.as_bytes().to_vec(),
            _ => unreachable!(),
        };
        context_history::ingest(
            &actor,
            &scope(),
            &SourceIngestion {
                version: 1,
                scope: scope(),
                session_id: "session".into(),
                conversation: "conversation".into(),
                message: source,
                original_bytes,
            },
        )
        .await
        .unwrap();
    }
    capability(
        &actor,
        "cap",
        BTreeSet::new(),
        BTreeSet::from(["checkpoint".into(), "lesson-one".into()]),
    )
    .await;
    let model = tokio::task::spawn_blocking(provider).await.unwrap();
    let initial = request(
        "cap",
        "before-relation",
        BTreeSet::new(),
        vec!["lesson-one".into()],
    );
    let before = actor.stats().await.unwrap();
    assert!(matches!(
        retrospective::run(
            &actor,
            &scope(),
            &worker(),
            "retrospective-worker",
            initial,
            model.clone()
        )
        .await
        .unwrap(),
        RetrospectiveOutcome::NoSignal {
            provider_calls: 0,
            ..
        }
    ));
    assert_eq!(actor.stats().await.unwrap(), before);
    context_history::relate(
        &actor,
        &scope(),
        &RelationIngestion {
            version: 1,
            scope: scope(),
            session_id: "session".into(),
            conversation: "conversation".into(),
            relation: SourceRelation::Edit {
                id: "user-correction".into(),
                original_id: "original".into(),
                replacement_id: "correction".into(),
            },
        },
    )
    .await
    .unwrap();
    let planned = retrospective::propose(
        &actor,
        &scope(),
        &worker(),
        "retrospective-worker",
        request("cap", "lessons", BTreeSet::new(), vec!["lesson-one".into()]),
        model.clone(),
    )
    .await
    .unwrap();
    let plan = match &planned {
        RetrospectiveProposal::Proposed {
            plan,
            provider_calls: 1,
            ..
        } => plan.clone(),
        _ => panic!("eligible correction did not call provider"),
    };
    assert_eq!(plan.mutations.len(), 2);
    let lesson = plan
        .mutations
        .iter()
        .find_map(|m| match m {
            PlannedKnowledgeMutation::Create { record } if record.category == LESSON_CATEGORY => {
                Some(record)
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(lesson.authority, Authority::DerivedInference);
    assert_eq!(lesson.kind, DevelopmentRecordKind::Note);
    assert_eq!(lesson.metadata["review_status"], "proposed");
    assert_eq!(lesson.provenance[0].source_id, "correction");
    assert!(!lesson.content.is_empty());
    assert!(!lesson.provenance.is_empty());
    let before = actor.stats().await.unwrap().log_events;
    let outcome = retrospective::publish(
        &actor,
        &scope(),
        &worker(),
        "retrospective-worker",
        planned.clone(),
    )
    .await
    .unwrap();
    assert!(matches!(
        outcome,
        RetrospectiveOutcome::Published {
            provider_calls: 1,
            ..
        }
    ));
    assert_eq!(actor.stats().await.unwrap().log_events, before + 1);
    let state = context_memory::rebuild(&actor, &scope()).await.unwrap();
    assert_eq!(state.records["checkpoint"].category, CHECKPOINT_CATEGORY);
    assert_eq!(state.records["checkpoint"].status, RecordStatus::Archived);
    assert_eq!(state.records["checkpoint"].revision, 2);
    assert_eq!(
        state.records["checkpoint"].last_lsn,
        state.records["lesson-one"].last_lsn
    );
    assert!(!state
        .visible_records(&scope(), 1000)
        .unwrap()
        .iter()
        .any(|r| r.id == "checkpoint"));
    assert!(matches!(
        retrospective::publish(&actor, &scope(), &worker(), "retrospective-worker", planned)
            .await
            .unwrap(),
        RetrospectiveOutcome::Published {
            receipt: WorkerReceipt { replayed: true, .. },
            ..
        }
    ));
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(dir.path())).await.unwrap();
    let records = BTreeSet::from(["checkpoint".into(), "lesson-one".into()]);
    capability(
        &actor,
        "resume-cap",
        records.clone(),
        BTreeSet::from(["lesson-two".into()]),
    )
    .await;
    let before = actor.stats().await.unwrap();
    assert!(matches!(
        retrospective::run(
            &actor,
            &scope(),
            &worker(),
            "retrospective-worker",
            request("resume-cap", "resume", records, vec!["lesson-two".into()]),
            model
        )
        .await
        .unwrap(),
        RetrospectiveOutcome::NoSignal {
            provider_calls: 0,
            ..
        }
    ));
    assert_eq!(actor.stats().await.unwrap(), before);
    capability(
        &actor,
        "cancel-cap",
        BTreeSet::new(),
        BTreeSet::from(["checkpoint-two".into(), "lesson-two".into()]),
    )
    .await;
    let cancelled = rebound_plan(
        &actor,
        plan.clone(),
        "cancel-cap",
        "cancelled",
        "lesson-two",
        "checkpoint-two",
    )
    .await;
    command(
        &actor,
        "cancel",
        MemoryCommand::RevokeWorker {
            id: "cancel-cap".into(),
            expected_revision: 1,
        },
    )
    .await;
    let before = actor.stats().await.unwrap();
    assert!(retrospective::publish(
        &actor,
        &scope(),
        &worker(),
        "retrospective-worker",
        RetrospectiveProposal::Proposed {
            watermark: serde_json::from_value(
                cancelled
                    .mutations
                    .iter()
                    .find_map(|mutation| match mutation {
                        PlannedKnowledgeMutation::Create { record }
                            if record.category == CHECKPOINT_CATEGORY =>
                            Some(record.metadata.clone()),
                        _ => None,
                    })
                    .unwrap()
            )
            .unwrap(),
            plan: cancelled,
            provider_calls: 1
        }
    )
    .await
    .is_err());
    assert_eq!(actor.stats().await.unwrap(), before);
    capability(
        &actor,
        "stale-cap",
        BTreeSet::new(),
        BTreeSet::from(["checkpoint-three".into(), "lesson-three".into()]),
    )
    .await;
    let stale = rebound_plan(
        &actor,
        plan,
        "stale-cap",
        "stale-input",
        "lesson-three",
        "checkpoint-three",
    )
    .await;
    context_history::relate(
        &actor,
        &scope(),
        &RelationIngestion {
            version: 1,
            scope: scope(),
            session_id: "session".into(),
            conversation: "conversation".into(),
            relation: SourceRelation::Tombstone {
                id: "removed-correction".into(),
                source_id: "correction".into(),
            },
        },
    )
    .await
    .unwrap();
    let before = actor.stats().await.unwrap();
    assert!(retrospective::publish(
        &actor,
        &scope(),
        &worker(),
        "retrospective-worker",
        RetrospectiveProposal::Proposed {
            watermark: serde_json::from_value(
                stale
                    .mutations
                    .iter()
                    .find_map(|mutation| match mutation {
                        PlannedKnowledgeMutation::Create { record }
                            if record.category == CHECKPOINT_CATEGORY =>
                            Some(record.metadata.clone()),
                        _ => None,
                    })
                    .unwrap()
            )
            .unwrap(),
            plan: stale,
            provider_calls: 1
        }
    )
    .await
    .is_err());
    assert_eq!(actor.stats().await.unwrap(), before);
    actor.shutdown().await.unwrap();
}
