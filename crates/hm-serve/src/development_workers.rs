use crate::{
    actor::ActorEngine,
    context_memory::{self, MemoryError},
    development_scheduler::{
        DevelopmentWorker, DispatchContext, TrustedNoWork, WorkerFailure, WorkerOutcome,
    },
};
use hm_context::{ContextError, development::*, history::SourceRelation, maintenance::Usage};
use hm_cortex::{
    development_curation::{CurationAction, CurationRequest},
    development_mapping::AuthorizedRepository,
};
use hm_llm::{LlmError, LlmProvider, StructuredRequest, StructuredResponse};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerConfiguration {
    pub id: String,
    pub operation: WorkerOperation,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "family", rename_all = "snake_case", deny_unknown_fields)]
pub enum WorkerOperation {
    Curation {
        actions: Vec<CurationAction>,
    },
    Verification {
        root: PathBuf,
        files: BTreeMap<String, String>,
    },
    Profile {
        target_id: String,
    },
    Primer {
        job_id: String,
    },
    Retrospective {
        checkpoint_id: String,
        lesson_ids: Vec<String>,
    },
}
impl WorkerOperation {
    pub fn kind(&self) -> DevelopmentKind {
        match self {
            Self::Curation { .. } => DevelopmentKind::Curation,
            Self::Verification { .. } => DevelopmentKind::Verification,
            Self::Profile { .. } => DevelopmentKind::ProfileProposal,
            Self::Primer { .. } => DevelopmentKind::Primer,
            Self::Retrospective { .. } => DevelopmentKind::Retrospective,
        }
    }
}
impl WorkerConfiguration {
    pub fn validate(&self) -> Result<(), ContextError> {
        hm_context::validate_id(&self.id)?;
        if matches!(self.id.as_str(), "historian" | "extraction") {
            return Err(ContextError::Conflict);
        }
        match &self.operation {
            WorkerOperation::Curation { actions } => {
                if actions.is_empty() || actions.len() > 64 {
                    return Err(ContextError::Capacity);
                }
                for action in actions {
                    match action {
                        CurationAction::Reword { id } | CurationAction::Classify { id } => {
                            hm_context::validate_id(id)?
                        }
                        CurationAction::Merge { id, parents, .. } => {
                            hm_context::validate_id(id)?;
                            if parents.is_empty() || parents.len() > 32 {
                                return Err(ContextError::Capacity);
                            }
                            for p in parents {
                                hm_context::validate_id(p)?;
                            }
                        }
                        CurationAction::Archive { id, duplicate_of } => {
                            hm_context::validate_id(id)?;
                            hm_context::validate_id(duplicate_of)?;
                        }
                    }
                }
            }
            WorkerOperation::Verification { root, files } => {
                if !root.is_absolute() || !root.is_dir() || files.len() > 256 {
                    return Err(ContextError::Invalid(
                        "authorized repository configuration".into(),
                    ));
                }
                for (id, path) in files {
                    hm_context::validate_id(id)?;
                    if path.is_empty()
                        || !std::path::Path::new(path)
                            .components()
                            .all(|c| matches!(c, std::path::Component::Normal(_)))
                    {
                        return Err(ContextError::Invalid(
                            "authorized relative repository path".into(),
                        ));
                    }
                }
            }
            WorkerOperation::Profile { target_id } => hm_context::validate_id(target_id)?,
            WorkerOperation::Primer { job_id } => hm_context::validate_id(job_id)?,
            WorkerOperation::Retrospective {
                checkpoint_id,
                lesson_ids,
            } => {
                hm_context::validate_id(checkpoint_id)?;
                if lesson_ids.is_empty() || lesson_ids.len() > 63 {
                    return Err(ContextError::Capacity);
                }
                let mut ids = BTreeSet::new();
                for id in lesson_ids {
                    hm_context::validate_id(id)?;
                    if id == checkpoint_id || !ids.insert(id) {
                        return Err(ContextError::Conflict);
                    }
                }
            }
        };
        Ok(())
    }
}
pub fn configured_workers(
    actor: ActorEngine,
    configurations: &[WorkerConfiguration],
    endpoint: &str,
    model: &str,
    timeout: Duration,
) -> Result<Vec<(String, Arc<dyn DevelopmentWorker>)>, MemoryError> {
    if configurations.len() > 16 || timeout.is_zero() || timeout > Duration::from_secs(300) {
        return Err(ContextError::Capacity.into());
    }
    let mut ids = BTreeSet::new();
    let mut result = Vec::new();
    for config in configurations {
        config.validate()?;
        if !ids.insert(&config.id) {
            return Err(ContextError::Conflict.into());
        }
        result.push((
            config.id.clone(),
            Arc::new(ConfiguredWorker {
                actor: actor.clone(),
                configuration: config.clone(),
                endpoint: endpoint.into(),
                model: model.into(),
                timeout,
            }) as Arc<dyn DevelopmentWorker>,
        ));
    }
    Ok(result)
}
struct ConfiguredWorker {
    actor: ActorEngine,
    configuration: WorkerConfiguration,
    endpoint: String,
    model: String,
    timeout: Duration,
}
#[derive(Default)]
struct Observed {
    calls: u64,
    tokens: u64,
    unknown: bool,
}
struct SharedTransport(Arc<crate::development_usage::ObservedTransport>);
impl hm_llm::WireTransport for SharedTransport {
    fn send(&self, request: &hm_llm::WireRequest) -> Result<hm_llm::WireResponse, LlmError> {
        hm_llm::WireTransport::send(self.0.as_ref(), request)
    }
}
struct CountedProvider {
    inner: Arc<dyn LlmProvider>,
    observed: Arc<Mutex<Observed>>,
}
impl LlmProvider for CountedProvider {
    fn model_id(&self) -> &str {
        self.inner.model_id()
    }
    fn tier(&self) -> hm_llm::ModelTier {
        self.inner.tier()
    }
    fn generate_structured(
        &self,
        request: &StructuredRequest,
    ) -> Result<StructuredResponse, LlmError> {
        {
            let mut usage = self.observed.lock().map_err(|_| LlmError::Capacity)?;
            usage.calls = usage.calls.checked_add(1).ok_or(LlmError::Capacity)?;
        }
        let response = self.inner.generate_structured(request);
        let mut usage = self.observed.lock().map_err(|_| LlmError::Capacity)?;
        match &response {
            Ok(result) => {
                let Some(tokens) = result
                    .usage
                    .input_tokens
                    .checked_add(result.usage.output_tokens)
                    .and_then(|n| usage.tokens.checked_add(n))
                else {
                    usage.unknown = true;
                    return Err(LlmError::Capacity);
                };
                usage.tokens = tokens;
            }
            Err(_) => usage.unknown = true,
        };
        response
    }
}
fn failure(error: impl std::fmt::Display, usage: Usage) -> WorkerFailure {
    WorkerFailure {
        error: error.to_string(),
        usage,
    }
}
fn observed_usage(observed: &Mutex<Observed>) -> Usage {
    match observed.lock() {
        Ok(state) if !state.unknown => Usage::Known(state.tokens),
        _ => Usage::Unknown,
    }
}
impl DevelopmentWorker for ConfiguredWorker {
    fn kind(&self) -> DevelopmentKind {
        self.configuration.operation.kind()
    }
    fn provider_policy(&self) -> Option<&str> {
        Some("ollama")
    }
    fn run(
        &self,
        evidence: EvidenceSnapshot,
        plan_id: String,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<DevelopmentPlan, WorkerFailure>> + Send + '_>,
    > {
        Box::pin(async move {
            match self.run_configured(evidence, plan_id, None).await? {
                WorkerOutcome::Publication(plan) => Ok(plan),
                WorkerOutcome::NoWork(_) => Err(failure(
                    "no-work outcome requires scheduler receipt",
                    Usage::Known(0),
                )),
            }
        })
    }
    fn run_outcome(
        &self,
        evidence: EvidenceSnapshot,
        plan_id: String,
        context: DispatchContext,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<WorkerOutcome, WorkerFailure>> + Send + '_>,
    > {
        self.run_configured(evidence, plan_id, Some(context))
    }
}
impl ConfiguredWorker {
    fn run_configured(
        &self,
        evidence: EvidenceSnapshot,
        plan_id: String,
        context: Option<DispatchContext>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<WorkerOutcome, WorkerFailure>> + Send + '_>,
    > {
        Box::pin(async move {
            let tail = self
                .actor
                .stats()
                .await
                .map_err(|e| failure(e, Usage::Known(0)))?
                .applied
                .last_lsn;
            let state = context_memory::rebuild(&self.actor, &evidence.scope)
                .await
                .map_err(|e| failure(e, Usage::Known(0)))?;
            let cap = state
                .worker_capabilities
                .get(&evidence.capability_id)
                .ok_or_else(|| failure(ContextError::Stale, Usage::Known(0)))?;
            if cap.digest().map_err(|e| failure(e, Usage::Known(0)))? != evidence.capability_digest
                || cap.worker_id != self.configuration.id
            {
                return Err(failure(ContextError::Stale, Usage::Known(0)));
            }
            let operation = self.configuration.operation.clone();
            let mut profile = Vec::new();
            let mut primers = Vec::new();
            let mut primer_target = None;
            let mut retro = None;
            let mut previous = Vec::new();
            match &operation {
                WorkerOperation::Profile { target_id } => {
                    if !cap.new_record_ids.contains(target_id) {
                        return Err(failure(ContextError::ScopeMismatch, Usage::Known(0)));
                    }
                    let policy = evidence
                        .records
                        .iter()
                        .find(|r| r.id == "profile-collection-policy")
                        .ok_or_else(|| failure(ContextError::ScopeMismatch, Usage::Known(0)))?;
                    for source in &evidence.sources {
                        let session = policy.metadata["observations"][&source.id]
                            .as_str()
                            .ok_or_else(|| failure(ContextError::ScopeMismatch, Usage::Known(0)))?;
                        let native = state
                            .sources
                            .get(&source.id)
                            .ok_or_else(|| failure(ContextError::ScopeMismatch, Usage::Known(0)))?;
                        if native.locator != format!("profile-session:{session}") {
                            return Err(failure(ContextError::ScopeMismatch, Usage::Known(0)));
                        }
                        profile.push(hm_cortex::development_profile::ProfileObservation {
                            source_id: source.id.clone(),
                            session_id: session.into(),
                        });
                    }
                }
                WorkerOperation::Primer { job_id } => {
                    let job = evidence
                        .records
                        .iter()
                        .find(|r| &r.id == job_id)
                        .ok_or_else(|| failure(ContextError::ScopeMismatch, Usage::Known(0)))?;
                    let target = job.metadata["target_id"]
                        .as_str()
                        .ok_or_else(|| failure(ContextError::ScopeMismatch, Usage::Known(0)))?
                        .to_string();
                    let refresh = job.metadata["refresh"]
                        .as_bool()
                        .ok_or_else(|| failure(ContextError::ScopeMismatch, Usage::Known(0)))?;
                    if (!refresh && !cap.new_record_ids.contains(&target))
                        || (refresh && !cap.record_ids.contains(&target))
                    {
                        return Err(failure(ContextError::ScopeMismatch, Usage::Known(0)));
                    }
                    let index = evidence
                        .records
                        .iter()
                        .find(|r| r.id == hm_cortex::development_primers::INDEX_ID)
                        .ok_or_else(|| failure(ContextError::ScopeMismatch, Usage::Known(0)))?;
                    for source in &evidence.sources {
                        let session = index.metadata["sessions"][&source.id]
                            .as_str()
                            .ok_or_else(|| failure(ContextError::ScopeMismatch, Usage::Known(0)))?;
                        let native = state
                            .sources
                            .get(&source.id)
                            .ok_or_else(|| failure(ContextError::ScopeMismatch, Usage::Known(0)))?;
                        if native.locator != format!("primer-session:{session}") {
                            return Err(failure(ContextError::ScopeMismatch, Usage::Known(0)));
                        }
                        primers.push(hm_cortex::development_primers::PrimerObservation {
                            source_id: source.id.clone(),
                            session_id: session.into(),
                        });
                    }
                    primer_target = Some((target, refresh));
                }
                WorkerOperation::Retrospective {
                    checkpoint_id,
                    lesson_ids,
                } => {
                    if state.records.values().any(|r| {
                        r.category == hm_cortex::development_retrospective::LESSON_CATEGORY
                            && !evidence.records.iter().any(|seen| seen.id == r.id)
                    }) {
                        return Err(failure(ContextError::ScopeMismatch, Usage::Known(0)));
                    }
                    if (!state.records.contains_key(checkpoint_id)
                        && !cap.new_record_ids.contains(checkpoint_id))
                        || lesson_ids.iter().any(|id| !cap.new_record_ids.contains(id))
                    {
                        return Err(failure(ContextError::ScopeMismatch, Usage::Known(0)));
                    }
                    let history = crate::context_history::replay(
                        &self.actor,
                        &evidence.scope,
                        &evidence.session_id,
                        &evidence.conversation,
                    )
                    .await
                    .map_err(|e| failure(format!("{e:?}"), Usage::Known(0)))?;
                    let mut signals = Vec::new();
                    for relation in history.history.relations() {
                        if let SourceRelation::Edit {
                            id, replacement_id, ..
                        } = relation
                        {
                            if let Some(source) =
                                evidence.sources.iter().find(|s| s.id == *replacement_id)
                            {
                                if history.history.visible_messages().iter().any(|s| {
                                    s.id == *replacement_id
                                        && s.role == hm_context::MessageRole::User
                                        && s.authority == hm_context::Authority::UserAsserted
                                }) {
                                    signals.push(
                                        hm_cortex::development_retrospective::CorrectionSignal {
                                            relation_id: id.clone(),
                                            source_id: replacement_id.clone(),
                                            source_digest: source.digest.clone(),
                                        },
                                    );
                                }
                            }
                        }
                    }
                    let watermark = if let Some(record) =
                        evidence.records.iter().find(|r| &r.id == checkpoint_id)
                    {
                        if record.category
                            != hm_cortex::development_retrospective::CHECKPOINT_CATEGORY
                            || record.status != DevelopmentRecordStatus::Archived
                        {
                            return Err(failure(ContextError::Conflict, Usage::Known(0)));
                        }
                        serde_json::from_value(record.metadata.clone())
                            .map_err(|e| failure(e, Usage::Known(0)))?
                    } else {
                        if state.records.contains_key(checkpoint_id) {
                            return Err(failure(ContextError::ScopeMismatch, Usage::Known(0)));
                        }
                        Default::default()
                    };
                    retro = Some(
                        hm_cortex::development_retrospective::detect_new_corrections(
                            evidence.clone(),
                            signals,
                            watermark,
                        )
                        .map_err(|e| failure(e, Usage::Known(0)))?,
                    );
                }
                WorkerOperation::Verification { .. } => {
                    previous = crate::development_mapping::previous_verifications(
                        &self.actor,
                        &evidence.scope,
                        &evidence.principal,
                        &evidence,
                    )
                    .await
                    .map_err(|e| failure(e, Usage::Known(0)))?;
                }
                WorkerOperation::Curation { .. } => {}
            }
            crate::context_projection::validate_tail(&self.actor, tail)
                .await
                .map_err(|e| failure(e, Usage::Known(0)))?;
            let context = context.ok_or_else(|| {
                failure(
                    "original scheduler dispatch binding required",
                    Usage::Known(0),
                )
            })?;
            let binding = crate::development_usage::OriginalCallBinding {
                call_ordinal: 0,
                scope: context.scope,
                job_id: context.lease.job_id.clone(),
                attempt: context.lease.attempt,
                maintenance_lease: context.maintenance_lease,
                worker_id: context.worker_id,
                session_id: evidence.session_id.clone(),
                plan_id: plan_id.clone(),
                provider_id: "ollama".into(),
                model_id: self.model.clone(),
                evidence_digest: evidence.digest.clone(),
                source_digests: evidence
                    .sources
                    .iter()
                    .map(|source| (source.id.clone(), source.content_digest.clone()))
                    .collect(),
            };
            let handle = tokio::runtime::Handle::current();
            let endpoint = self.endpoint.clone();
            let model = self.model.clone();
            let observed = Arc::new(Mutex::new(Observed::default()));
            let task_observed = observed.clone();
            let output_id = plan_id.clone();
            let task = tokio::task::spawn_blocking(move || -> Result<WorkerOutcome, String> {
                let transport = Arc::new(crate::development_usage::ObservedTransport::configured(
                    context.actor,
                    binding,
                    handle,
                ));
                let actual = hm_llm::ollama::Ollama::new(
                    hm_llm::ProviderConfig {
                        endpoint,
                        api_key: None,
                        model,
                        tier: hm_llm::ModelTier::Economy,
                        pricing: Default::default(),
                    },
                    SharedTransport(transport.clone()),
                )
                .map_err(|e| format!("{e:?}"))?;
                let provider = CountedProvider {
                    inner: Arc::new(actual),
                    observed: task_observed,
                };
                let outcome = (|| -> Result<WorkerOutcome, String> {
                    let mut plan = match operation {
                    WorkerOperation::Curation { actions } => {
                        hm_cortex::development_curation::curate(
                            evidence,
                            &provider,
                            CurationRequest { plan_id, actions },
                        )
                        .map_err(|e| format!("{e:?}"))?
                    }
                    WorkerOperation::Verification { root, files } => {
                        let repository = AuthorizedRepository { root, files };
                        let mappings =
                            hm_cortex::development_mapping::map_evidence(&evidence, &repository)
                                .map_err(|e| e.to_string())?;
                        let now =
                            crate::session_context::runtime_now_ns().map_err(|e| e.to_string())?;
                        let batch = hm_cortex::development_mapping::verify_changed(
                            &evidence, &mappings, &previous, &provider, &plan_id, now,
                        )
                        .map_err(|e| e.to_string())?;
                        hm_cortex::development_mapping::validate_repository(
                            &batch.plan.evidence,
                            &repository,
                        )
                        .map_err(|e| e.to_string())?;
                        batch.plan
                    }
                    WorkerOperation::Profile { target_id } => {
                        let plan = hm_cortex::development_profile::propose_profile(
                            evidence, &profile, &target_id, &provider,
                        )
                        .map_err(|e| e.to_string())?;
                        for old in state.development_proposals.values().filter(|p| {
                            p.kind == DevelopmentKind::ProfileProposal
                                && p.status == ProposalStatus::Rejected
                        }) {
                            for mutation in &old.mutations {
                                for new in plan.proposal.iter().flat_map(|p| &p.mutations) {
                                    if let (
                                        PlannedKnowledgeMutation::Create { record: a },
                                        PlannedKnowledgeMutation::Create { record: b },
                                    ) = (mutation, new)
                                    {
                                        if a.metadata["profile_attribute"]
                                            == b.metadata["profile_attribute"]
                                        {
                                            return Err(
                                                "owner rejected this profile candidate".into()
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        plan
                    }
                    WorkerOperation::Primer { job_id } => {
                        let (target, refresh) =
                            primer_target.ok_or("missing authorized primer target")?;
                        hm_cortex::development_primers::develop(
                            evidence, &primers, &job_id, &target, refresh, &provider,
                        )
                        .map_err(|e| e.to_string())?
                    }
                    WorkerOperation::Retrospective {
                        checkpoint_id,
                        lesson_ids,
                    } => match hm_cortex::development_retrospective::propose_lessons(
                        retro.ok_or("missing retrospective evidence")?,
                        &provider,
                        &plan_id,
                        &checkpoint_id,
                        &lesson_ids,
                    )
                    .map_err(|e| e.to_string())?
                    {
                        hm_cortex::development_retrospective::RetrospectiveProposal::Proposed {
                            plan,
                            ..
                        } => plan,
                        hm_cortex::development_retrospective::RetrospectiveProposal::NoSignal {
                            ..
                        } => {
                            let state = provider
                                .observed
                                .lock()
                                .map_err(|_| "provider call accounting unavailable")?;
                            if state.calls != 0 || state.unknown || state.tokens != 0 {
                                return Err("no-signal outcome followed provider attempt".into());
                            }
                            return Ok(WorkerOutcome::NoWork(TrustedNoWork {
                                evidence,
                                plan_id,
                                kind: DevelopmentKind::Retrospective,
                                reason: "no_signal".into(),
                            }));
                        }
                    },
                };
                    plan.id = output_id;
                    Ok(WorkerOutcome::Publication(plan))
                })();
                transport.close().map_err(|e| format!("{e:?}"))?;
                outcome
            });
            match tokio::time::timeout(self.timeout, task).await {
                Ok(Ok(Ok(plan))) => Ok(plan),
                Ok(Ok(Err(error))) => Err(failure(error, observed_usage(&observed))),
                Ok(Err(error)) => Err(failure(error, observed_usage(&observed))),
                Err(_) => Err(failure(
                    "development provider deadline exceeded",
                    Usage::Unknown,
                )),
            }
        })
    }
}
