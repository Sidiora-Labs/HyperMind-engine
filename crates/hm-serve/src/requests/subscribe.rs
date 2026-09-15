#![allow(clippy::missing_errors_doc)]

use crate::actor::ActorEngine;
use hm_core::{ConversationId, Error, LSN};
use hm_ledger::frame::Frame;
use tokio::sync::broadcast;

pub struct SubscriptionStart {
    pub receiver: broadcast::Receiver<Frame>,
    pub replay: Vec<Frame>,
    pub replay_tail: LSN,
}

pub async fn start(
    actor: &ActorEngine,
    since_lsn: LSN,
    conversation: Option<ConversationId>,
    maximum_replay: usize,
) -> Result<SubscriptionStart, Error> {
    let receiver = actor.subscribe();
    let replay = actor
        .frames_since(since_lsn, conversation, maximum_replay)
        .await?;
    let replay_tail = replay.last().map_or(since_lsn, |frame| frame.header.lsn);
    Ok(SubscriptionStart {
        receiver,
        replay,
        replay_tail,
    })
}
