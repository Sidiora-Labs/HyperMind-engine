use crate::{
    actor::{ActorEngine, IncomingEvent},
    context_projection, session_context,
};
use hm_context::{
    continuity_pressure::*,
    history::SourceRelation,
    reduction::{ReductionItem, ReductionPolicy},
    source_continuity::*,
    types::*,
};
use hm_core::{ConversationId, Error, ErrorCode, LSN};
use hm_schema::{
    event::{self, CURRENT_SCHEMA_VERSION},
    events::{EventEnvelope, EventPayload, ProviderFrame, Retention, Sensitivity},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
const PROVIDER: &str = "hypermind/continuity-service/v1";
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinuityMode {
    #[default]
    Off,
    PassThrough,
    Shadow,
    Primary,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModePolicy {
    pub revision: u64,
    pub mode: ContinuityMode,
    pub strict: bool,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HookTicket {
    pub fence: HookFence,
    pub source: ContinuityFence,
    pub policy: ModePolicy,
    pub budget: TokenBudget,
    pub required_ids: Vec<String>,
    pub rendered_digest: String,
    pub trace_id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum ContinuityAction {
    Configure {
        mode: ContinuityMode,
        strict: bool,
        expected_revision: u64,
    },
    Pressure {
        budget: TokenBudget,
        required_ids: Vec<String>,
    },
    PreHook {
        budget: TokenBudget,
        required_ids: Vec<String>,
    },
    PostHook {
        ticket: HookTicket,
    },
    CancelHook {
        trace_id: String,
    },
    PlanEdit {
        relation: SourceRelation,
    },
    PlanFork {
        child_session_id: String,
        limits: ContributionLimits,
    },
    Inspect,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContinuityRequest {
    pub version: u32,
    pub scope: Scope,
    pub session_id: String,
    pub request_id: String,
    pub expected_generation: u64,
    pub action: ContinuityAction,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct SavedReceipt {
    digest: String,
    value: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
struct State {
    version: u32,
    scope: Scope,
    session_id: String,
    sequence: u64,
    policy: ModePolicy,
    active: Option<HookTicket>,
    receipts: BTreeMap<String, SavedReceipt>,
}
fn error(code: ErrorCode) -> Error {
    Error::new(code)
}
fn context(e: ContextError) -> Error {
    session_context::context_error(e)
}
async fn load(actor: &ActorEngine, scope: &Scope, session: &str) -> Result<State, Error> {
    let mut state = State {
        version: 1,
        scope: scope.clone(),
        session_id: session.into(),
        sequence: 0,
        policy: ModePolicy {
            revision: 0,
            mode: ContinuityMode::Off,
            strict: false,
        },
        active: None,
        receipts: BTreeMap::new(),
    };
    for frame in actor
        .frames_since(
            LSN::new(0),
            Some(ConversationId::derive(PROVIDER)),
            usize::MAX,
        )
        .await?
    {
        let verified = actor.verified_event(frame.header.lsn).await?;
        let EventPayload::ProviderFrame(p) = verified.envelope.payload else {
            continue;
        };
        if p.provider != PROVIDER {
            continue;
        }
        let next: State =
            serde_json::from_slice(&p.api_content).map_err(|_| error(ErrorCode::SchemaInvalid))?;
        if next.scope != *scope || next.session_id != session {
            continue;
        }
        if next.version != 1 || next.sequence != state.sequence + 1 {
            return Err(error(ErrorCode::SequenceViolation));
        }
        state = next;
    }
    Ok(state)
}
async fn append(actor: &ActorEngine, tail: LSN, state: &mut State) -> Result<(), Error> {
    state.sequence = state
        .sequence
        .checked_add(1)
        .ok_or_else(|| error(ErrorCode::CapacityExceeded))?;
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
                        api_content: serde_json::to_vec(state)
                            .map_err(|_| error(ErrorCode::SchemaInvalid))?,
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
struct Snapshot {
    request: session_context::SessionContextRequest,
    unsupported_parts: bool,
    history: hm_context::history::SourceHistory,
    material: session_context::RequiredMaterialization,
    projection: Option<context_projection::ProjectionView>,
    summaries: Vec<(
        hm_context::historian::SourceChunk,
        hm_context::historian::HistorianResult,
    )>,
}
async fn snapshot(
    actor: &ActorEngine,
    scope: &Scope,
    session: &str,
    expected_generation: u64,
) -> Result<Snapshot, Error> {
    let (request, conversation) = session_context::session_request(actor, scope, session).await?;
    let ledger = crate::context_history::replay(actor, scope, session, &conversation)
        .await
        .map_err(session_context::history_error)?;
    let unsupported_parts = !ledger.unsupported_parts.is_empty();
    let material =
        session_context::accepted_materialization_locked(actor, &request, &ledger.history).await?;
    let mut projection = context_projection::current(actor, scope, session).await?;
    if let Some(view) = &mut projection {
        for block in &material.blocks {
            if !view.blocks.iter().any(|existing| existing.id == block.id) {
                view.blocks.push(block.clone());
            }
        }
    }
    if projection.as_ref().map_or(0, |p| p.generation) != expected_generation {
        return Err(error(ErrorCode::SequenceViolation));
    }
    let memory = crate::context_memory::rebuild(actor, scope)
        .await
        .map_err(session_context::memory_error)?;
    let mut summaries = crate::development_service::current_summaries(
        &memory,
        scope,
        &ledger.history,
        material.policy,
        material.now_ns,
    )
    .map_err(session_context::memory_error)?
    .summaries;
    for job in crate::context_jobs::published_summaries(actor, scope, session).await? {
        if job.policy_revision == material.policy
            && job
                .chunk
                .sources
                .iter()
                .all(|source| ledger.history.visible_messages().contains(&source))
            && job.chunk.spans.iter().all(|span| {
                ledger
                    .history
                    .source_span(&span.source_id)
                    .is_ok_and(|current| current == *span)
            })
        {
            if let Some(result) = job.result {
                summaries.push((job.chunk, result));
            }
        }
    }
    Ok(Snapshot {
        request,
        unsupported_parts,
        history: ledger.history,
        material,
        projection,
        summaries,
    })
}
fn pressure_error(e: PressureError) -> Error {
    match e {
        PressureError::Context(e) => context(e),
        PressureError::Stale => error(ErrorCode::SequenceViolation),
        PressureError::Reduction(_) => error(ErrorCode::CapacityExceeded),
        PressureError::Render(hm_context::provider::RenderError::Context(e)) => context(e),
        PressureError::Render(hm_context::provider::RenderError::RequiredOverflow { .. }) => {
            error(ErrorCode::CapacityExceeded)
        }
        PressureError::Render(_) => error(ErrorCode::OperationUnavailable),
    }
}
fn pressure(
    s: &Snapshot,
    policy: &ModePolicy,
    budget: TokenBudget,
    required_ids: &[String],
) -> Result<(PressureSnapshot, PressurePlan), Error> {
    if s.unsupported_parts {
        return Err(error(ErrorCode::OperationUnavailable));
    }
    if budget.context_tokens > s.request.budget.context_tokens
        || budget.reserved_output_tokens < s.request.budget.reserved_output_tokens
        || budget.required_tokens < s.request.budget.required_tokens
    {
        return Err(error(ErrorCode::CapacityExceeded));
    }
    budget.available().map_err(context)?;
    let view = s
        .projection
        .as_ref()
        .ok_or_else(|| error(ErrorCode::OperationUnavailable))?;
    let fresh = context_projection::fence_for(&s.history, &s.material.projection_request)
        .map_err(context)?;
    let verified = view.fence == fresh;
    if policy.mode == ContinuityMode::Primary && !verified {
        return Err(error(ErrorCode::SequenceViolation));
    }
    let tokenizer = crate::context_tokenizer::counter_for_model(&s.request.model_id)?;
    let counter = |bytes: &[u8]| {
        tokenizer
            .count(bytes)
            .map(|n| n as u64)
            .map_err(|_| ContextError::Unavailable("tokenizer".into()))
    };
    let messages: Vec<_> = if verified {
        view.messages.clone()
    } else {
        s.history.visible_messages().into_iter().cloned().collect()
    };
    let source_spans: BTreeMap<_, _> = s
        .history
        .messages()
        .iter()
        .map(|message| Ok((message.id.clone(), s.history.source_span(&message.id)?)))
        .collect::<Result<_, ContextError>>()
        .map_err(context)?;
    let mut items = Vec::new();
    for message in &messages {
        let text =
            serde_json::to_string(&message.parts).map_err(|_| error(ErrorCode::SchemaInvalid))?;
        let mut item = ReductionItem::original(ContextBlock {
            id: message.id.clone(),
            tokens: counter(text.as_bytes()).map_err(context)?,
            text,
            authority: message.authority,
            provenance: vec![source_spans[&message.id].clone()],
            required: s.material.required_ids.contains(&message.id),
        });
        item.age = s.history.cursor().sequence.saturating_sub(message.ordinal);
        items.push(item);
    }
    let mut blocks = if verified {
        view.blocks.clone()
    } else {
        s.material.blocks.clone()
    };
    for block in &mut blocks {
        block.tokens = counter(block.text.as_bytes()).map_err(context)?;
        let mut item = ReductionItem::original(block.clone());
        if !block.required {
            if let Some((_, result)) = s
                .summaries
                .iter()
                .find(|(chunk, _)| chunk.spans == block.provenance)
            {
                for (index, tier) in result.tiers.iter().enumerate() {
                    let tokens = counter(tier.text.as_bytes()).map_err(context)?;
                    if tokens <= block.tokens {
                        item.summaries[index] = Some(ContextBlock {
                            text: tier.text.clone(),
                            tokens,
                            ..block.clone()
                        });
                    }
                }
            }
        }
        items.push(item);
    }
    let mut required = s.material.required_ids.clone();
    for id in required_ids {
        if !required.contains(id) {
            required.push(id.clone())
        }
    }
    let profile_digest = digest_bytes(
        &serde_json::to_vec(&s.request.profile).map_err(|_| error(ErrorCode::SchemaInvalid))?,
    );
    let snap = PressureSnapshot {
        fence: HookFence {
            cache: fresh,
            generation: view.generation,
            grant_revision: s.material.permission_revision.clone(),
            profile_revision: profile_digest,
        },
        cursor: s.history.cursor(),
        messages,
        source_spans,
        items,
        policy: ReductionPolicy::default(),
    };
    let plan = plan_pressure(&snap, &required, budget, s.request.profile, &counter)
        .map_err(pressure_error)?;
    if policy.strict && !plan.reduction.omitted.is_empty() {
        return Err(error(ErrorCode::CapacityExceeded));
    }
    Ok((snap, plan))
}
pub async fn execute(
    actor: &ActorEngine,
    trusted_scope: &Scope,
    owner: &Scope,
    request: ContinuityRequest,
) -> Result<Value, Error> {
    if owner != trusted_scope || request.scope != *trusted_scope {
        return Err(error(ErrorCode::CapabilityDenied));
    }
    if request.version != 1 {
        return Err(error(ErrorCode::ProtocolVersion));
    }
    trusted_scope.validate().map_err(context)?;
    validate_id(&request.session_id).map_err(context)?;
    validate_id(&request.request_id).map_err(context)?;
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let tail = actor.stats().await?.applied.last_lsn;
    let mut state = load(actor, trusted_scope, &request.session_id).await?;
    let digest =
        digest_bytes(&serde_json::to_vec(&request).map_err(|_| error(ErrorCode::SchemaInvalid))?);
    if let Some(receipt) = state.receipts.get(&request.request_id) {
        if receipt.digest != digest {
            return Err(error(ErrorCode::IdempotencyConflict));
        }
        if matches!(&request.action, ContinuityAction::PreHook { .. }) {
            let ticket: HookTicket = serde_json::from_value(receipt.value["ticket"].clone())
                .map_err(|_| error(ErrorCode::SchemaInvalid))?;
            let current = snapshot(
                actor,
                trusted_scope,
                &request.session_id,
                request.expected_generation,
            )
            .await?;
            ticket.source.validate(&current.history).map_err(context)?;
            if state.active.as_ref() != Some(&ticket) || state.policy != ticket.policy {
                return Err(error(ErrorCode::SequenceViolation));
            }
            let (_, plan) = pressure(&current, &state.policy, ticket.budget, &ticket.required_ids)?;
            if plan.fence != ticket.fence || plan.rendered.digest != ticket.rendered_digest {
                return Err(error(ErrorCode::SequenceViolation));
            }
            let mut value = receipt.value.clone();
            value["plan"] =
                serde_json::to_value(plan).map_err(|_| error(ErrorCode::SchemaInvalid))?;
            context_projection::validate_tail(actor, tail).await?;
            return Ok(value);
        }
        context_projection::validate_tail(actor, tail).await?;
        return Ok(receipt.value.clone());
    }
    if matches!(
        &request.action,
        ContinuityAction::Pressure { .. } | ContinuityAction::PreHook { .. }
    ) && matches!(
        state.policy.mode,
        ContinuityMode::Off | ContinuityMode::PassThrough
    ) {
        session_context::session_request(actor, trusted_scope, &request.session_id).await?;
        let generation = context_projection::current(actor, trusted_scope, &request.session_id)
            .await?
            .map_or(0, |view| view.generation);
        if generation != request.expected_generation {
            return Err(error(ErrorCode::SequenceViolation));
        }
        context_projection::validate_tail(actor, tail).await?;
        return Ok(
            json!({"mode":state.policy.mode,"strict":state.policy.strict,"provider_input_changed":false,"published":false,"plan":null}),
        );
    }
    let s = snapshot(
        actor,
        trusted_scope,
        &request.session_id,
        request.expected_generation,
    )
    .await?;
    let is_pre = matches!(&request.action, ContinuityAction::PreHook { .. });
    let mut publish = false;
    let value = match request.action {
        ContinuityAction::Inspect => {
            json!({"version":1,"scope":trusted_scope,"session_id":request.session_id,"policy":state.policy,"generation":s.projection.as_ref().map_or(0,|v|v.generation),"cursor":s.history.cursor(),"unsupported_parts":s.unsupported_parts,"active_hook":state.active,"receipts":state.receipts,"sequence":state.sequence})
        }
        ContinuityAction::Configure {
            mode,
            strict,
            expected_revision,
        } => {
            if state.active.is_some() || state.policy.revision != expected_revision {
                return Err(error(ErrorCode::SequenceViolation));
            }
            state.policy = ModePolicy {
                revision: expected_revision
                    .checked_add(1)
                    .ok_or_else(|| error(ErrorCode::CapacityExceeded))?,
                mode,
                strict,
            };
            publish = true;
            json!({"policy":state.policy,"turn_boundary":true})
        }
        ContinuityAction::PlanEdit { relation } => {
            serde_json::to_value(plan_edit(&s.history, relation).map_err(context)?)
                .map_err(|_| error(ErrorCode::SchemaInvalid))?
        }
        ContinuityAction::PlanFork {
            child_session_id,
            limits,
        } => {
            serde_json::to_value(plan_fork(&s.history, &child_session_id, limits).map_err(context)?)
                .map_err(|_| error(ErrorCode::SchemaInvalid))?
        }
        ContinuityAction::Pressure {
            budget,
            required_ids,
        }
        | ContinuityAction::PreHook {
            budget,
            required_ids,
        } => {
            if matches!(
                state.policy.mode,
                ContinuityMode::Off | ContinuityMode::PassThrough
            ) {
                json!({"mode":state.policy.mode,"strict":state.policy.strict,"provider_input_changed":false,"published":false,"plan":null})
            } else {
                let (snap, plan) = pressure(&s, &state.policy, budget, &required_ids)?;
                let pre = pre_hook_fence(&snap).map_err(pressure_error)?;
                let ticket = HookTicket {
                    fence: pre,
                    source: ContinuityFence::capture(&s.history).map_err(context)?,
                    policy: state.policy.clone(),
                    budget,
                    required_ids,
                    rendered_digest: plan.rendered.digest.clone(),
                    trace_id: request.request_id.clone(),
                };
                if is_pre && state.policy.mode == ContinuityMode::Primary {
                    if state.active.is_some() {
                        return Err(error(ErrorCode::SequenceViolation));
                    }
                    state.active = Some(ticket.clone());
                    publish = true;
                }
                json!({"mode":state.policy.mode,"strict":state.policy.strict,"published":publish,"plan":plan,"ticket":ticket})
            }
        }
        ContinuityAction::PostHook { ticket } => {
            ticket.source.validate(&s.history).map_err(context)?;
            if ticket.policy != state.policy {
                return Err(error(ErrorCode::SequenceViolation));
            }
            let (snap, plan) = pressure(&s, &state.policy, ticket.budget, &ticket.required_ids)?;
            if ticket.rendered_digest != plan.rendered.digest {
                return Err(error(ErrorCode::SequenceViolation));
            }
            let receipt = post_hook_receipt(
                &ticket.fence,
                &snap.fence,
                &plan,
                s.request.profile,
                ticket.budget,
            )
            .map_err(pressure_error)?;
            if state.policy.mode == ContinuityMode::Primary {
                if state.active.as_ref() != Some(&ticket) {
                    return Err(error(ErrorCode::SequenceViolation));
                }
                state.active = None;
                publish = true;
            }
            json!({"receipt":receipt,"advisory":true,"mode":state.policy.mode})
        }
        ContinuityAction::CancelHook { trace_id } => {
            if state
                .active
                .as_ref()
                .is_none_or(|ticket| ticket.trace_id != trace_id)
            {
                return Err(error(ErrorCode::SequenceViolation));
            }
            state.active = None;
            publish = true;
            json!({"cancelled":true,"trace_id":trace_id})
        }
    };
    context_projection::validate_tail(actor, tail).await?;
    if publish {
        if state.receipts.len() >= 256 {
            return Err(error(ErrorCode::CapacityExceeded));
        }
        let mut saved_value = value.clone();
        if is_pre {
            if let Some(object) = saved_value.as_object_mut() {
                object.remove("plan");
            }
        }
        state.receipts.insert(
            request.request_id,
            SavedReceipt {
                digest,
                value: saved_value,
            },
        );
        append(actor, tail, &mut state).await?;
    }
    Ok(value)
}

pub async fn policy(
    actor: &ActorEngine,
    trusted_scope: &Scope,
    owner: &Scope,
    session: &str,
) -> Result<ModePolicy, Error> {
    if owner != trusted_scope {
        return Err(error(ErrorCode::CapabilityDenied));
    }
    trusted_scope.validate().map_err(context)?;
    validate_id(session).map_err(context)?;
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let tail = actor.stats().await?.applied.last_lsn;
    session_context::session_request(actor, trusted_scope, session).await?;
    let state = load(actor, trusted_scope, session).await?;
    context_projection::validate_tail(actor, tail).await?;
    Ok(state.policy)
}
pub async fn inspect(
    actor: &ActorEngine,
    trusted_scope: &Scope,
    owner: &Scope,
    session: &str,
) -> Result<Value, Error> {
    if owner != trusted_scope {
        return Err(error(ErrorCode::CapabilityDenied));
    }
    let generation = context_projection::current(actor, trusted_scope, session)
        .await?
        .map_or(0, |view| view.generation);
    execute(
        actor,
        trusted_scope,
        owner,
        ContinuityRequest {
            version: 1,
            scope: trusted_scope.clone(),
            session_id: session.into(),
            request_id: "continuity-inspect".into(),
            expected_generation: generation,
            action: ContinuityAction::Inspect,
        },
    )
    .await
}
