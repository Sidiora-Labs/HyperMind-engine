use ed25519_dalek::SigningKey;
use hm_context::Scope;
use hm_fabric::transport::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
};
fn credentials() -> (Credentials, Credentials) {
    let a = SigningKey::from_bytes(&[17; 32]);
    let b = SigningKey::from_bytes(&[29; 32]);
    let scope = Scope {
        owner_id: "owner".into(),
        project_id: "project".into(),
        workspace_id: Some("workspace".into()),
    };
    (
        Credentials {
            scope: scope.clone(),
            signing_key: a.clone(),
            peer_key: b.verifying_key(),
        },
        Credentials {
            scope,
            signing_key: b,
            peer_key: a.verifying_key(),
        },
    )
}
async fn pair() -> (UnixTransport, UnixTransport) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("socket");
    let listener = UnixListener::bind(&path).unwrap();
    let (a, b) = credentials();
    let server = async {
        let (stream, _) = listener.accept().await.unwrap();
        UnixTransport::accept(stream, b, ReplayGuard::default(), Limits::default())
            .await
            .unwrap()
    };
    let client = UnixTransport::connect(&path, a, ReplayGuard::default(), Limits::default());
    let (client, server) = tokio::join!(client, server);
    (client.unwrap(), server)
}
#[tokio::test]
async fn authenticated_local_request_response_and_metadata() {
    let (mut client, mut server) = pair().await;
    assert_eq!(client.identity().scope(), server.identity().scope());
    assert_eq!(
        client.identity().session_id(),
        server.identity().session_id()
    );
    assert_ne!(client.identity().peer_key(), server.identity().peer_key());
    let request = Frame::request("call-1", b"inspect".to_vec());
    client.send(request.clone()).await.unwrap();
    assert_eq!(server.receive().await.unwrap(), request);
    let mut response = Frame::request("call-1", b"ok".to_vec());
    response.kind = FrameKind::Response;
    server.send(response.clone()).await.unwrap();
    assert_eq!(client.receive().await.unwrap(), response);
    let cancel = Frame {
        kind: FrameKind::Cancel,
        payload: vec![],
        ..Frame::request("call-1", vec![])
    };
    client.send(cancel.clone()).await.unwrap();
    assert_eq!(server.receive().await.unwrap(), cancel);
    let failure = Frame {
        kind: FrameKind::Failure,
        failure: Some(TypedFailure {
            code: FailureCode::Cancelled,
            effect: Effect::Applied,
            message: "cancelled after publication".into(),
        }),
        ..Frame::request("call-1", vec![])
    };
    server.send(failure.clone()).await.unwrap();
    assert_eq!(client.receive().await.unwrap(), failure);
}
#[tokio::test]
async fn bounds_deadlines_and_chunk_roundtrip() {
    let (mut client, mut server) = pair().await;
    let mut expired = Frame::request("expired", vec![]);
    expired.deadline_unix_ms = Some(1);
    let error = client.send(expired).await.unwrap_err();
    assert_eq!(error.code, FailureCode::Deadline);
    assert_eq!(error.effect, Effect::NotApplied);
    let mut chunk = Frame::request("chunk", vec![1; 64]);
    chunk.kind = FrameKind::Chunk;
    chunk.chunk = Some(ChunkMeta { index: 0, total: 2 });
    client.send(chunk.clone()).await.unwrap();
    assert_eq!(server.receive().await.unwrap(), chunk);
    chunk.chunk = Some(ChunkMeta { index: 2, total: 2 });
    assert_eq!(
        client.send(chunk).await.unwrap_err().code,
        FailureCode::Protocol
    );
    let mut oversized = Frame::request("chunk", vec![1; 65537]);
    oversized.kind = FrameKind::Chunk;
    oversized.chunk = Some(ChunkMeta { index: 0, total: 1 });
    assert!(client.send(oversized).await.is_err());
    assert_eq!(
        UnixTransport::remote().err().unwrap().code,
        FailureCode::Unsupported
    );
}
#[tokio::test]
async fn rejects_wrong_scope_and_pinned_identity() {
    for wrong_scope in [true, false] {
        let (a, mut b) = credentials();
        if wrong_scope {
            b.scope.project_id = "other".into();
        } else {
            b.signing_key = SigningKey::from_bytes(&[31; 32]);
        }
        let (left, right) = UnixStream::pair().unwrap();
        let (x, y) = tokio::join!(
            UnixTransport::accept(left, a, ReplayGuard::default(), Limits::default()),
            UnixTransport::accept(right, b, ReplayGuard::default(), Limits::default())
        );
        assert!(x.is_err());
        assert!(y.is_err());
    }
}
#[derive(serde::Serialize)]
struct Hello {
    version: u32,
    scope: Scope,
    key: [u8; 32],
    nonce: [u8; 32],
}
async fn packet(stream: &mut UnixStream) -> Vec<u8> {
    let n = stream.read_u32().await.unwrap();
    let mut bytes = vec![0; n as usize];
    stream.read_exact(&mut bytes).await.unwrap();
    bytes
}
async fn send_packet(stream: &mut UnixStream, bytes: &[u8]) {
    stream.write_u32(bytes.len() as u32).await.unwrap();
    stream.write_all(bytes).await.unwrap();
}
#[tokio::test]
async fn rejects_nonce_replay_and_oversized_prefix() {
    let (a, b) = credentials();
    let guard = ReplayGuard::default();
    use ed25519_dalek::Signer;
    let hello = serde_json::to_vec(&Hello {
        version: VERSION,
        scope: b.scope.clone(),
        key: b.signing_key.verifying_key().to_bytes(),
        nonce: [8; 32],
    })
    .unwrap();
    for attempt in 0..2 {
        let (left, mut right) = UnixStream::pair().unwrap();
        let accept = UnixTransport::accept(left, a.clone(), guard.clone(), Limits::default());
        let raw = async {
            let peer = packet(&mut right).await;
            send_packet(&mut right, &hello).await;
            packet(&mut right).await;
            let mut proof = b"HyperMind transport v1\0".to_vec();
            proof.extend_from_slice(&(hello.len() as u64).to_be_bytes());
            proof.extend_from_slice(&hello);
            proof.extend_from_slice(&(peer.len() as u64).to_be_bytes());
            proof.extend_from_slice(&peer);
            send_packet(&mut right, &b.signing_key.sign(&proof).to_bytes()).await;
        };
        let (result, _) = tokio::join!(accept, raw);
        if attempt == 0 {
            assert!(result.is_ok());
        } else {
            let error = result.err().unwrap();
            assert_eq!(error.code, FailureCode::Authentication);
            assert_eq!(error.message, "replayed nonce");
        }
    }
    let (left, mut right) = UnixStream::pair().unwrap();
    let raw = async {
        packet(&mut right).await;
        right.write_u32(u32::MAX).await.unwrap();
    };
    let (result, _) = tokio::join!(
        UnixTransport::accept(left, a, guard, Limits::default()),
        raw
    );
    assert_eq!(result.err().unwrap().code, FailureCode::Capacity);
}
#[tokio::test]
async fn rejects_signed_out_of_order_and_cross_session_frames() {
    use ed25519_dalek::Signer;
    for (sequence, wrong_session, bad_signature) in [
        (0u64, false, false),
        (2, false, false),
        (1, true, false),
        (1, false, true),
    ] {
        let (a, b) = credentials();
        let (left, mut right) = UnixStream::pair().unwrap();
        let accept = UnixTransport::accept(left, a, ReplayGuard::default(), Limits::default());
        let raw = async {
            let peer = packet(&mut right).await;
            let hello = serde_json::to_vec(&Hello {
                version: VERSION,
                scope: b.scope.clone(),
                key: b.signing_key.verifying_key().to_bytes(),
                nonce: [9u8; 32],
            })
            .unwrap();
            send_packet(&mut right, &hello).await;
            packet(&mut right).await;
            let mut proof = b"HyperMind transport v1\0".to_vec();
            proof.extend_from_slice(&(hello.len() as u64).to_be_bytes());
            proof.extend_from_slice(&hello);
            proof.extend_from_slice(&(peer.len() as u64).to_be_bytes());
            proof.extend_from_slice(&peer);
            send_packet(&mut right, &b.signing_key.sign(&proof).to_bytes()).await;
            right
        };
        let (server, right) = tokio::join!(accept, raw);
        if let Ok(mut server) = server {
            let mut right = right;
            let session = if wrong_session {
                [0; 32]
            } else {
                *server.identity().session_id()
            };
            let body=serde_json::to_vec(&serde_json::json!({"version":VERSION,"session":session,"scope":b.scope,"sequence":sequence,"frame":Frame::request("invalid",vec![])})).unwrap();
            let mut signature = b.signing_key.sign(&body).to_bytes();
            if bad_signature {
                signature[0] ^= 1;
            }
            let wire = serde_json::to_vec(
                &serde_json::json!({"signature":signature.to_vec(),"body":body}),
            )
            .unwrap();
            send_packet(&mut right, &wire).await;
            assert_eq!(
                server.receive().await.unwrap_err().code,
                if bad_signature {
                    FailureCode::Authentication
                } else {
                    FailureCode::Protocol
                }
            );
        } else {
            panic!("handshake must authenticate before sequence test");
        }
    }
}
