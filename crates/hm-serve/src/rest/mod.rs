#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

use crate::grpc::{Gateway, ListenerRole, TlsIdentity};
use axum::extract::{DefaultBodyLimit, Path, State, rejection::JsonRejection};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine as _;
use hm_schema::protocol::{
    CURRENT_PROTOCOL_VERSION, MAXIMUM_QUERY_BYTES, encode_wire_envelope, verify_wire_envelope,
};
use hm_schema::wire::{
    CryptoDelete, Hello, RebuildProjection, Request, RequestPayload, ResponsePayload, Stats,
    ToolRequest, VerifyStatus, WireEnvelope, WirePayload,
};
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder;
use hyper_util::service::TowerToHyperService;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::future::Future;
use std::net::SocketAddr;
use std::path::{Path as FilePath, PathBuf};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{Semaphore, watch};
use tokio_rustls::TlsAcceptor;
use utoipa::OpenApi;

pub const VERBS: [&str; 14] = [
    "remember",
    "recall",
    "activate",
    "believe",
    "retract",
    "dispute",
    "intend",
    "bind",
    "predict",
    "outcome",
    "attest",
    "consolidate",
    "inspect",
    "forget",
];

pub const CONSOLE_ASSET_TYPES: [(&str, &str); 5] = [
    ("html", "text/html; charset=utf-8"),
    ("js", "text/javascript; charset=utf-8"),
    ("css", "text/css; charset=utf-8"),
    ("json", "application/json"),
    ("svg", "image/svg+xml"),
];

pub const MAXIMUM_CONSOLE_ASSET_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct ToolCall {
    /// Stable 16-byte connection identity encoded as 32 hexadecimal characters.
    pub connection_id: String,
    pub request_id: u64,
    /// Unchanged arguments accepted by the corresponding MCP verb.
    pub arguments: Value,
}

#[derive(Clone, Deserialize, Serialize, utoipa::ToSchema)]
pub struct AdminCall {
    pub connection_id: String,
    pub request_id: u64,
    #[serde(default)]
    pub actor: u16,
    #[serde(default)]
    pub arguments: Value,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ToolEnvelope {
    pub ok: bool,
    pub items: Vec<Value>,
    pub provenance: Vec<String>,
    pub budget: Option<Value>,
    pub gaps: Vec<Value>,
    pub health: Value,
    pub warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_state: Option<String>,
    /// Retrieval manifest returned by activate: retrieved, selected, included, and used LSNs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest: Option<Value>,
}

#[derive(OpenApi)]
#[openapi(
    paths(tool, admin),
    components(schemas(ToolCall, AdminCall, ToolEnvelope))
)]
struct Api;

#[must_use]
pub fn openapi() -> utoipa::openapi::OpenApi {
    use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityRequirement, SecurityScheme};
    let mut document = Api::openapi();
    "HyperMind remote API".clone_into(&mut document.info.title);
    "3.0.0".clone_into(&mut document.info.version);
    document.info.description = Some("Frozen NCPR v3. Mandatory mutual TLS plus capability bearer token. Actor and admin listeners are separate. Existing fourteen MCP verb names are preserved.".to_owned());
    let components = document
        .components
        .as_mut()
        .expect("declared OpenAPI components");
    let mut capability = Http::new(HttpAuthScheme::Bearer);
    capability.description = Some("32-byte actor or admin token encoded as 64 hexadecimal characters; role must match listener.".to_owned());
    components.add_security_scheme("capability", SecurityScheme::Http(capability));
    components.add_security_scheme(
        "mutualTLS",
        SecurityScheme::MutualTls {
            description: Some("Client certificate signed by the configured client CA".to_owned()),
            extensions: None,
        },
    );
    document.security = Some(vec![
        SecurityRequirement::new("capability", Vec::<String>::new())
            .add("mutualTLS", Vec::<String>::new()),
    ]);
    let operation = document
        .paths
        .paths
        .remove("/v1/{verb}")
        .expect("declared tool path");
    for verb in VERBS {
        let mut path = operation.clone();
        let post = path.post.as_mut().expect("declared POST operation");
        post.operation_id = Some(verb.to_owned());
        post.parameters = None;
        document.paths.paths.insert(format!("/v1/{verb}"), path);
    }
    document
}

pub struct RestServer {
    listener: TcpListener,
    gateway: Gateway,
    acceptor: TlsAcceptor,
    console_directory: Option<PathBuf>,
}

#[derive(Clone)]
struct ConsoleState {
    directory: Option<Arc<PathBuf>>,
    role: ListenerRole,
}

impl RestServer {
    pub async fn bind(
        address: SocketAddr,
        gateway: Gateway,
        tls: TlsIdentity,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let acceptor = TlsAcceptor::from(tls.rustls_config()?);
        let listener = TcpListener::bind(address).await?;
        Ok(Self {
            listener,
            gateway,
            acceptor,
            console_directory: None,
        })
    }

    #[must_use]
    pub fn with_console_directory(mut self, directory: PathBuf) -> Self {
        self.console_directory = Some(directory);
        self
    }

    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    pub async fn serve_until(
        self,
        shutdown: impl Future<Output = ()> + Send + 'static,
    ) -> Result<(), std::io::Error> {
        let limit = Arc::new(Semaphore::new(self.gateway.config.maximum_connections));
        let console = Router::new()
            .route("/console", get(console_index))
            .route("/console/{file}", get(console_asset))
            .with_state(ConsoleState {
                directory: self.console_directory.map(Arc::new),
                role: self.gateway.role,
            });
        let app = Router::new()
            .route("/v1/{verb}", post(tool))
            .route("/v1/admin/{verb}", post(admin))
            .route("/openapi.json", get(|| async { Json(openapi()) }))
            .route(
                "/openapi.yaml",
                get(|| async {
                    (
                        [("content-type", "application/yaml")],
                        openapi().to_yaml().expect("OpenAPI YAML"),
                    )
                }),
            )
            .with_state(self.gateway)
            .merge(console)
            .layer(DefaultBodyLimit::max(MAXIMUM_QUERY_BYTES + 4096));
        let (stop, stopped) = watch::channel(false);
        let mut tasks = tokio::task::JoinSet::new();
        tokio::pin!(shutdown);
        loop {
            tokio::select! {
                () = &mut shutdown => break,
                Some(_) = tasks.join_next(), if !tasks.is_empty() => {},
                accepted = self.listener.accept() => {
                    let (socket, _) = accepted?;
                    let Ok(permit) = limit.clone().try_acquire_owned() else { continue; };
                    let acceptor = self.acceptor.clone();
                    let app = app.clone();
                    let mut stopped = stopped.clone();
                    tasks.spawn(async move {
                        let _permit = permit;
                        let handshake = tokio::time::timeout(std::time::Duration::from_secs(10), acceptor.accept(socket));
                        let Ok(Ok(stream)) = (tokio::select! {
                            result = handshake => result,
                            _ = stopped.changed() => return,
                        }) else { return; };
                        let builder = Builder::new(TokioExecutor::new());
                        let connection = builder.serve_connection(TokioIo::new(stream), TowerToHyperService::new(app));
                        tokio::pin!(connection);
                        tokio::select! {
                            _ = &mut connection => {},
                            _ = stopped.changed() => {},
                        }
                    });
                }
            }
        }
        let _ = stop.send(true);
        while tasks.join_next().await.is_some() {}
        Ok(())
    }
}

#[utoipa::path(post, path = "/v1/{verb}", request_body = ToolCall,
    params(("verb" = String, Path, description = "Existing MCP verb")),
    responses((status = 200, body = ToolEnvelope), (status = 400, body = ToolEnvelope), (status = 401, body = ToolEnvelope), (status = 403, body = ToolEnvelope), (status = 503, body = ToolEnvelope)))]
async fn tool(
    State(gateway): State<Gateway>,
    Path(verb): Path<String>,
    headers: HeaderMap,
    body: Result<Json<ToolCall>, JsonRejection>,
) -> Response {
    let mutation = !matches!(verb.as_str(), "recall" | "activate" | "inspect");
    if gateway.role != ListenerRole::Actor {
        return failure(
            StatusCode::FORBIDDEN,
            "actor methods require the actor listener",
            mutation,
            "not_dispatched",
        );
    }
    if !VERBS.contains(&verb.as_str()) {
        return failure(
            StatusCode::NOT_FOUND,
            "unknown verb",
            mutation,
            "not_dispatched",
        );
    }
    let Ok(Json(body)) = body else {
        return failure(
            StatusCode::BAD_REQUEST,
            "invalid JSON request",
            mutation,
            "not_dispatched",
        );
    };
    if verb == "forget"
        && body.arguments.get("action").and_then(Value::as_str) == Some("crypto_shred")
    {
        return failure(
            StatusCode::FORBIDDEN,
            "crypto_shred requires the separate admin listener",
            true,
            "not_dispatched",
        );
    }
    let hello = match make_hello(&headers, &body.connection_id) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let Ok(arguments_json) = serde_json::to_vec(&body.arguments) else {
        return failure(
            StatusCode::BAD_REQUEST,
            "invalid arguments",
            mutation,
            "not_dispatched",
        );
    };
    let request = request_bytes(
        body.request_id,
        RequestPayload::ToolRequest(Box::new(ToolRequest {
            verb,
            arguments_json,
        })),
    );
    dispatch(&gateway, &hello, &request, mutation).await
}

#[utoipa::path(post, path = "/v1/admin/{verb}", request_body = AdminCall,
    params(("verb" = String, Path, description = "health, stats, verify, rebuild, or forget (crypto_shred only)")),
    responses((status = 200, body = ToolEnvelope), (status = 400, body = ToolEnvelope), (status = 401, body = ToolEnvelope), (status = 403, body = ToolEnvelope), (status = 503, body = ToolEnvelope)))]
async fn admin(
    State(gateway): State<Gateway>,
    Path(verb): Path<String>,
    headers: HeaderMap,
    body: Result<Json<AdminCall>, JsonRejection>,
) -> Response {
    let mutation = matches!(verb.as_str(), "rebuild" | "forget");
    if gateway.role != ListenerRole::Admin {
        return failure(
            StatusCode::FORBIDDEN,
            "admin methods require the admin listener",
            mutation,
            "not_dispatched",
        );
    }
    let Ok(Json(body)) = body else {
        return failure(
            StatusCode::BAD_REQUEST,
            "invalid JSON request",
            mutation,
            "not_dispatched",
        );
    };
    let hello = match make_hello(&headers, &body.connection_id) {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let payload = match verb.as_str() {
        "health" => RequestPayload::Health(Box::default()),
        "stats" => RequestPayload::Stats(Box::new(Stats { actor: body.actor })),
        "verify" => RequestPayload::VerifyStatus(Box::new(VerifyStatus { actor: body.actor })),
        "rebuild" => {
            let Some(name) = body.arguments.get("name").and_then(Value::as_str) else {
                return failure(
                    StatusCode::BAD_REQUEST,
                    "projection name required",
                    true,
                    "not_dispatched",
                );
            };
            RequestPayload::RebuildProjection(Box::new(RebuildProjection {
                actor: body.actor,
                name: name.to_owned(),
            }))
        }
        "forget"
            if body.arguments.get("action").and_then(Value::as_str) == Some("crypto_shred") =>
        {
            RequestPayload::CryptoDelete(Box::new(CryptoDelete { actor: body.actor }))
        }
        _ => {
            return failure(
                StatusCode::BAD_REQUEST,
                "unknown admin operation",
                mutation,
                "not_dispatched",
            );
        }
    };
    dispatch(
        &gateway,
        &hello,
        &request_bytes(body.request_id, payload),
        mutation,
    )
    .await
}

fn make_hello(headers: &HeaderMap, connection_id: &str) -> Result<Vec<u8>, Box<Response>> {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .and_then(decode_hex::<32>);
    let Some(token) = token else {
        return Err(Box::new(failure(
            StatusCode::UNAUTHORIZED,
            "capability bearer token required",
            true,
            "not_dispatched",
        )));
    };
    let Some(connection_id) = decode_hex::<16>(connection_id) else {
        return Err(Box::new(failure(
            StatusCode::BAD_REQUEST,
            "invalid connection_id",
            true,
            "not_dispatched",
        )));
    };
    Ok(encode_wire_envelope(&WireEnvelope {
        proto_version: CURRENT_PROTOCOL_VERSION,
        payload: WirePayload::Hello(Box::new(Hello {
            proto_version: CURRENT_PROTOCOL_VERSION,
            connection_id: connection_id.to_vec(),
            capability_token: token.to_vec(),
        })),
    }))
}

fn decode_hex<const N: usize>(value: &str) -> Option<[u8; N]> {
    if value.len() != N * 2 || !value.bytes().all(|value| value.is_ascii_hexdigit()) {
        return None;
    }
    let mut decoded = [0; N];
    for (index, byte) in decoded.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).ok()?;
    }
    Some(decoded)
}

fn request_bytes(request_id: u64, payload: RequestPayload) -> Vec<u8> {
    encode_wire_envelope(&WireEnvelope {
        proto_version: CURRENT_PROTOCOL_VERSION,
        payload: WirePayload::Request(Box::new(Request {
            request_id,
            payload,
        })),
    })
}

async fn dispatch(gateway: &Gateway, hello: &[u8], request: &[u8], mutation: bool) -> Response {
    let response = match gateway.exchange(hello, request).await {
        Ok(value) => value,
        Err(error) => {
            let (status, fallback_effect) = match error.code() {
                tonic::Code::Unauthenticated => (StatusCode::UNAUTHORIZED, "not_dispatched"),
                tonic::Code::PermissionDenied => (StatusCode::FORBIDDEN, "not_dispatched"),
                tonic::Code::InvalidArgument => (StatusCode::BAD_REQUEST, "not_dispatched"),
                tonic::Code::ResourceExhausted => (StatusCode::TOO_MANY_REQUESTS, "not_dispatched"),
                _ => (StatusCode::SERVICE_UNAVAILABLE, "unknown"),
            };
            let effect = error
                .metadata()
                .get("effect-state")
                .and_then(|value| value.to_str().ok())
                .unwrap_or(fallback_effect);
            return failure(status, error.message(), mutation, effect);
        }
    };
    let Ok(decoded) = verify_wire_envelope(&response) else {
        return failure(
            StatusCode::BAD_GATEWAY,
            "invalid daemon response",
            mutation,
            "unknown",
        );
    };
    let WirePayload::Response(response) = decoded.payload else {
        return failure(
            StatusCode::BAD_GATEWAY,
            "unexpected daemon response",
            mutation,
            "unknown",
        );
    };
    let item = match response.payload {
        Some(ResponsePayload::BytesResult(result)) => {
            return match serde_json::from_slice::<Value>(&result.bytes) {
                Ok(value) => Json(value).into_response(),
                Err(_) => failure(
                    StatusCode::BAD_GATEWAY,
                    "invalid tool envelope",
                    mutation,
                    "unknown",
                ),
            };
        }
        Some(ResponsePayload::HealthResult(value)) => {
            json!({"ready":value.ready,"actor_count":value.actor_count,"active_connections":value.active_connections})
        }
        Some(ResponsePayload::StatsResult(value)) => {
            json!({"actor":value.actor,"log_events":value.log_events,"log_bytes":value.log_bytes,"projection_stats":value.projection_stats.iter().map(|stat| json!({"name":stat.name,"applied_lsn":stat.applied_lsn})).collect::<Vec<_>>()})
        }
        Some(ResponsePayload::VerifyResult(value)) => {
            json!({"actor":value.actor,"verified":value.verified,"leaf_count":value.leaf_count,"root_base64":base64::engine::general_purpose::STANDARD.encode(value.root),"last_checkpoint_lsn":value.last_checkpoint_lsn})
        }
        Some(ResponsePayload::RebuildResult(value)) => {
            json!({"actor":value.actor,"name":value.name,"applied_lsn":value.applied_lsn})
        }
        Some(ResponsePayload::DeleteResult(value)) => {
            json!({"action":"crypto_shred","actor":value.actor,"receipt_base64":base64::engine::general_purpose::STANDARD.encode(value.receipt)})
        }
        Some(ResponsePayload::ErrorDetail(error)) => {
            let effect = match error.effect_state {
                hm_schema::wire::MutationEffectState::NotDispatched => "not_dispatched",
                hm_schema::wire::MutationEffectState::Rejected => "rejected",
                _ => "unknown",
            };
            let name = hm_core::ErrorCode::try_from(error.code)
                .map_or("kProtocolInvalid", hm_core::ErrorCode::as_str);
            let mut envelope = error_envelope(name, mutation, effect);
            envelope.items = vec![
                json!({"error":name,"code":error.code,"system_error":error.system_error,"lsn":error.lsn,"offset":error.offset}),
            ];
            return Json(envelope).into_response();
        }
        _ => {
            return failure(
                StatusCode::BAD_GATEWAY,
                "unexpected response payload",
                mutation,
                "unknown",
            );
        }
    };
    Json(ToolEnvelope {
        ok: true,
        items: vec![item],
        provenance: Vec::new(),
        budget: None,
        gaps: Vec::new(),
        health: json!({"projection":"ready"}),
        warnings: Vec::new(),
        effect_state: None,
        manifest: None,
    })
    .into_response()
}

fn error_envelope(message: &str, mutation: bool, effect: &str) -> ToolEnvelope {
    ToolEnvelope {
        ok: false,
        items: vec![json!({"error":message})],
        provenance: Vec::new(),
        budget: None,
        gaps: Vec::new(),
        health: json!({"projection":"unavailable"}),
        warnings: Vec::new(),
        effect_state: mutation.then(|| effect.to_owned()),
        manifest: None,
    }
}

fn failure(status: StatusCode, message: &str, mutation: bool, effect: &str) -> Response {
    (status, Json(error_envelope(message, mutation, effect))).into_response()
}

async fn console_index(State(state): State<ConsoleState>) -> Response {
    console_response(&state, "index.html").await
}

async fn console_asset(State(state): State<ConsoleState>, Path(file): Path<String>) -> Response {
    console_response(&state, &file).await
}

async fn console_response(state: &ConsoleState, name: &str) -> Response {
    if state.role != ListenerRole::Actor {
        return StatusCode::FORBIDDEN.into_response();
    }
    let (Some(directory), Some(content_type)) =
        (state.directory.as_ref(), console_asset_name(name))
    else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let path = directory.join(name);
    let Ok(Some(bytes)) = tokio::task::spawn_blocking(move || console_bytes(&path)).await else {
        return StatusCode::NOT_FOUND.into_response();
    };
    (
        [
            (axum::http::header::CONTENT_TYPE, content_type),
            (axum::http::header::CACHE_CONTROL, "no-store"),
        ],
        bytes,
    )
        .into_response()
}

fn console_asset_name(name: &str) -> Option<&'static str> {
    let (stem, extension) = name.rsplit_once('.')?;
    if stem.is_empty() || stem.len() > 64 {
        return None;
    }
    let mut bytes = stem.bytes();
    if !bytes.next()?.is_ascii_lowercase() {
        return None;
    }
    if !bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-') {
        return None;
    }
    CONSOLE_ASSET_TYPES
        .iter()
        .find(|(suffix, _)| *suffix == extension)
        .map(|(_, content_type)| *content_type)
}

fn console_bytes(path: &FilePath) -> Option<Vec<u8>> {
    let metadata = std::fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAXIMUM_CONSOLE_ASSET_BYTES {
        return None;
    }
    std::fs::read(path).ok()
}
