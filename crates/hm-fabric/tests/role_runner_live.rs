use hm_context::{
    maintenance::JobKind,
    provider::CapabilityProfile,
    provider_continuity::ProviderProfile,
    types::{Scope, TokenBudget, digest_bytes},
};
use hm_core::{ActorId, ConversationId, UtcNanos};
use hm_fabric::{
    role_runner::*,
    role_store::*,
    roles::RunTerminal,
    supervisor::{Probe, ProcessSpec, RestartPolicy},
    transport::Limits,
};
use hm_ledger::{
    frame::EventKind,
    segment::{AppendRequest, SegmentLog, SegmentLogOptions},
};
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    usage_service::*,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as i64
}
fn scope() -> Scope {
    Scope {
        owner_id: "runner-owner".into(),
        project_id: "runner-live".into(),
        workspace_id: None,
    }
}
fn actor_config(root: &Path) -> ActorConfig {
    ActorConfig {
        actor_directory: root.join("usage"),
        actor: ActorId::new(94),
        user: [13; 16],
        kek: [31; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}
fn process() -> ProcessSpec {
    let (command, args) = match std::env::var_os("HM_FABRIC_RUNNER_BIN") {
        Some(p) => (PathBuf::from(p), vec!["fabric-runner-worker".into()]),
        None => (
            std::env::current_exe().unwrap(),
            vec![
                "--exact".into(),
                "runner_worker_process".into(),
                "--ignored".into(),
            ],
        ),
    };
    ProcessSpec {
        module_id: "qwen-runner".into(),
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
fn declaration() -> RunnerDeclaration {
    let digest = "357c53fb659c5076de1d65ccb0b397446227b71a42be9d1603d46168015c9e4b";
    RunnerDeclaration {
        id: "qwen-answer".into(),
        semantic_version: "1.0.0".into(),
        endpoint: std::env::var("HM_FABRIC_RUNNER_ENDPOINT")
            .expect("owner-configured local runner endpoint required"),
        model_digest: digest.into(),
        profile: ProviderProfile {
            model_id: "qwen2.5:3b".into(),
            model_revision: digest.into(),
            tokenizer_id: "ollama-native-counters".into(),
            tokenizer_revision: digest.into(),
            serializer_id: "ollama-structured-chat-v1".into(),
            serializer_revision: "1".into(),
            capabilities: CapabilityProfile {
                user: true,
                assistant: true,
                tool: false,
                text: true,
                tool_calls: false,
                tool_results: false,
                opaque: false,
            },
        },
        budget: TokenBudget {
            context_tokens: 4096,
            reserved_output_tokens: 64,
            required_tokens: 0,
        },
        system: "Return only the requested JSON object. Keep answer brief.".into(),
        json_schema: serde_json::json!({"type":"object","properties":{"answer":{"type":"string"}},"required":["answer"],"additionalProperties":false}),
        max_prompt_bytes: 8192,
        max_response_bytes: 16384,
        timeout_ms: 120000,
    }
}
fn configuration(root: &Path, d: RunnerDeclaration) -> RunnerWorkerConfig {
    RunnerWorkerConfig {
        root: root.into(),
        registry_path: root.join("roles.sqlite"),
        scope: scope(),
        host_key: [41; 32],
        worker_key: [71; 32],
        declaration: d,
    }
}
fn source(root: &Path, prompt: &str) -> SourceFence {
    let mut log = SegmentLog::open(
        root.join("source"),
        ActorId::new(95),
        SegmentLogOptions::default(),
    )
    .unwrap();
    log.append_batch(&[AppendRequest {
        kind: EventKind::UserMsg,
        wall_timestamp_ns: UtcNanos::new(now()),
        conversation: ConversationId::new([9; 16]),
        sealed_payload: prompt.as_bytes().into(),
    }])
    .unwrap();
    SourceFence {
        epoch: 1,
        generation: 1,
        digest: digest_bytes(prompt.as_bytes()),
        start: 1,
        end: 2,
    }
}
async fn canonical(root: &Path, prompt: &str) -> (ActorEngine, CanonicalUsageBinding) {
    let a = ActorEngine::open(actor_config(root)).await.unwrap();
    configure(
        &a,
        &scope(),
        &scope(),
        UsageLimits {
            total_tokens: 8192,
            hourly_tokens: 8192,
            daily_tokens: 8192,
            job_tokens: 4096,
            concurrency: 1,
            lease_ms: 180000,
        },
    )
    .await
    .unwrap();
    let r = reserve(
        &a,
        &scope(),
        &scope(),
        ReservationRequest {
            request_id: "native-run-request".into(),
            attribution: UsageAttribution {
                job_id: "native-run-job".into(),
                worker_id: "qwen-runner".into(),
                session_id: "runner-session".into(),
                turn_id: "runner-turn".into(),
                provider_id: "ollama-local".into(),
                model_id: "qwen2.5:3b".into(),
                source_ids: BTreeSet::from(["source-event-1".into()]),
            },
            kind: JobKind::Extraction,
            reserved_tokens: 4096,
            input_digest: digest_bytes(prompt.as_bytes()),
        },
    )
    .await
    .unwrap();
    let b = CanonicalUsageBinding {
        reservation_id: r.id,
        input_digest: r.input_digest,
        reservation_digest: r.request_digest,
        reserved_tokens: r.reserved_tokens,
    };
    (a, b)
}
fn work(
    reg: &mut DurableRoleRegistry,
    d: &RunnerDeclaration,
    s: &SourceFence,
    id: &str,
    prompt: &str,
    binding: CanonicalUsageBinding,
) {
    let descriptor = d.descriptor().unwrap();
    reg.register(descriptor.clone()).unwrap();
    let held = reg
        .hold(RoleWork {
            id: id.into(),
            role_id: descriptor.id,
            operation: "run".into(),
            capability_version: 1,
            pin: descriptor.pin,
            semantic_version: descriptor.semantic_version,
            source: s.clone(),
            payload: serde_json::to_vec(&RunnerInput {
                prompt: prompt.into(),
                usage: binding,
            })
            .unwrap(),
            blocks: vec![],
            hook: "runner_dispatch".into(),
            deadline_ns: now() + 120_000_000_000,
        })
        .unwrap();
    reg.approve(id, held.revision).unwrap();
}
fn limits() -> Limits {
    Limits {
        io_timeout: Duration::from_secs(130),
        ..Limits::default()
    }
}
#[tokio::test]
#[ignore]
async fn runner_worker_process() {
    runner_worker_from_env().await.unwrap();
}
#[tokio::test]
async fn actual_model_signed_route_original_evidence_and_late_ack() {
    let dir = tempfile::tempdir().unwrap();
    let prompt = "Set answer to the exact vessel serial Quartz-219.";
    let s = source(dir.path(), prompt);
    let (actor, binding) = canonical(dir.path(), prompt).await;
    let mut reg = DurableRoleRegistry::open(dir.path().join("roles.sqlite"), scope()).unwrap();
    let d = declaration();
    work(&mut reg, &d, &s, "live-run", prompt, binding.clone());
    let mut worker =
        RunnerWorkerSession::launch(process(), configuration(dir.path(), d), &reg, limits())
            .await
            .unwrap();
    let ticket = worker.submit(&mut reg, "live-run", &s).await.unwrap();
    reg.withdraw("qwen-answer", "owner withdrew while model running")
        .unwrap();
    let result = worker.receive(&mut reg, &s).await.unwrap();
    assert_eq!(result.state, WorkState::AwaitingAcknowledgement);
    let output = result.output.as_ref().unwrap();
    assert!(matches!(output.terminal, RunTerminal::Completed { .. }));
    let outcome: RunnerOutcome = serde_json::from_slice(&output.payload).unwrap();
    assert_eq!(outcome.value["answer"], "Quartz-219");
    assert_eq!(outcome.usage, binding);
    assert_eq!(outcome.request_digest, ticket.work_digest);
    assert!(outcome.observation.tokens.input.unwrap() > 0);
    assert!(outcome.observation.tokens.output.unwrap() > 0);
    assert_eq!(
        outcome.original_provider_bytes,
        outcome.observation.original_bytes
    );
    let raw: serde_json::Value = serde_json::from_slice(&outcome.original_provider_bytes).unwrap();
    assert_eq!(raw["model"], "qwen2.5:3b");
    assert_eq!(raw["done"], true);
    assert!(
        reg.deliver("live-run", "premature", &output.payload)
            .is_err()
    );
    let digest = digest_bytes(&serde_json::to_vec(output).unwrap());
    let epoch = reg.epoch();
    assert!(
        reg.acknowledge_late("live-run", "wrong", epoch, &s)
            .is_err()
    );
    assert_eq!(
        reg.acknowledge_late("live-run", &digest, epoch, &s)
            .unwrap()
            .state,
        WorkState::Terminal
    );
    worker.shutdown().unwrap();
    let held = inspect(&actor, &scope(), &scope()).await.unwrap();
    assert_eq!(
        held.reservations[&outcome.usage.reservation_id].settled_tokens,
        None
    );
    assert!(held.observations.is_empty());
    actor.shutdown().await.unwrap();
}
#[tokio::test]
async fn actual_worker_kill_reopen_retains_canonical_unknown_and_refuses_redispatch() {
    let dir = tempfile::tempdir().unwrap();
    let prompt = "Return answer containing a detailed account of ship navigation instruments.";
    let s = source(dir.path(), prompt);
    let (actor, binding) = canonical(dir.path(), prompt).await;
    let mut reg = DurableRoleRegistry::open(dir.path().join("roles.sqlite"), scope()).unwrap();
    let d = declaration();
    work(&mut reg, &d, &s, "interrupted-run", prompt, binding.clone());
    let mut worker = RunnerWorkerSession::launch(
        process(),
        configuration(dir.path(), d.clone()),
        &reg,
        limits(),
    )
    .await
    .unwrap();
    worker
        .submit(&mut reg, "interrupted-run", &s)
        .await
        .unwrap();
    worker.kill_worker(&mut reg).unwrap();
    mark_unknown(&actor, &scope(), &scope(), &binding.reservation_id)
        .await
        .unwrap();
    drop(worker);
    drop(reg);
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(actor_config(dir.path())).await.unwrap();
    let mut reg = DurableRoleRegistry::open(dir.path().join("roles.sqlite"), scope()).unwrap();
    assert_eq!(
        reg.get("interrupted-run").unwrap().unwrap().state,
        WorkState::Uncertain
    );
    let mut fresh =
        RunnerWorkerSession::launch(process(), configuration(dir.path(), d), &reg, limits())
            .await
            .unwrap();
    assert!(fresh.submit(&mut reg, "interrupted-run", &s).await.is_err());
    let held = inspect(&actor, &scope(), &scope()).await.unwrap();
    let reservation = &held.reservations[&binding.reservation_id];
    assert_eq!(reservation.reserved_tokens, 4096);
    assert_eq!(
        held.accounting.unknown_usage.get(&format!(
            "{}:{}",
            reservation.lease.job_id, reservation.lease.attempt
        )),
        Some(&4096)
    );
    assert_eq!(reservation.settled_tokens, None);
    assert!(held.observations.is_empty());
    fresh.shutdown().unwrap();
    actor.shutdown().await.unwrap();
}
