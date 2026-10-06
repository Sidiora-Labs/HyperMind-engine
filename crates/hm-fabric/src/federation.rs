use crate::{
    compat_transport::{MessageKind, PublicEnvelope, PROTOCOL},
    enrollment::{intersect_grants, EnrollmentError, Grants, MutualClientConfig, PublicTlsSession},
    storage::{FencedStore, Migration, StorageError},
    transport::AuthenticatedIdentity,
};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use hm_context::Scope;
use rustls::pki_types::ServerName;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    net::SocketAddr,
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::net::TcpStream;
#[derive(Debug, thiserror::Error)]
pub enum FederationError {
    #[error("federation refusal: {0}")]
    Refused(String),
    #[error(transparent)]
    Enrollment(#[from] EnrollmentError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
fn refuse(s: &str) -> FederationError {
    FederationError::Refused(s.into())
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn id(s: &str) -> bool {
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
fn grants(g: &Grants) -> bool {
    g.len() <= 128 && g.iter().all(|s| id(s))
}
fn scope(s: &Scope) -> bool {
    id(&s.owner_id) && id(&s.project_id) && s.workspace_id.as_deref().is_none_or(id)
}
fn hash<T: Serialize>(v: &T) -> String {
    format!("{:x}", Sha256::digest(serde_json::to_vec(v).unwrap()))
}
fn sign<T: Serialize>(domain: &[u8], body: &T, key: &SigningKey) -> Vec<u8> {
    let mut bytes = domain.to_vec();
    bytes.extend(serde_json::to_vec(body).unwrap());
    key.sign(&bytes).to_bytes().to_vec()
}
fn verify<T: Serialize>(
    domain: &[u8],
    body: &T,
    signature: &[u8],
    key: &[u8; 32],
) -> Result<(), FederationError> {
    let sig = Signature::from_slice(signature).map_err(|_| refuse("signature length"))?;
    let key = VerifyingKey::from_bytes(key).map_err(|_| refuse("signer key"))?;
    let mut bytes = domain.to_vec();
    bytes.extend(serde_json::to_vec(body).map_err(|_| refuse("signed JSON"))?);
    key.verify_strict(&bytes, &sig)
        .map_err(|_| refuse("signed data authentication"))
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteCatalog {
    pub generation: u64,
    pub operations: BTreeMap<String, u32>,
}
impl RemoteCatalog {
    fn validate(&self) -> Result<(), FederationError> {
        if self.generation == 0
            || self.generation > 9_007_199_254_740_991
            || self.operations.is_empty()
            || self.operations.len() > 128
            || self.operations.iter().any(|(k, v)| !id(k) || *v == 0)
        {
            return Err(refuse("invalid remote catalog"));
        }
        Ok(())
    }
    pub fn digest(&self) -> String {
        hash(self)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Endpoint {
    pub address: SocketAddr,
    pub server_name: String,
    pub certificate_sha256: String,
}
impl Endpoint {
    fn validate(&self) -> Result<(), FederationError> {
        if self.address.port() == 0
            || self.server_name.len() > 253
            || ServerName::try_from(self.server_name.clone()).is_err()
            || !hex(&self.certificate_sha256)
        {
            return Err(refuse("invalid pinned endpoint"));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateAdvertisement {
    pub version: u32,
    pub peer_id: String,
    pub generation: u64,
    pub scope: Scope,
    pub expires_ms: u64,
    pub direct: Vec<Endpoint>,
    pub relay: Option<Endpoint>,
    pub relay_peer_certificate_sha256: Option<String>,
    pub grants: Grants,
    pub catalog: RemoteCatalog,
}
impl CandidateAdvertisement {
    fn validate(&self) -> Result<(), FederationError> {
        self.catalog.validate()?;
        if self.version != 1
            || !id(&self.peer_id)
            || self.generation == 0
            || self.generation > 9_007_199_254_740_991
            || !scope(&self.scope)
            || self.expires_ms <= now()
            || self.expires_ms > now() + 300_000
            || self.direct.len() > 8
            || self.direct.is_empty() && self.relay.is_none()
            || !grants(&self.grants)
            || self.relay.is_some() != self.relay_peer_certificate_sha256.is_some()
            || self
                .relay_peer_certificate_sha256
                .as_deref()
                .is_some_and(|s| !hex(s))
        {
            return Err(refuse("invalid or expired candidates"));
        }
        for endpoint in self.direct.iter().chain(self.relay.iter()) {
            endpoint.validate()?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedCandidate {
    pub advertisement: CandidateAdvertisement,
    pub signature: Vec<u8>,
}
impl SignedCandidate {
    pub fn sign(
        advertisement: CandidateAdvertisement,
        key: &SigningKey,
    ) -> Result<Self, FederationError> {
        advertisement.validate()?;
        let signature = sign(b"hypermind-federation-candidate-v1\0", &advertisement, key);
        Ok(Self {
            advertisement,
            signature,
        })
    }
    pub fn from_slice(bytes: &[u8]) -> Result<Self, FederationError> {
        if bytes.len() > 65536 {
            return Err(refuse("candidate capacity"));
        }
        serde_json::from_slice(bytes).map_err(|_| refuse("candidate JSON"))
    }
}
#[derive(Clone, Debug)]
pub struct VerifiedCandidate {
    advertisement: CandidateAdvertisement,
}
impl VerifiedCandidate {
    pub fn advertisement(&self) -> &CandidateAdvertisement {
        &self.advertisement
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelaySide {
    Initiator,
    Responder,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelayGrant {
    pub version: u32,
    pub pair_id: String,
    pub nonce: String,
    pub side: RelaySide,
    pub certificate_sha256: String,
    pub remote_certificate_sha256: String,
    pub scope: Scope,
    pub expires_ms: u64,
    pub grants: Grants,
    pub catalog: RemoteCatalog,
}
impl RelayGrant {
    fn validate(&self) -> Result<(), FederationError> {
        self.catalog.validate()?;
        if self.version != 1
            || !id(&self.pair_id)
            || !hex(&self.nonce)
            || !hex(&self.certificate_sha256)
            || !hex(&self.remote_certificate_sha256)
            || self.certificate_sha256 == self.remote_certificate_sha256
            || !scope(&self.scope)
            || !grants(&self.grants)
            || self.expires_ms <= now()
            || self.expires_ms > now() + 300_000
        {
            return Err(refuse("invalid or expired relay grant"));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedRelayGrant {
    pub grant: RelayGrant,
    pub signature: Vec<u8>,
}
impl SignedRelayGrant {
    pub fn sign(grant: RelayGrant, key: &SigningKey) -> Result<Self, FederationError> {
        grant.validate()?;
        let signature = sign(b"hypermind-federation-relay-grant-v1\0", &grant, key);
        Ok(Self { grant, signature })
    }
}
const MIGRATIONS:&[Migration]=&[Migration{version:1,name:"federation-admission-v1",sql:"CREATE TABLE owner(id INTEGER PRIMARY KEY CHECK(id=1),key BLOB NOT NULL,scope TEXT NOT NULL);CREATE TABLE advertisements(peer TEXT PRIMARY KEY,generation INTEGER NOT NULL,digest TEXT NOT NULL);CREATE TABLE used_grants(nonce TEXT PRIMARY KEY,pair TEXT NOT NULL,digest TEXT NOT NULL);CREATE TABLE revoked_pairs(pair TEXT PRIMARY KEY);"}];
pub struct FederationStore {
    store: FencedStore,
    owner_key: [u8; 32],
    owner_scope: Scope,
}
impl FederationStore {
    pub fn open(
        path: impl AsRef<Path>,
        owner: &AuthenticatedIdentity,
    ) -> Result<Self, FederationError> {
        let mut store = FencedStore::open(path, MIGRATIONS)?;
        let key = *owner.peer_key();
        let scope = owner.scope().clone();
        let encoded = serde_json::to_string(&scope).unwrap();
        let epoch = store.epoch();
        let ok = store.transaction(epoch, |tx| {
            tx.execute(
                "INSERT OR IGNORE INTO owner VALUES(1,?1,?2)",
                rusqlite::params![key.as_slice(), encoded],
            )?;
            let row: (Vec<u8>, String) =
                tx.query_row("SELECT key,scope FROM owner WHERE id=1", [], |r| {
                    Ok((r.get(0)?, r.get(1)?))
                })?;
            Ok(row.0 == key && row.1 == encoded)
        })?;
        if !ok {
            return Err(refuse("federation owner identity"));
        }
        Ok(Self {
            store,
            owner_key: key,
            owner_scope: scope,
        })
    }
    pub fn accept_candidate(
        &mut self,
        identity: &AuthenticatedIdentity,
        candidate: SignedCandidate,
    ) -> Result<VerifiedCandidate, FederationError> {
        candidate.advertisement.validate()?;
        if identity.scope() != &candidate.advertisement.scope
            || identity.scope() != &self.owner_scope
        {
            return Err(refuse("advertiser scope"));
        }
        verify(
            b"hypermind-federation-candidate-v1\0",
            &candidate.advertisement,
            &candidate.signature,
            identity.peer_key(),
        )?;
        let body = &candidate.advertisement;
        let digest = hash(body);
        let epoch = self.store.epoch();
        let ok=self.store.transaction(epoch,|tx|{let row=tx.query_row("SELECT generation,digest FROM advertisements WHERE peer=?1",[&body.peer_id],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?)));match row{Ok((generation,old)) if generation as u64>body.generation||generation as u64==body.generation&&old!=digest=>return Ok(false),Ok(_)=>{},Err(rusqlite::Error::QueryReturnedNoRows)=>{let count:i64=tx.query_row("SELECT count(*) FROM advertisements",[],|r|r.get(0))?;if count>=1024{return Ok(false);}},Err(e)=>return Err(e.into())}tx.execute("INSERT INTO advertisements VALUES(?1,?2,?3) ON CONFLICT(peer) DO UPDATE SET generation=excluded.generation,digest=excluded.digest",rusqlite::params![body.peer_id,body.generation as i64,digest])?;Ok(true)})?;
        if !ok {
            return Err(refuse("stale or conflicting advertisement"));
        }
        Ok(VerifiedCandidate {
            advertisement: candidate.advertisement,
        })
    }
    pub fn revoke_pair(
        &mut self,
        owner: &AuthenticatedIdentity,
        pair_id: &str,
    ) -> Result<(), FederationError> {
        if owner.peer_key() != &self.owner_key || owner.scope() != &self.owner_scope || !id(pair_id)
        {
            return Err(refuse("relay revoke authority"));
        }
        let epoch = self.store.epoch();
        let admitted = self.store.transaction(epoch, |tx| {
            let exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM revoked_pairs WHERE pair=?1)",
                [pair_id],
                |r| r.get(0),
            )?;
            let count: i64 =
                tx.query_row("SELECT count(*) FROM revoked_pairs", [], |r| r.get(0))?;
            if !exists && count >= 65536 {
                return Ok(false);
            }
            tx.execute("INSERT OR IGNORE INTO revoked_pairs VALUES(?1)", [pair_id])?;
            Ok(true)
        })?;
        if !admitted {
            return Err(refuse("relay revocation capacity"));
        }
        Ok(())
    }
    fn consume(
        &mut self,
        session: &PublicTlsSession,
        signed: &SignedRelayGrant,
    ) -> Result<(), FederationError> {
        let body = &signed.grant;
        body.validate()?;
        if session.device().is_none()
            || session.identity().certificate_sha256() != body.certificate_sha256
            || session.scope() != &body.scope
            || body.scope != self.owner_scope
        {
            return Err(refuse("relay grant certificate or scope"));
        }
        verify(
            b"hypermind-federation-relay-grant-v1\0",
            body,
            &signed.signature,
            &self.owner_key,
        )?;
        let epoch = self.store.epoch();
        let ok = self.store.transaction(epoch, |tx| {
            let revoked: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM revoked_pairs WHERE pair=?1)",
                [&body.pair_id],
                |r| r.get(0),
            )?;
            let count: i64 = tx.query_row("SELECT count(*) FROM used_grants", [], |r| r.get(0))?;
            if revoked || count >= 65536 {
                return Ok(false);
            }
            Ok(tx.execute(
                "INSERT OR IGNORE INTO used_grants VALUES(?1,?2,?3)",
                rusqlite::params![body.nonce, body.pair_id, hash(body)],
            )? == 1)
        })?;
        if !ok {
            return Err(refuse("relay grant revoked, replayed or capacity"));
        }
        Ok(())
    }
    fn live(&self, grant: &RelayGrant) -> Result<(), FederationError> {
        if grant.expires_ms <= now() {
            return Err(refuse("relay grant expired"));
        }
        let ok = self.store.read(|db| {
            let revoked: bool = db.query_row(
                "SELECT EXISTS(SELECT 1 FROM revoked_pairs WHERE pair=?1)",
                [&grant.pair_id],
                |r| r.get(0),
            )?;
            let exists: bool = db.query_row(
                "SELECT EXISTS(SELECT 1 FROM used_grants WHERE nonce=?1 AND digest=?2)",
                rusqlite::params![grant.nonce, hash(grant)],
                |r| r.get(0),
            )?;
            Ok(!revoked && exists)
        })?;
        if !ok {
            return Err(refuse("relay grant no longer live"));
        }
        Ok(())
    }
}
fn lock(
    store: &Arc<Mutex<FederationStore>>,
) -> Result<std::sync::MutexGuard<'_, FederationStore>, FederationError> {
    store.lock().map_err(|_| refuse("federation store lock"))
}
pub struct RelayAdmission {
    session: PublicTlsSession,
    grant: RelayGrant,
    message_id: String,
}
pub struct RelayBroker {
    store: Arc<Mutex<FederationStore>>,
}
impl RelayBroker {
    pub fn new(store: Arc<Mutex<FederationStore>>) -> Self {
        Self { store }
    }
    pub async fn admit(
        &self,
        mut session: PublicTlsSession,
    ) -> Result<RelayAdmission, FederationError> {
        let e = session.receive().await?;
        if e.kind != MessageKind::Request
            || e.operation.as_deref() != Some("relay")
            || e.sequence != 1
        {
            return Err(refuse("expected relay admission"));
        }
        let payload = e.payload.ok_or_else(|| refuse("missing relay grant"))?;
        let signed: SignedRelayGrant =
            serde_json::from_value(payload).map_err(|_| refuse("relay grant JSON"))?;
        lock(&self.store)?.consume(&session, &signed)?;
        Ok(RelayAdmission {
            session,
            grant: signed.grant,
            message_id: e.message_id,
        })
    }
    pub async fn pair(
        &self,
        a: RelayAdmission,
        b: RelayAdmission,
    ) -> Result<RelayPair, FederationError> {
        let (mut a, mut b) = if a.grant.side == RelaySide::Initiator {
            (a, b)
        } else {
            (b, a)
        };
        if a.grant.side != RelaySide::Initiator
            || b.grant.side != RelaySide::Responder
            || a.grant.pair_id != b.grant.pair_id
            || a.grant.scope != b.grant.scope
            || a.grant.remote_certificate_sha256 != b.grant.certificate_sha256
            || b.grant.remote_certificate_sha256 != a.grant.certificate_sha256
            || a.grant.catalog != b.grant.catalog
        {
            return Err(refuse("relay side, peer or catalog pairing"));
        }
        {
            let store = lock(&self.store)?;
            store.live(&a.grant)?;
            store.live(&b.grant)?;
        }
        let effective = intersect_grants(
            &intersect_grants(&a.grant.grants, &b.grant.grants),
            &intersect_grants(a.session.grants(), b.session.grants()),
        );
        for admission in [&mut a, &mut b] {
            let e = PublicEnvelope {
                protocol: PROTOCOL.into(),
                kind: MessageKind::Response,
                message_id: "relay-accepted".into(),
                sequence: 1,
                reply_to: Some(admission.message_id.clone()),
                route_id: None,
                route_epoch: None,
                operation: Some("relay".into()),
                scope: Some(admission.grant.scope.clone()),
                trace: None,
                deadline_ms: None,
                payload: Some(
                    serde_json::json!({"pair_id":admission.grant.pair_id,"grants":effective}),
                ),
                error: None,
            };
            admission.session.send(&e).await?;
        }
        Ok(RelayPair {
            initiator: a,
            responder: b,
            effective,
            store: self.store.clone(),
            sequence: 2,
        })
    }
}
pub struct RelayPair {
    initiator: RelayAdmission,
    responder: RelayAdmission,
    effective: Grants,
    store: Arc<Mutex<FederationStore>>,
    sequence: u64,
}
impl RelayPair {
    pub fn grants(&self) -> &Grants {
        &self.effective
    }
    pub async fn forward_once(&mut self) -> Result<(), FederationError> {
        {
            let store = lock(&self.store)?;
            store.live(&self.initiator.grant)?;
            store.live(&self.responder.grant)?;
        }
        let mut request = self.initiator.session.receive().await?;
        let operation = request
            .operation
            .as_deref()
            .ok_or_else(|| refuse("relay operation"))?;
        if request.kind != MessageKind::Request
            || !self.effective.contains(operation)
            || (operation != "catalog"
                && !self
                    .responder
                    .grant
                    .catalog
                    .operations
                    .contains_key(operation))
            || request.deadline_ms.is_none_or(|v| v <= now())
        {
            return Err(refuse("relay operation unavailable or deadline"));
        }
        let correlation = request.message_id.clone();
        request.sequence = self.sequence;
        self.responder.session.send(&request).await?;
        let mut response = self.responder.session.receive().await?;
        if response.kind != MessageKind::Response
            || response.reply_to.as_deref() != Some(&correlation)
        {
            return Err(refuse("relay response correlation"));
        }
        {
            let store = lock(&self.store)?;
            store.live(&self.initiator.grant)?;
            store.live(&self.responder.grant)?;
        }
        response.sequence = self.sequence;
        self.initiator.session.send(&response).await?;
        self.sequence += 1;
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub struct CandidatePair {
    pub direct: Vec<Endpoint>,
    pub relay: Option<Endpoint>,
    pub effective_grants: Grants,
}
pub fn pair_candidates(
    local: &VerifiedCandidate,
    remote: &VerifiedCandidate,
) -> Result<CandidatePair, FederationError> {
    local.advertisement.validate()?;
    remote.advertisement.validate()?;
    if local.advertisement.scope != remote.advertisement.scope {
        return Err(refuse("candidate scope mismatch"));
    }
    Ok(CandidatePair {
        direct: remote.advertisement.direct.clone(),
        relay: remote.advertisement.relay.clone(),
        effective_grants: intersect_grants(
            &local.advertisement.grants,
            &remote.advertisement.grants,
        ),
    })
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionPath {
    Direct,
    Relay,
}
pub struct FederatedConnection {
    session: PublicTlsSession,
    candidate: VerifiedCandidate,
    effective: Grants,
    path: ConnectionPath,
    outgoing: u64,
    incoming: u64,
    catalog: Option<RemoteCatalog>,
}
impl FederatedConnection {
    pub fn path(&self) -> ConnectionPath {
        self.path
    }
    pub fn grants(&self) -> &Grants {
        &self.effective
    }
    pub fn scope(&self) -> &Scope {
        self.session.scope()
    }
    pub fn catalog(&self) -> Option<&RemoteCatalog> {
        self.catalog.as_ref()
    }
    fn frame(&self, operation: &str, payload: serde_json::Value) -> PublicEnvelope {
        PublicEnvelope {
            protocol: PROTOCOL.into(),
            kind: MessageKind::Request,
            message_id: format!("federation-{}", self.outgoing + 1),
            sequence: self.outgoing + 1,
            reply_to: None,
            route_id: Some(self.candidate.advertisement.peer_id.clone()),
            route_epoch: Some(self.candidate.advertisement.catalog.generation),
            operation: Some(operation.into()),
            scope: Some(self.candidate.advertisement.scope.clone()),
            trace: None,
            deadline_ms: Some(now() + 10_000),
            payload: Some(payload),
            error: None,
        }
    }
    async fn exchange(
        &mut self,
        operation: &str,
        payload: serde_json::Value,
    ) -> Result<PublicEnvelope, FederationError> {
        self.candidate.advertisement.validate()?;
        if !self.effective.contains(operation) {
            return Err(refuse("operation outside intersection"));
        }
        let e = self.frame(operation, payload);
        self.session.send(&e).await?;
        self.outgoing += 1;
        let response = self.session.receive().await?;
        if response.kind != MessageKind::Response
            || response.reply_to.as_deref() != Some(&e.message_id)
            || response.sequence != self.incoming + 1
            || response.error.is_some()
        {
            return Err(refuse("remote response correlation or failure"));
        }
        self.incoming = response.sequence;
        Ok(response)
    }
    pub async fn remote_catalog(&mut self) -> Result<&RemoteCatalog, FederationError> {
        let response = self.exchange("catalog", serde_json::Value::Null).await?;
        let catalog: RemoteCatalog = serde_json::from_value(
            response
                .payload
                .ok_or_else(|| refuse("missing remote catalog"))?,
        )
        .map_err(|_| refuse("remote catalog JSON"))?;
        catalog.validate()?;
        if catalog != self.candidate.advertisement.catalog {
            return Err(refuse("remote catalog changed since signed discovery"));
        }
        self.catalog = Some(catalog);
        Ok(self.catalog.as_ref().unwrap())
    }
    pub async fn call(
        &mut self,
        operation: &str,
        payload: serde_json::Value,
    ) -> Result<PublicEnvelope, FederationError> {
        if self
            .catalog
            .as_ref()
            .is_none_or(|c| !c.operations.contains_key(operation))
        {
            return Err(refuse("operation not remotely discoverable"));
        }
        self.exchange(operation, payload).await
    }
}
async fn tls_connect(
    endpoint: &Endpoint,
    config: MutualClientConfig,
    candidate: &VerifiedCandidate,
    allowed: &Grants,
) -> Result<PublicTlsSession, FederationError> {
    let stream = tokio::time::timeout(Duration::from_secs(1), TcpStream::connect(endpoint.address))
        .await
        .map_err(|_| refuse("connection timeout"))??;
    let name =
        ServerName::try_from(endpoint.server_name.clone()).map_err(|_| refuse("server name"))?;
    let session = PublicTlsSession::connect(
        stream,
        config,
        name,
        candidate.advertisement.scope.clone(),
        None,
        allowed.clone(),
    )
    .await?;
    if session.identity().certificate_sha256() != endpoint.certificate_sha256 {
        return Err(refuse("signed endpoint certificate pin"));
    }
    Ok(session)
}
pub async fn connect_direct_then_relay(
    candidate: &VerifiedCandidate,
    config: MutualClientConfig,
    local_grants: Grants,
    relay_grant: Option<SignedRelayGrant>,
) -> Result<FederatedConnection, FederationError> {
    candidate.advertisement.validate()?;
    if !grants(&local_grants) {
        return Err(refuse("local grant bounds"));
    }
    for endpoint in &candidate.advertisement.direct {
        let stream = match tokio::time::timeout(
            Duration::from_millis(500),
            TcpStream::connect(endpoint.address),
        )
        .await
        {
            Ok(Ok(stream)) => stream,
            _ => continue,
        };
        let name = ServerName::try_from(endpoint.server_name.clone())
            .map_err(|_| refuse("server name"))?;
        let session = PublicTlsSession::connect(
            stream,
            config.clone(),
            name,
            candidate.advertisement.scope.clone(),
            None,
            local_grants.clone(),
        )
        .await?;
        if session.identity().certificate_sha256() != endpoint.certificate_sha256 {
            return Err(refuse("signed direct certificate pin"));
        }
        let effective = intersect_grants(
            &intersect_grants(&local_grants, &candidate.advertisement.grants),
            session.grants(),
        );
        let mut connection = FederatedConnection {
            session,
            candidate: candidate.clone(),
            effective,
            path: ConnectionPath::Direct,
            outgoing: 0,
            incoming: 0,
            catalog: None,
        };
        connection.remote_catalog().await?;
        return Ok(connection);
    }
    let endpoint = candidate
        .advertisement
        .relay
        .as_ref()
        .ok_or_else(|| refuse("no reachable direct or authorized relay"))?;
    let signed = relay_grant.ok_or_else(|| refuse("relay grant required"))?;
    signed.grant.validate()?;
    if signed.grant.side != RelaySide::Initiator
        || signed.grant.scope != candidate.advertisement.scope
        || signed.grant.catalog != candidate.advertisement.catalog
        || candidate
            .advertisement
            .relay_peer_certificate_sha256
            .as_deref()
            != Some(signed.grant.remote_certificate_sha256.as_str())
    {
        return Err(refuse("relay grant destination"));
    }
    let mut session = tls_connect(endpoint, config, candidate, &local_grants).await?;
    let admission = PublicEnvelope {
        protocol: PROTOCOL.into(),
        kind: MessageKind::Request,
        message_id: "relay-admission".into(),
        sequence: 1,
        reply_to: None,
        route_id: Some(signed.grant.pair_id.clone()),
        route_epoch: Some(1),
        operation: Some("relay".into()),
        scope: Some(signed.grant.scope.clone()),
        trace: None,
        deadline_ms: Some(now() + 10_000),
        payload: Some(serde_json::to_value(&signed).unwrap()),
        error: None,
    };
    session.send(&admission).await?;
    let ack = session.receive().await?;
    if ack.kind != MessageKind::Response || ack.reply_to.as_deref() != Some("relay-admission") {
        return Err(refuse("relay admission response"));
    }
    let relay_effective: Grants = serde_json::from_value(
        ack.payload
            .as_ref()
            .and_then(|v| v.get("grants"))
            .cloned()
            .ok_or_else(|| refuse("relay grants missing"))?,
    )
    .map_err(|_| refuse("relay grants JSON"))?;
    if !relay_effective.is_subset(&signed.grant.grants) {
        return Err(refuse("relay grant widening"));
    }
    let effective = intersect_grants(
        &intersect_grants(&local_grants, &candidate.advertisement.grants),
        &intersect_grants(session.grants(), &relay_effective),
    );
    let mut connection = FederatedConnection {
        session,
        candidate: candidate.clone(),
        effective,
        path: ConnectionPath::Relay,
        outgoing: 1,
        incoming: 1,
        catalog: None,
    };
    connection.remote_catalog().await?;
    Ok(connection)
}
pub async fn serve_catalog(
    session: &mut PublicTlsSession,
    catalog: &RemoteCatalog,
) -> Result<(), FederationError> {
    catalog.validate()?;
    let request = session.receive().await?;
    if request.kind != MessageKind::Request || request.operation.as_deref() != Some("catalog") {
        return Err(refuse("expected remote catalog request"));
    }
    let response = PublicEnvelope {
        protocol: PROTOCOL.into(),
        kind: MessageKind::Response,
        message_id: format!("catalog-{}", request.sequence),
        sequence: request.sequence,
        reply_to: Some(request.message_id),
        route_id: request.route_id,
        route_epoch: Some(catalog.generation),
        operation: Some("catalog".into()),
        scope: Some(session.scope().clone()),
        trace: request.trace,
        deadline_ms: None,
        payload: Some(serde_json::to_value(catalog).unwrap()),
        error: None,
    };
    session.send(&response).await?;
    Ok(())
}
