use crate::{
    provider_egress::{RunnerProviderGuard, RunnerProviderSession},
    role_dispatch::{RoleWireRequest, handoff},
    role_store::*,
    roles::{RoleVersion, RunTerminal},
    routing::{Binding, ModuleManifest, RouteCall, Router},
    secret_handles::SecretProviderResponse,
    supervisor::{ProcessSpec, Supervisor},
    transport::{Credentials, Frame, FrameKind, Limits, ReplayGuard, UnixTransport},
};
use ed25519_dalek::{SigningKey, VerifyingKey};
use hm_context::{
    provider_continuity::{ProviderProfile, fence_profile},
    types::{Authority, Scope, TokenBudget, digest_bytes},
};
use hm_llm::{
    LlmError, LlmProvider, ModelTier, Pricing, ProviderConfig, StructuredRequest, WireRequest,
    WireResponse, WireTransport,
    ollama::Ollama,
    provider_usage::{ObservationFormat, ObservationMetadata, ProviderUsageSnapshot},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::net::UnixListener;
const CONFIG: &str = "HM_FABRIC_RUNNER_CONFIG";
static SEQUENCE: AtomicU64 = AtomicU64::new(1);
type Result<T> = std::result::Result<T, RunnerError>;
#[derive(Debug, thiserror::Error)]
#[error("runner refused: {0}")]
pub struct RunnerError(pub String);
fn err(e: impl std::fmt::Debug) -> RunnerError {
    RunnerError(format!("{e:?}"))
}
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos()
        .min(i64::MAX as u128) as i64
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunnerDeclaration {
    pub id: String,
    pub semantic_version: String,
    pub endpoint: String,
    pub model_digest: String,
    pub profile: ProviderProfile,
    pub budget: TokenBudget,
    pub system: String,
    pub json_schema: serde_json::Value,
    pub max_prompt_bytes: usize,
    pub max_response_bytes: usize,
    pub timeout_ms: u64,
}
impl RunnerDeclaration {
    fn validate(&self) -> Result<()> {
        hm_context::types::validate_id(&self.id).map_err(err)?;
        if self.json_schema
            != serde_json::json!({"type":"object","properties":{"answer":{"type":"string"}},"required":["answer"],"additionalProperties":false})
        {
            return Err(err("unsupported structured answer schema"));
        }
        let endpoint = reqwest::Url::parse(&self.endpoint).map_err(err)?;
        if endpoint.scheme() != "http"
            || endpoint.host_str() != Some("127.0.0.1")
            || endpoint.port().is_none()
            || endpoint.path() != "/api/chat"
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
            || self.profile.model_id != "qwen2.5:3b"
            || self.profile.tokenizer_id != "ollama-native-counters"
            || self.profile.tokenizer_revision != self.model_digest
            || self.profile.serializer_id != "ollama-structured-chat-v1"
            || self.profile.serializer_revision != "1"
            || !self.profile.capabilities.user
            || !self.profile.capabilities.assistant
            || !self.profile.capabilities.text
            || self.profile.model_revision != self.model_digest
            || self.model_digest.len() != 64
            || !self.model_digest.bytes().all(|b| b.is_ascii_hexdigit())
            || self.system.len() > 8192
            || self.max_prompt_bytes == 0
            || self.max_prompt_bytes > 32768
            || self.max_response_bytes == 0
            || self.max_response_bytes > 65536
            || self.timeout_ms == 0
            || self.timeout_ms > 120000
            || self.budget.reserved_output_tokens == 0
            || self.budget.reserved_output_tokens > 2048
            || self.profile.capabilities.tool
            || self.profile.capabilities.tool_calls
            || self.profile.capabilities.tool_results
            || self.profile.capabilities.opaque
        {
            return Err(err("invalid pinned local model declaration"));
        }
        self.budget.available().map_err(err)?;
        Ok(())
    }
    pub fn descriptor(&self) -> Result<RoleDescriptor> {
        self.validate()?;
        Ok(RoleDescriptor{id:self.id.clone(),kind:RoleKind::Runner,pin:RoleVersion::new(b"HyperMind structured Ollama runner v2: mandatory authenticated host egress broker; scoped secret dispatch; owner-approved canonical reservation; original provider evidence; no settlement",&serde_json::to_vec(self).map_err(err)?),semantic_version:self.semantic_version.clone(),capabilities:BTreeMap::from([("run".into(),1)]),grants:RoleGrants{operations:BTreeSet::from(["run".into()]),authorities:vec![Authority::AssistantGenerated],hooks:BTreeSet::from(["runner_dispatch".into()]),max_input_tokens:self.budget.context_tokens,max_output_tokens:self.budget.reserved_output_tokens,may_reorder:false},transform:None})
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalUsageBinding {
    pub lease: hm_context::maintenance::JobLease,
    pub reservation_id: String,
    pub input_digest: String,
    pub reservation_digest: String,
    pub reserved_tokens: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunnerInput {
    pub prompt: String,
    pub usage: CanonicalUsageBinding,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunnerOutcome {
    pub provider_request_digest: String,
    pub value: serde_json::Value,
    pub original_provider_bytes: Vec<u8>,
    pub observation: ProviderUsageSnapshot,
    pub usage: CanonicalUsageBinding,
    pub source: SourceFence,
    pub profile_fence: String,
    pub request_digest: String,
}
pub struct AuthenticatedRunnerReceipt {
    outcome: RunnerOutcome,
    scope: Scope,
    worker_id: String,
}
impl AuthenticatedRunnerReceipt {
    pub fn outcome(&self) -> &RunnerOutcome {
        &self.outcome
    }
    pub fn scope(&self) -> &Scope {
        &self.scope
    }
    pub fn worker_id(&self) -> &str {
        &self.worker_id
    }
}
pub fn provider_request_digest(d: &RunnerDeclaration, input: &RunnerInput) -> Result<String> {
    d.validate()?;
    let body = serde_json::json!({"model":d.profile.model_id,"messages":[{"role":"system","content":d.system},{"role":"user","content":input.prompt}],"format":d.json_schema,"stream":false,"options":{"num_predict":d.budget.reserved_output_tokens as u32}});
    Ok(digest_bytes(&serde_json::to_vec(&body).map_err(err)?))
}
#[derive(Clone)]
pub struct RunnerWorkerConfig {
    pub root: PathBuf,
    pub registry_path: PathBuf,
    pub scope: Scope,
    pub host_key: [u8; 32],
    pub worker_key: [u8; 32],
    pub declaration: RunnerDeclaration,
}
#[derive(Serialize, Deserialize)]
struct Configuration {
    socket: PathBuf,
    registry_path: PathBuf,
    scope: Scope,
    signing_key: [u8; 32],
    peer_key: [u8; 32],
    declaration: RunnerDeclaration,
}
#[derive(Serialize, Deserialize)]
struct Registration {
    module: String,
    launch: String,
    generation: u64,
    pid: u32,
    descriptor: RoleDescriptor,
    manifest: ModuleManifest,
}
pub struct RunnerWorkerSession {
    supervisor: Supervisor,
    router: Router,
    binding: Binding,
    transport: UnixTransport,
    socket: PathBuf,
    scope: Scope,
    declaration: RunnerDeclaration,
    in_flight: Option<DispatchTicket>,
    provider_guard: RunnerProviderGuard,
    provider_session: Arc<RunnerProviderSession>,
    provider_task: Option<tokio::task::JoinHandle<Result<SecretProviderResponse>>>,
}
impl RunnerWorkerSession {
    pub async fn launch(
        spec: ProcessSpec,
        config: RunnerWorkerConfig,
        registry: &DurableRoleRegistry,
        limits: Limits,
    ) -> Result<Self> {
        let guard = RunnerProviderGuard::local(&config.scope, &config.declaration).map_err(err)?;
        Self::launch_guarded(spec, config, registry, limits, guard).await
    }
    pub async fn launch_guarded(
        mut spec: ProcessSpec,
        config: RunnerWorkerConfig,
        registry: &DurableRoleRegistry,
        limits: Limits,
        provider_guard: RunnerProviderGuard,
    ) -> Result<Self> {
        if provider_guard.policy().scope != config.scope {
            return Err(err("provider scope mismatch"));
        }
        if registry.scope() != &config.scope {
            return Err(err("scope mismatch"));
        }
        let descriptor = config.declaration.descriptor()?;
        let catalog = registry
            .catalog(&descriptor.id)
            .map_err(err)?
            .ok_or_else(|| err("unregistered runner"))?;
        if catalog.descriptor != descriptor || catalog.withdrawal.is_some() {
            return Err(err("runner withdrawn or changed"));
        }
        let socket = std::fs::canonicalize(&config.root)
            .map_err(err)?
            .join(format!(
                "runner-{}-{}.sock",
                std::process::id(),
                SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
        let listener = UnixListener::bind(&socket).map_err(err)?;
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600)).map_err(err)?;
        if spec.env.contains_key(CONFIG) {
            return Err(err("configuration owner controlled"));
        }
        let conf = Configuration {
            socket: socket.clone(),
            registry_path: std::fs::canonicalize(&config.registry_path).map_err(err)?,
            scope: config.scope.clone(),
            signing_key: config.worker_key,
            peer_key: SigningKey::from_bytes(&config.host_key)
                .verifying_key()
                .to_bytes(),
            declaration: config.declaration.clone(),
        };
        spec.env
            .insert(CONFIG.into(), serde_json::to_string(&conf).map_err(err)?);
        let mut supervisor = Supervisor::new();
        let launch = supervisor.start(spec).map_err(err)?;
        let accepted = async {
            let (stream, _) = tokio::time::timeout(limits.io_timeout, listener.accept())
                .await
                .map_err(err)?
                .map_err(err)?;
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
            .map_err(err)?;
            let frame = transport.receive().await.map_err(err)?;
            let reg: Registration = serde_json::from_slice(&frame.payload).map_err(err)?;
            if frame.kind != FrameKind::Request
                || frame.correlation_id != "registration"
                || reg.module != launch.module_id
                || reg.launch != launch.launch_id
                || reg.generation != launch.generation
                || reg.pid != launch.pid
                || reg.descriptor != descriptor
                || reg.manifest.module_id != launch.module_id
                || reg.manifest.capabilities != descriptor.capabilities
                || reg.manifest.max_calls != 1
            {
                return Err(err("registration pin mismatch"));
            }
            let mut router = Router::new();
            router
                .authorize_launch(
                    transport.identity(),
                    &launch.module_id,
                    &launch.launch_id,
                    launch.generation,
                )
                .map_err(err)?;
            let binding = router
                .register(
                    transport.identity(),
                    reg.manifest,
                    &launch.launch_id,
                    launch.generation,
                )
                .map_err(err)?;
            let mut ack = Frame::request("registration", vec![]);
            ack.kind = FrameKind::Response;
            transport.send(ack).await.map_err(err)?;
            Ok::<_, RunnerError>((transport, router, binding))
        }
        .await;
        match accepted {
            Ok((transport, router, binding)) => {
                let provider_session = match provider_guard
                    .session(transport.identity(), &launch.module_id, launch.generation)
                    .await
                {
                    Ok(session) => Arc::new(session),
                    Err(error) => {
                        let _ = supervisor.shutdown();
                        let _ = std::fs::remove_file(&socket);
                        return Err(err(error));
                    }
                };
                Ok(Self {
                    supervisor,
                    transport,
                    router,
                    binding,
                    socket,
                    scope: config.scope,
                    declaration: config.declaration,
                    in_flight: None,
                    provider_guard,
                    provider_session,
                    provider_task: None,
                })
            }
            Err(e) => {
                let _ = supervisor.shutdown();
                let _ = std::fs::remove_file(socket);
                Err(e)
            }
        }
    }
    pub async fn submit(
        &mut self,
        registry: &mut DurableRoleRegistry,
        id: &str,
        source: &SourceFence,
    ) -> Result<DispatchTicket> {
        if self.in_flight.is_some() || registry.scope() != &self.scope {
            return Err(err("busy or scope mismatch"));
        }
        let record = registry
            .get(id)
            .map_err(err)?
            .ok_or_else(|| err("missing work"))?;
        if record.state != WorkState::Approved
            || record.descriptor != self.declaration.descriptor()?
        {
            return Err(err("unapproved runner work"));
        }
        validate_input(&self.declaration, &record.work)?;
        let catalog = registry
            .catalog(&record.work.role_id)
            .map_err(err)?
            .ok_or_else(|| err("runner catalog missing"))?;
        if record.work.source != *source
            || record.work.deadline_ns <= now()
            || catalog.withdrawal.is_some()
            || catalog.descriptor != record.descriptor
            || record.approval_revision != Some(catalog.revision)
        {
            return Err(err("current runner approval refused"));
        }
        self.provider_guard
            .check_public(&record.work.payload)
            .await
            .map_err(err)?;
        verify_model(
            &self.declaration,
            &self.provider_guard,
            &self.provider_session,
            &format!(
                "catalog-{}-{}",
                id,
                SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ),
        )
        .await?;
        self.router
            .enqueue(
                self.transport.identity(),
                &self.binding,
                RouteCall {
                    id: id.into(),
                    caller: "owner".into(),
                    capability: "run".into(),
                    capability_version: 1,
                    deadline_unix_ms: Some(record.work.deadline_ns as u64 / 1_000_000),
                    payload: record.work.payload.clone(),
                },
                now() as u64 / 1_000_000,
            )
            .map_err(err)?;
        let (call, _) = self
            .router
            .dispatch(&self.binding, now() as u64 / 1_000_000)
            .map_err(err)?;
        if call.is_none() {
            return Err(err("no dispatch credit"));
        }
        let ticket = match handoff(registry, &mut self.transport, id, source, now()).await {
            Ok(t) => t,
            Err(e) => {
                let _ = self.router.complete(&self.binding, id);
                return Err(err(e));
            }
        };
        self.in_flight = Some(ticket.clone());
        let started = self.transport.receive().await.map_err(err)?;
        if started.kind != FrameKind::Response
            || started.correlation_id != id
            || started.payload != b"provider_dispatch_started"
        {
            let _ = registry.mark_uncertain(&ticket);
            return Err(err("provider start acknowledgement missing"));
        }
        let requested = self.transport.receive().await.map_err(err)?;
        let broker_request: WireRequest = serde_json::from_slice(&requested.payload)
            .map_err(|_| err("invalid provider broker request"))?;
        let input = validate_input(&self.declaration, &record.work)?;
        if requested.kind != FrameKind::Request
            || requested.correlation_id != id
            || broker_request.method != "POST"
            || broker_request.url != self.declaration.endpoint
            || broker_request.headers
                != BTreeMap::from([("content-type".into(), "application/json".into())])
            || digest_bytes(&serde_json::to_vec(&broker_request.body).map_err(err)?)
                != provider_request_digest(&self.declaration, &input)?
        {
            let _ = registry.mark_uncertain(&ticket);
            return Err(err("provider broker request fence mismatch"));
        }
        let current = registry
            .get(id)
            .map_err(err)?
            .ok_or_else(|| err("missing dispatched work"))?;
        let catalog = registry
            .catalog(&record.work.role_id)
            .map_err(err)?
            .ok_or_else(|| err("missing runner catalog"))?;
        if current.state != WorkState::Dispatched
            || current.work != record.work
            || current.ticket.as_ref() != Some(&ticket)
            || catalog.withdrawal.is_some()
            || catalog.descriptor != record.descriptor
            || current.approval_revision != Some(catalog.revision)
            || current.work.deadline_ns <= now()
        {
            let _ = registry.mark_uncertain(&ticket);
            return Err(err("current provider dispatch approval refused"));
        }
        let guard = self.provider_guard.clone();
        let session = self.provider_session.clone();
        let operation = format!(
            "provider-{}",
            digest_bytes(&serde_json::to_vec(&ticket).map_err(err)?)
        );
        self.provider_task = Some(tokio::spawn(async move {
            guard
                .dispatch(&session, &operation, &broker_request)
                .await
                .map_err(err)
        }));
        Ok(ticket)
    }
    pub async fn receive(
        &mut self,
        registry: &mut DurableRoleRegistry,
        source: &SourceFence,
    ) -> Result<WorkRecord> {
        let ticket = self
            .in_flight
            .clone()
            .ok_or_else(|| err("no in-flight run"))?;
        if let Some(task) = self.provider_task.take() {
            let dispatched = task.await;
            let mut response = Frame::request(&ticket.id, Vec::new());
            match dispatched {
                Ok(Ok(provider)) => {
                    response.kind = FrameKind::Response;
                    response.payload = serde_json::to_vec(&BrokerResponse {
                        status: provider.receipt.status,
                        original_bytes: provider.original_bytes,
                    })
                    .map_err(err)?;
                }
                _ => {
                    registry.mark_uncertain(&ticket).map_err(err)?;
                    response.kind = FrameKind::Cancel;
                }
            }
            if self.transport.send(response).await.is_err() {
                let _ = registry.mark_uncertain(&ticket);
                return Err(err("provider broker delivery uncertain"));
            }
        }
        let received = self.transport.receive().await;
        let result = (|| {
            let frame = received.map_err(err)?;
            if frame.kind != FrameKind::Response || frame.correlation_id != ticket.id {
                return Err(err("response fence mismatch"));
            }
            let output: RoleOutput = serde_json::from_slice(&frame.payload).map_err(err)?;
            if let RunTerminal::Completed { .. } = &output.terminal {
                let result: RunnerOutcome = serde_json::from_slice(&output.payload).map_err(err)?;
                let work = registry
                    .get(&ticket.id)
                    .map_err(err)?
                    .ok_or_else(|| err("missing work"))?;
                let input = validate_input(&self.declaration, &work.work)?;
                let parsed = observation(&result.original_provider_bytes, &ticket.id)?;
                if result.provider_request_digest
                    != provider_request_digest(&self.declaration, &input)?
                    || result.usage != input.usage
                    || result.request_digest != ticket.work_digest
                    || result.source != work.work.source
                    || result.profile_fence != profile(&self.declaration, &work.work.source)?
                    || result.observation.original_bytes != result.original_provider_bytes
                    || parsed.tokens != result.observation.tokens
                    || result.original_provider_bytes.len() > self.declaration.max_response_bytes
                {
                    return Err(err("provider evidence binding mismatch"));
                }
                measured(&self.declaration, &input, &parsed)?;
            }
            if matches!(output.terminal, RunTerminal::Cancelled) {
                registry.mark_uncertain(&ticket).map_err(err)?;
            }
            registry
                .complete(&ticket, source, output, now())
                .map_err(err)
        })();
        if result.is_err() {
            let _ = registry.mark_uncertain(&ticket);
        }
        let _ = self.router.complete(&self.binding, &ticket.id);
        self.in_flight = None;
        result
    }
    pub async fn receive_with_receipt(
        &mut self,
        registry: &mut DurableRoleRegistry,
        source: &SourceFence,
    ) -> Result<(WorkRecord, AuthenticatedRunnerReceipt)> {
        let worker_id = self
            .supervisor
            .active()
            .ok_or_else(|| err("worker unavailable"))?
            .identity
            .module_id
            .clone();
        let record = self.receive(registry, source).await?;
        let output = record
            .output
            .as_ref()
            .ok_or_else(|| err("missing output"))?;
        if !matches!(output.terminal, RunTerminal::Completed { .. }) {
            return Err(err("provider usage remains unknown"));
        }
        let outcome = serde_json::from_slice(&output.payload).map_err(err)?;
        let receipt = AuthenticatedRunnerReceipt {
            outcome,
            scope: self.scope.clone(),
            worker_id,
        };
        Ok((record, receipt))
    }
    pub async fn cancel(&mut self, registry: &mut DurableRoleRegistry) -> Result<()> {
        let t = self.in_flight.as_ref().ok_or_else(|| err("no run"))?;
        registry.mark_uncertain(t).map_err(err)?;
        if let Some(task) = self.provider_task.take() {
            task.abort();
        }
        let mut f = Frame::request(&t.id, vec![]);
        f.kind = FrameKind::Cancel;
        self.transport.send(f).await.map_err(err)
    }
    pub fn kill_worker(&mut self, registry: &mut DurableRoleRegistry) -> Result<()> {
        if let Some(t) = &self.in_flight {
            registry.mark_uncertain(t).map_err(err)?;
        }
        if let Some(task) = self.provider_task.take() {
            task.abort();
        }
        self.supervisor.shutdown().map_err(err)
    }
    pub fn shutdown(&mut self) -> Result<()> {
        if let Some(task) = self.provider_task.take() {
            task.abort();
        }
        self.supervisor.shutdown().map_err(err)
    }
}
impl Drop for RunnerWorkerSession {
    fn drop(&mut self) {
        if let Some(task) = self.provider_task.take() {
            task.abort();
        }
        let _ = self.supervisor.shutdown();
        let _ = std::fs::remove_file(&self.socket);
    }
}
fn validate_input(d: &RunnerDeclaration, w: &RoleWork) -> Result<RunnerInput> {
    let input: RunnerInput = serde_json::from_slice(&w.payload).map_err(err)?;
    hm_context::types::validate_id(&input.usage.reservation_id).map_err(err)?;
    if input.prompt.len() > d.max_prompt_bytes
        || input.usage.input_digest != digest_bytes(input.prompt.as_bytes())
        || input.usage.reservation_digest.len() != 64
        || !input
            .usage
            .reservation_digest
            .bytes()
            .all(|b| b.is_ascii_hexdigit())
        || input.usage.reserved_tokens < d.budget.reserved_output_tokens
        || w.operation != "run"
        || w.hook != "runner_dispatch"
    {
        return Err(err("canonical reservation or request mismatch"));
    }
    Ok(input)
}
fn profile(d: &RunnerDeclaration, s: &SourceFence) -> Result<String> {
    Ok(fence_profile(
        &d.profile,
        d.budget,
        &serde_json::to_string(s).map_err(err)?,
    )
    .map_err(err)?
    .digest)
}
fn observation(raw: &[u8], id: &str) -> Result<ProviderUsageSnapshot> {
    ProviderUsageSnapshot::parse(
        raw,
        ObservationMetadata {
            provider_id: "ollama-local".into(),
            source: "runner-original-response".into(),
            evidence_id: id.into(),
            observed_at_ns: now(),
            expires_at_ns: Some(now() + 60_000_000_000),
            format: ObservationFormat::Ollama,
        },
    )
    .map_err(err)
}
fn measured(d: &RunnerDeclaration, input: &RunnerInput, o: &ProviderUsageSnapshot) -> Result<()> {
    let i = o
        .tokens
        .input
        .ok_or_else(|| err("missing actual input count"))?;
    let n = o
        .tokens
        .output
        .ok_or_else(|| err("missing actual output count"))?;
    if n > d.budget.reserved_output_tokens
        || i.saturating_add(n) > input.usage.reserved_tokens
        || i.saturating_add(n) > d.budget.context_tokens
    {
        return Err(err("measured token bounds exceeded"));
    }
    Ok(())
}
#[derive(Deserialize)]
struct Snapshot {
    scope: Scope,
    catalog: BTreeMap<String, CatalogReceipt>,
    work: BTreeMap<String, WorkRecord>,
}
fn approved(c: &Configuration, r: &RoleWireRequest) -> Result<()> {
    let db = rusqlite::Connection::open_with_flags(
        &c.registry_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(err)?;
    let tx = db.unchecked_transaction().map_err(err)?;
    let bytes: Vec<u8> = tx
        .query_row("SELECT data FROM role_state WHERE id=1", [], |r| r.get(0))
        .map_err(err)?;
    let epoch: i64 = tx
        .query_row("SELECT epoch FROM hm_store_meta WHERE id=1", [], |r| {
            r.get(0)
        })
        .map_err(err)?;
    let s: Snapshot = serde_json::from_slice(&bytes).map_err(err)?;
    let w = s
        .work
        .get(&r.work.id)
        .ok_or_else(|| err("missing approval"))?;
    let cat = s
        .catalog
        .get(&r.work.role_id)
        .ok_or_else(|| err("missing catalog"))?;
    let descriptor = c.declaration.descriptor()?;
    if s.scope != c.scope
        || u64::try_from(epoch).map_err(err)? != r.ticket.writer_epoch
        || w.state != WorkState::Dispatched
        || w.work != r.work
        || w.ticket.as_ref() != Some(&r.ticket)
        || w.approval_revision != Some(cat.revision)
        || cat.withdrawal.is_some()
        || cat.descriptor != descriptor
        || w.descriptor != descriptor
        || r.ticket.work_digest != digest_bytes(&serde_json::to_vec(&r.work).map_err(err)?)
        || r.work.deadline_ns <= now()
    {
        return Err(err("persisted approval fence refused"));
    }
    validate_input(&c.declaration, &r.work)?;
    Ok(())
}
#[derive(Serialize, Deserialize)]
struct BrokerResponse {
    status: u16,
    original_bytes: Vec<u8>,
}
struct Observed {
    request: tokio::sync::mpsc::UnboundedSender<WireRequest>,
    response: Mutex<std::sync::mpsc::Receiver<BrokerResponse>>,
    bytes: Arc<Mutex<Option<Vec<u8>>>>,
    request_digest: Arc<Mutex<Option<String>>>,
}
impl WireTransport for Observed {
    fn send(&self, r: &WireRequest) -> std::result::Result<WireResponse, LlmError> {
        *self.request_digest.lock().map_err(|_| LlmError::Capacity)? = Some(digest_bytes(
            &serde_json::to_vec(&r.body).map_err(|_| LlmError::Capacity)?,
        ));
        self.request
            .send(r.clone())
            .map_err(|_| LlmError::Network("provider broker unavailable".into()))?;
        let observed = self
            .response
            .lock()
            .map_err(|_| LlmError::Capacity)?
            .recv()
            .map_err(|_| LlmError::Network("provider outcome unknown".into()))?;
        let body = serde_json::from_slice(&observed.original_bytes)
            .map_err(|_| LlmError::Wire("provider JSON rejected".into()))?;
        *self.bytes.lock().map_err(|_| LlmError::Capacity)? = Some(observed.original_bytes);
        Ok(WireResponse {
            status: observed.status,
            body,
        })
    }
}
async fn verify_model(
    d: &RunnerDeclaration,
    guard: &RunnerProviderGuard,
    session: &RunnerProviderSession,
    operation: &str,
) -> Result<()> {
    let request = WireRequest {
        method: "GET".into(),
        url: d
            .endpoint
            .strip_suffix("/api/chat")
            .map(|base| format!("{base}/api/tags"))
            .ok_or_else(|| err("invalid configured endpoint"))?,
        headers: BTreeMap::new(),
        body: serde_json::Value::Null,
    };
    let result = guard
        .dispatch(session, operation, &request)
        .await
        .map_err(err)?;
    if !(200..300).contains(&result.receipt.status) {
        return Err(err("model catalog unavailable"));
    }
    let value: serde_json::Value = serde_json::from_slice(&result.original_bytes)
        .map_err(|_| err("model catalog JSON rejected"))?;
    if !value["models"]
        .as_array()
        .ok_or_else(|| err("model catalog missing"))?
        .iter()
        .any(|model| model["name"] == d.profile.model_id && model["digest"] == d.model_digest)
    {
        return Err(err("configured model digest changed"));
    }
    Ok(())
}
pub async fn runner_worker_from_env() -> Result<()> {
    let c: Configuration =
        serde_json::from_str(&std::env::var(CONFIG).map_err(err)?).map_err(err)?;
    c.declaration.validate()?;
    let module = std::env::var("HYPERMIND_MODULE_ID").map_err(err)?;
    let launch = std::env::var("HYPERMIND_LAUNCH_ID").map_err(err)?;
    let generation = std::env::var("HYPERMIND_GENERATION")
        .map_err(err)?
        .parse()
        .map_err(err)?;
    let mut transport = UnixTransport::connect(
        &c.socket,
        Credentials {
            scope: c.scope.clone(),
            signing_key: SigningKey::from_bytes(&c.signing_key),
            peer_key: VerifyingKey::from_bytes(&c.peer_key).map_err(err)?,
        },
        ReplayGuard::default(),
        Limits {
            io_timeout: Duration::from_secs(130),
            ..Limits::default()
        },
    )
    .await
    .map_err(err)?;
    let descriptor = c.declaration.descriptor()?;
    let registration = Registration {
        module: module.clone(),
        launch,
        generation,
        pid: std::process::id(),
        descriptor,
        manifest: ModuleManifest {
            module_id: module,
            protocol_version: 1,
            capabilities: BTreeMap::from([("run".into(), 1)]),
            max_calls: 1,
            max_bytes: 512 * 1024,
            queue_calls: 1,
            queue_bytes: 512 * 1024,
        },
    };
    transport
        .send(Frame::request(
            "registration",
            serde_json::to_vec(&registration).map_err(err)?,
        ))
        .await
        .map_err(err)?;
    let ack = transport.receive().await.map_err(err)?;
    if ack.kind != FrameKind::Response || ack.correlation_id != "registration" {
        return Err(err("registration denied"));
    }
    loop {
        let f = transport.receive().await.map_err(err)?;
        if f.kind == FrameKind::Cancel && f.correlation_id == "shutdown" {
            return Ok(());
        }
        if f.kind != FrameKind::Request {
            return Err(err("request required"));
        }
        let r: RoleWireRequest = serde_json::from_slice(&f.payload).map_err(err)?;
        if f.correlation_id != r.work.id {
            return Err(err("correlation mismatch"));
        }
        approved(&c, &r)?;
        let input = validate_input(&c.declaration, &r.work)?;
        let d = c.declaration.clone();
        let prompt = input.prompt.clone();
        let id = r.work.id.clone();
        let (request_sender, mut request_receiver) = tokio::sync::mpsc::unbounded_channel();
        let (response_sender, response_receiver) = std::sync::mpsc::channel();
        let mut task = tokio::task::spawn_blocking(move || {
            let bytes = Arc::new(Mutex::new(None));
            let request_digest = Arc::new(Mutex::new(None));
            let provider = Ollama::new(
                ProviderConfig {
                    endpoint: d.endpoint,
                    api_key: None,
                    model: d.profile.model_id,
                    tier: ModelTier::Economy,
                    pricing: Pricing::default(),
                },
                Observed {
                    request: request_sender,
                    response: Mutex::new(response_receiver),
                    bytes: bytes.clone(),
                    request_digest: request_digest.clone(),
                },
            )
            .map_err(err)?;
            let result = provider.generate_structured(&StructuredRequest {
                prompt_id: id,
                system: d.system,
                prompt,
                json_schema: d.json_schema,
                maximum_output_tokens: d.budget.reserved_output_tokens as u32,
            });
            let raw = bytes
                .lock()
                .map_err(|_| err("provider capture poisoned"))?
                .take();
            let request_digest = request_digest
                .lock()
                .map_err(|_| err("request capture poisoned"))?
                .take();
            Ok::<_, RunnerError>((result, raw, request_digest))
        });
        let wire = request_receiver
            .recv()
            .await
            .ok_or_else(|| err("provider request unavailable"))?;
        let mut started = Frame::request(&r.work.id, b"provider_dispatch_started".to_vec());
        started.kind = FrameKind::Response;
        transport.send(started).await.map_err(err)?;
        transport
            .send(Frame::request(
                &r.work.id,
                serde_json::to_vec(&wire).map_err(err)?,
            ))
            .await
            .map_err(err)?;
        let timeout = Duration::from_millis(c.declaration.timeout_ms).min(Duration::from_nanos(
            r.work.deadline_ns.saturating_sub(now()).max(0) as u64,
        ));
        let provider_reply = tokio::select! { incoming=transport.receive()=>Some(incoming.map_err(err)?),_=tokio::time::sleep(timeout)=>None };
        let finished = if let Some(reply) = provider_reply {
            if reply.correlation_id != r.work.id {
                return Err(err("provider response correlation mismatch"));
            }
            match reply.kind {
                FrameKind::Response => {
                    let response: BrokerResponse = serde_json::from_slice(&reply.payload)
                        .map_err(|_| err("provider broker response rejected"))?;
                    response_sender
                        .send(response)
                        .map_err(|_| err("provider broker unavailable"))?;
                    Some((&mut task).await.map_err(err)??)
                }
                FrameKind::Cancel => {
                    drop(response_sender);
                    let _ = (&mut task).await;
                    None
                }
                _ => return Err(err("unexpected provider response frame")),
            }
        } else {
            drop(response_sender);
            let _ = (&mut task).await;
            None
        };
        let output = if let Some((response, raw, request_digest)) = finished {
            match response {
                Ok(response) => {
                    if !response.value.is_object()
                        || response.value.as_object().map(|o| o.len()) != Some(1)
                        || !response.value["answer"].is_string()
                    {
                        return Err(err("structured answer contract rejected"));
                    }
                    let raw = raw.ok_or_else(|| err("missing original response"))?;
                    if raw.len() > c.declaration.max_response_bytes {
                        return Err(err("response byte bound exceeded"));
                    }
                    let observed = observation(&raw, &r.work.id)?;
                    measured(&c.declaration, &input, &observed)?;
                    let outcome = RunnerOutcome {
                        provider_request_digest: request_digest
                            .ok_or_else(|| err("missing original request digest"))?,
                        value: response.value,
                        original_provider_bytes: raw,
                        observation: observed,
                        usage: input.usage,
                        source: r.work.source.clone(),
                        profile_fence: profile(&c.declaration, &r.work.source)?,
                        request_digest: r.ticket.work_digest,
                    };
                    let payload = serde_json::to_vec(&outcome).map_err(err)?;
                    RoleOutput {
                        terminal: RunTerminal::Completed {
                            result_digest: digest_bytes(&payload),
                        },
                        payload,
                        blocks: vec![],
                        grants: c.declaration.descriptor()?.grants,
                    }
                }
                Err(e) => RoleOutput {
                    terminal: RunTerminal::Failed {
                        reason: format!("provider response: {e:?}"),
                    },
                    payload: raw.unwrap_or_default(),
                    blocks: vec![],
                    grants: c.declaration.descriptor()?.grants,
                },
            }
        } else {
            RoleOutput {
                terminal: RunTerminal::Cancelled,
                payload: vec![],
                blocks: vec![],
                grants: c.declaration.descriptor()?.grants,
            }
        };
        let mut reply = Frame::request(&r.work.id, serde_json::to_vec(&output).map_err(err)?);
        reply.kind = FrameKind::Response;
        transport.send(reply).await.map_err(err)?;
        if finished_is_cancelled(&output) {
            return Ok(());
        }
    }
}
fn finished_is_cancelled(o: &RoleOutput) -> bool {
    matches!(o.terminal, RunTerminal::Cancelled)
}
