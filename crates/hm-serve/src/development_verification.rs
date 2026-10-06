use crate::{
    actor::{ActorEngine, IncomingEvent},
    context_memory::{self, MemoryError},
    development_admission, development_mapping,
};
use hm_context::{ContextError, Scope, development::*, maintenance::Usage};
use hm_core::{ConversationId, LSN};
use hm_cortex::{
    development_mapping::{AuthorizedRepository, RecordVerification, VerificationBatch},
    development_verification::{self, VerificationCycle},
};
use hm_llm::LlmProvider;
use hm_schema::{
    event::{self, Boundary, CURRENT_SCHEMA_VERSION},
    events::{EventEnvelope, EventPayload, ProviderFrame, Retention, Sensitivity},
};
use serde::{Deserialize, Serialize};
use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

static WRITER: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
const PROVIDER: &str = "hypermind/development-verification/v1";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PendingBatch {
    pub plan: DevelopmentPlan,
    pub records: Vec<RecordVerification>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CycleState {
    pub sequence: u64,
    pub cycle: VerificationCycle,
    pub running_until_ns: Option<i64>,
    pub pending: Option<PendingBatch>,
    pub last_error: Option<String>,
    pub last_usage: Usage,
    pub last_capability_revision: Option<u64>,
}
fn now() -> Result<i64, MemoryError> {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ContextError::Unavailable("clock".into()))?
            .as_nanos(),
    )
    .map_err(|_| ContextError::Capacity.into())
}
async fn load(
    actor: &ActorEngine,
    scope: &Scope,
    id: &str,
) -> Result<Option<CycleState>, MemoryError> {
    let mut saved: Option<CycleState> = None;
    for frame in actor.frames_since(LSN::new(0), None, usize::MAX).await? {
        if frame.header.kind != hm_ledger::frame::EventKind::ProviderFrame {
            continue;
        }
        let parsed = event::verify_event(
            &frame.sealed_payload,
            event::EventKind::ProviderFrame,
            Boundary::Disk,
        )?;
        let EventPayload::ProviderFrame(p) = parsed.envelope.payload else {
            continue;
        };
        if p.provider != PROVIDER {
            continue;
        }
        let next: CycleState = serde_json::from_slice(&p.api_content)?;
        if next.cycle.scope != *scope || next.cycle.id != id {
            continue;
        }
        next.cycle.validate()?;
        if next.sequence != saved.as_ref().map_or(1, |s| s.sequence + 1) {
            return Err(ContextError::Conflict.into());
        }
        saved = Some(next);
    }
    Ok(saved)
}
async fn save(actor: &ActorEngine, saved: &mut CycleState) -> Result<(), MemoryError> {
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let tail = actor.stats().await?.applied.last_lsn;
    let current = load(actor, &saved.cycle.scope, &saved.cycle.id).await?;
    if current.as_ref().map_or(0, |s| s.sequence) != saved.sequence {
        return Err(ContextError::Stale.into());
    }
    saved.cycle.validate()?;
    saved.sequence = saved
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
async fn reconcile(actor: &ActorEngine, saved: &mut CycleState) -> Result<(), MemoryError> {
    if let Some(pending) = &saved.pending {
        let memory = context_memory::rebuild(actor, &saved.cycle.scope).await?;
        if let Some(receipt) = memory.development_receipts.get(&pending.plan.id) {
            let batch = VerificationBatch {
                plan: pending.plan.clone(),
                records: pending.records.clone(),
                skipped: vec![],
            };
            saved.cycle.complete_batch(&batch, receipt)?;
            saved.last_usage = batch.plan.usage;
            saved.pending = None;
            saved.running_until_ns = None;
        }
    }
    Ok(())
}
fn authorize(
    state: &CycleState,
    scope: &Scope,
    caller: &Scope,
    worker_id: &str,
) -> Result<(), MemoryError> {
    if state.cycle.scope != *scope
        || state.cycle.principal != *caller
        || state.cycle.worker_id != worker_id
    {
        return Err(ContextError::ScopeMismatch.into());
    }
    Ok(())
}
async fn full_snapshot(
    actor: &ActorEngine,
    scope: &Scope,
    caller: &Scope,
    worker_id: &str,
    capability_id: &str,
) -> Result<EvidenceSnapshot, MemoryError> {
    let memory = context_memory::rebuild(actor, scope).await?;
    let cap = memory
        .worker_capabilities
        .get(capability_id)
        .ok_or(ContextError::ScopeMismatch)?;
    if !cap.allowed_kinds.contains(&DevelopmentKind::Verification) {
        return Err(ContextError::ScopeMismatch.into());
    }
    development_admission::snapshot(
        actor,
        scope,
        caller,
        worker_id,
        SnapshotRequest {
            capability_id: capability_id.into(),
            source_ids: cap.source_ids.clone(),
            record_ids: cap.record_ids.clone(),
        },
    )
    .await
}
pub async fn open_cycle(
    actor: &ActorEngine,
    scope: &Scope,
    caller: &Scope,
    worker_id: &str,
    capability_id: &str,
    repository: &AuthorizedRepository,
    id: &str,
) -> Result<CycleState, MemoryError> {
    let _guard = WRITER.lock().await;
    if let Some(mut state) = load(actor, scope, id).await? {
        authorize(&state, scope, caller, worker_id)?;
        if state.cycle.capability_id != capability_id {
            return Err(ContextError::Conflict.into());
        }
        reconcile(actor, &mut state).await?;
        save(actor, &mut state).await?;
        return Ok(state);
    }
    let snapshot = full_snapshot(actor, scope, caller, worker_id, capability_id).await?;
    let mut saved = CycleState {
        sequence: 0,
        cycle: development_verification::open_cycle(&snapshot, repository, id)?,
        running_until_ns: None,
        pending: None,
        last_error: None,
        last_usage: Usage::Known(0),
        last_capability_revision: None,
    };
    save(actor, &mut saved).await?;
    Ok(saved)
}
pub async fn inspect(
    actor: &ActorEngine,
    scope: &Scope,
    caller: &Scope,
    id: &str,
) -> Result<CycleState, MemoryError> {
    let _guard = WRITER.lock().await;
    let mut saved = load(actor, scope, id).await?.ok_or(ContextError::Stale)?;
    if caller != scope && caller != &saved.cycle.principal {
        return Err(ContextError::ScopeMismatch.into());
    }
    reconcile(actor, &mut saved).await?;
    save(actor, &mut saved).await?;
    Ok(saved)
}
pub async fn set_cancelled(
    actor: &ActorEngine,
    scope: &Scope,
    owner: &Scope,
    id: &str,
    cancelled: bool,
) -> Result<CycleState, MemoryError> {
    if owner != scope {
        return Err(ContextError::ScopeMismatch.into());
    }
    let _guard = WRITER.lock().await;
    let mut saved = load(actor, scope, id).await?.ok_or(ContextError::Stale)?;
    reconcile(actor, &mut saved).await?;
    saved.cycle.cancelled = cancelled;
    saved.cycle.generation = saved
        .cycle
        .generation
        .checked_add(1)
        .ok_or(ContextError::Capacity)?;
    if saved.running_until_ns.take().is_some() {
        saved.last_usage = Usage::Unknown;
    }
    save(actor, &mut saved).await?;
    Ok(saved)
}
pub async fn run_batch(
    actor: &ActorEngine,
    scope: &Scope,
    caller: &Scope,
    worker_id: &str,
    id: &str,
    repository: &AuthorizedRepository,
    provider: Arc<dyn LlmProvider>,
    limit: usize,
    timeout: Duration,
    now_ns: i64,
) -> Result<CycleState, MemoryError> {
    if limit == 0 || limit > 64 {
        return Err(ContextError::Capacity.into());
    }
    let (snapshot, generation, plan_id) = {
        let _guard = WRITER.lock().await;
        let mut saved = load(actor, scope, id).await?.ok_or(ContextError::Stale)?;
        authorize(&saved, scope, caller, worker_id)?;
        reconcile(actor, &mut saved).await?;
        if saved.cycle.cancelled {
            return Err(ContextError::Stale.into());
        }
        if let Some(pending) = saved.pending.clone() {
            let batch = VerificationBatch {
                plan: pending.plan,
                records: pending.records,
                skipped: vec![],
            };
            match development_mapping::publish(actor, scope, caller, worker_id, repository, batch)
                .await
            {
                Ok(Some(_)) => reconcile(actor, &mut saved).await?,
                outcome => {
                    saved.pending = None;
                    saved.running_until_ns = None;
                    saved.last_error = Some(format!("pending publication refused: {outcome:?}"));
                }
            }
            save(actor, &mut saved).await?;
            return Ok(saved);
        }
        if saved
            .running_until_ns
            .is_some_and(|end| end > now().unwrap_or(i64::MAX))
        {
            return Err(ContextError::Conflict.into());
        }
        let all =
            full_snapshot(actor, scope, caller, worker_id, &saved.cycle.capability_id).await?;
        if saved
            .last_capability_revision
            .is_some_and(|revision| all.capability_revision <= revision)
        {
            return Err(ContextError::Conflict.into());
        }
        saved.cycle.refresh(&all, repository)?;
        let selected = saved
            .cycle
            .next_batch(limit.min(all.budget.max_mutations))?;
        if selected.is_empty() {
            save(actor, &mut saved).await?;
            return Ok(saved);
        }
        let snapshot = development_admission::snapshot(
            actor,
            scope,
            caller,
            worker_id,
            SnapshotRequest {
                capability_id: saved.cycle.capability_id.clone(),
                source_ids: all.sources.iter().map(|s| s.id.clone()).collect(),
                record_ids: selected,
            },
        )
        .await?;
        if snapshot.capability_digest != all.capability_digest
            || snapshot.sources != all.sources
            || snapshot.records.iter().any(|record| {
                !all.records
                    .iter()
                    .any(|old| old.id == record.id && old.revision_digest == record.revision_digest)
            })
        {
            return Err(ContextError::Stale.into());
        }
        saved.cycle.generation = saved
            .cycle
            .generation
            .checked_add(1)
            .ok_or(ContextError::Capacity)?;
        let plan_id = format!("verification:{}:{}", id, saved.cycle.generation);
        saved.running_until_ns = Some(
            now()?
                .checked_add(i64::try_from(timeout.as_nanos()).map_err(|_| ContextError::Capacity)?)
                .ok_or(ContextError::Capacity)?,
        );
        saved.last_error = None;
        saved.last_usage = Usage::Unknown;
        saved.last_capability_revision = Some(snapshot.capability_revision);
        save(actor, &mut saved).await?;
        (snapshot, saved.cycle.generation, plan_id)
    };
    let mappings = hm_cortex::development_mapping::map_evidence(&snapshot, repository)?;
    let task = tokio::task::spawn_blocking(move || {
        hm_cortex::development_mapping::verify_changed(
            &snapshot,
            &mappings,
            &[],
            provider.as_ref(),
            &plan_id,
            now_ns,
        )
    });
    let result = tokio::time::timeout(timeout, task).await;
    let _guard = WRITER.lock().await;
    let mut saved = load(actor, scope, id).await?.ok_or(ContextError::Stale)?;
    if saved.cycle.generation != generation || saved.cycle.cancelled {
        return Err(ContextError::Stale.into());
    }
    let batch = match result {
        Ok(Ok(Ok(batch))) => batch,
        failure => {
            saved.running_until_ns = None;
            saved.last_error = Some(format!("verification did not complete: {failure:?}"));
            saved.last_usage = Usage::Unknown;
            save(actor, &mut saved).await?;
            return Ok(saved);
        }
    };
    saved.last_usage = batch.plan.usage;
    saved.pending = Some(PendingBatch {
        plan: batch.plan.clone(),
        records: batch.records.clone(),
    });
    save(actor, &mut saved).await?;
    match development_mapping::publish(actor, scope, caller, worker_id, repository, batch).await {
        Ok(Some(_)) => {
            reconcile(actor, &mut saved).await?;
            saved.last_error = None;
        }
        Ok(None) => {
            saved.pending = None;
            saved.running_until_ns = None;
            saved.last_error = Some("unmapped batch".into());
        }
        Err(error) => {
            saved.pending = None;
            saved.running_until_ns = None;
            saved.last_error = Some(error.to_string());
        }
    }
    save(actor, &mut saved).await?;
    Ok(saved)
}
