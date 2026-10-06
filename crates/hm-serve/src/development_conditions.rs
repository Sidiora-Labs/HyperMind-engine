use crate::{
    actor::{ActorEngine, IncomingEvent},
    context_memory::{self, MemoryError, RecordKind, RecordStatus},
};
use hm_context::{Authority, ContextError, Scope, digest_bytes};
use hm_core::{ConversationId, LSN};
use hm_cortex::development_conditions::{
    AuthorizedFactReader, ConditionEvidence, ConditionLimits, ConditionOutcome,
};
use hm_ledger::frame::EventKind;
use hm_schema::{
    event::{CURRENT_SCHEMA_VERSION, encode_event_envelope},
    events::{EventEnvelope, EventPayload, ProviderFrame, Retention, Sensitivity},
};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
const PROVIDER: &str = "hypermind/deferred-note/v1";
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceFence {
    pub id: String,
    pub digest: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NoteReadiness {
    pub scope: Scope,
    pub note_id: String,
    pub revision: u64,
    pub revision_digest: String,
    pub evidence: ConditionEvidence,
    pub sources: Vec<SourceFence>,
    pub occurrence_id: Option<String>,
    pub delivered: bool,
    pub ledger_lsn: u64,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NoteDelivery {
    pub occurrence_id: String,
    pub note_id: String,
    pub text: String,
    pub authority: Authority,
    pub ledger_lsn: u64,
    pub replayed: bool,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum EventState {
    Evaluated,
    Delivered,
    Invalidated,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct Event {
    version: u32,
    id: String,
    scope: Scope,
    state: EventState,
    readiness: NoteReadiness,
    delivery: Option<NoteDelivery>,
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
async fn events(actor: &ActorEngine, scope: &Scope) -> Result<Vec<(u64, Event)>, MemoryError> {
    let mut output = Vec::new();
    for frame in actor.frames_since(LSN::new(0), None, usize::MAX).await? {
        if frame.header.kind != EventKind::ProviderFrame {
            continue;
        };
        let event = actor.verified_event(frame.header.lsn).await?;
        if let EventPayload::ProviderFrame(provider) = event.envelope.payload {
            if provider.provider == PROVIDER {
                let event: Event = serde_json::from_slice(&provider.api_content)?;
                if event.version != 1 || event.scope != *scope {
                    return Err(ContextError::ScopeMismatch.into());
                };
                output.push((frame.header.lsn.get(), event))
            }
        }
    }
    Ok(output)
}
async fn observe(
    actor: &ActorEngine,
    scope: &Scope,
    reader: &AuthorizedFactReader,
    note_id: &str,
    limits: ConditionLimits,
) -> Result<(NoteReadiness, String, Authority), MemoryError> {
    if reader.scope() != scope {
        return Err(ContextError::ScopeMismatch.into());
    };
    let state = context_memory::rebuild(actor, scope).await?;
    let record = state
        .read(scope, note_id, now()?)?
        .ok_or_else(|| ContextError::Unavailable("conditional note".into()))?;
    if record.kind != RecordKind::ConditionalNote || record.status != RecordStatus::Active {
        return Err(
            ContextError::Invalid("record is not an active conditional note".into()).into(),
        );
    }
    let predicate = record
        .predicate
        .clone()
        .ok_or_else(|| ContextError::Unavailable("note predicate".into()))?;
    let mut sources = Vec::new();
    for provenance in &record.provenance {
        let source = state
            .sources
            .get(&provenance.source_id)
            .filter(|s| !s.tombstoned)
            .ok_or(ContextError::Stale)?;
        if source.digest != provenance.source_digest
            || source.digest != digest_bytes(&source.content)
        {
            return Err(ContextError::Stale.into());
        };
        sources.push(SourceFence {
            id: source.id.clone(),
            digest: source.digest.clone(),
        })
    }
    sources.sort_by(|a, b| a.id.cmp(&b.id));
    sources.dedup();
    let evaluator = reader.clone();
    let expected = scope.clone();
    let evidence =
        tokio::task::spawn_blocking(move || evaluator.evaluate(&predicate, &expected, limits))
            .await
            .map_err(|_| ContextError::Unavailable("condition evaluator".into()))??;
    let occurrence_id = if evidence.outcome == ConditionOutcome::True {
        Some(digest_bytes(&serde_json::to_vec(&(
            scope,
            &record.id,
            record.revision,
            &record.revision_digest,
            &evidence.fingerprint,
            &sources,
        ))?))
    } else {
        None
    };
    Ok((
        NoteReadiness {
            scope: scope.clone(),
            note_id: record.id.clone(),
            revision: record.revision,
            revision_digest: record.revision_digest.clone(),
            evidence,
            sources,
            occurrence_id,
            delivered: false,
            ledger_lsn: 0,
        },
        record.content.clone(),
        record.authority,
    ))
}
async fn append(
    actor: &ActorEngine,
    scope: &Scope,
    event: &Event,
    tail: LSN,
) -> Result<u64, MemoryError> {
    let content = serde_json::to_vec(event)?;
    if content.len() > 512 * 1024 {
        return Err(ContextError::Capacity.into());
    };
    let payload = encode_event_envelope(&EventEnvelope {
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
        event_time_ns: now()?,
    });
    let receipt = actor
        .append_if_tail(
            tail,
            vec![IncomingEvent {
                kind: EventKind::ProviderFrame,
                conversation: ConversationId::derive(&format!("deferred-note:{}", scope.digest()?)),
                payload,
            }],
        )
        .await?;
    Ok(receipt.last_lsn.get())
}
pub async fn evaluate_note(
    actor: &ActorEngine,
    scope: &Scope,
    reader: &AuthorizedFactReader,
    note_id: &str,
    limits: ConditionLimits,
) -> Result<NoteReadiness, MemoryError> {
    scope.validate()?;
    hm_context::validate_id(note_id)?;
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let tail = actor.stats().await?.applied.last_lsn;
    let (mut readiness, _, _) = observe(actor, scope, reader, note_id, limits).await?;
    let history = events(actor, scope).await?;
    readiness.delivered = readiness.occurrence_id.as_ref().is_some_and(|id| {
        history.iter().any(|(_, e)| {
            e.state == EventState::Delivered
                && e.delivery.as_ref().is_some_and(|d| &d.occurrence_id == id)
        })
    });
    let id = digest_bytes(&serde_json::to_vec(&("evaluation", &readiness))?);
    if let Some((lsn, _)) = history.iter().find(|(_, event)| event.id == id) {
        readiness.ledger_lsn = *lsn;
        return Ok(readiness);
    }
    let event = Event {
        version: 1,
        id,
        scope: scope.clone(),
        state: EventState::Evaluated,
        readiness: readiness.clone(),
        delivery: None,
    };
    readiness.ledger_lsn = append(actor, scope, &event, tail).await?;
    Ok(readiness)
}
pub async fn deliver_once(
    actor: &ActorEngine,
    scope: &Scope,
    reader: &AuthorizedFactReader,
    expected: &NoteReadiness,
    limits: ConditionLimits,
) -> Result<NoteDelivery, MemoryError> {
    if &expected.scope != scope
        || reader.scope() != scope
        || expected.evidence.outcome != ConditionOutcome::True
    {
        return Err(ContextError::ScopeMismatch.into());
    };
    let occurrence = expected.occurrence_id.as_ref().ok_or(ContextError::Stale)?;
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let tail = actor.stats().await?.applied.last_lsn;
    let history = events(actor, scope).await?;
    if let Some((lsn, event)) = history.iter().find(|(_, e)| {
        e.state == EventState::Delivered
            && e.delivery
                .as_ref()
                .is_some_and(|d| &d.occurrence_id == occurrence)
    }) {
        let mut receipt = event.delivery.clone().unwrap();
        receipt.ledger_lsn = *lsn;
        receipt.replayed = true;
        return Ok(receipt);
    }
    if !history.iter().any(|(lsn, e)| {
        *lsn == expected.ledger_lsn
            && e.state == EventState::Evaluated
            && e.readiness.occurrence_id == expected.occurrence_id
            && e.readiness.evidence == expected.evidence
    }) {
        return Err(ContextError::Stale.into());
    }
    let (current, text, authority) =
        observe(actor, scope, reader, &expected.note_id, limits).await?;
    if current.revision_digest != expected.revision_digest
        || current.evidence != expected.evidence
        || current.sources != expected.sources
        || current.occurrence_id != expected.occurrence_id
    {
        let event = Event {
            version: 1,
            id: digest_bytes(&serde_json::to_vec(&("invalidated", occurrence, &current))?),
            scope: scope.clone(),
            state: EventState::Invalidated,
            readiness: current,
            delivery: None,
        };
        if !history.iter().any(|(_, e)| e.id == event.id) {
            append(actor, scope, &event, tail).await?;
        };
        return Err(ContextError::Stale.into());
    }
    let mut receipt = NoteDelivery {
        occurrence_id: occurrence.clone(),
        note_id: expected.note_id.clone(),
        text,
        authority,
        ledger_lsn: 0,
        replayed: false,
    };
    let event = Event {
        version: 1,
        id: occurrence.clone(),
        scope: scope.clone(),
        state: EventState::Delivered,
        readiness: current,
        delivery: Some(receipt.clone()),
    };
    receipt.ledger_lsn = append(actor, scope, &event, tail).await?;
    Ok(receipt)
}
