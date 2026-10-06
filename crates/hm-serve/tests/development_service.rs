use hm_context::{ContextError, Scope, development::*, development_schedule::*, digest_bytes};
use hm_core::ActorId;
use hm_cortex::development_historian::HistorianProvider;
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_memory::{self, MemoryCommand, MemoryError, MemoryRequest, MemorySource},
    development_historian::HistorianWorker,
    development_service::{
        DevelopmentAction, DevelopmentRequest, DevelopmentService, ServiceSchedule,
    },
};
use std::{collections::BTreeSet, sync::Arc, time::Duration};
fn scope() -> Scope {
    Scope {
        owner_id: "service-owner".into(),
        project_id: "laboratory".into(),
        workspace_id: None,
    }
}
fn config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.into(),
        actor: ActorId::new(62),
        user: [6; 16],
        kek: [7; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}
fn request(id: &str, action: DevelopmentAction) -> DevelopmentRequest {
    DevelopmentRequest {
        version: 1,
        scope: scope(),
        request_id: id.into(),
        action,
    }
}
#[tokio::test]
async fn real_provider_scoped_dispatch_rejection_revocation_and_restart() {
    let endpoint = std::env::var("HM_DEVELOPMENT_PROVIDER_ENDPOINT")
        .expect("actual provider endpoint required");
    let model =
        std::env::var("HM_DEVELOPMENT_PROVIDER_MODEL").expect("actual provider model required");
    let directory = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    let content = b"The calibration vessel serial is Zircon-58. Its operating pressure is 24 kPa.";
    context_memory::execute(
        &actor,
        &scope(),
        &scope(),
        MemoryRequest {
            version: 1,
            scope: scope(),
            request_id: "source".into(),
            command: MemoryCommand::Source {
                source: MemorySource {
                    id: "calibration".into(),
                    digest: digest_bytes(content),
                    content: content.to_vec(),
                    locator: "measurement:calibration".into(),
                    occurred_at_ns: None,
                    recorded_at_ns: 200,
                    tombstoned: false,
                },
            },
        },
    )
    .await
    .unwrap();
    let empty = DevelopmentService::new(scope(), vec![]).unwrap();
    let register = DevelopmentAction::Register {
        worker_id: "extractor".into(),
        capability_id: "extractor-capability".into(),
        session_id: "session".into(),
        conversation: "conversation".into(),
        source_ids: BTreeSet::from(["calibration".into()]),
        record_ids: BTreeSet::new(),
        new_record_ids: BTreeSet::from(["calibration-fact".into()]),
        budget: DevelopmentBudget {
            reserved_tokens: 8192,
            max_input_bytes: 65536,
            max_output_bytes: 65536,
            max_mutations: 2,
        },
        lease_ms: 600_000,
    };
    assert!(matches!(
        empty
            .execute(&actor, &scope(), request("missing", register.clone()))
            .await,
        Err(MemoryError::Context(ContextError::Unavailable(_)))
    ));
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
    let service =
        DevelopmentService::new(scope(), vec![("extractor".into(), Arc::new(worker))]).unwrap();
    let mut foreign = scope();
    foreign.owner_id = "other-owner".into();
    let tail = actor.stats().await.unwrap().applied.last_lsn;
    assert!(
        service
            .execute(&actor, &foreign, request("foreign", register.clone()))
            .await
            .is_err()
    );
    assert!(service.inspect(&actor, &foreign).await.is_err());
    assert!(
        service
            .execute(
                &actor,
                &foreign,
                request(
                    "review",
                    DevelopmentAction::Review {
                        decision: ProposalDecision {
                            request_id: "review".into(),
                            proposal_id: "unknown".into(),
                            expected_revision: 1,
                            expected_digest: "0".repeat(64),
                            decision: ProposalDecisionKind::Accept
                        }
                    }
                )
            )
            .await
            .is_err()
    );
    assert_eq!(actor.stats().await.unwrap().applied.last_lsn, tail);
    let registered = service
        .execute(&actor, &scope(), request("register", register))
        .await
        .unwrap();
    let capability: WorkerCapability =
        serde_json::from_value(registered["capability"].clone()).unwrap();
    assert_ne!(capability.principal, scope());
    assert_eq!(
        capability.allowed_kinds,
        BTreeSet::from([DevelopmentKind::Extraction])
    );
    let schedule = ServiceSchedule {
        id: "manual".into(),
        mode: ScheduleMode::Manual,
        worker_id: "extractor".into(),
        snapshot: SnapshotRequest {
            capability_id: capability.id.clone(),
            source_ids: capability.source_ids.clone(),
            record_ids: BTreeSet::new(),
        },
        reservation: 8192,
        timeout_ms: 60_000,
        backoff_ms: 0,
        max_attempts: 2,
        identical_failure_limit: 2,
    };
    service
        .execute(
            &actor,
            &scope(),
            request(
                "configure",
                DevelopmentAction::Configure {
                    schedule: schedule.clone(),
                },
            ),
        )
        .await
        .unwrap();
    let job = service
        .execute(
            &actor,
            &scope(),
            request(
                "enqueue",
                DevelopmentAction::Enqueue {
                    schedule_id: "manual".into(),
                },
            ),
        )
        .await
        .unwrap()["job_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let owner = scope();
    let dispatch = service.execute(
        &actor,
        &owner,
        request(
            "dispatch",
            DevelopmentAction::Dispatch {
                job_id: job.clone(),
            },
        ),
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
    let (result, ()) = tokio::join!(dispatch, health);
    let state: DevelopmentSchedules = serde_json::from_value(result.unwrap()).unwrap();
    assert!(
        matches!(state.jobs[&job].status, DispatchStatus::Complete { .. }),
        "{}",
        serde_json::to_string(&state).unwrap()
    );
    assert!(state.accounting.spent > 0);
    let memory = context_memory::rebuild(&actor, &scope()).await.unwrap();
    let fact = &memory.records["calibration-fact"];
    assert_eq!(fact.authority, hm_context::Authority::DerivedInference);
    assert_eq!(fact.provenance[0].source_digest, digest_bytes(content));
    let mut pending_schedule = schedule;
    pending_schedule.id = "pending-manual".into();
    service
        .execute(
            &actor,
            &scope(),
            request(
                "configure-pending",
                DevelopmentAction::Configure {
                    schedule: pending_schedule,
                },
            ),
        )
        .await
        .unwrap();
    let pending = service
        .execute(
            &actor,
            &scope(),
            request(
                "pending",
                DevelopmentAction::Enqueue {
                    schedule_id: "pending-manual".into(),
                },
            ),
        )
        .await
        .unwrap()["job_id"]
        .as_str()
        .unwrap()
        .to_owned();
    service
        .execute(
            &actor,
            &scope(),
            request(
                "cancel",
                DevelopmentAction::Cancel {
                    job_id: pending.clone(),
                },
            ),
        )
        .await
        .unwrap();
    service
        .execute(
            &actor,
            &scope(),
            request(
                "revoke",
                DevelopmentAction::Revoke {
                    capability_id: capability.id,
                    expected_revision: 1,
                },
            ),
        )
        .await
        .unwrap();
    assert!(
        service
            .execute(
                &actor,
                &scope(),
                request(
                    "denied",
                    DevelopmentAction::Enqueue {
                        schedule_id: "manual".into()
                    }
                )
            )
            .await
            .is_err()
    );
    let view = service.inspect(&actor, &scope()).await.unwrap();
    assert_eq!(view["runtime_workers"][0]["registered"], true);
    assert_eq!(
        view["scheduler"]["jobs"][pending]["status"]["state"],
        "cancelled"
    );
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    let view = service.inspect(&actor, &scope()).await.unwrap();
    assert_eq!(
        view["capabilities"]["extractor-capability"]["revoked"],
        true
    );
    assert!(
        context_memory::rebuild(&actor, &scope())
            .await
            .unwrap()
            .records
            .contains_key("calibration-fact")
    );
    actor.shutdown().await.unwrap();
}
