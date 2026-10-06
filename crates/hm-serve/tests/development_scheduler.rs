use hm_context::{
    Scope, development::*, development_schedule::*, digest_bytes, maintenance::Usage,
};
use hm_core::ActorId;
use hm_cortex::development_historian::HistorianProvider;
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_memory::{self, MemoryCommand, MemoryRequest, MemorySource},
    development_historian::HistorianWorker,
    development_scheduler::{self, SchedulerAction, SchedulerRequest, WorkerRegistry},
};
use std::{collections::BTreeSet, sync::Arc, time::Duration};
fn scope() -> Scope {
    Scope {
        owner_id: "scheduled-owner".into(),
        project_id: "laboratory".into(),
        workspace_id: None,
    }
}
fn worker_scope() -> Scope {
    Scope {
        owner_id: "scheduled-worker".into(),
        project_id: "laboratory".into(),
        workspace_id: None,
    }
}
fn config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.into(),
        actor: ActorId::new(61),
        user: [6; 16],
        kek: [7; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}
fn schedule(id: &str, mode: ScheduleMode) -> DevelopmentSchedule {
    DevelopmentSchedule {
        id: id.into(),
        kind: DevelopmentKind::Extraction,
        mode,
        worker_id: "extractor".into(),
        principal: worker_scope(),
        snapshot: SnapshotRequest {
            capability_id: "extractor-capability".into(),
            source_ids: BTreeSet::from(["calibration".into()]),
            record_ids: BTreeSet::new(),
        },
        reservation: 8192,
        timeout_ms: 60_000,
        backoff_ms: 0,
        max_attempts: 3,
        identical_failure_limit: 2,
        provider_policy: None,
    }
}
async fn action(actor: &ActorEngine, id: &str, action: SchedulerAction) -> serde_json::Value {
    development_scheduler::execute(
        actor,
        &scope(),
        &scope(),
        SchedulerRequest {
            version: 1,
            scope: scope(),
            request_id: id.into(),
            action,
        },
    )
    .await
    .unwrap()
}
async fn memory(actor: &ActorEngine, id: &str, command: MemoryCommand) {
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
#[test]
fn component_backlog_frontier_circuit_and_unknown_holds() {
    let mut state = DevelopmentSchedules::new(scope(), 50_000, 2).unwrap();
    let mut policy = schedule("manual", ScheduleMode::Manual);
    policy.reservation = 100;
    state.configure(policy, 0).unwrap();
    state
        .configure(schedule("off", ScheduleMode::Disabled), 0)
        .unwrap();
    state
        .configure(
            schedule("clock", ScheduleMode::Timed { interval_ms: 10 }),
            0,
        )
        .unwrap();
    assert_eq!(state.due(0), vec!["clock"]);
    assert!(state.enqueue("off", digest_bytes(b"off"), 0).is_err());
    let first = state.enqueue("manual", digest_bytes(b"first"), 0).unwrap();
    let second = state.enqueue("manual", digest_bytes(b"second"), 0).unwrap();
    let lease = state.claim(&second, 0).unwrap();
    state
        .finish(&lease, Usage::Known(7), "component-receipt".into(), 1)
        .unwrap();
    assert_eq!(state.progress["manual"].completed_frontier, 0);
    let lease = state.claim(&first, 2).unwrap();
    state
        .fail(&lease, Usage::Unknown, "same provider refusal".into(), 3)
        .unwrap();
    let lease = state.claim(&first, 4).unwrap();
    state
        .fail(&lease, Usage::Known(3), "same provider refusal".into(), 5)
        .unwrap();
    assert!(state.progress["manual"].circuit_open);
    assert!(state.claim(&first, 6).is_err());
    assert_eq!(state.accounting.spent, 10);
    assert_eq!(state.accounting.unknown_usage.values().sum::<u64>(), 100);
    let restored: DevelopmentSchedules =
        serde_json::from_slice(&serde_json::to_vec(&state).unwrap()).unwrap();
    restored.validate().unwrap();
    assert!(restored.progress["manual"].circuit_open);
    assert_eq!(restored.progress["manual"].completed_frontier, 0);
    let mut restored = restored;
    let policy = restored.schedules["manual"].clone();
    restored.configure(policy, 10).unwrap();
    let lease = restored.claim(&first, 11).unwrap();
    restored
        .finish(&lease, Usage::Known(2), "completed-prefix".into(), 12)
        .unwrap();
    assert_eq!(restored.progress["manual"].completed_frontier, 2);
    assert_eq!(restored.accounting.unknown_usage.values().sum::<u64>(), 100);
}
#[tokio::test]
async fn real_actor_registered_provider_publication_and_restart() {
    let endpoint = std::env::var("HM_DEVELOPMENT_PROVIDER_ENDPOINT")
        .expect("actual provider endpoint required");
    let model =
        std::env::var("HM_DEVELOPMENT_PROVIDER_MODEL").expect("actual provider model required");
    let dir = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(dir.path())).await.unwrap();
    let content = b"The calibration vessel serial is Zircon-58. Its operating pressure is 24 kPa.";
    memory(
        &actor,
        "source",
        MemoryCommand::Source {
            source: MemorySource {
                id: "calibration".into(),
                digest: digest_bytes(content),
                content: content.to_vec(),
                locator: "measurement:calibration".into(),
                occurred_at_ns: Some(1791288000123456789),
                recorded_at_ns: 1791288000987654321,
                tombstoned: false,
            },
        },
    )
    .await;
    memory(
        &actor,
        "worker",
        MemoryCommand::RegisterWorker {
            capability: WorkerCapability {
                id: "extractor-capability".into(),
                scope: scope(),
                principal: worker_scope(),
                worker_id: "extractor".into(),
                revision: 1,
                revoked: false,
                allowed_kinds: BTreeSet::from([DevelopmentKind::Extraction]),
                source_ids: BTreeSet::from(["calibration".into()]),
                record_ids: BTreeSet::new(),
                new_record_ids: BTreeSet::from(["calibration-fact".into()]),
                session_id: "calibration-session".into(),
                conversation: "calibration-conversation".into(),
                lease: DevelopmentLease {
                    id: "extraction-lease".into(),
                    attempt: 1,
                    expires_at_ns: i64::MAX,
                },
                budget: DevelopmentBudget {
                    reserved_tokens: 8192,
                    max_input_bytes: 65536,
                    max_output_bytes: 65536,
                    max_mutations: 2,
                },
            },
        },
    )
    .await;
    action(
        &actor,
        "disabled",
        SchedulerAction::Configure {
            schedule: schedule("disabled", ScheduleMode::Disabled),
        },
    )
    .await;
    action(
        &actor,
        "manual",
        SchedulerAction::Configure {
            schedule: schedule("manual", ScheduleMode::Manual),
        },
    )
    .await;
    let job = action(
        &actor,
        "enqueue",
        SchedulerAction::Enqueue {
            schedule_id: "manual".into(),
        },
    )
    .await["job_id"]
        .as_str()
        .unwrap()
        .to_string();
    let provider = HistorianProvider::new(
        endpoint,
        model,
        DevelopmentKind::Extraction,
        None,
        vec!["calibration-fact".into()],
        None,
    )
    .unwrap();
    let worker = HistorianWorker::new(provider, Duration::from_secs(55)).unwrap();
    let mut registry = WorkerRegistry::default();
    registry
        .register("extractor".into(), Arc::new(worker))
        .unwrap();
    let trusted_scope = scope();
    let dispatch = development_scheduler::dispatch(
        &actor,
        &trusted_scope,
        &trusted_scope,
        "dispatch",
        &job,
        &registry,
    );
    let health = async {
        for _ in 0..4 {
            assert!(
                tokio::time::timeout(Duration::from_secs(2), actor.stats())
                    .await
                    .unwrap()
                    .is_ok()
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    };
    let (state, ()) = tokio::join!(dispatch, health);
    let state = state.unwrap();
    assert!(
        matches!(state.jobs[&job].status, DispatchStatus::Complete { .. }),
        "{}",
        serde_json::to_string(&state).unwrap()
    );
    assert_eq!(state.progress["manual"].completed_frontier, 1);
    assert!(state.accounting.spent > 0);
    assert!(state.accounting.unknown_usage.is_empty());
    let memory_state = context_memory::rebuild(&actor, &scope()).await.unwrap();
    let fact = &memory_state.records["calibration-fact"];
    assert_eq!(fact.provenance[0].source_id, "calibration");
    assert_eq!(fact.provenance[0].source_digest, digest_bytes(content));
    assert_eq!(fact.authority, hm_context::Authority::DerivedInference);
    assert!(!fact.content.is_empty());
    action(
        &actor,
        "second-manual",
        SchedulerAction::Configure {
            schedule: schedule("pending", ScheduleMode::Manual),
        },
    )
    .await;
    let pending = action(
        &actor,
        "pending-enqueue",
        SchedulerAction::Enqueue {
            schedule_id: "pending".into(),
        },
    )
    .await["job_id"]
        .as_str()
        .unwrap()
        .to_string();
    let lease: DispatchLease = serde_json::from_value(
        action(
            &actor,
            "pending-claim",
            SchedulerAction::Claim {
                job_id: pending.clone(),
            },
        )
        .await,
    )
    .unwrap();
    action(
        &actor,
        "cancel",
        SchedulerAction::Cancel {
            job_id: pending.clone(),
        },
    )
    .await;
    let state = development_scheduler::inspect(&actor, &scope())
        .await
        .unwrap();
    assert_eq!(state.accounting.unknown_usage.values().sum::<u64>(), 8192);
    let spent = state.accounting.spent;
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(dir.path())).await.unwrap();
    let state = development_scheduler::inspect(&actor, &scope())
        .await
        .unwrap();
    assert_eq!(state.accounting.spent, spent);
    assert_eq!(state.accounting.unknown_usage.values().sum::<u64>(), 8192);
    assert_eq!(state.jobs[&pending].status, DispatchStatus::Cancelled);
    assert_eq!(state.progress["manual"].completed_frontier, 1);
    assert!(matches!(
        state.schedules["disabled"].mode,
        ScheduleMode::Disabled
    ));
    assert!(
        context_memory::rebuild(&actor, &scope())
            .await
            .unwrap()
            .records
            .contains_key("calibration-fact")
    );
    assert_eq!(lease.attempt, 1);
    actor.shutdown().await.unwrap();
}
