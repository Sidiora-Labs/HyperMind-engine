use crate::{
    actor::{ActorEngine, IncomingEvent},
    context_memory::MemoryError,
    development_admission,
};
use hm_context::{
    ContextError, Scope,
    development::{DevelopmentKind, DevelopmentPlan, EvidenceSnapshot},
    development_schedule::*,
    digest_bytes,
    maintenance::Usage,
    validate_id,
};
use hm_core::{ConversationId, LSN};
use hm_schema::{
    event::{self, Boundary, CURRENT_SCHEMA_VERSION},
    events::{EventEnvelope, EventPayload, ProviderFrame, Retention, Sensitivity},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

static SCHEDULER_MUTATIONS: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
const PROVIDER: &str = "hypermind/development-scheduler/v1";
#[derive(Clone, Debug)]
pub struct WorkerFailure {
    pub error: String,
    pub usage: Usage,
}
#[derive(Clone)]
pub struct DispatchContext {
    pub actor: ActorEngine,
    pub scope: Scope,
    pub lease: DispatchLease,
    pub maintenance_lease: hm_context::maintenance::JobLease,
    pub worker_id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ObservedSettlement {
    pub observation_id: String,
    pub job_id: String,
    pub attempt: u64,
    pub actual_tokens: u64,
}
pub trait DevelopmentWorker: Send + Sync {
    fn kind(&self) -> DevelopmentKind;
    fn provider_policy(&self) -> Option<&str> {
        None
    }
    fn run_observed(
        &self,
        evidence: EvidenceSnapshot,
        plan_id: String,
        _context: DispatchContext,
    ) -> Pin<Box<dyn Future<Output = Result<DevelopmentPlan, WorkerFailure>> + Send + '_>> {
        self.run(evidence, plan_id)
    }
    fn run(
        &self,
        evidence: EvidenceSnapshot,
        plan_id: String,
    ) -> Pin<Box<dyn Future<Output = Result<DevelopmentPlan, WorkerFailure>> + Send + '_>>;
}
#[derive(Default)]
pub struct WorkerRegistry {
    workers: BTreeMap<String, Arc<dyn DevelopmentWorker>>,
}
impl WorkerRegistry {
    pub fn describe(&self, id: &str) -> Option<(DevelopmentKind, Option<String>)> {
        self.workers
            .get(id)
            .map(|worker| (worker.kind(), worker.provider_policy().map(str::to_owned)))
    }
    pub fn ids(&self) -> Vec<String> {
        self.workers.keys().cloned().collect()
    }
    pub fn remove(&mut self, id: &str) -> bool {
        self.workers.remove(id).is_some()
    }
    pub fn register(
        &mut self,
        id: String,
        worker: Arc<dyn DevelopmentWorker>,
    ) -> Result<(), ContextError> {
        validate_id(&id)?;
        if self.workers.contains_key(&id) {
            return Err(ContextError::Conflict);
        }
        self.workers.insert(id, worker);
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum SchedulerAction {
    Configure { schedule: DevelopmentSchedule },
    Enqueue { schedule_id: String },
    Claim { job_id: String },
    Heartbeat { lease: DispatchLease },
    Cancel { job_id: String },
    SettleUnknown { charge_key: String, actual: u64 },
    Inspect,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchedulerRequest {
    pub version: u32,
    pub scope: Scope,
    pub request_id: String,
    pub action: SchedulerAction,
}
#[derive(Clone, Serialize, Deserialize)]
struct SavedReceipt {
    digest: String,
    value: serde_json::Value,
}
#[derive(Clone, Serialize, Deserialize)]
struct PendingPublication {
    lease: DispatchLease,
    plan_id: String,
    usage: Usage,
}
#[derive(Clone, Serialize, Deserialize)]
struct LedgerState {
    state: DevelopmentSchedules,
    pending: BTreeMap<String, PendingPublication>,
    receipts: BTreeMap<String, SavedReceipt>,
    #[serde(default)]
    observed_settlements: BTreeMap<String, ObservedSettlement>,
}
fn error(message: &str) -> MemoryError {
    ContextError::Invalid(message.into()).into()
}
fn now() -> Result<u64, MemoryError> {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| error("clock"))?
            .as_millis(),
    )
    .map_err(|_| ContextError::Capacity.into())
}
fn input_digest(evidence: &EvidenceSnapshot) -> Result<String, MemoryError> {
    Ok(digest_bytes(&serde_json::to_vec(&(
        &evidence.capability_digest,
        &evidence.sources,
        &evidence.records,
        evidence.policy_revision,
        &evidence.grant_digest,
    ))?))
}
async fn load(actor: &ActorEngine, scope: &Scope) -> Result<LedgerState, MemoryError> {
    let mut saved = LedgerState {
        state: DevelopmentSchedules::new(scope.clone(), 1_000_000, 2)?,
        pending: BTreeMap::new(),
        receipts: BTreeMap::new(),
        observed_settlements: BTreeMap::new(),
    };
    for frame in actor.frames_since(LSN::new(0), None, usize::MAX).await? {
        if frame.header.kind != hm_ledger::frame::EventKind::ProviderFrame {
            continue;
        }
        let event = event::verify_event(
            &frame.sealed_payload,
            event::EventKind::ProviderFrame,
            Boundary::Disk,
        )?;
        let EventPayload::ProviderFrame(provider) = event.envelope.payload else {
            continue;
        };
        if provider.provider != PROVIDER {
            continue;
        }
        let next: LedgerState = serde_json::from_slice(&provider.api_content)?;
        if next.state.scope != *scope {
            continue;
        }
        if next.state.version != 1 || next.state.sequence != saved.state.sequence + 1 {
            return Err(error("scheduler replay sequence"));
        }
        next.state.validate()?;
        saved = next;
    }
    Ok(saved)
}
async fn append(
    actor: &ActorEngine,
    tail: LSN,
    saved: &mut LedgerState,
) -> Result<(), MemoryError> {
    saved.state.sequence = saved
        .state
        .sequence
        .checked_add(1)
        .ok_or(ContextError::Capacity)?;
    actor
        .append_if_tail(
            tail,
            vec![IncomingEvent {
                kind: hm_ledger::frame::EventKind::ProviderFrame,
                conversation: ConversationId::derive(PROVIDER),
                payload: event::encode_event_envelope(&EventEnvelope {
                    schema_version: CURRENT_SCHEMA_VERSION,
                    payload: EventPayload::ProviderFrame(Box::new(ProviderFrame {
                        provider: PROVIDER.into(),
                        api_content: serde_json::to_vec(saved)?,
                    })),
                    connection_id: None,
                    client_seq: 0,
                    client_event_index: 0,
                    client_event_count: 1,
                    origin_actor: 0,
                    run_id: None,
                    model_provenance: None,
                    authority: hm_schema::events::Authority::RuntimeFact,
                    retention: Retention::Durable,
                    sensitivity: Sensitivity::Personal,
                    event_time_ns: 0,
                }),
            }],
        )
        .await?;
    Ok(())
}
pub async fn inspect(
    actor: &ActorEngine,
    scope: &Scope,
) -> Result<DevelopmentSchedules, MemoryError> {
    scope.validate()?;
    Ok(load(actor, scope).await?.state)
}
pub async fn execute(
    actor: &ActorEngine,
    scope: &Scope,
    owner: &Scope,
    request: SchedulerRequest,
) -> Result<serde_json::Value, MemoryError> {
    let _scheduler_guard = SCHEDULER_MUTATIONS.lock().await;
    if owner != scope || request.scope != *scope || request.version != 1 {
        return Err(ContextError::ScopeMismatch.into());
    }
    validate_id(&request.request_id)?;
    let request_digest = digest_bytes(&serde_json::to_vec(&request)?);
    if let Some(receipt) = load(actor, scope).await?.receipts.get(&request.request_id) {
        if receipt.digest != request_digest {
            return Err(ContextError::Conflict.into());
        }
        return Ok(receipt.value.clone());
    }
    // Evidence capture obtains its own mutation lock. Check the tail again before admission.
    let captured = if let SchedulerAction::Enqueue { schedule_id } = &request.action {
        let state = inspect(actor, scope).await?;
        let schedule = state
            .schedules
            .get(schedule_id)
            .ok_or_else(|| error("schedule unavailable"))?;
        Some(
            development_admission::snapshot(
                actor,
                scope,
                &schedule.principal,
                &schedule.worker_id,
                schedule.snapshot.clone(),
            )
            .await?,
        )
    } else {
        None
    };
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let tail = actor.stats().await?.applied.last_lsn;
    let mut saved = load(actor, scope).await?;
    let digest = digest_bytes(&serde_json::to_vec(&request)?);
    if let Some(receipt) = saved.receipts.get(&request.request_id) {
        if receipt.digest != digest {
            return Err(ContextError::Conflict.into());
        }
        return Ok(receipt.value.clone());
    }
    let time = now()?;
    reconcile(actor, scope, &mut saved).await?;
    saved.state.expire(time)?;
    let value = match request.action {
        SchedulerAction::Inspect => serde_json::to_value(&saved.state)?,
        SchedulerAction::Configure { schedule } => {
            saved.state.configure(schedule, time)?;
            serde_json::json!({"configured":true})
        }
        SchedulerAction::Enqueue { schedule_id } => {
            let evidence = captured.ok_or_else(|| error("missing snapshot"))?;
            let schedule = &saved.state.schedules[&schedule_id];
            if evidence.worker_id != schedule.worker_id
                || evidence.capability_id != schedule.snapshot.capability_id
                || evidence.ledger_tail != tail.get()
            {
                return Err(ContextError::Stale.into());
            }
            let id = saved
                .state
                .enqueue(&schedule_id, input_digest(&evidence)?, time)?;
            saved.state.mark_due(&schedule_id, time);
            serde_json::json!({"job_id":id})
        }
        SchedulerAction::Claim { job_id } => {
            serde_json::to_value(saved.state.claim(&job_id, time)?)?
        }
        SchedulerAction::Heartbeat { lease } => {
            serde_json::to_value(saved.state.renew(&lease, time)?)?
        }
        SchedulerAction::Cancel { job_id } => {
            saved.state.cancel(&job_id)?;
            serde_json::json!({"cancelled":true})
        }
        SchedulerAction::SettleUnknown { charge_key, actual } => {
            saved.state.settle_unknown(&charge_key, actual)?;
            serde_json::json!({"settled":true})
        }
    };
    saved.receipts.insert(
        request.request_id,
        SavedReceipt {
            digest,
            value: value.clone(),
        },
    );
    append(actor, tail, &mut saved).await?;
    Ok(value)
}
async fn terminal_locked(
    actor: &ActorEngine,
    scope: &Scope,
    lease: &DispatchLease,
    result: Result<(Usage, String), WorkerFailure>,
) -> Result<DevelopmentSchedules, MemoryError> {
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let tail = actor.stats().await?.applied.last_lsn;
    let mut saved = load(actor, scope).await?;
    let time = now()?;
    saved
        .pending
        .retain(|_, publication| publication.lease.job_id != lease.job_id);
    let observation =
        crate::development_usage::for_attempt(actor, scope, &lease.job_id, lease.attempt).await?;
    let result = if let Some(observation) = &observation {
        validate_binding_state(&saved, &observation.binding, false)?;
        let usage = crate::development_usage::actual_tokens(observation)
            .map_or(Usage::Unknown, Usage::Known);
        match result {
            Ok((_, receipt)) => Ok((usage, receipt)),
            Err(mut failure) => {
                failure.usage = usage;
                Err(failure)
            }
        }
    } else {
        result
    };
    match result {
        Ok((usage, receipt)) => saved.state.finish(lease, usage, receipt, time)?,
        Err(failure) => saved
            .state
            .fail(lease, failure.usage, failure.error, time)?,
    };
    if let Some(observation) = observation {
        if let Some(actual_tokens) = crate::development_usage::actual_tokens(&observation) {
            saved.observed_settlements.insert(
                DevelopmentSchedules::charge_key(lease),
                ObservedSettlement {
                    observation_id: observation.id,
                    job_id: lease.job_id.clone(),
                    attempt: lease.attempt,
                    actual_tokens,
                },
            );
        }
    }
    append(actor, tail, &mut saved).await?;
    Ok(saved.state)
}
pub async fn dispatch(
    actor: &ActorEngine,
    scope: &Scope,
    owner: &Scope,
    request_id: &str,
    job_id: &str,
    registry: &WorkerRegistry,
) -> Result<DevelopmentSchedules, MemoryError> {
    if scope != owner {
        return Err(ContextError::ScopeMismatch.into());
    }
    let state = inspect(actor, scope).await?;
    let job = state.jobs.get(job_id).ok_or(ContextError::Stale)?;
    let schedule = state.schedules[&job.schedule_id].clone();
    let worker = registry
        .workers
        .get(&schedule.worker_id)
        .ok_or_else(|| error("worker not registered"))?;
    if schedule
        .provider_policy
        .as_deref()
        .is_some_and(|policy| worker.provider_policy() != Some(policy))
    {
        return Err(error("worker provider policy unavailable"));
    }
    if worker.kind() != schedule.kind {
        return Err(ContextError::ScopeMismatch.into());
    }
    let value = execute(
        actor,
        scope,
        owner,
        SchedulerRequest {
            version: 1,
            scope: scope.clone(),
            request_id: request_id.into(),
            action: SchedulerAction::Claim {
                job_id: job_id.into(),
            },
        },
    )
    .await?;
    let lease: DispatchLease = serde_json::from_value(value)?;
    inspect(actor, scope)
        .await?
        .validate_lease(&lease, now()?)?;
    let evidence = match development_admission::snapshot(
        actor,
        scope,
        &schedule.principal,
        &schedule.worker_id,
        schedule.snapshot.clone(),
    )
    .await
    {
        Ok(evidence) => evidence,
        Err(e) => {
            return terminal(
                actor,
                scope,
                &lease,
                Err(WorkerFailure {
                    error: e.to_string(),
                    usage: Usage::Known(0),
                }),
            )
            .await;
        }
    };
    if input_digest(&evidence)? != job.input_digest
        || evidence.budget.reserved_tokens != schedule.reservation
    {
        return terminal(
            actor,
            scope,
            &lease,
            Err(WorkerFailure {
                error: "dispatch evidence changed".into(),
                usage: Usage::Known(0),
            }),
        )
        .await;
    }
    let plan_id = format!("development-{}-{}", lease.job_id, lease.attempt);
    let result = tokio::time::timeout(
        std::time::Duration::from_millis(schedule.timeout_ms),
        worker.run_observed(
            evidence.clone(),
            plan_id.clone(),
            DispatchContext {
                actor: actor.clone(),
                scope: scope.clone(),
                lease: lease.clone(),
                maintenance_lease: {
                    let active = inspect(actor, scope).await?;
                    let job = &active.jobs[&lease.job_id];
                    match &active.accounting.jobs[&job.accounting_id].status {
                        hm_context::maintenance::JobStatus::Running(lease) => lease.clone(),
                        _ => return Err(ContextError::Stale.into()),
                    }
                },
                worker_id: schedule.worker_id.clone(),
            },
        ),
    )
    .await;
    let plan = match result {
        Ok(Ok(plan)) => plan,
        Ok(Err(failure)) => return terminal(actor, scope, &lease, Err(failure)).await,
        Err(_) => return expire_dispatch(actor, scope, &lease).await,
    };
    if plan.id != plan_id
        || plan.kind != schedule.kind
        || plan.evidence != evidence
        || matches!(plan.usage,Usage::Known(actual)if actual>schedule.reservation)
    {
        return terminal(
            actor,
            scope,
            &lease,
            Err(WorkerFailure {
                error: "invalid worker plan".into(),
                usage: Usage::Unknown,
            }),
        )
        .await;
    }
    let _scheduler_guard = SCHEDULER_MUTATIONS.lock().await;
    // Cancellation and publication share this service lock; actor admission also fences its tail.
    let state = inspect(actor, scope).await?;
    state.validate_lease(&lease, now()?)?;
    let usage = plan.usage;
    {
        let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
        let tail = actor.stats().await?.applied.last_lsn;
        let mut saved = load(actor, scope).await?;
        saved.state.validate_lease(&lease, now()?)?;
        saved.pending.insert(
            plan_id.clone(),
            PendingPublication {
                lease: lease.clone(),
                plan_id: plan_id.clone(),
                usage,
            },
        );
        append(actor, tail, &mut saved).await?;
    }
    match development_admission::admit(actor, scope, &schedule.principal, &schedule.worker_id, plan)
        .await
    {
        Ok(receipt) => terminal_locked(actor, scope, &lease, Ok((usage, receipt.plan_id))).await,
        Err(e) => {
            terminal_locked(
                actor,
                scope,
                &lease,
                Err(WorkerFailure {
                    error: e.to_string(),
                    usage,
                }),
            )
            .await
        }
    }
}
async fn expire_dispatch(
    actor: &ActorEngine,
    scope: &Scope,
    lease: &DispatchLease,
) -> Result<DevelopmentSchedules, MemoryError> {
    let _scheduler_guard = SCHEDULER_MUTATIONS.lock().await;
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let tail = actor.stats().await?.applied.last_lsn;
    let mut saved = load(actor, scope).await?;
    saved.state.expire(lease.expires_ms.max(now()?))?;
    append(actor, tail, &mut saved).await?;
    Ok(saved.state)
}

async fn terminal(
    actor: &ActorEngine,
    scope: &Scope,
    lease: &DispatchLease,
    result: Result<(Usage, String), WorkerFailure>,
) -> Result<DevelopmentSchedules, MemoryError> {
    let _guard = SCHEDULER_MUTATIONS.lock().await;
    terminal_locked(actor, scope, lease, result).await
}
async fn reconcile(
    actor: &ActorEngine,
    scope: &Scope,
    saved: &mut LedgerState,
) -> Result<(), MemoryError> {
    let memory = crate::context_memory::rebuild(actor, scope).await?;
    let pending: Vec<_> = saved.pending.values().cloned().collect();
    for publication in pending {
        if memory
            .development_receipts
            .contains_key(&publication.plan_id)
        {
            if saved
                .state
                .jobs
                .get(&publication.lease.job_id)
                .is_some_and(|job| matches!(job.status, DispatchStatus::Running { .. }))
            {
                saved.state.finish(
                    &publication.lease,
                    publication.usage,
                    publication.plan_id.clone(),
                    publication.lease.expires_ms.saturating_sub(1),
                )?;
            }
            saved.pending.remove(&publication.plan_id);
        }
    }
    Ok(())
}
pub async fn tick(
    actor: &ActorEngine,
    scope: &Scope,
    owner: &Scope,
    request_prefix: &str,
) -> Result<Vec<String>, MemoryError> {
    validate_id(request_prefix)?;
    let state = inspect(actor, scope).await?;
    let mut jobs = Vec::new();
    for schedule_id in state.due(now()?) {
        let value = execute(
            actor,
            scope,
            owner,
            SchedulerRequest {
                version: 1,
                scope: scope.clone(),
                request_id: format!("{request_prefix}-{schedule_id}"),
                action: SchedulerAction::Enqueue { schedule_id },
            },
        )
        .await?;
        jobs.push(
            value["job_id"]
                .as_str()
                .ok_or_else(|| error("missing job receipt"))?
                .into(),
        );
    }
    Ok(jobs)
}

fn validate_binding_state(
    saved: &LedgerState,
    binding: &crate::development_usage::OriginalCallBinding,
    require_running: bool,
) -> Result<(), MemoryError> {
    if saved.state.scope != binding.scope {
        return Err(ContextError::ScopeMismatch.into());
    }
    let job = saved
        .state
        .jobs
        .get(&binding.job_id)
        .ok_or(ContextError::Stale)?;
    let schedule = &saved.state.schedules[&job.schedule_id];
    let accounting = saved
        .state
        .accounting
        .jobs
        .get(&job.accounting_id)
        .ok_or(ContextError::Stale)?;
    if job.accounting_id != binding.maintenance_lease.job_id
        || binding.attempt != binding.maintenance_lease.attempt
        || job.attempt < binding.attempt
        || accounting.fence != binding.maintenance_lease.fence
        || schedule.worker_id != binding.worker_id
        || binding.plan_id != format!("development-{}-{}", binding.job_id, binding.attempt)
    {
        return Err(ContextError::ScopeMismatch.into());
    }
    if require_running {
        let hm_context::maintenance::JobStatus::Running(lease) = &accounting.status else {
            return Err(ContextError::Stale.into());
        };
        if lease != &binding.maintenance_lease
            || job.attempt != binding.attempt
            || !matches!(job.status, DispatchStatus::Running { .. })
            || lease.expires_ms <= now()?
        {
            return Err(ContextError::Stale.into());
        }
    }
    Ok(())
}
pub(crate) async fn validate_original_binding(
    actor: &ActorEngine,
    binding: &crate::development_usage::OriginalCallBinding,
    require_running: bool,
) -> Result<(), MemoryError> {
    let saved = load(actor, &binding.scope).await?;
    validate_binding_state(&saved, binding, require_running)
}
pub(crate) async fn reconcile_original_observation(
    actor: &ActorEngine,
    scope: &Scope,
    observation: &crate::development_usage::OriginalUsageObservation,
) -> Result<(), MemoryError> {
    let _scheduler_guard = SCHEDULER_MUTATIONS.lock().await;
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let tail = actor.stats().await?.applied.last_lsn;
    let mut saved = load(actor, scope).await?;
    validate_binding_state(&saved, &observation.binding, false)?;
    let actual_tokens = crate::development_usage::actual_tokens(observation)
        .ok_or_else(|| error("original counters remain unknown"))?;
    let key = format!(
        "{}:{}",
        observation.binding.job_id, observation.binding.attempt
    );
    if let Some(receipt) = saved.observed_settlements.get(&key) {
        if receipt.observation_id != observation.id || receipt.actual_tokens != actual_tokens {
            return Err(ContextError::Conflict.into());
        }
        return Ok(());
    }
    saved.state.settle_unknown(&key, actual_tokens)?;
    saved.observed_settlements.insert(
        key,
        ObservedSettlement {
            observation_id: observation.id.clone(),
            job_id: observation.binding.job_id.clone(),
            attempt: observation.binding.attempt,
            actual_tokens,
        },
    );
    append(actor, tail, &mut saved).await
}
pub async fn observed_settlements(
    actor: &ActorEngine,
    scope: &Scope,
    owner: &Scope,
) -> Result<Vec<ObservedSettlement>, MemoryError> {
    if scope != owner {
        return Err(ContextError::ScopeMismatch.into());
    }
    Ok(load(actor, scope)
        .await?
        .observed_settlements
        .into_values()
        .collect())
}
