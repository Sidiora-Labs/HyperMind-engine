use hm_context::Scope;
use hm_fabric::{egress::*, secret_handles::*};
use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};
use zeroize::Zeroizing;
const CANARY: &str = "Bearer native-secret-canary-7b390f18";
fn scope() -> Scope {
    Scope {
        owner_id: "owner".into(),
        project_id: "project".into(),
        workspace_id: None,
    }
}
fn request(url: &str, method: EgressMethod) -> EgressRequest {
    EgressRequest {
        url: url.into(),
        method,
        headers: Vec::new(),
        body: Vec::new(),
    }
}
fn grant(
    scope: &Scope,
    handle: &SecretHandle,
    id: &str,
    operation: &str,
    revision: u64,
) -> SecretGrant {
    SecretGrant {
        id: id.into(),
        scope: scope.clone(),
        principal: "worker".into(),
        principal_revision: revision,
        handle: handle.clone(),
        operation_id: operation.into(),
        request_digest: String::new(),
        policy_digest: String::new(),
        route_id: "tool".into(),
        header: "authorization".into(),
        expires_unix_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
            + 60000,
        remaining_uses: 1,
    }
}
#[test]
fn credential_dispatch_process_exports_no_secret_bytes() {
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "secret_dispatch_child",
            "--nocapture",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for bytes in [&output.stdout, &output.stderr] {
        assert!(
            !bytes
                .windows(CANARY.len())
                .any(|window| window == CANARY.as_bytes())
        );
    }
    assert!(String::from_utf8_lossy(&output.stdout).contains("native dispatch evidence"));
}
#[tokio::test]
#[ignore = "run in an isolated process by credential_dispatch_process_exports_no_secret_bytes"]
async fn secret_dispatch_child() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let wire = Arc::new(Mutex::new(Vec::new()));
    let captured = wire.clone();
    let server = tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            loop {
                let mut byte = [0];
                if socket.read(&mut byte).await.unwrap() == 0 {
                    break;
                }
                bytes.push(byte[0]);
                if bytes.ends_with(b"\r\n\r\n") {
                    break;
                }
                assert!(bytes.len() < 65536);
            }
            captured.lock().unwrap().push(bytes.clone());
            let redirect = bytes.starts_with(b"POST");
            let response = if redirect {
                format!(
                    "HTTP/1.1 307 Temporary Redirect\r\nLocation: /second\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                )
            } else {
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{CANARY}",
                    CANARY.len()
                )
            };
            socket.write_all(response.as_bytes()).await.unwrap();
            socket.shutdown().await.unwrap();
        }
    });
    let scope = scope();
    let url = format!("http://{address}/");
    let mut route = EgressRoute::for_url(
        "tool",
        EgressRouteKind::Destination,
        &url,
        BTreeSet::from([EgressMethod::Get, EgressMethod::Post]),
    )
    .unwrap();
    route.allowed_headers.insert("authorization".into());
    route.allowed_non_public_addresses.insert(address.ip());
    let policy = EgressPolicy {
        version: 1,
        scope: scope.clone(),
        routes: vec![route],
        proxy: None,
        max_redirects: 3,
        max_request_bytes: 1024,
        max_response_bytes: 1024,
        max_header_bytes: 4096,
        timeout_ms: 2000,
    };
    let client = EgressHttpClient::new(policy.clone()).unwrap();
    let service = SecretHandleService::new();
    assert_eq!(
        service
            .set_principal(&scope, "worker", "worker", Some(1))
            .await
            .unwrap_err(),
        SecretError::Denied
    );
    service
        .set_principal(&scope, "owner", "worker", Some(1))
        .await
        .unwrap();
    let handle = service
        .insert(&scope, "owner", Zeroizing::new(CANARY.as_bytes().to_vec()))
        .await
        .unwrap();
    let original = grant(&scope, &handle, "grant-one", "operation", 1);
    assert_eq!(
        service
            .grant(
                &scope,
                "worker",
                original.clone(),
                &client,
                &request(&url, EgressMethod::Get)
            )
            .await
            .unwrap_err(),
        SecretError::Denied
    );
    service
        .grant(
            &scope,
            "owner",
            original,
            &client,
            &request(&url, EgressMethod::Get),
        )
        .await
        .unwrap();
    let ids = vec!["grant-one".into()];
    for (principal, revision, operation) in [
        ("other", 1, "operation"),
        ("worker", 2, "operation"),
        ("worker", 1, "different"),
    ] {
        let error = service
            .dispatch(
                &client,
                &scope,
                principal,
                revision,
                operation,
                request(&url, EgressMethod::Get),
                &ids,
            )
            .await
            .unwrap_err();
        println!("denied metadata {error:?}");
    }
    let mut changed = request(&url, EgressMethod::Get);
    changed.url.push_str("changed");
    assert_eq!(
        service
            .dispatch(&client, &scope, "worker", 1, "operation", changed, &ids)
            .await
            .unwrap_err(),
        SecretError::Denied
    );
    let mut changed_policy = policy.clone();
    changed_policy.max_response_bytes = 2048;
    let changed_client = EgressHttpClient::new(changed_policy).unwrap();
    assert_eq!(
        service
            .dispatch(
                &changed_client,
                &scope,
                "worker",
                1,
                "operation",
                request(&url, EgressMethod::Get),
                &ids
            )
            .await
            .unwrap_err(),
        SecretError::Denied
    );
    let other_scope = Scope {
        project_id: "other-project".into(),
        ..scope.clone()
    };
    assert_eq!(
        service
            .dispatch(
                &client,
                &other_scope,
                "worker",
                1,
                "operation",
                request(&url, EgressMethod::Get),
                &ids
            )
            .await
            .unwrap_err(),
        SecretError::Denied
    );
    let mut exposed = request(&url, EgressMethod::Get);
    exposed
        .headers
        .push(("authorization".into(), CANARY.as_bytes().to_vec()));
    assert_eq!(
        service
            .dispatch(&client, &scope, "worker", 1, "operation", exposed, &ids)
            .await
            .unwrap_err(),
        SecretError::Denied
    );
    assert!(wire.lock().unwrap().is_empty());
    let receipt = service
        .dispatch(
            &client,
            &scope,
            "worker",
            1,
            "operation",
            request(&url, EgressMethod::Get),
            &ids,
        )
        .await
        .unwrap();
    assert_eq!(receipt.status, 200);
    assert_eq!(receipt.egress.connected_peer, address);
    assert!(
        wire.lock().unwrap()[0]
            .windows(CANARY.len())
            .any(|window| window == CANARY.as_bytes())
    );
    assert_eq!(wire.lock().unwrap().len(), 1);
    assert_eq!(
        service
            .dispatch(
                &client,
                &scope,
                "worker",
                1,
                "operation",
                request(&url, EgressMethod::Get),
                &ids
            )
            .await
            .unwrap_err(),
        SecretError::Denied
    );
    let export = serde_json::to_string(&service.inventory(&scope, "owner").await.unwrap()).unwrap();
    let evidence = serde_json::to_string(&receipt).unwrap();
    for text in [&export, &evidence, &format!("{service:?}")] {
        assert!(!text.contains(CANARY));
        println!("native dispatch evidence {text}");
    }
    let mut leak = grant(&scope, &handle, CANARY, "operation", 1);
    assert!(
        service
            .grant(
                &scope,
                "owner",
                leak.clone(),
                &client,
                &request(&url, EgressMethod::Get)
            )
            .await
            .is_err()
    );
    leak.id = "revocable".into();
    service
        .grant(
            &scope,
            "owner",
            leak,
            &client,
            &request(&url, EgressMethod::Get),
        )
        .await
        .unwrap();
    service
        .set_principal(&scope, "owner", "worker", Some(2))
        .await
        .unwrap();
    assert_eq!(
        service
            .dispatch(
                &client,
                &scope,
                "worker",
                1,
                "operation",
                request(&url, EgressMethod::Get),
                &["revocable".into()]
            )
            .await
            .unwrap_err(),
        SecretError::Denied
    );
    let post = grant(&scope, &handle, "write", "write-operation", 2);
    service
        .grant(
            &scope,
            "owner",
            post,
            &client,
            &request(&url, EgressMethod::Post),
        )
        .await
        .unwrap();
    let error = service
        .dispatch(
            &client,
            &scope,
            "worker",
            2,
            "write-operation",
            request(&url, EgressMethod::Post),
            &["write".into()],
        )
        .await
        .unwrap_err();
    assert_eq!(error, SecretError::Egress(EgressError::OutcomeUncertain));
    println!("uncertain metadata {error:?}");
    assert_eq!(wire.lock().unwrap().len(), 2);
    assert_eq!(
        service
            .dispatch(
                &client,
                &scope,
                "worker",
                2,
                "write-operation",
                request(&url, EgressMethod::Post),
                &["write".into()]
            )
            .await
            .unwrap_err(),
        SecretError::Denied
    );
    service
        .grant(
            &scope,
            "owner",
            grant(&scope, &handle, "revoke", "operation", 2),
            &client,
            &request(&url, EgressMethod::Get),
        )
        .await
        .unwrap();
    service
        .revoke_handle(&scope, "owner", &handle)
        .await
        .unwrap();
    assert_eq!(
        service
            .dispatch(
                &client,
                &scope,
                "worker",
                2,
                "operation",
                request(&url, EgressMethod::Get),
                &["revoke".into()]
            )
            .await
            .unwrap_err(),
        SecretError::Denied
    );
    assert!(
        service
            .inventory(&scope, "owner")
            .await
            .unwrap()
            .handles
            .is_empty()
    );
    let fresh = SecretHandleService::new();
    assert_eq!(
        fresh
            .dispatch(
                &client,
                &scope,
                "worker",
                2,
                "operation",
                request(&url, EgressMethod::Get),
                &["revoke".into()]
            )
            .await
            .unwrap_err(),
        SecretError::Denied
    );
    assert_eq!(wire.lock().unwrap().len(), 2);
    server.abort();
}
