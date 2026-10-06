use crate::actor::{ActorEngine, IncomingEvent};
use crate::context_projection::{ProjectionRequest, SummaryLevel};
use hm_context::{
    history::SourceHistory,
    provider::{CapabilityProfile, RenderRequest, render_context},
    temporal::{TemporalContext, TemporalSource},
    *,
};
use hm_core::{ConversationId, Error, ErrorCode, LSN};
use hm_schema::{
    event::{self, Boundary, CURRENT_SCHEMA_VERSION},
    events::{EventEnvelope, EventPayload, ProviderFrame, Retention, Sensitivity},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;

const PROVIDER: &str = "hypermind/context-session/v1";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionContextRequest {
    pub version: u32,
    pub scope: Scope,
    pub session_id: String,
    pub budget: TokenBudget,
    #[serde(default)]
    pub generation: u64,
    #[serde(default = "default_model")]
    pub model_id: String,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub utc_offset_seconds: Option<i32>,
    #[serde(default)]
    pub memory_scopes: Vec<Scope>,
    #[serde(default)]
    pub evidence_grants: Vec<hm_context::retrieval::EvidenceGrant>,
    #[serde(default)]
    pub required_message_ids: Vec<String>,
    #[serde(default)]
    pub tier: SummaryLevel,
    #[serde(default)]
    pub defer_reductions: bool,
    #[serde(default)]
    pub profile: CapabilityProfile,
}
fn default_model() -> String {
    "gpt-4o".into()
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct AcceptedKnowledge {
    plans: Vec<hm_context::knowledge_injection::KnowledgePlan>,
    cues: Vec<hm_context::memory_cues::CuePlan>,
    blocks: Vec<ContextBlock>,
    policy_digest: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Binding {
    #[serde(default)]
    knowledge: Option<AcceptedKnowledge>,
    request: SessionContextRequest,
    conversation: String,
}
fn invalid() -> Error {
    Error::new(ErrorCode::InvalidArgument)
}
pub fn context_error(error: ContextError) -> Error {
    match error {
        ContextError::ScopeMismatch => Error::new(ErrorCode::CapabilityDenied),
        ContextError::Stale => Error::new(ErrorCode::SequenceViolation),
        ContextError::Conflict => Error::new(ErrorCode::IdempotencyConflict),
        ContextError::Capacity => Error::new(ErrorCode::CapacityExceeded),
        ContextError::Unavailable(_) => Error::new(ErrorCode::OperationUnavailable),
        ContextError::Io(error) => {
            Error::new(ErrorCode::ReadFailed).with_system_error(error.raw_os_error().unwrap_or(0))
        }
        ContextError::Json(_) => Error::new(ErrorCode::SchemaInvalid),
        ContextError::Invalid(_) => invalid(),
    }
}
fn map(error: ContextError) -> Error {
    context_error(error)
}
pub fn history_error(error: crate::context_history::HistoryError) -> Error {
    match error {
        crate::context_history::HistoryError::Ledger(error) => error,
        crate::context_history::HistoryError::Context(error) => context_error(error),
    }
}
pub fn import_error(error: crate::hypermid_import::ImportError) -> Error {
    match error {
        crate::hypermid_import::ImportError::Ledger(error) => error,
        crate::hypermid_import::ImportError::Context(error) => context_error(error),
    }
}
pub fn retrieval_error(error: crate::context_retrieval::RetrievalError) -> Error {
    match error {
        crate::context_retrieval::RetrievalError::Ledger(error) => error,
        crate::context_retrieval::RetrievalError::Context(error) => context_error(error),
    }
}
async fn bindings(actor: &ActorEngine) -> Result<BTreeMap<String, Binding>, Error> {
    let mut bindings = BTreeMap::new();
    for frame in actor.frames_since(LSN::new(0), None, usize::MAX).await? {
        if frame.header.kind != hm_ledger::frame::EventKind::ProviderFrame {
            continue;
        }
        let verified = event::verify_event(
            &frame.sealed_payload,
            event::EventKind::ProviderFrame,
            Boundary::Disk,
        )?;
        if let EventPayload::ProviderFrame(provider) = verified.envelope.payload {
            if provider.provider == PROVIDER {
                let binding: Binding =
                    serde_json::from_slice(&provider.api_content).map_err(|_| invalid())?;
                if binding.request.version != CONTRACT_VERSION {
                    return Err(invalid());
                }
                if bindings.values().any(|b: &Binding| {
                    b.request.scope != binding.request.scope
                        || b.conversation == binding.conversation
                            && b.request.session_id != binding.request.session_id
                }) {
                    return Err(invalid());
                }
                bindings.insert(binding.request.session_id.clone(), binding);
            }
        }
    }
    Ok(bindings)
}
pub async fn session_request(
    actor: &ActorEngine,
    scope: &Scope,
    session: &str,
) -> Result<(SessionContextRequest, String), Error> {
    let all = bindings(actor).await?;
    let binding = all.get(session).ok_or_else(invalid)?;
    if &binding.request.scope != scope {
        return Err(Error::new(ErrorCode::CapabilityDenied));
    }
    Ok((binding.request.clone(), binding.conversation.clone()))
}

pub async fn conversation_for_session(
    actor: &ActorEngine,
    scope: &Scope,
    session: &str,
) -> Result<String, Error> {
    let all = bindings(actor).await?;
    let binding = all.get(session).ok_or_else(invalid)?;
    if &binding.request.scope != scope {
        return Err(invalid());
    }
    Ok(binding.conversation.clone())
}
pub async fn activate(
    actor: &ActorEngine,
    conversation: &str,
    trusted_scope: &Scope,
    request: SessionContextRequest,
) -> Result<Value, Error> {
    activate_with_provider(actor, conversation, trusted_scope, request, None).await
}
pub async fn activate_with_provider(
    actor: &ActorEngine,
    conversation: &str,
    trusted_scope: &Scope,
    request: SessionContextRequest,
    provider: Option<&crate::context_retrieval::EmbeddingProvider>,
) -> Result<Value, Error> {
    if &request.scope != trusted_scope {
        return Err(Error::new(ErrorCode::CapabilityDenied));
    }
    if request.version != CONTRACT_VERSION {
        return Err(Error::new(ErrorCode::ProtocolVersion));
    }
    if let Some(offset) = request.utc_offset_seconds {
        hm_context::temporal::validate_offset(offset).map_err(map)?;
    }
    request.scope.validate().map_err(map)?;
    validate_id(conversation).map_err(map)?;
    validate_id(&request.session_id).map_err(map)?;
    request.budget.available().map_err(map)?;
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let existing = bindings(actor).await?;
    if existing.values().any(|b| {
        b.request.scope != request.scope
            || b.conversation == conversation && b.request.session_id != request.session_id
    }) {
        return Err(invalid());
    }
    if existing
        .get(&request.session_id)
        .is_some_and(|b| b.conversation != conversation || b.request.scope != request.scope)
    {
        return Err(invalid());
    }
    if request.generation != 0 {
        if let Some(current) =
            crate::context_projection::current(actor, &request.scope, &request.session_id).await?
        {
            if request.generation != current.generation {
                return Err(invalid());
            }
        }
    }
    let knowledge = if request.generation == 0 {
        None
    } else {
        existing
            .get(&request.session_id)
            .filter(|b| {
                b.request.model_id == request.model_id
                    && b.request.budget == request.budget
                    && b.request.memory_scopes == request.memory_scopes
                    && b.request.required_message_ids == request.required_message_ids
                    && serde_json::to_value(b.request.profile).ok()
                        == serde_json::to_value(request.profile).ok()
            })
            .and_then(|b| b.knowledge.clone())
    };
    let mut binding = Binding {
        knowledge,
        request,
        conversation: conversation.into(),
    };
    let result = assemble(actor, &binding, provider).await?;
    binding.knowledge = Some(
        serde_json::from_value(result["knowledge"]["accepted"].clone()).map_err(|_| invalid())?,
    );
    let content = serde_json::to_vec(&binding).map_err(|_| invalid())?;
    if existing
        .get(&binding.request.session_id)
        .is_none_or(|old| serde_json::to_vec(old).ok().as_ref() != Some(&content))
    {
        actor
            .append_if_tail(
                LSN::new(result["ledger_tail"].as_u64().ok_or_else(invalid)?),
                vec![IncomingEvent {
                    kind: hm_ledger::frame::EventKind::ProviderFrame,
                    conversation: ConversationId::derive(conversation),
                    payload: event::encode_event_envelope(&EventEnvelope {
                        schema_version: CURRENT_SCHEMA_VERSION,
                        payload: EventPayload::ProviderFrame(Box::new(ProviderFrame {
                            provider: PROVIDER.into(),
                            api_content: content,
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
    }
    Ok(result)
}
pub async fn inspect(
    actor: &ActorEngine,
    trusted_scope: &Scope,
    session: Option<&str>,
) -> Result<Value, Error> {
    inspect_with_provider(actor, trusted_scope, session, None).await
}
pub async fn inspect_with_provider(
    actor: &ActorEngine,
    trusted_scope: &Scope,
    session: Option<&str>,
    provider: Option<&crate::context_retrieval::EmbeddingProvider>,
) -> Result<Value, Error> {
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let all = bindings(actor).await?;
    if let Some(session) = session {
        let binding = all.get(session).ok_or_else(invalid)?;
        if &binding.request.scope != trusted_scope {
            return Err(invalid());
        }
        return assemble(actor, binding, provider).await;
    }
    let mut sessions = Vec::new();
    for binding in all.values().filter(|b| &b.request.scope == trusted_scope) {
        let history = crate::context_history::replay(
            actor,
            trusted_scope,
            &binding.request.session_id,
            &binding.conversation,
        )
        .await
        .map_err(history_error)?;
        let projection =
            crate::context_projection::current(actor, trusted_scope, &binding.request.session_id)
                .await?;
        sessions.push(json!({"session_id":binding.request.session_id,"scope":trusted_scope,"generation":projection.map_or(0,|p|p.generation),"cursor":history.history.cursor()}));
    }
    Ok(json!({"version":CONTRACT_VERSION,"sessions":sessions}))
}
pub async fn inspect_history(
    actor: &ActorEngine,
    scope: &Scope,
    session: &str,
    source: Option<&str>,
) -> Result<Value, Error> {
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let conversation = conversation_for_session(actor, scope, session).await?;
    let state = crate::context_history::replay(actor, scope, session, &conversation)
        .await
        .map_err(history_error)?;
    if let Some(id) = source {
        let span = state.history.source_span(id).map_err(map)?;
        let bytes = state.history.recover(scope, &span).map_err(map)?;
        return Ok(
            json!({"version":1,"scope":scope,"session_id":session,"source":state.history.message(id).map_err(map)?,"span":span,"original_bytes":bytes}),
        );
    }
    let spans = state
        .history
        .messages()
        .iter()
        .map(|m| state.history.source_span(&m.id))
        .collect::<Result<Vec<_>, _>>()
        .map_err(map)?;
    Ok(
        json!({"version":1,"scope":scope,"session_id":session,"cursor":state.history.cursor(),"messages":state.history.messages(),"relations":state.history.relations(),"parent":state.history.parent(),"spans":spans,"unsupported_parts":state.unsupported_parts}),
    )
}
#[derive(Clone, Debug)]
pub struct RequiredMaterialization {
    pub jobs: Value,
    pub policy: u64,
    pub now_ns: i64,
    pub required_ids: Vec<String>,
    pub blocks: Vec<ContextBlock>,
    pub memory_views: Vec<Value>,
    pub permission_revision: String,
    pub memory_scopes: Vec<Scope>,
    pub facts: BTreeMap<String, String>,
    pub memory_fence: String,
    pub projection_request: ProjectionRequest,
}
pub async fn required_materialization_locked(
    actor: &ActorEngine,
    request: &SessionContextRequest,
    history: &hm_context::history::SourceHistory,
) -> Result<RequiredMaterialization, Error> {
    if &request.scope != history.scope() || request.session_id != history.session_id() {
        return Err(Error::new(ErrorCode::CapabilityDenied));
    }
    let jobs =
        crate::context_jobs::inspect_state(actor, &request.scope, &request.scope.owner_id).await?;
    let policy = jobs["policies"][&request.session_id].as_u64().unwrap_or(1);
    let permissions = jobs["permission_revision"].as_str().ok_or_else(invalid)?;
    let mut required = request.required_message_ids.clone();
    if let Some(last) = history.visible_messages().last() {
        if !required.contains(&last.id) {
            required.push(last.id.clone());
        }
    }
    let note_events: Vec<hm_context::notes::NotesEvent> =
        serde_json::from_value(jobs["notes"].clone()).map_err(|_| invalid())?;
    let notes =
        hm_context::notes::NotesProjection::restore(request.scope.clone(), note_events.clone())
            .map_err(map)?;
    let ids: std::collections::BTreeSet<String> = note_events
        .iter()
        .filter_map(|event| match &event.command {
            hm_context::notes::NotesCommand::Create(note)
            | hm_context::notes::NotesCommand::Revise { note, .. } => Some(note.id.clone()),
            _ => None,
        })
        .collect();
    let now_ns = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| invalid())?
            .as_nanos(),
    )
    .map_err(|_| invalid())?;
    let facts: BTreeMap<String, String> = [
        ("session_id".into(), request.session_id.clone()),
        ("owner_id".into(), request.scope.owner_id.clone()),
        ("project_id".into(), request.scope.project_id.clone()),
    ]
    .into_iter()
    .collect();
    let mut required_blocks = Vec::new();
    for id in ids {
        if let Some(note) = notes
            .read(&request.scope.owner_id, &id, now_ns, &facts)
            .map_err(map)?
        {
            let digest = digest_bytes(note.text.as_bytes());
            required_blocks.push(ContextBlock {
                id: format!(
                    "note:{}:{}",
                    digest_bytes(note.id.as_bytes()),
                    note.revision
                ),
                text: note.text.clone(),
                authority: Authority::DerivedInference,
                provenance: vec![SourceSpan {
                    source_id: format!(
                        "note:{}:{}",
                        digest_bytes(note.id.as_bytes()),
                        note.revision
                    ),
                    source_digest: digest,
                    byte_start: 0,
                    byte_end: note.text.len() as u64,
                }],
                tokens: 0,
                required: true,
            });
        }
    }
    let mut memory_scopes = request.memory_scopes.clone();
    if memory_scopes.is_empty() {
        memory_scopes.push(request.scope.clone());
    }
    memory_scopes.sort();
    memory_scopes.dedup();
    if memory_scopes.len() > 16 {
        return Err(Error::new(ErrorCode::CapacityExceeded));
    }
    let (memory_blocks, memory_views, memory_fence) =
        memory_materialization(actor, &request.scope, &memory_scopes, &facts, now_ns).await?;
    required_blocks.extend(memory_blocks);
    let permission_revision = digest_bytes(
        &serde_json::to_vec(&(permissions, &memory_fence))
            .map_err(|_| Error::new(ErrorCode::SchemaInvalid))?,
    );
    let projection_request = ProjectionRequest {
        model_id: request.model_id.clone(),
        policy_revision: policy.to_string(),
        permission_revision: permission_revision.clone(),
        required_blocks: required_blocks.clone(),
        required_message_ids: required.clone(),
        tier: request.tier,
        defer_reductions: request.defer_reductions,
    };
    Ok(RequiredMaterialization {
        jobs,
        policy,
        now_ns,
        required_ids: required,
        blocks: required_blocks,
        memory_views,
        permission_revision,
        memory_scopes,
        facts,
        memory_fence,
        projection_request,
    })
}

async fn assemble(
    actor: &ActorEngine,
    binding: &Binding,
    provider: Option<&crate::context_retrieval::EmbeddingProvider>,
) -> Result<Value, Error> {
    let request = &binding.request;
    let source_tail = actor.stats().await?.applied.last_lsn;
    let ledger = crate::context_history::replay(
        actor,
        &request.scope,
        &request.session_id,
        &binding.conversation,
    )
    .await
    .map_err(history_error)?;
    if !ledger.unsupported_parts.is_empty() {
        return Err(Error::new(ErrorCode::OperationUnavailable));
    }
    let mut uris = ledger.visible_source_uris();
    let history = ledger.history;
    let materialization = required_materialization_locked(actor, request, &history).await?;
    let RequiredMaterialization {
        jobs,
        policy,
        now_ns,
        required_ids: required,
        memory_views,
        mut projection_request,
        memory_scopes,
        facts,
        memory_fence,
        ..
    } = materialization;
    let mut accepted_knowledge = match &binding.knowledge {
        Some(accepted) => {
            validate_accepted_knowledge(actor, accepted).await?;
            accepted.clone()
        }
        None => {
            build_knowledge(
                actor,
                request,
                &history,
                &memory_scopes,
                &projection_request.required_blocks,
            )
            .await?
        }
    };
    bind_knowledge_policy(&mut projection_request, &accepted_knowledge.policy_digest)?;
    let initial = crate::context_projection::assemble(
        actor,
        &history,
        projection_request.clone(),
        source_tail,
    )
    .await?;
    let mut gaps = Vec::new();
    let mut published = Vec::new();
    for job in
        crate::context_jobs::published_summaries(actor, &request.scope, &request.session_id).await?
    {
        let current = job.policy_revision == policy
            && job
                .chunk
                .sources
                .iter()
                .zip(&job.chunk.spans)
                .all(|(source, span)| {
                    history.visible_messages().contains(&source)
                        && history
                            .source_span(&source.id)
                            .as_ref()
                            .is_ok_and(|current| current == span)
                });
        if !current {
            gaps.push(format!("Historian publication {} is stale", job.id));
            continue;
        }
        if let Some(result) = job.result {
            published.push((job.chunk, result));
        }
    }
    let native_memory = crate::context_memory::rebuild(actor, &request.scope)
        .await
        .map_err(memory_error)?;
    let native_summaries = crate::development_service::current_summaries(
        &native_memory,
        &request.scope,
        &history,
        policy,
        now_ns,
    )
    .map_err(memory_error)?;
    for (id, reason) in native_summaries.omitted {
        gaps.push(format!("Native summary {id} omitted: {reason}"));
    }
    for (chunk, result) in native_summaries.summaries {
        if let Some((_, existing)) = published.iter().find(|(old, _)| old.digest == chunk.digest) {
            if existing != &result {
                gaps.push(format!(
                    "Native summary for {} conflicts with an existing publication",
                    chunk.digest
                ));
            }
        } else {
            published.push((chunk, result));
        }
    }
    let synced = crate::context_projection::sync_summaries(
        actor,
        &history,
        &initial.fence,
        published,
        LSN::new(initial.ledger_tail),
    )
    .await?;
    let projection = crate::context_projection::assemble(
        actor,
        &history,
        projection_request,
        LSN::new(synced.ledger_tail),
    )
    .await?;
    let source_messages: Vec<_> = history.visible_messages().into_iter().cloned().collect();
    let temporal_sources = source_messages
        .iter()
        .map(TemporalSource::from_message)
        .collect::<Result<Vec<_>, _>>()
        .map_err(map)?;
    let temporal = TemporalContext::build_with_offset(
        request.scope.clone(),
        request.session_id.clone(),
        request.utc_offset_seconds,
        temporal_sources.clone(),
        temporal_sources.into_iter().map(|s| s.source).collect(),
    )
    .map_err(map)?;
    let tokenizer = crate::context_tokenizer::counter_for_model(&request.model_id)?;
    let (evidence, evidence_gaps, evidence_omissions, retrieval_view) =
        retrieve(actor, binding, &history, &tokenizer, provider).await?;
    gaps.extend(evidence_gaps);
    for plan in &mut accepted_knowledge.plans {
        plan.generation = projection.generation;
        plan.digest.clear();
        plan.digest = digest_bytes(&serde_json::to_vec(plan).map_err(|_| invalid())?);
    }
    for cue in &mut accepted_knowledge.cues {
        cue.generation = projection.generation;
        cue.digest = cue.computed_digest().map_err(map)?;
    }
    let mut blocks = projection.blocks.clone();
    blocks.extend(
        accepted_knowledge
            .blocks
            .iter()
            .filter(|b| !projection.blocks.iter().any(|old| old.id == b.id))
            .cloned(),
    );
    for block in evidence {
        if !blocks.iter().any(|existing| existing.id == block.id) {
            blocks.push(block);
        }
    }
    let counter = |bytes: &[u8]| {
        tokenizer
            .count(bytes)
            .map(|n| n as u64)
            .map_err(|_| ContextError::Unavailable("tokenizer failed".into()))
    };
    let source_spans: BTreeMap<_, _> = history
        .messages()
        .iter()
        .map(|m| Ok((m.id.clone(), history.source_span(&m.id)?)))
        .collect::<Result<_, ContextError>>()
        .map_err(map)?;
    let mut rendered = render_context(
        RenderRequest {
            scope: request.scope.clone(),
            session_id: request.session_id.clone(),
            cursor: history.cursor(),
            generation: projection.generation,
            messages: &projection.messages,
            source_spans: &source_spans,
            blocks: &blocks,
            required_message_ids: &required,
            budget: request.budget,
            profile: request.profile,
        },
        &counter,
    )
    .map_err(|error| match error {
        hm_context::provider::RenderError::RequiredOverflow { .. } => {
            Error::new(ErrorCode::CapacityExceeded)
        }
        _ => invalid(),
    })?;
    rendered.report.omitted.extend(evidence_omissions);
    rendered.report.gaps = gaps.clone();
    rendered.report.digest.clear();
    rendered.report.digest =
        digest_bytes(&serde_json::to_vec(&rendered.report).map_err(|_| invalid())?);
    let migrations = crate::hypermid_import::list_import_receipts(actor, &request.scope, 64)
        .await
        .map_err(import_error)?;
    let final_tail = LSN::new(projection.ledger_tail);
    crate::context_projection::validate_tail(actor, final_tail).await?;
    let final_history = crate::context_history::replay(
        actor,
        &request.scope,
        &request.session_id,
        &binding.conversation,
    )
    .await
    .map_err(history_error)?;
    let final_projection =
        crate::context_projection::current(actor, &request.scope, &request.session_id)
            .await?
            .ok_or_else(invalid)?;
    if final_projection.fence != projection.fence
        || final_projection.generation != projection.generation
        || final_projection.bytes != projection.bytes
    {
        return Err(Error::new(ErrorCode::SequenceViolation));
    }
    let (_, _, final_memory_fence) = memory_materialization(
        actor,
        &request.scope,
        &memory_scopes,
        &facts,
        runtime_now_ns()?,
    )
    .await?;
    if final_memory_fence != memory_fence {
        return Err(Error::new(ErrorCode::SequenceViolation));
    }
    validate_accepted_knowledge(actor, &accepted_knowledge).await?;
    let final_jobs =
        crate::context_jobs::inspect_state(actor, &request.scope, &request.scope.owner_id).await?;
    if final_history.history.export_canonical().map_err(map)?
        != history.export_canonical().map_err(map)?
        || final_jobs["permission_revision"] != jobs["permission_revision"]
        || final_jobs["policies"] != jobs["policies"]
        || final_jobs["historian"] != jobs["historian"]
    {
        return Err(Error::new(ErrorCode::SequenceViolation));
    }
    crate::context_projection::validate_tail(actor, final_tail).await?;
    let spans = history
        .messages()
        .iter()
        .map(|m| history.source_span(&m.id))
        .collect::<Result<Vec<_>, _>>()
        .map_err(map)?;
    Ok(
        json!({"version":CONTRACT_VERSION,"session_id":request.session_id,"ledger_tail":final_tail.get(),"report":rendered.report,"messages":rendered.messages,"history":{"message_count":source_messages.len(),"cursor":history.cursor(),"parent":history.parent(),"relations":history.relations(),"source_spans":spans},"coverage":temporal,"cache":{"available":true,"generation":projection.generation,"fence":projection.fence,"replayed":projection.replayed,"pending_reductions":projection.pending_reductions,"coverage":projection.coverage,"materialization_digest":digest_bytes(&projection.bytes)},"jobs":{"available":true,"state":jobs},"memory":{"available":true,"scopes":memory_views},"retrieval":retrieval_view,"knowledge":{"available":true,"observed_use":"not_recorded_by_assembly","accepted":accepted_knowledge},"provenance":uris,"migrations":{"available":true,"recent_receipts":migrations},"gaps":gaps}),
    )
}

async fn retrieve(
    actor: &ActorEngine,
    binding: &Binding,
    history: &SourceHistory,
    tokenizer: &crate::context_tokenizer::BoundTokenCounter,
    provider: Option<&crate::context_retrieval::EmbeddingProvider>,
) -> Result<(Vec<ContextBlock>, Vec<String>, Vec<Omission>, Value), Error> {
    let r = &binding.request;
    if r.query.is_empty() {
        return Ok((
            Vec::new(),
            Vec::new(),
            Vec::new(),
            json!({"available":true,"results":[],"semantic":{"state":"unavailable","reason":"No retrieval query supplied"}}),
        ));
    }
    let now_ns = runtime_now_ns()?;
    let request = hm_context::retrieval::RetrievalRequest {
        scope: r.scope.clone(),
        now_ns,
        grants: r.evidence_grants.clone(),
        visible: history
            .visible_messages()
            .iter()
            .map(|m| history.source_span(&m.id))
            .collect::<Result<Vec<_>, _>>()
            .map_err(map)?,
        time_filter: None,
        query_vector: None,
        max_candidates: 100_000,
        max_results: 1000,
        max_tokens: r.budget.available().map_err(map)?,
    };
    let retrieved = crate::context_retrieval::retrieve(
        actor, &r.scope, &r.query, &request, tokenizer.native_counter(), provider,
    )
    .await
    .map_err(retrieval_error)?;
    tokenizer.ensure_current()?;
    let mut gaps = Vec::new();
    if let hm_context::retrieval::SemanticStatus::Unavailable { reason } = &retrieved.semantic {
        gaps.push(format!("Semantic retrieval unavailable: {reason}"));
    }
    let items: Vec<_> = retrieved
        .results
        .iter()
        .map(|r| hm_context::reduction::ReductionItem::original(r.context_block()))
        .collect();
    let plan = hm_context::reduction::select(
        &items,
        r.budget,
        hm_context::reduction::ReductionPolicy { half_life: 100 },
    )
    .map_err(|error| match error {
        hm_context::reduction::ReductionError::Context(error) => map(error),
        _ => Error::new(ErrorCode::CapacityExceeded),
    })?;
    let diagnostics =
        serde_json::to_value(&retrieved).map_err(|_| Error::new(ErrorCode::SchemaInvalid))?;
    let mut omissions = retrieved.omitted;
    omissions.extend(plan.omitted);
    Ok((plan.blocks, gaps, omissions, diagnostics))
}
pub fn memory_error(error: crate::context_memory::MemoryError) -> Error {
    match error {
        crate::context_memory::MemoryError::Ledger(error) => error,
        crate::context_memory::MemoryError::Context(error) => context_error(error),
    }
}
pub fn runtime_now_ns() -> Result<i64, Error> {
    i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| Error::new(ErrorCode::OperationUnavailable))?
            .as_nanos(),
    )
    .map_err(|_| Error::new(ErrorCode::CapacityExceeded))
}

async fn memory_materialization(
    actor: &ActorEngine,
    principal: &Scope,
    scopes: &[Scope],
    facts: &BTreeMap<String, String>,
    now_ns: i64,
) -> Result<(Vec<ContextBlock>, Vec<Value>, String), Error> {
    let mut blocks = Vec::new();
    let mut views = Vec::new();
    let mut fences = Vec::new();
    for scope in scopes {
        scope.validate().map_err(map)?;
        let memory = crate::context_memory::rebuild(actor, scope)
            .await
            .map_err(memory_error)?;
        let mut visible = Vec::new();
        let mut origins = Vec::new();
        let mut required_records = Vec::new();
        for record in memory
            .records
            .values()
            .filter(|r| r.status == crate::context_memory::RecordStatus::Active)
        {
            let record = match memory.read_with_facts(principal, &record.id, now_ns, facts) {
                Ok(Some(record)) => record,
                Ok(None) => continue,
                Err(crate::context_memory::MemoryError::Context(ContextError::ScopeMismatch)) => {
                    continue;
                }
                Err(error) => return Err(memory_error(error)),
            };
            visible.push((record.id.clone(), record.revision_digest.clone()));
            origins.push(json!({"scope":scope,"record_id":record.id,"revision_digest":record.revision_digest,"authority":record.authority,"provenance":record.provenance}));
            if !record.pinned
                && !matches!(
                    record.kind,
                    crate::context_memory::RecordKind::Note
                        | crate::context_memory::RecordKind::ConditionalNote
                        | crate::context_memory::RecordKind::Anchor
                        | crate::context_memory::RecordKind::Primer
                )
            {
                continue;
            }
            required_records.push((record.id.clone(), record.revision_digest.clone()));
            let mut provenance: Vec<_> = record
                .provenance
                .iter()
                .map(|p| SourceSpan {
                    source_id: p.source_id.clone(),
                    source_digest: p.source_digest.clone(),
                    byte_start: p.span_start,
                    byte_end: p.span_end,
                })
                .collect();
            if provenance.is_empty() {
                provenance.push(SourceSpan {
                    source_id: format!(
                        "memory-source:{}:{}",
                        digest_bytes(record.id.as_bytes()),
                        record.revision
                    ),
                    source_digest: digest_bytes(record.content.as_bytes()),
                    byte_start: 0,
                    byte_end: record.content.len() as u64,
                });
            }
            let id = if scope == principal {
                format!("memory:{}", digest_bytes(record.id.as_bytes()))
            } else {
                format!(
                    "memory:{}:{}",
                    scope.digest().map_err(map)?,
                    digest_bytes(record.id.as_bytes())
                )
            };
            blocks.push(ContextBlock {
                id,
                text: record.content.clone(),
                authority: record.authority,
                provenance,
                tokens: 0,
                required: true,
            });
        }
        fences.push(json!({"scope":scope,"required_records":required_records}));
        views.push(
            json!({"scope":scope,"cursor":memory.cursor,"visible":visible,"provenance":origins}),
        );
    }
    let fence = digest_bytes(
        &serde_json::to_vec(&fences).map_err(|_| Error::new(ErrorCode::SchemaInvalid))?,
    );
    Ok((blocks, views, fence))
}
pub async fn export_memory(actor: &ActorEngine, scope: &Scope) -> Result<Value, Error> {
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let tail = actor.stats().await?.applied.last_lsn;
    let export = crate::context_memory::export(actor, scope, scope)
        .await
        .map_err(memory_error)?;
    let bytes = export.to_jsonl().map_err(memory_error)?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err(Error::new(ErrorCode::CapacityExceeded));
    }
    crate::context_projection::validate_tail(actor, tail).await?;
    Ok(json!({
        "version": 1,
        "scope": export.scope,
        "cursor": export.cursor,
        "media_type": "application/x-ndjson",
        "restore_max_bytes": 512 * 1024,
        "byte_count": bytes.len(),
        "artifact_digest": hm_context::digest_bytes(&bytes),
        "export_digest": export.digest,
        "bytes": bytes,
    }))
}

pub async fn inspect_memory(
    actor: &ActorEngine,
    principal: &Scope,
    owner: &Scope,
    id: Option<&str>,
    source: Option<&str>,
) -> Result<Value, Error> {
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let tail = actor.stats().await?.applied.last_lsn;
    let memory = crate::context_memory::rebuild(actor, owner)
        .await
        .map_err(memory_error)?;
    let now = runtime_now_ns()?;
    let facts: BTreeMap<String, String> = [
        ("owner_id".into(), principal.owner_id.clone()),
        ("project_id".into(), principal.project_id.clone()),
    ]
    .into_iter()
    .collect();
    let value = if let Some(id) = id {
        let record = memory
            .read_with_facts(principal, id, now, &facts)
            .map_err(memory_error)?
            .ok_or_else(|| Error::new(ErrorCode::OperationUnavailable))?;
        if let Some(source_id) = source {
            if !record.provenance.iter().any(|p| p.source_id == source_id) {
                return Err(Error::new(ErrorCode::CapabilityDenied));
            }
            let source = memory
                .sources
                .get(source_id)
                .filter(|s| !s.tombstoned)
                .ok_or_else(|| Error::new(ErrorCode::OperationUnavailable))?;
            json!({"version":1,"scope":owner,"record_id":id,"source":source})
        } else {
            json!({"version":1,"scope":owner,"record":record,"cursor":memory.cursor})
        }
    } else {
        let records = memory
            .records
            .values()
            .filter(|r| r.status == crate::context_memory::RecordStatus::Active)
            .filter_map(
                |record| match memory.read_with_facts(principal, &record.id, now, &facts) {
                    Ok(Some(r)) => Some(Ok(r)),
                    Ok(None) => None,
                    Err(crate::context_memory::MemoryError::Context(
                        ContextError::ScopeMismatch,
                    )) => None,
                    Err(e) => Some(Err(memory_error(e))),
                },
            )
            .collect::<Result<Vec<_>, Error>>()?;
        json!({"version":1,"scope":owner,"principal":principal,"cursor":memory.cursor,"records":records})
    };
    crate::context_projection::validate_tail(actor, tail).await?;
    Ok(value)
}

async fn validate_accepted_knowledge(
    actor: &ActorEngine,
    accepted: &AcceptedKnowledge,
) -> Result<(), Error> {
    for plan in &accepted.plans {
        for selected in &plan.selected {
            let record = crate::knowledge_injection::exact_recall(
                actor,
                &plan.scope,
                &plan.principal,
                &selected.id,
            )
            .await
            .map_err(memory_error)?
            .ok_or_else(|| Error::new(ErrorCode::SequenceViolation))?;
            if record.revision_digest != selected.revision_digest
                || record.status != crate::context_memory::RecordStatus::Active
            {
                return Err(Error::new(ErrorCode::SequenceViolation));
            }
            let memory = crate::context_memory::rebuild(actor, &plan.scope)
                .await
                .map_err(memory_error)?;
            for provenance in &record.provenance {
                let source = memory
                    .sources
                    .get(&provenance.source_id)
                    .ok_or_else(invalid)?;
                if let Some(locator) = source.locator.strip_prefix("conversation:") {
                    let suffix = format!(":{}", source.id);
                    let session = locator.strip_suffix(&suffix).ok_or_else(invalid)?;
                    let conversations: std::collections::BTreeSet<_> = memory
                        .worker_capabilities
                        .values()
                        .filter(|cap| {
                            cap.scope == plan.scope
                                && cap.session_id == session
                                && cap.source_ids.contains(&source.id)
                        })
                        .map(|cap| cap.conversation.clone())
                        .collect();
                    if conversations.len() != 1 {
                        return Err(Error::new(ErrorCode::OperationUnavailable));
                    }
                    let conversation = conversations.iter().next().ok_or_else(invalid)?;
                    let original =
                        crate::context_history::replay(actor, &plan.scope, session, conversation)
                            .await
                            .map_err(history_error)?;
                    if !original
                        .history
                        .visible_messages()
                        .iter()
                        .any(|m| m.id == source.id)
                    {
                        return Err(Error::new(ErrorCode::SequenceViolation));
                    }
                    let span = original.history.source_span(&source.id).map_err(map)?;
                    if digest_bytes(&original.history.recover(&plan.scope, &span).map_err(map)?)
                        != source.digest
                    {
                        return Err(Error::new(ErrorCode::SequenceViolation));
                    }
                }
            }
        }
    }
    for cue in &accepted.cues {
        cue.validate().map_err(map)?;
    }
    Ok(())
}
async fn build_knowledge(
    actor: &ActorEngine,
    request: &SessionContextRequest,
    history: &SourceHistory,
    scopes: &[Scope],
    required: &[ContextBlock],
) -> Result<AcceptedKnowledge, Error> {
    use hm_context::{
        knowledge_injection::VisibleEvidence,
        memory_cues::{CueProfile, CueRecord, CueSnapshot, CueSource, build_cues},
    };
    let tokenizer = crate::context_tokenizer::counter_for_model(&request.model_id)?;
    let counter = |bytes: &[u8]| {
        tokenizer
            .count(bytes)
            .map(|n| n as u64)
            .map_err(|_| ContextError::Unavailable("knowledge tokenizer".into()))
    };
    let mut accepted = AcceptedKnowledge {
        plans: vec![],
        cues: vec![],
        blocks: vec![],
        policy_digest: String::new(),
    };
    let protected_blocks = required
        .iter()
        .try_fold(0u64, |sum, block| {
            counter(&serde_json::to_vec(block)?)
                .and_then(|n| sum.checked_add(n).ok_or(ContextError::Capacity))
        })
        .map_err(map)?;
    let mut protected_messages = 0u64;
    for message in history.visible_messages() {
        if request.required_message_ids.contains(&message.id)
            || history
                .visible_messages()
                .last()
                .is_some_and(|last| last.id == message.id)
        {
            protected_messages = protected_messages
                .checked_add(
                    counter(&serde_json::to_vec(message).map_err(|_| invalid())?).map_err(map)?,
                )
                .ok_or_else(invalid)?;
        }
    }
    let mut remaining = request
        .budget
        .available()
        .map_err(map)?
        .saturating_sub(protected_blocks.saturating_add(protected_messages))
        / 4;
    let visible = VisibleEvidence {
        records: vec![],
        source_spans: history
            .visible_messages()
            .iter()
            .map(|m| history.source_span(&m.id))
            .collect::<Result<Vec<_>, _>>()
            .map_err(map)?,
    };
    for scope in scopes {
        let memory = crate::context_memory::rebuild(actor, scope)
            .await
            .map_err(memory_error)?;
        let mut frozen = crate::knowledge_injection::snapshot(
            actor,
            scope,
            &request.scope,
            &request.session_id,
            1,
        )
        .await
        .map_err(memory_error)?;
        frozen.records.retain(|r| {
            memory.records.get(&r.id).is_some_and(|record| {
                matches!(
                    record.kind,
                    crate::context_memory::RecordKind::Fact
                        | crate::context_memory::RecordKind::Episode
                ) && !record.pinned
            })
        });
        frozen.seal().map_err(map)?;
        let plan = hm_context::knowledge_injection::select_knowledge(
            &frozen, remaining, &visible, &counter,
        )
        .map_err(map)?;
        let mut cue_records = vec![];
        for block in &plan.blocks {
            let record = &memory.records[&block.id];
            if record.metadata.get("context_cue").and_then(Value::as_bool) == Some(true) {
                let mut sources = vec![];
                for p in &record.provenance {
                    let source = memory.sources.get(&p.source_id).ok_or_else(invalid)?;
                    let bytes = usize::try_from(p.span_start)
                        .ok()
                        .zip(usize::try_from(p.span_end).ok())
                        .and_then(|(start, end)| source.content.get(start..end))
                        .ok_or_else(invalid)?;
                    if digest_bytes(bytes) != p.quoted_digest
                        || digest_bytes(&source.content) != p.source_digest
                    {
                        return Err(Error::new(ErrorCode::SequenceViolation));
                    }
                    sources.push(CueSource {
                        id: p.source_id.clone(),
                        digest: p.source_digest.clone(),
                        start: p.span_start,
                        end: p.span_end,
                        quoted_digest: p.quoted_digest.clone(),
                        bytes: bytes.to_vec(),
                    });
                }
                cue_records.push(CueRecord {
                    id: record.id.clone(),
                    revision: record.revision,
                    revision_digest: record.revision_digest.clone(),
                    content: record.content.clone(),
                    authority: record.authority,
                    sources,
                });
            } else {
                let mut block = block.clone();
                block.id = knowledge_block_id(scope, &record.id)?;
                if !required.iter().any(|r| r.id == block.id) {
                    accepted.blocks.push(block);
                }
            }
        }
        if !cue_records.is_empty() {
            let cue = build_cues(
                &CueSnapshot {
                    scope: scope.clone(),
                    principal: request.scope.clone(),
                    permission_digest: digest_bytes(
                        &serde_json::to_vec(&memory.grants).map_err(|_| invalid())?,
                    ),
                    records: cue_records,
                },
                CueProfile {
                    model_id: request.model_id.clone(),
                    max_cue_bytes: 256,
                    request_visual: false,
                },
                TokenBudget {
                    context_tokens: remaining.saturating_add(1),
                    reserved_output_tokens: 1,
                    required_tokens: 0,
                },
                1,
                &counter,
            )
            .map_err(map)?;
            for compact in &cue.cues {
                let original = plan
                    .blocks
                    .iter()
                    .find(|b| b.id == compact.record_id)
                    .ok_or_else(invalid)?;
                accepted.blocks.push(ContextBlock {
                    id: knowledge_block_id(scope, &compact.record_id)?,
                    text: format!(
                        "Memory cue{}; exact recall {}\n{}",
                        if compact.truncated {
                            " (truncated)"
                        } else {
                            ""
                        },
                        compact.record_id,
                        compact.text
                    ),
                    authority: compact.authority,
                    provenance: original.provenance.clone(),
                    tokens: 0,
                    required: false,
                });
            }
            accepted.cues.push(cue);
        }
        remaining = remaining.saturating_sub(plan.token_count);
        accepted.plans.push(plan);
    }
    accepted.policy_digest = digest_bytes(
        &serde_json::to_vec(&(&accepted.plans, &accepted.cues, &accepted.blocks))
            .map_err(|_| invalid())?,
    );
    Ok(accepted)
}
fn knowledge_block_id(scope: &Scope, id: &str) -> Result<String, Error> {
    Ok(format!(
        "knowledge:{}:{}",
        scope.digest().map_err(map)?,
        digest_bytes(id.as_bytes())
    ))
}

fn bind_knowledge_policy(
    request: &mut ProjectionRequest,
    accepted_digest: &str,
) -> Result<(), Error> {
    request.policy_revision = digest_bytes(
        &serde_json::to_vec(&(&request.policy_revision, accepted_digest)).map_err(|_| invalid())?,
    );
    Ok(())
}
pub async fn accepted_materialization_locked(
    actor: &ActorEngine,
    request: &SessionContextRequest,
    history: &SourceHistory,
) -> Result<RequiredMaterialization, Error> {
    let mut material = required_materialization_locked(actor, request, history).await?;
    let saved = bindings(actor).await?;
    let binding = saved.get(&request.session_id).ok_or_else(invalid)?;
    if binding.request.scope != request.scope
        || binding.request.model_id != request.model_id
        || binding.request.budget != request.budget
        || binding.request.memory_scopes != request.memory_scopes
    {
        return Err(Error::new(ErrorCode::SequenceViolation));
    }
    if let Some(accepted) = &binding.knowledge {
        validate_accepted_knowledge(actor, accepted).await?;
        bind_knowledge_policy(&mut material.projection_request, &accepted.policy_digest)?;
        for block in &accepted.blocks {
            if !material
                .blocks
                .iter()
                .any(|existing| existing.id == block.id)
            {
                material.blocks.push(block.clone());
            }
        }
    }
    Ok(material)
}
