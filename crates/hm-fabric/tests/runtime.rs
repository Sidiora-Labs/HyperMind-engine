use hm_context::types::{digest_bytes, Cursor, Scope};
use hm_fabric::{
    effects::{EffectObservation, EffectOutcome, EffectState},
    runtime::*,
    supervisor::{Probe, ProcessSpec, RestartPolicy},
};
use std::{collections::BTreeMap, path::PathBuf, time::Duration};
fn scope() -> Scope {
    Scope {
        owner_id: "runtime-owner".into(),
        project_id: "runtime-project".into(),
        workspace_id: Some("workspace".into()),
    }
}
fn config(root: &std::path::Path) -> RuntimeConfig {
    RuntimeConfig::new(root, scope(), [31; 32], [57; 32])
}
fn process() -> ProcessSpec {
    ProcessSpec {
        module_id: "digest-worker".into(),
        command: std::env::current_exe().unwrap(),
        args: vec![
            "--exact".into(),
            "worker_process_entry".into(),
            "--nocapture".into(),
            "--ignored".into(),
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
#[test]
#[ignore = "child process entry for the production worker service"]
fn worker_process_entry() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(worker_from_env())
        .unwrap();
}
#[tokio::test]
async fn real_worker_roundtrip_receipts_and_restart_replay() {
    let dir = tempfile::tempdir().unwrap();
    let mut runtime = RuntimeService::open(config(dir.path())).unwrap();
    assert!(RuntimeService::open(config(dir.path())).is_err());
    let first_epoch = runtime.writer_epoch();
    let launch = runtime.launch_worker(process()).await.unwrap();
    assert_eq!(launch.module_id, "digest-worker");
    let request = WorkerRequest::digest("digest-1", b"hello".to_vec());
    let result = runtime.execute(request.clone()).await.unwrap();
    assert_eq!(
        result.digest,
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    );
    assert_eq!(runtime.execute(request.clone()).await.unwrap(), result);
    assert_eq!(
        runtime.effect("digest-1").unwrap().unwrap().state,
        EffectState::Terminal
    );
    let receipts = runtime.receipts(Cursor::default(), 100).unwrap();
    assert_eq!(
        receipts.iter().map(|r| r.intent.state).collect::<Vec<_>>(),
        vec![
            EffectState::Prepared,
            EffectState::Dispatched,
            EffectState::Terminal
        ]
    );
    let events = runtime.events(0, 100).unwrap();
    assert_eq!(events.len(), 3);
    for (event, receipt) in events.iter().zip(&receipts) {
        assert_eq!(
            serde_json::from_slice::<hm_fabric::effects::EffectReceipt>(&event.payload).unwrap(),
            *receipt
        )
    }
    assert_eq!(runtime.relay_receipts().unwrap(), 0);
    assert!(runtime.drain().unwrap());
    assert!(runtime
        .submit(WorkerRequest::digest("after-drain", vec![]))
        .is_err());
    runtime.shutdown().await.unwrap();
    drop(runtime);
    let mut reopened = RuntimeService::open(config(dir.path())).unwrap();
    assert!(reopened.writer_epoch() > first_epoch);
    assert_eq!(reopened.events(0, 100).unwrap().len(), 3);
    assert_eq!(reopened.execute(request).await.unwrap(), result);
    assert_eq!(
        reopened.effect("digest-1").unwrap().unwrap().state,
        EffectState::Terminal
    );
    assert!(reopened
        .execute(WorkerRequest::digest("digest-1", b"conflict".to_vec()))
        .await
        .is_err());
}
#[tokio::test]
async fn interrupted_dispatch_recovers_uncertain_without_redispatch() {
    let dir = tempfile::tempdir().unwrap();
    let request = WorkerRequest::digest("interrupted", vec![9; 4096]);
    let mut runtime = RuntimeService::open(config(dir.path())).unwrap();
    runtime.launch_worker(process()).await.unwrap();
    runtime.submit(request.clone()).unwrap();
    assert_eq!(
        runtime.dispatch_next().await.unwrap().as_deref(),
        Some("interrupted")
    );
    assert_eq!(
        runtime.effect("interrupted").unwrap().unwrap().state,
        EffectState::Dispatched
    );
    drop(runtime);
    let mut recovered = RuntimeService::open(config(dir.path())).unwrap();
    let effect = recovered.effect("interrupted").unwrap().unwrap();
    assert_eq!(effect.state, EffectState::Uncertain);
    assert!(matches!(
        recovered.submit(request.clone()),
        Err(RuntimeError::Uncertain(_))
    ));
    recovered.launch_worker(process()).await.unwrap();
    assert_eq!(recovered.dispatch_next().await.unwrap(), None);
    assert!(matches!(
        recovered.execute(request).await,
        Err(RuntimeError::Uncertain(_))
    ));
    let terminal = recovered
        .reconcile(
            "interrupted",
            EffectObservation {
                outcome: EffectOutcome::NotApplied,
                evidence: "operator accepted reconciliation after checking the interrupted worker"
                    .into(),
            },
        )
        .unwrap();
    assert_eq!(terminal.state, EffectState::Terminal);
    assert_eq!(recovered.events(0, 100).unwrap().len(), 4);
    recovered.shutdown().await.unwrap();
}
#[tokio::test]
async fn authenticated_pins_credits_and_drain_control_dispatch() {
    let dir = tempfile::tempdir().unwrap();
    let mut runtime = RuntimeService::open(config(dir.path())).unwrap();
    runtime.launch_worker(process()).await.unwrap();
    let mut wrong = WorkerRequest::digest("wrong-pin", vec![]);
    wrong.pin = hm_fabric::roles::RoleVersion::new(b"other", b"other");
    assert!(runtime.submit(wrong).is_err());
    assert!(runtime.effect("wrong-pin").unwrap().is_none());
    for i in 0..5 {
        runtime
            .submit(WorkerRequest::digest(format!("request-{i}"), vec![i]))
            .unwrap();
    }
    for i in 0..4 {
        assert_eq!(
            runtime.dispatch_next().await.unwrap(),
            Some(format!("request-{i}"))
        );
    }
    assert_eq!(runtime.dispatch_next().await.unwrap(), None);
    let first = runtime.receive_result().await.unwrap();
    assert_eq!(first.digest, digest_bytes(&[0]));
    assert_eq!(
        runtime.dispatch_next().await.unwrap().as_deref(),
        Some("request-4")
    );
    assert!(!runtime.drain().unwrap());
    for _ in 0..4 {
        runtime.receive_result().await.unwrap();
    }
    assert!(runtime.drain().unwrap());
    runtime.shutdown().await.unwrap();
}
#[test]
fn scope_and_aliases_cannot_create_another_operational_owner() {
    let dir = tempfile::tempdir().unwrap();
    let runtime = RuntimeService::open(config(dir.path())).unwrap();
    drop(runtime);
    let mut foreign = config(dir.path());
    foreign.scope.project_id = "other-project".into();
    assert!(RuntimeService::open(foreign).is_err());
    let alias = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(
        dir.path().join("effects.sqlite"),
        alias.path().join("effects.sqlite"),
    )
    .unwrap();
    assert!(RuntimeService::open(config(alias.path())).is_err());
}
