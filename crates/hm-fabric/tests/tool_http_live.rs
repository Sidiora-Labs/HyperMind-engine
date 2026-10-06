use hm_context::types::{Scope, digest_bytes};
use hm_core::{ActorId, ConversationId, UtcNanos};
use hm_fabric::{
    egress::*,
    role_store::*,
    role_workers::{
        ToolDeclaration, ToolProcessOutcome, ToolWorkerConfig, ToolWorkerSession,
        tool_worker_from_env,
    },
    secret_handles::SecretHandleService,
    supervisor::{Probe, ProcessSpec, RestartPolicy},
    tool_http::*,
    transport::Limits,
};
use hm_ledger::{
    frame::EventKind,
    segment::{AppendRequest, SegmentLog, SegmentLogOptions},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::Notify,
};
use zeroize::Zeroizing;
const CANARY: &str = "Bearer native-http-tool-canary-4857ebda";
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as i64
}
fn scope() -> Scope {
    Scope {
        owner_id: "http-owner".into(),
        project_id: "http-tools".into(),
        workspace_id: None,
    }
}
fn process() -> ProcessSpec {
    ProcessSpec {
        module_id: "native-http-worker".into(),
        command: std::env::current_exe().unwrap(),
        args: vec![
            "--exact".into(),
            "http_tool_worker_process".into(),
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
    }
}
fn config(root: &Path, declaration: &HttpToolDeclaration) -> HttpToolWorkerConfig {
    HttpToolWorkerConfig {
        root: root.into(),
        registry_path: root.join("roles.sqlite"),
        scope: scope(),
        host_key: [19; 32],
        worker_key: [39; 32],
        declaration: declaration.clone(),
    }
}
fn limits() -> Limits {
    Limits {
        io_timeout: Duration::from_secs(5),
        ..Limits::default()
    }
}
fn request(url: &str, path: &str, body: &[u8]) -> HttpToolRequest {
    HttpToolRequest {
        url: format!("{url}{path}"),
        method: if path == "echo" {
            EgressMethod::Get
        } else {
            EgressMethod::Post
        },
        headers: Vec::new(),
        body: body.into(),
    }
}
async fn read_request(socket: &mut TcpStream) -> (String, Vec<u8>) {
    let mut bytes = Vec::new();
    loop {
        let mut one = [0];
        assert_eq!(socket.read(&mut one).await.unwrap(), 1);
        bytes.push(one[0]);
        if bytes.ends_with(b"\r\n\r\n") {
            break;
        }
        assert!(bytes.len() < 32768);
    }
    let headers = String::from_utf8(bytes).unwrap();
    let length = headers
        .lines()
        .find_map(|line| {
            line.split_once(':')
                .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                .map(|(_, value)| value.trim().parse::<usize>().unwrap())
        })
        .unwrap_or(0);
    let mut body = vec![0; length];
    socket.read_exact(&mut body).await.unwrap();
    (headers, body)
}
async fn hold(
    registry: &mut DurableRoleRegistry,
    authority: &HttpToolAuthority,
    declaration: &HttpToolDeclaration,
    source: &SourceFence,
    id: &str,
    request: &HttpToolRequest,
) -> u64 {
    let descriptor = declaration.descriptor().unwrap();
    registry.register(descriptor.clone()).unwrap();
    registry
        .hold(RoleWork {
            id: id.into(),
            role_id: descriptor.id,
            operation: "invoke".into(),
            capability_version: 1,
            pin: descriptor.pin,
            semantic_version: descriptor.semantic_version,
            source: source.clone(),
            payload: authority
                .encode_request(request, declaration)
                .await
                .unwrap(),
            blocks: Vec::new(),
            hook: "tool_dispatch".into(),
            deadline_ns: now() + 20000000000,
        })
        .unwrap()
        .revision
}
fn no_canary(bytes: &[u8]) {
    assert!(
        !bytes
            .windows(CANARY.len())
            .any(|window| window == CANARY.as_bytes())
    );
}
#[tokio::test]
#[ignore]
async fn http_tool_worker_process() {
    tool_worker_from_env().await.unwrap();
}
#[tokio::test]
async fn actual_http_effects_scoped_dispatch_restart_and_process_protocol() {
    let root = tempfile::tempdir().unwrap();
    let destination = root.path().join("effects.bin");
    std::fs::write(&destination, b"").unwrap();
    let mut source_log = SegmentLog::open(
        root.path().join("source"),
        ActorId::new(123),
        SegmentLogOptions::default(),
    )
    .unwrap();
    source_log
        .append_batch(&[AppendRequest {
            kind: EventKind::UserMsg,
            wall_timestamp_ns: UtcNanos::new(now()),
            conversation: ConversationId::new([23; 16]),
            sealed_payload: b"owner HTTP tool request".to_vec(),
        }])
        .unwrap();
    let source = SourceFence {
        epoch: 1,
        generation: 1,
        digest: digest_bytes(b"owner HTTP tool request"),
        start: 1,
        end: 2,
    };
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let url = format!("http://{address}/");
    let target = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target_address = target.local_addr().unwrap();
    let target_count = Arc::new(Mutex::new(0));
    let target_seen = target_count.clone();
    let redirect_target = tokio::spawn(async move {
        loop {
            let (mut socket, _) = target.accept().await.unwrap();
            *target_seen.lock().unwrap() += 1;
            let _ = read_request(&mut socket).await;
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .await
                .unwrap();
        }
    });
    let target_wire = Arc::new(Mutex::new(Vec::new()));
    let captured = target_wire.clone();
    let file = destination.clone();
    let release = Arc::new(Notify::new());
    let unblock = release.clone();
    let endpoint = tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let (headers, body) = read_request(&mut socket).await;
            let path = headers.split_whitespace().nth(1).unwrap().to_owned();
            captured.lock().unwrap().push(headers.clone());
            if path != "/echo" {
                use std::io::Write;
                let mut output = std::fs::OpenOptions::new()
                    .append(true)
                    .open(&file)
                    .unwrap();
                output.write_all(&body).unwrap();
                output.sync_all().unwrap();
            }
            if path == "/drop" {
                let _ = socket.shutdown().await;
                continue;
            }
            if path == "/slow" {
                unblock.notified().await;
                let _ = socket.shutdown().await;
                continue;
            }
            let response = if path == "/redirect" {
                format!(
                    "HTTP/1.1 307 Temporary Redirect\r\nLocation: http://{target_address}/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                )
            } else {
                let bytes = if path == "/echo" {
                    headers
                        .lines()
                        .find_map(|line| {
                            line.split_once(':')
                                .filter(|(name, _)| name.eq_ignore_ascii_case("authorization"))
                                .map(|(_, value)| value.trim().as_bytes().to_vec())
                        })
                        .unwrap()
                } else {
                    serde_json::to_vec(&serde_json::json!({"file_digest":digest_bytes(&std::fs::read(&file).unwrap()),"bytes":std::fs::metadata(&file).unwrap().len()})).unwrap()
                };
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    bytes.len()
                );
                socket.write_all(head.as_bytes()).await.unwrap();
                socket.write_all(&bytes).await.unwrap();
                let _ = socket.shutdown().await;
                continue;
            };
            socket.write_all(response.as_bytes()).await.unwrap();
            let _ = socket.shutdown().await;
        }
    });
    let proxy_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_address = proxy_listener.local_addr().unwrap();
    let proxy_wire = Arc::new(Mutex::new(Vec::new()));
    let proxy_capture = proxy_wire.clone();
    let proxy = tokio::spawn(async move {
        loop {
            let (mut socket, _) = proxy_listener.accept().await.unwrap();
            let (headers, body) = read_request(&mut socket).await;
            proxy_capture.lock().unwrap().push(headers.clone());
            let first = headers.lines().next().unwrap();
            let mut parts = first.split_whitespace();
            let method = parts.next().unwrap();
            let uri = reqwest::Url::parse(parts.next().unwrap()).unwrap();
            assert_eq!(uri.host_str(), Some("127.0.0.1"));
            assert_eq!(uri.port(), Some(address.port()));
            let mut upstream = TcpStream::connect(address).await.unwrap();
            let mut request = format!("{method} {} HTTP/1.1\r\n", uri.path());
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
            if !response.is_empty() {
                socket.write_all(&response).await.unwrap();
            }
            let _ = socket.shutdown().await;
        }
    });
    let mut routes = Vec::new();
    for (id, route_url, kind) in [
        ("http-target", url.clone(), EgressRouteKind::Destination),
        (
            "http-proxy",
            format!("http://{proxy_address}/"),
            EgressRouteKind::Proxy,
        ),
    ] {
        let mut route = EgressRoute::for_url(
            id,
            kind,
            &route_url,
            BTreeSet::from([EgressMethod::Post, EgressMethod::Get]),
        )
        .unwrap();
        route.allowed_headers.insert("authorization".into());
        route.allowed_non_public_addresses.insert(address.ip());
        routes.push(route);
    }
    let policy = EgressPolicy {
        version: 1,
        scope: scope(),
        routes,
        proxy: Some(EgressProxy {
            url: format!("http://{proxy_address}/"),
            route_id: "http-proxy".into(),
        }),
        max_redirects: 3,
        max_request_bytes: 8192,
        max_response_bytes: 8192,
        max_header_bytes: 8192,
        timeout_ms: 2000,
    };
    let secrets = Arc::new(SecretHandleService::new());
    let handle = secrets
        .insert(
            &scope(),
            "http-owner",
            Zeroizing::new(CANARY.as_bytes().to_vec()),
        )
        .await
        .unwrap();
    let authority = HttpToolAuthority::new(
        policy.clone(),
        secrets.clone(),
        Some((handle.clone(), "authorization".into())),
        "http-owner",
    )
    .unwrap();
    let declaration =
        HttpToolDeclaration::for_policy("native-http", "1.0.0", &policy, 8192, 8192, 3000).unwrap();
    let configuration = config(root.path(), &declaration);
    let mut registry =
        DurableRoleRegistry::open(root.path().join("roles.sqlite"), scope()).unwrap();
    let mut raw_secret = request(&url, "write", CANARY.as_bytes());
    assert!(
        authority
            .encode_request(&raw_secret, &declaration)
            .await
            .is_err()
    );
    raw_secret.body = Vec::new();
    raw_secret
        .headers
        .push(("authorization".into(), CANARY.as_bytes().to_vec()));
    assert!(
        authority
            .encode_request(&raw_secret, &declaration)
            .await
            .is_err()
    );
    let revision = hold(
        &mut registry,
        &authority,
        &declaration,
        &source,
        "write-once",
        &request(&url, "write", b"one-write\n"),
    )
    .await;
    let mut worker = HttpToolWorkerSession::launch(
        process(),
        configuration.clone(),
        &registry,
        limits(),
        authority.clone(),
    )
    .await
    .unwrap();
    assert!(
        worker
            .execute(&mut registry, "write-once", &source)
            .await
            .is_err()
    );
    assert!(target_wire.lock().unwrap().is_empty());
    registry.approve("write-once", revision).unwrap();
    let mut stale = source.clone();
    stale.generation += 1;
    assert!(
        worker
            .submit(&mut registry, "write-once", &stale)
            .await
            .is_err()
    );
    assert!(target_wire.lock().unwrap().is_empty());
    let pid = worker.worker_pid().unwrap();
    for path in [
        format!("/proc/{pid}/cmdline"),
        format!("/proc/{pid}/environ"),
    ] {
        no_canary(&std::fs::read(path).unwrap());
    }
    let original = worker
        .execute(&mut registry, "write-once", &source)
        .await
        .unwrap();
    assert_eq!(original.state, WorkState::Terminal);
    let outcome: HttpToolOutcome =
        serde_json::from_slice(&original.output.as_ref().unwrap().payload).unwrap();
    assert_eq!(outcome.egress.connected_peer, proxy_address);
    assert_eq!(outcome.status, 200);
    assert_eq!(outcome.body_digest, digest_bytes(&outcome.original_body));
    assert_eq!(std::fs::read(&destination).unwrap(), b"one-write\n");
    assert_eq!(
        worker
            .execute(&mut registry, "write-once", &source)
            .await
            .unwrap(),
        original
    );
    assert_eq!(target_wire.lock().unwrap().len(), 1);
    no_canary(&serde_json::to_vec(&original).unwrap());
    no_canary(
        &serde_json::to_vec(&secrets.inventory(&scope(), "http-owner").await.unwrap()).unwrap(),
    );
    no_canary(&worker.stderr_tail());
    assert!(target_wire.lock().unwrap()[0].contains(CANARY));
    assert!(proxy_wire.lock().unwrap()[0].contains(CANARY));
    let granted = secrets
        .inventory(&scope(), "http-owner")
        .await
        .unwrap()
        .grants[0]
        .clone();
    secrets
        .set_principal(
            &scope(),
            "http-owner",
            &granted.principal,
            Some(granted.principal_revision + 1),
        )
        .await
        .unwrap();
    let revision = hold(
        &mut registry,
        &authority,
        &declaration,
        &source,
        "revoked-principal",
        &request(&url, "write", b"forbidden\n"),
    )
    .await;
    registry.approve("revoked-principal", revision).unwrap();
    assert_eq!(
        worker
            .execute(&mut registry, "revoked-principal", &source)
            .await
            .unwrap_err(),
        HttpToolError::Uncertain
    );
    assert_eq!(target_wire.lock().unwrap().len(), 1);
    worker.shutdown().await.unwrap();
    drop(worker);
    for (id, path, body) in [
        ("redirect-write", "redirect", b"redirect-once\n".as_slice()),
        ("dropped-write", "drop", b"drop-once\n".as_slice()),
        ("credential-echo", "echo", b"".as_slice()),
    ] {
        let revision = hold(
            &mut registry,
            &authority,
            &declaration,
            &source,
            id,
            &request(&url, path, body),
        )
        .await;
        registry.approve(id, revision).unwrap();
        let mut worker = HttpToolWorkerSession::launch(
            process(),
            configuration.clone(),
            &registry,
            limits(),
            authority.clone(),
        )
        .await
        .unwrap();
        assert_eq!(
            worker
                .execute(&mut registry, id, &source)
                .await
                .unwrap_err(),
            HttpToolError::Uncertain
        );
        assert_eq!(
            registry.get(id).unwrap().unwrap().state,
            WorkState::Uncertain
        );
        assert!(registry.get(id).unwrap().unwrap().output.is_none());
        no_canary(&worker.stderr_tail());
        worker.shutdown().await.unwrap();
    }
    assert_eq!(*target_count.lock().unwrap(), 0);
    assert_eq!(
        std::fs::read(&destination).unwrap(),
        b"one-write\nredirect-once\ndrop-once\n"
    );
    let revision = hold(
        &mut registry,
        &authority,
        &declaration,
        &source,
        "killed-write",
        &request(&url, "slow", b"kill-once\n"),
    )
    .await;
    registry.approve("killed-write", revision).unwrap();
    let mut interrupted = HttpToolWorkerSession::launch(
        process(),
        configuration.clone(),
        &registry,
        limits(),
        authority.clone(),
    )
    .await
    .unwrap();
    interrupted
        .submit(&mut registry, "killed-write", &source)
        .await
        .unwrap();
    for _ in 0..100 {
        if std::fs::read(&destination)
            .unwrap()
            .ends_with(b"kill-once\n")
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert!(
        std::fs::read(&destination)
            .unwrap()
            .ends_with(b"kill-once\n")
    );
    interrupted.kill_worker().unwrap();
    drop(interrupted);
    release.notify_one();
    drop(registry);
    let mut registry =
        DurableRoleRegistry::open(root.path().join("roles.sqlite"), scope()).unwrap();
    assert_eq!(
        registry.get("killed-write").unwrap().unwrap().state,
        WorkState::Uncertain
    );
    let mut fresh = HttpToolWorkerSession::launch(
        process(),
        configuration.clone(),
        &registry,
        limits(),
        authority.clone(),
    )
    .await
    .unwrap();
    let count = target_wire.lock().unwrap().len();
    for id in [
        "redirect-write",
        "dropped-write",
        "credential-echo",
        "killed-write",
        "revoked-principal",
    ] {
        assert_eq!(
            fresh.submit(&mut registry, id, &source).await.unwrap_err(),
            HttpToolError::Uncertain
        );
    }
    assert_eq!(
        fresh
            .execute(&mut registry, "write-once", &source)
            .await
            .unwrap(),
        original
    );
    assert_eq!(target_wire.lock().unwrap().len(), count);
    fresh.shutdown().await.unwrap();
    no_canary(&std::fs::read(root.path().join("roles.sqlite")).unwrap());
    let mut denied_policy = policy.clone();
    denied_policy.routes[0].allowed_non_public_addresses.clear();
    let denied_authority =
        HttpToolAuthority::new(denied_policy.clone(), secrets.clone(), None, "http-owner").unwrap();
    let denied_declaration =
        HttpToolDeclaration::for_policy("denied-http", "1.0.0", &denied_policy, 8192, 8192, 3000)
            .unwrap();
    assert_ne!(
        denied_declaration.descriptor().unwrap().pin,
        declaration.descriptor().unwrap().pin
    );
    let revision = hold(
        &mut registry,
        &denied_authority,
        &denied_declaration,
        &source,
        "denied-write",
        &request(&url, "write", b"no-write\n"),
    )
    .await;
    registry.approve("denied-write", revision).unwrap();
    let mut denied = HttpToolWorkerSession::launch(
        process(),
        config(root.path(), &denied_declaration),
        &registry,
        limits(),
        denied_authority,
    )
    .await
    .unwrap();
    assert_eq!(
        denied
            .execute(&mut registry, "denied-write", &source)
            .await
            .unwrap_err(),
        HttpToolError::Denied
    );
    assert_eq!(
        registry.get("denied-write").unwrap().unwrap().state,
        WorkState::Approved
    );
    assert_eq!(target_wire.lock().unwrap().len(), count);
    denied.shutdown().await.unwrap();
    let process_file = root.path().join("process.txt");
    let process_declaration = ToolDeclaration::admit(
        "original-process-tool",
        "/usr/bin/tee",
        vec![process_file.to_string_lossy().into_owned()],
        root.path(),
        8192,
        8192,
        2000,
    )
    .unwrap();
    let descriptor = process_declaration.descriptor().unwrap();
    registry.register(descriptor.clone()).unwrap();
    let held = registry
        .hold(RoleWork {
            id: "process-work".into(),
            role_id: descriptor.id,
            pin: descriptor.pin,
            semantic_version: descriptor.semantic_version,
            operation: "invoke".into(),
            capability_version: 1,
            source: source.clone(),
            payload: b"original process protocol\n".to_vec(),
            blocks: Vec::new(),
            hook: "tool_dispatch".into(),
            deadline_ns: now() + 10000000000,
        })
        .unwrap();
    registry.approve("process-work", held.revision).unwrap();
    let mut process_worker = ToolWorkerSession::launch(
        process(),
        ToolWorkerConfig {
            root: root.path().into(),
            registry_path: root.path().join("roles.sqlite"),
            scope: scope(),
            host_key: [29; 32],
            worker_key: [49; 32],
            declaration: process_declaration,
        },
        &registry,
        limits(),
    )
    .await
    .unwrap();
    let record = process_worker
        .execute(&mut registry, "process-work", &source)
        .await
        .unwrap();
    let process_result: ToolProcessOutcome =
        serde_json::from_slice(&record.output.unwrap().payload).unwrap();
    assert_eq!(process_result.stdout, b"original process protocol\n");
    assert_eq!(std::fs::read(process_file).unwrap(), process_result.stdout);
    process_worker.shutdown().await.unwrap();
    let original_data = source_log.read_all().unwrap();
    assert_eq!(
        digest_bytes(&original_data[0].sealed_payload),
        source.digest
    );
    proxy.abort();
    endpoint.abort();
    redirect_target.abort();
}
