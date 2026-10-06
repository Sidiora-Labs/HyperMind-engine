use crate::{
    actor::{ActorEngine, IncomingEvent},
    context_memory::MemoryError,
};
use hm_context::{
    ContextError, Cursor, Scope, digest_bytes,
    maintenance::{
        JobKind, JobLease, JobStatus, MaintenanceScheduler, SchedulerConfig, SchedulerSnapshot,
        Usage,
    },
    validate_id,
};
use hm_core::{ConversationId, LSN};
use hm_llm::{
    catalog::CatalogModel,
    provider_usage::{
        CatalogCharge, InputSemantics, ObservationFormat, ObservationMetadata,
        ProviderUsageSnapshot, account_catalog_usage,
    },
};
use hm_schema::{
    event::{self, Boundary, CURRENT_SCHEMA_VERSION},
    events::{EventEnvelope, EventPayload, ProviderFrame, Retention, Sensitivity},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
const PROVIDER: &str = "hypermind/native-usage/v1";
static WRITER: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
fn invalid(message: &str) -> MemoryError {
    ContextError::Invalid(message.into()).into()
}
fn now_ns() -> Result<i64, MemoryError> {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| invalid("clock"))?
            .as_nanos(),
    )
    .map_err(|_| ContextError::Capacity.into())
}
fn now_ms() -> Result<u64, MemoryError> {
    Ok(u64::try_from(now_ns()?).map_err(|_| ContextError::Capacity)? / 1_000_000)
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsageAttribution {
    pub job_id: String,
    pub worker_id: String,
    pub session_id: String,
    pub turn_id: String,
    pub provider_id: String,
    pub model_id: String,
    pub source_ids: BTreeSet<String>,
}
impl UsageAttribution {
    fn validate(&self) -> Result<(), ContextError> {
        for id in [
            &self.job_id,
            &self.worker_id,
            &self.session_id,
            &self.turn_id,
            &self.provider_id,
            &self.model_id,
        ] {
            validate_id(id)?;
        }
        if self.source_ids.len() > 256 {
            return Err(ContextError::Capacity);
        }
        for id in &self.source_ids {
            validate_id(id)?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsageLimits {
    pub total_tokens: u64,
    pub hourly_tokens: u64,
    pub daily_tokens: u64,
    pub job_tokens: u64,
    pub concurrency: usize,
    pub lease_ms: u64,
}
impl Default for UsageLimits {
    fn default() -> Self {
        Self {
            total_tokens: 1_000_000,
            hourly_tokens: 100_000,
            daily_tokens: 1_000_000,
            job_tokens: 32_768,
            concurrency: 2,
            lease_ms: 60_000,
        }
    }
}
impl UsageLimits {
    fn validate(&self) -> Result<(), ContextError> {
        if self.total_tokens == 0
            || self.hourly_tokens == 0
            || self.daily_tokens == 0
            || self.job_tokens == 0
            || self.concurrency == 0
            || self.concurrency > 16
            || self.lease_ms == 0
            || self.lease_ms > 300_000
        {
            return Err(ContextError::Invalid("usage limits".into()));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReservationRequest {
    pub request_id: String,
    pub attribution: UsageAttribution,
    pub kind: JobKind,
    pub reserved_tokens: u64,
    pub input_digest: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UsageReservation {
    #[serde(default)]
    pub runner_dispatch: Option<RunnerDispatchBinding>,
    pub id: String,
    pub request_digest: String,
    pub input_digest: String,
    pub attribution: UsageAttribution,
    pub lease: JobLease,
    pub reserved_tokens: u64,
    pub reserved_at_ms: u64,
    pub dispatched: bool,
    pub observation_id: Option<String>,
    pub settled_tokens: Option<u64>,
    pub failed_response_bytes: Option<Vec<u8>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UsageObservation {
    pub id: String,
    pub reservation_id: String,
    pub attribution: UsageAttribution,
    pub snapshot: ProviderUsageSnapshot,
    pub catalog_estimate: Option<CatalogCharge>,
    pub accepted_response: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UsageState {
    pub version: u32,
    pub scope: Scope,
    pub sequence: u64,
    pub limits: UsageLimits,
    pub accounting: SchedulerSnapshot,
    pub reservations: BTreeMap<String, UsageReservation>,
    pub observations: BTreeMap<String, UsageObservation>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct UsageRollup {
    pub known_tokens: u64,
    pub held_tokens: u64,
    pub unknown_reservations: u64,
    pub active_reservations: u64,
    pub observed_calls: u64,
    pub attributions: Vec<UsageAttribution>,
    pub observations: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RollupQuery {
    pub session_id: Option<String>,
    pub turn_id: Option<String>,
    pub job_id: Option<String>,
    pub provider_id: Option<String>,
    pub start_ms: Option<u64>,
    pub end_ms: Option<u64>,
}
fn new_state(scope: &Scope) -> Result<UsageState, MemoryError> {
    let limits = UsageLimits::default();
    Ok(UsageState {
        version: 1,
        scope: scope.clone(),
        sequence: 0,
        accounting: MaintenanceScheduler::new(
            scope.clone(),
            SchedulerConfig {
                max_concurrency: limits.concurrency,
                budget: limits.total_tokens,
                lease_ms: limits.lease_ms,
                backoff_ms: 0,
                max_attempts: 1,
            },
        )?
        .snapshot(),
        limits,
        reservations: BTreeMap::new(),
        observations: BTreeMap::new(),
    })
}
async fn load(actor: &ActorEngine, scope: &Scope) -> Result<UsageState, MemoryError> {
    scope.validate()?;
    let mut state = new_state(scope)?;
    for frame in actor.frames_since(LSN::new(0), None, usize::MAX).await? {
        if frame.header.kind != hm_ledger::frame::EventKind::ProviderFrame {
            continue;
        }
        let verified = event::verify_event(
            &frame.sealed_payload,
            event::EventKind::ProviderFrame,
            Boundary::Disk,
        )?;
        let EventPayload::ProviderFrame(provider) = verified.envelope.payload else {
            continue;
        };
        if provider.provider != PROVIDER {
            continue;
        }
        let next: UsageState = serde_json::from_slice(&provider.api_content)?;
        if next.scope != *scope {
            continue;
        }
        if next.version != 1 || next.sequence != state.sequence + 1 {
            return Err(invalid("usage replay sequence"));
        }
        next.limits.validate()?;
        MaintenanceScheduler::from_snapshot(next.accounting.clone())?;
        for (id, reservation) in &next.reservations {
            reservation.attribution.validate()?;
            if id != &reservation.id
                || !next.accounting.jobs.contains_key(&reservation.lease.job_id)
            {
                return Err(invalid("usage reservation replay"));
            }
        }
        for (id, observation) in &next.observations {
            if id != &observation.id || !next.reservations.contains_key(&observation.reservation_id)
            {
                return Err(invalid("usage observation replay"));
            }
            let verified = ProviderUsageSnapshot::parse(
                &observation.snapshot.original_bytes,
                observation.snapshot.observation.clone(),
            )
            .map_err(|e| invalid(&e.to_string()))?;
            if verified != observation.snapshot {
                return Err(invalid("usage evidence replay"));
            }
        }
        state = next;
    }
    Ok(state)
}
async fn append(actor: &ActorEngine, tail: LSN, state: &mut UsageState) -> Result<(), MemoryError> {
    state.sequence = state
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
                        api_content: serde_json::to_vec(state)?,
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
fn authorize(scope: &Scope, owner: &Scope) -> Result<(), MemoryError> {
    if scope != owner {
        return Err(ContextError::ScopeMismatch.into());
    }
    scope.validate()?;
    Ok(())
}
pub async fn inspect(
    actor: &ActorEngine,
    scope: &Scope,
    owner: &Scope,
) -> Result<UsageState, MemoryError> {
    authorize(scope, owner)?;
    load(actor, scope).await
}
pub async fn configure(
    actor: &ActorEngine,
    scope: &Scope,
    owner: &Scope,
    limits: UsageLimits,
) -> Result<UsageState, MemoryError> {
    authorize(scope, owner)?;
    limits.validate()?;
    let _writer = WRITER.lock().await;
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let tail = actor.stats().await?.applied.last_lsn;
    let mut state = load(actor, scope).await?;
    if !state.reservations.is_empty() {
        return Err(ContextError::Conflict.into());
    }
    state.accounting.config = SchedulerConfig {
        max_concurrency: limits.concurrency,
        budget: limits.total_tokens,
        lease_ms: limits.lease_ms,
        backoff_ms: 0,
        max_attempts: 1,
    };
    state.limits = limits;
    append(actor, tail, &mut state).await?;
    Ok(state)
}
fn committed(reservation: &UsageReservation, state: &UsageState) -> u64 {
    if let Some(actual) = reservation.settled_tokens {
        actual
    } else if state.accounting.jobs[&reservation.lease.job_id].status
        == JobStatus::Running(reservation.lease.clone())
        || state.accounting.unknown_usage.contains_key(&format!(
            "{}:{}",
            reservation.lease.job_id, reservation.lease.attempt
        ))
    {
        reservation.reserved_tokens
    } else {
        0
    }
}
fn window_committed(
    state: &UsageState,
    now: u64,
    span: u64,
    job: Option<&str>,
) -> Result<u64, MemoryError> {
    state
        .reservations
        .values()
        .filter(|r| {
            job.is_none_or(|job| r.attribution.job_id == job)
                && (r.reserved_at_ms / span == now / span
                    || r.settled_tokens.is_none() && committed(r, state) > 0)
        })
        .try_fold(0u64, |sum, r| {
            sum.checked_add(committed(r, state))
                .ok_or(ContextError::Capacity.into())
        })
}
pub async fn reserve(
    actor: &ActorEngine,
    scope: &Scope,
    owner: &Scope,
    request: ReservationRequest,
) -> Result<UsageReservation, MemoryError> {
    authorize(scope, owner)?;
    validate_id(&request.request_id)?;
    request.attribution.validate()?;
    if request.input_digest.len() != 64
        || !request.input_digest.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(invalid("input digest"));
    }
    let _writer = WRITER.lock().await;
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let tail = actor.stats().await?.applied.last_lsn;
    let mut state = load(actor, scope).await?;
    let request_digest = digest_bytes(&serde_json::to_vec(&request)?);
    let id = digest_bytes(&serde_json::to_vec(&(scope, &request.request_id))?);
    if let Some(saved) = state.reservations.get(&id) {
        if saved.request_digest != request_digest {
            return Err(ContextError::Conflict.into());
        }
        return Ok(saved.clone());
    }
    let now = now_ms()?;
    let mut accounting = MaintenanceScheduler::from_snapshot(state.accounting.clone())?;
    accounting.expire(now)?;
    state.accounting = accounting.snapshot();
    if request.reserved_tokens == 0
        || window_committed(&state, now, 3_600_000, None)?
            .checked_add(request.reserved_tokens)
            .ok_or(ContextError::Capacity)?
            > state.limits.hourly_tokens
        || window_committed(&state, now, 86_400_000, None)?
            .checked_add(request.reserved_tokens)
            .ok_or(ContextError::Capacity)?
            > state.limits.daily_tokens
        || window_committed(&state, now, u64::MAX, Some(&request.attribution.job_id))?
            .checked_add(request.reserved_tokens)
            .ok_or(ContextError::Capacity)?
            > state.limits.job_tokens
    {
        return Err(ContextError::Capacity.into());
    }
    let job_id = accounting.enqueue_evidence(
        request.kind,
        request_digest.clone(),
        Cursor {
            epoch: 1,
            sequence: state.reservations.len() as u64 + 1,
        },
        1,
        request.reserved_tokens,
    )?;
    let lease = accounting
        .claim_job(&job_id, now, state.limits.lease_ms)?
        .ok_or(ContextError::Capacity)?;
    let reservation = UsageReservation {
        runner_dispatch: None,
        id: id.clone(),
        request_digest,
        input_digest: request.input_digest,
        attribution: request.attribution,
        lease,
        reserved_tokens: request.reserved_tokens,
        reserved_at_ms: now,
        dispatched: false,
        observation_id: None,
        settled_tokens: None,
        failed_response_bytes: None,
    };
    state.accounting = accounting.snapshot();
    state.reservations.insert(id, reservation.clone());
    append(actor, tail, &mut state).await?;
    Ok(reservation)
}
pub async fn mark_unknown(
    actor: &ActorEngine,
    scope: &Scope,
    owner: &Scope,
    id: &str,
) -> Result<UsageReservation, MemoryError> {
    authorize(scope, owner)?;
    let _writer = WRITER.lock().await;
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let tail = actor.stats().await?.applied.last_lsn;
    let mut state = load(actor, scope).await?;
    let reservation = state
        .reservations
        .get(id)
        .ok_or(ContextError::Stale)?
        .clone();
    if reservation.settled_tokens.is_some() {
        return Err(ContextError::Stale.into());
    }
    let mut accounting = MaintenanceScheduler::from_snapshot(state.accounting.clone())?;
    if matches!(
        accounting.snapshot().jobs[&reservation.lease.job_id].status,
        JobStatus::Running(_)
    ) {
        accounting.cancel(&reservation.lease.job_id, now_ms()?)?;
        state.accounting = accounting.snapshot();
        append(actor, tail, &mut state).await?;
    }
    Ok(reservation)
}
fn total(snapshot: &ProviderUsageSnapshot) -> Option<u64> {
    let input = snapshot.tokens.input?;
    let output = snapshot.tokens.output?;
    match snapshot.tokens.input_semantics {
        InputSemantics::IncludesCache => input.checked_add(output),
        InputSemantics::ExcludesCache => input
            .checked_add(snapshot.tokens.cache_read?)
            .and_then(|n| n.checked_add(snapshot.tokens.cache_write?))
            .and_then(|n| n.checked_add(output)),
        InputSemantics::Unknown if snapshot.observation.format == ObservationFormat::Ollama => {
            input.checked_add(output)
        }
        _ => None,
    }
}
fn settle_state(
    state: &mut UsageState,
    id: &str,
    observation_id: &str,
    now: u64,
) -> Result<(), MemoryError> {
    let observation = state
        .observations
        .get(observation_id)
        .ok_or_else(|| invalid("trusted observation unavailable"))?;
    let reservation = state
        .reservations
        .get(id)
        .ok_or(ContextError::Stale)?
        .clone();
    if observation.reservation_id != id || observation.attribution != reservation.attribution {
        return Err(ContextError::ScopeMismatch.into());
    }
    if let Some(existing) = &reservation.observation_id {
        if existing != observation_id {
            return Err(ContextError::Conflict.into());
        }
    }
    if reservation.settled_tokens.is_some() {
        return Ok(());
    }
    let usage = if observation.accepted_response && observation.snapshot.error.is_none() {
        total(&observation.snapshot).map_or(Usage::Unknown, Usage::Known)
    } else {
        Usage::Unknown
    };
    let mut accounting = MaintenanceScheduler::from_snapshot(state.accounting.clone())?;
    accounting.expire(now)?;
    let snapshot = accounting.snapshot();
    let job = &snapshot.jobs[&reservation.lease.job_id];
    match usage {
        Usage::Known(actual) => {
            if actual > reservation.reserved_tokens {
                return Err(ContextError::Capacity.into());
            }
            if matches!(job.status, JobStatus::Running(_)) {
                accounting.complete(
                    &reservation.lease,
                    &reservation.lease.fence,
                    usage,
                    &observation.snapshot.fingerprint,
                    now,
                )?;
            } else {
                accounting.settle_unknown(
                    &reservation.lease.job_id,
                    reservation.lease.attempt,
                    actual,
                )?;
            }
            state.reservations.get_mut(id).unwrap().settled_tokens = Some(actual);
        }
        Usage::Unknown => {
            if matches!(job.status, JobStatus::Running(_)) {
                accounting.cancel(&reservation.lease.job_id, now)?;
            }
        }
    }
    state.reservations.get_mut(id).unwrap().observation_id = Some(observation_id.into());
    state.accounting = accounting.snapshot();
    Ok(())
}
pub async fn reconcile_unknown(
    actor: &ActorEngine,
    scope: &Scope,
    owner: &Scope,
    reservation_id: &str,
    observation_id: &str,
) -> Result<UsageReservation, MemoryError> {
    authorize(scope, owner)?;
    let _writer = WRITER.lock().await;
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let tail = actor.stats().await?.applied.last_lsn;
    let mut state = load(actor, scope).await?;
    settle_state(&mut state, reservation_id, observation_id, now_ms()?)?;
    let reservation = state.reservations[reservation_id].clone();
    append(actor, tail, &mut state).await?;
    Ok(reservation)
}
pub async fn rollup(
    actor: &ActorEngine,
    scope: &Scope,
    owner: &Scope,
    query: RollupQuery,
) -> Result<UsageRollup, MemoryError> {
    let state = inspect(actor, scope, owner).await?;
    let mut rollup = UsageRollup::default();
    for reservation in state.reservations.values().filter(|r| {
        query
            .session_id
            .as_ref()
            .is_none_or(|v| &r.attribution.session_id == v)
            && query
                .turn_id
                .as_ref()
                .is_none_or(|v| &r.attribution.turn_id == v)
            && query
                .job_id
                .as_ref()
                .is_none_or(|v| &r.attribution.job_id == v)
            && query
                .provider_id
                .as_ref()
                .is_none_or(|v| &r.attribution.provider_id == v)
            && query.start_ms.is_none_or(|v| r.reserved_at_ms >= v)
            && query.end_ms.is_none_or(|v| r.reserved_at_ms < v)
    }) {
        if let Some(actual) = reservation.settled_tokens {
            rollup.known_tokens = rollup
                .known_tokens
                .checked_add(actual)
                .ok_or(ContextError::Capacity)?;
        } else {
            rollup.held_tokens = rollup
                .held_tokens
                .checked_add(committed(reservation, &state))
                .ok_or(ContextError::Capacity)?;
            if state.accounting.unknown_usage.contains_key(&format!(
                "{}:{}",
                reservation.lease.job_id, reservation.lease.attempt
            )) {
                rollup.unknown_reservations += 1;
            } else if matches!(
                state.accounting.jobs[&reservation.lease.job_id].status,
                JobStatus::Running(_)
            ) {
                rollup.active_reservations += 1;
            }
        }
        if let Some(id) = &reservation.observation_id {
            rollup.observed_calls += 1;
            rollup.observations.push(id.clone());
        }
        rollup.attributions.push(reservation.attribution.clone());
    }
    let scheduler = crate::development_scheduler::inspect(actor, scope).await?;
    let original_attempts =
        crate::development_usage::dispatched_attempts(actor, scope, owner).await?;
    let original_observations = crate::development_usage::inspect(actor, scope, owner).await?;
    let settlements =
        crate::development_scheduler::observed_settlements(actor, scope, owner).await?;
    for binding in original_attempts {
        let Some(job) = scheduler.jobs.get(&binding.job_id) else {
            continue;
        };
        let started_ms = binding
            .maintenance_lease
            .expires_ms
            .saturating_sub(scheduler.schedules[&job.schedule_id].timeout_ms);
        if query
            .session_id
            .as_ref()
            .is_some_and(|v| v != &binding.session_id)
            || query
                .turn_id
                .as_ref()
                .is_some_and(|v| v != &binding.plan_id)
            || query.job_id.as_ref().is_some_and(|v| v != &binding.job_id)
            || query
                .provider_id
                .as_ref()
                .is_some_and(|v| v != &binding.provider_id)
            || query.start_ms.is_some_and(|v| started_ms < v)
            || query.end_ms.is_some_and(|v| started_ms >= v)
        {
            continue;
        }
        let observation = original_observations
            .iter()
            .find(|observation| observation.binding == binding);
        if let Some(settlement) = settlements.iter().find(|settlement| {
            settlement.job_id == binding.job_id && settlement.attempt == binding.attempt
        }) {
            rollup.known_tokens = rollup
                .known_tokens
                .checked_add(settlement.actual_tokens)
                .ok_or(ContextError::Capacity)?;
        } else {
            let key = format!("{}:{}", binding.maintenance_lease.job_id, binding.attempt);
            if let Some(held) = scheduler.accounting.unknown_usage.get(&key) {
                rollup.held_tokens = rollup
                    .held_tokens
                    .checked_add(*held)
                    .ok_or(ContextError::Capacity)?;
                rollup.unknown_reservations += 1;
            } else if scheduler.accounting.jobs[&binding.maintenance_lease.job_id].status
                == JobStatus::Running(binding.maintenance_lease.clone())
            {
                rollup.held_tokens = rollup
                    .held_tokens
                    .checked_add(
                        scheduler.accounting.jobs[&binding.maintenance_lease.job_id]
                            .request
                            .reservation,
                    )
                    .ok_or(ContextError::Capacity)?;
                rollup.active_reservations += 1;
            }
        }
        if let Some(observation) = observation {
            rollup.observed_calls += 1;
            rollup.observations.push(observation.id.clone());
        }
        rollup.attributions.push(UsageAttribution {
            job_id: binding.job_id,
            worker_id: binding.worker_id,
            session_id: binding.session_id,
            turn_id: binding.plan_id,
            provider_id: binding.provider_id,
            model_id: binding.model_id,
            source_ids: binding.source_digests.keys().cloned().collect(),
        });
    }
    Ok(rollup)
}

pub struct ProviderObserver {
    endpoint: String,
    provider_id: String,
    model_id: String,
    client: reqwest::Client,
}
impl ProviderObserver {
    pub fn ollama(
        endpoint: String,
        provider_id: String,
        model_id: String,
    ) -> Result<Self, MemoryError> {
        validate_id(&provider_id)?;
        validate_id(&model_id)?;
        let url = reqwest::Url::parse(&endpoint).map_err(|_| invalid("provider endpoint"))?;
        if !matches!(url.scheme(), "https" | "http")
            || url.username() != ""
            || url.password().is_some()
        {
            return Err(invalid("provider endpoint"));
        }
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| invalid("provider transport"))?;
        Ok(Self {
            endpoint,
            provider_id,
            model_id,
            client,
        })
    }
    pub async fn observe(
        &self,
        actor: &ActorEngine,
        scope: &Scope,
        owner: &Scope,
        reservation_id: &str,
        prompt: &str,
        catalog: Option<&CatalogModel>,
    ) -> Result<UsageObservation, MemoryError> {
        authorize(scope, owner)?;
        if prompt.is_empty() || prompt.len() > 65536 {
            return Err(ContextError::Capacity.into());
        }
        if catalog.is_some_and(|model| model.id != self.model_id) {
            return Err(ContextError::ScopeMismatch.into());
        }
        let reservation = {
            let _writer = WRITER.lock().await;
            let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
            let tail = actor.stats().await?.applied.last_lsn;
            let mut state = load(actor, scope).await?;
            let reservation = state
                .reservations
                .get(reservation_id)
                .ok_or(ContextError::Stale)?
                .clone();
            if reservation.input_digest != digest_bytes(prompt.as_bytes()) {
                return Err(ContextError::Conflict.into());
            }
            if reservation.attribution.provider_id != self.provider_id
                || reservation.attribution.model_id != self.model_id
            {
                return Err(ContextError::ScopeMismatch.into());
            }
            if let Some(id) = &reservation.observation_id {
                let observation = state.observations[id].clone();
                let needs_settlement = reservation.settled_tokens.is_none();
                drop(_guard);
                drop(_writer);
                if needs_settlement {
                    reconcile_unknown(actor, scope, owner, reservation_id, id).await?;
                }
                return Ok(observation);
            }
            if !matches!(
                state.accounting.jobs[&reservation.lease.job_id].status,
                JobStatus::Running(_)
            ) {
                return Err(ContextError::Stale.into());
            }
            if reservation.dispatched {
                return Err(invalid("provider dispatch uncertain; not repeated"));
            }
            if reservation.lease.expires_ms <= now_ms()? || reservation.settled_tokens.is_some() {
                return Err(ContextError::Stale.into());
            }
            state
                .reservations
                .get_mut(reservation_id)
                .unwrap()
                .dispatched = true;
            append(actor, tail, &mut state).await?;
            reservation
        };
        let response=self.client.post(&self.endpoint).json(&serde_json::json!({"model":self.model_id,"stream":false,"options":{"temperature":0,"num_predict":reservation.reserved_tokens.min(128)},"messages":[{"role":"user","content":prompt}]})).send().await;
        let response = match response {
            Ok(response) => response,
            Err(_) => {
                record_failure(actor, scope, owner, reservation_id, None).await?;
                return Err(invalid("provider response unavailable; usage unknown"));
            }
        };
        let status = response.status();
        if response
            .content_length()
            .is_some_and(|len| len > 8 * 1024 * 1024)
        {
            record_failure(actor, scope, owner, reservation_id, None).await?;
            return Err(ContextError::Capacity.into());
        }
        let mut response = response;
        let mut bytes = Vec::new();
        loop {
            let chunk = match response.chunk().await {
                Ok(chunk) => chunk,
                Err(_) => {
                    record_failure(actor, scope, owner, reservation_id, Some(bytes)).await?;
                    return Err(invalid("provider response interrupted; usage unknown"));
                }
            };
            let Some(chunk) = chunk else {
                break;
            };
            if bytes.len().saturating_add(chunk.len()) > 8 * 1024 * 1024 {
                record_failure(actor, scope, owner, reservation_id, Some(bytes)).await?;
                return Err(ContextError::Capacity.into());
            }
            bytes.extend_from_slice(&chunk);
        }
        let observation_id = digest_bytes(&serde_json::to_vec(&(scope, reservation_id, &bytes))?);
        let snapshot = ProviderUsageSnapshot::parse(
            &bytes,
            ObservationMetadata {
                provider_id: self.provider_id.clone(),
                source: "native-configured-provider-response".into(),
                evidence_id: observation_id.clone(),
                observed_at_ns: now_ns()?,
                expires_at_ns: None,
                format: ObservationFormat::Ollama,
            },
        );
        let snapshot = match snapshot {
            Ok(snapshot) => snapshot,
            Err(e) => {
                record_failure(actor, scope, owner, reservation_id, Some(bytes)).await?;
                return Err(invalid(&e.to_string()));
            }
        };
        let accepted_response = status.is_success()
            && snapshot.raw["model"].as_str() == Some(&self.model_id)
            && snapshot.raw["done"] == true;
        let catalog_estimate = if let Some(model) = catalog {
            if model.id != self.model_id {
                return Err(ContextError::ScopeMismatch.into());
            }
            Some(
                account_catalog_usage(model, &snapshot.tokens, snapshot.tokens.input)
                    .map_err(|e| invalid(&e.to_string()))?,
            )
        } else {
            None
        };
        let observation = UsageObservation {
            id: observation_id.clone(),
            reservation_id: reservation_id.into(),
            attribution: reservation.attribution,
            snapshot,
            catalog_estimate,
            accepted_response,
        };
        let _writer = WRITER.lock().await;
        let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
        let tail = actor.stats().await?.applied.last_lsn;
        let mut state = load(actor, scope).await?;
        state
            .observations
            .insert(observation_id.clone(), observation.clone());
        state
            .reservations
            .get_mut(reservation_id)
            .ok_or(ContextError::Stale)?
            .observation_id = Some(observation_id.clone());
        append(actor, tail, &mut state).await?;
        drop(_guard);
        drop(_writer);
        reconcile_unknown(actor, scope, owner, reservation_id, &observation_id).await?;
        Ok(observation)
    }
}

async fn record_failure(
    actor: &ActorEngine,
    scope: &Scope,
    owner: &Scope,
    id: &str,
    bytes: Option<Vec<u8>>,
) -> Result<(), MemoryError> {
    mark_unknown(actor, scope, owner, id).await?;
    if let Some(bytes) = bytes {
        let _writer = WRITER.lock().await;
        let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
        let tail = actor.stats().await?.applied.last_lsn;
        let mut state = load(actor, scope).await?;
        state
            .reservations
            .get_mut(id)
            .ok_or(ContextError::Stale)?
            .failed_response_bytes = Some(bytes);
        append(actor, tail, &mut state).await?;
    }
    Ok(())
}
pub async fn expire(
    actor: &ActorEngine,
    scope: &Scope,
    owner: &Scope,
) -> Result<UsageState, MemoryError> {
    authorize(scope, owner)?;
    let _writer = WRITER.lock().await;
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let tail = actor.stats().await?.applied.last_lsn;
    let mut state = load(actor, scope).await?;
    let mut accounting = MaintenanceScheduler::from_snapshot(state.accounting.clone())?;
    if accounting.expire(now_ms()?)? > 0 {
        state.accounting = accounting.snapshot();
        append(actor, tail, &mut state).await?;
    }
    Ok(state)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RunnerDispatchBinding {
    pub role_work_digest: String,
    pub provider_request_digest: String,
    pub source: hm_fabric::role_store::SourceFence,
    pub profile_fence: String,
    pub worker_id: String,
}
fn runner_binding_matches(
    reservation: &UsageReservation,
    binding: &hm_fabric::role_runner::CanonicalUsageBinding,
) -> bool {
    reservation.id == binding.reservation_id
        && reservation.request_digest == binding.reservation_digest
        && reservation.input_digest == binding.input_digest
        && reservation.reserved_tokens == binding.reserved_tokens
        && reservation.lease == binding.lease
}
pub async fn begin_runner_dispatch(
    actor: &ActorEngine,
    scope: &Scope,
    owner: &Scope,
    registry: &hm_fabric::role_store::DurableRoleRegistry,
    work_id: &str,
    declaration: &hm_fabric::role_runner::RunnerDeclaration,
    worker_id: &str,
) -> Result<hm_fabric::role_runner::CanonicalUsageBinding, MemoryError> {
    authorize(scope, owner)?;
    if registry.scope() != scope {
        return Err(ContextError::ScopeMismatch.into());
    }
    let record = registry
        .get(work_id)
        .map_err(|e| invalid(&e.to_string()))?
        .ok_or(ContextError::Stale)?;
    if record.state != hm_fabric::role_store::WorkState::Approved
        || record.descriptor
            != declaration
                .descriptor()
                .map_err(|e| invalid(&e.to_string()))?
    {
        return Err(ContextError::Stale.into());
    }
    let input: hm_fabric::role_runner::RunnerInput = serde_json::from_slice(&record.work.payload)?;
    if input.usage.input_digest != digest_bytes(input.prompt.as_bytes()) {
        return Err(ContextError::Conflict.into());
    }
    let dispatch = RunnerDispatchBinding {
        role_work_digest: digest_bytes(&serde_json::to_vec(&record.work)?),
        provider_request_digest: hm_fabric::role_runner::provider_request_digest(
            declaration,
            &input,
        )
        .map_err(|e| invalid(&e.to_string()))?,
        source: record.work.source.clone(),
        profile_fence: hm_context::provider_continuity::fence_profile(
            &declaration.profile,
            declaration.budget,
            &serde_json::to_string(&record.work.source)?,
        )?
        .digest,
        worker_id: worker_id.into(),
    };
    let _writer = WRITER.lock().await;
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let tail = actor.stats().await?.applied.last_lsn;
    let mut state = load(actor, scope).await?;
    let reservation = state
        .reservations
        .get(&input.usage.reservation_id)
        .ok_or(ContextError::Stale)?;
    if !runner_binding_matches(reservation, &input.usage)
        || reservation.attribution.worker_id != worker_id
        || reservation.attribution.model_id != declaration.profile.model_id
        || reservation.attribution.provider_id != "ollama-local"
    {
        return Err(ContextError::ScopeMismatch.into());
    }
    if reservation.dispatched
        || reservation.observation_id.is_some()
        || reservation.settled_tokens.is_some()
        || reservation.lease.expires_ms <= now_ms()?
        || state.accounting.jobs[&reservation.lease.job_id].status
            != JobStatus::Running(reservation.lease.clone())
    {
        return Err(ContextError::Stale.into());
    }
    let reservation = state
        .reservations
        .get_mut(&input.usage.reservation_id)
        .ok_or(ContextError::Stale)?;
    reservation.dispatched = true;
    reservation.runner_dispatch = Some(dispatch);
    append(actor, tail, &mut state).await?;
    Ok(input.usage)
}
pub async fn ingest_runner_receipt(
    actor: &ActorEngine,
    scope: &Scope,
    owner: &Scope,
    receipt: &hm_fabric::role_runner::AuthenticatedRunnerReceipt,
) -> Result<UsageObservation, MemoryError> {
    authorize(scope, owner)?;
    if receipt.scope() != scope {
        return Err(ContextError::ScopeMismatch.into());
    }
    let outcome = receipt.outcome();
    let _writer = WRITER.lock().await;
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let tail = actor.stats().await?.applied.last_lsn;
    let mut state = load(actor, scope).await?;
    let reservation = state
        .reservations
        .get(&outcome.usage.reservation_id)
        .ok_or(ContextError::Stale)?
        .clone();
    let dispatch = reservation
        .runner_dispatch
        .as_ref()
        .ok_or(ContextError::Stale)?;
    if !reservation.dispatched
        || !runner_binding_matches(&reservation, &outcome.usage)
        || dispatch.worker_id != receipt.worker_id()
        || reservation.attribution.worker_id != receipt.worker_id()
        || dispatch.source != outcome.source
        || dispatch.role_work_digest != outcome.request_digest
        || dispatch.provider_request_digest != outcome.provider_request_digest
        || dispatch.profile_fence != outcome.profile_fence
    {
        return Err(ContextError::Conflict.into());
    }
    let observation_id = digest_bytes(&serde_json::to_vec(&(
        scope,
        &reservation.id,
        &outcome.original_provider_bytes,
    ))?);
    if let Some(existing) = &reservation.observation_id {
        if existing != &observation_id {
            return Err(ContextError::Conflict.into());
        }
        return state
            .observations
            .get(existing)
            .cloned()
            .ok_or(ContextError::Stale.into());
    }
    let snapshot = ProviderUsageSnapshot::parse(
        &outcome.original_provider_bytes,
        ObservationMetadata {
            provider_id: reservation.attribution.provider_id.clone(),
            source: "authenticated-original-runner-response".into(),
            evidence_id: observation_id.clone(),
            observed_at_ns: now_ns()?,
            expires_at_ns: None,
            format: ObservationFormat::Ollama,
        },
    )
    .map_err(|e| invalid(&e.to_string()))?;
    let accepted_response = snapshot.raw["model"].as_str()
        == Some(reservation.attribution.model_id.as_str())
        && snapshot.raw["done"] == true
        && snapshot.error.is_none();
    let observation = UsageObservation {
        id: observation_id.clone(),
        reservation_id: reservation.id.clone(),
        attribution: reservation.attribution.clone(),
        snapshot,
        catalog_estimate: None,
        accepted_response,
    };
    state
        .observations
        .insert(observation_id.clone(), observation.clone());
    settle_state(&mut state, &reservation.id, &observation_id, now_ms()?)?;
    append(actor, tail, &mut state).await?;
    Ok(observation)
}
