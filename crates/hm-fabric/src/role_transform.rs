use crate::{
    role_dispatch::{RoleWireRequest, handoff},
    role_store::*,
    roles::{RoleVersion, RunTerminal, TransformDeclaration, TransformInput, TransformRegistry},
    routing::{Binding, ModuleManifest, RouteCall, Router},
    supervisor::{ProcessSpec, Supervisor},
    transport::{Credentials, Frame, FrameKind, Limits, ReplayGuard, UnixTransport},
};
use ed25519_dalek::{SigningKey, VerifyingKey};
use hm_context::{
    history::SourceHistory,
    types::{Authority, ContextBlock, MessagePart, Scope, digest_bytes},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
const CONFIG: &str = "HM_FABRIC_TRANSFORM_CONFIG";
static SEQUENCE: AtomicU64 = AtomicU64::new(1);
#[derive(Debug, thiserror::Error)]
#[error("transform refused: {0}")]
pub struct TransformError(pub String);
type Result<T> = std::result::Result<T, TransformError>;
fn err(e: impl std::fmt::Debug) -> TransformError {
    TransformError(format!("{e:?}"))
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
pub struct TransformPolicy {
    pub id: String,
    pub semantic_version: String,
    pub input_authorities: Vec<Authority>,
    pub authorities: Vec<Authority>,
    pub hooks: BTreeSet<String>,
    pub max_input_tokens: u64,
    pub max_output_tokens: u64,
}
impl TransformPolicy {
    pub fn descriptor(&self) -> Result<RoleDescriptor> {
        let pin=RoleVersion::new(b"HyperMind declared native TransformRegistry apply v1: ordered authority/token selection; required blocks preserved or refuse; no text or authority rewrite",&serde_json::to_vec(self).map_err(err)?);
        let transform = TransformDeclaration {
            id: self.id.clone(),
            pin: pin.clone(),
            authorities: self.authorities.clone(),
            hooks: self.hooks.clone(),
            max_input_tokens: self.max_input_tokens,
            max_output_tokens: self.max_output_tokens,
            may_reorder: false,
        };
        let mut native = TransformRegistry::default();
        native.register(transform.clone()).map_err(err)?;
        if self.input_authorities.is_empty()
            || self
                .authorities
                .iter()
                .any(|a| !self.input_authorities.contains(a))
            || self.max_output_tokens > self.max_input_tokens
        {
            return Err(err("output policy cannot widen input ceiling"));
        }
        Ok(RoleDescriptor {
            id: self.id.clone(),
            kind: RoleKind::Transform,
            pin,
            semantic_version: self.semantic_version.clone(),
            capabilities: BTreeMap::from([("transform".into(), 1)]),
            grants: RoleGrants {
                operations: BTreeSet::from(["transform".into()]),
                authorities: self.input_authorities.clone(),
                hooks: self.hooks.clone(),
                max_input_tokens: self.max_input_tokens,
                max_output_tokens: self.max_output_tokens,
                may_reorder: false,
            },
            transform: Some(transform),
        })
    }
}
pub fn history_fence(history: &SourceHistory) -> Result<SourceFence> {
    let visible = history.visible_messages();
    let first = visible.first().ok_or_else(|| err("empty history"))?;
    let last = visible.last().ok_or_else(|| err("empty history"))?;
    Ok(SourceFence {
        epoch: history.cursor().epoch,
        generation: history.cursor().sequence,
        digest: digest_bytes(&history.export_canonical().map_err(err)?),
        start: first.ordinal,
        end: last
            .ordinal
            .checked_add(1)
            .ok_or_else(|| err("range overflow"))?,
    })
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransformBoundary {
    pub current_work_ids: BTreeSet<String>,
}
fn native_sources(history: &SourceHistory, work: &RoleWork) -> Result<()> {
    let boundary: TransformBoundary = serde_json::from_slice(&work.payload).map_err(err)?;
    if boundary
        .current_work_ids
        .iter()
        .any(|id| !work.blocks.iter().any(|b| &b.id == id && b.required))
    {
        return Err(err(
            "current work must remain required at transform boundary",
        ));
    }

    if history_fence(history)? != work.source {
        return Err(err("native source cursor/digest changed"));
    }
    let mut seen = BTreeSet::new();
    let mut previous = None;
    for block in &work.blocks {
        if !seen.insert(&block.id) || block.provenance.len() != 1 {
            return Err(err("duplicate or incomplete native block coverage"));
        }
        let source = history.message(&block.id).map_err(err)?;
        if previous.is_some_and(|old| old >= source.ordinal) {
            return Err(err("native input order changed"));
        }
        previous = Some(source.ordinal);
        if !history.visible_messages().iter().any(|s| s.id == source.id)
            || block.provenance[0] != history.source_span(&source.id).map_err(err)?
            || block.authority != source.authority
            || source
                .parts
                .iter()
                .any(|p| !matches!(p, MessagePart::Text { .. }))
        {
            return Err(err("native block identity/authority/coverage mismatch"));
        }
        let text = source
            .parts
            .iter()
            .filter_map(|p| match p {
                MessagePart::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        if block.text != text {
            return Err(err("native block text changed"));
        }
        history
            .recover(history.scope(), &block.provenance[0])
            .map_err(err)?;
    }
    Ok(())
}
fn apply(descriptor: &RoleDescriptor, scope: &Scope, work: &RoleWork) -> Result<Vec<ContextBlock>> {
    let boundary: TransformBoundary = serde_json::from_slice(&work.payload).map_err(err)?;
    if boundary
        .current_work_ids
        .iter()
        .any(|id| !work.blocks.iter().any(|b| &b.id == id && b.required))
    {
        return Err(err("current work must remain required"));
    }

    let mut native = TransformRegistry::default();
    native
        .register(
            descriptor
                .transform
                .clone()
                .ok_or_else(|| err("missing transform contract"))?,
        )
        .map_err(err)?;
    let input = TransformInput {
        scope: scope.clone(),
        hook: work.hook.clone(),
        blocks: work.blocks.clone(),
    };
    let output = native
        .apply(&descriptor.id, &descriptor.pin, input.clone())
        .map_err(err)?;
    native
        .validate_output(&descriptor.id, &descriptor.pin, &input, &output)
        .map_err(err)?;
    Ok(output.blocks)
}
#[derive(Clone)]
pub struct TransformWorkerConfig {
    pub root: PathBuf,
    pub registry_path: PathBuf,
    pub scope: Scope,
    pub host_key: [u8; 32],
    pub worker_key: [u8; 32],
    pub policy: TransformPolicy,
}
#[derive(Serialize, Deserialize)]
struct Configuration {
    socket: PathBuf,
    registry_path: PathBuf,
    scope: Scope,
    signing_key: [u8; 32],
    peer_key: [u8; 32],
    policy: TransformPolicy,
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
pub struct PreparedTransformResult {
    ticket: DispatchTicket,
    output: RoleOutput,
    scope: Scope,
}
pub struct TransformWorkerSession {
    supervisor: Supervisor,
    router: Router,
    binding: Binding,
    transport: UnixTransport,
    socket: PathBuf,
    scope: Scope,
    policy: TransformPolicy,
    in_flight: Option<DispatchTicket>,
}
impl TransformWorkerSession {
    pub async fn launch(
        mut spec: ProcessSpec,
        config: TransformWorkerConfig,
        registry: &DurableRoleRegistry,
        limits: Limits,
    ) -> Result<Self> {
        if registry.scope() != &config.scope {
            return Err(err("scope mismatch"));
        }
        let descriptor = config.policy.descriptor()?;
        let catalog = registry
            .catalog(&descriptor.id)
            .map_err(err)?
            .ok_or_else(|| err("missing policy"))?;
        if catalog.descriptor != descriptor || catalog.withdrawal.is_some() {
            return Err(err("policy withdrawn or changed"));
        }
        let socket = std::fs::canonicalize(&config.root)
            .map_err(err)?
            .join(format!(
                "transform-{}-{}.sock",
                std::process::id(),
                SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
        let listener = tokio::net::UnixListener::bind(&socket).map_err(err)?;
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600)).map_err(err)?;
        if spec.env.contains_key(CONFIG) {
            return Err(err("configuration owner controlled"));
        }
        let c = Configuration {
            socket: socket.clone(),
            registry_path: std::fs::canonicalize(&config.registry_path).map_err(err)?,
            scope: config.scope.clone(),
            signing_key: config.worker_key,
            peer_key: SigningKey::from_bytes(&config.host_key)
                .verifying_key()
                .to_bytes(),
            policy: config.policy.clone(),
        };
        spec.env
            .insert(CONFIG.into(), serde_json::to_string(&c).map_err(err)?);
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
            let f = transport.receive().await.map_err(err)?;
            let reg: Registration = serde_json::from_slice(&f.payload).map_err(err)?;
            if f.kind != FrameKind::Request
                || f.correlation_id != "registration"
                || reg.module != launch.module_id
                || reg.launch != launch.launch_id
                || reg.generation != launch.generation
                || reg.pid != launch.pid
                || reg.descriptor != descriptor
                || reg.manifest.module_id != launch.module_id
                || reg.manifest.capabilities != descriptor.capabilities
                || reg.manifest.max_calls != 1
            {
                return Err(err("registration identity/policy mismatch"));
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
            Ok::<_, TransformError>((transport, router, binding))
        }
        .await;
        match accepted {
            Ok((transport, router, binding)) => Ok(Self {
                supervisor,
                router,
                binding,
                transport,
                socket,
                scope: config.scope,
                policy: config.policy,
                in_flight: None,
            }),
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
        history: &SourceHistory,
    ) -> Result<DispatchTicket> {
        if self.in_flight.is_some()
            || registry.scope() != &self.scope
            || history.scope() != &self.scope
        {
            return Err(err("busy or scope mismatch"));
        }
        let work = registry
            .get(id)
            .map_err(err)?
            .ok_or_else(|| err("missing work"))?;
        if work.state != WorkState::Approved || work.descriptor != self.policy.descriptor()? {
            return Err(err("approval/policy revision mismatch"));
        }
        native_sources(history, &work.work)?;
        apply(&work.descriptor, &self.scope, &work.work)?;
        self.router
            .enqueue(
                self.transport.identity(),
                &self.binding,
                RouteCall {
                    id: id.into(),
                    caller: "owner".into(),
                    capability: "transform".into(),
                    capability_version: 1,
                    deadline_unix_ms: Some(work.work.deadline_ns as u64 / 1_000_000),
                    payload: serde_json::to_vec(&work.work).map_err(err)?,
                },
                now() as u64 / 1_000_000,
            )
            .map_err(err)?;
        let (call, _) = self
            .router
            .dispatch(&self.binding, now() as u64 / 1_000_000)
            .map_err(err)?;
        if call.is_none() {
            return Err(err("no routing credit"));
        }
        let ticket =
            match handoff(registry, &mut self.transport, id, &work.work.source, now()).await {
                Ok(t) => t,
                Err(e) => {
                    let _ = self.router.complete(&self.binding, id);
                    return Err(err(e));
                }
            };
        self.in_flight = Some(ticket.clone());
        let f = self.transport.receive().await.map_err(err)?;
        if f.kind != FrameKind::Response
            || f.correlation_id != id
            || f.payload != b"approved_transform_ready"
        {
            let _ = registry.mark_uncertain(&ticket);
            return Err(err("transform readiness acknowledgement missing"));
        }
        Ok(ticket)
    }
    pub async fn collect(
        &mut self,
        registry: &mut DurableRoleRegistry,
    ) -> Result<PreparedTransformResult> {
        let ticket = self.in_flight.clone().ok_or_else(|| err("no transform"))?;
        let mut ack = Frame::request(&ticket.id, vec![]);
        ack.kind = FrameKind::Response;
        let collected = async {
            self.transport.send(ack).await.map_err(err)?;
            let frame = self.transport.receive().await.map_err(err)?;
            if frame.kind != FrameKind::Response || frame.correlation_id != ticket.id {
                return Err(err("result correlation mismatch"));
            }
            let output: RoleOutput = serde_json::from_slice(&frame.payload).map_err(err)?;
            let record = registry
                .get(&ticket.id)
                .map_err(err)?
                .ok_or_else(|| err("missing original"))?;
            let expected = apply(&record.descriptor, &self.scope, &record.work)?;
            if !matches!(output.terminal, RunTerminal::Completed { .. })
                || expected != output.blocks
                || serde_json::to_vec(&output.blocks).map_err(err)? != output.payload
            {
                return Err(err("native narrowing semantics mismatch"));
            }
            Ok(PreparedTransformResult {
                ticket: ticket.clone(),
                output,
                scope: self.scope.clone(),
            })
        }
        .await;
        if collected.is_err() {
            let _ = registry.mark_uncertain(&ticket);
        }
        let _ = self.router.complete(&self.binding, &ticket.id);
        self.in_flight = None;
        collected
    }
    pub fn commit(
        &self,
        registry: &mut DurableRoleRegistry,
        history: &SourceHistory,
        result: PreparedTransformResult,
    ) -> Result<WorkRecord> {
        if registry.scope() != &result.scope
            || history.scope() != &result.scope
            || result.scope != self.scope
        {
            return Err(err("prepared result scope mismatch"));
        }
        registry
            .complete(
                &result.ticket,
                &history_fence(history)?,
                result.output,
                now(),
            )
            .map_err(err)
    }
    pub async fn receive(
        &mut self,
        registry: &mut DurableRoleRegistry,
        history: &SourceHistory,
    ) -> Result<WorkRecord> {
        let result = self.collect(registry).await?;
        self.commit(registry, history, result)
    }
    pub fn consume(
        &self,
        registry: &DurableRoleRegistry,
        id: &str,
        history: &SourceHistory,
    ) -> Result<Vec<ContextBlock>> {
        let record = registry
            .get(id)
            .map_err(err)?
            .ok_or_else(|| err("missing transform"))?;
        native_sources(history, &record.work)?;
        let catalog = registry
            .catalog(&self.policy.id)
            .map_err(err)?
            .ok_or_else(|| err("missing catalog"))?;
        if record.state != WorkState::Terminal
            || record.descriptor != self.policy.descriptor()?
            || catalog.descriptor != record.descriptor
            || catalog.withdrawal.is_some()
            || record.ticket.as_ref().map(|t| t.catalog_revision) != Some(catalog.revision)
            || record.work.deadline_ns <= now()
        {
            return Err(err("current native consumer fence refused"));
        }
        let output = record.output.ok_or_else(|| err("missing output"))?;
        if !matches!(output.terminal, RunTerminal::Completed { .. })
            || output.blocks != apply(&catalog.descriptor, &self.scope, &record.work)?
        {
            return Err(err("transform result unavailable"));
        }
        Ok(output.blocks)
    }
    pub fn kill_worker(&mut self, registry: &mut DurableRoleRegistry) -> Result<()> {
        if let Some(t) = &self.in_flight {
            registry.mark_uncertain(t).map_err(err)?;
        }
        self.supervisor.shutdown().map_err(err)
    }
    pub fn shutdown(&mut self) -> Result<()> {
        self.supervisor.shutdown().map_err(err)
    }
}
impl Drop for TransformWorkerSession {
    fn drop(&mut self) {
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
fn approved(c: &Configuration, r: &RoleWireRequest) -> Result<RoleDescriptor> {
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
        .ok_or_else(|| err("missing approved work"))?;
    let catalog = s
        .catalog
        .get(&r.work.role_id)
        .ok_or_else(|| err("missing catalog"))?;
    let expected = c.policy.descriptor()?;
    if s.scope != c.scope
        || u64::try_from(epoch).map_err(err)? != r.ticket.writer_epoch
        || w.state != WorkState::Dispatched
        || w.work != r.work
        || w.ticket.as_ref() != Some(&r.ticket)
        || w.approval_revision != Some(catalog.revision)
        || catalog.withdrawal.is_some()
        || catalog.descriptor != expected
        || w.descriptor != expected
        || r.ticket.work_digest != digest_bytes(&serde_json::to_vec(&r.work).map_err(err)?)
        || r.work.deadline_ns <= now()
    {
        return Err(err("persisted transform approval refused"));
    }
    Ok(expected)
}
pub async fn transform_worker_from_env() -> Result<()> {
    let c: Configuration =
        serde_json::from_str(&std::env::var(CONFIG).map_err(err)?).map_err(err)?;
    let descriptor = c.policy.descriptor()?;
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
        Limits::default(),
    )
    .await
    .map_err(err)?;
    let registration = Registration {
        module: module.clone(),
        launch,
        generation,
        pid: std::process::id(),
        descriptor,
        manifest: ModuleManifest {
            module_id: module,
            protocol_version: 1,
            capabilities: BTreeMap::from([("transform".into(), 1)]),
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
        return Err(err("registration refused"));
    }
    loop {
        let f = transport.receive().await.map_err(err)?;
        if f.kind == FrameKind::Cancel && f.correlation_id == "shutdown" {
            return Ok(());
        }
        if f.kind != FrameKind::Request {
            return Err(err("transform request required"));
        }
        let request: RoleWireRequest = serde_json::from_slice(&f.payload).map_err(err)?;
        if request.work.id != f.correlation_id {
            return Err(err("request correlation mismatch"));
        }
        approved(&c, &request)?;
        let mut ready = Frame::request(&request.work.id, b"approved_transform_ready".to_vec());
        ready.kind = FrameKind::Response;
        transport.send(ready).await.map_err(err)?;
        let ack = transport.receive().await.map_err(err)?;
        if ack.kind != FrameKind::Response
            || ack.correlation_id != request.work.id
            || !ack.payload.is_empty()
        {
            return Err(err("owner dispatch acknowledgement required"));
        }
        let descriptor = approved(&c, &request)?;
        let blocks = apply(&descriptor, &c.scope, &request.work)?;
        let payload = serde_json::to_vec(&blocks).map_err(err)?;
        let output = RoleOutput {
            terminal: RunTerminal::Completed {
                result_digest: digest_bytes(&payload),
            },
            payload,
            blocks,
            grants: RoleGrants {
                authorities: descriptor
                    .transform
                    .as_ref()
                    .ok_or_else(|| err("missing transform contract"))?
                    .authorities
                    .clone(),
                ..descriptor.grants
            },
        };
        let mut reply = Frame::request(&request.work.id, serde_json::to_vec(&output).map_err(err)?);
        reply.kind = FrameKind::Response;
        transport.send(reply).await.map_err(err)?;
    }
}
