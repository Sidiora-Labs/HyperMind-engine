use hm_context::types::{Cursor, Scope, digest_bytes};
use hm_fabric::{
    backend_config::{
        BackendConfigError, BackendDescriptor, BusBackendDescriptor, HomeDescriptor,
        MissingHomePolicy, OperationalBackendDescriptor, select_backend_async, validate_descriptor,
    },
    backend_runtime::{BackendRuntime, RUNTIME_BACKEND_MIGRATIONS},
    effects::{EffectObservation, EffectOutcome, EffectState, EffectStore},
    postgres::{PostgresDescriptor, PostgresSecretHandle},
    runtime::{RuntimeConfig, RuntimeService, WorkerRequest, worker_from_env},
    supervisor::{Probe, ProcessSpec, RestartPolicy},
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
async fn native(home: &Path) -> (BackendRuntime, Scope) {
    let pg: serde_json::Value = serde_json::from_slice(
        &fs::read(
            std::env::var_os("HM_POSTGRES_TEST_CONFIG").expect("real PostgreSQL fixture required"),
        )
        .unwrap(),
    )
    .unwrap();
    let broker: serde_json::Value = serde_json::from_slice(
        &fs::read(std::env::var_os("HM_NATS_TEST_SERVER").expect("bound scope fixture required"))
            .unwrap(),
    )
    .unwrap();
    let scope: Scope = serde_json::from_value(broker["bindings"][0]["scope"].clone()).unwrap();
    let password = fs::read_to_string(pg["password_file"].as_str().unwrap())
        .unwrap()
        .trim_end()
        .splitn(5, ':')
        .nth(4)
        .unwrap()
        .to_owned();
    let descriptor = BackendDescriptor {
        version: 1,
        scope: scope.clone(),
        home: HomeDescriptor {
            path: home.into(),
            base: None,
            missing: MissingHomePolicy::Refuse,
        },
        operational: OperationalBackendDescriptor::Postgres {
            descriptor: PostgresDescriptor {
                scope: scope.clone(),
                host: pg["host"].as_str().unwrap().into(),
                port: pg["port"].as_u64().unwrap() as u16,
                user: pg["user"].as_str().unwrap().into(),
                database: pg["databases"]["bus"].as_str().unwrap().into(),
                schema: pg["schema"].as_str().unwrap().into(),
                namespace: "selected_runtime_v1".into(),
            },
            secret_ref: "database".into(),
        },
        bus: BusBackendDescriptor::ProcessLocal {
            limits: Default::default(),
        },
    };
    let selected = select_backend_async(
        validate_descriptor(&descriptor).unwrap(),
        RUNTIME_BACKEND_MIGRATIONS,
        move |_| Ok(PostgresSecretHandle::new(password.clone())),
        |_| Err(BackendConfigError::SecretUnavailable),
    )
    .await
    .unwrap();
    (BackendRuntime::from_selected(selected).unwrap(), scope)
}
fn effects(backend: &BackendRuntime, scope: &Scope) -> EffectStore {
    EffectStore::from_backend(
        scope.clone(),
        backend.home().to_str().unwrap().into(),
        backend.record_store(),
    )
    .unwrap()
}
#[test]
#[ignore = "actual abrupt owner process entry"]
fn interrupted_effect_entry() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
 let home=PathBuf::from(std::env::var_os("HM_EFFECT_HOME").unwrap());let target=PathBuf::from(std::env::var_os("HM_EFFECT_TARGET").unwrap());
 let (backend,scope)=native(&home).await;let mut store=effects(&backend,&scope);
 let intent=store.prepare(&scope,"external-append","append-file",target.as_os_str().as_encoded_bytes()).unwrap();
 store.begin_dispatch(&scope,&intent.key,intent.version).unwrap();
 let status=std::process::Command::new("python3").args(["-c","import os,sys; f=open(sys.argv[1],'ab'); f.write(b'applied\\n'); f.flush(); os.fsync(f.fileno()); f.close()",target.to_str().unwrap()]).status().unwrap();assert!(status.success());
 std::process::exit(75);
 });
}
#[test]
#[ignore = "actual authenticated worker entry"]
fn worker_entry() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(worker_from_env())
        .unwrap();
}
fn process() -> ProcessSpec {
    ProcessSpec {
        module_id: "selected-effect-worker".into(),
        command: std::env::current_exe().unwrap(),
        args: vec![
            "--exact".into(),
            "worker_entry".into(),
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
#[tokio::test]
async fn real_postgres_interrupt_reconcile_receipts_and_runtime() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("side-effect");
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "interrupted_effect_entry",
            "--ignored",
            "--nocapture",
        ])
        .env("HM_EFFECT_HOME", dir.path())
        .env("HM_EFFECT_TARGET", &target)
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(75));
    assert_eq!(fs::read(&target).unwrap(), b"applied\n");
    let (backend, scope) = native(dir.path()).await;
    let mut store = effects(&backend, &scope);
    let intent = store.get(&scope, "external-append").unwrap().unwrap();
    assert_eq!((intent.state, intent.version), (EffectState::Uncertain, 3));
    assert!(
        store
            .begin_dispatch(&scope, &intent.key, intent.version)
            .is_err()
    );
    assert!(
        store
            .complete(
                &scope,
                &intent.key,
                intent.version,
                EffectObservation {
                    outcome: EffectOutcome::Succeeded,
                    evidence: "observed".into()
                }
            )
            .is_err()
    );
    let terminal = store
        .reconcile(
            &scope,
            &intent.key,
            intent.version,
            EffectObservation {
                outcome: EffectOutcome::Succeeded,
                evidence: digest_bytes(&fs::read(&target).unwrap()),
            },
        )
        .unwrap();
    assert_eq!(terminal.version, 4);
    let receipts = store
        .receipts(
            &scope,
            Cursor {
                epoch: 1,
                sequence: 0,
            },
            10,
        )
        .unwrap();
    assert_eq!(
        receipts.iter().map(|r| r.intent.state).collect::<Vec<_>>(),
        vec![
            EffectState::Prepared,
            EffectState::Dispatched,
            EffectState::Uncertain,
            EffectState::Terminal
        ]
    );
    store
        .acknowledge(&scope, "consumer", receipts[3].cursor)
        .unwrap();
    assert!(
        store
            .acknowledge(&scope, "consumer", receipts[0].cursor)
            .is_err()
    );
    assert!(
        store
            .acknowledge(
                &scope,
                "consumer",
                Cursor {
                    epoch: 1,
                    sequence: 5
                }
            )
            .is_err()
    );
    drop(backend);
    assert!(store.get(&scope, &intent.key).is_err());
    drop(store);
    let (backend, scope) = native(dir.path()).await;
    let store = effects(&backend, &scope);
    assert_eq!(store.get(&scope, &intent.key).unwrap(), Some(terminal));
    assert_eq!(store.acknowledged(&scope, "consumer").unwrap().sequence, 4);
    drop(store);
    let mut runtime = RuntimeService::from_backends(
        RuntimeConfig::new(dir.path(), scope.clone(), [61; 32], [73; 32]),
        backend,
    )
    .unwrap();
    runtime.launch_worker(process()).await.unwrap();
    let request = WorkerRequest::digest(
        "selected-effect-result",
        b"native effect authority".to_vec(),
    );
    let result = runtime.execute(request.clone()).await.unwrap();
    assert_eq!(result.digest, digest_bytes(&request.bytes));
    assert!(runtime.backend_publications(100).unwrap().len() >= 3);
    runtime.shutdown().await.unwrap();
    drop(runtime);
    let (backend, scope) = native(dir.path()).await;
    let store = effects(&backend, &scope);
    assert_eq!(
        store
            .receipts(
                &scope,
                Cursor {
                    epoch: 1,
                    sequence: 0
                },
                100
            )
            .unwrap()
            .len(),
        7
    );
    drop(store);
    let mut runtime = RuntimeService::from_backends(
        RuntimeConfig::new(dir.path(), scope, [61; 32], [73; 32]),
        backend,
    )
    .unwrap();
    assert_eq!(runtime.result(&request.id).unwrap(), Some(result));
    runtime.shutdown().await.unwrap();
    for file in [
        "effects.sqlite",
        "effects.sqlite.effects.lock",
        "backend-operations.sqlite",
    ] {
        assert!(!dir.path().join(file).exists());
    }
    assert_eq!(fs::read(target).unwrap(), b"applied\n");
}
#[test]
fn legacy_sqlite_recovery_compatible() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("effects.sqlite");
    let scope = Scope {
        owner_id: "effect_owner".into(),
        project_id: "legacy".into(),
        workspace_id: None,
    };
    let mut store = EffectStore::open(&path).unwrap();
    let intent = store.prepare(&scope, "effect", "write", b"bytes").unwrap();
    store
        .begin_dispatch(&scope, &intent.key, intent.version)
        .unwrap();
    drop(store);
    let mut store = EffectStore::open(path).unwrap();
    let intent = store.get(&scope, "effect").unwrap().unwrap();
    assert_eq!(intent.state, EffectState::Uncertain);
    store
        .reconcile(
            &scope,
            &intent.key,
            intent.version,
            EffectObservation {
                outcome: EffectOutcome::NotApplied,
                evidence: "inspected actual target".into(),
            },
        )
        .unwrap();
    assert_eq!(
        store
            .receipts(
                &scope,
                Cursor {
                    epoch: 1,
                    sequence: 0
                },
                10
            )
            .unwrap()
            .len(),
        4
    );
}
