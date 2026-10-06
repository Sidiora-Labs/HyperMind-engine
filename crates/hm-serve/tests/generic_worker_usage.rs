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
async fn original_multicall_worker_settles_one_reservation_and_survives_restart() {
    let endpoint = std::env::var("HM_DEVELOPMENT_PROVIDER_ENDPOINT")
        .expect("actual provider endpoint required");
    let model =
        std::env::var("HM_DEVELOPMENT_PROVIDER_MODEL").expect("actual provider model required");
    let directory = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    let original = b"The station measures pressure in kPa. Its calibration records are retained.";
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
    for id in ["pressure-unit", "calibration-log"] {
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
                record_ids: BTreeSet::from(["pressure-unit".into(), "calibration-log".into()]),
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
                record_ids: BTreeSet::from(["pressure-unit".into(), "calibration-log".into()]),
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
                    record_ids: BTreeSet::from(["pressure-unit".into(), "calibration-log".into()]),
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
            actions: vec![
                CurationAction::Classify {
                    id: "pressure-unit".into(),
                },
                CurationAction::Classify {
                    id: "calibration-log".into(),
                },
            ],
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
    task.await.unwrap().unwrap();
    let observations = development_usage::inspect(&actor, &scope(), &scope())
        .await
        .unwrap();
    assert_eq!(observations.len(), 2);
    let mut ordinals: Vec<_> = observations
        .iter()
        .map(|o| o.binding.call_ordinal)
        .collect();
    ordinals.sort();
    assert_eq!(ordinals, vec![1, 2]);
    let mut total = 0u64;
    for observation in &observations {
        assert!(observation.accepted_response);
        assert_eq!(observation.binding.job_id, job_id);
        assert_eq!(observation.binding.attempt, 1);
        assert_eq!(observation.binding.model_id, model);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&observation.snapshot.original_bytes)
                .unwrap(),
            observation.snapshot.raw
        );
        total += observation.snapshot.tokens.input.unwrap()
            + observation.snapshot.tokens.output.unwrap();
    }
    let proof = development_usage::inspect_attempt(&actor, &scope(), &scope(), &job_id, 1)
        .await
        .unwrap()
        .unwrap();
    assert!(proof.complete);
    assert_eq!(proof.known_tokens, Some(total));
    assert_eq!(proof.observation_ids.len(), 2);
    let state = development_scheduler::inspect(&actor, &scope())
        .await
        .unwrap();
    assert!(matches!(
        state.jobs[&job_id].status,
        DispatchStatus::Complete { .. }
    ));
    assert_eq!(state.accounting.spent, total);
    assert!(state.accounting.unknown_usage.is_empty());
    let settlements = development_scheduler::observed_settlements(&actor, &scope(), &scope())
        .await
        .unwrap();
    assert_eq!(settlements.len(), 1);
    assert_eq!(settlements[0].actual_tokens, total);
    assert_eq!(settlements[0].observation_id, proof.id);
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
    assert_eq!(rollup.known_tokens, total);
    assert_eq!(rollup.observed_calls, 2);
    assert_eq!(rollup.attributions.len(), 1);
    assert_eq!(rollup.observations.len(), 2);
    assert!(
        usage_service::inspect(&actor, &scope(), &scope())
            .await
            .unwrap()
            .reservations
            .is_empty()
    );
    drop(registry);
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    let restored = development_usage::inspect(&actor, &scope(), &scope())
        .await
        .unwrap();
    for old in observations {
        assert_eq!(
            restored
                .iter()
                .find(|o| o.id == old.id)
                .unwrap()
                .snapshot
                .original_bytes,
            old.snapshot.original_bytes
        );
    }
    development_usage::reconcile_unknown(&actor, &scope(), &scope(), &job_id, 1, &proof.id)
        .await
        .unwrap();
    development_usage::reconcile_unknown(&actor, &scope(), &scope(), &job_id, 1, &proof.id)
        .await
        .unwrap();
    assert!(
        development_usage::reconcile_unknown(&actor, &scope(), &scope(), &job_id, 2, &proof.id)
            .await
            .is_err()
    );
    assert!(
        development_usage::reconcile_unknown(
            &actor,
            &scope(),
            &scope(),
            &job_id,
            1,
            "authored-zero"
        )
        .await
        .is_err()
    );
    assert_eq!(
        development_scheduler::inspect(&actor, &scope())
            .await
            .unwrap()
            .accounting
            .spent,
        total
    );
    assert_eq!(
        usage_service::rollup(&actor, &scope(), &scope(), query)
            .await
            .unwrap()
            .known_tokens,
        total
    );
    actor.shutdown().await.unwrap();
}
