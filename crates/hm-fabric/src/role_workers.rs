use crate::{
    containment::OwnedProcessTree,
    role_dispatch::{handoff, DispatchError, RoleWireRequest},
    role_store::{
        CatalogReceipt, DispatchTicket, DurableRoleRegistry, RoleDescriptor, RoleError, RoleGrants,
        RoleKind, RoleOutput, SourceFence, WorkRecord, WorkState,
    },
    roles::{RoleVersion, RunTerminal},
    routing::{Binding, ModuleManifest, RouteCall, Router},
    supervisor::{ProcessSpec, Supervisor, SupervisorError},
    transport::{Credentials, Frame, FrameKind, Limits, ReplayGuard, TypedFailure, UnixTransport},
};
use ed25519_dalek::{SigningKey, VerifyingKey};
use hm_context::types::{digest_bytes, validate_id, Authority, ContextError, Scope};
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    net::UnixListener,
    sync::mpsc,
};
const TOOL_ENV: &str = "HM_FABRIC_TOOL_CONFIG";
static SOCKET_SEQUENCE: AtomicU64 = AtomicU64::new(1);
#[derive(Debug, thiserror::Error)]
pub enum ToolWorkerError {
    #[error(transparent)]
    Context(#[from] ContextError),
    #[error(transparent)]
    Role(#[from] RoleError),
    #[error(transparent)]
    Dispatch(#[from] DispatchError),
    #[error(transparent)]
    Transport(#[from] TypedFailure),
    #[error(transparent)]
    Supervisor(#[from] SupervisorError),
    #[error("tool worker refused: {0}")]
    Refused(String),
}
impl From<std::io::Error> for ToolWorkerError {
    fn from(e: std::io::Error) -> Self {
        ContextError::Io(e).into()
    }
}
impl From<serde_json::Error> for ToolWorkerError {
    fn from(e: serde_json::Error) -> Self {
        ContextError::Json(e).into()
    }
}
fn clock_ns() -> Result<i64, ToolWorkerError> {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ToolWorkerError::Refused("clock before epoch".into()))?
            .as_nanos(),
    )
    .map_err(|_| ContextError::Capacity.into())
}
fn route(e: impl std::fmt::Debug) -> ToolWorkerError {
    ToolWorkerError::Refused(format!("routing: {e:?}"))
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolDeclaration {
    pub id: String,
    pub semantic_version: String,
    pub program: PathBuf,
    pub program_digest: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub environment: BTreeMap<String, String>,
    pub max_input_bytes: usize,
    pub max_output_bytes: usize,
    pub timeout_ms: u64,
}
impl ToolDeclaration {
    pub fn admit(
        id: impl Into<String>,
        program: impl AsRef<Path>,
        args: Vec<String>,
        cwd: impl AsRef<Path>,
        max_input_bytes: usize,
        max_output_bytes: usize,
        timeout_ms: u64,
    ) -> Result<Self, ToolWorkerError> {
        let program = fs::canonicalize(program)?;
        if fs::metadata(&program)?.len() > 64 * 1024 * 1024 {
            return Err(ContextError::Capacity.into());
        }
        let declaration = Self {
            id: id.into(),
            semantic_version: "1.0.0".into(),
            program_digest: digest_bytes(&fs::read(&program)?),
            program,
            args,
            cwd: fs::canonicalize(cwd)?,
            environment: BTreeMap::new(),
            max_input_bytes,
            max_output_bytes,
            timeout_ms,
        };
        declaration.validate()?;
        Ok(declaration)
    }
    pub fn descriptor(&self) -> Result<RoleDescriptor, ToolWorkerError> {
        self.validate()?;
        Ok(RoleDescriptor{id:self.id.clone(),kind:RoleKind::Tool,pin:RoleVersion::new(b"HyperMind declared native tool v1: exact executable/argv/cwd; request bytes on stdin; bounded process output",&serde_json::to_vec(self)?),semantic_version:self.semantic_version.clone(),capabilities:BTreeMap::from([("invoke".into(),1)]),grants:RoleGrants{operations:BTreeSet::from(["invoke".into()]),authorities:vec![Authority::ToolObserved],hooks:BTreeSet::from(["tool_dispatch".into()]),max_input_tokens:1_048_576,max_output_tokens:1_048_576,may_reorder:false},transform:None})
    }
    fn validate(&self) -> Result<(), ToolWorkerError> {
        validate_id(&self.id)?;
        if !self.program.is_absolute()
            || !self.cwd.is_absolute()
            || fs::canonicalize(&self.program)? != self.program
            || fs::canonicalize(&self.cwd)? != self.cwd
            || !self.cwd.is_dir()
            || self.args.len() > 64
            || self.args.iter().any(|a| a.len() > 4096 || a.contains('\0'))
            || self.environment.iter().any(|(k, v)| {
                !matches!(k.as_str(), "LANG" | "LC_ALL" | "TZ") || v.len() > 256 || v.contains('\0')
            })
            || self.max_input_bytes == 0
            || self.max_input_bytes > 65536
            || self.max_output_bytes == 0
            || self.max_output_bytes > 65536
            || self.timeout_ms == 0
            || self.timeout_ms > 30000
        {
            return Err(ToolWorkerError::Refused(
                "invalid declared command or bounds".into(),
            ));
        }
        if fs::metadata(&self.program)?.len() > 64 * 1024 * 1024 {
            return Err(ContextError::Capacity.into());
        }
        let bytes = fs::read(&self.program)?;
        let name = self
            .program
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if bytes.len() > 64 * 1024 * 1024
            || !matches!(name, "cat" | "tee" | "wc" | "head" | "sort" | "sleep")
            || !bytes.starts_with(b"\x7fELF")
            || matches!(
                name,
                "sh" | "bash"
                    | "dash"
                    | "zsh"
                    | "ksh"
                    | "fish"
                    | "python"
                    | "python3"
                    | "node"
                    | "perl"
                    | "ruby"
            )
            || digest_bytes(&bytes) != self.program_digest
        {
            return Err(ToolWorkerError::Refused(
                "executable identity changed or interpreter refused".into(),
            ));
        }
        Ok(())
    }
}
#[derive(Clone)]
pub struct ToolWorkerConfig {
    pub root: PathBuf,
    pub registry_path: PathBuf,
    pub scope: Scope,
    pub host_key: [u8; 32],
    pub worker_key: [u8; 32],
    pub declaration: ToolDeclaration,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkerConfiguration {
    socket: PathBuf,
    registry_path: PathBuf,
    scope: Scope,
    signing_key: [u8; 32],
    peer_key: [u8; 32],
    declaration: ToolDeclaration,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registration {
    launch: LaunchIdentityWire,
    descriptor: RoleDescriptor,
    manifest: ModuleManifest,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LaunchIdentityWire {
    module_id: String,
    launch_id: String,
    generation: u64,
    pid: u32,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolProcessOutcome {
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub stdout_digest: String,
    pub stderr_digest: String,
    pub cancelled: bool,
    pub timed_out: bool,
    pub overflow: bool,
    pub effect_unknown: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum WorkerReply {
    Started { pid: u32 },
    Finished { output: RoleOutput },
}

pub struct ToolWorkerSession {
    supervisor: Supervisor,
    router: Router,
    binding: Binding,
    transport: UnixTransport,
    scope: Scope,
    declaration: ToolDeclaration,
    socket: PathBuf,
    in_flight: Option<DispatchTicket>,
    tool_process: Option<OwnedProcessTree>,
}
impl ToolWorkerSession {
    pub async fn launch(
        mut spec: ProcessSpec,
        config: ToolWorkerConfig,
        registry: &DurableRoleRegistry,
        limits: Limits,
    ) -> Result<Self, ToolWorkerError> {
        if &config.scope != registry.scope() {
            return Err(ContextError::ScopeMismatch.into());
        }
        let root = fs::canonicalize(&config.root)?;
        if !root.is_dir() {
            return Err(ToolWorkerError::Refused("existing root required".into()));
        }
        let registry_path = fs::canonicalize(&config.registry_path)?;
        let descriptor = config.declaration.descriptor()?;
        let receipt = registry
            .catalog(&descriptor.id)?
            .ok_or(ContextError::Stale)?;
        if receipt.descriptor != descriptor || receipt.withdrawal.is_some() {
            return Err(RoleError::Withdrawn.into());
        }
        let socket = root.join(format!(
            "tool-{}-{}.sock",
            std::process::id(),
            SOCKET_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let listener = UnixListener::bind(&socket)?;
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&socket, fs::Permissions::from_mode(0o600))?;
        if spec.env.contains_key(TOOL_ENV) {
            return Err(ToolWorkerError::Refused(
                "tool configuration is owner controlled".into(),
            ));
        }
        let worker = WorkerConfiguration {
            socket: socket.clone(),
            registry_path,
            scope: config.scope.clone(),
            signing_key: config.worker_key,
            peer_key: SigningKey::from_bytes(&config.host_key)
                .verifying_key()
                .to_bytes(),
            declaration: config.declaration.clone(),
        };
        spec.env
            .insert(TOOL_ENV.into(), serde_json::to_string(&worker)?);
        let mut supervisor = Supervisor::new();
        let launch = supervisor.start(spec)?;
        let accepted = async {
            let (stream, _) = tokio::time::timeout(limits.io_timeout, listener.accept())
                .await
                .map_err(|_| ToolWorkerError::Refused("worker readiness timeout".into()))??;
            let mut transport = UnixTransport::accept(
                stream,
                Credentials {
                    scope: config.scope.clone(),
                    signing_key: SigningKey::from_bytes(&config.host_key),
                    peer_key: SigningKey::from_bytes(&config.worker_key).verifying_key(),
                },
                ReplayGuard::default(),
                limits.clone(),
            )
            .await?;
            let frame = transport.receive().await?;
            let registered: Registration = serde_json::from_slice(&frame.payload)?;
            if frame.kind != FrameKind::Request
                || frame.correlation_id != "registration"
                || registered.launch.module_id != launch.module_id
                || registered.launch.launch_id != launch.launch_id
                || registered.launch.generation != launch.generation
                || registered.launch.pid != launch.pid
                || registered.descriptor != descriptor
                || registered.manifest.module_id != launch.module_id
            {
                return Err(ToolWorkerError::Refused(
                    "worker launch or contract pin mismatch".into(),
                ));
            }
            let mut router = Router::new();
            router
                .authorize_launch(
                    transport.identity(),
                    &launch.module_id,
                    &launch.launch_id,
                    launch.generation,
                )
                .map_err(route)?;
            let binding = router
                .register(
                    transport.identity(),
                    registered.manifest,
                    &launch.launch_id,
                    launch.generation,
                )
                .map_err(route)?;
            let mut reply = Frame::request("registration", vec![]);
            reply.kind = FrameKind::Response;
            transport.send(reply).await?;
            Ok::<_, ToolWorkerError>((transport, router, binding))
        }
        .await;
        match accepted {
            Ok((transport, router, binding)) => Ok(Self {
                supervisor,
                router,
                binding,
                transport,
                scope: config.scope,
                declaration: config.declaration,
                socket,
                in_flight: None,
                tool_process: None,
            }),
            Err(error) => {
                let _ = supervisor.shutdown();
                let _ = fs::remove_file(socket);
                Err(error)
            }
        }
    }
    pub async fn submit(
        &mut self,
        registry: &mut DurableRoleRegistry,
        id: &str,
        source: &SourceFence,
    ) -> Result<DispatchTicket, ToolWorkerError> {
        if self.in_flight.is_some() {
            return Err(ContextError::Conflict.into());
        }
        let record = registry.get(id)?.ok_or(ContextError::Stale)?;
        if record.state != WorkState::Approved {
            return Err(if record.state == WorkState::Uncertain {
                RoleError::Uncertain
            } else {
                RoleError::Unapproved
            }
            .into());
        }
        if record.descriptor != self.declaration.descriptor()?
            || registry.scope() != &self.scope
            || record.work.payload.len() > self.declaration.max_input_bytes
        {
            return Err(ContextError::Conflict.into());
        }
        let bytes = serde_json::to_vec(&record.work)?;
        self.router
            .enqueue(
                self.transport.identity(),
                &self.binding,
                RouteCall {
                    id: id.into(),
                    caller: "owner".into(),
                    capability: "invoke".into(),
                    capability_version: 1,
                    deadline_unix_ms: Some((record.work.deadline_ns as u64) / 1_000_000),
                    payload: bytes,
                },
                (clock_ns()? as u64) / 1_000_000,
            )
            .map_err(route)?;
        let (dispatch, _) = self
            .router
            .dispatch(&self.binding, (clock_ns()? as u64) / 1_000_000)
            .map_err(route)?;
        if dispatch.is_none() {
            return Err(ToolWorkerError::Refused(
                "routing credits unavailable".into(),
            ));
        }
        let ticket = match handoff(registry, &mut self.transport, id, source, clock_ns()?).await {
            Ok(ticket) => ticket,
            Err(error) => {
                let _ = self.router.complete(&self.binding, id);
                return Err(error.into());
            }
        };
        self.in_flight = Some(ticket.clone());
        let result = async {
            let frame = self.transport.receive().await?;
            if frame.kind != FrameKind::Response || frame.correlation_id != id {
                return Err(ContextError::Conflict.into());
            }
            let WorkerReply::Started { pid } = serde_json::from_slice(&frame.payload)? else {
                return Err(ToolWorkerError::Refused(
                    "tool process start acknowledgement required".into(),
                ));
            };
            let worker_pid = self
                .supervisor
                .active()
                .ok_or(ContextError::Stale)?
                .identity
                .pid;
            let status = fs::read_to_string(format!("/proc/{pid}/status"))?;
            let parent = status
                .lines()
                .find(|line| line.starts_with("PPid:"))
                .and_then(|line| line.split_whitespace().nth(1))
                .and_then(|value| value.parse::<u32>().ok());
            if parent != Some(worker_pid) {
                return Err(ToolWorkerError::Refused(
                    "tool process is not owned by worker".into(),
                ));
            }
            self.tool_process = Some(
                OwnedProcessTree::attach(pid)
                    .map_err(|e| ToolWorkerError::Refused(e.to_string()))?,
            );
            let mut ack = Frame::request(id, vec![]);
            ack.kind = FrameKind::Response;
            self.transport.send(ack).await?;
            Ok::<_, ToolWorkerError>(())
        }
        .await;
        if let Err(error) = result {
            registry.mark_uncertain(&ticket)?;
            return Err(error);
        }
        Ok(ticket)
    }
    pub async fn receive(
        &mut self,
        registry: &mut DurableRoleRegistry,
        source: &SourceFence,
    ) -> Result<WorkRecord, ToolWorkerError> {
        let ticket = self.in_flight.clone().ok_or(ContextError::Stale)?;
        let result = async {
            let frame = self.transport.receive().await?;
            if frame.kind != FrameKind::Response || frame.correlation_id != ticket.id {
                return Err(ContextError::Conflict.into());
            }
            let WorkerReply::Finished { output } = serde_json::from_slice(&frame.payload)? else {
                return Err(ToolWorkerError::Refused(
                    "terminal tool result required".into(),
                ));
            };
            let outcome: ToolProcessOutcome = serde_json::from_slice(&output.payload)?;
            if outcome.stdout_digest != digest_bytes(&outcome.stdout)
                || outcome.stderr_digest != digest_bytes(&outcome.stderr)
            {
                return Err(ContextError::Conflict.into());
            }
            if outcome.effect_unknown {
                registry.mark_uncertain(&ticket)?;
            }
            let record = registry.complete(&ticket, source, output, clock_ns()?)?;
            self.router
                .complete(&self.binding, &ticket.id)
                .map_err(route)?;
            self.in_flight = None;
            self.tool_process.take();
            Ok::<_, ToolWorkerError>(record)
        }
        .await;
        if result.is_err() {
            registry.mark_uncertain(&ticket)?;
        }
        result
    }
    pub async fn execute(
        &mut self,
        registry: &mut DurableRoleRegistry,
        id: &str,
        source: &SourceFence,
    ) -> Result<WorkRecord, ToolWorkerError> {
        self.submit(registry, id, source).await?;
        self.receive(registry, source).await
    }
    pub async fn cancel(
        &mut self,
        registry: &mut DurableRoleRegistry,
    ) -> Result<(), ToolWorkerError> {
        let ticket = self.in_flight.as_ref().ok_or(ContextError::Stale)?;
        registry.mark_uncertain(ticket)?;
        let mut frame = Frame::request(&ticket.id, vec![]);
        frame.kind = FrameKind::Cancel;
        self.transport.send(frame).await?;
        Ok(())
    }
    pub fn kill_worker(&mut self) -> Result<(), ToolWorkerError> {
        if let Some(tool) = &self.tool_process {
            tool.terminate()
                .map_err(|e| ToolWorkerError::Refused(e.to_string()))?;
        }
        if let Some(worker) = self.supervisor.active() {
            OwnedProcessTree::attach(worker.identity.pid)
                .and_then(|p| p.terminate())
                .map_err(|e| ToolWorkerError::Refused(e.to_string()))?;
        }
        self.supervisor.shutdown()?;
        Ok(())
    }
    pub async fn shutdown(&mut self) -> Result<(), ToolWorkerError> {
        if self.in_flight.is_none() {
            let mut frame = Frame::request("shutdown", vec![]);
            frame.kind = FrameKind::Cancel;
            let _ = self.transport.send(frame).await;
        } else if let Some(tool) = &self.tool_process {
            let _ = tool.terminate();
        }
        self.supervisor.shutdown()?;
        Ok(())
    }
}
impl Drop for ToolWorkerSession {
    fn drop(&mut self) {
        if let Some(tool) = &self.tool_process {
            let _ = tool.terminate();
        }
        let _ = self.supervisor.shutdown();
        let _ = fs::remove_file(&self.socket);
    }
}

#[derive(Deserialize)]
struct ApprovalSnapshot {
    scope: Scope,
    catalog: BTreeMap<String, CatalogReceipt>,
    work: BTreeMap<String, WorkRecord>,
}
fn approved(
    config: &WorkerConfiguration,
    request: &RoleWireRequest,
) -> Result<RoleDescriptor, ToolWorkerError> {
    let db = Connection::open_with_flags(&config.registry_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| ToolWorkerError::Refused(e.to_string()))?;
    let bytes: Vec<u8> = db
        .query_row("SELECT data FROM role_state WHERE id=1", [], |r| r.get(0))
        .map_err(|e| ToolWorkerError::Refused(e.to_string()))?;
    let state: ApprovalSnapshot = serde_json::from_slice(&bytes)?;
    let work = state
        .work
        .get(&request.work.id)
        .ok_or(ContextError::Stale)?;
    let catalog = state
        .catalog
        .get(&request.work.role_id)
        .ok_or(ContextError::Stale)?;
    let expected = config.declaration.descriptor()?;
    if state.scope != config.scope
        || work.state != WorkState::Dispatched
        || work.work != request.work
        || work.ticket.as_ref() != Some(&request.ticket)
        || work.approval_revision != Some(catalog.revision)
        || catalog.withdrawal.is_some()
        || catalog.descriptor != expected
        || work.descriptor != expected
        || request.ticket.work_digest != digest_bytes(&serde_json::to_vec(&request.work)?)
        || request.work.payload.len() > config.declaration.max_input_bytes
        || request.work.deadline_ns <= clock_ns()?
    {
        return Err(RoleError::Unapproved.into());
    }
    Ok(expected)
}

pub async fn tool_worker_from_env() -> Result<(), ToolWorkerError> {
    let config: WorkerConfiguration = serde_json::from_str(
        &std::env::var(TOOL_ENV)
            .map_err(|_| ToolWorkerError::Refused("tool configuration missing".into()))?,
    )?;
    config.declaration.validate()?;
    let module_id = std::env::var("HYPERMIND_MODULE_ID")
        .map_err(|_| ToolWorkerError::Refused("module identity missing".into()))?;
    let launch_id = std::env::var("HYPERMIND_LAUNCH_ID")
        .map_err(|_| ToolWorkerError::Refused("launch identity missing".into()))?;
    let generation = std::env::var("HYPERMIND_GENERATION")
        .map_err(|_| ToolWorkerError::Refused("generation missing".into()))?
        .parse()
        .map_err(|_| ToolWorkerError::Refused("generation invalid".into()))?;
    let descriptor = config.declaration.descriptor()?;
    let mut transport = UnixTransport::connect(
        &config.socket,
        Credentials {
            scope: config.scope.clone(),
            signing_key: SigningKey::from_bytes(&config.signing_key),
            peer_key: VerifyingKey::from_bytes(&config.peer_key)
                .map_err(|_| ToolWorkerError::Refused("peer key invalid".into()))?,
        },
        ReplayGuard::default(),
        Limits::default(),
    )
    .await?;
    let registration = Registration {
        launch: LaunchIdentityWire {
            module_id: module_id.clone(),
            launch_id,
            generation,
            pid: std::process::id(),
        },
        descriptor,
        manifest: ModuleManifest {
            module_id,
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
            serde_json::to_vec(&registration)?,
        ))
        .await?;
    let ack = transport.receive().await?;
    if ack.kind != FrameKind::Response || ack.correlation_id != "registration" {
        return Err(ToolWorkerError::Refused("registration refused".into()));
    }
    loop {
        let frame = transport.receive().await?;
        if frame.kind == FrameKind::Cancel && frame.correlation_id == "shutdown" {
            return Ok(());
        }
        if frame.kind != FrameKind::Request {
            return Err(ToolWorkerError::Refused("role request required".into()));
        }
        let request: RoleWireRequest = serde_json::from_slice(&frame.payload)?;
        if request.work.id != frame.correlation_id {
            return Err(ContextError::Conflict.into());
        }
        let descriptor = approved(&config, &request)?;
        let outcome = run_tool(&config.declaration, &request, &mut transport).await?;
        let payload = serde_json::to_vec(&outcome)?;
        let terminal = if outcome.cancelled {
            RunTerminal::Cancelled
        } else if outcome.timed_out || outcome.overflow || outcome.exit_code != Some(0) {
            RunTerminal::Failed {
                reason: if outcome.timed_out {
                    "declared tool deadline exceeded"
                } else if outcome.overflow {
                    "declared output budget exceeded"
                } else {
                    "declared command exited unsuccessfully"
                }
                .into(),
            }
        } else {
            RunTerminal::Completed {
                result_digest: digest_bytes(&payload),
            }
        };
        let output = RoleOutput {
            terminal,
            payload,
            blocks: vec![],
            grants: descriptor.grants,
        };
        let mut reply = Frame::request(
            &request.work.id,
            serde_json::to_vec(&WorkerReply::Finished { output })?,
        );
        reply.kind = FrameKind::Response;
        transport.send(reply).await?;
    }
}
async fn capture<R: AsyncRead + Unpin>(
    mut reader: R,
    used: Arc<AtomicUsize>,
    limit: usize,
    overflow: mpsc::Sender<()>,
) -> Result<Vec<u8>, std::io::Error> {
    let mut bytes = Vec::new();
    let mut chunk = [0; 4096];
    loop {
        let count = reader.read(&mut chunk).await?;
        if count == 0 {
            return Ok(bytes);
        }
        let prior = used.fetch_add(count, Ordering::Relaxed);
        let allowed = limit.saturating_sub(prior).min(count);
        bytes.extend_from_slice(&chunk[..allowed]);
        if allowed < count {
            let _ = overflow.send(()).await;
            return Ok(bytes);
        }
    }
}
async fn run_tool(
    declaration: &ToolDeclaration,
    request: &RoleWireRequest,
    transport: &mut UnixTransport,
) -> Result<ToolProcessOutcome, ToolWorkerError> {
    declaration.validate()?;
    let mut command = tokio::process::Command::new(&declaration.program);
    command
        .args(&declaration.args)
        .current_dir(&declaration.cwd)
        .env_clear()
        .envs(&declaration.environment)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command.spawn()?;
    let pid = child.id().ok_or(ContextError::Stale)?;
    let mut started = Frame::request(
        &request.work.id,
        serde_json::to_vec(&WorkerReply::Started { pid })?,
    );
    started.kind = FrameKind::Response;
    transport.send(started).await?;
    let ack = transport.receive().await?;
    if ack.kind != FrameKind::Response
        || ack.correlation_id != request.work.id
        || !ack.payload.is_empty()
    {
        child.kill().await?;
        return Err(ToolWorkerError::Refused(
            "start acknowledgement required".into(),
        ));
    }
    let mut stdin = child.stdin.take().ok_or(ContextError::Stale)?;
    let input = request.work.payload.clone();
    let writer = tokio::spawn(async move {
        stdin.write_all(&input).await?;
        stdin.shutdown().await
    });
    let used = Arc::new(AtomicUsize::new(0));
    let (tx, mut rx) = mpsc::channel(2);
    let stdout = tokio::spawn(capture(
        child.stdout.take().ok_or(ContextError::Stale)?,
        used.clone(),
        declaration.max_output_bytes,
        tx.clone(),
    ));
    let stderr = tokio::spawn(capture(
        child.stderr.take().ok_or(ContextError::Stale)?,
        used.clone(),
        declaration.max_output_bytes,
        tx,
    ));
    let remaining = request.work.deadline_ns.saturating_sub(clock_ns()?).max(0) as u64;
    let timeout =
        Duration::from_millis(declaration.timeout_ms).min(Duration::from_nanos(remaining));
    let mut cancelled = false;
    let mut timed_out = false;
    let mut exceeded = false;
    let status = tokio::select! {status=child.wait()=>status?,frame=transport.receive()=>{let frame=frame?;if frame.kind!=FrameKind::Cancel||frame.correlation_id!=request.work.id{return Err(ToolWorkerError::Refused("only matching cancellation allowed while running".into()))}cancelled=true;child.kill().await?;child.wait().await?},_=tokio::time::sleep(timeout)=>{timed_out=true;child.kill().await?;child.wait().await?},Some(_)=rx.recv()=>{exceeded=true;child.kill().await?;child.wait().await?}};
    let writer_ok = writer.await.is_ok_and(|result| result.is_ok());
    let stdout = stdout
        .await
        .map_err(|e| ToolWorkerError::Refused(e.to_string()))??;
    let stderr = stderr
        .await
        .map_err(|e| ToolWorkerError::Refused(e.to_string()))??;
    exceeded |= used.load(Ordering::Relaxed) > declaration.max_output_bytes;
    Ok(ToolProcessOutcome {
        exit_code: status.code(),
        stdout_digest: digest_bytes(&stdout),
        stderr_digest: digest_bytes(&stderr),
        stdout,
        stderr,
        cancelled,
        timed_out,
        overflow: exceeded,
        effect_unknown: cancelled || timed_out || exceeded || !writer_ok || status.code().is_none(),
    })
}
