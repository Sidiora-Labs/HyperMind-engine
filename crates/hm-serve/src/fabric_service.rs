use crate::actor::ActorEngine;
use hm_context::{ContextError, Cursor, Scope, validate_id};
use hm_fabric::{
    artifacts::VerifiedArtifact,
    backend_config::BackendReadiness,
    backend_runtime::{BackendRuntime, BackendRuntimeError, RecordChange, RuntimeRecordStore},
    effects::{EffectIntent, EffectObservation, EffectState},
    role_store::{DurableRoleRegistry, RoleError},
    runtime::{RuntimeConfig, RuntimeError, RuntimeService, WorkerRequest},
    supervisor::ProcessSpec,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::sync::{Mutex as AsyncMutex, oneshot, watch};

#[derive(Debug, thiserror::Error)]
pub enum FabricError {
    #[error(transparent)]
    Context(#[from] ContextError),
    #[error(transparent)]
    Backend(#[from] BackendRuntimeError),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    Role(#[from] RoleError),
    #[error(transparent)]
    Ledger(#[from] hm_core::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("cancelled owned runtime operation {effect_id}; outcome requires reconciliation")]
    Cancelled { effect_id: String },
    #[error("trusted original effect evidence unavailable for {effect_id}")]
    EvidenceUnavailable { effect_id: String },
}
pub trait EffectEvidenceObserver: Send + Sync {
    fn observe<'a>(
        &'a self,
        scope: &'a Scope,
        intent: &'a EffectIntent,
    ) -> Pin<Box<dyn Future<Output = Result<Option<EffectObservation>, FabricError>> + Send + 'a>>;
}
#[derive(Default)]
pub struct TrustedMetadata {
    pub artifacts: Vec<VerifiedArtifact>,
    pub roles: Option<(Mutex<DurableRoleRegistry>, Vec<String>)>,
    pub evidence: Option<Arc<dyn EffectEvidenceObserver>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum FabricAction {
    Dispatch {
        operation: String,
        bytes: Vec<u8>,
        timeout_ms: u64,
    },
    Cancel {
        effect_id: String,
    },
    Inspect {
        #[serde(default)]
        after: Cursor,
        limit: usize,
    },
    Reconcile {
        effect_id: String,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FabricRequest {
    pub version: u32,
    pub scope: Scope,
    pub request_id: String,
    pub action: FabricAction,
}
struct Active {
    id: String,
    phase: String,
    cancel: watch::Sender<bool>,
}
pub struct FabricService {
    scope: Scope,
    max_wire_bytes: usize,
    max_input_bytes: usize,
    runtime: Arc<AsyncMutex<RuntimeService>>,
    records: RuntimeRecordStore,
    available: Arc<AtomicBool>,
    worker_stopped: Arc<AtomicBool>,
    readiness: BackendReadiness,
    active: Arc<Mutex<Option<Active>>>,
    metadata: TrustedMetadata,
}
impl FabricService {
    pub async fn start(
        config: RuntimeConfig,
        backend: BackendRuntime,
        worker: ProcessSpec,
        metadata: TrustedMetadata,
    ) -> Result<Self, FabricError> {
        config.scope.validate()?;
        if &config.scope != backend.scope()
            || metadata
                .artifacts
                .iter()
                .any(|artifact| artifact.manifest().scope != config.scope)
            || metadata.roles.as_ref().is_some_and(|(registry, _)| {
                registry
                    .lock()
                    .map_or(true, |registry| registry.scope() != &config.scope)
            })
        {
            return Err(ContextError::ScopeMismatch.into());
        }
        let max_wire_bytes = config.limits.max_frame_bytes;
        let empty = WorkerRequest::digest("\"".repeat(hm_context::MAX_IDENTIFIER_BYTES), vec![]);
        let overhead = serialized_wire_bound(&config.scope, &empty)?;
        let max_input_bytes = max_wire_bytes.saturating_sub(overhead) / 64;
        if max_input_bytes == 0 {
            return Err(ContextError::Capacity.into());
        }
        let max_input_bytes = max_input_bytes.min(131072);
        let records = backend.record_store();
        let scope = config.scope.clone();
        let mut runtime = RuntimeService::from_backends(config, backend)?;
        let readiness = runtime
            .backend_readiness()?
            .ok_or_else(|| ContextError::Unavailable("selected runtime backend".into()))?;
        runtime.launch_worker(worker).await?;
        Ok(Self {
            scope,
            max_wire_bytes,
            max_input_bytes,
            runtime: Arc::new(AsyncMutex::new(runtime)),
            records,
            available: Arc::new(AtomicBool::new(true)),
            worker_stopped: Arc::new(AtomicBool::new(false)),
            readiness,
            active: Arc::new(Mutex::new(None)),
            metadata,
        })
    }
    fn authorize(&self, owner: &Scope) -> Result<(), FabricError> {
        if owner != &self.scope {
            return Err(ContextError::ScopeMismatch.into());
        }
        Ok(())
    }
    fn active(&self) -> Result<std::sync::MutexGuard<'_, Option<Active>>, FabricError> {
        self.active
            .lock()
            .map_err(|_| ContextError::Unavailable("runtime control".into()).into())
    }
    fn descriptors(&self) -> Result<Value, FabricError> {
        let artifacts:Vec<_>=self.metadata.artifacts.iter().map(|artifact|{let manifest=artifact.manifest();json!({"module_id":manifest.module_id,"version":manifest.version,"scope":manifest.scope,"manifest_digest":artifact.digest(),"capabilities":manifest.capabilities,"signature_verified":true,"execution_binding":"not_connected"})}).collect();
        let roles = if let Some((registry, ids)) = &self.metadata.roles {
            ids.iter()
                .map(|id| Ok(json!({"id":id,"catalog":registry.lock().map_err(|_|ContextError::Unavailable("role catalog".into()))?.catalog(id)?})))
                .collect::<Result<Vec<_>, FabricError>>()?
        } else {
            vec![]
        };
        Ok(
            json!({"backend":self.readiness,"effect_authority":"selected_operational","max_active_operations":1,"cancel_semantics":"stops the sole owned worker and preserves uncertain effects; recreate trusted runtime before new work","operations":[{"operation":"digest","available":self.available.load(Ordering::Acquire),"contract":hm_fabric::runtime::digest_contract(),"max_input_bytes":self.max_input_bytes,"max_timeout_ms":30000}],"artifacts":artifacts,"roles":roles,"families":[{"family":"digest_worker","available":self.available.load(Ordering::Acquire)},{"family":"declared_tool_dispatch","available":false,"reason":"declared tool session is not attached to this service"},{"family":"model_runner","available":false,"reason":"runner session is not attached to this service"},{"family":"provider_compaction","available":false,"reason":"compaction session is not attached to this service"},{"family":"transform","available":false,"reason":"transform session is not attached to this service"},{"family":"artifact_lifecycle","available":false,"reason":"verified metadata inspection only"},{"family":"remote_federation","available":false,"reason":"remote routes are not attached to this service"}]}),
        )
    }
    pub async fn inspect(
        &self,
        owner: &Scope,
        after: Cursor,
        limit: usize,
    ) -> Result<Value, FabricError> {
        self.authorize(owner)?;
        after.validate()?;
        if limit == 0 || limit > 256 {
            return Err(ContextError::Capacity.into());
        }
        let active = self
            .active()?
            .as_ref()
            .map(|active| json!({"id":active.id,"phase":active.phase}));
        let Ok(runtime) = self.runtime.try_lock() else {
            return Ok(
                json!({"version":1,"scope":self.scope,"runtime_busy":true,"active_effect":active,"descriptors":self.descriptors()?,"receipts_available":false,"reason":"owned runtime operation is in progress"}),
            );
        };
        let ready = runtime.backend_readiness()?;
        let receipts = runtime.receipts(after, limit)?;
        let publications = runtime.backend_publications(limit as u32)?;
        let mut descriptors = self.descriptors()?;
        descriptors["backend"] = serde_json::to_value(ready)?;
        Ok(
            json!({"version":1,"scope":self.scope,"runtime_busy":false,"active_effect":active,"writer_epoch":runtime.writer_epoch(),"descriptors":descriptors,"receipts_available":true,"receipts":receipts,"publications":publications}),
        )
    }
    pub async fn execute(
        &self,
        actor: &ActorEngine,
        owner: &Scope,
        request: FabricRequest,
    ) -> Result<Value, FabricError> {
        self.authorize(owner)?;
        if request.scope != self.scope {
            return Err(ContextError::ScopeMismatch.into());
        }
        if request.version != 1 {
            return Err(hm_core::Error::new(hm_core::ErrorCode::ProtocolVersion).into());
        }
        validate_id(&request.request_id)?;
        actor.stats().await?;
        let logical_digest = hm_context::digest_bytes(&serde_json::to_vec(&request)?);
        match request.action {
            FabricAction::Inspect { after, limit } => self.inspect(owner, after, limit).await,
            FabricAction::Dispatch {
                operation,
                bytes,
                timeout_ms,
            } => {
                if operation != "digest" {
                    return Err(
                        ContextError::Unavailable("configured fabric operation".into()).into(),
                    );
                }
                if bytes.len() > self.max_input_bytes || timeout_ms == 0 || timeout_ms > 30000 {
                    return Err(ContextError::Capacity.into());
                }
                let wire_bound = serialized_wire_bound(
                    &self.scope,
                    &WorkerRequest::digest(request.request_id.clone(), bytes.clone()),
                )?;
                if wire_bound > self.max_wire_bytes {
                    return Err(ContextError::Capacity.into());
                }
                if !self.available.load(Ordering::Acquire) {
                    return Err(RuntimeError::WorkerUnavailable.into());
                }
                let key = format!(
                    "fabric-request-{}",
                    hm_context::digest_bytes(&serde_json::to_vec(&(
                        &self.scope,
                        &request.request_id
                    ))?)
                );
                if let Some(record) = self.records.get(&key)? {
                    if record.value["digest"] != logical_digest {
                        return Err(ContextError::Conflict.into());
                    }
                } else {
                    self.records.compare_exchange(RecordChange {
                        key,
                        expected: None,
                        value: json!({"digest":logical_digest}),
                    })?;
                }
                let (cancel, cancelled) = watch::channel(false);
                {
                    let mut active = self.active()?;
                    if active.is_some() {
                        return Err(ContextError::Conflict.into());
                    }
                    *active = Some(Active {
                        id: request.request_id.clone(),
                        phase: "admitting".into(),
                        cancel,
                    });
                }
                let runtime = self.runtime.clone();
                let active = self.active.clone();
                let available = self.available.clone();
                let worker_stopped = self.worker_stopped.clone();
                let scope = self.scope.clone();
                let id = request.request_id;
                let (sender, receiver) = oneshot::channel();
                std::thread::spawn(move || {
                    let executor = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build();
                    let outcome = match executor {
                        Ok(executor) => executor.block_on(async {
                            let outcome = dispatch_operation(
                                runtime,
                                active.clone(),
                                available.clone(),
                                worker_stopped.clone(),
                                scope,
                                id,
                                bytes,
                                timeout_ms,
                                cancelled,
                            )
                            .await;
                            if matches!(
                                &outcome,
                                Err(FabricError::Runtime(
                                    RuntimeError::Transport(_)
                                        | RuntimeError::Supervisor(_)
                                        | RuntimeError::WorkerUnavailable
                                        | RuntimeError::Protocol(_)
                                ))
                            ) {
                                available.store(false, Ordering::Release);
                            }
                            outcome
                        }),
                        Err(error) => Err(FabricError::Context(ContextError::Unavailable(
                            error.to_string(),
                        ))),
                    };
                    if let Ok(mut state) = active.lock() {
                        *state = None;
                    }
                    let _ = sender.send(outcome);
                });
                receiver
                    .await
                    .map_err(|_| ContextError::Unavailable("owned runtime dispatch".into()))?
            }
            FabricAction::Cancel { effect_id } => {
                validate_id(&effect_id)?;
                {
                    let active = self.active()?;
                    let current = active
                        .as_ref()
                        .filter(|active| active.id == effect_id)
                        .ok_or(ContextError::Stale)?;
                    if current.phase == "reconciling" {
                        return Err(ContextError::Unavailable(
                            "reconciliation cancellation unsupported".into(),
                        )
                        .into());
                    }
                    current.cancel.send(true).map_err(|_| ContextError::Stale)?;
                }
                let runtime = self.runtime.lock().await;
                Ok(
                    json!({"cancel_requested":true,"effect_id":effect_id,"effect":runtime.effect(&effect_id)?,"worker_stopped":self.worker_stopped.load(Ordering::Acquire),"automatic_replay":false}),
                )
            }
            FabricAction::Reconcile { effect_id } => {
                validate_id(&effect_id)?;
                let (cancel, cancelled) = watch::channel(false);
                {
                    let mut active = self.active()?;
                    if active.is_some() {
                        return Err(ContextError::Conflict.into());
                    }
                    *active = Some(Active {
                        id: effect_id.clone(),
                        phase: "reconciling".into(),
                        cancel,
                    });
                }
                let runtime = self.runtime.clone();
                let active = self.active.clone();
                let scope = self.scope.clone();
                let observer = self.metadata.evidence.clone();
                let (sender, receiver) = oneshot::channel();
                std::thread::spawn(move || {
                    let _cancelled = cancelled;
                    let outcome = match tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                    {
                        Ok(executor) => executor.block_on(async {
                            match tokio::time::timeout(
                                std::time::Duration::from_secs(30),
                                reconcile_operation(runtime, scope, effect_id, observer),
                            )
                            .await
                            {
                                Ok(outcome) => outcome,
                                Err(_) => Err(ContextError::Unavailable(
                                    "trusted reconciliation deadline elapsed".into(),
                                )
                                .into()),
                            }
                        }),
                        Err(error) => Err(ContextError::Unavailable(error.to_string()).into()),
                    };
                    if let Ok(mut current) = active.lock() {
                        *current = None;
                    }
                    let _ = sender.send(outcome);
                });
                receiver
                    .await
                    .map_err(|_| ContextError::Unavailable("owned runtime reconciliation".into()))?
            }
        }
    }
    pub async fn shutdown(&self, owner: &Scope) -> Result<(), FabricError> {
        self.authorize(owner)?;
        if let Some(active) = self.active()?.as_ref() {
            let _ = active.cancel.send(true);
        }
        self.available.store(false, Ordering::Release);
        self.runtime.lock().await.shutdown().await?;
        self.worker_stopped.store(true, Ordering::Release);
        Ok(())
    }
}

async fn dispatch_operation(
    runtime: Arc<AsyncMutex<RuntimeService>>,
    active: Arc<Mutex<Option<Active>>>,
    available: Arc<AtomicBool>,
    worker_stopped: Arc<AtomicBool>,
    scope: Scope,
    id: String,
    bytes: Vec<u8>,
    timeout_ms: u64,
    mut cancelled: watch::Receiver<bool>,
) -> Result<Value, FabricError> {
    let mut runtime = runtime.lock().await;
    let result = tokio::select! {
     biased;
     _=cancelled.changed()=>{available.store(false,Ordering::Release);runtime.shutdown().await?;worker_stopped.store(true,Ordering::Release);return Err(FabricError::Cancelled{effect_id:id});},
     _=tokio::time::sleep(std::time::Duration::from_millis(timeout_ms))=>{available.store(false,Ordering::Release);runtime.shutdown().await?;worker_stopped.store(true,Ordering::Release);return Err(FabricError::Cancelled{effect_id:id});},
     result=async {
      let intent=runtime.submit(WorkerRequest::digest(id.clone(),bytes))?;
      if intent.state==EffectState::Terminal{runtime.flush_backend_events().await?;return runtime.result(&id)?.ok_or_else(||RuntimeError::Protocol("terminal effect has no successful result".into()));}
      if let Ok(mut current)=active.lock(){if let Some(current)=current.as_mut(){current.phase="prepared".into();}}
      if runtime.dispatch_next().await?.as_deref()!=Some(&id){return Err(RuntimeError::Protocol("request was not dispatched".into()))}
      if let Ok(mut current)=active.lock(){if let Some(current)=current.as_mut(){current.phase="dispatched".into();}}
      runtime.receive_result().await
     }=>result?,
    };
    let effect = runtime.effect(&id)?;
    Ok(json!({"version":1,"scope":scope,"result":result,"effect":effect}))
}

fn serialized_wire_bound(scope: &Scope, request: &WorkerRequest) -> Result<usize, FabricError> {
    let request_bytes = serde_json::to_vec(request)?.len();
    let scope_bytes = serde_json::to_vec(scope)?.len();
    let id_bytes = serde_json::to_vec(&request.id)?.len();
    // Two nested byte arrays each encode one byte in at most four JSON bytes.
    // The remaining envelope, session, sequence and signature metadata fit 4096 bytes.
    request_bytes
        .checked_mul(16)
        .and_then(|n| scope_bytes.checked_mul(4).and_then(|s| n.checked_add(s)))
        .and_then(|n| id_bytes.checked_mul(4).and_then(|s| n.checked_add(s)))
        .and_then(|n| n.checked_add(4096))
        .ok_or_else(|| ContextError::Capacity.into())
}

async fn reconcile_operation(
    runtime: Arc<AsyncMutex<RuntimeService>>,
    scope: Scope,
    effect_id: String,
    observer: Option<Arc<dyn EffectEvidenceObserver>>,
) -> Result<Value, FabricError> {
    let mut runtime = runtime.lock().await;
    let intent = runtime.effect(&effect_id)?.ok_or(ContextError::Stale)?;
    if intent.state == EffectState::Terminal {
        return Ok(
            json!({"effect":intent,"result":runtime.result(&effect_id)?,"reconciled":false,"already_terminal":true}),
        );
    }
    let observer = observer.ok_or_else(|| FabricError::EvidenceUnavailable {
        effect_id: effect_id.clone(),
    })?;
    let observation = observer.observe(&scope, &intent).await?.ok_or_else(|| {
        FabricError::EvidenceUnavailable {
            effect_id: effect_id.clone(),
        }
    })?;
    let receipt = runtime.reconcile(&effect_id, observation)?;
    runtime.flush_backend_events().await?;
    Ok(json!({"effect":receipt,"reconciled":true}))
}
