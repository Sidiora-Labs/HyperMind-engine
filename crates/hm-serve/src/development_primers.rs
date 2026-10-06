use crate::{
    actor::ActorEngine,
    context_memory::{
        self, MemoryCommand, MemoryError, MemoryRecord, MemoryRequest, MemorySource, RecordKind,
    },
    development_admission,
};
use hm_context::{development::*, digest_bytes, validate_id, Authority, ContextError, Scope};
use hm_cortex::development_primers::{self, PrimerObservation, INDEX_ID};
use hm_llm::LlmProvider;
use serde_json::{json, Value};
use std::sync::Arc;
fn invalid(s: &str) -> MemoryError {
    ContextError::Invalid(s.into()).into()
}
pub async fn collect_session(
    actor: &ActorEngine,
    scope: &Scope,
    owner: &Scope,
    session: &str,
    conversation: &str,
) -> Result<Vec<String>, MemoryError> {
    if owner != scope {
        return Err(ContextError::ScopeMismatch.into());
    }
    validate_id(session)?;
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let history = crate::context_history::replay(actor, scope, session, conversation)
        .await
        .map_err(|e| invalid(&format!("primer history: {e}")))?;
    let messages = history.history.visible_messages();
    if messages.len() > 64 {
        return Err(ContextError::Capacity.into());
    }
    let mut ids = Vec::new();
    for message in messages {
        if message.authority != Authority::UserAsserted
            || message.role != hm_context::MessageRole::User
        {
            continue;
        }
        let span = history.history.source_span(&message.id)?;
        let content = history.history.recover(scope, &span)?;
        let id = format!(
            "primer-evidence-{}",
            digest_bytes(&serde_json::to_vec(&(
                session,
                &message.id,
                &message.source_digest
            ))?)
        );
        let state = context_memory::rebuild(actor, scope).await?;
        if !state.sources.contains_key(&id) {
            let tail = actor.stats().await?.applied.last_lsn;
            context_memory::execute_fenced_locked(
                actor,
                scope,
                owner,
                MemoryRequest {
                    version: 1,
                    scope: scope.clone(),
                    request_id: format!("collect-{id}"),
                    command: MemoryCommand::Source {
                        source: MemorySource {
                            id: id.clone(),
                            digest: digest_bytes(&content),
                            content,
                            locator: format!("primer-session:{session}"),
                            occurred_at_ns: message.occurred_at_ns,
                            recorded_at_ns: message.recorded_at_ns,
                            tombstoned: false,
                        },
                    },
                },
                Some(tail),
            )
            .await?;
        }
        let state = context_memory::rebuild(actor, scope).await?;
        let old = state.records.get(INDEX_ID);
        if old.is_none_or(|r| r.metadata["sessions"][&id].as_str() != Some(session)) {
            if old
                .and_then(|r| r.metadata["sessions"].as_object())
                .is_some_and(|m| m.len() >= 256 && !m.contains_key(&id))
            {
                return Err(ContextError::Capacity.into());
            }
            let mut index = old.cloned().unwrap_or_else(|| {
                let mut r = MemoryRecord::new(
                    INDEX_ID,
                    RecordKind::Note,
                    "Authorized primer evidence sessions",
                    message.recorded_at_ns,
                );
                r.category = "primer_index".into();
                r.authority = Authority::UserAsserted;
                r.pinned = true;
                r.metadata = json!({"sessions":{}});
                r
            });
            index.metadata["sessions"][&id] = json!(session);
            let command = if let Some(old) = old {
                index.revision = old.revision + 1;
                index.revision_digest.clear();
                MemoryCommand::Revise {
                    record: index,
                    expected_revision: old.revision,
                }
            } else {
                MemoryCommand::Create { record: index }
            };
            let tail = actor.stats().await?.applied.last_lsn;
            context_memory::execute_fenced_locked(
                actor,
                scope,
                owner,
                MemoryRequest {
                    version: 1,
                    scope: scope.clone(),
                    request_id: format!("associate-{id}"),
                    command,
                },
                Some(tail),
            )
            .await?;
        }
        ids.push(id);
    }
    Ok(ids)
}
pub async fn enqueue(
    actor: &ActorEngine,
    scope: &Scope,
    owner: &Scope,
    job_id: &str,
    target_id: &str,
    refresh: bool,
) -> Result<context_memory::MemoryReceipt, MemoryError> {
    if owner != scope {
        return Err(ContextError::ScopeMismatch.into());
    }
    validate_id(job_id)?;
    validate_id(target_id)?;
    let mut record = MemoryRecord::new(job_id, RecordKind::Note, "Pending primer development", 0);
    record.category = "primer_backlog".into();
    record.authority = Authority::UserAsserted;
    record.metadata = json!({"primer_job":true,"target_id":target_id,"refresh":refresh});
    context_memory::execute(
        actor,
        scope,
        owner,
        MemoryRequest {
            version: 1,
            scope: scope.clone(),
            request_id: format!("enqueue-{job_id}"),
            command: MemoryCommand::Create { record },
        },
    )
    .await
}
pub async fn run(
    actor: &ActorEngine,
    scope: &Scope,
    caller: &Scope,
    worker: &str,
    mut request: SnapshotRequest,
    job_id: &str,
    provider: Arc<dyn LlmProvider>,
) -> Result<WorkerReceipt, MemoryError> {
    let state = context_memory::rebuild(actor, scope).await?;
    let cap = state
        .worker_capabilities
        .get(&request.capability_id)
        .ok_or(ContextError::ScopeMismatch)?;
    if !cap.allowed_kinds.contains(&DevelopmentKind::Primer) {
        return Err(ContextError::ScopeMismatch.into());
    }
    let job = state
        .records
        .get(job_id)
        .ok_or_else(|| invalid("primer job unavailable"))?;
    let target = job.metadata["target_id"]
        .as_str()
        .ok_or_else(|| invalid("invalid primer job"))?
        .to_string();
    let refresh = job.metadata["refresh"]
        .as_bool()
        .ok_or_else(|| invalid("invalid primer job"))?;
    if (!refresh && !cap.new_record_ids.contains(&target))
        || (refresh && !cap.record_ids.contains(&target))
    {
        return Err(ContextError::ScopeMismatch.into());
    }
    request.record_ids.insert(INDEX_ID.into());
    request.record_ids.insert(job_id.into());
    if refresh {
        request.record_ids.insert(target.clone());
    }
    let evidence = development_admission::snapshot(actor, scope, caller, worker, request).await?;
    let index = evidence
        .records
        .iter()
        .find(|r| r.id == INDEX_ID)
        .ok_or(ContextError::ScopeMismatch)?;
    let mut observations = Vec::new();
    for source in &evidence.sources {
        let native = state
            .sources
            .get(&source.id)
            .ok_or(ContextError::ScopeMismatch)?;
        let session = index.metadata["sessions"][&source.id]
            .as_str()
            .ok_or(ContextError::ScopeMismatch)?;
        if native.locator != format!("primer-session:{session}") {
            return Err(ContextError::ScopeMismatch.into());
        }
        observations.push(PrimerObservation {
            source_id: source.id.clone(),
            session_id: session.into(),
        });
    }
    let job_id = job_id.to_owned();
    let plan = tokio::task::spawn_blocking(move || {
        development_primers::develop(
            evidence,
            &observations,
            &job_id,
            &target,
            refresh,
            provider.as_ref(),
        )
    })
    .await
    .map_err(|e| invalid(&format!("primer task: {e}")))??;
    development_admission::admit(actor, scope, caller, worker, plan).await
}
pub async fn inspect(
    actor: &ActorEngine,
    scope: &Scope,
    owner: &Scope,
) -> Result<Value, MemoryError> {
    if owner != scope {
        return Err(ContextError::ScopeMismatch.into());
    }
    let state = context_memory::rebuild(actor, scope).await?;
    Ok(
        json!({"primers":state.records.values().filter(|r|r.kind==RecordKind::Primer).collect::<Vec<_>>(),"pending":state.records.values().filter(|r|r.category=="primer_backlog"&&r.status==context_memory::RecordStatus::Active).collect::<Vec<_>>()}),
    )
}
