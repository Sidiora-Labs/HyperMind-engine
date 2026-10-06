use ed25519_dalek::SigningKey;
use hm_context::Scope;
use hm_fabric::{
    compat_transport::{MessageKind, PublicEnvelope, PROTOCOL},
    enrollment::*,
    federation::*,
    transport::{AuthenticatedIdentity, Credentials, Limits, ReplayGuard, UnixTransport},
};
use rcgen::{
    BasicConstraints, CertificateParams, CertifiedIssuer, ExtendedKeyUsagePurpose, IsCa, KeyPair,
    KeyUsagePurpose,
};
use rustls::{
    pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName},
    RootCertStore,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    net::SocketAddr,
    path::{Path, PathBuf},
    process::{Child, Command},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::net::{TcpListener, TcpStream, UnixListener};
fn scope() -> Scope {
    Scope {
        owner_id: "owner".into(),
        project_id: "project".into(),
        workspace_id: None,
    }
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
fn all_grants() -> Grants {
    ["catalog", "inspect", "remember", "relay"]
        .into_iter()
        .map(str::to_string)
        .collect()
}
fn effective() -> Grants {
    ["catalog", "inspect"]
        .into_iter()
        .map(str::to_string)
        .collect()
}
fn catalog() -> RemoteCatalog {
    RemoteCatalog {
        generation: 1,
        operations: BTreeMap::from([("inspect".into(), 1)]),
    }
}
async fn owner() -> AuthenticatedIdentity {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("owner.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let a = SigningKey::from_bytes(&[31; 32]);
    let b = SigningKey::from_bytes(&[32; 32]);
    let ca = Credentials {
        scope: scope(),
        signing_key: a.clone(),
        peer_key: b.verifying_key(),
    };
    let cb = Credentials {
        scope: scope(),
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
        UnixTransport::connect(path, ca, ReplayGuard::default(), Limits::default()),
        server
    );
    a.unwrap();
    b.identity().clone()
}
#[derive(Clone, Serialize, Deserialize)]
struct Fixture {
    role: String,
    address: SocketAddr,
    ca: PathBuf,
    cert: PathBuf,
    key: PathBuf,
    enrollment: PathBuf,
    federation: PathBuf,
    candidate: PathBuf,
    grant: Option<SignedRelayGrant>,
    ready: PathBuf,
    stage: PathBuf,
    source: PathBuf,
    expected: Vec<u8>,
    refuse: bool,
}
fn roots(path: &Path) -> RootCertStore {
    let mut root = RootCertStore::empty();
    root.add(CertificateDer::from(std::fs::read(path).unwrap()))
        .unwrap();
    root
}
fn key(path: &Path) -> PrivateKeyDer<'static> {
    PrivatePkcs8KeyDer::from(std::fs::read(path).unwrap()).into()
}
fn client(f: &Fixture) -> MutualClientConfig {
    client_config(
        roots(&f.ca),
        vec![CertificateDer::from(std::fs::read(&f.cert).unwrap())],
        key(&f.key),
    )
    .unwrap()
}
fn server(f: &Fixture) -> MutualServerConfig {
    server_config(
        roots(&f.ca),
        vec![CertificateDer::from(std::fs::read(&f.cert).unwrap())],
        key(&f.key),
    )
    .unwrap()
}
fn spawn(f: &Fixture) -> Child {
    Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "federation_peer_process",
            "--ignored",
            "--nocapture",
        ])
        .env("HM_FEDERATION_FIXTURE", serde_json::to_string(f).unwrap())
        .spawn()
        .unwrap()
}
async fn ready(path: &Path) {
    for _ in 0..200 {
        if path.exists() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("peer readiness deadline");
}
async fn inspect(session: &mut PublicTlsSession, f: &Fixture) {
    let request = session.receive().await.unwrap();
    assert_eq!(request.kind, MessageKind::Request);
    assert_eq!(request.operation.as_deref(), Some("inspect"));
    let bytes = std::fs::read(&f.source).unwrap();
    let response = PublicEnvelope {
        protocol: PROTOCOL.into(),
        kind: MessageKind::Response,
        message_id: format!("inspect-{}", request.sequence),
        sequence: request.sequence,
        reply_to: Some(request.message_id),
        route_id: request.route_id,
        route_epoch: request.route_epoch,
        operation: Some("inspect".into()),
        scope: Some(scope()),
        trace: None,
        deadline_ms: None,
        payload: Some(serde_json::json!({"original_bytes":bytes,"pid":std::process::id()})),
        error: None,
    };
    session.send(&response).await.unwrap();
}
#[tokio::test]
#[ignore = "invoked by actual direct and relay process journey"]
async fn federation_peer_process() {
    let f: Fixture =
        serde_json::from_str(&std::env::var("HM_FEDERATION_FIXTURE").unwrap()).unwrap();
    let owner = owner().await;
    match f.role.as_str() {
        "direct-server" => {
            let enrollment = Arc::new(Mutex::new(
                EnrollmentStore::open(&f.enrollment, &owner).unwrap(),
            ));
            let listener = TcpListener::bind(f.address).await.unwrap();
            std::fs::write(&f.ready, b"ready").unwrap();
            let (stream, _) = listener.accept().await.unwrap();
            drop(
                PublicTlsSession::accept(
                    stream,
                    server(&f),
                    enrollment.clone(),
                    all_grants(),
                    "peer-b",
                )
                .await
                .unwrap(),
            );
            let (stream, _) = listener.accept().await.unwrap();
            let mut session =
                PublicTlsSession::accept(stream, server(&f), enrollment, all_grants(), "peer-b")
                    .await
                    .unwrap();
            serve_catalog(&mut session, &catalog()).await.unwrap();
            inspect(&mut session, &f).await;
        }
        "initiator" => {
            let candidate =
                SignedCandidate::from_slice(&std::fs::read(&f.candidate).unwrap()).unwrap();
            let verified = {
                let mut store = FederationStore::open(&f.federation, &owner).unwrap();
                store.accept_candidate(&owner, candidate).unwrap()
            };
            let connection =
                connect_direct_then_relay(&verified, client(&f), all_grants(), f.grant.clone())
                    .await;
            if f.refuse {
                assert!(connection.is_err());
                return;
            }
            let mut connection = connection.unwrap();
            assert_eq!(
                connection.path(),
                if f.grant.is_some() {
                    ConnectionPath::Relay
                } else {
                    ConnectionPath::Direct
                }
            );
            assert_eq!(connection.grants(), &effective());
            assert_eq!(connection.catalog(), Some(&catalog()));
            assert!(connection
                .call("remember", serde_json::Value::Null)
                .await
                .is_err());
            let response = connection
                .call("inspect", serde_json::Value::Null)
                .await
                .unwrap();
            let bytes: Vec<u8> = serde_json::from_value(
                response.payload.as_ref().unwrap()["original_bytes"].clone(),
            )
            .unwrap();
            assert_eq!(bytes, f.expected);
            assert_ne!(
                response.payload.unwrap()["pid"].as_u64().unwrap(),
                std::process::id() as u64
            );
            if f.grant.is_some() {
                assert!(connection
                    .call("inspect", serde_json::Value::Null)
                    .await
                    .is_err());
            }
        }
        "responder-relay" => {
            let stream = TcpStream::connect(f.address).await.unwrap();
            let mut session = PublicTlsSession::connect(
                stream,
                client(&f),
                ServerName::try_from("localhost").unwrap(),
                scope(),
                None,
                all_grants(),
            )
            .await
            .unwrap();
            let grant = f.grant.clone().unwrap();
            let admission = PublicEnvelope {
                protocol: PROTOCOL.into(),
                kind: MessageKind::Request,
                message_id: "responder-admit".into(),
                sequence: 1,
                reply_to: None,
                route_id: Some(grant.grant.pair_id.clone()),
                route_epoch: Some(1),
                operation: Some("relay".into()),
                scope: Some(scope()),
                trace: None,
                deadline_ms: Some(now() + 10000),
                payload: Some(serde_json::to_value(grant).unwrap()),
                error: None,
            };
            session.send(&admission).await.unwrap();
            assert_eq!(
                session.receive().await.unwrap().reply_to.as_deref(),
                Some("responder-admit")
            );
            serve_catalog(&mut session, &catalog()).await.unwrap();
            inspect(&mut session, &f).await;
            assert!(session.receive().await.is_err());
        }
        "relay" | "relay-restart" => {
            let enrollment = Arc::new(Mutex::new(
                EnrollmentStore::open(&f.enrollment, &owner).unwrap(),
            ));
            let store = Arc::new(Mutex::new(
                FederationStore::open(&f.federation, &owner).unwrap(),
            ));
            let broker = RelayBroker::new(store.clone());
            let listener = TcpListener::bind(f.address).await.unwrap();
            std::fs::write(&f.ready, b"ready").unwrap();
            if f.role == "relay" {
                for _ in 0..2 {
                    let (stream, _) = listener.accept().await.unwrap();
                    drop(
                        PublicTlsSession::accept(
                            stream,
                            server(&f),
                            enrollment.clone(),
                            all_grants(),
                            "relay",
                        )
                        .await
                        .unwrap(),
                    );
                }
                let (stream, _) = listener.accept().await.unwrap();
                let first = PublicTlsSession::accept(
                    stream,
                    server(&f),
                    enrollment.clone(),
                    all_grants(),
                    "relay",
                )
                .await
                .unwrap();
                let first = broker.admit(first).await.unwrap();
                let (stream, _) = listener.accept().await.unwrap();
                let second = PublicTlsSession::accept(
                    stream,
                    server(&f),
                    enrollment.clone(),
                    all_grants(),
                    "relay",
                )
                .await
                .unwrap();
                let second = broker.admit(second).await.unwrap();
                let mut pair = broker.pair(first, second).await.unwrap();
                assert_eq!(pair.grants(), &effective());
                pair.forward_once().await.unwrap();
                pair.forward_once().await.unwrap();
                std::fs::write(&f.stage, b"forwarded").unwrap();
                let (stream, _) = listener.accept().await.unwrap();
                let replay =
                    PublicTlsSession::accept(stream, server(&f), enrollment, all_grants(), "relay")
                        .await
                        .unwrap();
                assert!(broker.admit(replay).await.is_err());
                store.lock().unwrap().revoke_pair(&owner, "pair-1").unwrap();
                assert!(pair.forward_once().await.is_err());
                drop(pair);
            } else {
                let (stream, _) = listener.accept().await.unwrap();
                let session =
                    PublicTlsSession::accept(stream, server(&f), enrollment, all_grants(), "relay")
                        .await
                        .unwrap();
                assert!(broker.admit(session).await.is_err());
            }
        }
        other => panic!("unknown role {other}"),
    }
}
struct Cert {
    cert: PathBuf,
    key: PathBuf,
    fingerprint: String,
}
fn issue(dir: &Path, name: &str, issuer: &CertifiedIssuer<'_, KeyPair>) -> Cert {
    let key = KeyPair::generate().unwrap();
    let mut params = CertificateParams::new(vec!["localhost".into()]).unwrap();
    params.extended_key_usages = vec![
        ExtendedKeyUsagePurpose::ClientAuth,
        ExtendedKeyUsagePurpose::ServerAuth,
    ];
    let certificate = params.signed_by(&key, issuer).unwrap();
    let cert = dir.join(format!("{name}.der"));
    let keypath = dir.join(format!("{name}-key.der"));
    std::fs::write(&cert, certificate.der()).unwrap();
    std::fs::write(&keypath, key.serialize_der()).unwrap();
    Cert {
        cert,
        key: keypath,
        fingerprint: format!("{:x}", Sha256::digest(certificate.der())),
    }
}
fn address() -> SocketAddr {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap()
}
fn base(dir: &Path, role: &str, ca: &Path, cert: &Cert) -> Fixture {
    Fixture {
        role: role.into(),
        address: address(),
        ca: ca.into(),
        cert: cert.cert.clone(),
        key: cert.key.clone(),
        enrollment: dir.join(format!("{role}-enrollment.db")),
        federation: dir.join(format!("{role}-federation.db")),
        candidate: dir.join("candidate.json"),
        grant: None,
        ready: dir.join(format!("{role}-ready")),
        stage: dir.join("relay-stage"),
        source: dir.join("source.bin"),
        expected: vec![255, 0, 13, 10, 206, 187],
        refuse: false,
    }
}
fn review(cert: &Cert, id: &str) -> DeviceReview {
    DeviceReview {
        device_id: id.into(),
        name: id.into(),
        certificate_sha256: cert.fingerprint.clone(),
        scope: scope(),
        grants: all_grants(),
        expires_ms: now() + 120000,
    }
}
async fn enroll(f: &Fixture, cert: &Cert, ticket: String) {
    let mut c = f.clone();
    c.cert = cert.cert.clone();
    c.key = cert.key.clone();
    let stream = TcpStream::connect(f.address).await.unwrap();
    let session = PublicTlsSession::connect(
        stream,
        client(&c),
        ServerName::try_from("localhost").unwrap(),
        scope(),
        Some(ticket),
        all_grants(),
    )
    .await
    .unwrap();
    drop(session);
}
fn endpoint(address: SocketAddr, cert: &Cert) -> Endpoint {
    Endpoint {
        address,
        server_name: "localhost".into(),
        certificate_sha256: cert.fingerprint.clone(),
    }
}
fn advertisement(
    peer: &Cert,
    direct: SocketAddr,
    relay: Option<(&Cert, SocketAddr)>,
    generation: u64,
) -> SignedCandidate {
    SignedCandidate::sign(
        CandidateAdvertisement {
            version: 1,
            peer_id: "peer-b".into(),
            generation,
            scope: scope(),
            expires_ms: now() + 120000,
            direct: vec![endpoint(direct, peer)],
            relay: relay.map(|(cert, address)| endpoint(address, cert)),
            relay_peer_certificate_sha256: relay.map(|_| peer.fingerprint.clone()),
            grants: effective(),
            catalog: catalog(),
        },
        &SigningKey::from_bytes(&[31; 32]),
    )
    .unwrap()
}
fn relay_grant(local: &Cert, remote: &Cert, side: RelaySide, nonce: &str) -> SignedRelayGrant {
    SignedRelayGrant::sign(
        RelayGrant {
            version: 1,
            pair_id: "pair-1".into(),
            nonce: nonce.repeat(32),
            side,
            certificate_sha256: local.fingerprint.clone(),
            remote_certificate_sha256: remote.fingerprint.clone(),
            scope: scope(),
            expires_ms: now() + 120000,
            grants: effective(),
            catalog: catalog(),
        },
        &SigningKey::from_bytes(&[31; 32]),
    )
    .unwrap()
}
#[tokio::test]
async fn actual_direct_first_and_authenticated_single_use_relay() {
    let dir = tempfile::tempdir().unwrap();
    let owner = owner().await;
    let mut ca_params = CertificateParams::new(Vec::<String>::new()).unwrap();
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    let issuer = CertifiedIssuer::self_signed(ca_params, KeyPair::generate().unwrap()).unwrap();
    let ca = dir.path().join("ca.der");
    std::fs::write(&ca, issuer.der()).unwrap();
    let a = issue(dir.path(), "peer-a", &issuer);
    let b = issue(dir.path(), "peer-b", &issuer);
    let r = issue(dir.path(), "relay", &issuer);
    let direct = base(dir.path(), "direct-server", &ca, &b);
    std::fs::write(&direct.source, &direct.expected).unwrap();
    let ticket = {
        let mut store = EnrollmentStore::open(&direct.enrollment, &owner).unwrap();
        store.review(&owner, review(&a, "peer-a")).unwrap()
    };
    let mut direct_child = spawn(&direct);
    ready(&direct.ready).await;
    enroll(&direct, &a, ticket.nonce).await;
    let signed = advertisement(&b, direct.address, Some((&r, address())), 1);
    std::fs::write(&direct.candidate, serde_json::to_vec(&signed).unwrap()).unwrap();
    let mut initiator = base(dir.path(), "initiator", &ca, &a);
    initiator.candidate = direct.candidate.clone();
    let mut child_a = spawn(&initiator);
    assert!(child_a.wait().unwrap().success());
    assert!(direct_child.wait().unwrap().success());
    let relay = base(dir.path(), "relay", &ca, &r);
    let (ticket_a, ticket_b) = {
        let mut store = EnrollmentStore::open(&relay.enrollment, &owner).unwrap();
        (
            store.review(&owner, review(&a, "peer-a")).unwrap(),
            store.review(&owner, review(&b, "peer-b")).unwrap(),
        )
    };
    let mut relay_child = spawn(&relay);
    ready(&relay.ready).await;
    enroll(&relay, &a, ticket_a.nonce).await;
    enroll(&relay, &b, ticket_b.nonce).await;
    let unreachable = address();
    let signed = advertisement(&b, unreachable, Some((&r, relay.address)), 2);
    std::fs::write(&initiator.candidate, serde_json::to_vec(&signed).unwrap()).unwrap();
    let grant_a = relay_grant(&a, &b, RelaySide::Initiator, "aa");
    let grant_b = relay_grant(&b, &a, RelaySide::Responder, "bb");
    initiator.grant = Some(grant_a.clone());
    let mut responder = base(dir.path(), "responder-relay", &ca, &b);
    responder.address = relay.address;
    responder.grant = Some(grant_b);
    let mut child_b = spawn(&responder);
    let mut child_a = spawn(&initiator);
    ready(&relay.stage).await;
    let mut replay = initiator.clone();
    replay.refuse = true;
    let mut replay_child = spawn(&replay);
    assert!(replay_child.wait().unwrap().success());
    assert!(child_a.wait().unwrap().success());
    assert!(child_b.wait().unwrap().success());
    assert!(relay_child.wait().unwrap().success());
    let mut restart = relay.clone();
    restart.role = "relay-restart".into();
    restart.address = address();
    restart.ready = dir.path().join("relay-restart-ready");
    let mut restarted_relay = spawn(&restart);
    ready(&restart.ready).await;
    let signed = advertisement(&b, unreachable, Some((&r, restart.address)), 3);
    std::fs::write(&initiator.candidate, serde_json::to_vec(&signed).unwrap()).unwrap();
    replay.grant = Some(relay_grant(&a, &b, RelaySide::Initiator, "cc"));
    let mut revoked_peer = spawn(&replay);
    assert!(revoked_peer.wait().unwrap().success());
    assert!(restarted_relay.wait().unwrap().success());
    let mut expired_grant = relay_grant(&a, &b, RelaySide::Initiator, "dd").grant;
    expired_grant.expires_ms = now() + 200;
    let expired_grant =
        SignedRelayGrant::sign(expired_grant, &SigningKey::from_bytes(&[31; 32])).unwrap();
    tokio::time::sleep(Duration::from_millis(250)).await;
    replay.grant = Some(expired_grant);
    let mut expired_peer = spawn(&replay);
    assert!(expired_peer.wait().unwrap().success());
    replay.grant = Some(relay_grant(&a, &b, RelaySide::Responder, "ee"));
    let mut wrong_side_peer = spawn(&replay);
    assert!(wrong_side_peer.wait().unwrap().success());
    let mut store = FederationStore::open(dir.path().join("candidate-check.db"), &owner).unwrap();
    let mut tampered = signed.clone();
    tampered.advertisement.grants.insert("remember".into());
    assert!(store.accept_candidate(&owner, tampered).is_err());
    let valid = store.accept_candidate(&owner, signed.clone()).unwrap();
    assert_eq!(
        pair_candidates(&valid, &valid).unwrap().effective_grants,
        effective()
    );
    let stale = advertisement(&b, unreachable, Some((&r, restart.address)), 2);
    assert!(store.accept_candidate(&owner, stale).is_err());
    let mut expired = signed;
    expired.advertisement.expires_ms = now() - 1;
    assert!(store.accept_candidate(&owner, expired).is_err());
}
