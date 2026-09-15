use crate::Envelope;
use hm_core::{ConversationId, Error, ErrorCode};
use hm_ledger::frame::EventKind;
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{Authority, Binding, EventEnvelope, EventPayload, Retention, Sensitivity};
use hm_serve::actor::{ActorEngine, IncomingEvent};
use rmcp::schemars;
use serde::Deserialize;
use serde_json::json;

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct BindInput {
    pub conversation: String,
    #[serde(default)]
    pub task: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    pub canonical_entity: String,
    pub property: String,
    pub evidence_lsn: u64,
    pub revision: String,
    pub freshness_requirement_ns: u64,
}

pub async fn run(actor: &ActorEngine, input: BindInput) -> Result<Envelope, Error> {
    let has_task = input.task.as_ref().is_some_and(|value| !value.is_empty());
    let has_scope = input.scope.as_ref().is_some_and(|value| !value.is_empty());
    if input.conversation.is_empty()
        || has_task == has_scope
        || input.canonical_entity.is_empty()
        || input.property.is_empty()
        || input.evidence_lsn == 0
        || input.revision.is_empty()
        || input.freshness_requirement_ns == 0
    {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let target_kind = if has_task { "task" } else { "scope" };
    let target = input.task.as_ref().or(input.scope.as_ref()).cloned();
    let conversation = ConversationId::derive(&input.conversation);
    let payload = EventPayload::Binding(Box::new(Binding {
        task: input.task.map(String::into_bytes),
        scope: input.scope.map(String::into_bytes),
        canonical_entity: input.canonical_entity.clone(),
        property: input.property.clone(),
        evidence_lsn: input.evidence_lsn,
        revision: input.revision.clone().into_bytes(),
        freshness_requirement_ns: input.freshness_requirement_ns,
    }));
    let outcome = actor
        .append(vec![IncomingEvent {
            kind: EventKind::Binding,
            conversation,
            payload: encode_event_envelope(&EventEnvelope {
                schema_version: CURRENT_SCHEMA_VERSION,
                payload,
                connection_id: None,
                client_seq: 0,
                client_event_index: 0,
                client_event_count: 1,
                origin_actor: 0,
                run_id: None,
                model_provenance: None,
                authority: Authority::UserAsserted,
                retention: Retention::Durable,
                sensitivity: Sensitivity::Personal,
                event_time_ns: 0,
            }),
        }])
        .await?;
    let uri = format!("hm://{}/lsn/{}", actor.actor(), outcome.first_lsn.get());
    let evidence_uri = format!("hm://{}/lsn/{}", actor.actor(), input.evidence_lsn);
    let mut envelope = Envelope::empty();
    envelope.items.push(json!({
        "status": "resolved",
        "target_kind": target_kind,
        "target": target,
        "canonical_entity": input.canonical_entity,
        "property": input.property,
        "revision": input.revision,
        "evidence_lsn": input.evidence_lsn,
        "lsn": outcome.first_lsn.get(),
    }));
    envelope.provenance.extend([uri, evidence_uri]);
    Ok(envelope)
}
