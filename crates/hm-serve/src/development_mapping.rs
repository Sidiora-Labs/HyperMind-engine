use crate::{
    actor::ActorEngine,
    context_memory::{self, MemoryError},
    development_admission,
};
use hm_context::{ContextError, Scope, development::*};
use hm_cortex::development_mapping::{
    self, AuthorizedRepository, MappingPlan, RecordVerification, VerificationBatch,
};
use hm_llm::LlmProvider;

pub async fn map_evidence(
    actor: &ActorEngine,
    scope: &Scope,
    caller: &Scope,
    worker_id: &str,
    request: SnapshotRequest,
    repository: &AuthorizedRepository,
) -> Result<(EvidenceSnapshot, MappingPlan), MemoryError> {
    let snapshot =
        development_admission::snapshot(actor, scope, caller, worker_id, request).await?;
    let mappings = development_mapping::map_evidence(&snapshot, repository)?;
    Ok((snapshot, mappings))
}
pub async fn previous_verifications(
    actor: &ActorEngine,
    scope: &Scope,
    caller: &Scope,
    snapshot: &EvidenceSnapshot,
) -> Result<Vec<RecordVerification>, MemoryError> {
    if &snapshot.scope != scope || &snapshot.principal != caller {
        return Err(ContextError::ScopeMismatch.into());
    }
    let state = context_memory::rebuild(actor, scope).await?;
    let now_ns = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| ContextError::Unavailable("clock".into()))?
            .as_nanos(),
    )
    .map_err(|_| ContextError::Capacity)?;
    let mut previous = std::collections::BTreeMap::new();
    for verification in state.verifications.values() {
        let Some(record) = snapshot
            .records
            .iter()
            .find(|r| r.id == verification.record_id)
        else {
            continue;
        };
        state
            .read(caller, &record.id, now_ns)?
            .ok_or(ContextError::ScopeMismatch)?;
        if verification.metadata.is_null() {
            continue;
        }
        let result: RecordVerification = serde_json::from_value(verification.metadata.clone())?;
        if result.record_id != record.id {
            return Err(ContextError::Conflict.into());
        }
        let old = previous
            .entry(record.id.clone())
            .or_insert((i64::MIN, result.clone()));
        if verification.created_at_ns > old.0 {
            *old = (verification.created_at_ns, result);
        }
    }
    Ok(previous.into_values().map(|(_, result)| result).collect())
}
pub async fn verify_changed(
    actor: &ActorEngine,
    scope: &Scope,
    caller: &Scope,
    worker_id: &str,
    snapshot: EvidenceSnapshot,
    repository: &AuthorizedRepository,
    provider: std::sync::Arc<dyn LlmProvider>,
    plan_id: &str,
    now_ns: i64,
) -> Result<VerificationBatch, MemoryError> {
    if snapshot.worker_id != worker_id || snapshot.scope != *scope || snapshot.principal != *caller
    {
        return Err(ContextError::ScopeMismatch.into());
    }
    let mappings = development_mapping::map_evidence(&snapshot, repository)?;
    let previous = previous_verifications(actor, scope, caller, &snapshot).await?;
    let plan_id = plan_id.to_owned();
    tokio::task::spawn_blocking(move || {
        development_mapping::verify_changed(
            &snapshot,
            &mappings,
            &previous,
            provider.as_ref(),
            &plan_id,
            now_ns,
        )
    })
    .await
    .map_err(|_| ContextError::Unavailable("verification worker stopped".into()))?
    .map_err(MemoryError::from)
}
pub async fn publish(
    actor: &ActorEngine,
    scope: &Scope,
    caller: &Scope,
    worker_id: &str,
    repository: &AuthorizedRepository,
    batch: VerificationBatch,
) -> Result<Option<WorkerReceipt>, MemoryError> {
    development_mapping::validate_repository(&batch.plan.evidence, repository)?;
    let expected = development_mapping::map_evidence(&batch.plan.evidence, repository)?;
    for verified in &batch.records {
        if !expected
            .records
            .iter()
            .any(|m| m.record_id == verified.record_id && m.fingerprint == verified.fingerprint)
        {
            return Err(ContextError::Stale.into());
        }
    }
    if batch.plan.mutations.is_empty() {
        return Ok(None);
    }
    Ok(Some(
        development_admission::admit(actor, scope, caller, worker_id, batch.plan).await?,
    ))
}
