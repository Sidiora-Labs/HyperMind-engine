use crate::{
    actor::ActorEngine,
    context_memory::{self, MemoryCommand, MemoryError, MemoryGrant, MemoryRequest},
    development_admission,
    development_scheduler::{
        self, DevelopmentWorker, SchedulerAction, SchedulerRequest, WorkerRegistry,
    },
};
use hm_context::{
    ContextError, Scope, development::*, development_schedule::*, digest_bytes, validate_id,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceSchedule {
    pub id: String,
    pub mode: ScheduleMode,
    pub worker_id: String,
    pub snapshot: SnapshotRequest,
    pub reservation: u64,
    pub timeout_ms: u64,
    pub backoff_ms: u64,
    pub max_attempts: u64,
    pub identical_failure_limit: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum DevelopmentAction {
    Register {
        worker_id: String,
        capability_id: String,
        session_id: String,
        conversation: String,
        source_ids: BTreeSet<String>,
        record_ids: BTreeSet<String>,
        new_record_ids: BTreeSet<String>,
        budget: DevelopmentBudget,
        lease_ms: u64,
    },
    Revoke {
        capability_id: String,
        expected_revision: u64,
    },
    Configure {
        schedule: ServiceSchedule,
    },
    Enqueue {
        schedule_id: String,
    },
    Dispatch {
        job_id: String,
    },
    Review {
        decision: ProposalDecision,
    },
    Cancel {
        job_id: String,
    },
    Inspect,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DevelopmentRequest {
    pub version: u32,
    pub scope: Scope,
    pub request_id: String,
    pub action: DevelopmentAction,
}
pub struct DevelopmentService {
    scope: Scope,
    registry: WorkerRegistry,
    workers: BTreeMap<String, (DevelopmentKind, Option<String>)>,
}
impl DevelopmentService {
    pub fn new(
        scope: Scope,
        workers: Vec<(String, Arc<dyn DevelopmentWorker>)>,
    ) -> Result<Self, MemoryError> {
        scope.validate()?;
        let mut service = Self {
            scope,
            registry: WorkerRegistry::default(),
            workers: BTreeMap::new(),
        };
        for (id, worker) in workers {
            service.register_worker(id, worker)?;
        }
        Ok(service)
    }
    pub fn register_worker(
        &mut self,
        id: String,
        worker: Arc<dyn DevelopmentWorker>,
    ) -> Result<(), MemoryError> {
        let metadata = (worker.kind(), worker.provider_policy().map(str::to_owned));
        self.registry.register(id.clone(), worker)?;
        self.workers.insert(id, metadata);
        Ok(())
    }
    fn authorize(&self, owner: &Scope) -> Result<(), MemoryError> {
        if owner != &self.scope {
            return Err(ContextError::ScopeMismatch.into());
        }
        Ok(())
    }
    fn principal(&self, worker: &str) -> Result<Scope, MemoryError> {
        if !self.workers.contains_key(worker) {
            return Err(ContextError::Unavailable("registered development worker".into()).into());
        }
        Ok(Scope {
            owner_id: format!(
                "worker:{}",
                digest_bytes(&serde_json::to_vec(&(&self.scope, worker))?)
            ),
            project_id: self.scope.project_id.clone(),
            workspace_id: self.scope.workspace_id.clone(),
        })
    }
    pub async fn inspect(&self, actor: &ActorEngine, owner: &Scope) -> Result<Value, MemoryError> {
        self.authorize(owner)?;
        let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
        let tail = actor.stats().await?.applied.last_lsn;
        let state = context_memory::rebuild(actor, &self.scope).await?;
        let scheduler = development_scheduler::inspect(actor, &self.scope).await?;
        let workers: Vec<_> = self.workers.iter().map(|(id,(kind,policy))|json!({"id":id,"kind":kind,"provider_policy":policy,"registered":true})).collect();
        let unavailable: BTreeMap<_, _> = state
            .worker_capabilities
            .iter()
            .filter(|(_, cap)| !self.workers.contains_key(&cap.worker_id))
            .map(|(id, _)| (id.clone(), "runtime worker is not registered"))
            .collect();
        crate::context_projection::validate_tail(actor, tail).await?;
        Ok(
            json!({"version":1,"scope":self.scope,"runtime_workers":workers,"worker_unavailability":unavailable,"capabilities":state.worker_capabilities,"proposals":state.development_proposals,"receipts":state.development_receipts,"scheduler":scheduler}),
        )
    }
    async fn scheduler(
        &self,
        actor: &ActorEngine,
        request_id: String,
        action: SchedulerAction,
    ) -> Result<Value, MemoryError> {
        development_scheduler::execute(
            actor,
            &self.scope,
            &self.scope,
            SchedulerRequest {
                version: 1,
                scope: self.scope.clone(),
                request_id,
                action,
            },
        )
        .await
    }
    pub async fn execute(
        &self,
        actor: &ActorEngine,
        owner: &Scope,
        request: DevelopmentRequest,
    ) -> Result<Value, MemoryError> {
        self.authorize(owner)?;
        if request.version != 1 {
            return Err(MemoryError::Ledger(hm_core::Error::new(
                hm_core::ErrorCode::ProtocolVersion,
            )));
        }
        if request.scope != self.scope {
            return Err(ContextError::ScopeMismatch.into());
        }
        validate_id(&request.request_id)?;
        let request_digest = digest_bytes(&serde_json::to_vec(&request)?);
        match request.action {
            DevelopmentAction::Inspect => self.inspect(actor, owner).await,
            DevelopmentAction::Register {
                worker_id,
                capability_id,
                session_id,
                conversation,
                source_ids,
                record_ids,
                new_record_ids,
                budget,
                lease_ms,
            } => {
                if lease_ms == 0 || lease_ms > 3_600_000 {
                    return Err(ContextError::Invalid("worker lease bounds".into()).into());
                }
                let principal = self.principal(&worker_id)?;
                let kind = self.workers[&worker_id].0;
                let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
                let tail = actor.stats().await?.applied.last_lsn;
                let memory = context_memory::rebuild(actor, &self.scope).await?;
                let now = crate::session_context::runtime_now_ns().map_err(MemoryError::Ledger)?;
                let expires_at_ns = now
                    .checked_add((lease_ms as i64) * 1_000_000)
                    .ok_or(ContextError::Capacity)?;
                let revision = memory
                    .worker_capabilities
                    .get(&capability_id)
                    .map_or(Ok(1), |old| {
                        old.revision.checked_add(1).ok_or(ContextError::Capacity)
                    })?;
                let mut capability = WorkerCapability {
                    id: capability_id.clone(),
                    scope: self.scope.clone(),
                    principal: principal.clone(),
                    worker_id,
                    revision,
                    revoked: false,
                    allowed_kinds: BTreeSet::from([kind]),
                    source_ids,
                    record_ids: record_ids.clone(),
                    new_record_ids,
                    session_id,
                    conversation,
                    lease: DevelopmentLease {
                        id: request_digest.clone(),
                        attempt: 1,
                        expires_at_ns,
                    },
                    budget,
                };
                capability.validate()?;
                if let Some(previous) = memory
                    .worker_capabilities
                    .get(&capability_id)
                    .filter(|old| old.lease.id == request_digest)
                {
                    if previous.revoked {
                        return Err(ContextError::Stale.into());
                    }
                    capability = previous.clone();
                }
                let mut record_revisions = BTreeMap::new();
                for id in &record_ids {
                    let record = memory
                        .read(owner, id, now)?
                        .ok_or_else(|| ContextError::Unavailable("development record".into()))?;
                    record_revisions.insert(id.clone(), record.revision_digest.clone());
                }
                crate::context_projection::validate_tail(actor, tail).await?;
                let receipt = context_memory::execute_fenced_locked(
                    actor,
                    &self.scope,
                    owner,
                    MemoryRequest {
                        version: 1,
                        scope: self.scope.clone(),
                        request_id: request.request_id.clone(),
                        command: MemoryCommand::RegisterWorker {
                            capability: capability.clone(),
                        },
                    },
                    Some(tail),
                )
                .await?;
                if !record_ids.is_empty() {
                    context_memory::execute_fenced_locked(
                        actor,
                        &self.scope,
                        owner,
                        MemoryRequest {
                            version: 1,
                            scope: self.scope.clone(),
                            request_id: format!("{}-read", request.request_id),
                            command: MemoryCommand::SetGrant {
                                grant: MemoryGrant {
                                    id: format!("development-{}", capability_id),
                                    principal_digest: None,
                                    principal,
                                    record_ids,
                                    categories: BTreeSet::new(),
                                    read: true,
                                    expires_at_ns: Some(capability.lease.expires_at_ns),
                                    revoked: false,
                                    revision: capability.revision,
                                    record_revisions,
                                },
                            },
                        },
                        Some(if receipt.replayed {
                            tail
                        } else {
                            hm_core::LSN::new(receipt.last_lsn)
                        }),
                    )
                    .await?;
                }
                Ok(json!({"capability":capability,"receipt":receipt}))
            }
            DevelopmentAction::Revoke {
                capability_id,
                expected_revision,
            } => {
                let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
                let tail = actor.stats().await?.applied.last_lsn;
                let state = context_memory::rebuild(actor, &self.scope).await?;
                let grant_id = format!("development-{}", capability_id);
                let receipt = context_memory::execute_fenced_locked(
                    actor,
                    &self.scope,
                    owner,
                    MemoryRequest {
                        version: 1,
                        scope: self.scope.clone(),
                        request_id: request.request_id.clone(),
                        command: MemoryCommand::RevokeWorker {
                            id: capability_id,
                            expected_revision,
                        },
                    },
                    Some(tail),
                )
                .await?;
                if state.grants.contains_key(&grant_id) {
                    context_memory::execute_fenced_locked(
                        actor,
                        &self.scope,
                        owner,
                        MemoryRequest {
                            version: 1,
                            scope: self.scope.clone(),
                            request_id: format!("{}-read", request.request_id),
                            command: MemoryCommand::RevokeGrant { id: grant_id },
                        },
                        Some(if receipt.replayed {
                            tail
                        } else {
                            hm_core::LSN::new(receipt.last_lsn)
                        }),
                    )
                    .await?;
                }
                Ok(serde_json::to_value(receipt)?)
            }
            DevelopmentAction::Configure { schedule } => {
                let principal = self.principal(&schedule.worker_id)?;
                let (kind, provider_policy) = self.workers[&schedule.worker_id].clone();
                let memory = context_memory::rebuild(actor, &self.scope).await?;
                let cap = memory
                    .worker_capabilities
                    .get(&schedule.snapshot.capability_id)
                    .ok_or_else(|| ContextError::Unavailable("worker capability".into()))?;
                if cap.revoked
                    || cap.worker_id != schedule.worker_id
                    || cap.principal != principal
                    || !cap.allowed_kinds.contains(&kind)
                    || !schedule.snapshot.source_ids.is_subset(&cap.source_ids)
                    || !schedule.snapshot.record_ids.is_subset(&cap.record_ids)
                    || schedule.reservation != cap.budget.reserved_tokens
                {
                    return Err(ContextError::ScopeMismatch.into());
                }
                self.scheduler(
                    actor,
                    request.request_id,
                    SchedulerAction::Configure {
                        schedule: DevelopmentSchedule {
                            id: schedule.id,
                            mode: schedule.mode,
                            worker_id: schedule.worker_id,
                            kind,
                            principal,
                            snapshot: schedule.snapshot,
                            reservation: schedule.reservation,
                            timeout_ms: schedule.timeout_ms,
                            backoff_ms: schedule.backoff_ms,
                            max_attempts: schedule.max_attempts,
                            identical_failure_limit: schedule.identical_failure_limit,
                            provider_policy,
                        },
                    },
                )
                .await
            }
            DevelopmentAction::Enqueue { schedule_id } => {
                self.scheduler(
                    actor,
                    request.request_id,
                    SchedulerAction::Enqueue { schedule_id },
                )
                .await
            }
            DevelopmentAction::Cancel { job_id } => {
                self.scheduler(
                    actor,
                    request.request_id,
                    SchedulerAction::Cancel { job_id },
                )
                .await
            }
            DevelopmentAction::Dispatch { job_id } => Ok(serde_json::to_value(
                development_scheduler::dispatch(
                    actor,
                    &self.scope,
                    owner,
                    &request.request_id,
                    &job_id,
                    &self.registry,
                )
                .await?,
            )?),
            DevelopmentAction::Review { mut decision } => {
                decision.request_id = request.request_id;
                Ok(serde_json::to_value(
                    development_admission::decide(actor, &self.scope, owner, decision).await?,
                )?)
            }
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct CurrentSummaries {
    pub summaries: Vec<(
        hm_context::historian::SourceChunk,
        hm_context::historian::HistorianResult,
    )>,
    pub omitted: BTreeMap<String, String>,
}
pub fn current_summaries(
    memory: &context_memory::MemoryProjection,
    principal: &Scope,
    history: &hm_context::history::SourceHistory,
    policy_revision: u64,
    now_ns: i64,
) -> Result<CurrentSummaries, MemoryError> {
    if memory.scope != *history.scope() || principal != &memory.scope {
        return Err(ContextError::ScopeMismatch.into());
    }
    let mut report = CurrentSummaries {
        summaries: Vec::new(),
        omitted: BTreeMap::new(),
    };
    for record in memory.records.values().filter(|record| {
        record.kind == context_memory::RecordKind::Summary
            && record.authority == hm_context::Authority::DerivedInference
            && record.metadata.get("historian_result").is_some()
    }) {
        if memory.read(principal, &record.id, now_ns)?.is_none() {
            report
                .omitted
                .insert(record.id.clone(), "summary is not currently visible".into());
            continue;
        }
        let publication = crate::development_historian::decode_publication(record)?;
        let visible = history.visible_messages();
        let current = publication.policy_revision == policy_revision
            && publication
                .chunk
                .sources
                .iter()
                .all(|source| visible.iter().any(|message| *message == source))
            && publication.chunk.spans.iter().all(|span| {
                history
                    .source_span(&span.source_id)
                    .is_ok_and(|current| current == *span)
            });
        if current {
            report
                .summaries
                .push((publication.chunk, publication.result));
        } else {
            report.omitted.insert(
                record.id.clone(),
                "summary source or policy fence changed".into(),
            );
        }
    }
    Ok(report)
}
