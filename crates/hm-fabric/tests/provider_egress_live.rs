use hm_context::{
    maintenance::JobKind,
    provider::CapabilityProfile,
    provider_continuity::ProviderProfile,
    types::{Scope, TokenBudget, digest_bytes},
};
use hm_core::ActorId;
use hm_fabric::{
    egress::*,
    provider_egress::RunnerProviderGuard,
    role_runner::*,
    role_store::*,
    roles::RunTerminal,
    secret_handles::SecretHandleService,
    supervisor::{Probe, ProcessSpec, RestartPolicy},
    transport::Limits,
};
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    usage_service::*,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use zeroize::Zeroizing;
const CANARY: &str = "Bearer runner-egress-native-canary-93b7a052";
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as i64
}
#[tokio::test]
#[ignore]
async fn guarded_provider_worker_process() {
    runner_worker_from_env().await.unwrap();
}
#[tokio::test]
async fn actual_guarded_runner_preserves_original_usage_and_excludes_credentials() {
    let endpoint =
        std::env::var("HM_FABRIC_RUNNER_ENDPOINT").expect("configured provider endpoint required");
    let digest =
        std::env::var("HM_FABRIC_RUNNER_MODEL_DIGEST").expect("configured model digest required");
    let endpoint_url = reqwest::Url::parse(&endpoint).unwrap();
    let provider_address = format!(
        "{}:{}",
        endpoint_url.host_str().unwrap(),
        endpoint_url.port().unwrap()
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_address = listener.local_addr().unwrap();
    let captured = Arc::new(Mutex::new(Vec::new()));
    let wire = captured.clone();
    let proxy = tokio::spawn(async move {
        loop {
            let (mut client, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            loop {
                let mut one = [0];
                assert_eq!(client.read(&mut one).await.unwrap(), 1);
                bytes.push(one[0]);
                if bytes.ends_with(b"\r\n\r\n") {
                    break;
                }
                assert!(bytes.len() < 32768);
            }
            let headers = String::from_utf8(bytes.clone()).unwrap();
            let content_length = headers
                .lines()
                .find_map(|line| {
                    line.split_once(':')
                        .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                        .map(|(_, value)| value.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            let mut body = vec![0; content_length];
            client.read_exact(&mut body).await.unwrap();
            let first = headers.lines().next().unwrap();
            let mut parts = first.split_whitespace();
            let method = parts.next().unwrap();
            let target = reqwest::Url::parse(parts.next().unwrap()).unwrap();
            assert_eq!(
                format!("{}:{}", target.host_str().unwrap(), target.port().unwrap()),
                provider_address
            );
            wire.lock().unwrap().push(headers.clone());
            let mut upstream = TcpStream::connect(&provider_address).await.unwrap();
            let mut request = format!("{method} {} HTTP/1.1\r\n", target.path());
            for line in headers.lines().skip(1) {
                if !line.is_empty() && !line.to_ascii_lowercase().starts_with("connection:") {
                    request.push_str(line);
                    request.push_str("\r\n");
                }
            }
            request.push_str("Connection: close\r\n\r\n");
            upstream.write_all(request.as_bytes()).await.unwrap();
            upstream.write_all(&body).await.unwrap();
            let mut response = Vec::new();
            upstream.read_to_end(&mut response).await.unwrap();
            client.write_all(&response).await.unwrap();
            client.shutdown().await.unwrap();
        }
    });
    let dir = tempfile::tempdir().unwrap();
    let scope = Scope {
        owner_id: "provider-owner".into(),
        project_id: "guarded-runner".into(),
        workspace_id: None,
    };
    let declaration = RunnerDeclaration {
        id: "guarded-answer".into(),
        semantic_version: "1.0.0".into(),
        endpoint: endpoint.clone(),
        model_digest: digest.clone(),
        profile: ProviderProfile {
            model_id: "qwen2.5:3b".into(),
            model_revision: digest.clone(),
            tokenizer_id: "ollama-native-counters".into(),
            tokenizer_revision: digest,
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
        system: "Return the requested JSON object with a brief answer.".into(),
        json_schema: serde_json::json!({"type":"object","properties":{"answer":{"type":"string"}},"required":["answer"],"additionalProperties":false}),
        max_prompt_bytes: 8192,
        max_response_bytes: 32768,
        timeout_ms: 120000,
    };
    let mut routes = Vec::new();
    for (id, url, method, kind) in [
        (
            "runner-model",
            endpoint.clone(),
            EgressMethod::Post,
            EgressRouteKind::Destination,
        ),
        (
            "runner-catalog",
            endpoint.replace("/api/chat", "/api/tags"),
            EgressMethod::Get,
            EgressRouteKind::Destination,
        ),
        (
            "proxy",
            format!("http://{proxy_address}/"),
            EgressMethod::Post,
            EgressRouteKind::Proxy,
        ),
    ] {
        let mut route = EgressRoute::for_url(id, kind, &url, BTreeSet::from([method])).unwrap();
        if kind == EgressRouteKind::Proxy {
            route.methods.insert(EgressMethod::Get);
        }
        route
            .allowed_headers
            .extend(["content-type".into(), "authorization".into()]);
        route
            .allowed_non_public_addresses
            .insert("127.0.0.1".parse().unwrap());
        routes.push(route);
    }
    let policy = EgressPolicy {
        version: 1,
        scope: scope.clone(),
        routes,
        proxy: Some(EgressProxy {
            url: format!("http://{proxy_address}/"),
            route_id: "proxy".into(),
        }),
        max_redirects: 0,
        max_request_bytes: 65536,
        max_response_bytes: 65536,
        max_header_bytes: 8192,
        timeout_ms: 120000,
    };
    let secrets = Arc::new(SecretHandleService::new());
    let handle = secrets
        .insert(
            &scope,
            &scope.owner_id,
            Zeroizing::new(CANARY.as_bytes().to_vec()),
        )
        .await
        .unwrap();
    let guard =
        RunnerProviderGuard::new(policy, secrets.clone(), Some(handle), &scope.owner_id).unwrap();
    let actor = ActorEngine::open(ActorConfig {
        actor_directory: dir.path().join("usage"),
        actor: ActorId::new(117),
        user: [17; 16],
        kek: [57; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    })
    .await
    .unwrap();
    configure(
        &actor,
        &scope,
        &scope,
        UsageLimits {
            total_tokens: 8192,
            hourly_tokens: 8192,
            daily_tokens: 8192,
            job_tokens: 4096,
            concurrency: 2,
            lease_ms: 180000,
        },
    )
    .await
    .unwrap();
    let prompt = "Set answer to the exact instrument serial Helios-481.";
    let reservation = reserve(
        &actor,
        &scope,
        &scope,
        ReservationRequest {
            request_id: "guarded-request".into(),
            attribution: UsageAttribution {
                job_id: "guarded-job".into(),
                worker_id: "guarded-model-worker".into(),
                session_id: "guarded-session".into(),
                turn_id: "guarded-turn".into(),
                provider_id: "ollama-local".into(),
                model_id: "qwen2.5:3b".into(),
                source_ids: BTreeSet::from(["owner-input".into()]),
            },
            kind: JobKind::Extraction,
            reserved_tokens: 4096,
            input_digest: digest_bytes(prompt.as_bytes()),
        },
    )
    .await
    .unwrap();
    let usage = CanonicalUsageBinding {
        lease: reservation.lease.clone(),
        reservation_id: reservation.id.clone(),
        input_digest: reservation.input_digest,
        reservation_digest: reservation.request_digest,
        reserved_tokens: reservation.reserved_tokens,
    };
    let mut source_log = hm_ledger::segment::SegmentLog::open(
        dir.path().join("source"),
        ActorId::new(118),
        hm_ledger::segment::SegmentLogOptions::default(),
    )
    .unwrap();
    source_log
        .append_batch(&[hm_ledger::segment::AppendRequest {
            kind: hm_ledger::frame::EventKind::UserMsg,
            wall_timestamp_ns: hm_core::UtcNanos::new(now()),
            conversation: hm_core::ConversationId::new([18; 16]),
            sealed_payload: prompt.as_bytes().to_vec(),
        }])
        .unwrap();
    let source = SourceFence {
        epoch: 1,
        generation: 1,
        digest: digest_bytes(prompt.as_bytes()),
        start: 1,
        end: 2,
    };
    let mut registry =
        DurableRoleRegistry::open(dir.path().join("roles.sqlite"), scope.clone()).unwrap();
    let descriptor = declaration.descriptor().unwrap();
    registry.register(descriptor.clone()).unwrap();
    let held = registry
        .hold(RoleWork {
            id: "guarded-work".into(),
            role_id: descriptor.id.clone(),
            operation: "run".into(),
            capability_version: 1,
            pin: descriptor.pin,
            semantic_version: descriptor.semantic_version,
            source: source.clone(),
            payload: serde_json::to_vec(&RunnerInput {
                prompt: prompt.into(),
                usage: usage.clone(),
            })
            .unwrap(),
            blocks: Vec::new(),
            hook: "runner_dispatch".into(),
            deadline_ns: now() + 120000000000,
        })
        .unwrap();
    registry.approve("guarded-work", held.revision).unwrap();
    let process = ProcessSpec {
        module_id: "guarded-model-worker".into(),
        command: std::env::current_exe().unwrap(),
        args: vec![
            "--exact".into(),
            "guarded_provider_worker_process".into(),
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
    };
    let config = RunnerWorkerConfig {
        root: dir.path().into(),
        registry_path: dir.path().join("roles.sqlite"),
        scope: scope.clone(),
        host_key: [81; 32],
        worker_key: [91; 32],
        declaration: declaration.clone(),
    };
    let mut legacy_process = process.clone();
    legacy_process.command = PathBuf::from(
        std::env::var("HM_FABRIC_LEGACY_RUNNER_BIN").expect("actual legacy runner binary required"),
    );
    legacy_process.args = vec!["fabric-runner-worker".into()];
    let help = std::process::Command::new(&legacy_process.command)
        .args(["fabric-runner-worker", "--help"])
        .output()
        .unwrap();
    assert!(help.status.success());
    assert!(
        String::from_utf8_lossy(&help.stdout)
            .contains("Run an authenticated worker for an approved model runner")
    );
    let legacy = RunnerWorkerSession::launch_guarded(
        legacy_process,
        config.clone(),
        &registry,
        Limits {
            io_timeout: Duration::from_secs(5),
            ..Limits::default()
        },
        guard.clone(),
    )
    .await;
    match legacy {
        Err(error) => assert!(
            error.0.contains("registration pin mismatch"),
            "unexpected legacy refusal: {error}"
        ),
        Ok(mut legacy) => {
            legacy.shutdown().unwrap();
            panic!("legacy worker admitted under broker contract");
        }
    }
    assert_eq!(
        registry.get("guarded-work").unwrap().unwrap().state,
        WorkState::Approved
    );
    assert!(captured.lock().unwrap().is_empty());
    let mut worker = RunnerWorkerSession::launch_guarded(
        process.clone(),
        config.clone(),
        &registry,
        Limits {
            io_timeout: Duration::from_secs(130),
            ..Limits::default()
        },
        guard,
    )
    .await
    .unwrap();
    let stale_source = SourceFence {
        digest: digest_bytes(b"changed source"),
        ..source.clone()
    };
    assert!(
        worker
            .submit(&mut registry, "guarded-work", &stale_source)
            .await
            .is_err()
    );
    assert!(captured.lock().unwrap().is_empty());
    let ticket = worker
        .submit(&mut registry, "guarded-work", &source)
        .await
        .unwrap();
    let (record, receipt) = worker
        .receive_with_receipt(&mut registry, &source)
        .await
        .unwrap();
    assert!(matches!(
        record.output.as_ref().unwrap().terminal,
        RunTerminal::Completed { .. }
    ));
    let outcome = receipt.outcome();
    assert_eq!(outcome.value["answer"], "Helios-481");
    assert_eq!(outcome.usage, usage);
    assert_eq!(outcome.request_digest, ticket.work_digest);
    assert_eq!(
        outcome.observation.original_bytes,
        outcome.original_provider_bytes
    );
    assert!(outcome.observation.tokens.input.unwrap() > 0);
    assert!(outcome.observation.tokens.output.unwrap() > 0);
    assert!(captured.lock().unwrap().len() >= 2);
    for headers in captured.lock().unwrap().iter() {
        assert!(headers.contains(CANARY));
    }
    for bytes in [
        serde_json::to_vec(&record).unwrap(),
        serde_json::to_vec(outcome).unwrap(),
        serde_json::to_vec(&secrets.inventory(&scope, &scope.owner_id).await.unwrap()).unwrap(),
        std::fs::read(dir.path().join("roles.sqlite")).unwrap(),
    ] {
        assert!(
            !bytes
                .windows(CANARY.len())
                .any(|window| window == CANARY.as_bytes())
        );
    }
    let state = inspect(&actor, &scope, &scope).await.unwrap();
    assert_eq!(state.reservations[&reservation.id].settled_tokens, None);
    assert!(state.observations.is_empty());
    worker.shutdown().unwrap();
    let default_prompt = "Set answer to the exact instrument serial Vega-602.";
    let default_reservation = reserve(
        &actor,
        &scope,
        &scope,
        ReservationRequest {
            request_id: "default-request".into(),
            attribution: UsageAttribution {
                job_id: "default-job".into(),
                worker_id: "guarded-model-worker".into(),
                session_id: "default-session".into(),
                turn_id: "default-turn".into(),
                provider_id: "ollama-local".into(),
                model_id: "qwen2.5:3b".into(),
                source_ids: BTreeSet::from(["owner-default-input".into()]),
            },
            kind: JobKind::Extraction,
            reserved_tokens: 4096,
            input_digest: digest_bytes(default_prompt.as_bytes()),
        },
    )
    .await
    .unwrap();
    let default_usage = CanonicalUsageBinding {
        lease: default_reservation.lease.clone(),
        reservation_id: default_reservation.id.clone(),
        input_digest: default_reservation.input_digest,
        reservation_digest: default_reservation.request_digest,
        reserved_tokens: default_reservation.reserved_tokens,
    };
    source_log
        .append_batch(&[hm_ledger::segment::AppendRequest {
            kind: hm_ledger::frame::EventKind::UserMsg,
            wall_timestamp_ns: hm_core::UtcNanos::new(now()),
            conversation: hm_core::ConversationId::new([18; 16]),
            sealed_payload: default_prompt.as_bytes().to_vec(),
        }])
        .unwrap();
    let default_source = SourceFence {
        epoch: 1,
        generation: 1,
        digest: digest_bytes(default_prompt.as_bytes()),
        start: 2,
        end: 3,
    };
    let descriptor = declaration.descriptor().unwrap();
    let held = registry
        .hold(RoleWork {
            id: "default-work".into(),
            role_id: descriptor.id,
            operation: "run".into(),
            capability_version: 1,
            pin: descriptor.pin,
            semantic_version: descriptor.semantic_version,
            source: default_source.clone(),
            payload: serde_json::to_vec(&RunnerInput {
                prompt: default_prompt.into(),
                usage: default_usage.clone(),
            })
            .unwrap(),
            blocks: Vec::new(),
            hook: "runner_dispatch".into(),
            deadline_ns: now() + 120000000000,
        })
        .unwrap();
    registry.approve("default-work", held.revision).unwrap();
    let mut default_worker = RunnerWorkerSession::launch(
        process,
        config,
        &registry,
        Limits {
            io_timeout: Duration::from_secs(130),
            ..Limits::default()
        },
    )
    .await
    .unwrap();
    default_worker
        .submit(&mut registry, "default-work", &default_source)
        .await
        .unwrap();
    let (_, default_receipt) = default_worker
        .receive_with_receipt(&mut registry, &default_source)
        .await
        .unwrap();
    assert_eq!(default_receipt.outcome().value["answer"], "Vega-602");
    assert_eq!(default_receipt.outcome().usage, default_usage);
    assert_eq!(
        default_receipt.outcome().observation.original_bytes,
        default_receipt.outcome().original_provider_bytes
    );
    assert!(default_receipt.outcome().observation.tokens.input.unwrap() > 0);
    let final_state = inspect(&actor, &scope, &scope).await.unwrap();
    assert_eq!(
        final_state.reservations[&reservation.id].settled_tokens,
        None
    );
    assert_eq!(
        final_state.reservations[&default_reservation.id].settled_tokens,
        None
    );
    assert!(final_state.observations.is_empty());
    default_worker.shutdown().unwrap();
    actor.shutdown().await.unwrap();
    proxy.abort();
}
