use crate::{
    actor::{ActorEngine, IncomingEvent},
    context_memory::{self, MemoryError, MemoryProjection, RecordKind, RecordStatus},
};
use hm_context::{ContextError, Scope, TokenBudget, digest_bytes, memory_cues::*, validate_id};
use hm_core::{ConversationId, LSN};
use hm_schema::{
    event::{self, Boundary, CURRENT_SCHEMA_VERSION},
    events::{EventEnvelope, EventPayload, ProviderFrame, Retention, Sensitivity},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    time::{SystemTime, UNIX_EPOCH},
};

const PROVIDER: &str = "hypermind/memory-cues/v1";
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CueRequest {
    pub version: u32,
    pub scope: Scope,
    pub cache_id: String,
    pub record_ids: BTreeSet<String>,
    pub profile: CueProfile,
    pub budget: TokenBudget,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Saved {
    scope: Scope,
    principal: Scope,
    request: CueRequest,
    plan: CuePlan,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CueView {
    pub plan: CuePlan,
    pub replayed: bool,
    pub ledger_tail: u64,
}
fn now() -> Result<i64, MemoryError> {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ContextError::Unavailable("clock".into()))?
            .as_nanos(),
    )
    .map_err(|_| ContextError::Capacity.into())
}
fn counter(model: &str) -> Result<hm_compose::tokens::TokenCounter, MemoryError> {
    let counter = hm_compose::tokens::TokenCounter::for_model(
        model,
        None,
        hm_compose::tokens::FallbackWeights::default(),
    )?;
    if matches!(counter, hm_compose::tokens::TokenCounter::Fallback { .. }) {
        return Err(ContextError::Unavailable("exact cue tokenizer unavailable".into()).into());
    }
    Ok(counter)
}
fn snapshot(
    memory: &MemoryProjection,
    principal: &Scope,
    ids: &BTreeSet<String>,
    now_ns: i64,
) -> Result<CueSnapshot, MemoryError> {
    if ids.len() > 256 {
        return Err(ContextError::Capacity.into());
    }
    let mut records = Vec::new();
    for id in ids {
        let record = memory
            .read(principal, id, now_ns)?
            .ok_or(ContextError::Stale)?;
        if record.status != RecordStatus::Active || record.kind == RecordKind::ConditionalNote {
            return Err(
                ContextError::Unavailable("record requires another recall policy".into()).into(),
            );
        }
        let mut sources = Vec::new();
        for p in &record.provenance {
            let source = memory
                .sources
                .get(&p.source_id)
                .ok_or(ContextError::Stale)?;
            let start = usize::try_from(p.span_start).map_err(|_| ContextError::Capacity)?;
            let end = usize::try_from(p.span_end).map_err(|_| ContextError::Capacity)?;
            let bytes = source.content.get(start..end).ok_or(ContextError::Stale)?;
            if source.tombstoned
                || source.digest != p.source_digest
                || source.digest != digest_bytes(&source.content)
                || p.quoted_digest != digest_bytes(bytes)
            {
                return Err(ContextError::Stale.into());
            }
            sources.push(CueSource {
                id: source.id.clone(),
                digest: source.digest.clone(),
                start: p.span_start,
                end: p.span_end,
                quoted_digest: p.quoted_digest.clone(),
                bytes: bytes.to_vec(),
            });
        }
        records.push(CueRecord {
            id: record.id.clone(),
            revision: record.revision,
            revision_digest: record.revision_digest.clone(),
            content: record.content.clone(),
            authority: record.authority,
            sources,
        });
    }
    let snapshot = CueSnapshot {
        scope: memory.scope.clone(),
        principal: principal.clone(),
        permission_digest: digest_bytes(&serde_json::to_vec(&memory.grants)?),
        records,
    };
    snapshot.digest()?;
    Ok(snapshot)
}
async fn fence_sources(
    actor: &ActorEngine,
    scope: &Scope,
    memory: &MemoryProjection,
    current: &CueSnapshot,
) -> Result<(), MemoryError> {
    let mut histories = std::collections::BTreeMap::new();
    for record in &current.records {
        for span in &record.sources {
            let source = memory.sources.get(&span.id).ok_or(ContextError::Stale)?;
            let Some(locator) = source.locator.strip_prefix("conversation:") else {
                continue;
            };
            let suffix = format!(":{}", source.id);
            let session = locator.strip_suffix(&suffix).ok_or(ContextError::Stale)?;
            let conversations: BTreeSet<_> = memory
                .worker_capabilities
                .values()
                .filter(|cap| {
                    cap.scope == *scope
                        && cap.session_id == session
                        && cap.source_ids.contains(&source.id)
                })
                .map(|cap| cap.conversation.clone())
                .collect();
            if conversations.len() != 1 {
                return Err(ContextError::Unavailable(
                    "conversation source binding unavailable".into(),
                )
                .into());
            }
            let conversation = conversations
                .into_iter()
                .next()
                .ok_or(ContextError::Stale)?;
            let key = (session.to_owned(), conversation.clone());
            if !histories.contains_key(&key) {
                let history = crate::context_history::replay(actor, scope, session, &conversation)
                    .await
                    .map_err(|error| match error {
                        crate::context_history::HistoryError::Context(error) => {
                            MemoryError::Context(error)
                        }
                        crate::context_history::HistoryError::Ledger(error) => {
                            MemoryError::Ledger(error)
                        }
                    })?;
                histories.insert(key.clone(), history);
            }
            let history = &histories[&key].history;
            if !history
                .visible_messages()
                .iter()
                .any(|message| message.id == source.id)
            {
                return Err(ContextError::Stale.into());
            }
            let original = history.recover(scope, &history.source_span(&source.id)?)?;
            if original != source.content || digest_bytes(&original) != source.digest {
                return Err(ContextError::Stale.into());
            }
        }
    }
    Ok(())
}
async fn load(
    actor: &ActorEngine,
    scope: &Scope,
    principal: &Scope,
    cache_id: &str,
) -> Result<Option<Saved>, MemoryError> {
    let mut saved: Option<Saved> = None;
    for frame in actor.frames_since(LSN::new(0), None, usize::MAX).await? {
        if frame.header.kind != hm_ledger::frame::EventKind::ProviderFrame {
            continue;
        }
        let parsed = event::verify_event(
            &frame.sealed_payload,
            event::EventKind::ProviderFrame,
            Boundary::Disk,
        )?;
        let EventPayload::ProviderFrame(p) = parsed.envelope.payload else {
            continue;
        };
        if p.provider != PROVIDER {
            continue;
        }
        let next: Saved = serde_json::from_slice(&p.api_content)?;
        if next.scope != *scope || next.principal != *principal || next.request.cache_id != cache_id
        {
            continue;
        }
        next.plan.validate()?;
        if next.request.scope != next.scope
            || next.request.version != 1
            || next.plan.profile != next.request.profile
            || next.plan.budget != next.request.budget
            || next.plan.generation != saved.as_ref().map_or(1, |s| s.plan.generation + 1)
        {
            return Err(ContextError::Stale.into());
        }
        saved = Some(next);
    }
    Ok(saved)
}
async fn stable(actor: &ActorEngine, tail: LSN) -> Result<(), MemoryError> {
    if actor.stats().await?.applied.last_lsn != tail {
        return Err(ContextError::Stale.into());
    }
    Ok(())
}
pub async fn assemble(
    actor: &ActorEngine,
    scope: &Scope,
    principal: &Scope,
    request: CueRequest,
) -> Result<CueView, MemoryError> {
    scope.validate()?;
    principal.validate()?;
    validate_id(&request.cache_id)?;
    if request.scope != *scope || request.version != 1 {
        return Err(ContextError::ScopeMismatch.into());
    }
    let tokenizer = counter(&request.profile.model_id)?;
    let count = |bytes: &[u8]| {
        tokenizer
            .count(bytes)
            .map(|n| n as u64)
            .map_err(|e| ContextError::Unavailable(format!("cue tokenizer: {e}")))
    };
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let tail = actor.stats().await?.applied.last_lsn;
    let memory = context_memory::rebuild(actor, scope).await?;
    let current = snapshot(&memory, principal, &request.record_ids, now()?)?;
    fence_sources(actor, scope, &memory, &current).await?;
    let saved = load(actor, scope, principal, &request.cache_id).await?;
    if let Some(saved) = &saved {
        if saved.request == request && saved.plan.snapshot_digest == current.digest()? {
            stable(actor, tail).await?;
            return Ok(CueView {
                plan: saved.plan.clone(),
                replayed: true,
                ledger_tail: tail.get(),
            });
        }
    }
    let generation = saved.as_ref().map_or(Ok(1), |s| {
        s.plan
            .generation
            .checked_add(1)
            .ok_or(ContextError::Capacity)
    })?;
    let plan = build_cues(
        &current,
        request.profile.clone(),
        request.budget,
        generation,
        &count,
    )?;
    let saved = Saved {
        scope: scope.clone(),
        principal: principal.clone(),
        request,
        plan: plan.clone(),
    };
    let appended = actor
        .append_if_tail(
            tail,
            vec![IncomingEvent {
                kind: hm_ledger::frame::EventKind::ProviderFrame,
                conversation: ConversationId::derive(PROVIDER),
                payload: event::encode_event_envelope(&EventEnvelope {
                    schema_version: CURRENT_SCHEMA_VERSION,
                    payload: EventPayload::ProviderFrame(Box::new(ProviderFrame {
                        provider: PROVIDER.into(),
                        api_content: serde_json::to_vec(&saved)?,
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
    Ok(CueView {
        plan,
        replayed: false,
        ledger_tail: appended.last_lsn.get(),
    })
}
pub async fn resolve_cue(
    actor: &ActorEngine,
    scope: &Scope,
    principal: &Scope,
    cache_id: &str,
    generation: u64,
    cue_id: &str,
    budget: TokenBudget,
) -> Result<CueExpansion, MemoryError> {
    scope.validate()?;
    principal.validate()?;
    validate_id(cache_id)?;
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let tail = actor.stats().await?.applied.last_lsn;
    let saved = load(actor, scope, principal, cache_id)
        .await?
        .ok_or(ContextError::Stale)?;
    if saved.plan.generation != generation {
        return Err(ContextError::Stale.into());
    }
    let memory = context_memory::rebuild(actor, scope).await?;
    let current = snapshot(&memory, principal, &saved.request.record_ids, now()?)?;
    fence_sources(actor, scope, &memory, &current).await?;
    let tokenizer = counter(&saved.request.profile.model_id)?;
    let count = |bytes: &[u8]| {
        tokenizer
            .count(bytes)
            .map(|n| n as u64)
            .map_err(|e| ContextError::Unavailable(format!("cue tokenizer: {e}")))
    };
    let expanded =
        hm_context::memory_cues::resolve_cue(&current, &saved.plan, cue_id, &count, budget)?;
    stable(actor, tail).await?;
    Ok(expanded)
}
