use hm_context::{
    Scope,
    development::*,
    development_schedule::{DevelopmentSchedule, DispatchStatus, ScheduleMode},
    digest_bytes,
};
use hm_core::ActorId;
use hm_cortex::development_historian::HistorianProvider;
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_memory::{self, MemoryCommand, MemoryRequest, MemorySource},
    development_scheduler::{self, SchedulerAction, SchedulerRequest, WorkerRegistry},
    development_usage::{self, ObservedHistorianWorker},
    usage_service,
};
use std::{collections::BTreeSet, sync::Arc, time::Duration};
fn scope() -> Scope {
    Scope {
        owner_id: "original-usage-owner".into(),
        project_id: "calibration".into(),
        workspace_id: None,
    }
}
fn worker_scope() -> Scope {
    Scope {
        owner_id: "original-usage-worker".into(),
        project_id: "calibration".into(),
        workspace_id: None,
    }
}
fn config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.into(),
        actor: ActorId::new(63),
        user: [3; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
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
#[tokio::test]
async fn cancelled_original_response_reconciles_existing_reservation_once_after_restart() {
    let endpoint = std::env::var("HM_DEVELOPMENT_PROVIDER_ENDPOINT")
        .expect("actual provider endpoint required");
    let model =
        std::env::var("HM_DEVELOPMENT_PROVIDER_MODEL").expect("actual provider model required");
    let directory = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    let owner = scope();
    let original = b"The calibration vessel serial is Zircon-58. Its pressure is 24 kPa.";
    memory(
        &actor,
        "source",
        MemoryCommand::Source {
            source: MemorySource {
                id: "calibration-source".into(),
                digest: digest_bytes(original),
                content: original.to_vec(),
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
        "capability",
        MemoryCommand::RegisterWorker {
            capability: WorkerCapability {
                id: "original-capability".into(),
                scope: scope(),
                principal: worker_scope(),
                worker_id: "original-extractor".into(),
                revision: 1,
                revoked: false,
                allowed_kinds: BTreeSet::from([DevelopmentKind::Extraction]),
                source_ids: BTreeSet::from(["calibration-source".into()]),
                record_ids: BTreeSet::new(),
                new_record_ids: BTreeSet::from(["cancelled-fact".into()]),
                session_id: "original-session".into(),
                conversation: "original-conversation".into(),
                lease: DevelopmentLease {
                    id: "original-capability-lease".into(),
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
        "schedule",
        SchedulerAction::Configure {
            schedule: DevelopmentSchedule {
                id: "original-extraction".into(),
                kind: DevelopmentKind::Extraction,
                mode: ScheduleMode::Manual,
                worker_id: "original-extractor".into(),
                principal: worker_scope(),
                snapshot: SnapshotRequest {
                    capability_id: "original-capability".into(),
                    source_ids: BTreeSet::from(["calibration-source".into()]),
                    record_ids: BTreeSet::new(),
                },
                reservation: 8192,
                timeout_ms: 60_000,
                backoff_ms: 0,
                max_attempts: 3,
                identical_failure_limit: 2,
                provider_policy: None,
            },
        },
    )
    .await;
    let job_id = action(
        &actor,
        "enqueue",
        SchedulerAction::Enqueue {
            schedule_id: "original-extraction".into(),
        },
    )
    .await["job_id"]
        .as_str()
        .unwrap()
        .to_string();
    let provider = HistorianProvider::new(
        endpoint,
        model.clone(),
        DevelopmentKind::Extraction,
        None,
        vec!["cancelled-fact".into()],
        None,
    )
    .unwrap();
    let worker =
        ObservedHistorianWorker::new(provider, Duration::from_secs(55), "local-generation".into())
            .unwrap();
    let mut registry = WorkerRegistry::default();
    registry
        .register("original-extractor".into(), Arc::new(worker))
        .unwrap();
    let registry = Arc::new(registry);
    let dispatch = {
        let actor = actor.clone();
        let owner = owner.clone();
        let job_id = job_id.clone();
        let registry = registry.clone();
        tokio::spawn(async move {
            development_scheduler::dispatch(
                &actor,
                &owner,
                &owner,
                "original-dispatch",
                &job_id,
                &registry,
            )
            .await
        })
    };
    let binding = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Some(binding) = development_usage::dispatched_attempts(&actor, &owner, &owner)
                .await
                .unwrap()
                .into_iter()
                .find(|binding| binding.job_id == job_id)
            {
                break binding;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(binding.attempt, 1);
    assert_eq!(binding.model_id, model);
    assert_eq!(binding.worker_id, "original-extractor");
    assert!(
        tokio::time::timeout(Duration::from_secs(2), actor.stats())
            .await
            .unwrap()
            .is_ok()
    );
    action(
        &actor,
        "cancel-original",
        SchedulerAction::Cancel {
            job_id: job_id.clone(),
        },
    )
    .await;
    let state = development_scheduler::inspect(&actor, &owner)
        .await
        .unwrap();
    assert_eq!(state.accounting.unknown_usage.values().sum::<u64>(), 8192);
    assert_eq!(state.accounting.spent, 0);
    assert_eq!(state.jobs[&job_id].status, DispatchStatus::Cancelled);
    assert!(
        tokio::time::timeout(Duration::from_secs(60), dispatch)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    let observations = development_usage::inspect(&actor, &owner, &owner)
        .await
        .unwrap();
    assert_eq!(observations.len(), 1);
    let observation = observations[0].clone();
    assert_eq!(observation.binding, binding);
    assert!(observation.accepted_response);
    let actual =
        observation.snapshot.tokens.input.unwrap() + observation.snapshot.tokens.output.unwrap();
    assert!(actual > 0 && actual < 8192);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&observation.snapshot.original_bytes).unwrap(),
        observation.snapshot.raw
    );
    assert_eq!(observation.snapshot.raw["model"], model);
    assert!(
        !context_memory::rebuild(&actor, &owner)
            .await
            .unwrap()
            .records
            .contains_key("cancelled-fact")
    );
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    let state = development_scheduler::inspect(&actor, &owner)
        .await
        .unwrap();
    assert_eq!(state.accounting.spent, 0);
    assert_eq!(state.accounting.unknown_usage.values().sum::<u64>(), 8192);
    assert_eq!(
        development_usage::inspect(&actor, &owner, &owner)
            .await
            .unwrap()[0]
            .snapshot
            .original_bytes,
        observation.snapshot.original_bytes
    );
    assert!(
        development_usage::reconcile_unknown(
            &actor,
            &owner,
            &owner,
            &job_id,
            1,
            "request-authored-actual-zero"
        )
        .await
        .is_err()
    );
    assert!(
        development_usage::reconcile_unknown(&actor, &owner, &owner, &job_id, 2, &observation.id)
            .await
            .is_err()
    );
    assert!(
        development_usage::reconcile_unknown(
            &actor,
            &owner,
            &worker_scope(),
            &job_id,
            1,
            &observation.id
        )
        .await
        .is_err()
    );
    assert_eq!(
        development_scheduler::inspect(&actor, &owner)
            .await
            .unwrap()
            .accounting
            .unknown_usage
            .values()
            .sum::<u64>(),
        8192
    );
    development_usage::reconcile_unknown(&actor, &owner, &owner, &job_id, 1, &observation.id)
        .await
        .unwrap();
    development_usage::reconcile_unknown(&actor, &owner, &owner, &job_id, 1, &observation.id)
        .await
        .unwrap();
    let state = development_scheduler::inspect(&actor, &owner)
        .await
        .unwrap();
    assert_eq!(state.accounting.spent, actual);
    assert!(state.accounting.unknown_usage.is_empty());
    assert_eq!(state.jobs[&job_id].status, DispatchStatus::Cancelled);
    let settlements = development_scheduler::observed_settlements(&actor, &owner, &owner)
        .await
        .unwrap();
    assert_eq!(settlements.len(), 1);
    assert_eq!(settlements[0].actual_tokens, actual);
    assert_eq!(settlements[0].observation_id, observation.id);
    let parallel = usage_service::inspect(&actor, &owner, &owner)
        .await
        .unwrap();
    assert!(parallel.reservations.is_empty());
    assert!(parallel.observations.is_empty());
    assert_eq!(parallel.accounting.spent, 0);
    let original_rollup = usage_service::rollup(
        &actor,
        &owner,
        &owner,
        usage_service::RollupQuery {
            session_id: None,
            turn_id: None,
            job_id: Some(job_id.clone()),
            provider_id: None,
            start_ms: None,
            end_ms: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(original_rollup.known_tokens, actual);
    assert_eq!(original_rollup.held_tokens, 0);
    assert_eq!(original_rollup.observed_calls, 1);
    assert_eq!(original_rollup.attributions[0].turn_id, binding.plan_id);
    assert_eq!(
        original_rollup.attributions[0].source_ids,
        BTreeSet::from(["calibration-source".into()])
    );
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    development_usage::reconcile_unknown(&actor, &owner, &owner, &job_id, 1, &observation.id)
        .await
        .unwrap();
    assert_eq!(
        development_scheduler::inspect(&actor, &owner)
            .await
            .unwrap()
            .accounting
            .spent,
        actual
    );
    assert_eq!(
        development_usage::dispatched_attempts(&actor, &owner, &owner)
            .await
            .unwrap()
            .len(),
        1
    );
    actor.shutdown().await.unwrap();
}
