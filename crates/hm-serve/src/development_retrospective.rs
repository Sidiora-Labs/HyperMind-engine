use crate::{
    actor::ActorEngine,
    context_memory::{self, MemoryError},
    development_admission,
};
use hm_context::{
    development::{SnapshotRequest, WorkerReceipt},
    history::SourceRelation,
    validate_id, Authority, ContextError, MessageRole, Scope,
};
use hm_cortex::development_retrospective::{
    self as retrospective, CorrectionSignal, RetrospectiveProposal, RetrospectiveWatermark,
    SignalBatch, CHECKPOINT_CATEGORY, LESSON_CATEGORY,
};
use hm_llm::LlmProvider;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetrospectiveRequest {
    pub plan_id: String,
    pub checkpoint_id: String,
    pub lesson_ids: Vec<String>,
    pub snapshot: SnapshotRequest,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case", deny_unknown_fields)]
pub enum RetrospectiveOutcome {
    NoSignal {
        watermark: RetrospectiveWatermark,
        provider_calls: u32,
    },
    Published {
        receipt: WorkerReceipt,
        watermark: RetrospectiveWatermark,
        provider_calls: u32,
    },
}
pub async fn prepare(
    actor: &ActorEngine,
    scope: &Scope,
    caller: &Scope,
    worker: &str,
    request: &RetrospectiveRequest,
) -> Result<SignalBatch, MemoryError> {
    validate_id(&request.plan_id)?;
    validate_id(&request.checkpoint_id)?;
    let state = context_memory::rebuild(actor, scope).await?;
    let checkpoint = state.records.get(&request.checkpoint_id);
    if checkpoint.is_some() && !request.snapshot.record_ids.contains(&request.checkpoint_id) {
        return Err(ContextError::ScopeMismatch.into());
    }
    if state.records.values().any(|record| {
        record.category == LESSON_CATEGORY && !request.snapshot.record_ids.contains(&record.id)
    }) {
        return Err(ContextError::ScopeMismatch.into());
    }
    let evidence =
        development_admission::snapshot(actor, scope, caller, worker, request.snapshot.clone())
            .await?;
    let history =
        crate::context_history::replay(actor, scope, &evidence.session_id, &evidence.conversation)
            .await
            .map_err(|error| match error {
                crate::context_history::HistoryError::Context(e) => MemoryError::Context(e),
                crate::context_history::HistoryError::Ledger(e) => MemoryError::Ledger(e),
            })?;
    let mut signals = Vec::new();
    for relation in history.history.relations() {
        if let SourceRelation::Edit {
            id, replacement_id, ..
        } = relation
        {
            let message = history.history.message(replacement_id)?;
            if message.role == MessageRole::User
                && message.authority == Authority::UserAsserted
                && request.snapshot.source_ids.contains(replacement_id)
                && history
                    .history
                    .visible_messages()
                    .iter()
                    .any(|source| source.id == *replacement_id)
            {
                signals.push(CorrectionSignal {
                    relation_id: id.clone(),
                    source_id: replacement_id.clone(),
                    source_digest: message.source_digest.clone(),
                });
            }
        }
    }
    let watermark = if let Some(record) = evidence
        .records
        .iter()
        .find(|r| r.id == request.checkpoint_id)
    {
        if record.category != CHECKPOINT_CATEGORY
            || record.status != hm_context::development::DevelopmentRecordStatus::Archived
        {
            return Err(ContextError::Conflict.into());
        }
        serde_json::from_value(record.metadata.clone())?
    } else {
        RetrospectiveWatermark::default()
    };
    Ok(retrospective::detect_new_corrections(
        evidence, signals, watermark,
    )?)
}
pub async fn propose(
    actor: &ActorEngine,
    scope: &Scope,
    caller: &Scope,
    worker: &str,
    request: RetrospectiveRequest,
    provider: Arc<dyn LlmProvider>,
) -> Result<RetrospectiveProposal, MemoryError> {
    let batch = prepare(actor, scope, caller, worker, &request).await?;
    tokio::task::spawn_blocking(move || {
        retrospective::propose_lessons(
            batch,
            provider.as_ref(),
            &request.plan_id,
            &request.checkpoint_id,
            &request.lesson_ids,
        )
    })
    .await
    .map_err(|e| ContextError::Unavailable(format!("retrospective worker: {e}")))?
    .map_err(MemoryError::from)
}
pub async fn publish(
    actor: &ActorEngine,
    scope: &Scope,
    caller: &Scope,
    worker: &str,
    proposal: RetrospectiveProposal,
) -> Result<RetrospectiveOutcome, MemoryError> {
    match proposal {
        RetrospectiveProposal::NoSignal {
            watermark,
            provider_calls,
        } => Ok(RetrospectiveOutcome::NoSignal {
            watermark,
            provider_calls,
        }),
        RetrospectiveProposal::Proposed {
            plan,
            watermark,
            provider_calls,
        } => {
            if plan.kind != hm_context::development::DevelopmentKind::Retrospective {
                return Err(ContextError::ScopeMismatch.into());
            }
            let checkpoints: Vec<_> = plan
                .mutations
                .iter()
                .filter_map(|mutation| match mutation {
                    hm_context::development::PlannedKnowledgeMutation::Create { record }
                    | hm_context::development::PlannedKnowledgeMutation::Revise {
                        record, ..
                    } if record.category == CHECKPOINT_CATEGORY => Some(record),
                    _ => None,
                })
                .collect();
            if checkpoints.len() != 1
                || checkpoints[0].status
                    != hm_context::development::DevelopmentRecordStatus::Archived
                || serde_json::from_value::<RetrospectiveWatermark>(
                    checkpoints[0].metadata.clone(),
                )? != watermark
            {
                return Err(ContextError::Conflict.into());
            }
            let receipt = development_admission::admit(actor, scope, caller, worker, plan).await?;
            Ok(RetrospectiveOutcome::Published {
                receipt,
                watermark,
                provider_calls,
            })
        }
    }
}
pub async fn run(
    actor: &ActorEngine,
    scope: &Scope,
    caller: &Scope,
    worker: &str,
    request: RetrospectiveRequest,
    provider: Arc<dyn LlmProvider>,
) -> Result<RetrospectiveOutcome, MemoryError> {
    let proposal = propose(actor, scope, caller, worker, request, provider).await?;
    publish(actor, scope, caller, worker, proposal).await
}
