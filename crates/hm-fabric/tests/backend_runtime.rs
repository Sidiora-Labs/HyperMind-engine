use hm_context::types::{Scope, digest_bytes};
use hm_fabric::{
    backend_config::{
        BackendConfigError, BackendDescriptor, BusBackendDescriptor, HomeDescriptor,
        MissingHomePolicy, OperationalBackendDescriptor, SelectedOperationalStore,
        select_backend_async, validate_descriptor,
    },
    backend_runtime::{BackendRuntime, PublicationState, RUNTIME_BACKEND_MIGRATIONS, RecordChange},
    bus_contract::StreamFamily,
    bus_nats::{NatsBinding, NatsCredentials},
    postgres::{PostgresDescriptor, PostgresSecretHandle},
    runtime::{RuntimeConfig, RuntimeService, WorkerRequest, worker_from_env},
    supervisor::{Probe, ProcessSpec, RestartPolicy},
};
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
#[derive(Deserialize)]
struct BrokerFixture {
    url: String,
    credentials: NatsCredentials,
    bindings: Vec<NatsBinding>,
}
fn process() -> ProcessSpec {
    ProcessSpec {
        module_id: "selected-digest-worker".into(),
        command: std::env::current_exe().unwrap(),
        args: vec![
            "--exact".into(),
            "actual_worker_entry".into(),
            "--ignored".into(),
            "--nocapture".into(),
        ],
        env: BTreeMap::new(),
        cwd: PathBuf::from("/tmp"),
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
    }
}
fn descriptor(home: &Path, scope: Scope, bus: BusBackendDescriptor) -> BackendDescriptor {
    BackendDescriptor {
        version: 1,
        scope,
        home: HomeDescriptor {
            path: home.to_owned(),
            base: None,
            missing: MissingHomePolicy::Refuse,
        },
        operational: OperationalBackendDescriptor::Sqlite,
        bus,
    }
}
#[test]
#[ignore = "child process entry for actual authenticated runtime worker"]
fn actual_worker_entry() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(worker_from_env())
        .unwrap();
}
async fn selected_local(home: &Path, scope: Scope, bus: BusBackendDescriptor) -> BackendRuntime {
    let selected = select_backend_async(
        validate_descriptor(&descriptor(home, scope, bus)).unwrap(),
        RUNTIME_BACKEND_MIGRATIONS,
        |_| Err(BackendConfigError::SecretUnavailable),
        |_| Err(BackendConfigError::SecretUnavailable),
    )
    .await
    .unwrap();
    BackendRuntime::from_selected(selected).unwrap()
}

#[tokio::test]
async fn actual_selected_sqlite_process_bus_worker_cas_and_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let scope = Scope {
        owner_id: "selected_runtime".into(),
        project_id: "local_backend".into(),
        workspace_id: None,
    };
    let backend = selected_local(
        directory.path(),
        scope.clone(),
        BusBackendDescriptor::ProcessLocal {
            limits: Default::default(),
        },
    )
    .await;
    let records = backend.record_store();
    let record = records
        .compare_exchange(RecordChange {
            key: "operational.effect.adapter".into(),
            expected: None,
            value: serde_json::json!({"state":"prepared"}),
        })
        .unwrap();
    assert!(
        records
            .compare_exchange_batch(vec![
                RecordChange {
                    key: record.key.clone(),
                    expected: Some(record.revision + 1),
                    value: serde_json::json!({"state":"dispatched"})
                },
                RecordChange {
                    key: "operational.atomic.peer".into(),
                    expected: None,
                    value: serde_json::json!({"state":"prepared"})
                }
            ])
            .is_err()
    );
    assert!(records.get("operational.atomic.peer").unwrap().is_none());
    assert_eq!(records.get(&record.key).unwrap().unwrap().revision, 1);
    let mut runtime = RuntimeService::from_backends(
        RuntimeConfig::new(directory.path(), scope.clone(), [61; 32], [73; 32]),
        backend,
    )
    .unwrap();
    let ready = runtime.backend_readiness().unwrap().unwrap();
    assert_eq!(ready.operational_backend, "sqlite");
    assert!(!ready.bus_capabilities.durable);
    assert!(!directory.path().join("runtime.sqlite").exists());
    assert!(!directory.path().join("bus.sqlite").exists());
    runtime.launch_worker(process()).await.unwrap();
    let request = WorkerRequest::digest("selected-local-1", b"real selected local worker".to_vec());
    let result = runtime.execute(request.clone()).await.unwrap();
    assert_eq!(result.digest, digest_bytes(&request.bytes));
    let publications = runtime.backend_publications(100).unwrap();
    assert_eq!(publications.len(), 3);
    assert!(
        publications
            .iter()
            .all(|row| row.state == PublicationState::Published)
    );
    let replay = runtime.events_async(0, 100).await.unwrap();
    assert_eq!(replay.events.len(), 3);
    assert!(replay.gaps.is_empty());
    assert!(replay.events.iter().all(|event| event.payload.len() == 64));
    runtime.shutdown().await.unwrap();
    drop(runtime);
    assert!(records.get(&record.key).is_err());
    let backend = selected_local(
        directory.path(),
        scope.clone(),
        BusBackendDescriptor::ProcessLocal {
            limits: Default::default(),
        },
    )
    .await;
    let mut reopened = RuntimeService::from_backends(
        RuntimeConfig::new(directory.path(), scope, [61; 32], [73; 32]),
        backend,
    )
    .unwrap();
    assert_eq!(reopened.result(&request.id).unwrap(), Some(result.clone()));
    assert_eq!(reopened.execute(request).await.unwrap(), result);
    assert!(
        reopened
            .events_async(0, 100)
            .await
            .unwrap()
            .events
            .is_empty()
    );
    assert!(reopened.backend_readiness().unwrap().unwrap().owner_epoch > ready.owner_epoch);
    reopened.shutdown().await.unwrap();
}

#[tokio::test]
async fn actual_selected_postgres_nats_worker_result_outbox_and_reopen() {
    let postgres_path = std::env::var_os("HM_POSTGRES_TEST_CONFIG")
        .expect("actual PostgreSQL configuration required");
    let pg: Value = serde_json::from_slice(&fs::read(postgres_path).unwrap()).unwrap();
    let broker_path = std::env::var_os("HM_NATS_TEST_SERVER")
        .expect("actual provisioned broker fixture required");
    let broker: BrokerFixture = serde_json::from_slice(&fs::read(broker_path).unwrap()).unwrap();
    let binding = broker
        .bindings
        .iter()
        .find(|binding| binding.family == StreamFamily::ModuleEvents)
        .unwrap()
        .clone();
    let scope = binding.scope.clone();
    let directory = tempfile::tempdir().unwrap();
    let mut config = descriptor(
        directory.path(),
        scope.clone(),
        BusBackendDescriptor::ProvisionedNats {
            server_url: broker.url.clone(),
            binding,
            secret_ref: "runtime_bus".into(),
        },
    );
    config.operational = OperationalBackendDescriptor::Postgres {
        descriptor: PostgresDescriptor {
            scope: scope.clone(),
            host: pg["host"].as_str().unwrap().into(),
            port: pg["port"].as_u64().unwrap() as u16,
            user: pg["user"].as_str().unwrap().into(),
            database: pg["databases"]["bus"].as_str().unwrap().into(),
            schema: pg["schema"].as_str().unwrap().into(),
            namespace: "selected_runtime_v1".into(),
        },
        secret_ref: "runtime_database".into(),
    };
    let password = fs::read_to_string(pg["password_file"].as_str().unwrap())
        .unwrap()
        .trim_end()
        .splitn(5, ':')
        .nth(4)
        .unwrap()
        .to_owned();
    let select = |config: BackendDescriptor, password: String, credentials: NatsCredentials| async move {
        select_backend_async(
            validate_descriptor(&config).unwrap(),
            RUNTIME_BACKEND_MIGRATIONS,
            move |reference| {
                assert_eq!(reference, "runtime_database");
                Ok(PostgresSecretHandle::new(password.clone()))
            },
            move |reference| {
                assert_eq!(reference, "runtime_bus");
                Ok(credentials.clone())
            },
        )
        .await
        .unwrap()
    };
    let selected = select(config.clone(), password.clone(), broker.credentials.clone()).await;
    assert!(matches!(
        selected.operational,
        SelectedOperationalStore::Postgres(_)
    ));
    let backend = BackendRuntime::from_selected(selected).unwrap();
    let records = backend.record_store();
    let mut runtime = RuntimeService::from_backends(
        RuntimeConfig::new(directory.path(), scope.clone(), [61; 32], [73; 32]),
        backend,
    )
    .unwrap();
    let ready = runtime.backend_readiness().unwrap().unwrap();
    assert_eq!(ready.operational_backend, "postgresql");
    assert_eq!(ready.bus_capabilities.backend, "nats");
    assert!(ready.bus_capabilities.distributed);
    for file in [
        "runtime.sqlite",
        "bus.sqlite",
        "backend-operations.sqlite",
        "backend-bus.sqlite",
    ] {
        assert!(!directory.path().join(file).exists());
    }
    runtime.launch_worker(process()).await.unwrap();
    let request = WorkerRequest::digest(
        format!(
            "selected-postgres-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ),
        b"actual postgres nats module execution".to_vec(),
    );
    let result = runtime.execute(request.clone()).await.unwrap();
    assert_eq!(result.digest, digest_bytes(&request.bytes));
    let publications = runtime.backend_publications(100).unwrap();
    assert_eq!(publications.len(), 3);
    assert!(
        publications
            .iter()
            .all(|row| row.state == PublicationState::Published)
    );
    for publication in &publications {
        let sequence = publication.event_sequence.unwrap();
        let replay = runtime.events_async(sequence - 1, 1).await.unwrap();
        assert_eq!(replay.cursor, sequence);
        assert!(replay.gaps.is_empty());
        assert_eq!(replay.events[0].payload, publication.digest.as_bytes());
    }
    assert_eq!(runtime.execute(request.clone()).await.unwrap(), result);
    assert_eq!(runtime.flush_backend_events().await.unwrap(), 0);
    runtime.shutdown().await.unwrap();
    drop(runtime);
    assert!(records.get("runtime.schema.probe").is_err());
    let selected = select(config, password, broker.credentials).await;
    let backend = BackendRuntime::from_selected(selected).unwrap();
    let mut reopened = RuntimeService::from_backends(
        RuntimeConfig::new(directory.path(), scope, [61; 32], [73; 32]),
        backend,
    )
    .unwrap();
    assert_eq!(reopened.result(&request.id).unwrap(), Some(result.clone()));
    assert_eq!(reopened.execute(request).await.unwrap(), result);
    assert_eq!(reopened.backend_publications(100).unwrap().len(), 3);
    assert!(
        reopened
            .backend_readiness()
            .unwrap()
            .unwrap()
            .operational_epoch
            > ready.operational_epoch
    );
    reopened.shutdown().await.unwrap();
}
