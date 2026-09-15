#![allow(clippy::missing_errors_doc)]

use crate::actor::{ActorEngine, CheckpointOutcome};
use hm_core::Error;
use hm_ledger::idempotency::ConnectionId;
use hm_proj::checkpoint::CheckpointRead;

pub async fn write(
    actor: &ActorEngine,
    connection_id: ConnectionId,
    client_seq: u64,
    turn_id: Vec<u8>,
    blob: Vec<u8>,
) -> Result<CheckpointOutcome, Error> {
    actor
        .write_checkpoint(connection_id, client_seq, turn_id, blob)
        .await
}

pub async fn latest(
    actor: &ActorEngine,
    turn_id: Vec<u8>,
) -> Result<Option<CheckpointRead>, Error> {
    actor.latest_checkpoint(turn_id).await
}
