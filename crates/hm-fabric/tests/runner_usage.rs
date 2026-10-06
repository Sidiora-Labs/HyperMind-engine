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
fn owner() -> Scope {
    Scope {
        owner_id: "original-runner-owner".into(),
        project_id: "original-runner-usage".into(),
        workspace_id: None,
    }
}
fn actor_config(path: &Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("canonical"),
        actor: ActorId::new(107),
        user: [17; 16],
        kek: [53; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}
fn declaration() -> RunnerDeclaration {
    let model_digest = "357c53fb659c5076de1d65ccb0b397446227b71a42be9d1603d46168015c9e4b";
    RunnerDeclaration {
        id: "original-response-runner".into(),
        semantic_version: "1.0.0".into(),
        endpoint: std::env::var("HM_FABRIC_RUNNER_ENDPOINT")
            .expect("configured runner endpoint required"),
        model_digest: model_digest.into(),
        profile: ProviderProfile {
            model_id: "qwen2.5:3b".into(),
            model_revision: model_digest.into(),
            tokenizer_id: "ollama-native-counters".into(),
            tokenizer_revision: model_digest.into(),
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
            context_tokens: 2048,
            reserved_output_tokens: 64,
            required_tokens: 0,
        },
        system: "Return only a JSON object containing the brief requested answer.".into(),
        json_schema: serde_json::json!({"type":"object","properties":{"answer":{"type":"string"}},"required":["answer"],"additionalProperties":false}),
        max_prompt_bytes: 4096,
        max_response_bytes: 16384,
        timeout_ms: 120000,
    }
}
fn process() -> ProcessSpec {
    let (command, args) = match std::env::var_os("HM_FABRIC_RUNNER_BIN") {
        Some(p) => (PathBuf::from(p), vec!["fabric-runner-worker".into()]),
        None => (
            std::env::current_exe().unwrap(),
            vec![
                "--exact".into(),
                "native_usage_runner_child".into(),
                "--ignored".into(),
            ],
        ),
    };
    ProcessSpec {
        module_id: "original-usage-worker".into(),
        command,
        args,
        env: BTreeMap::new(),
        cwd: "/tmp".into(),
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
fn config(root: &Path, d: RunnerDeclaration) -> RunnerWorkerConfig {
    RunnerWorkerConfig {
        root: root.into(),
        registry_path: root.join("roles.sqlite"),
        scope: owner(),
        host_key: [101; 32],
        worker_key: [127; 32],
        declaration: d,
    }
}
async fn prepare(
    actor: &ActorEngine,
    registry: &mut DurableRoleRegistry,
    d: &RunnerDeclaration,
    root: &Path,
    id: &str,
    prompt: &str,
) -> SourceFence {
    let source_digest = digest_bytes(prompt.as_bytes());
    let mut source = SegmentLog::open(
        root.join(id),
        ActorId::new(108),
        SegmentLogOptions::default(),
    )
    .unwrap();
    source
        .append_batch(&[AppendRequest {
            kind: EventKind::UserMsg,
            wall_timestamp_ns: UtcNanos::new(now()),
            conversation: ConversationId::new([19; 16]),
            sealed_payload: prompt.as_bytes().into(),
        }])
        .unwrap();
    let fence = SourceFence {
        epoch: 1,
        generation: 1,
        digest: source_digest.clone(),
        start: 1,
        end: 2,
    };
    let reservation = reserve(
        actor,
        &owner(),
        &owner(),
        ReservationRequest {
            request_id: id.into(),
            attribution: UsageAttribution {
                job_id: format!("job-{id}"),
                worker_id: "original-usage-worker".into(),
                session_id: "usage-session".into(),
                turn_id: format!("turn-{id}"),
                provider_id: "ollama-local".into(),
                model_id: "qwen2.5:3b".into(),
                source_ids: BTreeSet::from([format!("source-{id}")]),
            },
            kind: JobKind::Extraction,
            reserved_tokens: 2048,
            input_digest: source_digest,
        },
    )
    .await
    .unwrap();
    let input = RunnerInput {
        prompt: prompt.into(),
        usage: CanonicalUsageBinding {
            lease: reservation.lease.clone(),
            reservation_id: reservation.id,
            reservation_digest: reservation.request_digest,
            input_digest: reservation.input_digest,
            reserved_tokens: reservation.reserved_tokens,
        },
    };
    let descriptor = d.descriptor().unwrap();
    registry.register(descriptor.clone()).unwrap();
    let held = registry
        .hold(RoleWork {
            id: id.into(),
            role_id: descriptor.id,
            pin: descriptor.pin,
            semantic_version: descriptor.semantic_version,
            operation: "run".into(),
            capability_version: 1,
            source: fence.clone(),
            payload: serde_json::to_vec(&input).unwrap(),
            blocks: vec![],
            hook: "runner_dispatch".into(),
            deadline_ns: now() + 120_000_000_000,
        })
        .unwrap();
    registry.approve(id, held.revision).unwrap();
    fence
}
#[tokio::test]
#[ignore]
async fn native_usage_runner_child() {
    runner_worker_from_env().await.unwrap();
}
#[tokio::test]
async fn original_authenticated_call_settles_existing_reservation_once_and_unknown_survives_restart()
 {
    let root = tempfile::tempdir().unwrap();
    let mut actor = ActorEngine::open(actor_config(root.path())).await.unwrap();
    configure(
        &actor,
        &owner(),
        &owner(),
        UsageLimits {
            total_tokens: 8192,
            hourly_tokens: 8192,
            daily_tokens: 8192,
            job_tokens: 2048,
            concurrency: 1,
            lease_ms: 180000,
        },
    )
    .await
    .unwrap();
    let d = declaration();
    let mut registry =
        DurableRoleRegistry::open(root.path().join("roles.sqlite"), owner()).unwrap();
    let source = prepare(
        &actor,
        &mut registry,
        &d,
        root.path(),
        "deliver-later",
        r#"Reply with exactly this JSON object: {"answer":"Meridian-883"}."#,
    )
    .await;
    let binding = begin_runner_dispatch(
        &actor,
        &owner(),
        &owner(),
        &registry,
        "deliver-later",
        &d,
        "original-usage-worker",
    )
    .await
    .unwrap();
    assert!(
        begin_runner_dispatch(
            &actor,
            &owner(),
            &owner(),
            &registry,
            "deliver-later",
            &d,
            "original-usage-worker"
        )
        .await
        .is_err()
    );
    let limits = Limits {
        io_timeout: Duration::from_secs(130),
        ..Limits::default()
    };
    let mut worker = RunnerWorkerSession::launch(
        process(),
        config(root.path(), d.clone()),
        &registry,
        limits.clone(),
    )
    .await
    .unwrap();
    worker
        .submit(&mut registry, "deliver-later", &source)
        .await
        .unwrap();
    let (_, receipt) = worker
        .receive_with_receipt(&mut registry, &source)
        .await
        .unwrap();
    assert_eq!(receipt.outcome().value["answer"], "Meridian-883");
    worker.shutdown().unwrap();
    drop(worker);
    // A consumer interruption leaves the existing reservation unknown until the original receipt is admitted.
    mark_unknown(&actor, &owner(), &owner(), &binding.reservation_id)
        .await
        .unwrap();
    actor.shutdown().await.unwrap();
    actor = ActorEngine::open(actor_config(root.path())).await.unwrap();
    let before = inspect(&actor, &owner(), &owner()).await.unwrap();
    assert_eq!(before.reservations.len(), 1);
    assert!(before.observations.is_empty());
    assert!(before.reservations[&binding.reservation_id].dispatched);
    assert_eq!(
        before.accounting.unknown_usage.get(&format!(
            "{}:{}",
            binding.lease.job_id, binding.lease.attempt
        )),
        Some(&2048)
    );
    let wrong = Scope {
        project_id: "other-project".into(),
        ..owner()
    };
    assert!(
        ingest_runner_receipt(&actor, &wrong, &wrong, &receipt)
            .await
            .is_err()
    );
    let observation = ingest_runner_receipt(&actor, &owner(), &owner(), &receipt)
        .await
        .unwrap();
    assert_eq!(
        observation.snapshot.original_bytes,
        receipt.outcome().original_provider_bytes
    );
    let actual =
        observation.snapshot.tokens.input.unwrap() + observation.snapshot.tokens.output.unwrap();
    assert!(actual > 0);
    let duplicate = ingest_runner_receipt(&actor, &owner(), &owner(), &receipt)
        .await
        .unwrap();
    assert_eq!(duplicate.id, observation.id);
    actor.shutdown().await.unwrap();
    actor = ActorEngine::open(actor_config(root.path())).await.unwrap();
    assert_eq!(
        ingest_runner_receipt(&actor, &owner(), &owner(), &receipt)
            .await
            .unwrap()
            .id,
        observation.id
    );
    let settled = inspect(&actor, &owner(), &owner()).await.unwrap();
    assert_eq!(settled.reservations.len(), 1);
    assert_eq!(settled.observations.len(), 1);
    assert_eq!(
        settled.reservations[&binding.reservation_id].settled_tokens,
        Some(actual)
    );
    assert!(settled.accounting.unknown_usage.is_empty());
    let source = prepare(
        &actor,
        &mut registry,
        &d,
        root.path(),
        "killed-native-run",
        "Provide a detailed answer about maritime positioning instruments.",
    )
    .await;
    let killed = begin_runner_dispatch(
        &actor,
        &owner(),
        &owner(),
        &registry,
        "killed-native-run",
        &d,
        "original-usage-worker",
    )
    .await
    .unwrap();
    let mut worker = RunnerWorkerSession::launch(
        process(),
        config(root.path(), d.clone()),
        &registry,
        limits.clone(),
    )
    .await
    .unwrap();
    worker
        .submit(&mut registry, "killed-native-run", &source)
        .await
        .unwrap();
    worker.kill_worker(&mut registry).unwrap();
    mark_unknown(&actor, &owner(), &owner(), &killed.reservation_id)
        .await
        .unwrap();
    drop(worker);
    drop(registry);
    actor.shutdown().await.unwrap();
    actor = ActorEngine::open(actor_config(root.path())).await.unwrap();
    let mut registry =
        DurableRoleRegistry::open(root.path().join("roles.sqlite"), owner()).unwrap();
    let state = inspect(&actor, &owner(), &owner()).await.unwrap();
    assert_eq!(state.observations.len(), 1);
    assert_eq!(state.reservations.len(), 2);
    assert_eq!(
        state.reservations[&killed.reservation_id].settled_tokens,
        None
    );
    assert_eq!(
        state
            .accounting
            .unknown_usage
            .get(&format!("{}:{}", killed.lease.job_id, killed.lease.attempt)),
        Some(&2048)
    );
    assert_eq!(
        registry.get("killed-native-run").unwrap().unwrap().state,
        WorkState::Uncertain
    );
    assert!(
        begin_runner_dispatch(
            &actor,
            &owner(),
            &owner(),
            &registry,
            "killed-native-run",
            &d,
            "original-usage-worker"
        )
        .await
        .is_err()
    );
    let mut restarted =
        RunnerWorkerSession::launch(process(), config(root.path(), d), &registry, limits)
            .await
            .unwrap();
    assert!(
        restarted
            .submit(&mut registry, "killed-native-run", &source)
            .await
            .is_err()
    );
    restarted.shutdown().unwrap();
    actor.shutdown().await.unwrap();
}
