use crate::Envelope;
use hm_core::{ConversationId, Error, ErrorCode};
use hm_ledger::frame::EventKind;
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    Authority, EventEnvelope, EventPayload, IntentSet, LoopCloseReason, LoopClosed, LoopOpened,
    Retention, Sensitivity,
};
use hm_serve::actor::{ActorEngine, IncomingEvent};
use rmcp::schemars;
use serde::Deserialize;
use serde_json::json;

#[derive(Clone, Copy, Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IntendCloseReason {
    Done,
    Abandoned,
    HandedOff,
    Superseded,
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum IntendAction {
    SetObjective {
        objective: String,
    },
    OpenLoop {
        loop_id: String,
        objective: String,
    },
    CloseLoop {
        loop_id: String,
        reason: IntendCloseReason,
        #[serde(default)]
        cause: String,
        #[serde(default)]
        evidence_lsns: Vec<u64>,
    },
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct IntendInput {
    pub conversation: String,
    pub action: IntendAction,
}

pub async fn run(actor: &ActorEngine, input: IntendInput) -> Result<Envelope, Error> {
    if input.conversation.is_empty() {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let (kind, payload, action, loop_id) = match input.action {
        IntendAction::SetObjective { objective } => {
            if objective.is_empty() {
                return Err(Error::new(ErrorCode::InvalidArgument));
            }
            (
                EventKind::IntentSet,
                EventPayload::IntentSet(Box::new(IntentSet {
                    objective: objective.into_bytes(),
                })),
                "set_objective",
                None,
            )
        }
        IntendAction::OpenLoop { loop_id, objective } => {
            if loop_id.is_empty() || objective.is_empty() {
                return Err(Error::new(ErrorCode::InvalidArgument));
            }
            let item_loop_id = loop_id.clone();
            (
                EventKind::LoopOpened,
                EventPayload::LoopOpened(Box::new(LoopOpened {
                    loop_id: loop_id.into_bytes(),
                    objective: objective.into_bytes(),
                })),
                "open_loop",
                Some(item_loop_id),
            )
        }
        IntendAction::CloseLoop {
            loop_id,
            reason,
            cause,
            evidence_lsns,
        } => {
            if loop_id.is_empty()
                || matches!(reason, IntendCloseReason::Done) && evidence_lsns.is_empty()
            {
                return Err(Error::new(ErrorCode::InvalidArgument));
            }
            let item_loop_id = loop_id.clone();
            let reason = match reason {
                IntendCloseReason::Done => LoopCloseReason::Done,
                IntendCloseReason::Abandoned => LoopCloseReason::Abandoned,
                IntendCloseReason::HandedOff => LoopCloseReason::HandedOff,
                IntendCloseReason::Superseded => LoopCloseReason::Superseded,
            };
            (
                EventKind::LoopClosed,
                EventPayload::LoopClosed(Box::new(LoopClosed {
                    loop_id: loop_id.into_bytes(),
                    reason,
                    cause: cause.into_bytes(),
                    evidence_lsns: (!evidence_lsns.is_empty()).then_some(evidence_lsns),
                })),
                "close_loop",
                Some(item_loop_id),
            )
        }
    };
    let conversation = ConversationId::derive(&input.conversation);
    let outcome = actor
        .append(vec![IncomingEvent {
            kind,
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
    let mut envelope = Envelope::empty();
    envelope.items.push(json!({
        "action": action,
        "loop_id": loop_id,
        "lsn": outcome.first_lsn.get(),
    }));
    envelope.provenance.push(uri);
    Ok(envelope)
}
