use crate::{
    actor::ActorEngine,
    context_memory::{
        self, MemoryCommand, MemoryError, MemoryProjection, MemoryRequest, RecordKind, RecordStatus,
    },
};
use hm_context::{Authority, ContextError, Scope, development::*, digest_bytes, validate_id};
use hm_core::LSN;
use std::{
    collections::BTreeSet,
    time::{SystemTime, UNIX_EPOCH},
};

fn now_ns() -> Result<i64, MemoryError> {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ContextError::Unavailable("clock".into()))?
            .as_nanos(),
    )
    .map_err(|_| ContextError::Capacity.into())
}
fn invalid(message: &str) -> MemoryError {
    ContextError::Invalid(message.into()).into()
}
fn native_record(record: &DevelopmentRecord) -> Result<context_memory::MemoryRecord, MemoryError> {
    Ok(serde_json::from_value(serde_json::to_value(record)?)?)
}
fn portable_record(
    record: &context_memory::MemoryRecord,
) -> Result<DevelopmentRecord, MemoryError> {
    Ok(serde_json::from_value(serde_json::to_value(record)?)?)
}
fn capability<'a>(
    state: &'a MemoryProjection,
    caller: &Scope,
    worker: &str,
    id: &str,
    check_expiry: bool,
) -> Result<&'a WorkerCapability, MemoryError> {
    let capability = state
        .worker_capabilities
        .get(id)
        .ok_or_else(|| ContextError::Unavailable("worker capability".into()))?;
    capability.validate()?;
    if capability.revoked
        || &capability.principal != caller
        || capability.worker_id != worker
        || capability.scope != state.scope
    {
        return Err(ContextError::ScopeMismatch.into());
    }
    if check_expiry && now_ns()? >= capability.lease.expires_at_ns {
        return Err(ContextError::Stale.into());
    }
    Ok(capability)
}
async fn capture(
    actor: &ActorEngine,
    scope: &Scope,
    caller: &Scope,
    worker: &str,
    request: &SnapshotRequest,
    state: &MemoryProjection,
    tail: LSN,
    check_expiry: bool,
) -> Result<EvidenceSnapshot, MemoryError> {
    let cap = capability(state, caller, worker, &request.capability_id, check_expiry)?;
    if !request.source_ids.is_subset(&cap.source_ids)
        || !request.record_ids.is_subset(&cap.record_ids)
        || request.source_ids.len() > 256
        || request.record_ids.len() > 256
    {
        return Err(ContextError::ScopeMismatch.into());
    }
    let current = crate::context_history::replay(actor, scope, &cap.session_id, &cap.conversation)
        .await
        .map_err(|error| match error {
            crate::context_history::HistoryError::Context(e) => MemoryError::Context(e),
            crate::context_history::HistoryError::Ledger(e) => MemoryError::Ledger(e),
        })?;
    let mut sources = Vec::new();
    for id in &request.source_ids {
        if let Some(source) = state.sources.get(id).filter(|_| {
            !current
                .history
                .visible_messages()
                .iter()
                .any(|message| &message.id == id)
        }) {
            if source.tombstoned {
                return Err(ContextError::Stale.into());
            }
            sources.push(EvidenceSource {
                id: id.clone(),
                origin: EvidenceOrigin::Memory,
                revision: 1,
                digest: source.digest.clone(),
                content_digest: source.digest.clone(),
                authority: Authority::ExternalObserved,
                content: source.content.clone(),
                spans: vec![],
                occurred_at_ns: source.occurred_at_ns,
                recorded_at_ns: source.recorded_at_ns,
            });
        } else {
            let visible = current.history.visible_messages();
            let source = visible
                .into_iter()
                .find(|source| &source.id == id)
                .ok_or_else(|| ContextError::Unavailable("evidence source".into()))?;
            if !matches!(
                source.authority,
                Authority::UserAsserted
                    | Authority::ExternalObserved
                    | Authority::ToolObserved
                    | Authority::RuntimeFact
            ) {
                return Err(invalid("generated output is not observation evidence"));
            }
            let span = current.history.source_span(id)?;
            let content = current.history.recover(scope, &span)?;
            sources.push(EvidenceSource {
                id: id.clone(),
                origin: EvidenceOrigin::Conversation,
                revision: source.ordinal,
                digest: source.source_digest.clone(),
                content_digest: digest_bytes(&content),
                authority: source.authority,
                content,
                spans: vec![span],
                occurred_at_ns: source.occurred_at_ns,
                recorded_at_ns: source.recorded_at_ns,
            });
        }
    }
    let mut records = Vec::new();
    let now = now_ns()?;
    for id in &request.record_ids {
        let record = state
            .read(caller, id, now)?
            .ok_or_else(|| ContextError::Unavailable("evidence record".into()))?;
        records.push(portable_record(record)?);
    }
    let input_size = sources
        .iter()
        .map(|source| source.content.len())
        .sum::<usize>()
        .checked_add(
            records
                .iter()
                .map(|record| record.content.len())
                .sum::<usize>(),
        )
        .ok_or(ContextError::Capacity)?;
    if input_size as u64 > cap.budget.max_input_bytes {
        return Err(ContextError::Capacity.into());
    }
    let grants: Vec<_> = state
        .grants
        .values()
        .filter(|grant| {
            (grant.principal == *caller
                || grant
                    .principal_digest
                    .as_ref()
                    .is_some_and(|digest| digest == &context_memory::principal_digest(caller)))
                && (!grant.record_ids.is_disjoint(&request.record_ids)
                    || records
                        .iter()
                        .any(|record| grant.categories.contains(&record.category)))
        })
        .collect();
    let job_state = crate::context_jobs::inspect_state(actor, scope, &scope.owner_id).await?;
    let policy_revision = job_state["policies"][&cap.session_id].as_u64().unwrap_or(1);
    let mut evidence = EvidenceSnapshot {
        version: 1,
        scope: scope.clone(),
        principal: caller.clone(),
        worker_id: worker.into(),
        capability_id: cap.id.clone(),
        capability_revision: cap.revision,
        capability_digest: cap.digest()?,
        session_id: cap.session_id.clone(),
        conversation: cap.conversation.clone(),
        ledger_tail: tail.get(),
        memory_cursor: state.cursor,
        source_cursor: current.history.cursor(),
        policy_revision,
        grant_digest: digest_bytes(&serde_json::to_vec(&grants)?),
        sources,
        records,
        lease: cap.lease.clone(),
        budget: cap.budget.clone(),
        digest: String::new(),
    };
    evidence.digest = evidence.computed_digest()?;
    Ok(evidence)
}
pub async fn snapshot(
    actor: &ActorEngine,
    trusted_scope: &Scope,
    caller: &Scope,
    worker_id: &str,
    request: SnapshotRequest,
) -> Result<EvidenceSnapshot, MemoryError> {
    trusted_scope.validate()?;
    caller.validate()?;
    validate_id(worker_id)?;
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let tail = actor.stats().await?.applied.last_lsn;
    let state = context_memory::rebuild(actor, trusted_scope).await?;
    let evidence = capture(
        actor,
        trusted_scope,
        caller,
        worker_id,
        &request,
        &state,
        tail,
        true,
    )
    .await?;
    crate::context_projection::validate_tail(actor, tail).await?;
    Ok(evidence)
}
async fn validate_evidence(
    actor: &ActorEngine,
    scope: &Scope,
    caller: &Scope,
    worker: &str,
    evidence: &EvidenceSnapshot,
    state: &MemoryProjection,
    tail: LSN,
    check_expiry: bool,
) -> Result<(), MemoryError> {
    if evidence.version != 1
        || evidence.scope != *scope
        || evidence.principal != *caller
        || evidence.worker_id != worker
        || evidence.digest != evidence.computed_digest()?
    {
        return Err(ContextError::ScopeMismatch.into());
    }
    let source_ids: BTreeSet<_> = evidence
        .sources
        .iter()
        .map(|source| source.id.clone())
        .collect();
    let record_ids: BTreeSet<_> = evidence
        .records
        .iter()
        .map(|record| record.id.clone())
        .collect();
    if source_ids.len() != evidence.sources.len() || record_ids.len() != evidence.records.len() {
        return Err(ContextError::Conflict.into());
    }
    let request = SnapshotRequest {
        capability_id: evidence.capability_id.clone(),
        source_ids,
        record_ids,
    };
    let mut current = capture(
        actor,
        scope,
        caller,
        worker,
        &request,
        state,
        tail,
        check_expiry,
    )
    .await?;
    // Ledger and projection counters are diagnostic; publication fences cover the selected evidence.
    current.ledger_tail = evidence.ledger_tail;
    current.memory_cursor = evidence.memory_cursor;
    current.source_cursor = evidence.source_cursor;
    current.digest = current.computed_digest()?;
    if current != *evidence {
        return Err(ContextError::Stale.into());
    }
    Ok(())
}
fn protected(record: &context_memory::MemoryRecord) -> bool {
    record.kind == RecordKind::Anchor || record.pinned || !record.contradictions.is_empty()
}
fn expected_record<'a>(
    state: &'a MemoryProjection,
    evidence: &EvidenceSnapshot,
    cap: &WorkerCapability,
    id: &str,
    revision: u64,
    digest: &str,
) -> Result<&'a context_memory::MemoryRecord, MemoryError> {
    if !cap.record_ids.contains(id)
        || !evidence.records.iter().any(|record| {
            record.id == id && record.revision == revision && record.revision_digest == digest
        })
    {
        return Err(ContextError::ScopeMismatch.into());
    }
    let old = state
        .records
        .get(id)
        .ok_or_else(|| ContextError::Unavailable("record".into()))?;
    if old.revision != revision
        || old.revision_digest != digest
        || old.status == RecordStatus::Tombstoned
    {
        return Err(ContextError::Stale.into());
    }
    Ok(old)
}
fn translate(
    state: &MemoryProjection,
    cap: &WorkerCapability,
    evidence: &EvidenceSnapshot,
    mutations: &[PlannedKnowledgeMutation],
) -> Result<Vec<MemoryCommand>, MemoryError> {
    if mutations.len() > cap.budget.max_mutations
        || serde_json::to_vec(mutations)?.len() as u64 > cap.budget.max_output_bytes
    {
        return Err(ContextError::Capacity.into());
    }
    let mut commands = Vec::new();
    let mut writes = BTreeSet::new();
    let mut needed_sources = BTreeSet::new();
    for mutation in mutations {
        match mutation {
            PlannedKnowledgeMutation::Create { record }
            | PlannedKnowledgeMutation::Revise { record, .. } => {
                needed_sources.extend(record.provenance.iter().map(|p| p.source_id.clone()));
            }
            PlannedKnowledgeMutation::Verify { verification } => {
                context_memory::validate_verification_evidence(&verification.metadata, evidence)?;
                if let Some(id) = &verification.evidence_source_id {
                    needed_sources.insert(id.clone());
                }
            }
            _ => {}
        }
    }
    for source in &evidence.sources {
        if needed_sources.contains(&source.id) && source.origin == EvidenceOrigin::Conversation {
            if let Some(old) = state.sources.get(&source.id) {
                if old.tombstoned
                    || old.digest != source.content_digest
                    || old.content != source.content
                {
                    return Err(ContextError::Stale.into());
                }
            } else {
                commands.push(MemoryCommand::Source {
                    source: context_memory::MemorySource {
                        id: source.id.clone(),
                        digest: source.content_digest.clone(),
                        content: source.content.clone(),
                        locator: format!("conversation:{}:{}", evidence.session_id, source.id),
                        occurred_at_ns: source.occurred_at_ns,
                        recorded_at_ns: source.recorded_at_ns,
                        tombstoned: false,
                    },
                });
            }
        }
    }
    for mutation in mutations {
        let command = match mutation {
            PlannedKnowledgeMutation::Create { record } => {
                if !cap.new_record_ids.contains(&record.id)
                    || state.records.contains_key(&record.id)
                    || !writes.insert(record.id.clone())
                {
                    return Err(ContextError::ScopeMismatch.into());
                }
                let mut record = native_record(record)?;
                validate_generated_record(&record, evidence)?;
                if record.kind == RecordKind::Anchor || record.pinned {
                    return Err(invalid("workers cannot create protected records"));
                }
                if record.revision_digest.is_empty() {
                    record.revision_digest = record.computed_revision_digest()?;
                }
                MemoryCommand::Create { record }
            }
            PlannedKnowledgeMutation::Revise {
                record,
                expected_revision,
                expected_digest,
            } => {
                let old = expected_record(
                    state,
                    evidence,
                    cap,
                    &record.id,
                    *expected_revision,
                    expected_digest,
                )?;
                if protected(old) || !writes.insert(record.id.clone()) {
                    return Err(invalid("protected or repeated record revision"));
                }
                let mut record = native_record(record)?;
                validate_generated_record(&record, evidence)?;
                if record.pinned
                    || record.kind == RecordKind::Anchor
                    || !old
                        .contradictions
                        .iter()
                        .all(|id| record.contradictions.contains(id))
                {
                    return Err(invalid("protected record transition"));
                }
                if record.revision_digest.is_empty() {
                    record.revision_digest = record.computed_revision_digest()?;
                }
                MemoryCommand::Revise {
                    record,
                    expected_revision: *expected_revision,
                }
            }
            PlannedKnowledgeMutation::SetStatus {
                id,
                status,
                expected_revision,
                expected_digest,
            } => {
                let old = expected_record(
                    state,
                    evidence,
                    cap,
                    id,
                    *expected_revision,
                    expected_digest,
                )?;
                if protected(old)
                    || !writes.insert(id.clone())
                    || matches!(status, DevelopmentRecordStatus::Tombstoned)
                {
                    return Err(invalid("protected record status transition"));
                }
                MemoryCommand::SetStatus {
                    id: id.clone(),
                    status: serde_json::from_value(serde_json::to_value(status)?)?,
                    expected_revision: *expected_revision,
                }
            }
            PlannedKnowledgeMutation::Verify { verification } => {
                let snap = evidence
                    .records
                    .iter()
                    .find(|record| record.id == verification.record_id)
                    .ok_or(ContextError::ScopeMismatch)?;
                expected_record(
                    state,
                    evidence,
                    cap,
                    &verification.record_id,
                    snap.revision,
                    &verification.revision_digest,
                )?;
                let source = verification
                    .evidence_source_id
                    .as_ref()
                    .ok_or_else(|| invalid("verification needs original evidence"))?;
                if !evidence.sources.iter().any(|e| &e.id == source) {
                    return Err(ContextError::ScopeMismatch.into());
                }
                MemoryCommand::Verify {
                    verification: serde_json::from_value(serde_json::to_value(verification)?)?,
                }
            }
            PlannedKnowledgeMutation::AddLineage { lineage } => {
                let snap = evidence
                    .records
                    .iter()
                    .find(|record| record.id == lineage.child_record_id)
                    .ok_or(ContextError::ScopeMismatch)?;
                let old = expected_record(
                    state,
                    evidence,
                    cap,
                    &lineage.child_record_id,
                    lineage.child_revision,
                    &snap.revision_digest,
                )?;
                if protected(old)
                    || !cap.record_ids.contains(&lineage.parent_record_id)
                    || !evidence.records.iter().any(|record| {
                        record.id == lineage.parent_record_id
                            && record.revision_digest == lineage.parent_revision_digest
                    })
                {
                    return Err(ContextError::ScopeMismatch.into());
                }
                MemoryCommand::Lineage {
                    lineage: serde_json::from_value(serde_json::to_value(lineage)?)?,
                }
            }
        };
        commands.push(command);
    }
    Ok(commands)
}
fn validate_generated_record(
    record: &context_memory::MemoryRecord,
    evidence: &EvidenceSnapshot,
) -> Result<(), MemoryError> {
    if !matches!(
        record.authority,
        Authority::AssistantGenerated | Authority::DerivedInference
    ) || record.provenance.is_empty()
    {
        return Err(invalid(
            "generated record needs original evidence and generated authority",
        ));
    }
    for provenance in &record.provenance {
        if !evidence.sources.iter().any(|source| {
            source.id == provenance.source_id && source.content_digest == provenance.source_digest
        }) {
            return Err(ContextError::ScopeMismatch.into());
        }
    }
    for parent in record
        .lineage
        .iter()
        .map(|lineage| &lineage.parent_record_id)
        .chain(&record.contradictions)
    {
        if !evidence.records.iter().any(|record| &record.id == parent) {
            return Err(ContextError::ScopeMismatch.into());
        }
    }
    Ok(())
}
pub async fn admit(
    actor: &ActorEngine,
    trusted_scope: &Scope,
    caller: &Scope,
    worker_id: &str,
    plan: DevelopmentPlan,
) -> Result<WorkerReceipt, MemoryError> {
    trusted_scope.validate()?;
    caller.validate()?;
    validate_id(worker_id)?;
    validate_id(&plan.id)?;
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let tail = actor.stats().await?.applied.last_lsn;
    let state = context_memory::rebuild(actor, trusted_scope).await?;
    let cap = capability(
        &state,
        caller,
        worker_id,
        &plan.evidence.capability_id,
        true,
    )?;
    let digest = plan.digest()?;
    if let Some(receipt) = state.development_receipts.get(&plan.id) {
        if receipt.plan_digest != digest {
            return Err(ContextError::Conflict.into());
        }
        let mut receipt = receipt.clone();
        receipt.replayed = true;
        return Ok(receipt);
    }
    if plan.version != 1
        || !cap.allowed_kinds.contains(&plan.kind)
        || state
            .worker_attempts
            .contains(&(cap.id.clone(), cap.revision, cap.lease.attempt))
    {
        return Err(ContextError::ScopeMismatch.into());
    }
    if let hm_context::maintenance::Usage::Known(tokens) = plan.usage {
        if tokens > cap.budget.reserved_tokens {
            return Err(ContextError::Capacity.into());
        }
    }
    validate_evidence(
        actor,
        trusted_scope,
        caller,
        worker_id,
        &plan.evidence,
        &state,
        tail,
        true,
    )
    .await?;
    let commands = translate(&state, cap, &plan.evidence, &plan.mutations)?;
    let proposal = if let Some(mut proposal) = plan.proposal.clone() {
        if !matches!(
            plan.kind,
            DevelopmentKind::ProfileProposal | DevelopmentKind::DocumentationProposal
        ) || !plan.mutations.is_empty()
            || proposal.kind != plan.kind
            || proposal.scope != *trusted_scope
            || proposal.worker_id != worker_id
            || proposal.revision != 1
            || proposal.status != ProposalStatus::Pending
            || proposal.evidence != plan.evidence
            || proposal.mutations.is_empty()
        {
            return Err(invalid("invalid owner approval proposal"));
        }
        validate_id(&proposal.id)?;
        translate(&state, cap, &plan.evidence, &proposal.mutations)?;
        if proposal.digest.is_empty() {
            proposal.digest = proposal.computed_digest()?;
        }
        if proposal.digest != proposal.computed_digest()? {
            return Err(ContextError::Conflict.into());
        }
        Some(proposal)
    } else {
        if matches!(
            plan.kind,
            DevelopmentKind::ProfileProposal | DevelopmentKind::DocumentationProposal
        ) || commands.is_empty()
        {
            return Err(invalid("development plan has no admissible publication"));
        }
        None
    };
    context_memory::execute_fenced_locked(
        actor,
        trusted_scope,
        trusted_scope,
        MemoryRequest {
            version: 1,
            scope: trusted_scope.clone(),
            request_id: plan.id.clone(),
            command: MemoryCommand::DevelopmentCommit {
                capability_id: cap.id.clone(),
                capability_revision: cap.revision,
                lease_attempt: cap.lease.attempt,
                plan_id: plan.id.clone(),
                plan_digest: digest,
                snapshot_digest: plan.evidence.digest,
                commands,
                proposal,
            },
        },
        Some(tail),
    )
    .await?;
    context_memory::rebuild(actor, trusted_scope)
        .await?
        .development_receipts
        .remove(&plan.id)
        .ok_or_else(|| ContextError::Unavailable("development receipt".into()).into())
}
pub async fn decide(
    actor: &ActorEngine,
    trusted_scope: &Scope,
    owner: &Scope,
    decision: ProposalDecision,
) -> Result<WorkerReceipt, MemoryError> {
    if owner != trusted_scope {
        return Err(ContextError::ScopeMismatch.into());
    }
    validate_id(&decision.request_id)?;
    validate_id(&decision.proposal_id)?;
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let tail = actor.stats().await?.applied.last_lsn;
    let state = context_memory::rebuild(actor, trusted_scope).await?;
    let digest = digest_bytes(&serde_json::to_vec(&decision)?);
    if let Some(receipt) = state.development_receipts.get(&decision.request_id) {
        if receipt.plan_digest != digest {
            return Err(ContextError::Conflict.into());
        }
        let mut receipt = receipt.clone();
        receipt.replayed = true;
        return Ok(receipt);
    }
    let proposal = state
        .development_proposals
        .get(&decision.proposal_id)
        .ok_or_else(|| ContextError::Unavailable("proposal".into()))?;
    if proposal.revision != decision.expected_revision
        || proposal.digest != decision.expected_digest
        || proposal.status != ProposalStatus::Pending
    {
        return Err(ContextError::Stale.into());
    }
    let commands = if decision.decision == ProposalDecisionKind::Accept {
        validate_evidence(
            actor,
            trusted_scope,
            &proposal.evidence.principal,
            &proposal.worker_id,
            &proposal.evidence,
            &state,
            tail,
            false,
        )
        .await?;
        let cap = capability(
            &state,
            &proposal.evidence.principal,
            &proposal.worker_id,
            &proposal.evidence.capability_id,
            false,
        )?;
        translate(&state, cap, &proposal.evidence, &proposal.mutations)?
    } else {
        vec![]
    };
    context_memory::execute_fenced_locked(
        actor,
        trusted_scope,
        owner,
        MemoryRequest {
            version: 1,
            scope: trusted_scope.clone(),
            request_id: decision.request_id.clone(),
            command: MemoryCommand::DecideDevelopmentProposal {
                decision: decision.clone(),
                commands,
            },
        },
        Some(tail),
    )
    .await?;
    context_memory::rebuild(actor, trusted_scope)
        .await?
        .development_receipts
        .remove(&decision.request_id)
        .ok_or_else(|| ContextError::Unavailable("proposal decision receipt".into()).into())
}
