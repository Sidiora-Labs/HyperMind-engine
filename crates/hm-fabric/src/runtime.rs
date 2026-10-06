use crate::backend_config::BackendReadiness;
use crate::backend_runtime::{BackendRuntime, BackendRuntimeError, RuntimePublication};
use crate::bus_nats::NatsReplay;
use crate::{
    bus::{Bus, Event, Grant},
    effects::{
        EffectIntent, EffectObservation, EffectOutcome, EffectReceipt, EffectState, EffectStore,
    },
    roles::{RoleVersion, RunRegistry, RunTerminal, ToolContract, ToolReceipt, ToolRegistry},
    routing::{Binding, ModuleManifest, RouteCall, Router},
    storage::{CanonicalRoot, FencedStore, Migration, StorageError},
    supervisor::{LaunchIdentity, ProcessSpec, Supervisor, SupervisorError},
    transport::{
        Credentials, Effect, FailureCode, Frame, FrameKind, Limits, ReplayGuard, TypedFailure,
        UnixTransport,
    },
};
use ed25519_dalek::SigningKey;
use hm_context::types::{ContextError, Cursor, Scope, digest_bytes, validate_id};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::net::UnixListener;

const PRINCIPAL: &str = "runtime";
const STREAM: &str = "runtime_receipts";
const RELAY: &str = "runtime_receipt_relay";
const WORKER_ENV: &str = "HM_FABRIC_WORKER_CONFIG";
const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "runtime_contracts",
    sql: "CREATE TABLE runtime_scope(id INTEGER PRIMARY KEY CHECK(id=1), scope TEXT NOT NULL); CREATE TABLE runtime_results(id TEXT PRIMARY KEY, result BLOB NOT NULL);",
}];

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error(transparent)]
    Backend(#[from] BackendRuntimeError),
    #[error(transparent)]
    Context(#[from] ContextError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Supervisor(#[from] SupervisorError),
    #[error(transparent)]
    Transport(#[from] TypedFailure),
    #[error("routing refused: {0}")]
    Routing(String),
    #[error("effect {0} requires reconciliation; redispatch refused")]
    Uncertain(String),
    #[error("worker unavailable")]
    WorkerUnavailable,
    #[error("runtime protocol: {0}")]
    Protocol(String),
}
impl From<std::io::Error> for RuntimeError {
    fn from(error: std::io::Error) -> Self {
        ContextError::Io(error).into()
    }
}
impl From<serde_json::Error> for RuntimeError {
    fn from(error: serde_json::Error) -> Self {
        ContextError::Json(error).into()
    }
}
fn route(error: impl std::fmt::Debug) -> RuntimeError {
    RuntimeError::Routing(format!("{error:?}"))
}
fn now_ms() -> Result<u64, RuntimeError> {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| RuntimeError::Protocol("clock before epoch".into()))?
            .as_millis(),
    )
    .map_err(|_| RuntimeError::Protocol("clock overflow".into()))
}

pub fn digest_contract() -> ToolContract {
    ToolContract {
        name: "digest".into(),
        pin: RoleVersion::new(
            b"HyperMind digest request v1: id,pin,deadline_unix_ms,bytes",
            b"SHA-256 of exact input bytes, lowercase hexadecimal, no external writes",
        ),
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerRequest {
    pub id: String,
    pub pin: RoleVersion,
    pub deadline_unix_ms: Option<u64>,
    pub bytes: Vec<u8>,
}
impl WorkerRequest {
    pub fn digest(id: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            id: id.into(),
            pin: digest_contract().pin,
            deadline_unix_ms: None,
            bytes,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerResult {
    pub id: String,
    pub pin: RoleVersion,
    pub request_digest: String,
    pub digest: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkerConfiguration {
    scope: Scope,
    socket: PathBuf,
    signing_key: [u8; 32],
    peer_key: [u8; 32],
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registration {
    module_id: String,
    launch_id: String,
    generation: u64,
    manifest: ModuleManifest,
    contract: ToolContract,
}

pub struct RuntimeConfig {
    pub root: PathBuf,
    pub scope: Scope,
    pub host_key: [u8; 32],
    pub worker_key: [u8; 32],
    pub limits: Limits,
}
impl RuntimeConfig {
    pub fn new(
        root: impl Into<PathBuf>,
        scope: Scope,
        host_key: [u8; 32],
        worker_key: [u8; 32],
    ) -> Self {
        Self {
            root: root.into(),
            scope,
            host_key,
            worker_key,
            limits: Limits::default(),
        }
    }
}

pub struct RuntimeService {
    config: RuntimeConfig,
    root: CanonicalRoot,
    owner: Option<FencedStore>,
    backend: Option<BackendRuntime>,
    effects: EffectStore,
    bus: Option<Bus>,
    tools: ToolRegistry,
    runs: RunRegistry,
    router: Router,
    supervisor: Supervisor,
    listener: Option<UnixListener>,
    transport: Option<UnixTransport>,
    binding: Option<Binding>,
    queued: BTreeSet<String>,
    in_flight: BTreeMap<String, u64>,
}
impl RuntimeService {
    pub fn open(mut config: RuntimeConfig) -> Result<Self, RuntimeError> {
        config.scope.validate()?;
        let root = CanonicalRoot::admit(&config.root)?;
        config.root = root.path().to_path_buf();
        for name in ["runtime.sqlite", "effects.sqlite", "bus.sqlite"] {
            check_file(&config.root.join(name))?;
        }
        let mut owner = FencedStore::open(config.root.join("runtime.sqlite"), MIGRATIONS)?;
        let scope_json = serde_json::to_string(&config.scope)?;
        owner.transaction(owner.epoch(), |tx| {
            let old: Option<String> = tx
                .query_row("SELECT scope FROM runtime_scope WHERE id=1", [], |r| {
                    r.get(0)
                })
                .optional()?;
            if old.as_ref().is_some_and(|old| old != &scope_json) {
                return Err(StorageError::InvalidPath);
            }
            tx.execute(
                "INSERT OR IGNORE INTO runtime_scope VALUES(1,?1)",
                [scope_json],
            )?;
            Ok(())
        })?;
        for name in ["effects.sqlite", "effects.sqlite.effects.lock"] {
            let path = config.root.join(name);
            check_file(&path)?;
            use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
            let file = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .mode(0o600)
                .open(path)?;
            file.set_permissions(fs::Permissions::from_mode(0o600))?;
        }
        let effects = EffectStore::open(config.root.join("effects.sqlite"))?;
        let mut bus = Bus::open(config.root.join("bus.sqlite"), config.scope.clone())?;
        bus.grant(Grant {
            principal: PRINCIPAL.into(),
            stream: STREAM.into(),
            publish: true,
            subscribe: true,
            register: false,
        })?;
        let mut tools = ToolRegistry::default();
        tools.register(digest_contract())?;
        let mut service = Self {
            config,
            root,
            owner: Some(owner),
            backend: None,
            effects,
            bus: Some(bus),
            tools,
            runs: RunRegistry::default(),
            router: Router::new(),
            supervisor: Supervisor::new(),
            listener: None,
            transport: None,
            binding: None,
            queued: BTreeSet::new(),
            in_flight: BTreeMap::new(),
        };
        service.relay_receipts()?;
        Ok(service)
    }
    pub fn from_backends(
        mut config: RuntimeConfig,
        backend: BackendRuntime,
    ) -> Result<Self, RuntimeError> {
        config.scope.validate()?;
        let root = CanonicalRoot::admit(&config.root)?;
        if root.path() != backend.home() || config.scope != *backend.scope() {
            return Err(ContextError::ScopeMismatch.into());
        }
        config.root = root.path().to_owned();
        backend.fence()?;
        let effects = EffectStore::from_backend(
            config.scope.clone(),
            root.path().to_str().ok_or_else(|| RuntimeError::Protocol("UTF-8 backend home required".into()))?.to_owned(),
            backend.record_store(),
        )?;
        let mut tools = ToolRegistry::default();
        tools.register(digest_contract())?;
        let mut service = Self {
            config,
            root,
            owner: None,
            backend: Some(backend),
            effects,
            bus: None,
            tools,
            runs: RunRegistry::default(),
            router: Router::new(),
            supervisor: Supervisor::new(),
            listener: None,
            transport: None,
            binding: None,
            queued: BTreeSet::new(),
            in_flight: BTreeMap::new(),
        };
        service.relay_receipts()?;
        Ok(service)
    }
    pub fn backend_readiness(&self) -> Result<Option<BackendReadiness>, RuntimeError> {
        self.backend
            .as_ref()
            .map(|backend| backend.readiness().map_err(RuntimeError::from))
            .transpose()
    }
    pub fn backend_publications(
        &self,
        limit: u32,
    ) -> Result<Vec<RuntimePublication>, RuntimeError> {
        self.backend
            .as_ref()
            .ok_or_else(|| RuntimeError::Protocol("selected backend required".into()))?
            .publications(limit)
            .map_err(RuntimeError::from)
    }
    pub async fn flush_backend_events(&self) -> Result<usize, RuntimeError> {
        match &self.backend {
            Some(backend) => Ok(backend.flush_events().await?),
            None => Ok(0),
        }
    }
    pub async fn events_async(&self, after: u64, limit: u32) -> Result<NatsReplay, RuntimeError> {
        match &self.backend {
            Some(backend) => Ok(backend.replay_events(after, limit).await?),
            None => {
                let events = self.events(after, limit)?;
                let cursor = events.last().map_or(after, |event| event.sequence);
                Ok(NatsReplay {
                    events,
                    gaps: vec![],
                    cursor,
                })
            }
        }
    }
    pub fn writer_epoch(&self) -> u64 {
        self.backend.as_ref().map_or_else(
            || self.owner.as_ref().map_or(0, FencedStore::epoch),
            BackendRuntime::owner_epoch,
        )
    }
    pub fn root(&self) -> &Path {
        self.root.path()
    }
    fn fence(&self) -> Result<(), RuntimeError> {
        if let Some(backend) = &self.backend {
            backend.fence()?;
        } else {
            self.owner
                .as_ref()
                .ok_or_else(|| RuntimeError::Protocol("owner missing".into()))?
                .read(|_| Ok(()))?;
        }
        Ok(())
    }
    pub async fn launch_worker(
        &mut self,
        mut spec: ProcessSpec,
    ) -> Result<LaunchIdentity, RuntimeError> {
        self.fence()?;
        if self.transport.is_some() || self.supervisor.active().is_some() {
            return Err(ContextError::Conflict.into());
        }
        let socket = self.root.path().join("worker.sock");
        match fs::symlink_metadata(&socket) {
            Ok(meta)
                if {
                    use std::os::unix::fs::FileTypeExt;
                    meta.file_type().is_socket()
                } =>
            {
                fs::remove_file(&socket)?
            }
            Ok(_) => {
                return Err(RuntimeError::Protocol(
                    "socket path occupied by non-socket".into(),
                ));
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        let listener = UnixListener::bind(&socket)?;
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&socket, fs::Permissions::from_mode(0o600))?;
        if spec.env.contains_key(WORKER_ENV) {
            return Err(RuntimeError::Protocol(
                "worker configuration is owner controlled".into(),
            ));
        }
        let worker = WorkerConfiguration {
            scope: self.config.scope.clone(),
            socket,
            signing_key: self.config.worker_key,
            peer_key: SigningKey::from_bytes(&self.config.host_key)
                .verifying_key()
                .to_bytes(),
        };
        spec.env
            .insert(WORKER_ENV.into(), serde_json::to_string(&worker)?);
        let launch = self.supervisor.start(spec)?;
        let result = async {
            let (stream, _) =
                tokio::time::timeout(self.config.limits.io_timeout, listener.accept())
                    .await
                    .map_err(|_| RuntimeError::WorkerUnavailable)??;
            let credentials = Credentials {
                scope: self.config.scope.clone(),
                signing_key: SigningKey::from_bytes(&self.config.host_key),
                peer_key: SigningKey::from_bytes(&self.config.worker_key).verifying_key(),
            };
            let mut transport = UnixTransport::accept(
                stream,
                credentials,
                ReplayGuard::default(),
                self.config.limits.clone(),
            )
            .await?;
            let frame = transport.receive().await?;
            if frame.kind != FrameKind::Request || frame.correlation_id != "registration" {
                return Err(RuntimeError::Protocol("registration required".into()));
            }
            let registration: Registration = serde_json::from_slice(&frame.payload)?;
            if registration.module_id != launch.module_id
                || registration.launch_id != launch.launch_id
                || registration.generation != launch.generation
                || registration.manifest.module_id != launch.module_id
                || registration.contract != digest_contract()
            {
                return Err(RuntimeError::Protocol("launch or role pin mismatch".into()));
            }
            self.router
                .authorize_launch(
                    transport.identity(),
                    &launch.module_id,
                    &launch.launch_id,
                    launch.generation,
                )
                .map_err(route)?;
            let binding = self
                .router
                .register(
                    transport.identity(),
                    registration.manifest,
                    &launch.launch_id,
                    launch.generation,
                )
                .map_err(route)?;
            let mut accepted = Frame::request("registration", serde_json::to_vec(&binding)?);
            accepted.kind = FrameKind::Response;
            transport.send(accepted).await?;
            self.binding = Some(binding);
            self.transport = Some(transport);
            Ok::<_, RuntimeError>(())
        }
        .await;
        if let Err(error) = result {
            let _ = self.supervisor.shutdown();
            return Err(error);
        }
        self.listener = Some(listener);
        Ok(launch)
    }
    pub fn submit(&mut self, request: WorkerRequest) -> Result<EffectIntent, RuntimeError> {
        self.fence()?;
        validate_id(&request.id)?;
        if request
            .deadline_unix_ms
            .is_some_and(|d| now_ms().map_or(true, |now| d <= now))
        {
            return Err(RuntimeError::Protocol("request deadline elapsed".into()));
        }
        match self.tools.resolve("digest", &request.pin)? {
            ToolReceipt::Available { .. } => {}
            ToolReceipt::Withdrawn { .. } => {
                return Err(RuntimeError::Protocol("tool withdrawn".into()));
            }
        }
        let bytes = serde_json::to_vec(&request)?;
        if let Some(old) = self.effects.get(&self.config.scope, &request.id)? {
            if old.payload != bytes || old.kind != "digest" {
                return Err(ContextError::Conflict.into());
            }
            match old.state {
                EffectState::Terminal => return Ok(old),
                EffectState::Uncertain | EffectState::Dispatched => {
                    return Err(RuntimeError::Uncertain(old.key));
                }
                EffectState::Prepared => {
                    if self.queued.contains(&request.id) {
                        return Ok(old);
                    }
                }
            }
        }
        let binding = self
            .binding
            .as_ref()
            .ok_or(RuntimeError::WorkerUnavailable)?;
        let transport = self
            .transport
            .as_ref()
            .ok_or(RuntimeError::WorkerUnavailable)?;
        self.runs.start(
            request.id.clone(),
            self.config.scope.clone(),
            request.pin.clone(),
        )?;
        self.router
            .enqueue(
                transport.identity(),
                binding,
                RouteCall {
                    id: request.id.clone(),
                    caller: PRINCIPAL.into(),
                    capability: "digest".into(),
                    capability_version: 1,
                    deadline_unix_ms: request.deadline_unix_ms,
                    payload: bytes.clone(),
                },
                now_ms()?,
            )
            .map_err(route)?;
        let intent = match self
            .effects
            .prepare(&self.config.scope, &request.id, "digest", &bytes)
        {
            Ok(intent) => intent,
            Err(error) => {
                let _ = self.router.cancel(binding, &request.id);
                return Err(error.into());
            }
        };
        self.queued.insert(request.id);
        self.relay_receipts()?;
        Ok(intent)
    }
    pub async fn dispatch_next(&mut self) -> Result<Option<String>, RuntimeError> {
        self.fence()?;
        let binding = self
            .binding
            .clone()
            .ok_or(RuntimeError::WorkerUnavailable)?;
        let (dispatch, expired) = self.router.dispatch(&binding, now_ms()?).map_err(route)?;
        for outcome in expired {
            self.queued.remove(&outcome.id);
            let prepared = self
                .effects
                .get(&self.config.scope, &outcome.id)?
                .ok_or(ContextError::Stale)?;
            let dispatched =
                self.effects
                    .begin_dispatch(&self.config.scope, &outcome.id, prepared.version)?;
            self.effects.complete(
                &self.config.scope,
                &outcome.id,
                dispatched.version,
                EffectObservation {
                    outcome: EffectOutcome::NotApplied,
                    evidence: "routing deadline elapsed before transport send".into(),
                },
            )?;
            self.runs.finish(
                &outcome.id,
                &self.config.scope,
                RunTerminal::Failed {
                    reason: "routing deadline elapsed".into(),
                },
            )?;
        }
        let Some(dispatch) = dispatch else {
            self.relay_receipts()?;
            return Ok(None);
        };
        let prepared = self
            .effects
            .get(&self.config.scope, &dispatch.call.id)?
            .ok_or(ContextError::Stale)?;
        let dispatched =
            self.effects
                .begin_dispatch(&self.config.scope, &dispatch.call.id, prepared.version)?;
        self.queued.remove(&dispatch.call.id);
        self.in_flight
            .insert(dispatch.call.id.clone(), dispatched.version);
        self.relay_receipts()?;
        let mut frame = Frame::request(&dispatch.call.id, dispatch.call.payload);
        frame.deadline_unix_ms = dispatch.call.deadline_unix_ms;
        if let Err(error) = self
            .transport
            .as_mut()
            .ok_or(RuntimeError::WorkerUnavailable)?
            .send(frame)
            .await
        {
            self.mark_in_flight_uncertain()?;
            return Err(error.into());
        }
        self.flush_backend_events().await?;
        Ok(Some(dispatch.call.id))
    }
    pub async fn receive_result(&mut self) -> Result<WorkerResult, RuntimeError> {
        self.fence()?;
        if self.in_flight.is_empty() {
            return Err(ContextError::Conflict.into());
        }
        let frame = match self
            .transport
            .as_mut()
            .ok_or(RuntimeError::WorkerUnavailable)?
            .receive()
            .await
        {
            Ok(frame) => frame,
            Err(error) => {
                self.mark_in_flight_uncertain()?;
                return Err(error.into());
            }
        };
        let result = self.accept_result(frame);
        if result.is_err() {
            self.mark_in_flight_uncertain()?;
        } else {
            self.flush_backend_events().await?;
        }
        result
    }
    fn accept_result(&mut self, frame: Frame) -> Result<WorkerResult, RuntimeError> {
        if frame.kind != FrameKind::Response {
            return Err(RuntimeError::Protocol(
                "worker did not return a result".into(),
            ));
        }
        let result: WorkerResult = serde_json::from_slice(&frame.payload)?;
        let version = *self
            .in_flight
            .get(&frame.correlation_id)
            .ok_or(ContextError::Stale)?;
        let intent = self
            .effects
            .get(&self.config.scope, &frame.correlation_id)?
            .ok_or(ContextError::Stale)?;
        let request: WorkerRequest = serde_json::from_slice(&intent.payload)?;
        if result.id != frame.correlation_id
            || result.pin != request.pin
            || result.request_digest != intent.payload_digest
            || result.digest != digest_bytes(&request.bytes)
        {
            return Err(RuntimeError::Protocol(
                "result identity or digest mismatch".into(),
            ));
        }
        let encoded = serde_json::to_vec(&result)?;
        if let Some(backend) = &self.backend {
            backend.put_result(&result)?;
        } else {
            let owner = self
                .owner
                .as_mut()
                .ok_or_else(|| RuntimeError::Protocol("owner missing".into()))?;
            owner.transaction(owner.epoch(),|tx|{tx.execute("INSERT INTO runtime_results VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET result=excluded.result",params![result.id,encoded])?;Ok(())})?;
        }
        self.effects.complete(
            &self.config.scope,
            &result.id,
            version,
            EffectObservation {
                outcome: EffectOutcome::Succeeded,
                evidence: format!("authenticated result SHA-256 {}", result.digest),
            },
        )?;
        self.runs.finish(
            &result.id,
            &self.config.scope,
            RunTerminal::Completed {
                result_digest: digest_bytes(&frame.payload),
            },
        )?;
        self.runs.deliver(
            &result.id,
            &self.config.scope,
            result.id.clone(),
            &frame.payload,
        )?;
        self.router
            .complete(
                self.binding
                    .as_ref()
                    .ok_or(RuntimeError::WorkerUnavailable)?,
                &result.id,
            )
            .map_err(route)?;
        self.in_flight.remove(&result.id);
        self.relay_receipts()?;
        Ok(result)
    }
    pub async fn execute(&mut self, request: WorkerRequest) -> Result<WorkerResult, RuntimeError> {
        let id = request.id.clone();
        let intent = self.submit(request)?;
        if intent.state == EffectState::Terminal {
            self.flush_backend_events().await?;
            return self.result(&id)?.ok_or_else(|| {
                RuntimeError::Protocol("terminal effect has no successful result".into())
            });
        }
        if !self.in_flight.is_empty() || self.queued.len() != 1 {
            return Err(ContextError::Conflict.into());
        }
        if self.dispatch_next().await?.as_deref() != Some(&id) {
            return Err(RuntimeError::Protocol("request was not dispatched".into()));
        }
        self.receive_result().await
    }
    pub fn result(&self, id: &str) -> Result<Option<WorkerResult>, RuntimeError> {
        validate_id(id)?;
        if !self
            .effects
            .get(&self.config.scope, id)?
            .is_some_and(|intent| {
                intent.state == EffectState::Terminal
                    && intent
                        .observation
                        .is_some_and(|o| o.outcome == EffectOutcome::Succeeded)
            })
        {
            return Ok(None);
        }
        if let Some(backend) = &self.backend {
            return Ok(backend.result(id)?);
        }
        let bytes: Option<Vec<u8>> = self
            .owner
            .as_ref()
            .ok_or_else(|| RuntimeError::Protocol("owner missing".into()))?
            .read(|db| {
                Ok(db
                    .query_row(
                        "SELECT result FROM runtime_results WHERE id=?1",
                        [id],
                        |r| r.get(0),
                    )
                    .optional()?)
            })?;
        bytes
            .map(|b| serde_json::from_slice(&b).map_err(RuntimeError::from))
            .transpose()
    }
    pub fn effect(&self, id: &str) -> Result<Option<EffectIntent>, RuntimeError> {
        self.fence()?;
        Ok(self.effects.get(&self.config.scope, id)?)
    }
    pub fn receipts(
        &self,
        after: Cursor,
        limit: usize,
    ) -> Result<Vec<EffectReceipt>, RuntimeError> {
        self.fence()?;
        Ok(self.effects.receipts(&self.config.scope, after, limit)?)
    }
    pub fn events(&self, after: u64, limit: u32) -> Result<Vec<Event>, RuntimeError> {
        self.fence()?;
        if self.backend.is_some() {
            return Err(RuntimeError::Protocol(
                "selected backend event replay requires events_async".into(),
            ));
        }
        Ok(self
            .bus
            .as_ref()
            .ok_or_else(|| RuntimeError::Protocol("bus missing".into()))?
            .replay(PRINCIPAL, STREAM, after, limit)?)
    }
    fn mark_in_flight_uncertain(&mut self) -> Result<(), RuntimeError> {
        for (id, version) in &mut self.in_flight {
            if self
                .effects
                .get(&self.config.scope, id)?
                .is_some_and(|i| i.state == EffectState::Dispatched)
            {
                let intent = self
                    .effects
                    .mark_uncertain(&self.config.scope, id, *version)?;
                *version = intent.version;
            }
        }
        self.relay_receipts()?;
        Ok(())
    }
    pub fn reconcile(
        &mut self,
        id: &str,
        observation: EffectObservation,
    ) -> Result<EffectIntent, RuntimeError> {
        self.fence()?;
        let old = self
            .effects
            .get(&self.config.scope, id)?
            .ok_or(ContextError::Stale)?;
        let intent = self
            .effects
            .reconcile(&self.config.scope, id, old.version, observation)?;
        if self.in_flight.remove(id).is_some() {
            self.router
                .complete(
                    self.binding
                        .as_ref()
                        .ok_or(RuntimeError::WorkerUnavailable)?,
                    id,
                )
                .map_err(route)?;
        }
        self.relay_receipts()?;
        Ok(intent)
    }
    pub fn drain(&mut self) -> Result<bool, RuntimeError> {
        self.fence()?;
        let Some(binding) = &self.binding else {
            return Ok(true);
        };
        let outcomes = self.router.drain(binding).map_err(route)?;
        for outcome in outcomes {
            self.queued.remove(&outcome.id);
            let old = self
                .effects
                .get(&self.config.scope, &outcome.id)?
                .ok_or(ContextError::Stale)?;
            let dispatched =
                self.effects
                    .begin_dispatch(&self.config.scope, &outcome.id, old.version)?;
            self.effects.complete(
                &self.config.scope,
                &outcome.id,
                dispatched.version,
                EffectObservation {
                    outcome: EffectOutcome::NotApplied,
                    evidence: "runtime drained before transport dispatch".into(),
                },
            )?;
            self.runs
                .finish(&outcome.id, &self.config.scope, RunTerminal::Cancelled)?;
        }
        let drained = self.router.is_drained(binding).map_err(route)?;
        self.relay_receipts()?;
        Ok(drained)
    }
    pub async fn shutdown(&mut self) -> Result<(), RuntimeError> {
        self.drain()?;
        self.mark_in_flight_uncertain()?;
        if self.in_flight.is_empty() {
            if let Some(transport) = self.transport.as_mut() {
                let mut frame = Frame::request("shutdown", vec![]);
                frame.kind = FrameKind::Cancel;
                let _ = transport.send(frame).await;
            }
        }
        self.supervisor.shutdown()?;
        self.transport.take();
        self.listener.take();
        self.binding.take();
        self.flush_backend_events().await?;
        Ok(())
    }
    pub fn relay_receipts(&mut self) -> Result<usize, RuntimeError> {
        self.fence()?;
        let mut count = 0;
        loop {
            let cursor = self.effects.acknowledged(&self.config.scope, RELAY)?;
            let batch = self.effects.receipts(&self.config.scope, cursor, 128)?;
            if batch.is_empty() {
                break;
            }
            for receipt in batch {
                let payload = serde_json::to_vec(&receipt)?;
                let key = format!(
                    "receipt-{}-{}",
                    receipt.cursor.epoch, receipt.cursor.sequence
                );
                if let Some(backend) = &self.backend {
                    backend.enqueue_receipt(&receipt)?;
                } else {
                    self.bus
                        .as_mut()
                        .ok_or_else(|| RuntimeError::Protocol("bus missing".into()))?
                        .append_once(PRINCIPAL, STREAM, &key, &payload)?;
                }
                self.effects
                    .acknowledge(&self.config.scope, RELAY, receipt.cursor)?;
                count += 1;
            }
        }
        Ok(count)
    }
}
impl Drop for RuntimeService {
    fn drop(&mut self) {
        let _ = self.supervisor.shutdown();
    }
}
fn check_file(path: &Path) -> Result<(), RuntimeError> {
    match fs::symlink_metadata(path) {
        Ok(meta) => {
            use std::os::unix::fs::MetadataExt;
            if !meta.is_file() || meta.nlink() != 1 {
                return Err(RuntimeError::Protocol(
                    "operational path must be an unaliased regular file".into(),
                ));
            }
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

pub fn worker_main() -> Result<(), RuntimeError> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(worker_from_env())
}

pub async fn worker_from_env() -> Result<(), RuntimeError> {
    let encoded = std::env::var(WORKER_ENV)
        .map_err(|_| RuntimeError::Protocol("worker configuration missing".into()))?;
    let config: WorkerConfiguration = serde_json::from_str(&encoded)?;
    let launch = LaunchIdentity {
        module_id: std::env::var("HYPERMIND_MODULE_ID")
            .map_err(|_| RuntimeError::Protocol("module identity missing".into()))?,
        launch_id: std::env::var("HYPERMIND_LAUNCH_ID")
            .map_err(|_| RuntimeError::Protocol("launch identity missing".into()))?,
        generation: std::env::var("HYPERMIND_GENERATION")
            .map_err(|_| RuntimeError::Protocol("generation missing".into()))?
            .parse()
            .map_err(|_| RuntimeError::Protocol("generation invalid".into()))?,
        pid: std::process::id(),
    };
    let key = SigningKey::from_bytes(&config.signing_key);
    let peer_key = ed25519_dalek::VerifyingKey::from_bytes(&config.peer_key)
        .map_err(|_| RuntimeError::Protocol("peer key invalid".into()))?;
    serve_worker(
        &config.socket,
        Credentials {
            scope: config.scope,
            signing_key: key,
            peer_key,
        },
        launch,
        Limits::default(),
    )
    .await
}
pub async fn serve_worker(
    socket: impl AsRef<Path>,
    credentials: Credentials,
    launch: LaunchIdentity,
    limits: Limits,
) -> Result<(), RuntimeError> {
    let scope = credentials.scope.clone();
    let mut transport =
        UnixTransport::connect(socket, credentials, ReplayGuard::default(), limits).await?;
    let manifest = ModuleManifest {
        module_id: launch.module_id.clone(),
        protocol_version: 1,
        capabilities: BTreeMap::from([("digest".into(), 1)]),
        max_calls: 4,
        max_bytes: 1024 * 1024,
        queue_calls: 32,
        queue_bytes: 4 * 1024 * 1024,
    };
    let registration = Registration {
        module_id: launch.module_id,
        launch_id: launch.launch_id,
        generation: launch.generation,
        manifest,
        contract: digest_contract(),
    };
    transport
        .send(Frame::request(
            "registration",
            serde_json::to_vec(&registration)?,
        ))
        .await?;
    let ack = transport.receive().await?;
    if ack.kind != FrameKind::Response || ack.correlation_id != "registration" {
        return Err(RuntimeError::Protocol("registration refused".into()));
    }
    let _: Binding = serde_json::from_slice(&ack.payload)?;
    let mut tools = ToolRegistry::default();
    tools.register(digest_contract())?;
    let mut runs = RunRegistry::default();
    loop {
        let frame = transport.receive().await?;
        if frame.kind == FrameKind::Cancel && frame.correlation_id == "shutdown" {
            break;
        }
        let process = (|| {
            if frame.kind != FrameKind::Request {
                return Err(RuntimeError::Protocol("request frame required".into()));
            }
            let request: WorkerRequest = serde_json::from_slice(&frame.payload)?;
            if request.id != frame.correlation_id
                || request.deadline_unix_ms != frame.deadline_unix_ms
            {
                return Err(RuntimeError::Protocol("request metadata mismatch".into()));
            }
            tools.resolve("digest", &request.pin)?;
            runs.start(request.id.clone(), scope.clone(), request.pin.clone())?;
            let result = WorkerResult {
                id: request.id.clone(),
                pin: request.pin,
                request_digest: digest_bytes(&frame.payload),
                digest: digest_bytes(&request.bytes),
            };
            let payload = serde_json::to_vec(&result)?;
            runs.finish(
                &request.id,
                &scope,
                RunTerminal::Completed {
                    result_digest: digest_bytes(&payload),
                },
            )?;
            runs.deliver(&request.id, &scope, request.id.clone(), &payload)?;
            Ok::<_, RuntimeError>(payload)
        })();
        let reply = match process {
            Ok(payload) => Frame {
                kind: FrameKind::Response,
                ..Frame::request(frame.correlation_id, payload)
            },
            Err(error) => Frame {
                kind: FrameKind::Failure,
                failure: Some(TypedFailure {
                    code: FailureCode::Protocol,
                    effect: Effect::NotApplied,
                    message: error.to_string(),
                }),
                ..Frame::request(frame.correlation_id, vec![])
            },
        };
        transport.send(reply).await?;
    }
    Ok(())
}
