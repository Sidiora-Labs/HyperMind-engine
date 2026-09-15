#![allow(clippy::missing_errors_doc)]

use crate::bundle::{ActivationItem, Tier, WhyCode};
use crate::tokens::TokenCounter;
use hm_core::{ActorId, ConversationId, Error};
use hm_proj::intent::IntentFrameProjection;
use hm_proj::store::ReadSnapshot;

pub(crate) struct IntentTier {
    pub items: Vec<ActivationItem>,
    pub active_task: Option<Vec<u8>>,
}

pub(crate) fn read(
    snapshot: &ReadSnapshot<'_>,
    actor: ActorId,
    conversation: ConversationId,
    counter: &TokenCounter,
) -> Result<IntentTier, Error> {
    let frame = IntentFrameProjection::read(snapshot, conversation)?;
    let active_task = frame.open_loops.last().map(|item| item.loop_id.clone());
    let mut items =
        Vec::with_capacity(frame.open_loops.len() + usize::from(frame.objective.is_some()));
    if let Some(objective) = frame.objective {
        let mut content = b"objective ".to_vec();
        content.extend_from_slice(&objective.content);
        items.push(item(
            actor,
            conversation,
            objective.set_lsn,
            "objective",
            content,
            counter,
        )?);
    }
    for open_loop in frame.open_loops {
        let mut content = b"open_loop ".to_vec();
        content.extend_from_slice(hex(&open_loop.loop_id).as_bytes());
        content.push(b' ');
        content.extend_from_slice(&open_loop.objective);
        items.push(item(
            actor,
            conversation,
            open_loop.opened_lsn,
            "loop",
            content,
            counter,
        )?);
    }
    Ok(IntentTier { items, active_task })
}

fn item(
    actor: ActorId,
    conversation: ConversationId,
    lsn: hm_core::LSN,
    kind: &str,
    content: Vec<u8>,
    counter: &TokenCounter,
) -> Result<ActivationItem, Error> {
    Ok(ActivationItem {
        tier: Tier::Intent,
        uri: format!("hm://{actor}/{conversation}/{lsn}?src=intent&kind={kind}&why=intent"),
        provenance: vec![lsn],
        tokens: counter.count(&content)?,
        content,
        coarsened: false,
        vector_rank: 0,
        lexical_rank: 0,
        why: WhyCode::Intent,
    })
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    output
}
