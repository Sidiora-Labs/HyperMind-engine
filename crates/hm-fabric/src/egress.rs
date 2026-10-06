use hm_context::{Scope, validate_id};
use reqwest::{
    Url,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EgressMethod {
    Get,
    Head,
    Post,
    Put,
    Patch,
    Delete,
    Options,
}
impl EgressMethod {
    fn native(self) -> reqwest::Method {
        match self {
            Self::Get => reqwest::Method::GET,
            Self::Head => reqwest::Method::HEAD,
            Self::Post => reqwest::Method::POST,
            Self::Put => reqwest::Method::PUT,
            Self::Patch => reqwest::Method::PATCH,
            Self::Delete => reqwest::Method::DELETE,
            Self::Options => reqwest::Method::OPTIONS,
        }
    }
    fn read_only(self) -> bool {
        matches!(self, Self::Get | Self::Head | Self::Options)
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EgressRouteKind {
    Destination,
    Proxy,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EgressRoute {
    pub id: String,
    pub kind: EgressRouteKind,
    pub scheme: String,
    pub host: String,
    pub port: u16,
    pub path_prefix: String,
    pub methods: BTreeSet<EgressMethod>,
    pub allowed_headers: BTreeSet<String>,
    pub allowed_non_public_addresses: BTreeSet<IpAddr>,
}
impl EgressRoute {
    pub fn for_url(
        id: impl Into<String>,
        kind: EgressRouteKind,
        url: &str,
        methods: BTreeSet<EgressMethod>,
    ) -> Result<Self, EgressError> {
        let url = parse_url(url)?;
        Ok(Self {
            id: id.into(),
            kind,
            scheme: url.scheme().into(),
            host: url
                .host_str()
                .ok_or(EgressError::Invalid("destination host"))?
                .into(),
            port: url
                .port_or_known_default()
                .ok_or(EgressError::Invalid("destination port"))?,
            path_prefix: url.path().into(),
            methods,
            allowed_headers: BTreeSet::new(),
            allowed_non_public_addresses: BTreeSet::new(),
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EgressProxy {
    pub url: String,
    pub route_id: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EgressPolicy {
    pub version: u32,
    pub scope: Scope,
    pub routes: Vec<EgressRoute>,
    pub proxy: Option<EgressProxy>,
    pub max_redirects: u8,
    pub max_request_bytes: usize,
    pub max_response_bytes: usize,
    pub max_header_bytes: usize,
    pub timeout_ms: u64,
}
impl EgressPolicy {
    pub fn validate(&self) -> Result<(), EgressError> {
        self.scope
            .validate()
            .map_err(|_| EgressError::Invalid("scope"))?;
        if self.version != 1
            || self.routes.len() > 128
            || self.max_redirects > 8
            || self.max_request_bytes == 0
            || self.max_request_bytes > 16 * 1024 * 1024
            || self.max_response_bytes == 0
            || self.max_response_bytes > 16 * 1024 * 1024
            || self.max_header_bytes == 0
            || self.max_header_bytes > 65536
            || self.timeout_ms == 0
            || self.timeout_ms > 120000
        {
            return Err(EgressError::Invalid("policy bounds"));
        }
        let mut ids = BTreeSet::new();
        for route in &self.routes {
            validate_id(&route.id).map_err(|_| EgressError::Invalid("route identity"))?;
            let host = host_authority(&route.host);
            let canonical = parse_url(&format!("{}://{}:{}/", route.scheme, host, route.port))?;
            if !ids.insert(&route.id)
                || route.port == 0
                || canonical.host_str() != Some(route.host.as_str())
                || route.methods.is_empty()
                || route.path_prefix.len() > 2048
                || !route.path_prefix.starts_with('/')
                || route.path_prefix.contains(['%', '?', '#', '\\'])
                || route.path_prefix.chars().any(char::is_control)
                || route.allowed_non_public_addresses.len() > 64
                || route.allowed_headers.len() > 64
                || route.allowed_headers.iter().any(|name| {
                    HeaderName::from_bytes(name.as_bytes()).is_err()
                        || name != &name.to_ascii_lowercase()
                        || reserved_header(name)
                })
            {
                return Err(EgressError::Invalid("route bounds or canonical identity"));
            }
        }
        if let Some(proxy) = &self.proxy {
            validate_id(&proxy.route_id)
                .map_err(|_| EgressError::Invalid("proxy route identity"))?;
            let url = parse_url(&proxy.url)?;
            if url.scheme() != "http"
                || url.path() != "/"
                || url.query().is_some()
                || !self
                    .routes
                    .iter()
                    .any(|route| route.id == proxy.route_id && route.kind == EgressRouteKind::Proxy)
            {
                return Err(EgressError::Invalid("proxy configuration"));
            }
        }
        Ok(())
    }
}

pub struct EgressRequest {
    pub url: String,
    pub method: EgressMethod,
    pub headers: Vec<(String, Vec<u8>)>,
    pub body: Vec<u8>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResolvedDestination {
    pub route_id: String,
    pub origin: String,
    pub addresses: Vec<SocketAddr>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EgressReceipt {
    pub version: u32,
    pub scope: Scope,
    pub destination: ResolvedDestination,
    pub proxy: Option<ResolvedDestination>,
    pub connected_peer: SocketAddr,
    pub redirects: u8,
}
pub struct EgressResponse {
    pub status: u16,
    pub body: Vec<u8>,
    pub receipt: EgressReceipt,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum EgressError {
    #[error("invalid egress input: {0}")]
    Invalid(&'static str),
    #[error("egress denied: {0}")]
    Denied(&'static str),
    #[error("egress resolution failed")]
    Resolution,
    #[error("egress connection or response failed")]
    Network,
    #[error("write outcome uncertain; reconciliation required")]
    OutcomeUncertain,
    #[error("egress response limit exceeded")]
    ResponseLimit,
    #[error("egress transport unavailable: {0}")]
    Unavailable(&'static str),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EgressCapabilities {
    pub version: u32,
    pub direct_http: bool,
    pub direct_https: bool,
    pub pinned_http_proxy: bool,
    pub pinned_https_proxy: bool,
    pub ambient_proxy: bool,
}
pub struct EgressHttpClient {
    policy: EgressPolicy,
}
impl EgressHttpClient {
    pub fn new(policy: EgressPolicy) -> Result<Self, EgressError> {
        policy.validate()?;
        Ok(Self { policy })
    }
    pub fn policy(&self) -> &EgressPolicy {
        &self.policy
    }
    pub fn capabilities(&self) -> EgressCapabilities {
        EgressCapabilities {
            version: 1,
            direct_http: true,
            direct_https: true,
            pinned_http_proxy: true,
            pinned_https_proxy: false,
            ambient_proxy: false,
        }
    }
    pub async fn authorize(
        &self,
        url: &str,
        method: EgressMethod,
    ) -> Result<ResolvedDestination, EgressError> {
        let url = parse_url(url)?;
        self.resolve(&url, method, EgressRouteKind::Destination, None)
            .await
    }
    fn route<'a>(
        &'a self,
        url: &Url,
        method: EgressMethod,
        kind: EgressRouteKind,
        id: Option<&str>,
    ) -> Result<&'a EgressRoute, EgressError> {
        self.policy
            .routes
            .iter()
            .find(|route| {
                route.kind == kind
                    && id.is_none_or(|id| id == route.id)
                    && route.scheme == url.scheme()
                    && Some(route.host.as_str()) == url.host_str()
                    && Some(route.port) == url.port_or_known_default()
                    && path_matches(&route.path_prefix, url.path())
                    && route.methods.contains(&method)
            })
            .ok_or(EgressError::Denied("destination route"))
    }
    async fn resolve(
        &self,
        url: &Url,
        method: EgressMethod,
        kind: EgressRouteKind,
        id: Option<&str>,
    ) -> Result<ResolvedDestination, EgressError> {
        let route = self.route(url, method, kind, id)?;
        let host = url
            .host_str()
            .ok_or(EgressError::Invalid("destination host"))?;
        let port = route.port;
        let literal = host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<IpAddr>();
        let mut addresses = if let Ok(ip) = literal {
            vec![SocketAddr::new(ip, port)]
        } else {
            tokio::time::timeout(
                Duration::from_millis(self.policy.timeout_ms),
                tokio::net::lookup_host((host, port)),
            )
            .await
            .map_err(|_| EgressError::Resolution)?
            .map_err(|_| EgressError::Resolution)?
            .collect::<Vec<_>>()
        };
        addresses.sort();
        addresses.dedup();
        if addresses.is_empty()
            || addresses.len() > 64
            || addresses
                .iter()
                .any(|address| !address_allowed(route, address.ip()))
        {
            return Err(EgressError::Denied("resolved address set"));
        }
        Ok(ResolvedDestination {
            route_id: route.id.clone(),
            origin: origin(url)?,
            addresses,
        })
    }
    pub async fn execute(&self, request: EgressRequest) -> Result<EgressResponse, EgressError> {
        if request.body.len() > self.policy.max_request_bytes
            || request.headers.len() > 64
            || request
                .headers
                .iter()
                .map(|(name, value)| name.len() + value.len())
                .sum::<usize>()
                > self.policy.max_header_bytes
            || (request.method.read_only() && !request.body.is_empty())
        {
            return Err(EgressError::Invalid("request bounds"));
        }
        let mut url = parse_url(&request.url)?;
        let mut headers = request.headers;
        let mut redirects = 0;
        loop {
            let destination = self
                .resolve(&url, request.method, EgressRouteKind::Destination, None)
                .await?;
            let route = self.route(&url, request.method, EgressRouteKind::Destination, None)?;
            let mut outgoing = HeaderMap::new();
            let mut names = BTreeSet::new();
            for (name, value) in &headers {
                let name = HeaderName::from_bytes(name.as_bytes())
                    .map_err(|_| EgressError::Invalid("header name"))?;
                if reserved_header(name.as_str())
                    || !route.allowed_headers.contains(name.as_str())
                    || !names.insert(name.as_str().to_owned())
                {
                    return Err(EgressError::Denied("request header"));
                }
                let mut value = HeaderValue::from_bytes(value)
                    .map_err(|_| EgressError::Invalid("header value"))?;
                value.set_sensitive(true);
                outgoing.insert(name, value);
            }
            let mut builder = reqwest::Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .retry(reqwest::retry::never())
                .http1_only()
                .pool_max_idle_per_host(0)
                .connect_timeout(Duration::from_millis(self.policy.timeout_ms))
                .timeout(Duration::from_millis(self.policy.timeout_ms));
            let mut target = url.clone();
            let proxy = if let Some(proxy) = &self.policy.proxy {
                if url.scheme() != "http" {
                    return Err(EgressError::Unavailable("pinned HTTPS proxy tunneling"));
                }
                let proxy_url = parse_url(&proxy.url)?;
                let resolved = self
                    .resolve(
                        &proxy_url,
                        request.method,
                        EgressRouteKind::Proxy,
                        Some(&proxy.route_id),
                    )
                    .await?;
                let address = destination.addresses[0];
                target = literal_url(&url, address)?;
                outgoing.insert(
                    reqwest::header::HOST,
                    HeaderValue::from_str(&authority(&url)?)
                        .map_err(|_| EgressError::Invalid("host authority"))?,
                );
                let pinned_proxy = literal_url(&proxy_url, resolved.addresses[0])?;
                builder = builder.proxy(
                    reqwest::Proxy::http(pinned_proxy.as_str())
                        .map_err(|_| EgressError::Invalid("proxy configuration"))?,
                );
                Some(resolved)
            } else {
                builder = builder.resolve_to_addrs(
                    url.host_str()
                        .ok_or(EgressError::Invalid("destination host"))?,
                    &destination.addresses,
                );
                None
            };
            let client = builder
                .build()
                .map_err(|_| EgressError::Unavailable("HTTP client"))?;
            let mut response = client
                .request(request.method.native(), target)
                .headers(outgoing)
                .body(request.body.clone())
                .send()
                .await
                .map_err(|_| network_error(request.method))?;
            let peer = response
                .remote_addr()
                .ok_or(EgressError::Unavailable("connection peer evidence"))?;
            let expected = proxy
                .as_ref()
                .map_or(&destination.addresses, |proxy| &proxy.addresses);
            if !expected.iter().any(|address| same_socket(*address, peer)) {
                return Err(EgressError::Denied("connection peer pin"));
            }
            let status = response.status().as_u16();
            if response
                .headers()
                .iter()
                .map(|(name, value)| name.as_str().len() + value.as_bytes().len())
                .sum::<usize>()
                > self.policy.max_header_bytes
            {
                return Err(response_limit(request.method));
            }
            if matches!(status, 301 | 302 | 303 | 307 | 308) {
                if !request.method.read_only() {
                    return Err(EgressError::OutcomeUncertain);
                }
                if redirects >= self.policy.max_redirects {
                    return Err(EgressError::Denied("redirect bound"));
                }
                let location = response
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .ok_or(EgressError::Invalid("redirect location"))?
                    .to_str()
                    .map_err(|_| EgressError::Invalid("redirect location"))?;
                if location.len() > 2048 {
                    return Err(EgressError::Invalid("redirect location"));
                }
                let next = url
                    .join(location)
                    .map_err(|_| EgressError::Invalid("redirect location"))?;
                let next = parse_url(next.as_str())?;
                let next_route =
                    self.route(&next, request.method, EgressRouteKind::Destination, None)?;
                if origin(&url)? != origin(&next)? || destination.route_id != next_route.id {
                    headers.clear();
                }
                url = next;
                redirects += 1;
                continue;
            }
            if response
                .headers()
                .iter()
                .map(|(name, value)| name.as_str().len() + value.as_bytes().len())
                .sum::<usize>()
                > self.policy.max_header_bytes
            {
                return Err(response_limit(request.method));
            }
            if response
                .content_length()
                .is_some_and(|length| length > self.policy.max_response_bytes as u64)
            {
                return Err(response_limit(request.method));
            }
            let mut body = Vec::new();
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|_| network_error(request.method))?
            {
                if chunk.len() > self.policy.max_response_bytes.saturating_sub(body.len()) {
                    return Err(response_limit(request.method));
                }
                body.extend_from_slice(&chunk);
            }
            return Ok(EgressResponse {
                status,
                body,
                receipt: EgressReceipt {
                    version: 1,
                    scope: self.policy.scope.clone(),
                    destination,
                    proxy,
                    connected_peer: peer,
                    redirects,
                },
            });
        }
    }
}
fn response_limit(method: EgressMethod) -> EgressError {
    if method.read_only() {
        EgressError::ResponseLimit
    } else {
        EgressError::OutcomeUncertain
    }
}
fn network_error(method: EgressMethod) -> EgressError {
    if method.read_only() {
        EgressError::Network
    } else {
        EgressError::OutcomeUncertain
    }
}
fn reserved_header(name: &str) -> bool {
    matches!(
        name,
        "host"
            | "proxy-authorization"
            | "proxy-connection"
            | "connection"
            | "upgrade"
            | "transfer-encoding"
            | "content-length"
            | "te"
            | "trailer"
            | "forwarded"
            | "x-forwarded-host"
    )
}
fn parse_url(value: &str) -> Result<Url, EgressError> {
    if value.len() > 2048 || value.chars().any(char::is_control) {
        return Err(EgressError::Invalid("destination URL"));
    }
    let url = Url::parse(value).map_err(|_| EgressError::Invalid("destination URL"))?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.host_str().is_none()
        || url.path().contains(['%', '\\'])
    {
        return Err(EgressError::Denied("URL shape or credentials"));
    }
    Ok(url)
}
fn path_matches(prefix: &str, path: &str) -> bool {
    prefix == "/"
        || path == prefix
        || prefix.ends_with('/') && path.starts_with(prefix)
        || path
            .strip_prefix(prefix)
            .is_some_and(|tail| tail.starts_with('/'))
}
fn host_authority(host: &str) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]")
    } else {
        host.into()
    }
}
fn authority(url: &Url) -> Result<String, EgressError> {
    Ok(format!(
        "{}:{}",
        host_authority(
            url.host_str()
                .ok_or(EgressError::Invalid("destination host"))?
        ),
        url.port_or_known_default()
            .ok_or(EgressError::Invalid("destination port"))?
    ))
}
fn origin(url: &Url) -> Result<String, EgressError> {
    Ok(format!("{}://{}", url.scheme(), authority(url)?))
}
fn literal_url(url: &Url, address: SocketAddr) -> Result<Url, EgressError> {
    let mut pinned = Url::parse(&format!("{}://{address}/", url.scheme()))
        .map_err(|_| EgressError::Invalid("pinned address"))?;
    pinned.set_path(url.path());
    pinned.set_query(url.query());
    Ok(pinned)
}
fn normalize(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(ip) => ip.to_ipv4_mapped().map_or(IpAddr::V6(ip), IpAddr::V4),
        ip => ip,
    }
}
fn same_socket(left: SocketAddr, right: SocketAddr) -> bool {
    normalize(left.ip()) == normalize(right.ip()) && left.port() == right.port()
}
pub fn address_allowed(route: &EgressRoute, address: IpAddr) -> bool {
    let address = normalize(address);
    public_address(address)
        || route
            .allowed_non_public_addresses
            .iter()
            .any(|allowed| normalize(*allowed) == address)
}
pub fn public_address(address: IpAddr) -> bool {
    match normalize(address) {
        IpAddr::V4(ip) => {
            let value = u32::from(ip);
            ![
                (Ipv4Addr::new(0, 0, 0, 0), 8),
                (Ipv4Addr::new(10, 0, 0, 0), 8),
                (Ipv4Addr::new(100, 64, 0, 0), 10),
                (Ipv4Addr::new(127, 0, 0, 0), 8),
                (Ipv4Addr::new(169, 254, 0, 0), 16),
                (Ipv4Addr::new(172, 16, 0, 0), 12),
                (Ipv4Addr::new(192, 0, 0, 0), 24),
                (Ipv4Addr::new(192, 0, 2, 0), 24),
                (Ipv4Addr::new(192, 88, 99, 0), 24),
                (Ipv4Addr::new(192, 168, 0, 0), 16),
                (Ipv4Addr::new(198, 18, 0, 0), 15),
                (Ipv4Addr::new(198, 51, 100, 0), 24),
                (Ipv4Addr::new(203, 0, 113, 0), 24),
                (Ipv4Addr::new(224, 0, 0, 0), 3),
            ]
            .iter()
            .any(|(base, bits)| value & (u32::MAX << (32 - bits)) == u32::from(*base))
        }
        IpAddr::V6(ip) => {
            let s = ip.segments();
            (s[0] & 0xe000) == 0x2000
                && s[0] != 0x2002
                && !(s[0] == 0x2001 && (s[1] < 0x200 || s[1] == 0xdb8))
        }
    }
}
