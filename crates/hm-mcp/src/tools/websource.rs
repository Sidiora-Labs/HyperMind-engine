use hm_core::{Error, ErrorCode};
use std::collections::{HashMap, VecDeque};
use std::io::Read as _;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs as _};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};
use url::{Host, Url};

pub const MAXIMUM_SOURCE_BYTES: usize = 8 * 1024 * 1024;

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

pub struct WebSourceRuntime {
    policy: CrawlPolicy,
    transport: Arc<dyn FetchTransport>,
    last_request: Mutex<HashMap<String, Instant>>,
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
            policy,
            transport,
            last_request: Mutex::new(HashMap::new()),
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
        &self.policy
    }

    pub fn fetch(&self, url: &str) -> Result<FetchedSource, Error> {
        let requested = validate_target(&self.policy, url)?;
        let mut current = requested.clone();
        let mut redirects: Vec<String> = Vec::new();
        let mut hops: u8 = 0;
        loop {
            self.await_interval(&current);
            let response = self.transport.fetch(
                current.as_str(),
                self.policy.request_timeout_ms,
                self.policy.maximum_bytes,
            )?;
            let ceiling = u64::try_from(self.policy.maximum_bytes).unwrap_or(u64::MAX);
            if response
                .content_length
                .is_some_and(|length| length > ceiling)
                || response.body.len() > self.policy.maximum_bytes
            {
                return Err(Error::new(ErrorCode::CapacityExceeded));
            }
            if (300..400).contains(&response.status) {
                if hops >= self.policy.maximum_redirects {
                    return Err(Error::new(ErrorCode::CapacityExceeded));
                }
                let location = response
                    .location
                    .as_deref()
                    .ok_or_else(|| Error::new(ErrorCode::BackendUnavailable))?;
                let next = validate_redirect(&self.policy, &current, location)?;
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
        let interval = Duration::from_millis(self.policy.minimum_interval_ms);
        let mut last_request = self
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
