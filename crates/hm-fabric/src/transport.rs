use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use hm_context::{types::validate_id, Scope};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    io::Read,
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};

pub const VERSION: u32 = 1;
#[derive(Clone, Debug)]
pub struct Limits {
    pub max_frame_bytes: usize,
    pub max_chunk_bytes: usize,
    pub max_chunks: u32,
    pub io_timeout: Duration,
    pub max_nonces: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_frame_bytes: 1024 * 1024,
            max_chunk_bytes: 64 * 1024,
            max_chunks: 1024,
            io_timeout: Duration::from_secs(10),
            max_nonces: 65536,
        }
    }
}
#[derive(Clone)]
pub struct Credentials {
    pub scope: Scope,
    pub signing_key: SigningKey,
    pub peer_key: VerifyingKey,
}
#[derive(Clone, Debug)]
pub struct AuthenticatedIdentity {
    scope: Scope,
    peer_key: [u8; 32],
    session_id: [u8; 32],
}
impl AuthenticatedIdentity {
    pub fn scope(&self) -> &Scope {
        &self.scope
    }
    pub fn peer_key(&self) -> &[u8; 32] {
        &self.peer_key
    }
    pub fn session_id(&self) -> &[u8; 32] {
        &self.session_id
    }
}
#[derive(Clone, Default)]
pub struct ReplayGuard(Arc<Mutex<HashSet<([u8; 32], [u8; 32])>>>);
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Effect {
    NotApplied,
    Applied,
    Unknown,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum FailureCode {
    Authentication,
    Protocol,
    Capacity,
    Deadline,
    Cancelled,
    Unavailable,
    Unsupported,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, thiserror::Error)]
#[error("{code:?} ({effect:?}): {message}")]
pub struct TypedFailure {
    pub code: FailureCode,
    pub effect: Effect,
    pub message: String,
}
fn fail(code: FailureCode, effect: Effect, message: impl Into<String>) -> TypedFailure {
    TypedFailure {
        code,
        effect,
        message: message.into(),
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum FrameKind {
    Request,
    Response,
    Cancel,
    Chunk,
    Failure,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChunkMeta {
    pub index: u32,
    pub total: u32,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    pub correlation_id: String,
    pub deadline_unix_ms: Option<u64>,
    pub kind: FrameKind,
    pub payload: Vec<u8>,
    pub chunk: Option<ChunkMeta>,
    pub failure: Option<TypedFailure>,
}
impl Frame {
    pub fn request(id: impl Into<String>, payload: Vec<u8>) -> Self {
        Self {
            correlation_id: id.into(),
            deadline_unix_ms: None,
            kind: FrameKind::Request,
            payload,
            chunk: None,
            failure: None,
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Hello {
    version: u32,
    scope: Scope,
    key: [u8; 32],
    nonce: [u8; 32],
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Signed {
    body: Vec<u8>,
    signature: Vec<u8>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    version: u32,
    session: [u8; 32],
    scope: Scope,
    sequence: u64,
    frame: Frame,
}
pub struct UnixTransport {
    stream: UnixStream,
    credentials: Credentials,
    identity: AuthenticatedIdentity,
    limits: Limits,
    sent: u64,
    received: u64,
    poisoned: bool,
}
impl UnixTransport {
    pub fn identity(&self) -> &AuthenticatedIdentity {
        &self.identity
    }
    pub fn remote() -> Result<Self, TypedFailure> {
        Err(fail(
            FailureCode::Unsupported,
            Effect::NotApplied,
            "remote transport requires authenticated TLS",
        ))
    }
    pub async fn connect(
        path: impl AsRef<Path>,
        credentials: Credentials,
        replay: ReplayGuard,
        limits: Limits,
    ) -> Result<Self, TypedFailure> {
        let stream = tokio::time::timeout(limits.io_timeout, UnixStream::connect(path))
            .await
            .map_err(|_| fail(FailureCode::Deadline, Effect::NotApplied, "connect timeout"))?
            .map_err(|e| fail(FailureCode::Unavailable, Effect::NotApplied, e.to_string()))?;
        Self::accept(stream, credentials, replay, limits).await
    }
    pub async fn accept(
        mut stream: UnixStream,
        credentials: Credentials,
        replay: ReplayGuard,
        limits: Limits,
    ) -> Result<Self, TypedFailure> {
        credentials
            .scope
            .validate()
            .map_err(|e| fail(FailureCode::Protocol, Effect::NotApplied, e.to_string()))?;
        if limits.max_frame_bytes < 1024
            || limits.max_frame_bytes > 16 * 1024 * 1024
            || limits.max_chunk_bytes == 0
            || limits.max_chunk_bytes > limits.max_frame_bytes
            || limits.max_chunks == 0
            || limits.max_nonces == 0
        {
            return Err(fail(
                FailureCode::Capacity,
                Effect::NotApplied,
                "invalid limits",
            ));
        }
        let mut nonce = [0u8; 32];
        std::fs::File::open("/dev/urandom")
            .and_then(|mut f| f.read_exact(&mut nonce))
            .map_err(|e| fail(FailureCode::Unavailable, Effect::NotApplied, e.to_string()))?;
        let hello = Hello {
            version: VERSION,
            scope: credentials.scope.clone(),
            key: credentials.signing_key.verifying_key().to_bytes(),
            nonce,
        };
        let own = encode(&hello)?;
        write_packet(&mut stream, &own, &limits).await?;
        let peer: Hello = decode(&read_packet(&mut stream, &limits).await?)?;
        if peer.version != VERSION
            || peer.scope != credentials.scope
            || peer.key != credentials.peer_key.to_bytes()
            || peer.nonce == nonce
        {
            return Err(fail(
                FailureCode::Authentication,
                Effect::NotApplied,
                "peer identity or scope mismatch",
            ));
        }
        let other = encode(&peer)?;
        let proof = transcript(&own, &other);
        let signature = credentials.signing_key.sign(&proof).to_bytes().to_vec();
        write_packet(&mut stream, &signature, &limits).await?;
        let peer_signature = read_packet(&mut stream, &limits).await?;
        verify(
            &credentials.peer_key,
            &transcript(&other, &own),
            &peer_signature,
        )?;
        {
            let mut guard = replay.0.lock().map_err(|_| {
                fail(
                    FailureCode::Unavailable,
                    Effect::NotApplied,
                    "replay registry unavailable",
                )
            })?;
            if guard.contains(&(peer.key, peer.nonce)) {
                return Err(fail(
                    FailureCode::Authentication,
                    Effect::NotApplied,
                    "replayed nonce",
                ));
            }
            if guard.len() >= limits.max_nonces {
                return Err(fail(
                    FailureCode::Capacity,
                    Effect::NotApplied,
                    "nonce registry full",
                ));
            }
            guard.insert((peer.key, peer.nonce));
        }
        let session_id: [u8; 32] = if own < other {
            Sha256::digest(transcript(&own, &other)).into()
        } else {
            Sha256::digest(transcript(&other, &own)).into()
        };
        let identity = AuthenticatedIdentity {
            scope: credentials.scope.clone(),
            peer_key: peer.key,
            session_id,
        };
        Ok(Self {
            stream,
            credentials,
            identity,
            limits,
            sent: 0,
            received: 0,
            poisoned: false,
        })
    }
    pub async fn send(&mut self, frame: Frame) -> Result<(), TypedFailure> {
        self.ready()?;
        validate(&frame, &self.limits)?;
        let sequence = self.sent.checked_add(1).ok_or_else(|| {
            fail(
                FailureCode::Capacity,
                Effect::NotApplied,
                "sequence exhausted",
            )
        })?;
        let body = encode(&Envelope {
            version: VERSION,
            session: self.identity.session_id,
            scope: self.identity.scope.clone(),
            sequence,
            frame,
        })?;
        let packet = encode(&Signed {
            signature: self.credentials.signing_key.sign(&body).to_bytes().to_vec(),
            body,
        })?;
        if packet.len() > self.limits.max_frame_bytes {
            return Err(fail(
                FailureCode::Capacity,
                Effect::NotApplied,
                "frame too large",
            ));
        }
        match write_packet(&mut self.stream, &packet, &self.limits).await {
            Ok(()) => {
                self.sent = sequence;
                Ok(())
            }
            Err(mut e) => {
                self.poisoned = true;
                e.effect = Effect::Unknown;
                Err(e)
            }
        }
    }
    pub async fn receive(&mut self) -> Result<Frame, TypedFailure> {
        self.ready()?;
        let result = self.receive_inner().await;
        if result.is_err() {
            self.poisoned = true;
        }
        result
    }
    async fn receive_inner(&mut self) -> Result<Frame, TypedFailure> {
        let signed: Signed = decode(&read_packet(&mut self.stream, &self.limits).await?)?;
        verify(&self.credentials.peer_key, &signed.body, &signed.signature)?;
        let envelope: Envelope = decode(&signed.body)?;
        if envelope.version != VERSION
            || envelope.session != self.identity.session_id
            || envelope.scope != self.identity.scope
            || self.received.checked_add(1) != Some(envelope.sequence)
        {
            return Err(fail(
                FailureCode::Protocol,
                Effect::NotApplied,
                "invalid session or sequence",
            ));
        }
        validate(&envelope.frame, &self.limits)?;
        self.received = envelope.sequence;
        Ok(envelope.frame)
    }
    fn ready(&self) -> Result<(), TypedFailure> {
        if self.poisoned {
            Err(fail(
                FailureCode::Unavailable,
                Effect::Unknown,
                "session closed after transport failure",
            ))
        } else {
            Ok(())
        }
    }
}
fn transcript(a: &[u8], b: &[u8]) -> Vec<u8> {
    let mut out = b"HyperMind transport v1\0".to_vec();
    out.extend_from_slice(&(a.len() as u64).to_be_bytes());
    out.extend_from_slice(a);
    out.extend_from_slice(&(b.len() as u64).to_be_bytes());
    out.extend_from_slice(b);
    out
}
fn verify(key: &VerifyingKey, body: &[u8], sig: &[u8]) -> Result<(), TypedFailure> {
    let sig = Signature::from_slice(sig).map_err(|_| {
        fail(
            FailureCode::Authentication,
            Effect::NotApplied,
            "invalid signature length",
        )
    })?;
    key.verify_strict(body, &sig).map_err(|_| {
        fail(
            FailureCode::Authentication,
            Effect::NotApplied,
            "invalid signature",
        )
    })
}
fn encode<T: Serialize>(v: &T) -> Result<Vec<u8>, TypedFailure> {
    serde_json::to_vec(v)
        .map_err(|e| fail(FailureCode::Protocol, Effect::NotApplied, e.to_string()))
}
fn decode<T: serde::de::DeserializeOwned>(v: &[u8]) -> Result<T, TypedFailure> {
    serde_json::from_slice(v)
        .map_err(|e| fail(FailureCode::Protocol, Effect::NotApplied, e.to_string()))
}
fn validate(frame: &Frame, limits: &Limits) -> Result<(), TypedFailure> {
    validate_id(&frame.correlation_id)
        .map_err(|e| fail(FailureCode::Protocol, Effect::NotApplied, e.to_string()))?;
    if let Some(deadline) = frame.deadline_unix_ms {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| {
                fail(
                    FailureCode::Unavailable,
                    Effect::NotApplied,
                    "clock unavailable",
                )
            })?
            .as_millis();
        if now >= u128::from(deadline) {
            return Err(fail(
                FailureCode::Deadline,
                Effect::NotApplied,
                "request deadline elapsed",
            ));
        }
    }
    if frame.payload.len() > limits.max_frame_bytes {
        return Err(fail(
            FailureCode::Capacity,
            Effect::NotApplied,
            "payload too large",
        ));
    }
    match (&frame.kind, &frame.chunk, &frame.failure) {
        (FrameKind::Chunk, Some(chunk), None)
            if chunk.total > 0
                && chunk.total <= limits.max_chunks
                && chunk.index < chunk.total
                && frame.payload.len() <= limits.max_chunk_bytes => {}
        (FrameKind::Failure, None, Some(failure)) if failure.message.len() <= 4096 => {}
        (FrameKind::Request | FrameKind::Response, None, None) => {}
        (FrameKind::Cancel, None, None) if frame.payload.is_empty() => {}
        _ => {
            return Err(fail(
                FailureCode::Protocol,
                Effect::NotApplied,
                "invalid frame metadata",
            ))
        }
    }
    Ok(())
}
async fn write_packet(
    stream: &mut UnixStream,
    bytes: &[u8],
    limits: &Limits,
) -> Result<(), TypedFailure> {
    if bytes.len() > limits.max_frame_bytes {
        return Err(fail(
            FailureCode::Capacity,
            Effect::NotApplied,
            "frame too large",
        ));
    }
    tokio::time::timeout(limits.io_timeout, async {
        stream
            .write_all(&(bytes.len() as u32).to_be_bytes())
            .await?;
        stream.write_all(bytes).await
    })
    .await
    .map_err(|_| fail(FailureCode::Deadline, Effect::Unknown, "write timeout"))?
    .map_err(|e| fail(FailureCode::Unavailable, Effect::Unknown, e.to_string()))
}
async fn read_packet(stream: &mut UnixStream, limits: &Limits) -> Result<Vec<u8>, TypedFailure> {
    tokio::time::timeout(limits.io_timeout, async {
        let length = stream
            .read_u32()
            .await
            .map_err(|e| fail(FailureCode::Unavailable, Effect::Unknown, e.to_string()))?
            as usize;
        if length == 0 || length > limits.max_frame_bytes {
            return Err(fail(
                FailureCode::Capacity,
                Effect::NotApplied,
                "frame length out of bounds",
            ));
        }
        let mut bytes = vec![0; length];
        stream
            .read_exact(&mut bytes)
            .await
            .map_err(|e| fail(FailureCode::Unavailable, Effect::Unknown, e.to_string()))?;
        Ok(bytes)
    })
    .await
    .map_err(|_| fail(FailureCode::Deadline, Effect::Unknown, "read timeout"))?
}
