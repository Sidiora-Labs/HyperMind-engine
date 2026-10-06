use hm_context::Scope;
use hm_fabric::egress::*;
use std::{
    collections::BTreeSet,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

async fn endpoint(
    location: Option<String>,
    proxy: bool,
) -> (
    SocketAddr,
    Arc<Mutex<Vec<String>>>,
    tokio::task::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let captures = Arc::new(Mutex::new(Vec::new()));
    let captured = captures.clone();
    let task = tokio::spawn(async move {
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
            let text = String::from_utf8(bytes).unwrap();
            captured.lock().unwrap().push(text.clone());
            if proxy {
                let target = reqwest::Url::parse(text.split_whitespace().nth(1).unwrap()).unwrap();
                let mut upstream = tokio::net::TcpStream::connect((
                    target.host_str().unwrap(),
                    target.port_or_known_default().unwrap(),
                ))
                .await
                .unwrap();
                let first_end = text.find("\r\n").unwrap();
                let relative = format!("GET {} HTTP/1.1{}", target.path(), &text[first_end..]);
                upstream.write_all(relative.as_bytes()).await.unwrap();
                let mut response = Vec::new();
                upstream.read_to_end(&mut response).await.unwrap();
                socket.write_all(&response).await.unwrap();
            } else {
                let response = if let Some(location) = &location {
                    format!(
                        "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    )
                } else {
                    "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok".into()
                };
                socket.write_all(response.as_bytes()).await.unwrap();
            }
            socket.shutdown().await.unwrap();
        }
    });
    (address, captures, task)
}
fn route(id: &str, url: &str, kind: EgressRouteKind, local: bool) -> EgressRoute {
    let mut route = EgressRoute::for_url(
        id,
        kind,
        url,
        BTreeSet::from([EgressMethod::Get, EgressMethod::Post]),
    )
    .unwrap();
    route.allowed_headers.insert("x-canary".into());
    if local {
        route.allowed_non_public_addresses.extend([
            "127.0.0.1".parse::<std::net::IpAddr>().unwrap(),
            "::1".parse::<std::net::IpAddr>().unwrap(),
        ]);
    }
    route
}
fn policy(routes: Vec<EgressRoute>) -> EgressPolicy {
    EgressPolicy {
        version: 1,
        scope: Scope {
            owner_id: "owner".into(),
            project_id: "project".into(),
            workspace_id: None,
        },
        routes,
        proxy: None,
        max_redirects: 3,
        max_request_bytes: 1024,
        max_response_bytes: 1024,
        max_header_bytes: 4096,
        timeout_ms: 2000,
    }
}
fn request(url: &str, method: EgressMethod) -> EgressRequest {
    EgressRequest {
        url: url.into(),
        method,
        headers: vec![("x-canary".into(), b"egress-canary".to_vec())],
        body: Vec::new(),
    }
}
#[tokio::test]
async fn local_dns_denied_then_owner_allowed_and_peer_pinned() {
    let (address, captured, task) = endpoint(None, false).await;
    let url = format!("http://localhost:{}/", address.port());
    let denied = EgressHttpClient::new(policy(vec![route(
        "local",
        &url,
        EgressRouteKind::Destination,
        false,
    )]))
    .unwrap();
    assert!(matches!(
        denied.execute(request(&url, EgressMethod::Get)).await,
        Err(EgressError::Denied(_))
    ));
    assert!(captured.lock().unwrap().is_empty());
    let client = EgressHttpClient::new(policy(vec![route(
        "local",
        &url,
        EgressRouteKind::Destination,
        true,
    )]))
    .unwrap();
    let resolved = client.authorize(&url, EgressMethod::Get).await.unwrap();
    let response = client
        .execute(request(&url, EgressMethod::Get))
        .await
        .unwrap();
    assert_eq!(response.body, b"ok");
    assert!(
        resolved
            .addresses
            .contains(&response.receipt.connected_peer)
    );
    assert!(captured.lock().unwrap()[0].contains("egress-canary"));
    assert!(
        !serde_json::to_string(&response.receipt)
            .unwrap()
            .contains("egress-canary")
    );
    task.abort();
}
#[tokio::test]
async fn redirects_require_routes_strip_canary_and_never_replay_writes() {
    let (target, target_capture, target_task) = endpoint(None, false).await;
    let target_url = format!("http://{target}/");
    let (initial, initial_capture, initial_task) = endpoint(Some(target_url.clone()), false).await;
    let url = format!("http://{initial}/");
    let initial_route = route("initial", &url, EgressRouteKind::Destination, true);
    let denied = EgressHttpClient::new(policy(vec![initial_route.clone()])).unwrap();
    assert!(matches!(
        denied.execute(request(&url, EgressMethod::Get)).await,
        Err(EgressError::Denied(_))
    ));
    assert!(target_capture.lock().unwrap().is_empty());
    let client = EgressHttpClient::new(policy(vec![
        initial_route,
        route("target", &target_url, EgressRouteKind::Destination, true),
    ]))
    .unwrap();
    let response = client
        .execute(request(&url, EgressMethod::Get))
        .await
        .unwrap();
    assert_eq!(response.receipt.redirects, 1);
    assert!(!target_capture.lock().unwrap()[0].contains("egress-canary"));
    assert!(matches!(
        client.execute(request(&url, EgressMethod::Post)).await,
        Err(EgressError::OutcomeUncertain)
    ));
    assert_eq!(target_capture.lock().unwrap().len(), 1);
    assert_eq!(initial_capture.lock().unwrap().len(), 3);
    initial_task.abort();
    target_task.abort();
}
#[tokio::test]
async fn explicit_proxy_pins_actual_proxy_and_target_and_https_fails_closed() {
    let (target, target_capture, target_task) = endpoint(None, false).await;
    let (proxy, proxy_capture, proxy_task) = endpoint(None, true).await;
    let url = format!("http://localhost:{}/", target.port());
    let proxy_url = format!("http://{proxy}/");
    let destination = route("target", &url, EgressRouteKind::Destination, true);
    let mut configuration = policy(vec![
        destination.clone(),
        route("proxy", &proxy_url, EgressRouteKind::Proxy, false),
    ]);
    configuration.proxy = Some(EgressProxy {
        url: proxy_url.clone(),
        route_id: "proxy".into(),
    });
    let denied = EgressHttpClient::new(configuration.clone()).unwrap();
    assert!(matches!(
        denied.execute(request(&url, EgressMethod::Get)).await,
        Err(EgressError::Denied(_))
    ));
    assert!(proxy_capture.lock().unwrap().is_empty());
    configuration.routes[1] = route("proxy", &proxy_url, EgressRouteKind::Proxy, true);
    // Select the bound IPv4 endpoint while still resolving localhost for real.
    configuration.routes[0].host = "127.0.0.1".into();
    let literal_url = format!("http://{target}/");
    let https_url = format!("https://{target}/");
    configuration
        .routes
        .push(route("tls", &https_url, EgressRouteKind::Destination, true));
    let client = EgressHttpClient::new(configuration).unwrap();
    let response = client
        .execute(request(&literal_url, EgressMethod::Get))
        .await
        .unwrap();
    assert_eq!(response.body, b"ok");
    assert_eq!(response.receipt.connected_peer, proxy);
    assert!(proxy_capture.lock().unwrap()[0].starts_with(&format!("GET http://{target}/")));
    assert_eq!(target_capture.lock().unwrap().len(), 1);
    assert!(matches!(
        client.execute(request(&https_url, EgressMethod::Get)).await,
        Err(EgressError::Unavailable(_))
    ));
    assert_eq!(proxy_capture.lock().unwrap().len(), 1);
    assert!(!client.capabilities().pinned_https_proxy);
    target_task.abort();
    proxy_task.abort();
}
#[test]
fn address_classes_and_mixed_sets_fail_without_owner_exception() {
    let route = route(
        "public",
        "https://example.com/",
        EgressRouteKind::Destination,
        false,
    );
    for ip in [
        "127.0.0.1",
        "::1",
        "169.254.169.254",
        "10.0.0.1",
        "100.64.0.1",
        "::ffff:127.0.0.1",
        "2002::1",
        "2001:db8::1",
    ] {
        assert!(!address_allowed(&route, ip.parse().unwrap()), "{ip}");
    }
    let mixed = ["1.1.1.1".parse().unwrap(), "127.0.0.1".parse().unwrap()];
    assert!(!mixed.into_iter().all(|ip| address_allowed(&route, ip)));
    assert!(public_address("2606:4700:4700::1111".parse().unwrap()));
}
