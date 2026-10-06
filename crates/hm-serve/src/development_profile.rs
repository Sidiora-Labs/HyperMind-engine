use crate::{
    actor::ActorEngine,
    context_memory::{
        self, MemoryCommand, MemoryError, MemoryRecord, MemoryRequest, MemorySource, RecordKind,
    },
    development_admission,
};
use hm_context::{development::*, digest_bytes, validate_id, Authority, ContextError, Scope};
use hm_cortex::development_profile::{self, ProfileObservation};
use hm_llm::LlmProvider;
use serde_json::{json, Value};
use std::sync::Arc;
pub const POLICY_ID: &str = "profile-collection-policy";
fn invalid(s: &str) -> MemoryError {
    ContextError::Invalid(s.into()).into()
}
fn enabled(state: &context_memory::MemoryProjection) -> bool {
    state.records.get(POLICY_ID).is_some_and(|r| {
        r.status == context_memory::RecordStatus::Active
            && r.authority == Authority::UserAsserted
            && r.metadata["enabled"] == true
    })
}
pub async fn set_enabled(
    actor: &ActorEngine,
    scope: &Scope,
    owner: &Scope,
    enable: bool,
) -> Result<context_memory::MemoryReceipt, MemoryError> {
    if owner != scope {
        return Err(ContextError::ScopeMismatch.into());
    }
    let state = context_memory::rebuild(actor, scope).await?;
    let command = if let Some(old) = state.records.get(POLICY_ID) {
        let mut record = old.clone();
        record.revision = old.revision.checked_add(1).ok_or(ContextError::Capacity)?;
        record.revision_digest.clear();
        record.metadata["enabled"] = json!(enable);
        record.content = if enable {
            "Profile collection enabled"
        } else {
            "Profile collection disabled"
        }
        .into();
        MemoryCommand::Revise {
            record,
            expected_revision: old.revision,
        }
    } else {
        let mut record = MemoryRecord::new(
            POLICY_ID,
            RecordKind::Note,
            if enable {
                "Profile collection enabled"
            } else {
                "Profile collection disabled"
            },
            0,
        );
        record.category = "profile_policy".into();
        record.pinned = true;
        record.authority = Authority::UserAsserted;
        record.metadata = json!({"enabled":enable});
        MemoryCommand::Create { record }
    };
    context_memory::execute(
        actor,
        scope,
        owner,
        MemoryRequest {
            version: 1,
            scope: scope.clone(),
            request_id: format!(
                "profile-policy-{}-{enable}",
                state.records.get(POLICY_ID).map_or(0, |r| r.revision)
            ),
            command,
        },
    )
    .await
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
    let state = context_memory::rebuild(actor, scope).await?;
    if !enabled(&state) {
        return Err(invalid("profile collection is disabled"));
    }
    let history = crate::context_history::replay(actor, scope, session, conversation)
        .await
        .map_err(|e| invalid(&format!("profile history: {e}")))?;
    let mut ids = Vec::new();
    for message in history.history.visible_messages() {
        if message.authority != Authority::UserAsserted
            || message.role != hm_context::MessageRole::User
        {
            continue;
        }
        let span = history.history.source_span(&message.id)?;
        let content = history.history.recover(scope, &span)?;
        let id = format!(
            "profile-evidence-{}",
            digest_bytes(&serde_json::to_vec(&(
                session,
                &message.id,
                &message.source_digest
            ))?)
        );
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
                            locator: format!("profile-session:{session}"),
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
        let current = context_memory::rebuild(actor, scope).await?;
        let mut policy = current
            .records
            .get(POLICY_ID)
            .ok_or_else(|| invalid("profile policy unavailable"))?
            .clone();
        if policy.metadata["observations"][&id].as_str() != Some(session) {
            if !policy.metadata["observations"].is_object() {
                policy.metadata["observations"] = json!({});
            }
            policy.metadata["observations"][&id] = json!(session);
            let revision = policy.revision;
            policy.revision += 1;
            policy.revision_digest.clear();
            let tail = actor.stats().await?.applied.last_lsn;
            context_memory::execute_fenced_locked(
                actor,
                scope,
                owner,
                MemoryRequest {
                    version: 1,
                    scope: scope.clone(),
                    request_id: format!("associate-{id}"),
                    command: MemoryCommand::Revise {
                        record: policy,
                        expected_revision: revision,
                    },
                },
                Some(tail),
            )
            .await?;
        }
        ids.push(id);
    }
    Ok(ids)
}
pub async fn propose(
    actor: &ActorEngine,
    scope: &Scope,
    caller: &Scope,
    worker: &str,
    mut request: SnapshotRequest,
    record_id: &str,
    provider: Arc<dyn LlmProvider>,
) -> Result<WorkerReceipt, MemoryError> {
    let state = context_memory::rebuild(actor, scope).await?;
    if !enabled(&state) {
        return Err(invalid("profile collection is disabled"));
    }
    let capability = state
        .worker_capabilities
        .get(&request.capability_id)
        .ok_or(ContextError::ScopeMismatch)?;
    if !capability
        .allowed_kinds
        .contains(&DevelopmentKind::ProfileProposal)
        || !capability.new_record_ids.contains(record_id)
    {
        return Err(ContextError::ScopeMismatch.into());
    }
    request.record_ids.insert(POLICY_ID.into());
    let evidence = development_admission::snapshot(actor, scope, caller, worker, request).await?;
    if !evidence
        .records
        .iter()
        .any(|r| r.id == POLICY_ID && r.metadata["enabled"] == true)
    {
        return Err(invalid("profile opt-in not captured"));
    }
    let mut observations = Vec::new();
    for source in &evidence.sources {
        let native = state
            .sources
            .get(&source.id)
            .ok_or(ContextError::ScopeMismatch)?;
        let policy = evidence
            .records
            .iter()
            .find(|r| r.id == POLICY_ID)
            .ok_or(ContextError::ScopeMismatch)?;
        let session = policy.metadata["observations"][&source.id]
            .as_str()
            .ok_or(ContextError::ScopeMismatch)?;
        if native.locator != format!("profile-session:{session}") {
            return Err(ContextError::ScopeMismatch.into());
        }
        observations.push(ProfileObservation {
            source_id: source.id.clone(),
            session_id: session.into(),
        });
    }
    let record_id = record_id.to_owned();
    let plan = tokio::task::spawn_blocking(move || {
        development_profile::propose_profile(evidence, &observations, &record_id, provider.as_ref())
    })
    .await
    .map_err(|e| invalid(&format!("profile task: {e}")))??;
    let candidate = plan
        .proposal
        .as_ref()
        .ok_or_else(|| invalid("missing profile proposal"))?;
    let current = context_memory::rebuild(actor, scope).await?;
    for previous in current.development_proposals.values().filter(|p| {
        p.kind == DevelopmentKind::ProfileProposal && p.status == ProposalStatus::Rejected
    }) {
        for old in &previous.mutations {
            for proposed in &candidate.mutations {
                if let (
                    PlannedKnowledgeMutation::Create { record: a },
                    PlannedKnowledgeMutation::Create { record: b },
                ) = (old, proposed)
                {
                    if a.metadata["profile_attribute"] == b.metadata["profile_attribute"] {
                        return Err(invalid("owner rejected this profile candidate"));
                    }
                }
            }
        }
    }
    development_admission::admit(actor, scope, caller, worker, plan).await
}
pub async fn review(
    actor: &ActorEngine,
    scope: &Scope,
    owner: &Scope,
    decision: ProposalDecision,
) -> Result<WorkerReceipt, MemoryError> {
    if owner != scope {
        return Err(ContextError::ScopeMismatch.into());
    }
    let state = context_memory::rebuild(actor, scope).await?;
    let p = state
        .development_proposals
        .get(&decision.proposal_id)
        .ok_or_else(|| invalid("profile proposal unavailable"))?;
    if p.kind != DevelopmentKind::ProfileProposal {
        return Err(invalid("not a profile proposal"));
    }
    if decision.decision == ProposalDecisionKind::Accept && !enabled(&state) {
        return Err(invalid("profile collection is disabled"));
    }
    development_admission::decide(actor, scope, owner, decision).await
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
        json!({"enabled":enabled(&state),"proposals":state.development_proposals.values().filter(|p|p.kind==DevelopmentKind::ProfileProposal).collect::<Vec<_>>(),"source_count":state.sources.values().filter(|s|!s.tombstoned&&s.locator.starts_with("profile-session:")).count()}),
    )
}
