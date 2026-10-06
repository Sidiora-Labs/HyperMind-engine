use crate::{
    actor::{ActorEngine, IncomingEvent},
    context_memory::{self, MemoryError},
    development_admission,
};
use hm_context::{development::*, digest_bytes, ContextError, Scope};
use hm_core::{ConversationId, LSN};
use hm_cortex::development_documents::{
    self as documents, DocumentationPatch, RepositoryDelta, DOCUMENT_PATCH_CATEGORY,
};
use hm_llm::LlmProvider;
use hm_schema::{
    event::{self, Boundary, CURRENT_SCHEMA_VERSION},
    events::{EventEnvelope, EventPayload, ProviderFrame, Retention, Sensitivity},
};
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};
const PROVIDER: &str = "hypermind/document-apply/v1";
fn invalid() -> ContextError {
    ContextError::Conflict
}
fn file(root: &Path, relative: &str) -> Result<PathBuf, ContextError> {
    documents::validate_relative_path(relative)?;
    let root = root.canonicalize().map_err(|_| invalid())?;
    let mut path = root.clone();
    for part in Path::new(relative).components() {
        path.push(part.as_os_str());
        let meta = std::fs::symlink_metadata(&path).map_err(|_| invalid())?;
        if meta.file_type().is_symlink() {
            return Err(invalid());
        }
    }
    if !path.is_file() {
        return Err(invalid());
    }
    Ok(path)
}
fn read(root: &Path, relative: &str) -> Result<Vec<u8>, ContextError> {
    let path = file(root, relative)?;
    let meta = std::fs::metadata(&path).map_err(|_| invalid())?;
    if meta.len() > 1024 * 1024 {
        return Err(ContextError::Capacity);
    }
    std::fs::read(path).map_err(|_| invalid())
}
fn root_digest(root: &Path) -> Result<String, ContextError> {
    Ok(digest_bytes(
        root.canonicalize()
            .map_err(|_| invalid())?
            .as_os_str()
            .as_encoded_bytes(),
    ))
}
pub fn capture(
    root: &Path,
    source_path: &str,
    before_source: Vec<u8>,
    target_path: &str,
) -> Result<RepositoryDelta, ContextError> {
    let after_source = read(root, source_path)?;
    let target_bytes = read(root, target_path)?;
    let delta = RepositoryDelta {
        version: 1,
        repository_digest: root_digest(root)?,
        source_path: source_path.into(),
        before_source_digest: digest_bytes(&before_source),
        before_source,
        after_source_digest: digest_bytes(&after_source),
        after_source,
        target_path: target_path.into(),
        target_digest: digest_bytes(&target_bytes),
        target_bytes,
    };
    delta.validate()?;
    Ok(delta)
}
fn current(root: &Path, delta: &RepositoryDelta, target: &str) -> Result<(), ContextError> {
    delta.validate()?;
    if root_digest(root)? != delta.repository_digest
        || digest_bytes(&read(root, &delta.source_path)?) != delta.after_source_digest
        || digest_bytes(&read(root, &delta.target_path)?) != target
    {
        return Err(invalid());
    }
    Ok(())
}
#[derive(Clone, Serialize, Deserialize)]
pub struct DocumentRequest {
    pub plan_id: String,
    pub proposal_id: String,
    pub patch_record_id: String,
    pub source_id: String,
    pub snapshot: SnapshotRequest,
}
pub async fn propose(
    actor: &ActorEngine,
    scope: &Scope,
    caller: &Scope,
    worker: &str,
    root: &Path,
    delta: RepositoryDelta,
    request: DocumentRequest,
    provider: Arc<dyn LlmProvider>,
) -> Result<WorkerReceipt, MemoryError> {
    current(root, &delta, &delta.target_digest)?;
    let evidence =
        development_admission::snapshot(actor, scope, caller, worker, request.snapshot).await?;
    let copy = delta.clone();
    let plan = tokio::task::spawn_blocking(move || {
        documents::propose_docs(
            evidence,
            copy,
            provider.as_ref(),
            &request.plan_id,
            &request.proposal_id,
            &request.patch_record_id,
            &request.source_id,
        )
    })
    .await
    .map_err(|e| ContextError::Unavailable(e.to_string()))??;
    current(root, &delta, &delta.target_digest)?;
    development_admission::admit(actor, scope, caller, worker, plan).await
}
fn patch(proposal: &ApprovalProposal) -> Result<DocumentationPatch, ContextError> {
    if proposal.kind != DevelopmentKind::DocumentationProposal || proposal.mutations.len() != 1 {
        return Err(invalid());
    }
    let PlannedKnowledgeMutation::Create { record } = &proposal.mutations[0] else {
        return Err(invalid());
    };
    if record.category != DOCUMENT_PATCH_CATEGORY
        || record.status != DevelopmentRecordStatus::Archived
    {
        return Err(invalid());
    }
    let patch: DocumentationPatch = serde_json::from_value(record.metadata.clone())?;
    patch.validate()?;
    Ok(patch)
}
pub async fn review(
    actor: &ActorEngine,
    scope: &Scope,
    owner: &Scope,
    root: &Path,
    decision: ProposalDecision,
) -> Result<WorkerReceipt, MemoryError> {
    let state = context_memory::rebuild(actor, scope).await?;
    let proposal = state
        .development_proposals
        .get(&decision.proposal_id)
        .ok_or_else(invalid)?;
    let patch = patch(proposal)?;
    current(root, &patch.delta, &patch.delta.target_digest)?;
    development_admission::decide(actor, scope, owner, decision).await
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplyReceipt {
    pub version: u32,
    pub scope: Scope,
    pub proposal_id: String,
    pub plan_id: String,
    pub proposal_revision: u64,
    pub proposal_digest: String,
    pub patch_digest: String,
    pub target_digest: String,
    pub applied: bool,
}
async fn events(actor: &ActorEngine, scope: &Scope) -> Result<Vec<ApplyReceipt>, MemoryError> {
    let mut result = vec![];
    for frame in actor.frames_since(LSN::new(0), None, usize::MAX).await? {
        if frame.header.kind != hm_ledger::frame::EventKind::ProviderFrame {
            continue;
        }
        let verified = event::verify_event(
            &frame.sealed_payload,
            event::EventKind::ProviderFrame,
            Boundary::Disk,
        )?;
        if let EventPayload::ProviderFrame(p) = verified.envelope.payload {
            if p.provider == PROVIDER {
                let receipt: ApplyReceipt = serde_json::from_slice(&p.api_content)?;
                if receipt.scope == *scope {
                    result.push(receipt);
                }
            }
        }
    }
    Ok(result)
}
async fn append(actor: &ActorEngine, receipt: &ApplyReceipt) -> Result<(), MemoryError> {
    let tail = actor
        .frames_since(LSN::new(0), None, usize::MAX)
        .await?
        .last()
        .map(|f| f.header.lsn)
        .unwrap_or(LSN::new(0));
    let envelope = EventEnvelope {
        schema_version: CURRENT_SCHEMA_VERSION,
        payload: EventPayload::ProviderFrame(Box::new(ProviderFrame {
            provider: PROVIDER.into(),
            api_content: serde_json::to_vec(receipt)?,
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
    };
    actor
        .append_if_tail(
            tail,
            vec![IncomingEvent {
                kind: hm_ledger::frame::EventKind::ProviderFrame,
                conversation: ConversationId::derive(PROVIDER),
                payload: event::encode_event_envelope(&envelope),
            }],
        )
        .await?;
    Ok(())
}
async fn accepted(
    actor: &ActorEngine,
    scope: &Scope,
    owner: &Scope,
    id: &str,
    revision: u64,
    digest: &str,
) -> Result<(DocumentationPatch, ApplyReceipt), MemoryError> {
    if scope != owner {
        return Err(ContextError::ScopeMismatch.into());
    }
    let state = context_memory::rebuild(actor, scope).await?;
    let proposal = state.development_proposals.get(id).ok_or_else(invalid)?;
    if proposal.status != ProposalStatus::Accepted
        || proposal.revision != revision
        || proposal.digest != digest
    {
        return Err(invalid().into());
    }
    let capability = state
        .worker_capabilities
        .get(&proposal.evidence.capability_id)
        .ok_or_else(invalid)?;
    if capability.revoked
        || capability.revision != proposal.evidence.capability_revision
        || capability.digest()? != proposal.evidence.capability_digest
    {
        return Err(invalid().into());
    }
    let patch = patch(proposal)?;
    let source = state.sources.get(&patch.source_id).ok_or_else(invalid)?;
    if source.tombstoned || digest_bytes(&source.content) != patch.source_digest {
        return Err(invalid().into());
    }
    let PlannedKnowledgeMutation::Create { record } = &proposal.mutations[0] else {
        unreachable!()
    };
    let approved = state.records.get(&record.id).ok_or_else(invalid)?;
    if approved.status != context_memory::RecordStatus::Archived
        || approved.metadata != record.metadata
    {
        return Err(invalid().into());
    }
    let plan_id = state
        .development_receipts
        .values()
        .find(|receipt| receipt.proposal_id.as_deref() == Some(id))
        .ok_or_else(invalid)?
        .plan_id
        .clone();
    let receipt = ApplyReceipt {
        version: 1,
        scope: scope.clone(),
        proposal_id: id.into(),
        plan_id,
        proposal_revision: revision,
        proposal_digest: digest.into(),
        patch_digest: patch.digest.clone(),
        target_digest: patch.replacement_digest.clone(),
        applied: false,
    };
    Ok((patch, receipt))
}
// Publication is an explicit owner operation; staging never replaces the repository target.
pub async fn stage(
    actor: &ActorEngine,
    scope: &Scope,
    owner: &Scope,
    root: &Path,
    id: &str,
    revision: u64,
    digest: &str,
) -> Result<PathBuf, MemoryError> {
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let (patch, receipt) = accepted(actor, scope, owner, id, revision, digest).await?;
    current(root, &patch.delta, &patch.delta.target_digest)?;
    if events(actor, scope)
        .await?
        .iter()
        .any(|r| r.proposal_id == id)
    {
        return Err(invalid().into());
    }
    let target = file(root, &patch.delta.target_path)?;
    let staging = target.with_file_name(format!(".hypermind-document-{}.stage", patch.digest));
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&staging)
        .map_err(|_| invalid())?;
    output
        .write_all(&patch.replacement)
        .map_err(|_| invalid())?;
    output.sync_all().map_err(|_| invalid())?;
    current(root, &patch.delta, &patch.delta.target_digest)?;
    append(actor, &receipt).await?;
    Ok(staging)
}
// The owner applies the staged file through their editor, then requests digest-verified acknowledgement.
pub async fn acknowledge_applied(
    actor: &ActorEngine,
    scope: &Scope,
    owner: &Scope,
    root: &Path,
    id: &str,
    revision: u64,
    digest: &str,
) -> Result<ApplyReceipt, MemoryError> {
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let (patch, mut receipt) = accepted(actor, scope, owner, id, revision, digest).await?;
    current(root, &patch.delta, &patch.replacement_digest)?;
    let prior = events(actor, scope).await?;
    let intent = prior
        .iter()
        .find(|r| r.proposal_id == id && r.patch_digest == patch.digest)
        .ok_or_else(invalid)?;
    if intent.proposal_digest != digest {
        return Err(invalid().into());
    }
    if let Some(done) = prior.iter().find(|r| r.proposal_id == id && r.applied) {
        return Ok(done.clone());
    }
    receipt.applied = true;
    append(actor, &receipt).await?;
    Ok(receipt)
}
