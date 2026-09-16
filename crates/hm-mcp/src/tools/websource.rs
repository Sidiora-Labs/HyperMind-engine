use crate::tools::remember::{EmbeddingRuntime, RememberInput, RetentionInput, SensitivityInput};
use crate::tools::webtext;
use crate::{DEFAULT_CHUNK_BYTES, Envelope};
use hm_core::{ConversationId, Error, ErrorCode, LSN};
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    Authority, EventEnvelope, EventPayload, MediaRef, ProviderFrame, Retention, Sensitivity,
    UserMsg,
};
use hm_serve::actor::{ActorEngine, IncomingEvent};
use serde_json::json;
use std::collections::{HashMap, VecDeque};
use std::io::Read as _;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs as _};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};
use url::{Host, Url};

pub const MAXIMUM_SOURCE_BYTES: usize = 8 * 1024 * 1024;
pub const WEB_SOURCE_PROVIDER_LABEL: &str = "hm-web-source@1";

const DEFAULT_MAXIMUM_BYTES: u64 = 2 * 1024 * 1024;
const DEFAULT_MAXIMUM_REDIRECTS: u64 = 3;
const MAXIMUM_REDIRECT_BUDGET: u64 = 8;
const DEFAULT_MINIMUM_INTERVAL_MS: u64 = 1000;
const DEFAULT_TIMEOUT_MS: u64 = 15_000;
const DEFAULT_MEDIA_TYPE: &str = "application/octet-stream";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CrawlPolicy {
    pub allowed_hosts: Vec<String>,
    pub maximum_bytes: usize,
    pub maximum_redirects: u8,
    pub minimum_interval_ms: u64,
    pub request_timeout_ms: u64,
    pub allow_cross_host_redirect: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FetchResponse {
    pub status: u16,
    pub content_type: Option<String>,
    pub content_length: Option<u64>,
    pub location: Option<String>,
    pub body: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FetchedSource {
    pub requested_url: String,
    pub final_url: String,
    pub media_type: String,
    pub status: u16,
    pub redirects: Vec<String>,
    pub bytes: Vec<u8>,
    pub digest: [u8; 32],
}

pub trait FetchTransport: Send + Sync {
    fn fetch(
        &self,
        url: &str,
        timeout_ms: u64,
        maximum_bytes: usize,
    ) -> Result<FetchResponse, Error>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct HttpFetchTransport;

impl FetchTransport for HttpFetchTransport {
    fn fetch(
        &self,
        url: &str,
        timeout_ms: u64,
        maximum_bytes: usize,
    ) -> Result<FetchResponse, Error> {
        let target = Url::parse(url).map_err(|_| Error::new(ErrorCode::InvalidArgument))?;
        let host = target
            .host_str()
            .ok_or_else(|| Error::new(ErrorCode::CapabilityDenied))?;
        let host = host.trim_start_matches('[').trim_end_matches(']');
        let port = target
            .port_or_known_default()
            .ok_or_else(|| Error::new(ErrorCode::CapabilityDenied))?;
        resolved_addresses_are_public(host, port)?;
        let client = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .map_err(|_| Error::new(ErrorCode::BackendUnavailable))?;
        let response = client
            .get(target)
            .send()
            .map_err(|_| Error::new(ErrorCode::BackendUnavailable))?;
        let status = response.status().as_u16();
        let content_type = header_value(response.headers(), reqwest::header::CONTENT_TYPE);
        let location = header_value(response.headers(), reqwest::header::LOCATION);
        let content_length = response.content_length();
        let ceiling = u64::try_from(maximum_bytes)
            .unwrap_or(u64::MAX)
            .saturating_add(1);
        let mut body = Vec::new();
        response
            .take(ceiling)
            .read_to_end(&mut body)
            .map_err(|_| Error::new(ErrorCode::BackendUnavailable))?;
        Ok(FetchResponse {
            status,
            content_type,
            content_length,
            location,
            body,
        })
    }
}

fn header_value(
    headers: &reqwest::header::HeaderMap,
    name: reqwest::header::HeaderName,
) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
}

pub struct RecordedFetchTransport {
    responses: Mutex<VecDeque<(String, FetchResponse)>>,
}

impl RecordedFetchTransport {
    #[must_use]
    pub fn new(responses: Vec<(String, FetchResponse)>) -> Self {
        Self {
            responses: Mutex::new(responses.into_iter().collect()),
        }
    }

    #[must_use]
    pub fn remaining(&self) -> usize {
        self.responses
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }
}

impl FetchTransport for RecordedFetchTransport {
    fn fetch(
        &self,
        url: &str,
        _timeout_ms: u64,
        _maximum_bytes: usize,
    ) -> Result<FetchResponse, Error> {
        let mut responses = self
            .responses
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let (expected, response) = responses
            .pop_front()
            .ok_or_else(|| Error::new(ErrorCode::BackendUnavailable))?;
        if expected != url {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        Ok(response)
    }
}

struct WebSourceState {
    policy: CrawlPolicy,
    transport: Arc<dyn FetchTransport>,
    last_request: Mutex<HashMap<String, Instant>>,
}

#[derive(Clone)]
pub struct WebSourceRuntime {
    state: Arc<WebSourceState>,
}

impl WebSourceRuntime {
    #[must_use]
    pub fn new(policy: CrawlPolicy, transport: Arc<dyn FetchTransport>) -> Self {
        let mut policy = policy;
        policy.maximum_bytes = policy.maximum_bytes.min(MAXIMUM_SOURCE_BYTES);
        policy.maximum_redirects = policy
            .maximum_redirects
            .min(u8::try_from(MAXIMUM_REDIRECT_BUDGET).unwrap_or(u8::MAX));
        Self {
            state: Arc::new(WebSourceState {
                policy,
                transport,
                last_request: Mutex::new(HashMap::new()),
            }),
        }
    }

    pub fn from_env() -> Result<Option<Self>, Error> {
        let Some(policy) = policy_from_env()? else {
            return Ok(None);
        };
        Ok(Some(Self::new(policy, Arc::new(HttpFetchTransport))))
    }

    #[must_use]
    pub fn policy(&self) -> &CrawlPolicy {
        &self.state.policy
    }

    pub fn fetch(&self, url: &str) -> Result<FetchedSource, Error> {
        let policy = &self.state.policy;
        let requested = validate_target(policy, url)?;
        let mut current = requested.clone();
        let mut redirects: Vec<String> = Vec::new();
        let mut hops: u8 = 0;
        loop {
            self.await_interval(&current);
            let response = self.state.transport.fetch(
                current.as_str(),
                policy.request_timeout_ms,
                policy.maximum_bytes,
            )?;
            let ceiling = u64::try_from(policy.maximum_bytes).unwrap_or(u64::MAX);
            if response
                .content_length
                .is_some_and(|length| length > ceiling)
                || response.body.len() > policy.maximum_bytes
            {
                return Err(Error::new(ErrorCode::CapacityExceeded));
            }
            if (300..400).contains(&response.status) {
                if hops >= policy.maximum_redirects {
                    return Err(Error::new(ErrorCode::CapacityExceeded));
                }
                let location = response
                    .location
                    .as_deref()
                    .ok_or_else(|| Error::new(ErrorCode::BackendUnavailable))?;
                let next = validate_redirect(policy, &current, location)?;
                redirects.push(current.to_string());
                current = next;
                hops += 1;
                continue;
            }
            if !(200..300).contains(&response.status) {
                return Err(Error::new(ErrorCode::BackendUnavailable));
            }
            let media_type = media_type_of(response.content_type.as_deref());
            let digest = *blake3::hash(&response.body).as_bytes();
            return Ok(FetchedSource {
                requested_url: requested.to_string(),
                final_url: current.to_string(),
                media_type,
                status: response.status,
                redirects,
                bytes: response.body,
                digest,
            });
        }
    }

    fn await_interval(&self, target: &Url) {
        let host = target.host_str().unwrap_or_default().to_ascii_lowercase();
        let interval = Duration::from_millis(self.state.policy.minimum_interval_ms);
        let mut last_request = self
            .state
            .last_request
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(previous) = last_request.get(&host) {
            let elapsed = previous.elapsed();
            if elapsed < interval {
                std::thread::sleep(interval.saturating_sub(elapsed));
            }
        }
        last_request.insert(host, Instant::now());
    }
}

pub fn policy_from_env() -> Result<Option<CrawlPolicy>, Error> {
    let declared = std::env::var("HM_WEB_SOURCE_HOSTS").unwrap_or_default();
    let allowed_hosts: Vec<String> = declared
        .split(',')
        .map(str::trim)
        .filter(|host| !host.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();
    if allowed_hosts.is_empty() {
        return Ok(None);
    }
    let maximum_bytes = numeric_from_env("HM_WEB_SOURCE_MAX_BYTES", DEFAULT_MAXIMUM_BYTES)?;
    let maximum_bytes = usize::try_from(maximum_bytes)
        .unwrap_or(MAXIMUM_SOURCE_BYTES)
        .min(MAXIMUM_SOURCE_BYTES);
    let maximum_redirects =
        numeric_from_env("HM_WEB_SOURCE_MAX_REDIRECTS", DEFAULT_MAXIMUM_REDIRECTS)?
            .min(MAXIMUM_REDIRECT_BUDGET);
    let maximum_redirects = u8::try_from(maximum_redirects).unwrap_or(u8::MAX);
    let minimum_interval_ms =
        numeric_from_env("HM_WEB_SOURCE_MIN_INTERVAL_MS", DEFAULT_MINIMUM_INTERVAL_MS)?;
    let request_timeout_ms = numeric_from_env("HM_WEB_SOURCE_TIMEOUT_MS", DEFAULT_TIMEOUT_MS)?;
    let allow_cross_host_redirect = match std::env::var("HM_WEB_SOURCE_CROSS_HOST_REDIRECT") {
        Err(_) => false,
        Ok(value) => {
            let value = value.trim().to_ascii_lowercase();
            value == "1" || value == "true"
        }
    };
    Ok(Some(CrawlPolicy {
        allowed_hosts,
        maximum_bytes,
        maximum_redirects,
        minimum_interval_ms,
        request_timeout_ms,
        allow_cross_host_redirect,
    }))
}

fn numeric_from_env(key: &str, default: u64) -> Result<u64, Error> {
    match std::env::var(key) {
        Err(_) => Ok(default),
        Ok(value) if value.trim().is_empty() => Ok(default),
        Ok(value) => value
            .trim()
            .parse::<u64>()
            .map_err(|_| Error::new(ErrorCode::InvalidArgument)),
    }
}

pub fn validate_target(policy: &CrawlPolicy, url: &str) -> Result<Url, Error> {
    let target = Url::parse(url).map_err(|_| Error::new(ErrorCode::InvalidArgument))?;
    if !matches!(target.scheme(), "http" | "https") {
        return Err(Error::new(ErrorCode::CapabilityDenied));
    }
    if !target.username().is_empty() || target.password().is_some() {
        return Err(Error::new(ErrorCode::CapabilityDenied));
    }
    let host = target
        .host()
        .ok_or_else(|| Error::new(ErrorCode::CapabilityDenied))?;
    let literal = match host {
        Host::Ipv4(address) => Some(IpAddr::V4(address)),
        Host::Ipv6(address) => Some(IpAddr::V6(address)),
        Host::Domain(_) => None,
    };
    if literal.is_some_and(is_private_address) {
        return Err(Error::new(ErrorCode::CapabilityDenied));
    }
    let name = target
        .host_str()
        .ok_or_else(|| Error::new(ErrorCode::CapabilityDenied))?;
    if !policy
        .allowed_hosts
        .iter()
        .any(|allowed| allowed.eq_ignore_ascii_case(name))
    {
        return Err(Error::new(ErrorCode::CapabilityDenied));
    }
    Ok(target)
}

pub fn validate_redirect(policy: &CrawlPolicy, from: &Url, location: &str) -> Result<Url, Error> {
    let joined = from
        .join(location)
        .map_err(|_| Error::new(ErrorCode::InvalidArgument))?;
    let target = validate_target(policy, joined.as_str())?;
    if !policy.allow_cross_host_redirect {
        let origin = from.host_str().unwrap_or_default();
        let destination = target.host_str().unwrap_or_default();
        if !origin.eq_ignore_ascii_case(destination) {
            return Err(Error::new(ErrorCode::CapabilityDenied));
        }
    }
    Ok(target)
}

#[must_use]
pub fn is_private_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => is_private_v4(address),
        IpAddr::V6(address) => {
            if let Some(mapped) = address.to_ipv4_mapped() {
                return is_private_v4(mapped);
            }
            let segments = address.segments();
            address.is_unspecified()
                || address.is_loopback()
                || segments[0] & 0xfe00 == 0xfc00
                || segments[0] & 0xffc0 == 0xfe80
        }
    }
}

fn is_private_v4(address: Ipv4Addr) -> bool {
    let octets = address.octets();
    address.is_unspecified()
        || address.is_loopback()
        || address.is_private()
        || address.is_link_local()
        || address.is_broadcast()
        || address.is_documentation()
        || (octets[0] == 100 && (64..128).contains(&octets[1]))
        || (octets[0] == 198 && (octets[1] == 18 || octets[1] == 19))
}

pub fn resolved_addresses_are_public(host: &str, port: u16) -> Result<(), Error> {
    let addresses: Vec<SocketAddr> = (host, port)
        .to_socket_addrs()
        .map_err(|_| Error::new(ErrorCode::CapabilityDenied))?
        .collect();
    if addresses.is_empty()
        || addresses
            .iter()
            .any(|address| is_private_address(address.ip()))
    {
        return Err(Error::new(ErrorCode::CapabilityDenied));
    }
    Ok(())
}

#[must_use]
pub fn media_type_of(content_type: Option<&str>) -> String {
    content_type
        .and_then(|value| value.split(';').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map_or_else(|| DEFAULT_MEDIA_TYPE.to_owned(), str::to_ascii_lowercase)
}

#[must_use]
pub fn media_conversation(digest: &[u8; 32]) -> ConversationId {
    ConversationId::derive(&format!("hm-media-v1/{}", crate::hex(digest)))
}

fn observed_envelope(
    payload: EventPayload,
    index: u32,
    count: u32,
    retention: Retention,
    sensitivity: Sensitivity,
) -> EventEnvelope {
    EventEnvelope {
        schema_version: CURRENT_SCHEMA_VERSION,
        payload,
        connection_id: None,
        client_seq: 0,
        client_event_index: index,
        client_event_count: count,
        origin_actor: 0,
        run_id: None,
        model_provenance: None,
        authority: Authority::ExternalObserved,
        retention,
        sensitivity,
        event_time_ns: 0,
    }
}

#[allow(clippy::too_many_lines, clippy::single_match_else)]
pub(crate) async fn run(
    actor: &ActorEngine,
    runtime: Option<&WebSourceRuntime>,
    embedding: Option<&EmbeddingRuntime>,
    input: RememberInput,
) -> Result<Envelope, Error> {
    let runtime = runtime
        .ok_or_else(|| Error::new(ErrorCode::OperationUnavailable))?
        .clone();
    let source = input
        .source
        .as_ref()
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
    if source.url.is_empty() {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    if input
        .anchor
        .as_ref()
        .is_some_and(|anchor| anchor.value.is_empty() || anchor.value.len() > 4096)
    {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let retention: Retention = input.retention.unwrap_or(RetentionInput::Durable).into();
    let sensitivity: Sensitivity = input
        .sensitivity
        .unwrap_or(SensitivityInput::Personal)
        .into();
    if retention == Retention::DoNotStore {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let requested = source.url.clone();
    let fetched = tokio::task::spawn_blocking(move || runtime.fetch(&requested))
        .await
        .map_err(|_| Error::new(ErrorCode::OperationUnavailable))??;
    let extracted = if webtext::is_textual(&fetched.media_type) {
        Some(webtext::extract_text(&fetched.media_type, &fetched.bytes)?)
    } else {
        None
    };
    let chunks = match extracted.as_ref() {
        Some(extracted) => {
            hm_compose::reconstruct::guard_remember(&extracted.text)?;
            crate::chunk_text(
                &extracted.text,
                input.chunk_bytes.unwrap_or(DEFAULT_CHUNK_BYTES),
            )?
        }
        None => Vec::new(),
    };
    let conversation = ConversationId::derive(&input.conversation);
    let count = u32::try_from(chunks.len().saturating_add(2))
        .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
    let mut events = Vec::with_capacity(chunks.len() + 2);
    events.push(IncomingEvent {
        kind: hm_ledger::frame::EventKind::MediaRef,
        conversation,
        payload: encode_event_envelope(&observed_envelope(
            EventPayload::MediaRef(Box::new(MediaRef {
                uri: fetched.final_url.clone(),
                media_type: fetched.media_type.clone(),
                digest: fetched.digest.to_vec(),
            })),
            0,
            count,
            retention,
            sensitivity,
        )),
    });
    events.push(IncomingEvent {
        kind: hm_ledger::frame::EventKind::ProviderFrame,
        conversation: media_conversation(&fetched.digest),
        payload: encode_event_envelope(&observed_envelope(
            EventPayload::ProviderFrame(Box::new(ProviderFrame {
                provider: WEB_SOURCE_PROVIDER_LABEL.to_owned(),
                api_content: fetched.bytes.clone(),
            })),
            1,
            count,
            retention,
            sensitivity,
        )),
    });
    for (offset, chunk) in chunks.iter().enumerate() {
        let index = u32::try_from(offset.saturating_add(2))
            .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
        events.push(IncomingEvent {
            kind: hm_ledger::frame::EventKind::UserMsg,
            conversation,
            payload: encode_event_envelope(&observed_envelope(
                EventPayload::UserMsg(Box::new(UserMsg {
                    content: chunk.as_bytes().to_vec(),
                })),
                index,
                count,
                retention,
                sensitivity,
            )),
        });
    }
    let documents = embedding.map(|_| {
        chunks
            .iter()
            .map(|chunk| (*chunk).to_owned())
            .collect::<Vec<_>>()
    });
    let outcome = actor.append(events).await?;
    let first_lsn = outcome.first_lsn.get();
    let media_ref_lsn = first_lsn;
    let retained_lsn = first_lsn + 1;
    let text_first_lsn = LSN::new(first_lsn + 2);
    let mut envelope = Envelope::empty();
    envelope.items.push(json!({
        "first_lsn": first_lsn,
        "last_lsn": outcome.last_lsn.get(),
        "count": outcome.last_lsn.get() - first_lsn + 1,
        "media_ref_lsn": media_ref_lsn,
        "retained_lsn": retained_lsn,
        "final_url": fetched.final_url,
        "media_type": fetched.media_type,
        "digest": crate::hex(&fetched.digest),
        "bytes": fetched.bytes.len(),
        "redirects": fetched.redirects,
        "anchor": input.anchor.as_ref().map(|anchor| json!({
            "facet": format!("{:?}", anchor.facet).to_lowercase(),
            "value": anchor.value,
        })),
    }));
    if !chunks.is_empty() {
        envelope.items[0]["text_first_lsn"] = json!(text_first_lsn.get());
        envelope.items[0]["text_last_lsn"] = json!(outcome.last_lsn.get());
    }
    for lsn in first_lsn..=outcome.last_lsn.get() {
        envelope
            .provenance
            .push(format!("hm://{}/lsn/{lsn}", actor.actor()));
    }
    if extracted.is_none() {
        envelope.gaps.push(json!({
            "kind": "media_derivation_pending",
            "media_lsn": media_ref_lsn,
            "media_type": fetched.media_type,
        }));
    }
    match (embedding, documents) {
        (Some(runtime), Some(documents)) if !documents.is_empty() => {
            match crate::append_embeddings(
                actor,
                runtime,
                documents,
                text_first_lsn,
                conversation,
                retention,
                sensitivity,
            )
            .await
            {
                Ok((first, last)) => {
                    envelope.items[0]["embedding_first_lsn"] = json!(first.get());
                    envelope.items[0]["embedding_last_lsn"] = json!(last.get());
                    envelope.health["encoder"] = json!(runtime.health());
                    envelope.health["backlog"] = json!("semantic_ready");
                }
                Err(_) => {
                    envelope.health["encoder"] = json!("semantic_lagging");
                    envelope.health["backlog"] = json!("semantic_lagging");
                    envelope.gaps.push(json!({
                        "kind": "embedding_pending",
                        "first_lsn": text_first_lsn.get(),
                        "last_lsn": outcome.last_lsn.get(),
                    }));
                    envelope.warnings.push(
                        "Observation stored; its embedding was not confirmed committed.".to_owned(),
                    );
                }
            }
        }
        _ => envelope.health["encoder"] = json!("lexical_only"),
    }
    Ok(envelope)
}
