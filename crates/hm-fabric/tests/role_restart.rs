use ed25519_dalek::SigningKey;
use hm_context::types::{digest_bytes, Authority, ContextBlock, Scope};
use hm_core::{ActorId, ConversationId, UtcNanos};
use hm_fabric::{
    role_dispatch::*,
    role_store::*,
    roles::{RoleVersion, RunTerminal, TransformDeclaration},
    runtime::{digest_contract, worker_from_env, RuntimeConfig, RuntimeService},
    supervisor::{Probe, ProcessSpec, RestartPolicy},
    transport::{Credentials, Limits, ReplayGuard, UnixTransport},
};
use hm_ledger::{
    frame::EventKind,
    segment::{AppendRequest, SegmentLog, SegmentLogOptions},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    process::Command,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::net::UnixListener;
fn scope() -> Scope {
    Scope {
        owner_id: "role-owner".into(),
        project_id: "role-project".into(),
        workspace_id: None,
    }
}
fn grants() -> RoleGrants {
    RoleGrants {
        operations: BTreeSet::from(["execute".into()]),
        authorities: vec![Authority::ToolObserved, Authority::DerivedInference],
        hooks: BTreeSet::from(["baseline".into(), "model_view".into(), "plan_change".into()]),
        max_input_tokens: 64,
        max_output_tokens: 64,
        may_reorder: false,
    }
}
fn descriptor(id: &str, kind: RoleKind) -> RoleDescriptor {
    let pin = RoleVersion::new(id.as_bytes(), b"scoped role version one");
    let transform = if kind == RoleKind::Transform {
        Some(TransformDeclaration {
            id: id.into(),
            pin: pin.clone(),
            authorities: grants().authorities,
            max_input_tokens: 64,
            max_output_tokens: 64,
            hooks: grants().hooks,
            may_reorder: false,
        })
    } else {
        None
    };
    RoleDescriptor {
        id: id.into(),
        kind,
        pin,
        semantic_version: "1.0.0".into(),
        capabilities: BTreeMap::from([("execute".into(), 1)]),
        grants: grants(),
        transform,
    }
}
fn source() -> SourceFence {
    SourceFence {
        epoch: 1,
        generation: 1,
        digest: digest_bytes(b"durable source observation"),
        start: 1,
        end: 2,
    }
}
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as i64
}
fn work(id: &str, d: &RoleDescriptor) -> RoleWork {
    RoleWork {
        id: id.into(),
        role_id: d.id.clone(),
        operation: "execute".into(),
        capability_version: 1,
        pin: d.pin.clone(),
        semantic_version: d.semantic_version.clone(),
        source: source(),
        payload: b"durable source observation".to_vec(),
        blocks: vec![ContextBlock {
            id: "source".into(),
            text: "durable source observation".into(),
            authority: Authority::ToolObserved,
            provenance: vec![],
            tokens: 4,
            required: true,
        }],
        hook: "baseline".into(),
        deadline_ns: now() + 60_000_000_000,
    }
}
fn output(record: &WorkRecord) -> RoleOutput {
    let blocks = if record.descriptor.kind == RoleKind::Transform {
        record.work.blocks.clone()
    } else {
        vec![]
    };
    let payload = serde_json::to_vec(&blocks).unwrap();
    RoleOutput {
        terminal: RunTerminal::Completed {
            result_digest: digest_bytes(&payload),
        },
        payload,
        blocks,
        grants: record.descriptor.grants.clone(),
    }
}
fn creds(client: bool) -> Credentials {
    let a = SigningKey::from_bytes(&[17; 32]);
    let b = SigningKey::from_bytes(&[71; 32]);
    Credentials {
        scope: scope(),
        signing_key: if client { a.clone() } else { b.clone() },
        peer_key: if client {
            b.verifying_key()
        } else {
            a.verifying_key()
        },
    }
}
#[test]
#[ignore = "actual role owner process for interruption gate"]
fn owner_process() {
    let root = PathBuf::from(std::env::var("HM_ROLE_RESTART_ROOT").unwrap());
    let mut log = SegmentLog::open(
        root.join("actor"),
        ActorId::new(17),
        SegmentLogOptions::default(),
    )
    .unwrap();
    log.append_batch(&[AppendRequest {
        kind: EventKind::UserMsg,
        wall_timestamp_ns: UtcNanos::new(now()),
        conversation: ConversationId::new([5; 16]),
        sealed_payload: b"durable source observation".to_vec(),
    }])
    .unwrap();
    drop(log);
    let mut registry = DurableRoleRegistry::open(root.join("roles.sqlite"), scope()).unwrap();
    let kinds = [
        ("tool", RoleKind::Tool),
        ("runner", RoleKind::Runner),
        ("compact", RoleKind::Compaction),
        ("transform", RoleKind::Transform),
    ];
    for (id, kind) in kinds {
        let d = descriptor(id, kind);
        registry.register(d.clone()).unwrap();
        let held = registry.hold(work(id, &d)).unwrap();
        registry.approve(id, held.revision).unwrap();
    }
    let d = descriptor("tool", RoleKind::Tool);
    registry.hold(work("unapproved", &d)).unwrap();
    let held = registry.hold(work("approved-only", &d)).unwrap();
    registry.approve("approved-only", held.revision).unwrap();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let mut transport = UnixTransport::connect(
                root.join("owner.sock"),
                creds(true),
                ReplayGuard::default(),
                Limits::default(),
            )
            .await
            .unwrap();
            for (id, _) in kinds {
                handoff(&mut registry, &mut transport, id, &source(), now())
                    .await
                    .unwrap();
            }
            loop {
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });
}
#[tokio::test]
async fn killed_role_owner_recovers_all_declared_states_and_requires_late_ack() {
    let root = tempfile::tempdir().unwrap();
    let listener = UnixListener::bind(root.path().join("owner.sock")).unwrap();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "owner_process", "--ignored", "--nocapture"])
        .env("HM_ROLE_RESTART_ROOT", root.path())
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let (stream, _) = tokio::time::timeout(Duration::from_secs(5), listener.accept())
        .await
        .unwrap()
        .unwrap();
    let mut transport = UnixTransport::accept(
        stream,
        creds(false),
        ReplayGuard::default(),
        Limits::default(),
    )
    .await
    .unwrap();
    let mut tickets = BTreeMap::new();
    for _ in 0..4 {
        let frame = transport.receive().await.unwrap();
        let request: RoleWireRequest = serde_json::from_slice(&frame.payload).unwrap();
        assert_eq!(request.work.source, source());
        tickets.insert(request.work.id, request.ticket);
    }
    child.kill().unwrap();
    assert!(!child.wait().unwrap().success());
    let log = SegmentLog::open(
        root.path().join("actor"),
        ActorId::new(17),
        SegmentLogOptions::default(),
    )
    .unwrap();
    let frames = log.read_all().unwrap();
    assert_eq!(frames.len(), 1);
    assert_eq!(digest_bytes(&frames[0].sealed_payload), source().digest);
    let mut registry =
        DurableRoleRegistry::open(root.path().join("roles.sqlite"), scope()).unwrap();
    assert!(DurableRoleRegistry::open(root.path().join("roles.sqlite"), scope()).is_err());
    for (id, ticket) in &tickets {
        let recovered = registry.get(id).unwrap().unwrap();
        assert_eq!(recovered.state, WorkState::Uncertain);
        assert!(matches!(
            registry.begin(id, registry.epoch(), &source(), now()),
            Err(RoleError::Uncertain)
        ));
        assert!(registry
            .begin(id, ticket.writer_epoch, &source(), now())
            .is_err());
        let result = output(&recovered);
        let digest = digest_bytes(&serde_json::to_vec(&result).unwrap());
        let late = registry
            .complete(ticket, &source(), result.clone(), now())
            .unwrap();
        assert_eq!(late.state, WorkState::AwaitingAcknowledgement);
        assert!(registry.deliver(id, "receipt", &result.payload).is_err());
        assert!(registry
            .acknowledge_late(id, "wrong", registry.epoch(), &source())
            .is_err());
        assert_eq!(
            registry
                .acknowledge_late(id, &digest, registry.epoch(), &source())
                .unwrap()
                .state,
            WorkState::Terminal
        );
        assert!(registry.deliver(id, "receipt", &result.payload).unwrap());
        assert!(!registry.deliver(id, "receipt", &result.payload).unwrap());
    }
    assert!(matches!(
        registry.begin("unapproved", registry.epoch(), &source(), now()),
        Err(RoleError::Unapproved)
    ));
    assert_eq!(
        registry.get("approved-only").unwrap().unwrap().state,
        WorkState::Approved
    );
    drop(registry);
    let registry = DurableRoleRegistry::open(root.path().join("roles.sqlite"), scope()).unwrap();
    for id in tickets.keys() {
        let record = registry.get(id).unwrap().unwrap();
        assert_eq!(record.state, WorkState::Terminal);
        assert_eq!(record.deliveries.len(), 1);
        assert_eq!(
            registry
                .catalog(&record.work.role_id)
                .unwrap()
                .unwrap()
                .descriptor
                .capabilities
                .get("execute"),
            Some(&1)
        );
    }
}
#[test]
fn withdrawal_semantic_source_and_grant_fences_survive_reopen() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("roles.sqlite");
    let mut registry = DurableRoleRegistry::open(&path, scope()).unwrap();
    let d = descriptor("tool", RoleKind::Tool);
    registry.register(d.clone()).unwrap();
    let held = registry.hold(work("active", &d)).unwrap();
    registry.approve("active", held.revision).unwrap();
    let ticket = registry
        .begin("active", registry.epoch(), &source(), now())
        .unwrap();
    let withdrawal = registry.withdraw("tool", "withdrawn by owner").unwrap();
    assert_eq!(
        registry.withdraw("tool", "another reason").unwrap(),
        withdrawal
    );
    let result = output(&registry.get("active").unwrap().unwrap());
    assert_eq!(
        registry
            .complete(&ticket, &source(), result.clone(), now())
            .unwrap()
            .state,
        WorkState::AwaitingAcknowledgement
    );
    drop(registry);
    let mut registry = DurableRoleRegistry::open(&path, scope()).unwrap();
    assert_eq!(registry.catalog("tool").unwrap().unwrap(), withdrawal);
    let mut changed = source();
    changed.epoch += 1;
    let digest = digest_bytes(&serde_json::to_vec(&result).unwrap());
    assert!(registry
        .acknowledge_late("active", &digest, registry.epoch(), &changed)
        .is_err());
    registry
        .acknowledge_late("active", &digest, registry.epoch(), &source())
        .unwrap();
    let d = descriptor("transform", RoleKind::Transform);
    registry.register(d.clone()).unwrap();
    let mut broadened = d.clone();
    broadened.grants.operations.insert("network".into());
    assert!(registry.register(broadened).is_err());
    let held = registry.hold(work("transform-held", &d)).unwrap();
    registry.approve("transform-held", held.revision).unwrap();
    let mut narrow = d.clone();
    narrow.grants.max_output_tokens = 8;
    narrow.transform.as_mut().unwrap().max_output_tokens = 8;
    registry.register(narrow).unwrap();
    assert_eq!(
        registry.get("transform-held").unwrap().unwrap().state,
        WorkState::Held
    );
    let mut version = work("bad-version", &d);
    version.semantic_version = "2.0.0".into();
    assert!(registry.hold(version).is_err());
    let current = registry.get("transform-held").unwrap().unwrap();
    registry
        .approve("transform-held", current.revision)
        .unwrap();
    assert!(registry
        .begin("transform-held", registry.epoch(), &changed, now())
        .is_err());
}
#[test]
fn actual_transform_and_compaction_dispatch_persist_bounded_outputs() {
    let root = tempfile::tempdir().unwrap();
    let mut registry =
        DurableRoleRegistry::open(root.path().join("roles.sqlite"), scope()).unwrap();
    for (id, kind) in [
        ("compact", RoleKind::Compaction),
        ("transform", RoleKind::Transform),
    ] {
        let d = descriptor(id, kind);
        registry.register(d.clone()).unwrap();
        let held = registry.hold(work(id, &d)).unwrap();
        registry.approve(id, held.revision).unwrap();
    }
    let transformed =
        dispatch_transform(&mut registry, "transform", &source(), scope(), now()).unwrap();
    assert_eq!(
        transformed.output.unwrap().blocks[0].text,
        "durable source observation"
    );
    let compact = dispatch_compact(
        &mut registry,
        "compact",
        &source(),
        b"replacement summary",
        now(),
    )
    .unwrap();
    assert_eq!(compact.output.unwrap().payload, b"replacement summary");
    drop(registry);
    let registry = DurableRoleRegistry::open(root.path().join("roles.sqlite"), scope()).unwrap();
    assert_eq!(
        registry.get("compact").unwrap().unwrap().state,
        WorkState::Terminal
    );
    assert_eq!(
        registry.get("transform").unwrap().unwrap().state,
        WorkState::Terminal
    );
}
#[test]
#[ignore = "production digest worker process entry"]
fn digest_worker_process() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(worker_from_env())
        .unwrap();
}
#[tokio::test]
async fn approved_digest_dispatch_uses_real_worker_and_durable_terminal() {
    let root = tempfile::tempdir().unwrap();
    let mut runtime =
        RuntimeService::open(RuntimeConfig::new(root.path(), scope(), [21; 32], [43; 32])).unwrap();
    runtime
        .launch_worker(ProcessSpec {
            module_id: "digest-worker".into(),
            command: std::env::current_exe().unwrap(),
            args: vec![
                "--exact".into(),
                "digest_worker_process".into(),
                "--ignored".into(),
            ],
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
        })
        .await
        .unwrap();
    let mut registry =
        DurableRoleRegistry::open(root.path().join("roles.sqlite"), scope()).unwrap();
    let mut d = descriptor("digest", RoleKind::Tool);
    d.pin = digest_contract().pin;
    registry.register(d.clone()).unwrap();
    let held = registry.hold(work("digest-job", &d)).unwrap();
    assert!(dispatch_tool(
        &mut registry,
        &mut runtime,
        "digest-job",
        &source(),
        now(),
        now()
    )
    .await
    .is_err());
    assert!(runtime.effect("digest-job").unwrap().is_none());
    registry.approve("digest-job", held.revision).unwrap();
    let done = dispatch_tool(
        &mut registry,
        &mut runtime,
        "digest-job",
        &source(),
        now(),
        now(),
    )
    .await
    .unwrap();
    assert_eq!(done.state, WorkState::Terminal);
    assert!(runtime
        .effect("digest-job")
        .unwrap()
        .unwrap()
        .observation
        .is_some());
    runtime.shutdown().await.unwrap();
    drop(registry);
    let registry = DurableRoleRegistry::open(root.path().join("roles.sqlite"), scope()).unwrap();
    assert_eq!(
        registry.get("digest-job").unwrap().unwrap().state,
        WorkState::Terminal
    );
}
