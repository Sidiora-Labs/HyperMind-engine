use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::SigningKey;
use hm_context::Scope;
use hm_fabric::{compat_transport::*, transport::*};
use sha2::{Digest, Sha256};
use tokio::net::UnixListener;
fn envelope() -> PublicEnvelope {
    PublicEnvelope {
        protocol: PROTOCOL.into(),
        kind: MessageKind::Request,
        message_id: "call-1".into(),
        sequence: 1,
        reply_to: None,
        route_id: Some("route".into()),
        route_epoch: Some(1),
        operation: Some("inspect".into()),
        scope: None,
        trace: None,
        deadline_ms: Some(9000),
        payload: Some(serde_json::json!({"text":"x".repeat(150000)})),
        error: None,
    }
}
async fn pair() -> (UnixTransport, UnixTransport) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("socket");
    let listener = UnixListener::bind(&path).unwrap();
    let a = SigningKey::from_bytes(&[17; 32]);
    let b = SigningKey::from_bytes(&[29; 32]);
    let scope = Scope {
        owner_id: "owner".into(),
        project_id: "project".into(),
        workspace_id: None,
    };
    let ca = Credentials {
        scope: scope.clone(),
        signing_key: a.clone(),
        peer_key: b.verifying_key(),
    };
    let cb = Credentials {
        scope,
        signing_key: b,
        peer_key: a.verifying_key(),
    };
    let server = async {
        let (s, _) = listener.accept().await.unwrap();
        UnixTransport::accept(s, cb, ReplayGuard::default(), Limits::default())
            .await
            .unwrap()
    };
    let (a, b) = tokio::join!(
        UnixTransport::connect(&path, ca, ReplayGuard::default(), Limits::default()),
        server
    );
    (a.unwrap(), b)
}
#[tokio::test]
async fn real_authenticated_peers_negotiate_chunked_request_response() {
    let (a, b) = pair().await;
    let (a, b) = tokio::join!(
        PublicFramingAdapter::negotiate(a),
        PublicFramingAdapter::negotiate(b)
    );
    let (mut a, mut b) = (a.unwrap(), b.unwrap());
    assert_eq!(a.identity().session_id(), b.identity().session_id());
    let e = envelope();
    let (send, recv) = tokio::join!(a.send(&e), b.receive());
    send.unwrap();
    let got = recv.unwrap();
    assert_eq!(got.payload, e.payload);
    let mut response = envelope();
    response.kind = MessageKind::Response;
    response.reply_to = Some(e.message_id.clone());
    let (send, recv) = tokio::join!(b.send(&response), a.receive());
    send.unwrap();
    assert_eq!(recv.unwrap().reply_to, response.reply_to);
    assert!(a.send(&e).await.is_err());
    let mut foreign = e;
    foreign.sequence = 2;
    foreign.scope = Some(Scope {
        owner_id: "foreign".into(),
        project_id: "project".into(),
        workspace_id: None,
    });
    assert!(a.send(&foreign).await.is_err());
}
#[tokio::test]
async fn incompatible_negotiation_refuses() {
    let (a, mut b) = pair().await;
    let bad = async {
        b.send(Frame::request("compat-negotiation",br#"{"adapter":"hypermind.hypermid-adapter.v1","protocol":"hypermid.v0","frame_bytes":8388608,"chunk_bytes":65536}"#.to_vec())).await.unwrap();
        b.receive().await.unwrap();
    };
    let (result, _) = tokio::join!(PublicFramingAdapter::negotiate(a), bad);
    assert!(result.is_err());
}
#[test]
fn strict_public_vectors_and_limits() {
    let e = envelope();
    let bytes = encode(&e).unwrap();
    assert_eq!(decode(&bytes).unwrap().message_id, e.message_id);
    assert!(decode(&((MAX_FRAME_BYTES + 1) as u32).to_be_bytes()).is_err());
    let raw=br#"{"protocol":"hypermid.v1","kind":"ping","message_id":"a","sequence":1,"payload":{"x":1,"x":2}}"#;
    let mut b = (raw.len() as u32).to_be_bytes().to_vec();
    b.extend(raw);
    assert!(decode(&b).is_err());
    let mut exact = envelope();
    exact.payload = Some(serde_json::json!(""));
    let overhead = encode(&exact).unwrap().len() - 4;
    exact.payload = Some(serde_json::json!("x".repeat(MAX_FRAME_BYTES - overhead)));
    assert_eq!(encode(&exact).unwrap().len(), MAX_FRAME_BYTES + 4);
    let mut huge = envelope();
    huge.payload = Some(serde_json::json!("x".repeat(MAX_FRAME_BYTES)));
    assert!(encode(&huge).is_err());
    huge.payload = Some(serde_json::json!({"n":9007199254740992u64}));
    assert!(encode(&huge).is_err());
}
#[test]
fn digest_order_and_canonical_base64_before_delivery() {
    let data = vec![7; MAX_CHUNK_BYTES + 3];
    let digest = format!("{:x}", Sha256::digest(&data));
    let chunks: Vec<_> = data
        .chunks(MAX_CHUNK_BYTES)
        .enumerate()
        .map(|(i, b)| EventChunk {
            event_id: "event".into(),
            chunk_index: i as u32,
            chunk_count: 2,
            digest: digest.clone(),
            data: STANDARD.encode(b),
        })
        .collect();
    let mut a = EventAssembler::default();
    assert!(a.push(chunks[1].clone()).is_err());
    assert!(a.push(chunks[0].clone()).unwrap().is_none());
    assert!(a.push(chunks[0].clone()).is_err());
    assert!(a.push(chunks[0].clone()).unwrap().is_none());
    assert_eq!(a.push(chunks[1].clone()).unwrap().unwrap(), data);
    assert!(a.push(chunks[0].clone()).is_err());
    let mut corrupt = chunks[0].clone();
    corrupt.event_id = "corrupt".into();
    corrupt.digest = "0".repeat(64);
    let mut tail = chunks[1].clone();
    tail.event_id = corrupt.event_id.clone();
    tail.digest = corrupt.digest.clone();
    assert!(a.push(corrupt).unwrap().is_none());
    assert!(a.push(tail).is_err());
}
fn process_credentials(server: bool) -> Credentials {
    let a = SigningKey::from_bytes(&[61; 32]);
    let b = SigningKey::from_bytes(&[72; 32]);
    Credentials {
        scope: Scope {
            owner_id: "owner".into(),
            project_id: "project".into(),
            workspace_id: None,
        },
        signing_key: if server { b.clone() } else { a.clone() },
        peer_key: if server {
            a.verifying_key()
        } else {
            b.verifying_key()
        },
    }
}
#[tokio::test]
#[ignore = "invoked only by the authenticated process journey"]
async fn peer_process() {
    let path = std::env::var("HM_COMPAT_PEER_SOCKET").expect("process fixture socket");
    let native = UnixTransport::connect(
        path,
        process_credentials(false),
        ReplayGuard::default(),
        Limits::default(),
    )
    .await
    .unwrap();
    let mut peer = PublicFramingAdapter::negotiate(native).await.unwrap();
    let request = peer.receive().await.unwrap();
    assert_eq!(request.payload, envelope().payload);
    let mut response = envelope();
    response.kind = MessageKind::Response;
    response.reply_to = Some(request.message_id);
    peer.send(&response).await.unwrap();
}
#[tokio::test]
async fn two_actual_processes_exchange_authenticated_public_frames() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("peer.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "peer_process", "--ignored", "--nocapture"])
        .env("HM_COMPAT_PEER_SOCKET", &path)
        .spawn()
        .unwrap();
    let (stream, _) = tokio::time::timeout(std::time::Duration::from_secs(10), listener.accept())
        .await
        .unwrap()
        .unwrap();
    let native = UnixTransport::accept(
        stream,
        process_credentials(true),
        ReplayGuard::default(),
        Limits::default(),
    )
    .await
    .unwrap();
    let mut peer = PublicFramingAdapter::negotiate(native).await.unwrap();
    peer.send(&envelope()).await.unwrap();
    assert_eq!(
        peer.receive().await.unwrap().reply_to.as_deref(),
        Some("call-1")
    );
    assert!(child.wait().unwrap().success());
}
