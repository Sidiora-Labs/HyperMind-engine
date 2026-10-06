use hm_context::{
    Scope,
    development::*,
    development_schedule::{DevelopmentSchedule, DispatchStatus, ScheduleMode},
    digest_bytes,
};
use hm_core::ActorId;
use hm_cortex::development_curation::CurationAction;
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_memory::{
        self, MemoryCommand, MemoryGrant, MemoryRecord, MemoryRequest, MemorySource, Provenance,
        RecordKind,
    },
    development_scheduler::{self, SchedulerAction, SchedulerRequest, WorkerRegistry},
    development_usage::{self},
    development_workers::{WorkerConfiguration, WorkerOperation, configured_workers},
    usage_service,
};
use std::{collections::BTreeSet, sync::Arc, time::Duration};
fn scope() -> Scope {
    Scope {
        owner_id: "recovery-owner".into(),
        project_id: "calibration".into(),
        workspace_id: None,
    }
}
fn worker_scope() -> Scope {
    Scope {
        owner_id: "recovery-worker".into(),
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
async fn cancelled_original_completion_loss_retains_hold_and_reconciles_after_restart() {
    let endpoint = std::env::var("HM_DEVELOPMENT_PROVIDER_ENDPOINT")
        .expect("actual provider endpoint required");
    let model =
        std::env::var("HM_DEVELOPMENT_PROVIDER_MODEL").expect("actual provider model required");
    let directory = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    let original =
        b"The inspection chamber temperature is 36 Celsius. Its sensor is certified annually.";
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
    for id in ["pressure-unit"] {
        let mut record = MemoryRecord::new(
            id,
            RecordKind::Note,
            String::from_utf8(original.to_vec()).unwrap(),
            100,
        );
        record.provenance = vec![Provenance {
            source_id: "calibration-source".into(),
            source_digest: digest_bytes(original),
            span_start: 0,
            span_end: original.len() as u64,
            quoted_digest: digest_bytes(original),
        }];
        memory(&actor, id, MemoryCommand::Create { record }).await;
    }
    memory(
        &actor,
        "worker-read",
        MemoryCommand::SetGrant {
            grant: MemoryGrant {
                id: "worker-read".into(),
                principal_digest: None,
                principal: worker_scope(),
                record_ids: BTreeSet::from(["pressure-unit".into()]),
                categories: BTreeSet::new(),
                read: true,
                expires_at_ns: None,
                revoked: false,
                revision: 1,
                record_revisions: Default::default(),
            },
        },
    )
    .await;
    memory(
        &actor,
        "capability",
        MemoryCommand::RegisterWorker {
            capability: WorkerCapability {
                id: "curation-capability".into(),
                scope: scope(),
                principal: worker_scope(),
                worker_id: "classify".into(),
                revision: 1,
                revoked: false,
                allowed_kinds: BTreeSet::from([DevelopmentKind::Curation]),
                source_ids: BTreeSet::from(["calibration-source".into()]),
                record_ids: BTreeSet::from(["pressure-unit".into()]),
                new_record_ids: BTreeSet::new(),
                session_id: "original-session".into(),
                conversation: "original-conversation".into(),
                lease: DevelopmentLease {
                    id: "curation-lease".into(),
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
    action(
        &actor,
        "schedule",
        SchedulerAction::Configure {
            schedule: DevelopmentSchedule {
                id: "classify".into(),
                kind: DevelopmentKind::Curation,
                mode: ScheduleMode::Manual,
                worker_id: "classify".into(),
                principal: worker_scope(),
                snapshot: SnapshotRequest {
                    capability_id: "curation-capability".into(),
                    source_ids: BTreeSet::from(["calibration-source".into()]),
                    record_ids: BTreeSet::from(["pressure-unit".into()]),
                },
                reservation: 16384,
                timeout_ms: 60000,
                backoff_ms: 0,
                max_attempts: 2,
                identical_failure_limit: 2,
                provider_policy: Some("ollama".into()),
            },
        },
    )
    .await;
    let job_id = action(
        &actor,
        "enqueue",
        SchedulerAction::Enqueue {
            schedule_id: "classify".into(),
        },
    )
    .await["job_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let configurations = vec![WorkerConfiguration {
        id: "classify".into(),
        operation: WorkerOperation::Curation {
            actions: vec![CurationAction::Classify {
                id: "pressure-unit".into(),
            }],
        },
    }];
    let mut registry = WorkerRegistry::default();
    for (id, worker) in configured_workers(
        actor.clone(),
        &configurations,
        &endpoint,
        &model,
        Duration::from_secs(55),
    )
    .unwrap()
    {
        registry.register(id, worker).unwrap();
    }
    let registry = Arc::new(registry);
    let task = {
        let actor = actor.clone();
        let registry = registry.clone();
        let job_id = job_id.clone();
        tokio::spawn(async move {
            development_scheduler::dispatch(
                &actor,
                &scope(),
                &scope(),
                "dispatch",
                &job_id,
                &registry,
            )
            .await
        })
    };
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            if let Some(proof) =
                development_usage::inspect_attempt(&actor, &scope(), &scope(), &job_id, 1)
                    .await
                    .unwrap()
            {
                assert!(!proof.complete);
                assert_eq!(proof.known_tokens, None);
                break;
            }
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    })
    .await
    .unwrap();
    assert!(
        tokio::time::timeout(Duration::from_secs(2), actor.stats())
            .await
            .unwrap()
            .is_ok()
    );
    let initial = development_usage::inspect_attempt(&actor, &scope(), &scope(), &job_id, 1)
        .await
        .unwrap()
        .unwrap();
    assert!(!initial.complete);
    let original_lease = initial.binding.maintenance_lease.clone();
    action(
        &actor,
        "cancel-current",
        SchedulerAction::Cancel {
            job_id: job_id.clone(),
        },
    )
    .await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    let held = development_scheduler::inspect(&actor, &scope())
        .await
        .unwrap();
    assert_eq!(held.jobs[&job_id].status, DispatchStatus::Cancelled);
    assert_eq!(held.jobs[&job_id].attempt, 1);
    assert_eq!(held.accounting.spent, 0);
    assert_eq!(held.accounting.unknown_usage.values().sum::<u64>(), 16384);
    assert!(
        development_usage::reconcile_unknown(&actor, &scope(), &scope(), &job_id, 1, &initial.id)
            .await
            .is_err()
    );
    for request in ["dispatch", "repeat-before-proof"] {
        assert!(
            development_scheduler::dispatch(
                &actor,
                &scope(),
                &scope(),
                request,
                &job_id,
                &registry
            )
            .await
            .is_err()
        );
    }
    let proof = tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            if let Some(proof) =
                development_usage::inspect_attempt(&actor, &scope(), &scope(), &job_id, 1)
                    .await
                    .unwrap()
            {
                if proof.complete {
                    break proof;
                }
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let observations = development_usage::inspect(&actor, &scope(), &scope())
        .await
        .unwrap();
    assert_eq!(observations.len(), 1);
    let original_response = observations[0].clone();
    assert_eq!(original_response.binding.call_ordinal, 1);
    assert_eq!(original_response.binding.maintenance_lease, original_lease);
    assert_eq!(original_response.binding.job_id, job_id);
    assert_eq!(original_response.binding.attempt, 1);
    assert!(original_response.accepted_response);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&original_response.snapshot.original_bytes)
            .unwrap(),
        original_response.snapshot.raw
    );
    let actual = original_response
        .snapshot
        .tokens
        .input
        .unwrap()
        .checked_add(original_response.snapshot.tokens.output.unwrap())
        .unwrap();
    assert!(actual > 0);
    assert_eq!(proof.known_tokens, Some(actual));
    assert_eq!(proof.observation_ids, vec![original_response.id.clone()]);
    assert_eq!(
        development_scheduler::inspect(&actor, &scope())
            .await
            .unwrap()
            .accounting
            .unknown_usage
            .values()
            .sum::<u64>(),
        16384
    );
    assert_eq!(
        context_memory::rebuild(&actor, &scope())
            .await
            .unwrap()
            .records["pressure-unit"]
            .revision,
        1
    );
    drop(registry);
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    let restored = development_usage::inspect_attempt(&actor, &scope(), &scope(), &job_id, 1)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(restored.id, proof.id);
    assert_eq!(restored.binding.maintenance_lease, original_lease);
    assert_eq!(restored.known_tokens, Some(actual));
    let saved = development_usage::inspect(&actor, &scope(), &scope())
        .await
        .unwrap();
    assert_eq!(saved.len(), 1);
    assert_eq!(
        saved[0].snapshot.original_bytes,
        original_response.snapshot.original_bytes
    );
    let state = development_scheduler::inspect(&actor, &scope())
        .await
        .unwrap();
    assert_eq!(state.jobs[&job_id].status, DispatchStatus::Cancelled);
    assert_eq!(state.jobs[&job_id].attempt, 1);
    assert_eq!(state.accounting.spent, 0);
    assert_eq!(state.accounting.unknown_usage.values().sum::<u64>(), 16384);
    let query = usage_service::RollupQuery {
        session_id: None,
        turn_id: None,
        job_id: Some(job_id.clone()),
        provider_id: None,
        start_ms: None,
        end_ms: None,
    };
    let rollup = usage_service::rollup(&actor, &scope(), &scope(), query.clone())
        .await
        .unwrap();
    assert_eq!(rollup.known_tokens, 0);
    assert_eq!(rollup.held_tokens, 16384);
    assert_eq!(rollup.observed_calls, 1);
    let mut registry = WorkerRegistry::default();
    for (id, worker) in configured_workers(
        actor.clone(),
        &configurations,
        &endpoint,
        &model,
        Duration::from_secs(55),
    )
    .unwrap()
    {
        registry.register(id, worker).unwrap();
    }
    for request in ["dispatch", "repeat-after-restart"] {
        assert!(
            development_scheduler::dispatch(
                &actor,
                &scope(),
                &scope(),
                request,
                &job_id,
                &registry
            )
            .await
            .is_err()
        );
    }
    for (attempt, id) in [
        (2, proof.id.as_str()),
        (1, "authored-zero"),
        (1, initial.id.as_str()),
    ] {
        assert!(
            development_usage::reconcile_unknown(&actor, &scope(), &scope(), &job_id, attempt, id)
                .await
                .is_err()
        );
    }
    assert!(
        development_usage::reconcile_unknown(
            &actor,
            &scope(),
            &worker_scope(),
            &job_id,
            1,
            &proof.id
        )
        .await
        .is_err()
    );
    development_usage::reconcile_unknown(&actor, &scope(), &scope(), &job_id, 1, &proof.id)
        .await
        .unwrap();
    development_usage::reconcile_unknown(&actor, &scope(), &scope(), &job_id, 1, &proof.id)
        .await
        .unwrap();
    let state = development_scheduler::inspect(&actor, &scope())
        .await
        .unwrap();
    assert_eq!(state.accounting.spent, actual);
    assert!(state.accounting.unknown_usage.is_empty());
    assert_eq!(state.jobs[&job_id].attempt, 1);
    assert_eq!(state.jobs[&job_id].status, DispatchStatus::Cancelled);
    let settlements = development_scheduler::observed_settlements(&actor, &scope(), &scope())
        .await
        .unwrap();
    assert_eq!(settlements.len(), 1);
    assert_eq!(settlements[0].actual_tokens, actual);
    let rollup = usage_service::rollup(&actor, &scope(), &scope(), query)
        .await
        .unwrap();
    assert_eq!(rollup.known_tokens, actual);
    assert_eq!(rollup.held_tokens, 0);
    assert_eq!(rollup.observed_calls, 1);
    assert_eq!(rollup.attributions.len(), 1);
    assert_eq!(
        development_usage::inspect(&actor, &scope(), &scope())
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(
        usage_service::inspect(&actor, &scope(), &scope())
            .await
            .unwrap()
            .reservations
            .is_empty()
    );
    assert_eq!(
        context_memory::rebuild(&actor, &scope())
            .await
            .unwrap()
            .records["pressure-unit"]
            .revision,
        1
    );
    drop(registry);
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    development_usage::reconcile_unknown(&actor, &scope(), &scope(), &job_id, 1, &proof.id)
        .await
        .unwrap();
    assert_eq!(
        development_scheduler::inspect(&actor, &scope())
            .await
            .unwrap()
            .accounting
            .spent,
        actual
    );
    assert_eq!(
        development_usage::inspect(&actor, &scope(), &scope())
            .await
            .unwrap()
            .len(),
        1
    );
    actor.shutdown().await.unwrap();
}
