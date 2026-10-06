use crate::{
    actor::ActorEngine,
    context_memory::{self, MemoryCommand, MemoryRequest},
    development_admission,
    development_scheduler::{DevelopmentWorker, WorkerFailure},
};
use hm_context::{ContextError, Scope, development::*, maintenance::Usage};
use hm_cortex::development_historian::{HistorianExecution, HistorianFailure, HistorianProvider};
use std::{future::Future, pin::Pin, sync::atomic::Ordering, time::Duration};

#[derive(Clone)]
pub struct HistorianWorker {
    pub provider: HistorianProvider,
    pub timeout: Duration,
}
impl HistorianWorker {
    pub fn new(provider: HistorianProvider, timeout: Duration) -> Result<Self, ContextError> {
        if timeout.is_zero() || timeout > Duration::from_secs(300) {
            return Err(ContextError::Invalid("historian deadline".into()));
        }
        Ok(Self { provider, timeout })
    }
    pub async fn generate(
        &self,
        evidence: EvidenceSnapshot,
        plan_id: String,
    ) -> Result<HistorianExecution, HistorianFailure> {
        let provider = self.provider.clone();
        let task = tokio::task::spawn_blocking(move || provider.generate(evidence, plan_id));
        match tokio::time::timeout(self.timeout, task).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(HistorianFailure {
                error: "historian worker failed".into(),
                usage: Usage::Unknown,
            }),
            Err(_) => Err(HistorianFailure {
                error: "historian deadline elapsed; provider call may continue".into(),
                usage: Usage::Unknown,
            }),
        }
    }
}
impl DevelopmentWorker for HistorianWorker {
    fn kind(&self) -> DevelopmentKind {
        self.provider.kind
    }
    fn run(
        &self,
        evidence: EvidenceSnapshot,
        plan_id: String,
    ) -> Pin<Box<dyn Future<Output = Result<DevelopmentPlan, WorkerFailure>> + Send + '_>> {
        Box::pin(async move {
            self.generate(evidence, plan_id)
                .await
                .map(|result| result.plan)
                .map_err(|failure| WorkerFailure {
                    error: failure.error,
                    usage: failure.usage,
                })
        })
    }
}
#[derive(Clone, Debug)]
pub struct HistorianReceipt {
    pub publication: WorkerReceipt,
    pub usage: Usage,
    pub summary: Option<hm_context::historian::HistorianResult>,
    pub model_id: String,
}
#[derive(Debug)]
pub enum HistorianError {
    Memory(context_memory::MemoryError),
    Provider(HistorianFailure),
    Publication {
        error: context_memory::MemoryError,
        usage: Usage,
    },
}
impl std::fmt::Display for HistorianError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Memory(error) => error.fmt(f),
            Self::Provider(error) => error.fmt(f),
            Self::Publication { error, .. } => error.fmt(f),
        }
    }
}
impl std::error::Error for HistorianError {}
impl From<context_memory::MemoryError> for HistorianError {
    fn from(error: context_memory::MemoryError) -> Self {
        Self::Memory(error)
    }
}
pub async fn execute_historian(
    actor: &ActorEngine,
    scope: &Scope,
    principal: &Scope,
    worker_id: &str,
    request: SnapshotRequest,
    plan_id: String,
    worker: &HistorianWorker,
) -> Result<HistorianReceipt, HistorianError> {
    let evidence =
        development_admission::snapshot(actor, scope, principal, worker_id, request).await?;
    let output = worker
        .generate(evidence, plan_id)
        .await
        .map_err(HistorianError::Provider)?;
    if worker.provider.cancelled.load(Ordering::Acquire) {
        return Err(HistorianError::Provider(HistorianFailure {
            error: "historian cancelled before publication".into(),
            usage: output.plan.usage,
        }));
    }
    let usage = output.plan.usage.clone();
    let publication = development_admission::admit(actor, scope, principal, worker_id, output.plan)
        .await
        .map_err(|error| HistorianError::Publication {
            error,
            usage: usage.clone(),
        })?;
    Ok(HistorianReceipt {
        publication,
        usage,
        summary: output.summary,
        model_id: output.model_id,
    })
}
pub async fn cancel_attempt(
    actor: &ActorEngine,
    scope: &Scope,
    owner: &Scope,
    request_id: String,
    capability_id: String,
    expected_revision: u64,
    worker: &HistorianWorker,
) -> Result<context_memory::MemoryReceipt, context_memory::MemoryError> {
    let receipt = context_memory::execute(
        actor,
        scope,
        owner,
        MemoryRequest {
            version: 1,
            scope: scope.clone(),
            request_id,
            command: MemoryCommand::RevokeWorker {
                id: capability_id,
                expected_revision,
            },
        },
    )
    .await?;
    worker.provider.cancel();
    Ok(receipt)
}

#[derive(Clone, Debug)]
pub struct PublishedHistorian {
    pub chunk: hm_context::historian::SourceChunk,
    pub result: hm_context::historian::HistorianResult,
    pub source_cursor: hm_context::Cursor,
    pub policy_revision: u64,
    pub snapshot_digest: String,
    pub model_id: String,
}
pub fn decode_publication(
    record: &context_memory::MemoryRecord,
) -> Result<PublishedHistorian, ContextError> {
    if record.kind != context_memory::RecordKind::Summary
        || record.authority != hm_context::Authority::DerivedInference
    {
        return Err(ContextError::Invalid(
            "not a native historian summary".into(),
        ));
    }
    let publication = PublishedHistorian {
        chunk: serde_json::from_value(record.metadata["source_chunk"].clone())?,
        result: serde_json::from_value(record.metadata["historian_result"].clone())?,
        source_cursor: serde_json::from_value(record.metadata["source_cursor"].clone())?,
        policy_revision: record.metadata["policy_revision"]
            .as_u64()
            .ok_or(ContextError::Stale)?,
        snapshot_digest: record.metadata["snapshot_digest"]
            .as_str()
            .ok_or(ContextError::Stale)?
            .into(),
        model_id: record.metadata["model_id"]
            .as_str()
            .ok_or(ContextError::Stale)?
            .into(),
    };
    publication.source_cursor.validate()?;
    publication.result.validate(&publication.chunk)?;
    if record.content != publication.result.tiers[0].text {
        return Err(ContextError::Conflict);
    }
    Ok(publication)
}
