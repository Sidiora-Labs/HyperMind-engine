use crate::{
    actor::{ActorEngine, IncomingEvent},
    context_memory::{self, MemoryError, MemoryRecord, RecordKind},
};
use hm_context::{
    ContextError, Scope, SourceSpan, digest_bytes,
    knowledge_injection::{
        KnowledgePlan, KnowledgeRecord, KnowledgeSnapshot, VisibleEvidence, VisibleRecord,
    },
    validate_id,
};
use hm_core::{ConversationId, LSN};
use hm_ledger::frame::EventKind;
use hm_schema::{
    event::{CURRENT_SCHEMA_VERSION, encode_event_envelope},
    events::{EventEnvelope, EventPayload, ProviderFrame, Retention, Sensitivity},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    time::{SystemTime, UNIX_EPOCH},
};
const PROVIDER: &str = "hypermind/knowledge-use/v1";
fn now() -> Result<i64, MemoryError> {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ContextError::Unavailable("clock".into()))?
            .as_nanos(),
    )
    .map_err(|_| ContextError::Capacity.into())
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeUse {
    pub version: u32,
    pub scope: Scope,
    pub principal: Scope,
    pub session_id: String,
    pub generation: u64,
    pub turn_id: String,
    pub plan_digest: String,
    pub records: Vec<VisibleRecord>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KnowledgeUseReceipt {
    pub ledger_lsn: u64,
    pub replayed: bool,
}
async fn uses(actor: &ActorEngine, scope: &Scope) -> Result<Vec<(u64, KnowledgeUse)>, MemoryError> {
    let mut output = vec![];
    for frame in actor.frames_since(LSN::new(0), None, usize::MAX).await? {
        if frame.header.kind != EventKind::ProviderFrame {
            continue;
        }
        let event = actor.verified_event(frame.header.lsn).await?;
        if let EventPayload::ProviderFrame(provider) = event.envelope.payload {
            if provider.provider == PROVIDER {
                let usage: KnowledgeUse = serde_json::from_slice(&provider.api_content)?;
                if usage.version != 1 {
                    return Err(ContextError::Invalid("knowledge use version".into()).into());
                }
                if usage.scope == *scope {
                    output.push((frame.header.lsn.get(), usage));
                }
            }
        }
    }
    Ok(output)
}
pub async fn snapshot(
    actor: &ActorEngine,
    scope: &Scope,
    principal: &Scope,
    session_id: &str,
    generation: u64,
) -> Result<KnowledgeSnapshot, MemoryError> {
    scope.validate()?;
    principal.validate()?;
    validate_id(session_id)?;
    let state = context_memory::rebuild(actor, scope).await?;
    let observed = uses(actor, scope).await?;
    let mut records = vec![];
    for record in state.visible_records(principal, now()?)? {
        if !matches!(
            record.kind,
            RecordKind::Fact
                | RecordKind::Note
                | RecordKind::Episode
                | RecordKind::Anchor
                | RecordKind::Primer
        ) {
            continue;
        }
        let observed_uses = observed
            .iter()
            .filter(|(_, u)| {
                u.records
                    .iter()
                    .any(|r| r.id == record.id && r.revision_digest == record.revision_digest)
            })
            .count() as u64;
        records.push(KnowledgeRecord {
            id: record.id.clone(),
            revision_digest: record.revision_digest.clone(),
            category: record.category.clone(),
            text: record.content.clone(),
            authority: record.authority,
            pinned: record.pinned,
            importance: record.importance,
            recorded_at_ns: record.recorded_at_ns,
            provenance: record
                .provenance
                .iter()
                .map(|p| SourceSpan {
                    source_id: p.source_id.clone(),
                    source_digest: p.source_digest.clone(),
                    byte_start: p.span_start,
                    byte_end: p.span_end,
                })
                .collect(),
            verbatim_provenance: record.provenance.len() == 1
                && record.provenance.iter().all(|p| {
                    state.sources.get(&p.source_id).is_some_and(|source| {
                        usize::try_from(p.span_start)
                            .ok()
                            .zip(usize::try_from(p.span_end).ok())
                            .and_then(|(start, end)| source.content.get(start..end))
                            .is_some_and(|bytes| {
                                bytes == record.content.as_bytes()
                                    && digest_bytes(bytes) == p.quoted_digest
                            })
                    })
                }),
            observed_uses,
        });
    }
    let authority_digest = digest_bytes(&serde_json::to_vec(&(
        &state.records,
        &state.sources,
        &state.grants,
    ))?);
    let mut snapshot = KnowledgeSnapshot {
        scope: scope.clone(),
        principal: principal.clone(),
        session_id: session_id.into(),
        generation,
        records,
        authority_digest,
        digest: String::new(),
    };
    snapshot.seal()?;
    Ok(snapshot)
}
pub async fn select(
    actor: &ActorEngine,
    scope: &Scope,
    principal: &Scope,
    session_id: &str,
    generation: u64,
    budget: u64,
    visible: &VisibleEvidence,
    model_id: &str,
) -> Result<KnowledgePlan, MemoryError> {
    let frozen = snapshot(actor, scope, principal, session_id, generation).await?;
    let tokenizer = hm_compose::tokens::TokenCounter::for_model(
        model_id,
        None,
        hm_compose::tokens::FallbackWeights::default(),
    )
    .map_err(|_| ContextError::Unavailable("knowledge tokenizer".into()))?;
    if matches!(tokenizer, hm_compose::tokens::TokenCounter::Fallback { .. }) {
        return Err(ContextError::Unavailable("exact knowledge tokenizer".into()).into());
    }
    let counter = |bytes: &[u8]| {
        tokenizer
            .count(bytes)
            .map(|n| n as u64)
            .map_err(|_| ContextError::Unavailable("knowledge tokenizer".into()))
    };
    Ok(hm_context::knowledge_injection::select_knowledge(
        &frozen, budget, visible, &counter,
    )?)
}
pub async fn validate_plan(
    actor: &ActorEngine,
    plan: &KnowledgePlan,
    current_generation: u64,
) -> Result<(), MemoryError> {
    let current = snapshot(
        actor,
        &plan.scope,
        &plan.principal,
        &plan.session_id,
        current_generation,
    )
    .await?;
    Ok(plan.validate(&current, current_generation)?)
}
pub async fn exact_recall(
    actor: &ActorEngine,
    scope: &Scope,
    principal: &Scope,
    id: &str,
) -> Result<Option<MemoryRecord>, MemoryError> {
    validate_id(id)?;
    Ok(context_memory::rebuild(actor, scope)
        .await?
        .read(principal, id, now()?)?
        .cloned())
}
pub async fn observe_use(
    actor: &ActorEngine,
    plan: &KnowledgePlan,
    current_generation: u64,
    turn_id: &str,
    consumed_ids: &BTreeSet<String>,
) -> Result<KnowledgeUseReceipt, MemoryError> {
    validate_id(turn_id)?;
    if consumed_ids.is_empty()
        || !consumed_ids
            .iter()
            .all(|id| plan.selected.iter().any(|r| r.id == *id))
    {
        return Err(ContextError::Invalid("observed knowledge selection".into()).into());
    }
    let observation = KnowledgeUse {
        version: 1,
        scope: plan.scope.clone(),
        principal: plan.principal.clone(),
        session_id: plan.session_id.clone(),
        generation: plan.generation,
        turn_id: turn_id.into(),
        plan_digest: plan.digest.clone(),
        records: plan
            .selected
            .iter()
            .filter(|r| consumed_ids.contains(&r.id))
            .cloned()
            .collect(),
    };
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let tail = actor.stats().await?.applied.last_lsn;
    let history = uses(actor, &plan.scope).await?;
    if let Some((lsn, previous)) = history.iter().find(|(_, u)| {
        u.session_id == observation.session_id
            && u.turn_id == observation.turn_id
            && u.principal == observation.principal
    }) {
        if previous != &observation {
            return Err(ContextError::Stale.into());
        }
        return Ok(KnowledgeUseReceipt {
            ledger_lsn: *lsn,
            replayed: true,
        });
    }
    validate_plan(actor, plan, current_generation).await?;
    let payload = encode_event_envelope(&EventEnvelope {
        schema_version: CURRENT_SCHEMA_VERSION,
        payload: EventPayload::ProviderFrame(Box::new(ProviderFrame {
            provider: PROVIDER.into(),
            api_content: serde_json::to_vec(&observation)?,
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
        event_time_ns: now()?,
    });
    let receipt = actor
        .append_if_tail(
            tail,
            vec![IncomingEvent {
                kind: EventKind::ProviderFrame,
                conversation: ConversationId::derive(&format!(
                    "knowledge-use:{}",
                    plan.scope.digest()?
                )),
                payload,
            }],
        )
        .await?;
    Ok(KnowledgeUseReceipt {
        ledger_lsn: receipt.last_lsn.get(),
        replayed: false,
    })
}
