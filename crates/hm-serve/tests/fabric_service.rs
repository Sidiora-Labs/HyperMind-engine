use hm_context::{Cursor, Scope, digest_bytes};
use hm_core::ActorId;
use hm_fabric::{
    backend_config::*,
    backend_runtime::{BackendRuntime, RUNTIME_BACKEND_MIGRATIONS},
    runtime::{RuntimeConfig, worker_from_env},
    supervisor::{Probe, ProcessSpec, RestartPolicy},
};
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    fabric_service::*,
};
use std::{collections::BTreeMap, path::Path, sync::Arc, time::Duration};
fn scope() -> Scope {
    Scope {
        owner_id: "fabric-owner".into(),
        project_id: "fabric-project".into(),
        workspace_id: None,
    }
}
fn request(id: &str, action: FabricAction) -> FabricRequest {
    FabricRequest {
        version: 1,
        scope: scope(),
        request_id: id.into(),
        action,
    }
}
async fn build_service(home: &Path) -> FabricService {
    let selected = select_backend_async(
        validate_descriptor(&BackendDescriptor {
            version: 1,
            scope: scope(),
            home: HomeDescriptor {
                path: home.into(),
                base: None,
                missing: MissingHomePolicy::Refuse,
            },
            operational: OperationalBackendDescriptor::Sqlite,
            bus: BusBackendDescriptor::ProcessLocal {
                limits: Default::default(),
            },
        })
        .unwrap(),
        RUNTIME_BACKEND_MIGRATIONS,
        |_| Err(BackendConfigError::SecretUnavailable),
        |_| Err(BackendConfigError::SecretUnavailable),
    )
    .await
    .unwrap();
    let mut env = BTreeMap::new();
    env.insert(
        "FABRIC_TEST_PID".into(),
        home.join("worker.pid").display().to_string(),
    );
    let worker = ProcessSpec {
        module_id: "native-digest".into(),
        command: std::env::current_exe().unwrap(),
        args: vec![
            "--exact".into(),
            "actual_worker_entry".into(),
            "--ignored".into(),
            "--nocapture".into(),
        ],
        env,
        cwd: home.into(),
        readiness: Probe::ProcessAlive,
        health: Probe::ProcessAlive,
        readiness_timeout: Duration::from_secs(3),
        shutdown_timeout: Duration::from_millis(100),
        stderr_bytes: 8192,
        drain_message: None,
        restart: RestartPolicy {
            max_restarts: 0,
            initial_backoff: Duration::ZERO,
            max_backoff: Duration::ZERO,
        },
    };
    FabricService::start(
        RuntimeConfig::new(home, scope(), [7; 32], [8; 32]),
        BackendRuntime::from_selected(selected).unwrap(),
        worker,
        TrustedMetadata::default(),
    )
    .await
    .unwrap()
}
#[test]
#[ignore = "actual authenticated worker subprocess entry"]
fn actual_worker_entry() {
    std::fs::write(
        std::env::var("FABRIC_TEST_PID").unwrap(),
        std::process::id().to_string(),
    )
    .unwrap();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(worker_from_env())
        .unwrap();
}
#[tokio::test]
async fn selected_service_worker_cancel_and_reopen() {
    let home = tempfile::tempdir().unwrap();
    let ledger = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(ActorConfig {
        actor_directory: ledger.path().into(),
        actor: ActorId::new(74),
        user: [3; 16],
        kek: [4; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    })
    .await
    .unwrap();
    let service = Arc::new(build_service(home.path()).await);
    let dispatch = request(
        "digest-one",
        FabricAction::Dispatch {
            operation: "digest".into(),
            bytes: b"native bytes".to_vec(),
            timeout_ms: 10000,
        },
    );
    let result = service
        .execute(&actor, &scope(), dispatch.clone())
        .await
        .unwrap();
    assert_eq!(result["result"]["digest"], digest_bytes(b"native bytes"));
    assert_eq!(
        service
            .execute(&actor, &scope(), dispatch.clone())
            .await
            .unwrap(),
        result
    );
    let foreign = Scope {
        owner_id: "foreign".into(),
        ..scope()
    };
    assert!(
        service
            .inspect(&foreign, Cursor::default(), 16)
            .await
            .is_err()
    );
    let mut conflict = dispatch.clone();
    if let FabricAction::Dispatch { bytes, .. } = &mut conflict.action {
        bytes.push(1);
    }
    assert!(service.execute(&actor, &scope(), conflict).await.is_err());
    let inspected = service
        .inspect(&scope(), Cursor::default(), 16)
        .await
        .unwrap();
    assert_eq!(
        inspected["descriptors"]["effect_authority"],
        "selected_operational"
    );
    let limit = inspected["descriptors"]["operations"][0]["max_input_bytes"]
        .as_u64()
        .unwrap() as usize;
    let oversized = request(
        "oversized",
        FabricAction::Dispatch {
            operation: "digest".into(),
            bytes: vec![255; limit + 1],
            timeout_ms: 10000,
        },
    );
    assert!(matches!(
        service.execute(&actor, &scope(), oversized).await,
        Err(FabricError::Context(hm_context::ContextError::Capacity))
    ));
    let refused = service
        .inspect(&scope(), Cursor::default(), 16)
        .await
        .unwrap();
    assert_eq!(refused["receipts"], inspected["receipts"]);
    assert_eq!(refused["publications"], inspected["publications"]);
    let reused = service
        .execute(
            &actor,
            &scope(),
            request(
                "oversized",
                FabricAction::Dispatch {
                    operation: "digest".into(),
                    bytes: b"valid".to_vec(),
                    timeout_ms: 10000,
                },
            ),
        )
        .await
        .unwrap();
    assert_eq!(reused["result"]["digest"], digest_bytes(b"valid"));
    assert!(!home.path().join("effects.sqlite").exists());
    service.shutdown(&scope()).await.unwrap();
    drop(service);
    let service = Arc::new(build_service(home.path()).await);
    assert_eq!(
        service.execute(&actor, &scope(), dispatch).await.unwrap(),
        result
    );
    let pid = std::fs::read_to_string(home.path().join("worker.pid")).unwrap();
    assert!(
        std::process::Command::new("/bin/kill")
            .args(["-STOP", pid.trim()])
            .status()
            .unwrap()
            .success()
    );
    let owner = scope();
    let work = service.execute(
        &actor,
        &owner,
        request(
            "cancelled",
            FabricAction::Dispatch {
                operation: "digest".into(),
                bytes: vec![5; 4096],
                timeout_ms: 10000,
            },
        ),
    );
    let control = async {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            assert!(
                tokio::time::Instant::now() < deadline,
                "dispatch did not reach actual dispatched state"
            );
            let view = service
                .inspect(&scope(), Cursor::default(), 16)
                .await
                .unwrap();
            if matches!(view["active_effect"]["phase"].as_str(), Some("dispatched")) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
        actor.stats().await.unwrap();
        service
            .execute(
                &actor,
                &scope(),
                request(
                    "cancel-request",
                    FabricAction::Cancel {
                        effect_id: "cancelled".into(),
                    },
                ),
            )
            .await
            .unwrap()
    };
    let (work, cancel) = tokio::join!(work, control);
    assert!(
        matches!(work, Err(FabricError::Cancelled { .. })),
        "native dispatch outcome: {work:?}"
    );
    assert_eq!(cancel["effect"]["state"], "uncertain");
    assert_eq!(cancel["worker_stopped"], true);
    assert!(matches!(
        service
            .execute(
                &actor,
                &scope(),
                request(
                    "after-cancel",
                    FabricAction::Dispatch {
                        operation: "digest".into(),
                        bytes: b"blocked".to_vec(),
                        timeout_ms: 10000
                    }
                )
            )
            .await,
        Err(FabricError::Runtime(
            hm_fabric::runtime::RuntimeError::WorkerUnavailable
        ))
    ));

    assert!(matches!(
        service
            .execute(
                &actor,
                &scope(),
                request(
                    "reconcile",
                    FabricAction::Reconcile {
                        effect_id: "cancelled".into()
                    }
                )
            )
            .await,
        Err(FabricError::EvidenceUnavailable { .. })
    ));
    service.shutdown(&scope()).await.unwrap();
    drop(service);
    let reopened = build_service(home.path()).await;
    let view = reopened
        .inspect(&scope(), Cursor::default(), 256)
        .await
        .unwrap();
    assert!(
        view["receipts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["intent"]["key"] == "cancelled" && r["intent"]["state"] == "uncertain")
    );
    reopened.shutdown(&scope()).await.unwrap();
}
