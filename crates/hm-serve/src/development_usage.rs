use crate::{
    actor::{ActorEngine, IncomingEvent},
    context_memory::MemoryError,
    development_scheduler::{self, DevelopmentWorker, DispatchContext, WorkerFailure},
};
use hm_context::{
    ContextError, Scope,
    development::{DevelopmentKind, DevelopmentPlan, EvidenceSnapshot},
    digest_bytes,
    maintenance::{JobLease, Usage},
};
use hm_core::{ConversationId, LSN};
use hm_cortex::development_historian::HistorianProvider;
use hm_llm::{
    HttpTransport, LlmError, WireRequest, WireResponse, WireTransport,
    provider_usage::{ObservationFormat, ObservationMetadata, ProviderUsageSnapshot},
};
use hm_schema::{
    event::{self, Boundary, CURRENT_SCHEMA_VERSION},
    events::{EventEnvelope, EventPayload, ProviderFrame, Retention, Sensitivity},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    future::Future,
    pin::Pin,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
const PROVIDER: &str = "hypermind/development-usage/v1";
static WRITER: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OriginalCallBinding {
    pub scope: Scope,
    pub job_id: String,
    pub attempt: u64,
    pub maintenance_lease: JobLease,
    pub worker_id: String,
    pub session_id: String,
    pub plan_id: String,
    pub provider_id: String,
    pub model_id: String,
    pub evidence_digest: String,
    pub source_digests: BTreeMap<String, String>,
}
impl OriginalCallBinding {
    fn id(&self) -> Result<String, MemoryError> {
        Ok(digest_bytes(&serde_json::to_vec(self)?))
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OriginalUsageObservation {
    pub id: String,
    pub binding: OriginalCallBinding,
    pub request_digest: String,
    pub snapshot: ProviderUsageSnapshot,
    pub accepted_response: bool,
}
#[derive(Clone, Serialize, Deserialize)]
struct OriginalAttempt {
    binding: OriginalCallBinding,
    request_digest: String,
    observation: Option<OriginalUsageObservation>,
}
#[derive(Clone, Serialize, Deserialize)]
struct State {
    version: u32,
    scope: Scope,
    sequence: u64,
    attempts: BTreeMap<String, OriginalAttempt>,
}
fn invalid(message: &str) -> MemoryError {
    ContextError::Invalid(message.into()).into()
}
async fn load(actor: &ActorEngine, scope: &Scope) -> Result<State, MemoryError> {
    let mut state = State {
        version: 1,
        scope: scope.clone(),
        sequence: 0,
        attempts: BTreeMap::new(),
    };
    for frame in actor.frames_since(LSN::new(0), None, usize::MAX).await? {
        if frame.header.kind != hm_ledger::frame::EventKind::ProviderFrame {
            continue;
        }
        let verified = event::verify_event(
            &frame.sealed_payload,
            event::EventKind::ProviderFrame,
            Boundary::Disk,
        )?;
        let EventPayload::ProviderFrame(provider) = verified.envelope.payload else {
            continue;
        };
        if provider.provider != PROVIDER {
            continue;
        }
        let next: State = serde_json::from_slice(&provider.api_content)?;
        if next.scope != *scope {
            continue;
        }
        if next.version != 1 || next.sequence != state.sequence + 1 {
            return Err(invalid("original usage sequence"));
        }
        for (id, attempt) in &next.attempts {
            if id != &attempt.binding.id()? || attempt.binding.scope != *scope {
                return Err(ContextError::ScopeMismatch.into());
            }
            if let Some(observation) = &attempt.observation {
                if observation.binding != attempt.binding
                    || observation.request_digest != attempt.request_digest
                {
                    return Err(invalid("original response attribution"));
                }
                let parsed = ProviderUsageSnapshot::parse(
                    &observation.snapshot.original_bytes,
                    observation.snapshot.observation.clone(),
                )
                .map_err(|e| invalid(&e.to_string()))?;
                if parsed != observation.snapshot {
                    return Err(invalid("original usage evidence"));
                }
            }
        }
        state = next;
    }
    Ok(state)
}
async fn append(actor: &ActorEngine, tail: LSN, state: &mut State) -> Result<(), MemoryError> {
    state.sequence = state
        .sequence
        .checked_add(1)
        .ok_or(ContextError::Capacity)?;
    actor
        .append_if_tail(
            tail,
            vec![IncomingEvent {
                kind: hm_ledger::frame::EventKind::ProviderFrame,
                conversation: ConversationId::derive(PROVIDER),
                payload: event::encode_event_envelope(&EventEnvelope {
                    schema_version: CURRENT_SCHEMA_VERSION,
                    payload: EventPayload::ProviderFrame(Box::new(ProviderFrame {
                        provider: PROVIDER.into(),
                        api_content: serde_json::to_vec(state)?,
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
                }),
            }],
        )
        .await?;
    Ok(())
}
async fn begin(
    actor: &ActorEngine,
    binding: &OriginalCallBinding,
    request_digest: &str,
) -> Result<(), MemoryError> {
    let _writer = WRITER.lock().await;
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let tail = actor.stats().await?.applied.last_lsn;
    development_scheduler::validate_original_binding(actor, binding, true).await?;
    let mut state = load(actor, &binding.scope).await?;
    let id = binding.id()?;
    if state.attempts.contains_key(&id) {
        return Err(invalid(
            "original provider attempt already dispatched; never repeated",
        ));
    }
    state.attempts.insert(
        id,
        OriginalAttempt {
            binding: binding.clone(),
            request_digest: request_digest.into(),
            observation: None,
        },
    );
    append(actor, tail, &mut state).await
}
async fn capture(
    actor: &ActorEngine,
    binding: &OriginalCallBinding,
    request_digest: &str,
    response: &WireResponse,
    bytes: Vec<u8>,
) -> Result<OriginalUsageObservation, MemoryError> {
    let _writer = WRITER.lock().await;
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let tail = actor.stats().await?.applied.last_lsn;
    let mut state = load(actor, &binding.scope).await?;
    let attempt = state
        .attempts
        .get_mut(&binding.id()?)
        .ok_or(ContextError::Stale)?;
    if attempt.binding != *binding || attempt.request_digest != request_digest {
        return Err(ContextError::ScopeMismatch.into());
    }
    let id = digest_bytes(&serde_json::to_vec(&(binding, request_digest, &bytes))?);
    if let Some(observation) = &attempt.observation {
        if observation.id != id {
            return Err(ContextError::Conflict.into());
        }
        return Ok(observation.clone());
    }
    let observed_at_ns = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| invalid("clock"))?
            .as_nanos(),
    )
    .map_err(|_| ContextError::Capacity)?;
    let snapshot = ProviderUsageSnapshot::parse(
        &bytes,
        ObservationMetadata {
            provider_id: binding.provider_id.clone(),
            source: "original-development-provider-response".into(),
            evidence_id: id.clone(),
            observed_at_ns,
            expires_at_ns: None,
            format: ObservationFormat::Ollama,
        },
    )
    .map_err(|e| invalid(&e.to_string()))?;
    if snapshot.raw != response.body {
        return Err(invalid("original raw response mismatch"));
    }
    let accepted_response = (200..300).contains(&response.status)
        && snapshot.raw["model"].as_str() == Some(&binding.model_id)
        && snapshot.raw["done"] == true;
    let observation = OriginalUsageObservation {
        id,
        binding: binding.clone(),
        request_digest: request_digest.into(),
        snapshot,
        accepted_response,
    };
    attempt.observation = Some(observation.clone());
    append(actor, tail, &mut state).await?;
    Ok(observation)
}
pub async fn inspect(
    actor: &ActorEngine,
    scope: &Scope,
    owner: &Scope,
) -> Result<Vec<OriginalUsageObservation>, MemoryError> {
    if scope != owner {
        return Err(ContextError::ScopeMismatch.into());
    }
    Ok(load(actor, scope)
        .await?
        .attempts
        .into_values()
        .filter_map(|attempt| attempt.observation)
        .collect())
}
pub(crate) async fn for_attempt(
    actor: &ActorEngine,
    scope: &Scope,
    job_id: &str,
    attempt: u64,
) -> Result<Option<OriginalUsageObservation>, MemoryError> {
    let observations: Vec<_> = load(actor, scope)
        .await?
        .attempts
        .into_values()
        .filter(|a| a.binding.job_id == job_id && a.binding.attempt == attempt)
        .filter_map(|a| a.observation)
        .collect();
    if observations.len() > 1 {
        return Err(ContextError::Conflict.into());
    }
    Ok(observations.into_iter().next())
}
pub(crate) fn actual_tokens(observation: &OriginalUsageObservation) -> Option<u64> {
    if !observation.accepted_response || observation.snapshot.error.is_some() {
        return None;
    }
    observation
        .snapshot
        .tokens
        .input?
        .checked_add(observation.snapshot.tokens.output?)
}
pub async fn reconcile_unknown(
    actor: &ActorEngine,
    scope: &Scope,
    owner: &Scope,
    job_id: &str,
    attempt: u64,
    observation_id: &str,
) -> Result<(), MemoryError> {
    if scope != owner {
        return Err(ContextError::ScopeMismatch.into());
    }
    let observation = for_attempt(actor, scope, job_id, attempt)
        .await?
        .ok_or_else(|| invalid("original provider observation unavailable"))?;
    if observation.id != observation_id {
        return Err(ContextError::ScopeMismatch.into());
    }
    development_scheduler::reconcile_original_observation(actor, scope, &observation).await
}

struct ObservedTransport {
    actor: ActorEngine,
    binding: OriginalCallBinding,
    handle: tokio::runtime::Handle,
    transport: HttpTransport,
}
impl WireTransport for ObservedTransport {
    fn send(&self, request: &WireRequest) -> Result<WireResponse, LlmError> {
        if request.body["model"].as_str() != Some(&self.binding.model_id) {
            return Err(LlmError::InvalidArgument(
                "original provider model mismatch".into(),
            ));
        }
        let request_digest = digest_bytes(
            &serde_json::to_vec(&request.body).map_err(|e| LlmError::Wire(e.to_string()))?,
        );
        self.handle
            .block_on(begin(&self.actor, &self.binding, &request_digest))
            .map_err(|e| LlmError::Network(e.to_string()))?;
        let (response, bytes) = self.transport.send_observed(request)?;
        self.handle
            .block_on(capture(
                &self.actor,
                &self.binding,
                &request_digest,
                &response,
                bytes,
            ))
            .map_err(|e| LlmError::Network(e.to_string()))?;
        Ok(response)
    }
}
#[derive(Clone)]
pub struct ObservedHistorianWorker {
    pub provider: HistorianProvider,
    pub timeout: Duration,
    pub provider_id: String,
}
impl ObservedHistorianWorker {
    pub fn new(
        provider: HistorianProvider,
        timeout: Duration,
        provider_id: String,
    ) -> Result<Self, ContextError> {
        hm_context::validate_id(&provider_id)?;
        if timeout.is_zero() || timeout > Duration::from_secs(300) {
            return Err(ContextError::Capacity);
        }
        Ok(Self {
            provider,
            timeout,
            provider_id,
        })
    }
}
impl DevelopmentWorker for ObservedHistorianWorker {
    fn kind(&self) -> DevelopmentKind {
        self.provider.kind
    }
    fn run(
        &self,
        _evidence: EvidenceSnapshot,
        _plan_id: String,
    ) -> Pin<Box<dyn Future<Output = Result<DevelopmentPlan, WorkerFailure>> + Send + '_>> {
        Box::pin(async {
            Err(WorkerFailure {
                error: "original scheduler dispatch binding required".into(),
                usage: Usage::Unknown,
            })
        })
    }
    fn run_observed(
        &self,
        evidence: EvidenceSnapshot,
        plan_id: String,
        context: DispatchContext,
    ) -> Pin<Box<dyn Future<Output = Result<DevelopmentPlan, WorkerFailure>> + Send + '_>> {
        let provider = self.provider.clone();
        let timeout = self.timeout;
        let provider_id = self.provider_id.clone();
        Box::pin(async move {
            let binding = OriginalCallBinding {
                scope: context.scope,
                job_id: context.lease.job_id.clone(),
                attempt: context.lease.attempt,
                maintenance_lease: context.maintenance_lease,
                worker_id: context.worker_id,
                session_id: evidence.session_id.clone(),
                plan_id: plan_id.clone(),
                provider_id,
                model_id: provider.model.clone(),
                evidence_digest: evidence.digest.clone(),
                source_digests: evidence
                    .sources
                    .iter()
                    .map(|source| (source.id.clone(), source.content_digest.clone()))
                    .collect(),
            };
            let handle = tokio::runtime::Handle::current();
            let task = tokio::task::spawn_blocking(move || {
                let transport = ObservedTransport {
                    actor: context.actor,
                    binding,
                    handle,
                    transport: HttpTransport::default(),
                };
                provider.generate_with_transport(evidence, plan_id, &transport)
            });
            match tokio::time::timeout(timeout, task).await {
                Ok(Ok(Ok(output))) => Ok(output.plan),
                Ok(Ok(Err(error))) => Err(WorkerFailure {
                    error: error.error,
                    usage: error.usage,
                }),
                Ok(Err(_)) => Err(WorkerFailure {
                    error: "original provider worker failed".into(),
                    usage: Usage::Unknown,
                }),
                Err(_) => Err(WorkerFailure {
                    error: "original provider timeout; response may arrive later".into(),
                    usage: Usage::Unknown,
                }),
            }
        })
    }
}
pub async fn dispatched_attempts(
    actor: &ActorEngine,
    scope: &Scope,
    owner: &Scope,
) -> Result<Vec<OriginalCallBinding>, MemoryError> {
    if scope != owner {
        return Err(ContextError::ScopeMismatch.into());
    }
    Ok(load(actor, scope)
        .await?
        .attempts
        .into_values()
        .map(|attempt| attempt.binding)
        .collect())
}
