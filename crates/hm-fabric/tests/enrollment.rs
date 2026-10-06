use ed25519_dalek::SigningKey;
use hm_context::Scope;
use hm_fabric::{
    compat_transport::{MessageKind, PublicEnvelope, PROTOCOL},
    enrollment::*,
    transport::{Credentials, Limits, ReplayGuard, UnixTransport},
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
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::net::{TcpListener, TcpStream, UnixListener};
fn scope() -> Scope {
    Scope {
        owner_id: "owner".into(),
        project_id: "project".into(),
        workspace_id: None,
    }
}
fn grants() -> Grants {
    ["inspect".into(), "remember".into()].into_iter().collect()
}
fn current() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
async fn owner() -> hm_fabric::transport::AuthenticatedIdentity {
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
struct Certificates {
    ca: Vec<u8>,
    server: Vec<u8>,
    server_key: Vec<u8>,
    client: Vec<u8>,
    client_key: Vec<u8>,
}
fn certificates() -> Certificates {
    let ca_key = KeyPair::generate().unwrap();
    let mut ca = CertificateParams::new(vec![]).unwrap();
    ca.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    let issuer = CertifiedIssuer::self_signed(ca, ca_key).unwrap();
    let server_key = KeyPair::generate().unwrap();
    let mut server = CertificateParams::new(vec!["localhost".into()]).unwrap();
    server.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    let server = server.signed_by(&server_key, &issuer).unwrap();
    let client_key = KeyPair::generate().unwrap();
    let mut client = CertificateParams::new(vec!["device".into()]).unwrap();
    client.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];
    let client = client.signed_by(&client_key, &issuer).unwrap();
    Certificates {
        ca: issuer.der().to_vec(),
        server: server.der().to_vec(),
        server_key: server_key.serialize_der(),
        client: client.der().to_vec(),
        client_key: client_key.serialize_der(),
    }
}
fn roots(bytes: &[u8]) -> RootCertStore {
    let mut roots = RootCertStore::empty();
    roots.add(CertificateDer::from(bytes.to_vec())).unwrap();
    roots
}
fn key(bytes: Vec<u8>) -> PrivateKeyDer<'static> {
    PrivatePkcs8KeyDer::from(bytes).into()
}
#[derive(Serialize, Deserialize)]
struct Fixture {
    address: String,
    ca: PathBuf,
    cert: PathBuf,
    key: PathBuf,
    ticket: Option<String>,
    nonce: String,
    scope: Scope,
    refuse: bool,
    revoke: bool,
}
fn request(sequence: u64) -> PublicEnvelope {
    PublicEnvelope {
        protocol: PROTOCOL.into(),
        kind: MessageKind::Request,
        message_id: format!("request-{sequence}"),
        sequence,
        reply_to: None,
        route_id: Some("context".into()),
        route_epoch: Some(1),
        operation: Some("inspect".into()),
        scope: Some(scope()),
        trace: None,
        deadline_ms: Some(current() + 10000),
        payload: Some(serde_json::json!({"bytes":"x".repeat(150000)})),
        error: None,
    }
}
#[tokio::test]
#[ignore = "spawned by actual TLS enrollment journey"]
async fn tls_peer_process() {
    let fixture: Fixture =
        serde_json::from_str(&std::env::var("HM_ENROLLMENT_FIXTURE").unwrap()).unwrap();
    let config = client_config(
        roots(&std::fs::read(fixture.ca).unwrap()),
        vec![CertificateDer::from(std::fs::read(fixture.cert).unwrap())],
        key(std::fs::read(fixture.key).unwrap()),
    )
    .unwrap();
    let stream = TcpStream::connect(fixture.address).await.unwrap();
    let session = PublicTlsSession::connect_with_nonce(
        stream,
        config,
        ServerName::try_from("localhost").unwrap(),
        fixture.scope,
        fixture.ticket,
        grants(),
        fixture.nonce,
    )
    .await;
    if fixture.refuse {
        assert!(session.is_err());
        return;
    }
    let mut session = session.unwrap();
    assert_eq!(session.grants(), &BTreeSet::from(["inspect".into()]));
    session.send(&request(1)).await.unwrap();
    assert_eq!(
        session.receive().await.unwrap().reply_to.as_deref(),
        Some("request-1")
    );
    if fixture.revoke {
        if session.send(&request(2)).await.is_ok() {
            assert!(session.receive().await.is_err());
        }
    }
}
fn fixture(dir: &Path, certs: &Certificates, address: String) -> Fixture {
    let ca = dir.join("ca.der");
    let cert = dir.join("client.der");
    let key = dir.join("client-key.der");
    std::fs::write(&ca, &certs.ca).unwrap();
    std::fs::write(&cert, &certs.client).unwrap();
    std::fs::write(&key, &certs.client_key).unwrap();
    Fixture {
        address,
        ca,
        cert,
        key,
        ticket: None,
        nonce: "11".repeat(32),
        scope: scope(),
        refuse: false,
        revoke: false,
    }
}
fn review(certs: &Certificates, expires_ms: u64, device_id: &str) -> DeviceReview {
    DeviceReview {
        device_id: device_id.into(),
        name: "Personal device".into(),
        certificate_sha256: format!("{:x}", Sha256::digest(&certs.client)),
        scope: scope(),
        grants: grants(),
        expires_ms,
    }
}
async fn run_peer(
    listener: &TcpListener,
    config: MutualServerConfig,
    store: Arc<Mutex<EnrollmentStore>>,
    fixture: &Fixture,
    owner: &hm_fabric::transport::AuthenticatedIdentity,
) {
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "tls_peer_process", "--ignored", "--nocapture"])
        .env(
            "HM_ENROLLMENT_FIXTURE",
            serde_json::to_string(fixture).unwrap(),
        )
        .spawn()
        .unwrap();
    let (stream, _) = tokio::time::timeout(std::time::Duration::from_secs(10), listener.accept())
        .await
        .unwrap()
        .unwrap();
    let session = PublicTlsSession::accept(
        stream,
        config,
        store.clone(),
        BTreeSet::from(["inspect".into()]),
        "daemon",
    )
    .await;
    if fixture.refuse {
        assert!(session.is_err());
    } else {
        let mut session = session.unwrap();
        assert_eq!(
            session.identity().certificate_sha256(),
            session.device().unwrap().device().certificate_sha256
        );
        let incoming = session.receive().await.unwrap();
        assert_eq!(incoming.payload, request(1).payload);
        let mut response = incoming;
        response.kind = MessageKind::Response;
        response.reply_to = Some(response.message_id.clone());
        response.message_id = "response-1".into();
        session.send(&response).await.unwrap();
        if fixture.revoke {
            store.lock().unwrap().revoke(owner, "device-1").unwrap();
            assert!(session.receive().await.is_err());
        }
        drop(session);
    }
    assert!(child.wait().unwrap().success());
}
#[tokio::test]
async fn real_mutual_tls_enrollment_restart_nonce_and_revocation() {
    let dir = tempfile::tempdir().unwrap();
    let owner = owner().await;
    let certs = certificates();
    let config = server_config(
        roots(&certs.ca),
        vec![CertificateDer::from(certs.server.clone())],
        key(certs.server_key.clone()),
    )
    .unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let path = dir.path().join("enrollment.db");
    let mut store = EnrollmentStore::open(&path, &owner).unwrap();
    let ticket = store
        .review(&owner, review(&certs, current() + 60000, "device-1"))
        .unwrap();
    let store = Arc::new(Mutex::new(store));
    let mut fixture = fixture(
        dir.path(),
        &certs,
        listener.local_addr().unwrap().to_string(),
    );
    fixture.ticket = Some(ticket.nonce.clone());
    run_peer(&listener, config.clone(), store.clone(), &fixture, &owner).await;
    drop(store);
    let store = Arc::new(Mutex::new(EnrollmentStore::open(&path, &owner).unwrap()));
    fixture.ticket = None;
    fixture.nonce = "22".repeat(32);
    run_peer(&listener, config.clone(), store.clone(), &fixture, &owner).await;
    fixture.refuse = true;
    fixture.nonce = "11".repeat(32);
    run_peer(&listener, config.clone(), store.clone(), &fixture, &owner).await;
    fixture.nonce = "33".repeat(32);
    fixture.ticket = Some(ticket.nonce);
    run_peer(&listener, config.clone(), store.clone(), &fixture, &owner).await;
    fixture.ticket = None;
    fixture.scope.project_id = "foreign".into();
    fixture.nonce = "44".repeat(32);
    run_peer(&listener, config.clone(), store.clone(), &fixture, &owner).await;
    fixture.scope = scope();
    fixture.refuse = false;
    fixture.revoke = true;
    fixture.nonce = "55".repeat(32);
    run_peer(&listener, config.clone(), store.clone(), &fixture, &owner).await;
    fixture.revoke = false;
    fixture.refuse = true;
    fixture.nonce = "66".repeat(32);
    run_peer(&listener, config.clone(), store.clone(), &fixture, &owner).await;
    drop(store);
    let store = Arc::new(Mutex::new(EnrollmentStore::open(&path, &owner).unwrap()));
    fixture.nonce = "77".repeat(32);
    run_peer(&listener, config.clone(), store.clone(), &fixture, &owner).await;
    let renewed = store
        .lock()
        .unwrap()
        .review(&owner, review(&certs, current() + 250, "device-1"))
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    fixture.ticket = Some(renewed.nonce);
    fixture.nonce = "88".repeat(32);
    run_peer(&listener, config.clone(), store.clone(), &fixture, &owner).await;
    let rogue = certificates();
    std::fs::write(&fixture.cert, &rogue.client).unwrap();
    std::fs::write(&fixture.key, &rogue.client_key).unwrap();
    fixture.ticket = None;
    fixture.nonce = "99".repeat(32);
    run_peer(&listener, config, store, &fixture, &owner).await;
}
