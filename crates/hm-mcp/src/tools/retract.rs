use crate::Envelope;
use crate::tools::believe::{ProvenanceInput, append};
use hm_core::{Error, ErrorCode};
use hm_ledger::frame::EventKind;
use hm_schema::events::{Authority, EventPayload, Retract};
use hm_serve::actor::ActorEngine;
use rmcp::schemars;
use serde::Deserialize;
use serde_json::json;

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct RetractInput {
    pub conversation: String,
    pub belief_id: String,
    pub provenance: Vec<ProvenanceInput>,
}

pub async fn run(actor: &ActorEngine, input: RetractInput) -> Result<Envelope, Error> {
    if input.conversation.is_empty() || input.belief_id.is_empty() {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let outcome = append(
        actor,
        &input.conversation,
        EventKind::Retract,
        EventPayload::Retract(Box::new(Retract {
            belief_id: input.belief_id.as_bytes().to_vec(),
            provenance: input.provenance.iter().map(Into::into).collect(),
        })),
        Authority::UserAsserted,
        None,
    )
    .await?;
    let mut envelope = Envelope::empty();
    envelope.items.push(json!({
        "belief_id": input.belief_id,
        "tombstoned": true,
        "lsn": outcome.first_lsn.get(),
    }));
    envelope.provenance.push(format!(
        "hm://{}/lsn/{}",
        actor.actor(),
        outcome.first_lsn.get()
    ));
    Ok(envelope)
}
