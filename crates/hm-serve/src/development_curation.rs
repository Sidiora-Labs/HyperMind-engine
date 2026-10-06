use crate::{actor::ActorEngine, context_memory::MemoryError, development_admission};
use hm_context::{
    Scope,
    development::{DevelopmentPlan, EvidenceSnapshot, WorkerReceipt},
};
use hm_cortex::development_curation::{CurationError, CurationRequest};
use hm_llm::LlmProvider;
use std::sync::Arc;

#[derive(Debug)]
pub enum Error {
    Memory(MemoryError),
    Ledger(hm_core::Error),
    Curation(CurationError),
    Executor(String),
    Boundary,
}
/// A publication is consumed by a subsequent assembly; an existing frozen prompt is never rewritten.
#[derive(Clone, Debug)]
pub struct ApplyBoundary {
    pub session_id: String,
    pub generation: Option<u64>,
    pub materialization_digest: Option<String>,
}
pub async fn infer(
    snapshot: EvidenceSnapshot,
    provider: Arc<dyn LlmProvider>,
    request: CurationRequest,
) -> Result<DevelopmentPlan, Error> {
    tokio::task::spawn_blocking(move || {
        hm_cortex::development_curation::curate(snapshot, provider.as_ref(), request)
    })
    .await
    .map_err(|e| Error::Executor(e.to_string()))?
    .map_err(Error::Curation)
}
pub async fn apply(
    actor: &ActorEngine,
    scope: &Scope,
    principal: &Scope,
    worker_id: &str,
    plan: DevelopmentPlan,
    boundary: &ApplyBoundary,
) -> Result<WorkerReceipt, Error> {
    if boundary.session_id != plan.evidence.session_id {
        return Err(Error::Boundary);
    }
    let current = crate::context_projection::current(actor, scope, &boundary.session_id)
        .await
        .map_err(Error::Ledger)?;
    if current.as_ref().map(|v| v.generation) != boundary.generation
        || current.as_ref().map(|v| hm_context::digest_bytes(&v.bytes))
            != boundary.materialization_digest
    {
        return Err(Error::Boundary);
    }
    development_admission::admit(actor, scope, principal, worker_id, plan)
        .await
        .map_err(Error::Memory)
}
