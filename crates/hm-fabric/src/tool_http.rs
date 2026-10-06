use crate::{
    egress::{
        EgressError, EgressHttpClient, EgressMethod, EgressPolicy, EgressReceipt, EgressRequest,
    },
    role_dispatch::{RoleWireRequest, handoff},
    role_store::{
        CatalogReceipt, DispatchTicket, DurableRoleRegistry, RoleDescriptor, RoleGrants, RoleKind,
        RoleOutput, SourceFence, WorkRecord, WorkState,
    },
    roles::{RoleVersion, RunTerminal},
    routing::{Binding, ModuleManifest, RouteCall, Router},
    secret_handles::{
        SecretError, SecretGrant, SecretHandle, SecretHandleService, SecretProviderResponse,
    },
    supervisor::{ProcessSpec, Supervisor},
    transport::{Credentials, Frame, FrameKind, Limits, ReplayGuard, UnixTransport},
};
use ed25519_dalek::{SigningKey, VerifyingKey};
use hm_context::types::{Authority, Scope, digest_bytes, validate_id};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::net::UnixListener;
pub(crate) const HTTP_CONFIG: &str = "HM_FABRIC_HTTP_TOOL_CONFIG";
static SEQUENCE: AtomicU64 = AtomicU64::new(1);
type Result<T> = std::result::Result<T, HttpToolError>;
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum HttpToolError {
    #[error("HTTP tool denied")]
    Denied,
    #[error("invalid HTTP tool request")]
    Invalid,
    #[error("HTTP tool transport or storage unavailable")]
    Unavailable,
    #[error("HTTP tool effect unknown; replay refused")]
    Uncertain,
}
fn unavailable<T>(_error: T) -> HttpToolError {
    HttpToolError::Unavailable
}
fn now_ns() -> Result<i64> {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(unavailable)?
            .as_nanos(),
    )
    .map_err(unavailable)
}
fn policy_digest(policy: &EgressPolicy) -> Result<String> {
    Ok(digest_bytes(
        &serde_json::to_vec(policy).map_err(unavailable)?,
    ))
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HttpToolDeclaration {
    pub id: String,
    pub semantic_version: String,
    pub policy_digest: String,
    pub max_input_bytes: usize,
    pub max_output_bytes: usize,
    pub timeout_ms: u64,
}
impl HttpToolDeclaration {
    pub fn for_policy(
        id: impl Into<String>,
        semantic_version: impl Into<String>,
        policy: &EgressPolicy,
        max_input_bytes: usize,
        max_output_bytes: usize,
        timeout_ms: u64,
    ) -> Result<Self> {
        policy.validate().map_err(|_| HttpToolError::Invalid)?;
        let value = Self {
            id: id.into(),
            semantic_version: semantic_version.into(),
            policy_digest: policy_digest(policy)?,
            max_input_bytes,
            max_output_bytes,
            timeout_ms,
        };
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> Result<()> {
        validate_id(&self.id).map_err(|_| HttpToolError::Invalid)?;
        if self.policy_digest.len() != 64
            || !self
                .policy_digest
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            || self.max_input_bytes == 0
            || self.max_input_bytes > 65536
            || self.max_output_bytes == 0
            || self.max_output_bytes > 65536
            || self.timeout_ms == 0
            || self.timeout_ms > 30000
        {
            return Err(HttpToolError::Invalid);
        }
        Ok(())
    }
    pub fn descriptor(&self) -> Result<RoleDescriptor> {
        self.validate()?;
        Ok(RoleDescriptor { id:self.id.clone(),kind:RoleKind::Tool,pin:RoleVersion::new(b"HyperMind native HTTP tool v1: mandatory authenticated host egress broker; exact approved request and policy; scoped header handles; no redirects; original safe response; uncertain writes never replay",&serde_json::to_vec(self).map_err(unavailable)?),semantic_version:self.semantic_version.clone(),capabilities:BTreeMap::from([("invoke".into(),1)]),grants:RoleGrants { operations:BTreeSet::from(["invoke".into()]),authorities:vec![Authority::ToolObserved],hooks:BTreeSet::from(["tool_dispatch".into()]),max_input_tokens:65536,max_output_tokens:65536,may_reorder:false },transform:None })
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HttpToolRequest {
    pub url: String,
    pub method: EgressMethod,
    pub headers: Vec<(String, Vec<u8>)>,
    pub body: Vec<u8>,
}
impl HttpToolRequest {
    fn validate(&self, declaration: &HttpToolDeclaration) -> Result<()> {
        if serde_json::to_vec(self).map_err(unavailable)?.len() > declaration.max_input_bytes
            || self.headers.len() > 64
            || self.headers.iter().any(|(name, _)| {
                matches!(
                    name.to_ascii_lowercase().as_str(),
                    "authorization" | "proxy-authorization" | "cookie" | "x-api-key"
                )
            })
        {
            return Err(HttpToolError::Invalid);
        }
        Ok(())
    }
    fn egress(&self) -> EgressRequest {
        EgressRequest {
            url: self.url.clone(),
            method: self.method,
            headers: self.headers.clone(),
            body: self.body.clone(),
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HttpToolOutcome {
    pub status: u16,
    pub original_body: Vec<u8>,
    pub body_digest: String,
    pub egress: EgressReceipt,
}
#[derive(Clone)]
pub struct HttpToolAuthority {
    client: Arc<EgressHttpClient>,
    secrets: Arc<SecretHandleService>,
    credential: Option<(SecretHandle, String)>,
}
impl HttpToolAuthority {
    pub fn new(
        policy: EgressPolicy,
        secrets: Arc<SecretHandleService>,
        credential: Option<(SecretHandle, String)>,
        owner: &str,
    ) -> Result<Self> {
        if owner != policy.scope.owner_id {
            return Err(HttpToolError::Denied);
        }
        if credential.as_ref().is_some_and(|(_, header)| {
            reqwest::header::HeaderName::from_bytes(header.as_bytes()).is_err()
                || *header != header.to_ascii_lowercase()
        }) {
            return Err(HttpToolError::Invalid);
        }
        Ok(Self {
            client: Arc::new(EgressHttpClient::new(policy).map_err(|_| HttpToolError::Invalid)?),
            secrets,
            credential,
        })
    }
    pub fn policy(&self) -> &EgressPolicy {
        self.client.policy()
    }
    pub async fn encode_request(
        &self,
        request: &HttpToolRequest,
        declaration: &HttpToolDeclaration,
    ) -> Result<Vec<u8>> {
        request.validate(declaration)?;
        self.secrets
            .validate_public_bytes(request.url.as_bytes())
            .await
            .map_err(|_| HttpToolError::Denied)?;
        self.secrets
            .validate_public_bytes(&request.body)
            .await
            .map_err(|_| HttpToolError::Denied)?;
        for (name, value) in &request.headers {
            self.secrets
                .validate_public_bytes(name.as_bytes())
                .await
                .map_err(|_| HttpToolError::Denied)?;
            self.secrets
                .validate_public_bytes(value)
                .await
                .map_err(|_| HttpToolError::Denied)?;
        }
        let encoded = serde_json::to_vec(request).map_err(unavailable)?;
        self.secrets
            .validate_public_bytes(&encoded)
            .await
            .map_err(|_| HttpToolError::Denied)?;
        Ok(encoded)
    }
    async fn dispatch(
        &self,
        principal: &str,
        revision: u64,
        operation: &str,
        request: &HttpToolRequest,
    ) -> Result<SecretProviderResponse> {
        let mut ids = Vec::new();
        let scope = &self.client.policy().scope;
        let egress = request.egress();
        if let Some((handle, header)) = &self.credential {
            let route = self
                .client
                .authorize(&request.url, request.method)
                .await
                .map_err(|_| HttpToolError::Denied)?;
            let id = format!("http-{}", digest_bytes(operation.as_bytes()));
            self.secrets
                .grant(
                    scope,
                    &scope.owner_id,
                    SecretGrant {
                        id: id.clone(),
                        scope: scope.clone(),
                        principal: principal.into(),
                        principal_revision: revision,
                        handle: handle.clone(),
                        operation_id: operation.into(),
                        request_digest: String::new(),
                        policy_digest: String::new(),
                        route_id: route.route_id,
                        header: header.clone(),
                        expires_unix_ms: (now_ns()? as u64) / 1000000
                            + self.client.policy().timeout_ms,
                        remaining_uses: 1,
                    },
                    &self.client,
                    &egress,
                )
                .await
                .map_err(|_| HttpToolError::Denied)?;
            ids.push(id);
        }
        self.secrets
            .dispatch_provider_response(
                &self.client,
                scope,
                principal,
                revision,
                operation,
                egress,
                &ids,
            )
            .await
            .map_err(|error| match error {
                SecretError::Egress(EgressError::OutcomeUncertain) => HttpToolError::Uncertain,
                _ => HttpToolError::Denied,
            })
    }
}
#[derive(Clone)]
pub struct HttpToolWorkerConfig {
    pub root: PathBuf,
    pub registry_path: PathBuf,
    pub scope: Scope,
    pub host_key: [u8; 32],
    pub worker_key: [u8; 32],
    pub declaration: HttpToolDeclaration,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Configuration {
    socket: PathBuf,
    registry_path: PathBuf,
    scope: Scope,
    signing_key: [u8; 32],
    peer_key: [u8; 32],
    declaration: HttpToolDeclaration,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registration {
    module: String,
    launch: String,
    generation: u64,
    pid: u32,
    descriptor: RoleDescriptor,
    manifest: ModuleManifest,
}
pub struct HttpToolWorkerSession {
    supervisor: Supervisor,
    router: Router,
    binding: Binding,
    transport: UnixTransport,
    socket: PathBuf,
    scope: Scope,
    declaration: HttpToolDeclaration,
    authority: HttpToolAuthority,
    principal: String,
    revision: u64,
    in_flight: Option<DispatchTicket>,
    task: Option<tokio::task::JoinHandle<Result<SecretProviderResponse>>>,
}
impl HttpToolWorkerSession {
    pub async fn launch(
        mut spec: ProcessSpec,
        config: HttpToolWorkerConfig,
        registry: &DurableRoleRegistry,
        limits: Limits,
        authority: HttpToolAuthority,
    ) -> Result<Self> {
        if registry.scope() != &config.scope
            || authority.policy().scope != config.scope
            || policy_digest(authority.policy())? != config.declaration.policy_digest
            || authority.policy().timeout_ms > config.declaration.timeout_ms
            || authority.policy().max_response_bytes > config.declaration.max_output_bytes
        {
            return Err(HttpToolError::Denied);
        }
        authority
            .secrets
            .validate_public_bytes(&serde_json::to_vec(&config.declaration).map_err(unavailable)?)
            .await
            .map_err(|_| HttpToolError::Denied)?;
        authority
            .secrets
            .validate_public_bytes(&serde_json::to_vec(authority.policy()).map_err(unavailable)?)
            .await
            .map_err(|_| HttpToolError::Denied)?;
        authority
            .secrets
            .validate_public_bytes(
                &serde_json::to_vec(&(&spec.args, &spec.env)).map_err(unavailable)?,
            )
            .await
            .map_err(|_| HttpToolError::Denied)?;
        let descriptor = config.declaration.descriptor()?;
        let catalog = registry
            .catalog(&descriptor.id)
            .map_err(unavailable)?
            .ok_or(HttpToolError::Denied)?;
        if catalog.descriptor != descriptor
            || catalog.withdrawal.is_some()
            || spec.env.contains_key(HTTP_CONFIG)
            || spec.env.contains_key("HM_FABRIC_TOOL_CONFIG")
        {
            return Err(HttpToolError::Denied);
        }
        let socket = std::fs::canonicalize(&config.root)
            .map_err(unavailable)?
            .join(format!(
                "http-tool-{}-{}.sock",
                std::process::id(),
                SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
        let listener = UnixListener::bind(&socket).map_err(unavailable)?;
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600))
            .map_err(unavailable)?;
        let configuration = Configuration {
            socket: socket.clone(),
            registry_path: std::fs::canonicalize(&config.registry_path).map_err(unavailable)?,
            scope: config.scope.clone(),
            signing_key: config.worker_key,
            peer_key: SigningKey::from_bytes(&config.host_key)
                .verifying_key()
                .to_bytes(),
            declaration: config.declaration.clone(),
        };
        authority
            .secrets
            .validate_public_bytes(&serde_json::to_vec(&configuration).map_err(unavailable)?)
            .await
            .map_err(|_| HttpToolError::Denied)?;
        spec.env.insert(
            HTTP_CONFIG.into(),
            serde_json::to_string(&configuration).map_err(unavailable)?,
        );
        let mut supervisor = Supervisor::new();
        let launched = supervisor.start(spec).map_err(unavailable)?;
        let accepted = async {
            let (stream, _) = tokio::time::timeout(limits.io_timeout, listener.accept())
                .await
                .map_err(unavailable)?
                .map_err(unavailable)?;
            let mut transport = UnixTransport::accept(
                stream,
                Credentials {
                    scope: config.scope.clone(),
                    signing_key: SigningKey::from_bytes(&config.host_key),
                    peer_key: SigningKey::from_bytes(&config.worker_key).verifying_key(),
                },
                ReplayGuard::default(),
                limits,
            )
            .await
            .map_err(unavailable)?;
            let frame = transport.receive().await.map_err(unavailable)?;
            let registration: Registration =
                serde_json::from_slice(&frame.payload).map_err(|_| HttpToolError::Invalid)?;
            if frame.kind != FrameKind::Request
                || frame.correlation_id != "registration"
                || registration.module != launched.module_id
                || registration.launch != launched.launch_id
                || registration.generation != launched.generation
                || registration.pid != launched.pid
                || registration.descriptor != descriptor
                || registration.manifest.module_id != launched.module_id
                || registration.manifest.capabilities != descriptor.capabilities
                || registration.manifest.max_calls != 1
            {
                return Err(HttpToolError::Denied);
            }
            let mut router = Router::new();
            router
                .authorize_launch(
                    transport.identity(),
                    &launched.module_id,
                    &launched.launch_id,
                    launched.generation,
                )
                .map_err(unavailable)?;
            let binding = router
                .register(
                    transport.identity(),
                    registration.manifest,
                    &launched.launch_id,
                    launched.generation,
                )
                .map_err(unavailable)?;
            let principal = format!(
                "http-tool-{}",
                digest_bytes(
                    &serde_json::to_vec(&(
                        transport.identity().scope(),
                        transport.identity().peer_key(),
                        transport.identity().session_id(),
                        &launched.module_id,
                        &launched.launch_id,
                        launched.generation
                    ))
                    .map_err(unavailable)?
                )
            );
            authority
                .secrets
                .set_principal(
                    &config.scope,
                    &config.scope.owner_id,
                    &principal,
                    Some(launched.generation),
                )
                .await
                .map_err(|_| HttpToolError::Denied)?;
            let mut ack = Frame::request("registration", Vec::new());
            ack.kind = FrameKind::Response;
            transport.send(ack).await.map_err(unavailable)?;
            Ok((transport, router, binding, principal))
        }
        .await;
        match accepted {
            Ok((transport, router, binding, principal)) => Ok(Self {
                supervisor,
                router,
                binding,
                transport,
                socket,
                scope: config.scope,
                declaration: config.declaration,
                authority,
                principal,
                revision: launched.generation,
                in_flight: None,
                task: None,
            }),
            Err(error) => {
                let _ = supervisor.shutdown();
                let _ = std::fs::remove_file(socket);
                Err(error)
            }
        }
    }
    pub fn worker_pid(&self) -> Option<u32> {
        self.supervisor.active().map(|worker| worker.identity.pid)
    }
    pub fn stderr_tail(&self) -> Vec<u8> {
        self.supervisor
            .active()
            .map_or(Vec::new(), |worker| worker.stderr_tail())
    }
    fn approved(
        &self,
        registry: &DurableRoleRegistry,
        id: &str,
        source: &SourceFence,
    ) -> Result<WorkRecord> {
        if registry.scope() != &self.scope {
            return Err(HttpToolError::Denied);
        }
        let record = registry
            .get(id)
            .map_err(unavailable)?
            .ok_or(HttpToolError::Denied)?;
        let catalog = registry
            .catalog(&record.work.role_id)
            .map_err(unavailable)?
            .ok_or(HttpToolError::Denied)?;
        if record.state != WorkState::Approved
            || record.work.source != *source
            || record.work.deadline_ns <= now_ns()?
            || record.descriptor != self.declaration.descriptor()?
            || catalog.descriptor != record.descriptor
            || catalog.withdrawal.is_some()
            || record.approval_revision != Some(catalog.revision)
            || record.work.operation != "invoke"
            || record.work.hook != "tool_dispatch"
        {
            return Err(if record.state == WorkState::Uncertain {
                HttpToolError::Uncertain
            } else {
                HttpToolError::Denied
            });
        }
        Ok(record)
    }
    pub async fn submit(
        &mut self,
        registry: &mut DurableRoleRegistry,
        id: &str,
        source: &SourceFence,
    ) -> Result<DispatchTicket> {
        if self.in_flight.is_some() {
            return Err(HttpToolError::Denied);
        }
        let record = self.approved(registry, id, source)?;
        let request: HttpToolRequest =
            serde_json::from_slice(&record.work.payload).map_err(|_| HttpToolError::Invalid)?;
        request.validate(&self.declaration)?;
        if self
            .authority
            .encode_request(&request, &self.declaration)
            .await?
            != record.work.payload
        {
            return Err(HttpToolError::Denied);
        }
        self.authority
            .client
            .authorize(&request.url, request.method)
            .await
            .map_err(|_| HttpToolError::Denied)?;
        self.router
            .enqueue(
                self.transport.identity(),
                &self.binding,
                RouteCall {
                    id: id.into(),
                    caller: "owner".into(),
                    capability: "invoke".into(),
                    capability_version: 1,
                    deadline_unix_ms: Some(record.work.deadline_ns as u64 / 1000000),
                    payload: record.work.payload.clone(),
                },
                now_ns()? as u64 / 1000000,
            )
            .map_err(unavailable)?;
        if self
            .router
            .dispatch(&self.binding, now_ns()? as u64 / 1000000)
            .map_err(unavailable)?
            .0
            .is_none()
        {
            return Err(HttpToolError::Denied);
        }
        let ticket = handoff(registry, &mut self.transport, id, source, now_ns()?)
            .await
            .map_err(unavailable)?;
        self.in_flight = Some(ticket.clone());
        let admitted = async {
            let frame = self.transport.receive().await.map_err(unavailable)?;
            let broker: RoleWireRequest =
                serde_json::from_slice(&frame.payload).map_err(|_| HttpToolError::Invalid)?;
            let current = registry
                .get(id)
                .map_err(unavailable)?
                .ok_or(HttpToolError::Denied)?;
            let catalog = registry
                .catalog(&record.work.role_id)
                .map_err(unavailable)?
                .ok_or(HttpToolError::Denied)?;
            if frame.kind != FrameKind::Request
                || frame.correlation_id != id
                || broker.ticket != ticket
                || broker.work != record.work
                || current.state != WorkState::Dispatched
                || current.work != record.work
                || current.ticket.as_ref() != Some(&ticket)
                || current.work.deadline_ns <= now_ns()?
                || catalog.withdrawal.is_some()
                || catalog.descriptor != record.descriptor
                || current.approval_revision != Some(catalog.revision)
            {
                return Err(HttpToolError::Denied);
            }
            let authority = self.authority.clone();
            let principal = self.principal.clone();
            let revision = self.revision;
            let operation = format!(
                "http-request-{}",
                digest_bytes(&serde_json::to_vec(&ticket).map_err(unavailable)?)
            );
            self.task = Some(tokio::spawn(async move {
                authority
                    .dispatch(&principal, revision, &operation, &request)
                    .await
            }));
            Ok(())
        }
        .await;
        if let Err(error) = admitted {
            let _ = registry.mark_uncertain(&ticket);
            return Err(error);
        }
        Ok(ticket)
    }
    pub async fn receive(
        &mut self,
        registry: &mut DurableRoleRegistry,
        source: &SourceFence,
    ) -> Result<WorkRecord> {
        let ticket = self.in_flight.clone().ok_or(HttpToolError::Denied)?;
        let task = self.task.take().ok_or(HttpToolError::Uncertain)?;
        let result = task.await;
        let response = match result {
            Ok(Ok(response)) => response,
            _ => {
                registry.mark_uncertain(&ticket).map_err(unavailable)?;
                let mut frame = Frame::request(&ticket.id, Vec::new());
                frame.kind = FrameKind::Cancel;
                let _ = self.transport.send(frame).await;
                self.in_flight = None;
                let _ = self.router.complete(&self.binding, &ticket.id);
                return Err(HttpToolError::Uncertain);
            }
        };
        if response.original_bytes.len() > self.declaration.max_output_bytes {
            let _ = registry.mark_uncertain(&ticket);
            return Err(HttpToolError::Uncertain);
        }
        let outcome = HttpToolOutcome {
            status: response.receipt.status,
            body_digest: digest_bytes(&response.original_bytes),
            original_body: response.original_bytes,
            egress: response.receipt.egress,
        };
        let payload = serde_json::to_vec(&outcome).map_err(unavailable)?;
        let output = RoleOutput {
            terminal: RunTerminal::Completed {
                result_digest: digest_bytes(&payload),
            },
            payload,
            blocks: Vec::new(),
            grants: self.declaration.descriptor()?.grants,
        };
        let mut frame = Frame::request(
            &ticket.id,
            serde_json::to_vec(&output).map_err(unavailable)?,
        );
        frame.kind = FrameKind::Response;
        if self.transport.send(frame).await.is_err() {
            let _ = registry.mark_uncertain(&ticket);
            return Err(HttpToolError::Uncertain);
        }
        let received = self.transport.receive().await;
        let finished = async {
            let frame = received.map_err(unavailable)?;
            if frame.kind != FrameKind::Response || frame.correlation_id != ticket.id {
                return Err(HttpToolError::Uncertain);
            }
            let observed: RoleOutput =
                serde_json::from_slice(&frame.payload).map_err(|_| HttpToolError::Invalid)?;
            if observed != output {
                return Err(HttpToolError::Uncertain);
            }
            registry
                .complete(&ticket, source, observed, now_ns()?)
                .map_err(unavailable)
        }
        .await;
        if finished.is_err() {
            let _ = registry.mark_uncertain(&ticket);
        }
        let _ = self.router.complete(&self.binding, &ticket.id);
        self.in_flight = None;
        finished
    }
    pub async fn execute(
        &mut self,
        registry: &mut DurableRoleRegistry,
        id: &str,
        source: &SourceFence,
    ) -> Result<WorkRecord> {
        if registry.scope() != &self.scope {
            return Err(HttpToolError::Denied);
        }
        if let Some(record) = registry.get(id).map_err(unavailable)? {
            if record.state == WorkState::Terminal
                && record.work.source == *source
                && record.descriptor == self.declaration.descriptor()?
                && record.output.is_some()
            {
                return Ok(record);
            }
        }
        self.submit(registry, id, source).await?;
        self.receive(registry, source).await
    }
    pub async fn cancel(&mut self, registry: &mut DurableRoleRegistry) -> Result<()> {
        let ticket = self.in_flight.clone().ok_or(HttpToolError::Denied)?;
        registry.mark_uncertain(&ticket).map_err(unavailable)?;
        if let Some(task) = self.task.take() {
            task.abort();
        }
        let mut frame = Frame::request(&ticket.id, Vec::new());
        frame.kind = FrameKind::Cancel;
        self.transport.send(frame).await.map_err(unavailable)?;
        self.in_flight = None;
        let _ = self.router.complete(&self.binding, &ticket.id);
        Ok(())
    }
    pub fn kill_worker(&mut self) -> Result<()> {
        if let Some(task) = self.task.take() {
            task.abort();
        }
        self.supervisor.shutdown().map_err(unavailable)
    }
    pub async fn shutdown(&mut self) -> Result<()> {
        if let Some(task) = self.task.take() {
            task.abort();
        }
        let mut frame = Frame::request("shutdown", Vec::new());
        frame.kind = FrameKind::Cancel;
        let _ = self.transport.send(frame).await;
        self.supervisor.shutdown().map_err(unavailable)
    }
}
impl Drop for HttpToolWorkerSession {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
        let _ = self.supervisor.shutdown();
        let _ = std::fs::remove_file(&self.socket);
    }
}
#[derive(Deserialize)]
struct Snapshot {
    scope: Scope,
    catalog: BTreeMap<String, CatalogReceipt>,
    work: BTreeMap<String, WorkRecord>,
}
fn worker_approved(config: &Configuration, request: &RoleWireRequest) -> Result<()> {
    let connection = rusqlite::Connection::open_with_flags(
        &config.registry_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(unavailable)?;
    let transaction = connection.unchecked_transaction().map_err(unavailable)?;
    let bytes: Vec<u8> = transaction
        .query_row("SELECT data FROM role_state WHERE id=1", [], |row| {
            row.get(0)
        })
        .map_err(unavailable)?;
    let epoch: i64 = transaction
        .query_row("SELECT epoch FROM hm_store_meta WHERE id=1", [], |row| {
            row.get(0)
        })
        .map_err(unavailable)?;
    let state: Snapshot = serde_json::from_slice(&bytes).map_err(|_| HttpToolError::Invalid)?;
    let work = state
        .work
        .get(&request.work.id)
        .ok_or(HttpToolError::Denied)?;
    let catalog = state
        .catalog
        .get(&request.work.role_id)
        .ok_or(HttpToolError::Denied)?;
    if state.scope != config.scope
        || u64::try_from(epoch).map_err(unavailable)? != request.ticket.writer_epoch
        || work.state != WorkState::Dispatched
        || work.ticket.as_ref() != Some(&request.ticket)
        || work.work != request.work
        || work.descriptor != config.declaration.descriptor()?
        || catalog.descriptor != work.descriptor
        || catalog.withdrawal.is_some()
        || work.approval_revision != Some(catalog.revision)
        || work.work.deadline_ns <= now_ns()?
        || digest_bytes(&serde_json::to_vec(&work.work).map_err(unavailable)?)
            != request.ticket.work_digest
    {
        return Err(HttpToolError::Denied);
    }
    let input: HttpToolRequest =
        serde_json::from_slice(&work.work.payload).map_err(|_| HttpToolError::Invalid)?;
    input.validate(&config.declaration)
}
pub async fn http_tool_worker_from_env() -> Result<()> {
    if std::env::var_os("HM_FABRIC_TOOL_CONFIG").is_some() {
        return Err(HttpToolError::Denied);
    }
    let config: Configuration =
        serde_json::from_str(&std::env::var(HTTP_CONFIG).map_err(unavailable)?)
            .map_err(|_| HttpToolError::Invalid)?;
    config.declaration.validate()?;
    let module = std::env::var("HYPERMIND_MODULE_ID").map_err(unavailable)?;
    let launch = std::env::var("HYPERMIND_LAUNCH_ID").map_err(unavailable)?;
    let generation = std::env::var("HYPERMIND_GENERATION")
        .map_err(unavailable)?
        .parse::<u64>()
        .map_err(unavailable)?;
    let mut transport = UnixTransport::connect(
        &config.socket,
        Credentials {
            scope: config.scope.clone(),
            signing_key: SigningKey::from_bytes(&config.signing_key),
            peer_key: VerifyingKey::from_bytes(&config.peer_key).map_err(unavailable)?,
        },
        ReplayGuard::default(),
        Limits {
            io_timeout: Duration::from_millis(config.declaration.timeout_ms + 5000),
            ..Limits::default()
        },
    )
    .await
    .map_err(unavailable)?;
    let registration = Registration {
        module: module.clone(),
        launch,
        generation,
        pid: std::process::id(),
        descriptor: config.declaration.descriptor()?,
        manifest: ModuleManifest {
            module_id: module,
            protocol_version: 1,
            capabilities: BTreeMap::from([("invoke".into(), 1)]),
            max_calls: 1,
            max_bytes: 512 * 1024,
            queue_calls: 1,
            queue_bytes: 512 * 1024,
        },
    };
    transport
        .send(Frame::request(
            "registration",
            serde_json::to_vec(&registration).map_err(unavailable)?,
        ))
        .await
        .map_err(unavailable)?;
    let ack = transport.receive().await.map_err(unavailable)?;
    if ack.kind != FrameKind::Response || ack.correlation_id != "registration" {
        return Err(HttpToolError::Denied);
    }
    loop {
        let frame = transport.receive().await.map_err(unavailable)?;
        if frame.kind == FrameKind::Cancel && frame.correlation_id == "shutdown" {
            return Ok(());
        }
        if frame.kind != FrameKind::Request {
            return Err(HttpToolError::Denied);
        }
        let request: RoleWireRequest =
            serde_json::from_slice(&frame.payload).map_err(|_| HttpToolError::Invalid)?;
        if frame.correlation_id != request.work.id {
            return Err(HttpToolError::Denied);
        }
        worker_approved(&config, &request)?;
        transport
            .send(Frame::request(
                &request.work.id,
                serde_json::to_vec(&request).map_err(unavailable)?,
            ))
            .await
            .map_err(unavailable)?;
        let response = transport.receive().await.map_err(unavailable)?;
        if response.correlation_id != request.work.id {
            return Err(HttpToolError::Denied);
        }
        if response.kind == FrameKind::Cancel {
            return Ok(());
        }
        if response.kind != FrameKind::Response {
            return Err(HttpToolError::Denied);
        }
        let output: RoleOutput =
            serde_json::from_slice(&response.payload).map_err(|_| HttpToolError::Invalid)?;
        let outcome: HttpToolOutcome =
            serde_json::from_slice(&output.payload).map_err(|_| HttpToolError::Invalid)?;
        if outcome.original_body.len() > config.declaration.max_output_bytes
            || outcome.body_digest != digest_bytes(&outcome.original_body)
            || !matches!(&output.terminal,RunTerminal::Completed{result_digest} if result_digest==&digest_bytes(&output.payload))
            || !output
                .grants
                .tightens(&config.declaration.descriptor()?.grants)
        {
            return Err(HttpToolError::Denied);
        }
        let mut acknowledged = Frame::request(&request.work.id, response.payload);
        acknowledged.kind = FrameKind::Response;
        transport.send(acknowledged).await.map_err(unavailable)?;
    }
}
