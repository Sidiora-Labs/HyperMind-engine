use crate::{
    compat_transport::{self, PublicEnvelope, MAX_CHUNK_BYTES, MAX_FRAME_BYTES, PROTOCOL},
    storage::{FencedStore, Migration, StorageError},
    transport::AuthenticatedIdentity,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hm_context::Scope;
use rustls::{
    pki_types::{CertificateDer, PrivateKeyDer, ServerName},
    ClientConfig, RootCertStore, ServerConfig,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    io::Read,
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tokio_rustls::{TlsAcceptor, TlsConnector, TlsStream};

#[derive(Debug, thiserror::Error)]
pub enum EnrollmentError {
    #[error("enrollment refusal: {0}")]
    Refused(String),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
fn refuse(s: &str) -> EnrollmentError {
    EnrollmentError::Refused(s.into())
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn random() -> Result<String, EnrollmentError> {
    let mut b = [0; 32];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut b)?;
    Ok(b.iter().map(|v| format!("{v:02x}")).collect())
}
fn valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 160
        && s.as_bytes()[0].is_ascii_alphanumeric()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
}
fn hex(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn valid_scope(s: &Scope) -> bool {
    valid_id(&s.owner_id)
        && valid_id(&s.project_id)
        && s.workspace_id.as_deref().is_none_or(valid_id)
}
pub type Grants = BTreeSet<String>;
pub fn intersect_grants(a: &Grants, b: &Grants) -> Grants {
    a.intersection(b).cloned().collect()
}
fn valid_grants(g: &Grants) -> bool {
    g.len() <= 128 && g.iter().all(|s| valid_id(s))
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceReview {
    pub device_id: String,
    pub name: String,
    pub certificate_sha256: String,
    pub scope: Scope,
    pub grants: Grants,
    pub expires_ms: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnrollmentTicket {
    pub nonce: String,
    pub expires_ms: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PeerDescriptor {
    device: DeviceReview,
    generation: u64,
}
impl PeerDescriptor {
    pub fn device(&self) -> &DeviceReview {
        &self.device
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
}
#[derive(Clone, Debug)]
pub struct CertificateIdentity {
    fingerprint: String,
}
impl CertificateIdentity {
    pub fn certificate_sha256(&self) -> &str {
        &self.fingerprint
    }
}
const MIGRATIONS:&[Migration]=&[Migration{version:1,name:"peer-enrollment-v1",sql:"CREATE TABLE owner(id INTEGER PRIMARY KEY CHECK(id=1), key BLOB NOT NULL, scope TEXT NOT NULL); CREATE TABLE reviews(nonce TEXT PRIMARY KEY, body TEXT NOT NULL, expires INTEGER NOT NULL); CREATE TABLE devices(fingerprint TEXT PRIMARY KEY, id TEXT UNIQUE NOT NULL, body TEXT NOT NULL, generation INTEGER NOT NULL, revoked INTEGER NOT NULL); CREATE TABLE session_nonces(fingerprint TEXT NOT NULL,nonce TEXT NOT NULL, PRIMARY KEY(fingerprint,nonce));"}];
pub struct EnrollmentStore {
    store: FencedStore,
    owner_key: [u8; 32],
    owner_scope: Scope,
}
impl EnrollmentStore {
    pub fn open(
        path: impl AsRef<Path>,
        owner: &AuthenticatedIdentity,
    ) -> Result<Self, EnrollmentError> {
        let mut store = FencedStore::open(path, MIGRATIONS)?;
        let key = *owner.peer_key();
        let scope = owner.scope().clone();
        let encoded = serde_json::to_string(&scope).unwrap();
        let epoch = store.epoch();
        let matches = store.transaction(epoch, |tx| {
            tx.execute(
                "INSERT OR IGNORE INTO owner VALUES(1,?1,?2)",
                rusqlite::params![key.as_slice(), encoded],
            )?;
            let current: (Vec<u8>, String) =
                tx.query_row("SELECT key,scope FROM owner WHERE id=1", [], |r| {
                    Ok((r.get(0)?, r.get(1)?))
                })?;
            Ok(current.0 == key && current.1 == encoded)
        })?;
        if !matches {
            return Err(refuse("owner identity mismatch"));
        }
        Ok(Self {
            store,
            owner_key: key,
            owner_scope: scope,
        })
    }
    fn owner(&self, owner: &AuthenticatedIdentity) -> Result<(), EnrollmentError> {
        if owner.peer_key() != &self.owner_key || owner.scope() != &self.owner_scope {
            return Err(refuse("owner review requires bound authenticated identity"));
        }
        Ok(())
    }
    pub fn review(
        &mut self,
        owner: &AuthenticatedIdentity,
        review: DeviceReview,
    ) -> Result<EnrollmentTicket, EnrollmentError> {
        self.owner(owner)?;
        let time = now();
        if !valid_id(&review.device_id)
            || review.name.is_empty()
            || review.name.chars().count() > 128
            || review.name.chars().any(char::is_control)
            || !hex(&review.certificate_sha256)
            || !valid_scope(&review.scope)
            || review.scope != self.owner_scope
            || !valid_grants(&review.grants)
            || review.expires_ms <= time
            || review.expires_ms > time + 365 * 86400 * 1000
        {
            return Err(refuse("invalid device review"));
        }
        let ticket = EnrollmentTicket {
            nonce: random()?,
            expires_ms: review.expires_ms.min(time + 300_000),
        };
        let body = serde_json::to_string(&review).unwrap();
        let epoch = self.store.epoch();
        let admitted = self.store.transaction(epoch, |tx| {
            let n: i64 = tx.query_row("SELECT count(*) FROM reviews", [], |r| r.get(0))?;
            if n >= 1024 {
                return Ok(false);
            }
            tx.execute(
                "INSERT INTO reviews VALUES(?1,?2,?3)",
                rusqlite::params![ticket.nonce, body, ticket.expires_ms as i64],
            )?;
            Ok(true)
        })?;
        if !admitted {
            return Err(refuse("review capacity"));
        }
        Ok(ticket)
    }
    pub fn enroll(
        &mut self,
        identity: &CertificateIdentity,
        ticket: &str,
    ) -> Result<PeerDescriptor, EnrollmentError> {
        self.enroll_bound(identity, ticket, None)
    }
    fn enroll_bound(
        &mut self,
        identity: &CertificateIdentity,
        ticket: &str,
        binding: Option<(&Scope, &str)>,
    ) -> Result<PeerDescriptor, EnrollmentError> {
        if !hex(ticket) {
            return Err(refuse("invalid enrollment nonce"));
        }
        let epoch = self.store.epoch();
        let result = self.store.transaction(epoch, |tx| {
            let row = tx.query_row("SELECT body,expires FROM reviews WHERE nonce=?1", [ticket], |r| Ok((r.get::<_,String>(0)?, r.get::<_,i64>(1)?)));
            let (body, expires) = match row { Ok(v) => v, Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None), Err(e) => return Err(e.into()) };
            let device: DeviceReview = serde_json::from_str(&body).map_err(|_| StorageError::MigrationMismatch)?;
            if expires <= now() as i64 || device.expires_ms <= now() || device.certificate_sha256 != identity.fingerprint { return Ok(None); }
            if let Some((scope, nonce)) = binding {
                if scope != &device.scope { return Ok(None); }
                let count:i64 = tx.query_row("SELECT count(*) FROM session_nonces", [], |r| r.get(0))?;
                let used:bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM session_nonces WHERE fingerprint=?1 AND nonce=?2)", rusqlite::params![identity.fingerprint,nonce], |r| r.get(0))?;
                if count >= 65536 || used { return Ok(None); }
            }
            let n:i64 = tx.query_row("SELECT count(*) FROM devices", [], |r| r.get(0))?;
            if n >= 1024 { return Ok(None); }
            tx.execute("INSERT INTO devices VALUES(?1,?2,?3,1,0) ON CONFLICT(fingerprint) DO UPDATE SET body=excluded.body,id=excluded.id,generation=devices.generation+1,revoked=0", rusqlite::params![identity.fingerprint,device.device_id,body])?;
            tx.execute("DELETE FROM reviews WHERE nonce=?1", [ticket])?;
            if let Some((_,nonce)) = binding { tx.execute("INSERT INTO session_nonces VALUES(?1,?2)", rusqlite::params![identity.fingerprint,nonce])?; }
            let generation:i64 = tx.query_row("SELECT generation FROM devices WHERE fingerprint=?1", [&identity.fingerprint], |r| r.get(0))?;
            Ok(Some(PeerDescriptor { device, generation: u64::try_from(generation).map_err(|_| StorageError::MigrationMismatch)? }))
        })?;
        result.ok_or_else(|| refuse("ticket expired, replayed, wrong scope or certificate"))
    }
    pub fn authorize(
        &self,
        identity: &CertificateIdentity,
    ) -> Result<PeerDescriptor, EnrollmentError> {
        let result = self.store.read(|db| {
            let row = db.query_row(
                "SELECT body,generation,revoked FROM devices WHERE fingerprint=?1",
                [&identity.fingerprint],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, i64>(1)?,
                        r.get::<_, bool>(2)?,
                    ))
                },
            );
            match row {
                Ok((body, generation, false)) => {
                    let device: DeviceReview =
                        serde_json::from_str(&body).map_err(|_| StorageError::MigrationMismatch)?;
                    Ok(Some(PeerDescriptor {
                        device,
                        generation: u64::try_from(generation)
                            .map_err(|_| StorageError::MigrationMismatch)?,
                    }))
                }
                Ok(_) | Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                Err(e) => Err(e.into()),
            }
        })?;
        result
            .filter(|p| p.device.expires_ms > now())
            .ok_or_else(|| refuse("device unavailable, expired or revoked"))
    }
    pub fn revoke(
        &mut self,
        owner: &AuthenticatedIdentity,
        device_id: &str,
    ) -> Result<(), EnrollmentError> {
        self.owner(owner)?;
        let epoch = self.store.epoch();
        self.store.transaction(epoch, |tx| {
            tx.execute(
                "UPDATE devices SET revoked=1,generation=generation+1 WHERE id=?1",
                [device_id],
            )?;
            Ok(())
        })?;
        Ok(())
    }
    fn nonce(
        &mut self,
        identity: &CertificateIdentity,
        nonce: &str,
    ) -> Result<(), EnrollmentError> {
        let epoch = self.store.epoch();
        let ok = self.store.transaction(epoch, |tx| {
            let n: i64 = tx.query_row("SELECT count(*) FROM session_nonces", [], |r| r.get(0))?;
            if n >= 65536 {
                return Ok(false);
            }
            Ok(tx.execute(
                "INSERT OR IGNORE INTO session_nonces VALUES(?1,?2)",
                rusqlite::params![identity.fingerprint, nonce],
            )? == 1)
        })?;
        if !ok {
            return Err(refuse("client nonce replay or capacity"));
        }
        Ok(())
    }
}
#[derive(Clone)]
pub struct MutualServerConfig(Arc<ServerConfig>);
#[derive(Clone)]
pub struct MutualClientConfig(Arc<ClientConfig>);
pub fn server_config(
    roots: RootCertStore,
    certs: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static>,
) -> Result<MutualServerConfig, EnrollmentError> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
        Arc::new(roots),
        provider.clone(),
    )
    .build()
    .map_err(|e| EnrollmentError::Refused(e.to_string()))?;
    let mut config = ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|e| EnrollmentError::Refused(e.to_string()))?
        .with_client_cert_verifier(verifier)
        .with_single_cert(certs, key)
        .map_err(|e| EnrollmentError::Refused(e.to_string()))?;
    config.alpn_protocols = vec![PROTOCOL.as_bytes().to_vec()];
    Ok(MutualServerConfig(Arc::new(config)))
}
pub fn client_config(
    roots: RootCertStore,
    certs: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static>,
) -> Result<MutualClientConfig, EnrollmentError> {
    let mut config =
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(|e| EnrollmentError::Refused(e.to_string()))?
            .with_root_certificates(roots)
            .with_client_auth_cert(certs, key)
            .map_err(|e| EnrollmentError::Refused(e.to_string()))?;
    config.alpn_protocols = vec![PROTOCOL.as_bytes().to_vec()];
    Ok(MutualClientConfig(Arc::new(config)))
}
fn certificate(stream: &TlsStream<TcpStream>) -> Result<CertificateIdentity, EnrollmentError> {
    let (_, conn) = stream.get_ref();
    if conn.protocol_version() != Some(rustls::ProtocolVersion::TLSv1_3)
        || conn.alpn_protocol() != Some(PROTOCOL.as_bytes())
    {
        return Err(refuse("TLS1.3 and public protocol required"));
    }
    let cert = conn
        .peer_certificates()
        .and_then(|v| v.first())
        .ok_or_else(|| refuse("mutual certificate required"))?;
    Ok(CertificateIdentity {
        fingerprint: format!("{:x}", Sha256::digest(cert.as_ref())),
    })
}
async fn read_frame(
    stream: &mut TlsStream<TcpStream>,
    limit: usize,
) -> Result<Vec<u8>, EnrollmentError> {
    let n = stream.read_u32().await? as usize;
    if n == 0 || n > limit {
        return Err(refuse("frame capacity"));
    }
    let mut bytes = vec![0; n + 4];
    bytes[..4].copy_from_slice(&(n as u32).to_be_bytes());
    stream.read_exact(&mut bytes[4..]).await?;
    Ok(bytes)
}
async fn write_json<T: Serialize>(
    stream: &mut TlsStream<TcpStream>,
    v: &T,
) -> Result<(), EnrollmentError> {
    let bytes = serde_json::to_vec(v).map_err(|_| refuse("handshake JSON"))?;
    if bytes.len() > 16384 {
        return Err(refuse("handshake capacity"));
    }
    stream.write_u32(bytes.len() as u32).await?;
    stream.write_all(&bytes).await?;
    stream.flush().await?;
    Ok(())
}
async fn read_json<T: serde::de::DeserializeOwned>(
    stream: &mut TlsStream<TcpStream>,
) -> Result<T, EnrollmentError> {
    let b = read_frame(stream, 16384).await?;
    serde_json::from_slice(&b[4..]).map_err(|_| refuse("handshake JSON"))
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Hello {
    kind: String,
    protocols: Vec<String>,
    client_nonce: String,
    connection_class: String,
    scope: Scope,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Challenge {
    kind: String,
    protocol: String,
    server_nonce: String,
    daemon_instance_id: String,
    auth_method: String,
    expires_ms: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Authentication {
    kind: String,
    proof: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    module_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    spawn_generation: Option<u64>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Principal {
    id: String,
    kind: String,
    scopes: Grants,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConnectionLimits {
    max_frame_bytes: u64,
    event_chunk_bytes: u64,
    max_routes: u16,
    max_inflight_requests: u16,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Accepted {
    kind: String,
    protocol: String,
    session_id: String,
    principal: Principal,
    limits: ConnectionLimits,
    server_time_ms: u64,
}
fn proof(
    stream: &TlsStream<TcpStream>,
    hello: &Hello,
    challenge: &Challenge,
) -> Result<String, EnrollmentError> {
    let context = serde_json::to_vec(&(hello, challenge)).unwrap();
    let context = Some(context.as_slice());
    let label = b"EXPORTER-HyperMind-public-session-v1";
    let out = match stream {
        TlsStream::Client(stream) => stream
            .get_ref()
            .1
            .export_keying_material([0u8; 32], label, context),
        TlsStream::Server(stream) => stream
            .get_ref()
            .1
            .export_keying_material([0u8; 32], label, context),
    }
    .map_err(|e| EnrollmentError::Refused(e.to_string()))?;
    Ok(URL_SAFE_NO_PAD.encode(out))
}
fn locked(
    store: &Arc<Mutex<EnrollmentStore>>,
) -> Result<std::sync::MutexGuard<'_, EnrollmentStore>, EnrollmentError> {
    store.lock().map_err(|_| refuse("store lock poisoned"))
}
pub struct PublicTlsSession {
    stream: TlsStream<TcpStream>,
    identity: CertificateIdentity,
    scope: Scope,
    grants: Grants,
    session_id: String,
    device: Option<PeerDescriptor>,
    store: Option<Arc<Mutex<EnrollmentStore>>>,
    incoming: u64,
    outgoing: u64,
    closed: bool,
}
impl PublicTlsSession {
    pub fn identity(&self) -> &CertificateIdentity {
        &self.identity
    }
    pub fn scope(&self) -> &Scope {
        &self.scope
    }
    pub fn grants(&self) -> &Grants {
        &self.grants
    }
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
    pub fn device(&self) -> Option<&PeerDescriptor> {
        self.device.as_ref()
    }
    pub async fn accept(
        stream: TcpStream,
        config: MutualServerConfig,
        store: Arc<Mutex<EnrollmentStore>>,
        local_grants: Grants,
        daemon_id: &str,
    ) -> Result<Self, EnrollmentError> {
        tokio::time::timeout(
            Duration::from_secs(10),
            Self::accept_inner(stream, config, store, local_grants, daemon_id),
        )
        .await
        .map_err(|_| refuse("handshake deadline"))?
    }
    async fn accept_inner(
        stream: TcpStream,
        config: MutualServerConfig,
        store: Arc<Mutex<EnrollmentStore>>,
        local_grants: Grants,
        daemon_id: &str,
    ) -> Result<Self, EnrollmentError> {
        if !valid_grants(&local_grants) || !valid_id(daemon_id) {
            return Err(refuse("listener configuration"));
        }
        let mut stream = TlsStream::Server(TlsAcceptor::from(config.0).accept(stream).await?);
        let identity = certificate(&stream)?;
        let hello: Hello = read_json(&mut stream).await?;
        if hello.kind != "hello"
            || hello.protocols.len() > 8
            || !hello.protocols.iter().any(|s| s == PROTOCOL)
        {
            write_json(&mut stream,&serde_json::json!({"kind":"close","error":{"code":"UNSUPPORTED_PROTOCOL","message":"client and daemon protocol ranges do not overlap","retryable":false},"supported_protocol_min":PROTOCOL,"supported_protocol_max":PROTOCOL})).await?;
            return Err(refuse("unsupported protocol"));
        }
        if !hex(&hello.client_nonce)
            || hello.connection_class != "device"
            || !valid_scope(&hello.scope)
        {
            return Err(refuse("invalid hello"));
        }
        let challenge = Challenge {
            kind: "challenge".into(),
            protocol: PROTOCOL.into(),
            server_nonce: random()?,
            daemon_instance_id: daemon_id.into(),
            auth_method: "tls_certificate".into(),
            expires_ms: now() + 10_000,
        };
        write_json(&mut stream, &challenge).await?;
        let auth: Authentication = read_json(&mut stream).await?;
        let expected = proof(&stream, &hello, &challenge)?;
        use subtle::ConstantTimeEq;
        if auth.kind != "authenticate"
            || auth.module_id.is_some()
            || auth.spawn_generation.is_some()
            || now() >= challenge.expires_ms
            || !bool::from(auth.proof.as_bytes().ct_eq(expected.as_bytes()))
        {
            return Err(refuse("certificate session proof"));
        }
        let device = {
            let mut store = locked(&store)?;
            let device = if let Some(ticket) = &auth.token {
                store.enroll_bound(
                    &identity,
                    &ticket,
                    Some((&hello.scope, &hello.client_nonce)),
                )?
            } else {
                store.authorize(&identity)?
            };
            if device.device.scope != hello.scope {
                return Err(refuse("scope mismatch"));
            }
            if auth.token.is_none() {
                store.nonce(&identity, &hello.client_nonce)?;
            }
            device
        };
        let grants = intersect_grants(&device.device.grants, &local_grants);
        let session_id = random()?;
        let accepted = Accepted {
            kind: "accepted".into(),
            protocol: PROTOCOL.into(),
            session_id: session_id.clone(),
            principal: Principal {
                id: device.device.device_id.clone(),
                kind: "device".into(),
                scopes: grants.clone(),
            },
            limits: ConnectionLimits {
                max_frame_bytes: MAX_FRAME_BYTES as u64,
                event_chunk_bytes: MAX_CHUNK_BYTES as u64,
                max_routes: 1024,
                max_inflight_requests: 1024,
            },
            server_time_ms: now(),
        };
        write_json(&mut stream, &accepted).await?;
        Ok(Self {
            stream,
            identity,
            scope: hello.scope,
            grants,
            session_id,
            device: Some(device),
            store: Some(store),
            incoming: 0,
            outgoing: 0,
            closed: false,
        })
    }
    pub async fn connect(
        stream: TcpStream,
        config: MutualClientConfig,
        server_name: ServerName<'static>,
        scope: Scope,
        ticket: Option<String>,
        allowed_grants: Grants,
    ) -> Result<Self, EnrollmentError> {
        Self::connect_with_nonce(
            stream,
            config,
            server_name,
            scope,
            ticket,
            allowed_grants,
            random()?,
        )
        .await
    }
    pub async fn connect_with_nonce(
        stream: TcpStream,
        config: MutualClientConfig,
        server_name: ServerName<'static>,
        scope: Scope,
        ticket: Option<String>,
        allowed_grants: Grants,
        client_nonce: String,
    ) -> Result<Self, EnrollmentError> {
        tokio::time::timeout(Duration::from_secs(10), async {
            if !valid_scope(&scope) || !valid_grants(&allowed_grants) || !hex(&client_nonce) {
                return Err(refuse("client configuration"));
            }
            let mut stream = TlsStream::Client(
                TlsConnector::from(config.0)
                    .connect(server_name, stream)
                    .await?,
            );
            let identity = certificate(&stream)?;
            let hello = Hello {
                kind: "hello".into(),
                protocols: vec![PROTOCOL.into()],
                client_nonce,
                connection_class: "device".into(),
                scope: scope.clone(),
            };
            write_json(&mut stream, &hello).await?;
            let challenge: Challenge = read_json(&mut stream).await?;
            if challenge.kind != "challenge"
                || challenge.protocol != PROTOCOL
                || challenge.auth_method != "tls_certificate"
                || !hex(&challenge.server_nonce)
                || !valid_id(&challenge.daemon_instance_id)
                || challenge.expires_ms <= now()
                || challenge.expires_ms > now() + 30_000
            {
                return Err(refuse("invalid server challenge"));
            }
            let auth = Authentication {
                kind: "authenticate".into(),
                proof: proof(&stream, &hello, &challenge)?,
                token: ticket,
                module_id: None,
                spawn_generation: None,
            };
            write_json(&mut stream, &auth).await?;
            let accepted: Accepted = read_json(&mut stream).await?;
            if accepted.kind != "accepted"
                || accepted.protocol != PROTOCOL
                || !valid_id(&accepted.session_id)
                || accepted.principal.kind != "device"
                || !valid_id(&accepted.principal.id)
                || !accepted.principal.scopes.is_subset(&allowed_grants)
                || accepted.limits.max_frame_bytes != MAX_FRAME_BYTES as u64
                || accepted.limits.event_chunk_bytes != MAX_CHUNK_BYTES as u64
            {
                return Err(refuse("invalid accepted session"));
            }
            Ok(Self {
                stream,
                identity,
                scope,
                grants: accepted.principal.scopes,
                session_id: accepted.session_id,
                device: None,
                store: None,
                incoming: 0,
                outgoing: 0,
                closed: false,
            })
        })
        .await
        .map_err(|_| refuse("handshake deadline"))?
    }
    fn live(&self) -> Result<(), EnrollmentError> {
        if self.closed {
            return Err(refuse("session closed"));
        }
        if let Some(store) = &self.store {
            let current = locked(store)?.authorize(&self.identity)?;
            if self
                .device
                .as_ref()
                .is_none_or(|d| d.generation != current.generation)
            {
                return Err(refuse("session generation revoked"));
            }
        }
        Ok(())
    }
    fn envelope(&self, e: &PublicEnvelope) -> Result<(), EnrollmentError> {
        if e.scope.as_ref().is_some_and(|s| s != &self.scope) {
            return Err(refuse("frame scope mismatch"));
        }
        if let Some(op) = &e.operation {
            if !self.grants.contains(op) {
                return Err(refuse("operation outside effective grants"));
            }
        }
        Ok(())
    }
    pub async fn send(&mut self, e: &PublicEnvelope) -> Result<(), EnrollmentError> {
        self.live()?;
        self.envelope(e)?;
        if e.sequence != self.outgoing + 1 {
            return Err(refuse("outbound sequence"));
        }
        let bytes =
            compat_transport::encode(e).map_err(|v| EnrollmentError::Refused(v.to_string()))?;
        self.closed = true;
        tokio::time::timeout(Duration::from_secs(10), async {
            self.stream.write_all(&bytes).await?;
            self.stream.flush().await
        })
        .await
        .map_err(|_| refuse("write deadline"))??;
        self.outgoing = e.sequence;
        self.closed = false;
        Ok(())
    }
    pub async fn receive(&mut self) -> Result<PublicEnvelope, EnrollmentError> {
        self.live()?;
        self.closed = true;
        let bytes = tokio::time::timeout(
            Duration::from_secs(10),
            read_frame(&mut self.stream, MAX_FRAME_BYTES),
        )
        .await
        .map_err(|_| refuse("read deadline"))??;
        let e = compat_transport::decode(&bytes)
            .map_err(|v| EnrollmentError::Refused(v.to_string()))?;
        if e.sequence != self.incoming + 1 {
            return Err(refuse("inbound replay"));
        }
        self.envelope(&e)?;
        self.closed = false;
        self.live()?;
        self.incoming = e.sequence;
        Ok(e)
    }
}
