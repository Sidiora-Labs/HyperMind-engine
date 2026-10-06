use crate::transport::{
    AuthenticatedIdentity, ChunkMeta, Effect, FailureCode, Frame, FrameKind, TypedFailure,
    UnixTransport,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashSet;

pub const PROTOCOL: &str = "hypermid.v1";
pub const MAX_FRAME_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_CHUNK_BYTES: usize = 64 * 1024;
const ADAPTER: &str = "hypermind.hypermid-adapter.v1";
const SAFE: u64 = 9_007_199_254_740_991;
fn refusal(message: impl Into<String>) -> TypedFailure {
    TypedFailure {
        code: FailureCode::Protocol,
        effect: Effect::NotApplied,
        message: message.into(),
    }
}
fn id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 160
        && s.as_bytes()[0].is_ascii_alphanumeric()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MessageKind {
    Request,
    Response,
    Event,
    Credit,
    Cancel,
    Ping,
    Pong,
    Close,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicEnvelope {
    pub protocol: String,
    pub kind: MessageKind,
    pub message_id: String,
    pub sequence: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_epoch: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<hm_context::Scope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<Value>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PublicTrace {
    trace_id: String,
    request_id: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum PublicEffect {
    NotStarted,
    Committed,
    Unknown,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PublicError {
    code: String,
    message: String,
    retryable: bool,
    #[serde(default)]
    retry_after_ms: Option<u64>,
    #[serde(default)]
    effect_state: Option<PublicEffect>,
}
impl PublicEnvelope {
    fn validate(&self) -> Result<(), TypedFailure> {
        if self.protocol != PROTOCOL
            || !id(&self.message_id)
            || self.sequence == 0
            || self.sequence > SAFE
            || self.reply_to.as_deref().is_some_and(|s| !id(s))
            || self.route_id.as_deref().is_some_and(|s| !id(s))
        {
            return Err(refusal("invalid public envelope"));
        }
        if let Some(s) = &self.scope {
            if !id(&s.owner_id)
                || !id(&s.project_id)
                || s.workspace_id.as_deref().is_some_and(|v| !id(v))
            {
                return Err(refusal("invalid scope"));
            }
        }
        if let Some(v) = &self.trace {
            let t: PublicTrace =
                serde_json::from_value(v.clone()).map_err(|_| refusal("invalid trace"))?;
            if !id(&t.trace_id) || !id(&t.request_id) {
                return Err(refusal("invalid trace IDs"));
            }
        }
        if let Some(v) = &self.error {
            let e: PublicError =
                serde_json::from_value(v.clone()).map_err(|_| refusal("invalid error"))?;
            if !(2..=64).contains(&e.code.len())
                || !e.code.as_bytes()[0].is_ascii_uppercase()
                || !e
                    .code
                    .bytes()
                    .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
                || e.message.chars().count() > 2048
                || e.retry_after_ms.is_some_and(|n| n > 86_400_000)
            {
                return Err(refusal("invalid error fields"));
            }
        }
        let valid = match self.kind {
            MessageKind::Request => {
                self.route_id.is_some()
                    && self.route_epoch.is_some()
                    && self.operation.as_ref().is_some_and(|s| !s.is_empty())
                    && self.deadline_ms.is_some()
            }
            MessageKind::Response | MessageKind::Cancel => self.reply_to.is_some(),
            MessageKind::Credit => {
                self.route_id.is_some() && self.route_epoch.is_some() && self.payload.is_some()
            }
            _ => true,
        };
        if !valid {
            return Err(refusal("missing required public fields"));
        }
        Ok(())
    }
}
struct Unique;
impl<'de> serde::de::Visitor<'de> for Unique {
    type Value = Value;
    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "bounded JSON")
    }
    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut a: A) -> Result<Value, A::Error> {
        let mut seen = HashSet::new();
        let mut out = serde_json::Map::new();
        while let Some(k) = a.next_key::<String>()? {
            if !seen.insert(k.clone()) {
                return Err(serde::de::Error::custom("duplicate field"));
            }
            out.insert(k, a.next_value::<Checked>()?.0);
        }
        Ok(Value::Object(out))
    }
    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut a: A) -> Result<Value, A::Error> {
        let mut out = Vec::new();
        while let Some(v) = a.next_element::<Checked>()? {
            out.push(v.0);
        }
        Ok(Value::Array(out))
    }
    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Value, E> {
        Ok(Value::String(v.into()))
    }
    fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Value, E> {
        Ok(Value::Bool(v))
    }
    fn visit_unit<E: serde::de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Value, E> {
        if v > SAFE {
            return Err(E::custom("unsafe integer"));
        }
        Ok(v.into())
    }
    fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Value, E> {
        if v.unsigned_abs() > SAFE {
            return Err(E::custom("unsafe integer"));
        }
        Ok(v.into())
    }
    fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Value, E> {
        if v.fract() == 0.0 && v.abs() > SAFE as f64 {
            return Err(E::custom("unsafe integer"));
        }
        serde_json::Number::from_f64(v)
            .map(Value::Number)
            .ok_or_else(|| E::custom("invalid number"))
    }
}
struct Checked(Value);
impl<'de> Deserialize<'de> for Checked {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(Unique).map(Self)
    }
}
struct Bounded(Vec<u8>);
impl std::io::Write for Bounded {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        if self.0.len() + b.len() > MAX_FRAME_BYTES {
            return Err(std::io::Error::other("frame capacity"));
        }
        self.0.extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
pub fn encode(envelope: &PublicEnvelope) -> Result<Vec<u8>, TypedFailure> {
    envelope.validate()?;
    let mut b = Bounded(Vec::new());
    serde_json::to_writer(&mut b, envelope).map_err(|e| refusal(e.to_string()))?;
    let mut framed = (b.0.len() as u32).to_be_bytes().to_vec();
    framed.extend(b.0);
    decode(&framed)?;
    Ok(framed)
}
pub fn decode(bytes: &[u8]) -> Result<PublicEnvelope, TypedFailure> {
    if bytes.len() < 4 {
        return Err(refusal("truncated length"));
    }
    let n = u32::from_be_bytes(bytes[..4].try_into().unwrap()) as usize;
    if n == 0 || n > MAX_FRAME_BYTES || bytes.len() != n + 4 {
        return Err(refusal("invalid frame length"));
    }
    let v: Checked = serde_json::from_slice(&bytes[4..]).map_err(|e| refusal(e.to_string()))?;
    let e: PublicEnvelope = serde_json::from_value(v.0).map_err(|e| refusal(e.to_string()))?;
    e.validate()?;
    Ok(e)
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Offer {
    adapter: String,
    protocol: String,
    frame_bytes: usize,
    chunk_bytes: usize,
}
pub struct PublicFramingAdapter {
    transport: UnixTransport,
    outbound: u64,
    inbound: u64,
    poisoned: bool,
}
impl PublicFramingAdapter {
    pub async fn negotiate(mut transport: UnixTransport) -> Result<Self, TypedFailure> {
        let offer = Offer {
            adapter: ADAPTER.into(),
            protocol: PROTOCOL.into(),
            frame_bytes: MAX_FRAME_BYTES,
            chunk_bytes: MAX_CHUNK_BYTES,
        };
        transport
            .send(Frame::request(
                "compat-negotiation",
                serde_json::to_vec(&offer).unwrap(),
            ))
            .await?;
        let frame = transport.receive().await?;
        let peer: Offer =
            serde_json::from_slice(&frame.payload).map_err(|_| refusal("invalid negotiation"))?;
        if frame.kind != FrameKind::Request
            || frame.correlation_id != "compat-negotiation"
            || peer.adapter != ADAPTER
            || peer.protocol != PROTOCOL
            || peer.frame_bytes != MAX_FRAME_BYTES
            || peer.chunk_bytes != MAX_CHUNK_BYTES
        {
            return Err(refusal("incompatible adapter; no downgrade"));
        }
        Ok(Self {
            transport,
            outbound: 0,
            inbound: 0,
            poisoned: false,
        })
    }
    pub fn identity(&self) -> &AuthenticatedIdentity {
        self.transport.identity()
    }
    pub async fn send(&mut self, envelope: &PublicEnvelope) -> Result<(), TypedFailure> {
        if self.poisoned
            || envelope.sequence != self.outbound + 1
            || envelope
                .scope
                .as_ref()
                .is_some_and(|s| s != self.identity().scope())
        {
            return Err(refusal("session sequence or scope"));
        }
        let bytes = encode(envelope)?;
        let total = bytes.len().div_ceil(MAX_CHUNK_BYTES) as u32;
        for (index, payload) in bytes.chunks(MAX_CHUNK_BYTES).enumerate() {
            let mut f = Frame::request(&envelope.message_id, payload.to_vec());
            f.kind = FrameKind::Chunk;
            f.chunk = Some(ChunkMeta {
                index: index as u32,
                total,
            });
            if let Err(e) = self.transport.send(f).await {
                self.poisoned = true;
                return Err(e);
            }
        }
        self.outbound = envelope.sequence;
        Ok(())
    }
    pub async fn receive(&mut self) -> Result<PublicEnvelope, TypedFailure> {
        if self.poisoned {
            return Err(refusal("closed adapter"));
        }
        self.poisoned = true;
        let mut bytes = Vec::new();
        let mut correlation = None;
        let mut count = 0;
        for index in 0..=128 {
            let f = self.transport.receive().await?;
            let c = f.chunk.ok_or_else(|| refusal("missing chunk metadata"))?;
            if f.kind != FrameKind::Chunk
                || c.index != index
                || c.total == 0
                || c.total > 129
                || f.payload.is_empty()
                || f.payload.len() > MAX_CHUNK_BYTES
            {
                return Err(refusal("invalid tunnel chunk"));
            }
            if index == 0 {
                correlation = Some(f.correlation_id.clone());
                count = c.total;
            }
            if c.total != count
                || Some(&f.correlation_id) != correlation.as_ref()
                || bytes.len() + f.payload.len() > MAX_FRAME_BYTES + 4
            {
                return Err(refusal("chunk order or capacity"));
            }
            bytes.extend(f.payload);
            if index + 1 == count {
                let e = decode(&bytes)?;
                if e.sequence != self.inbound + 1
                    || Some(&e.message_id) != correlation.as_ref()
                    || e.scope
                        .as_ref()
                        .is_some_and(|s| s != self.identity().scope())
                {
                    return Err(refusal("envelope replay or scope"));
                }
                self.inbound = e.sequence;
                self.poisoned = false;
                return Ok(e);
            }
        }
        Err(refusal("incomplete chunks"))
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventChunk {
    pub event_id: String,
    pub chunk_index: u32,
    pub chunk_count: u32,
    pub digest: String,
    pub data: String,
}
#[derive(Default)]
pub struct EventAssembler {
    completed: HashSet<String>,
    pending: Option<(String, String, u32, u32, Vec<u8>)>,
}
impl EventAssembler {
    pub fn push(&mut self, c: EventChunk) -> Result<Option<Vec<u8>>, TypedFailure> {
        let result = self.append(c);
        if result.is_err() {
            self.pending = None;
        }
        result
    }
    fn append(&mut self, c: EventChunk) -> Result<Option<Vec<u8>>, TypedFailure> {
        if self.completed.contains(&c.event_id)
            || self.completed.len() >= 4096
            || !id(&c.event_id)
            || c.chunk_count == 0
            || c.chunk_count > 128
            || c.digest.len() != 64
            || !c
                .digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || c.data.len() > MAX_CHUNK_BYTES.div_ceil(3) * 4
        {
            return Err(refusal("invalid event chunk"));
        }
        let data = STANDARD
            .decode(&c.data)
            .map_err(|_| refusal("invalid base64"))?;
        if data.is_empty() || data.len() > MAX_CHUNK_BYTES || STANDARD.encode(&data) != c.data {
            return Err(refusal("noncanonical chunk"));
        }
        if self.pending.is_none() {
            if c.chunk_index != 0 {
                return Err(refusal("chunk must start at zero"));
            }
            self.pending = Some((
                c.event_id.clone(),
                c.digest.clone(),
                c.chunk_count,
                0,
                Vec::new(),
            ));
        }
        let (id, digest, total, next, bytes) = self.pending.as_mut().unwrap();
        if *id != c.event_id
            || *digest != c.digest
            || *total != c.chunk_count
            || *next != c.chunk_index
            || bytes.len() + data.len() > MAX_FRAME_BYTES
        {
            return Err(refusal("event order or capacity"));
        }
        bytes.extend(data);
        *next += 1;
        if *next == *total {
            let (event_id, digest, _, _, bytes) = self.pending.take().unwrap();
            if format!("{:x}", Sha256::digest(&bytes)) != digest {
                return Err(refusal("event digest mismatch"));
            }
            self.completed.insert(event_id);
            return Ok(Some(bytes));
        }
        Ok(None)
    }
}
