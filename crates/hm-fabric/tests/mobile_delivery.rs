use ed25519_dalek::SigningKey;
use hm_context::{
    mobile_journal::{
        JournalBinding, MobileJournal, MutationIdentity, MutationState, ReceiptOutcome,
    },
    types::digest_bytes,
    Scope,
};
use hm_fabric::{
    compat_transport::{MessageKind, PublicEnvelope, PROTOCOL},
    effects::{EffectObservation, EffectOutcome, EffectState, EffectStore},
    enrollment::*,
    mobile_delivery::*,
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
    collections::BTreeSet,
    io::Write,
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
fn grants() -> Grants {
    BTreeSet::from(["mobile.dispatch".into(), "mobile.inspect".into()])
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
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
    bindings: PathBuf,
    effects: PathBuf,
    output: PathBuf,
    ready: PathBuf,
    stage: PathBuf,
    payload1: Vec<u8>,
    payload2: Vec<u8>,
}
fn roots(path: &Path) -> RootCertStore {
    let mut roots = RootCertStore::empty();
    roots
        .add(CertificateDer::from(std::fs::read(path).unwrap()))
        .unwrap();
    roots
}
fn key(path: &Path) -> PrivateKeyDer<'static> {
    PrivatePkcs8KeyDer::from(std::fs::read(path).unwrap()).into()
}
fn server(f: &Fixture) -> MutualServerConfig {
    server_config(
        roots(&f.ca),
        vec![CertificateDer::from(std::fs::read(&f.cert).unwrap())],
        key(&f.key),
    )
    .unwrap()
}
struct Cert {
    cert: PathBuf,
    key: PathBuf,
    fingerprint: String,
}
fn client(f: &Fixture, c: &Cert) -> MutualClientConfig {
    client_config(
        roots(&f.ca),
        vec![CertificateDer::from(std::fs::read(&c.cert).unwrap())],
        key(&c.key),
    )
    .unwrap()
}
fn issue(dir: &Path, name: &str, issuer: &CertifiedIssuer<'_, KeyPair>) -> Cert {
    let key = KeyPair::generate().unwrap();
    let mut params = CertificateParams::new(vec!["localhost".into()]).unwrap();
    params.extended_key_usages = vec![
        ExtendedKeyUsagePurpose::ClientAuth,
        ExtendedKeyUsagePurpose::ServerAuth,
    ];
    let cert = params.signed_by(&key, issuer).unwrap();
    let certificate = dir.join(format!("{name}.der"));
    let private_key = dir.join(format!("{name}-key.der"));
    std::fs::write(&certificate, cert.der()).unwrap();
    std::fs::write(&private_key, key.serialize_der()).unwrap();
    Cert {
        cert: certificate,
        key: private_key,
        fingerprint: format!("{:x}", Sha256::digest(cert.der())),
    }
}
fn address() -> SocketAddr {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
}
fn spawn(f: &Fixture) -> Child {
    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "native_effect_peer", "--ignored", "--nocapture"])
        .env(
            "HM_MOBILE_DELIVERY_FIXTURE",
            serde_json::to_string(f).unwrap(),
        )
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
fn append(path: &Path, payload: &[u8]) -> EffectObservation {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap();
    file.write_all(payload).unwrap();
    file.sync_all().unwrap();
    EffectObservation {
        outcome: EffectOutcome::Succeeded,
        evidence: format!(
            "observed-file-sha256:{}",
            digest_bytes(&std::fs::read(path).unwrap())
        ),
    }
}
#[tokio::test]
#[ignore = "invoked by real encrypted native-effect journal journey"]
async fn native_effect_peer() {
    let f: Fixture =
        serde_json::from_str(&std::env::var("HM_MOBILE_DELIVERY_FIXTURE").unwrap()).unwrap();
    let owner = owner().await;
    let enrollment = Arc::new(Mutex::new(
        EnrollmentStore::open(&f.enrollment, &owner).unwrap(),
    ));
    let mut effects = EffectStore::open(&f.effects).unwrap();
    if f.role == "resume" {
        let intent = effects.get(&scope(), "operation-1").unwrap().unwrap();
        assert_eq!(intent.state, EffectState::Uncertain);
        assert_eq!(intent.version, 3);
        let foreign = Scope {
            project_id: "foreign".into(),
            ..scope()
        };
        effects
            .prepare(&foreign, "unrelated-source", "append_file", b"foreign")
            .unwrap();
    }
    let mut authority = MobileEffectAuthority::open(&f.bindings, &owner, effects).unwrap();
    let listener = TcpListener::bind(f.address).await.unwrap();
    std::fs::write(&f.ready, b"ready").unwrap();
    let (stream, _) = listener.accept().await.unwrap();
    let mut session = PublicTlsSession::accept(
        stream,
        server(&f),
        enrollment.clone(),
        grants(),
        "native-effects",
    )
    .await
    .unwrap();
    if f.role == "crash" {
        let envelope = session.receive().await.unwrap();
        let request =
            MobileEffectAuthority::parse_request(&session, &envelope, "mobile.dispatch").unwrap();
        let dispatch = authority.begin_dispatch(&session, &request).unwrap();
        append(&f.output, &dispatch.intent().payload);
        std::fs::write(&f.stage, b"effect-applied").unwrap();
        tokio::time::sleep(Duration::from_secs(30)).await;
        panic!("parent must kill server before completion");
    }
    let envelope = session.receive().await.unwrap();
    let request =
        MobileEffectAuthority::parse_request(&session, &envelope, "mobile.inspect").unwrap();
    let evidence = authority.evidence(&session, &request).unwrap();
    let receipt: hm_fabric::effects::EffectReceipt =
        serde_json::from_slice(&evidence.native_receipt_bytes).unwrap();
    assert_eq!(receipt.intent.state, EffectState::Uncertain);
    assert_eq!(receipt.cursor.sequence, 3);
    authority
        .send_evidence(&mut session, &envelope, &request)
        .await
        .unwrap();
    assert_eq!(std::fs::read(&f.output).unwrap(), f.payload1);
    authority
        .reconcile_observed(
            &owner,
            "operation-1",
            EffectObservation {
                outcome: EffectOutcome::Succeeded,
                evidence: format!(
                    "observed-file-sha256:{}",
                    digest_bytes(&std::fs::read(&f.output).unwrap())
                ),
            },
        )
        .unwrap();
    let envelope = session.receive().await.unwrap();
    let request =
        MobileEffectAuthority::parse_request(&session, &envelope, "mobile.inspect").unwrap();
    authority
        .send_evidence(&mut session, &envelope, &request)
        .await
        .unwrap();
    let envelope = session.receive().await.unwrap();
    let request =
        MobileEffectAuthority::parse_request(&session, &envelope, "mobile.dispatch").unwrap();
    let dispatch = authority.begin_dispatch(&session, &request).unwrap();
    let observation = append(&f.output, &dispatch.intent().payload);
    authority
        .complete_observed(&session, dispatch, observation)
        .unwrap();
    authority
        .send_evidence(&mut session, &envelope, &request)
        .await
        .unwrap();
    drop(session);
    for _ in 0..5 {
        let (stream, _) = listener.accept().await.unwrap();
        let mut session = PublicTlsSession::accept(
            stream,
            server(&f),
            enrollment.clone(),
            grants(),
            "native-effects",
        )
        .await
        .unwrap();
        let envelope = session.receive().await.unwrap();
        let operation = envelope.operation.as_deref().unwrap();
        let request = MobileEffectAuthority::parse_request(&session, &envelope, operation).unwrap();
        if operation == "mobile.dispatch" {
            assert!(authority.begin_dispatch(&session, &request).is_err());
        } else {
            assert!(authority.evidence(&session, &request).is_err());
        }
    }
}
async fn connect(f: &Fixture, c: &Cert, ticket: Option<String>) -> PublicTlsSession {
    let stream = TcpStream::connect(f.address).await.unwrap();
    PublicTlsSession::connect(
        stream,
        client(f, c),
        ServerName::try_from("localhost").unwrap(),
        scope(),
        ticket,
        grants(),
    )
    .await
    .unwrap()
}
async fn refused(
    f: &Fixture,
    c: &Cert,
    ticket: Option<String>,
    mut identity: MutationIdentity,
    original: &str,
    mode: &str,
) {
    let mut session = connect(f, c, ticket).await;
    let (operation, payload, epoch) = match mode {
        "replay" => ("mobile.dispatch", Some(f.payload1.clone()), 1),
        "foreign" => {
            identity.binding.device_id = "device-b".into();
            ("mobile.inspect", None, 1)
        }
        "scope" => {
            identity.binding.scope.project_id = "foreign".into();
            ("mobile.dispatch", Some(f.payload1.clone()), 1)
        }
        "session" => {
            identity.binding.session_id = "foreign-app".into();
            ("mobile.inspect", None, 1)
        }
        "epoch" => ("mobile.dispatch", Some(f.payload1.clone()), 2),
        _ => panic!("mode"),
    };
    let request = MutationRequest {
        version: 1,
        identity,
        expected_epoch: epoch,
        dispatched_session_id: if operation == "mobile.dispatch" {
            session.session_id().into()
        } else {
            original.into()
        },
        payload,
    };
    let envelope = PublicEnvelope {
        protocol: PROTOCOL.into(),
        kind: MessageKind::Request,
        message_id: format!("refuse-{mode}"),
        sequence: 1,
        reply_to: None,
        route_id: Some("app-session".into()),
        route_epoch: Some(epoch),
        operation: Some(operation.into()),
        scope: Some(scope()),
        trace: None,
        deadline_ms: Some(now() + 10000),
        payload: Some(serde_json::to_value(request).unwrap()),
        error: None,
    };
    session.send(&envelope).await.unwrap();
    assert!(session.receive().await.is_err());
}
#[tokio::test]
async fn encrypted_resume_reconciles_real_effect_before_next_write() {
    let dir = tempfile::tempdir().unwrap();
    let owner = owner().await;
    let mut params = CertificateParams::new(Vec::<String>::new()).unwrap();
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    let issuer = CertifiedIssuer::self_signed(params, KeyPair::generate().unwrap()).unwrap();
    let ca = dir.path().join("ca.der");
    std::fs::write(&ca, issuer.der()).unwrap();
    let server = issue(dir.path(), "server", &issuer);
    let a = issue(dir.path(), "a", &issuer);
    let b = issue(dir.path(), "b", &issuer);
    let f = Fixture {
        role: "crash".into(),
        address: address(),
        ca,
        cert: server.cert.clone(),
        key: server.key.clone(),
        enrollment: dir.path().join("enrollment.db"),
        bindings: dir.path().join("bindings.db"),
        effects: dir.path().join("effects.db"),
        output: dir.path().join("effect.bin"),
        ready: dir.path().join("crash-ready"),
        stage: dir.path().join("effect-applied"),
        payload1: vec![255, 0, 13, 10, 206, 187],
        payload2: b"second".to_vec(),
    };
    let (ticket_a, ticket_b) = {
        let mut store = EnrollmentStore::open(&f.enrollment, &owner).unwrap();
        let review = |cert: &Cert, id: &str| DeviceReview {
            device_id: id.into(),
            name: id.into(),
            certificate_sha256: cert.fingerprint.clone(),
            scope: scope(),
            grants: grants(),
            expires_ms: now() + 120000,
        };
        (
            store.review(&owner, review(&a, "device-a")).unwrap(),
            store.review(&owner, review(&b, "device-b")).unwrap(),
        )
    };
    let binding = JournalBinding {
        scope: scope(),
        device_id: "device-a".into(),
        session_id: "app-session".into(),
    };
    let journal_path = dir.path().join("journal.json");
    let mut journal = MobileJournal::open(&journal_path, binding.clone()).unwrap();
    journal
        .enqueue_mutation("operation-1", "append_file", &digest_bytes(&f.payload1))
        .unwrap();
    let mut child = spawn(&f);
    ready(&f.ready).await;
    let session = connect(&f, &a, Some(ticket_a.nonce)).await;
    let original = session.session_id().to_string();
    let mut encrypted =
        EncryptedJournalSession::new(session, binding.clone(), &server.fingerprint).unwrap();
    let dispatch = encrypted.dispatch_pending(&mut journal, &f.payload1);
    let kill = async {
        ready(&f.stage).await;
        child.kill().unwrap();
        child.wait().unwrap();
    };
    let (result, _) = tokio::join!(dispatch, kill);
    assert!(result.is_err());
    assert_eq!(journal.pending().unwrap().state, MutationState::Unknown);
    assert!(journal
        .enqueue_mutation("operation-2", "append_file", &digest_bytes(&f.payload2))
        .is_err());
    assert!(encrypted.reconcile_pending(&mut journal).await.is_err());
    drop(encrypted);
    drop(journal);
    assert_eq!(std::fs::read(&f.output).unwrap(), f.payload1);
    let mut journal = MobileJournal::open(&journal_path, binding.clone()).unwrap();
    assert_eq!(
        journal.pending().unwrap().dispatched_session_id.as_deref(),
        Some(original.as_str())
    );
    let mut resumed = f.clone();
    resumed.role = "resume".into();
    resumed.address = address();
    resumed.ready = dir.path().join("resume-ready");
    let mut child = spawn(&resumed);
    ready(&resumed.ready).await;
    let session = connect(&resumed, &a, None).await;
    assert_ne!(session.session_id(), original);
    let mut encrypted =
        EncryptedJournalSession::new(session, binding.clone(), &server.fingerprint).unwrap();
    let unknown = encrypted.reconcile_pending(&mut journal).await.unwrap();
    assert_eq!(unknown.state, MutationState::Unknown);
    assert_eq!(unknown.receipt.unwrap().outcome, ReceiptOutcome::Unknown);
    assert_eq!(journal.cursor_receipt().unwrap().sequence, 3);
    assert!(journal
        .enqueue_mutation("operation-2", "append_file", &digest_bytes(&f.payload2))
        .is_err());
    let terminal = encrypted.reconcile_pending(&mut journal).await.unwrap();
    assert_eq!(terminal.state, MutationState::Terminal);
    assert_eq!(terminal.receipt.unwrap().outcome, ReceiptOutcome::Succeeded);
    assert_eq!(journal.cursor_receipt().unwrap().sequence, 5);
    assert!(journal
        .enqueue_mutation("operation-1", "append_file", &digest_bytes(&f.payload1))
        .is_err());
    assert!(encrypted.reconcile_pending(&mut journal).await.is_err());
    journal
        .enqueue_mutation("operation-2", "append_file", &digest_bytes(&f.payload2))
        .unwrap();
    let second = encrypted
        .dispatch_pending(&mut journal, &f.payload2)
        .await
        .unwrap();
    assert_eq!(second.state, MutationState::Terminal);
    assert_eq!(journal.cursor_receipt().unwrap().sequence, 8);
    drop(encrypted);
    let identity = journal.records()[0].identity.clone();
    refused(&resumed, &a, None, identity.clone(), &original, "replay").await;
    refused(
        &resumed,
        &b,
        Some(ticket_b.nonce),
        identity.clone(),
        &original,
        "foreign",
    )
    .await;
    refused(&resumed, &a, None, identity.clone(), &original, "scope").await;
    refused(&resumed, &a, None, identity.clone(), &original, "session").await;
    refused(&resumed, &a, None, identity, &original, "epoch").await;
    assert!(child.wait().unwrap().success());
    let mut expected = f.payload1.clone();
    expected.extend(&f.payload2);
    assert_eq!(std::fs::read(&f.output).unwrap(), expected);
    let effects = EffectStore::open(&f.effects).unwrap();
    let first = effects.get(&scope(), "operation-1").unwrap().unwrap();
    assert_eq!(first.state, EffectState::Terminal);
    assert_eq!(first.version, 4);
    assert_eq!(
        effects
            .get(&scope(), "operation-2")
            .unwrap()
            .unwrap()
            .version,
        3
    );
    drop(journal);
    let reopened = MobileJournal::open(&journal_path, binding).unwrap();
    assert!(reopened.pending().is_none());
    assert_eq!(reopened.cursor_receipt().unwrap().sequence, 8);
    let disk = std::fs::read_to_string(journal_path).unwrap();
    assert!(!disk.contains("observed-file-sha256"));
    assert!(!disk.contains("payload1"));
}
