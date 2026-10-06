use hm_context::types::{digest_bytes, Scope};
use hm_core::{ActorId, ConversationId, UtcNanos};
use hm_fabric::{
    role_dispatch::dispatch_declared_tool,
    role_store::{DurableRoleRegistry, RoleWork, SourceFence, WorkState},
    role_workers::*,
    roles::RunTerminal,
    supervisor::{Probe, ProcessSpec, RestartPolicy},
    transport::Limits,
};
use hm_ledger::{
    frame::EventKind,
    segment::{AppendRequest, SegmentLog, SegmentLogOptions},
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
fn scope() -> Scope {
    Scope {
        owner_id: "tool-owner".into(),
        project_id: "tool-project".into(),
        workspace_id: None,
    }
}
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as i64
}
fn fresh_source(root: &Path) -> SourceFence {
    let mut log = SegmentLog::open(
        root.join("actor"),
        ActorId::new(29),
        SegmentLogOptions::default(),
    )
    .unwrap();
    log.append_batch(&[AppendRequest {
        kind: EventKind::UserMsg,
        wall_timestamp_ns: UtcNanos::new(now()),
        conversation: ConversationId::new([7; 16]),
        sealed_payload: b"approved tool source".to_vec(),
    }])
    .unwrap();
    SourceFence {
        epoch: 1,
        generation: 1,
        digest: digest_bytes(b"approved tool source"),
        start: 1,
        end: 2,
    }
}
fn process() -> ProcessSpec {
    let (command, args) = match std::env::var_os("HM_FABRIC_TOOL_BIN") {
        Some(binary) => (PathBuf::from(binary), vec!["fabric-tool-worker".into()]),
        None => (
            std::env::current_exe().unwrap(),
            vec![
                "--exact".into(),
                "tool_worker_process".into(),
                "--ignored".into(),
            ],
        ),
    };
    ProcessSpec {
        module_id: "declared-tool-worker".into(),
        command,
        args,
        env: BTreeMap::new(),
        cwd: PathBuf::from("/tmp"),
        readiness: Probe::ProcessAlive,
        health: Probe::ProcessAlive,
        readiness_timeout: Duration::from_secs(5),
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
fn config(root: &Path, declaration: ToolDeclaration) -> ToolWorkerConfig {
    ToolWorkerConfig {
        root: root.into(),
        registry_path: root.join("roles.sqlite"),
        scope: scope(),
        host_key: [23; 32],
        worker_key: [59; 32],
        declaration,
    }
}
fn hold(
    registry: &mut DurableRoleRegistry,
    declaration: &ToolDeclaration,
    source: &SourceFence,
    id: &str,
    input: &[u8],
) -> u64 {
    let descriptor = declaration.descriptor().unwrap();
    registry.register(descriptor.clone()).unwrap();
    registry
        .hold(RoleWork {
            id: id.into(),
            role_id: descriptor.id,
            pin: descriptor.pin,
            semantic_version: descriptor.semantic_version,
            operation: "invoke".into(),
            capability_version: 1,
            source: source.clone(),
            payload: input.into(),
            blocks: vec![],
            hook: "tool_dispatch".into(),
            deadline_ns: now() + 10_000_000_000,
        })
        .unwrap()
        .revision
}
#[test]
#[ignore = "production authenticated tool worker child entry"]
fn tool_worker_process() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(tool_worker_from_env())
        .unwrap();
}
#[tokio::test]
async fn real_file_tool_requires_approval_and_returns_pinned_bounded_output() {
    let root = tempfile::tempdir().unwrap();
    let source = fresh_source(root.path());
    let destination = root.path().join("approved.txt");
    let declaration = ToolDeclaration::admit(
        "append_note",
        "/usr/bin/tee",
        vec!["-a".into(), destination.to_string_lossy().into_owned()],
        root.path(),
        4096,
        4096,
        2000,
    )
    .unwrap();
    let mut registry =
        DurableRoleRegistry::open(root.path().join("roles.sqlite"), scope()).unwrap();
    let revision = hold(
        &mut registry,
        &declaration,
        &source,
        "write-note",
        b"one approved note\n",
    );
    let mut worker = ToolWorkerSession::launch(
        process(),
        config(root.path(), declaration.clone()),
        &registry,
        Limits::default(),
    )
    .await
    .unwrap();
    assert!(!destination.exists());
    assert!(
        dispatch_declared_tool(&mut registry, &mut worker, "write-note", &source)
            .await
            .is_err()
    );
    assert!(!destination.exists());
    registry.approve("write-note", revision).unwrap();
    let done = dispatch_declared_tool(&mut registry, &mut worker, "write-note", &source)
        .await
        .unwrap();
    assert_eq!(done.state, WorkState::Terminal);
    let output = done.output.unwrap();
    let outcome: ToolProcessOutcome = serde_json::from_slice(&output.payload).unwrap();
    assert_eq!(outcome.stdout, b"one approved note\n");
    assert_eq!(outcome.exit_code, Some(0));
    assert!(!outcome.effect_unknown);
    assert_eq!(std::fs::read(&destination).unwrap(), outcome.stdout);
    assert_eq!(outcome.stdout_digest, digest_bytes(&outcome.stdout));
    assert!(
        matches!(output.terminal,RunTerminal::Completed{result_digest} if result_digest==digest_bytes(&output.payload))
    );
    assert!(
        dispatch_declared_tool(&mut registry, &mut worker, "write-note", &source)
            .await
            .is_err()
    );
    assert_eq!(std::fs::read(&destination).unwrap(), b"one approved note\n");
    worker.shutdown().await.unwrap();
    drop(registry);
    let registry = DurableRoleRegistry::open(root.path().join("roles.sqlite"), scope()).unwrap();
    assert_eq!(
        registry.get("write-note").unwrap().unwrap().state,
        WorkState::Terminal
    );
    assert_eq!(
        registry
            .catalog("append_note")
            .unwrap()
            .unwrap()
            .descriptor
            .pin,
        declaration.descriptor().unwrap().pin
    );
    assert!(ToolDeclaration::admit(
        "shell",
        "/bin/sh",
        vec!["-c".into(), "echo forbidden".into()],
        root.path(),
        4096,
        4096,
        1000
    )
    .is_err());
}
#[tokio::test]
async fn real_worker_kill_after_file_write_recovers_unknown_and_never_replays() {
    let root = tempfile::tempdir().unwrap();
    let source = fresh_source(root.path());
    let destination = root.path().join("once.txt");
    let declaration = ToolDeclaration::admit(
        "append_once",
        "/usr/bin/tee",
        vec!["-a".into(), destination.to_string_lossy().into_owned()],
        root.path(),
        4096,
        4096,
        2000,
    )
    .unwrap();
    let mut registry =
        DurableRoleRegistry::open(root.path().join("roles.sqlite"), scope()).unwrap();
    let revision = hold(
        &mut registry,
        &declaration,
        &source,
        "interrupted-write",
        b"one actual write\n",
    );
    registry.approve("interrupted-write", revision).unwrap();
    let mut worker = ToolWorkerSession::launch(
        process(),
        config(root.path(), declaration.clone()),
        &registry,
        Limits::default(),
    )
    .await
    .unwrap();
    worker
        .submit(&mut registry, "interrupted-write", &source)
        .await
        .unwrap();
    for _ in 0..100 {
        if std::fs::read(&destination).ok().as_deref() == Some(b"one actual write\n") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert_eq!(std::fs::read(&destination).unwrap(), b"one actual write\n");
    worker.kill_worker().unwrap();
    assert_eq!(
        registry.get("interrupted-write").unwrap().unwrap().state,
        WorkState::Dispatched
    );
    drop(worker);
    drop(registry);
    let mut registry =
        DurableRoleRegistry::open(root.path().join("roles.sqlite"), scope()).unwrap();
    assert_eq!(
        registry.get("interrupted-write").unwrap().unwrap().state,
        WorkState::Uncertain
    );
    let mut worker = ToolWorkerSession::launch(
        process(),
        config(root.path(), declaration),
        &registry,
        Limits::default(),
    )
    .await
    .unwrap();
    assert!(worker
        .submit(&mut registry, "interrupted-write", &source)
        .await
        .is_err());
    assert_eq!(std::fs::read(&destination).unwrap(), b"one actual write\n");
    worker.shutdown().await.unwrap();
    let log = SegmentLog::open(
        root.path().join("actor"),
        ActorId::new(29),
        SegmentLogOptions::default(),
    )
    .unwrap();
    assert_eq!(
        digest_bytes(&log.read_all().unwrap()[0].sealed_payload),
        source.digest
    );
}
#[tokio::test]
async fn withdrawal_after_start_holds_real_result_until_source_bound_acknowledgement() {
    let root = tempfile::tempdir().unwrap();
    let source = fresh_source(root.path());
    let destination = root.path().join("withdrawn.txt");
    let declaration = ToolDeclaration::admit(
        "withdrawable",
        "/usr/bin/tee",
        vec![destination.to_string_lossy().into_owned()],
        root.path(),
        4096,
        4096,
        2000,
    )
    .unwrap();
    let mut registry =
        DurableRoleRegistry::open(root.path().join("roles.sqlite"), scope()).unwrap();
    let revision = hold(
        &mut registry,
        &declaration,
        &source,
        "late-result",
        b"actual accepted work\n",
    );
    registry.approve("late-result", revision).unwrap();
    let mut worker = ToolWorkerSession::launch(
        process(),
        config(root.path(), declaration),
        &registry,
        Limits::default(),
    )
    .await
    .unwrap();
    worker
        .submit(&mut registry, "late-result", &source)
        .await
        .unwrap();
    let receipt = registry
        .withdraw("withdrawable", "owner revoked future starts")
        .unwrap();
    let late = worker.receive(&mut registry, &source).await.unwrap();
    assert_eq!(late.state, WorkState::AwaitingAcknowledgement);
    let output = late.output.unwrap();
    assert_eq!(
        std::fs::read(destination).unwrap(),
        b"actual accepted work\n"
    );
    assert!(registry
        .deliver("late-result", "delivery", &output.payload)
        .is_err());
    worker.shutdown().await.unwrap();
    drop(registry);
    let mut registry =
        DurableRoleRegistry::open(root.path().join("roles.sqlite"), scope()).unwrap();
    assert_eq!(
        registry
            .withdraw("withdrawable", "different reason")
            .unwrap(),
        receipt
    );
    let digest = digest_bytes(&serde_json::to_vec(&output).unwrap());
    let mut stale = source.clone();
    stale.epoch += 1;
    assert!(registry
        .acknowledge_late("late-result", &digest, registry.epoch(), &stale)
        .is_err());
    registry
        .acknowledge_late("late-result", &digest, registry.epoch(), &source)
        .unwrap();
    assert!(registry
        .deliver("late-result", "delivery", &output.payload)
        .unwrap());
    assert!(!registry
        .deliver("late-result", "delivery", &output.payload)
        .unwrap());
}
#[tokio::test]
async fn real_sleep_cancellation_and_real_cat_output_bounds_preserve_unknown() {
    for cancel in [true, false] {
        let root = tempfile::tempdir().unwrap();
        let source = fresh_source(root.path());
        let declaration = if cancel {
            ToolDeclaration::admit(
                "wait",
                "/usr/bin/sleep",
                vec!["3".into()],
                root.path(),
                4096,
                4096,
                5000,
            )
            .unwrap()
        } else {
            let file = root.path().join("large.txt");
            std::fs::write(&file, vec![65; 32768]).unwrap();
            ToolDeclaration::admit(
                "bounded_read",
                "/usr/bin/cat",
                vec![file.to_string_lossy().into_owned()],
                root.path(),
                4096,
                16,
                2000,
            )
            .unwrap()
        };
        let mut registry =
            DurableRoleRegistry::open(root.path().join("roles.sqlite"), scope()).unwrap();
        let revision = hold(&mut registry, &declaration, &source, "bounded-work", b"");
        registry.approve("bounded-work", revision).unwrap();
        let mut worker = ToolWorkerSession::launch(
            process(),
            config(root.path(), declaration),
            &registry,
            Limits::default(),
        )
        .await
        .unwrap();
        worker
            .submit(&mut registry, "bounded-work", &source)
            .await
            .unwrap();
        if cancel {
            worker.cancel(&mut registry).await.unwrap();
        }
        let result = worker.receive(&mut registry, &source).await.unwrap();
        assert_eq!(result.state, WorkState::AwaitingAcknowledgement);
        let outcome: ToolProcessOutcome =
            serde_json::from_slice(&result.output.unwrap().payload).unwrap();
        assert!(outcome.effect_unknown);
        if cancel {
            assert!(outcome.cancelled)
        } else {
            assert!(outcome.overflow);
            assert!(outcome.stdout.len() + outcome.stderr.len() <= 16)
        }
        worker.shutdown().await.unwrap();
    }
}
