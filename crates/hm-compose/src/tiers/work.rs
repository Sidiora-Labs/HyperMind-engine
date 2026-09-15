#![allow(clippy::missing_errors_doc)]

use crate::bundle::{ActivationItem, Tier, WhyCode};
use crate::tokens::TokenCounter;
use hm_core::{ActorId, ConversationId, Error};
use hm_proj::ledger::{WorkKind, WorkLedgerProjection, WorkState};
use hm_proj::store::ReadSnapshot;

pub(crate) fn read(
    snapshot: &ReadSnapshot<'_>,
    actor: ActorId,
    conversation: ConversationId,
    counter: &TokenCounter,
) -> Result<Vec<ActivationItem>, Error> {
    WorkLedgerProjection::read_conversation(snapshot, conversation, 4096)?
        .into_iter()
        .filter(|item| item.requires_reconciliation)
        .map(|item| {
            let content = format!(
                "work {} call={} tool={} state={} requires_reconciliation=true",
                kind_name(item.kind),
                hex(&item.call_id),
                String::from_utf8_lossy(&item.tool_name),
                state_name(item.state)
            )
            .into_bytes();
            Ok(ActivationItem {
                tier: Tier::WorkLedger,
                uri: format!(
                    "hm://{actor}/{conversation}/{}?src=work&kind={}&state={}&why=work_ledger",
                    item.state_lsn,
                    kind_name(item.kind),
                    state_name(item.state)
                ),
                provenance: if item.state_lsn == item.tool_call_lsn {
                    vec![item.state_lsn]
                } else {
                    vec![item.tool_call_lsn, item.state_lsn]
                },
                tokens: counter.count(&content)?,
                content,
                coarsened: false,
                vector_rank: 0,
                lexical_rank: 0,
                why: WhyCode::WorkLedger,
            })
        })
        .collect()
}

const fn kind_name(kind: WorkKind) -> &'static str {
    match kind {
        WorkKind::ToolCall => "tool_call",
        WorkKind::Effect => "effect",
    }
}

const fn state_name(state: WorkState) -> &'static str {
    match state {
        WorkState::Dispatched => "dispatched",
        WorkState::Committed => "committed",
        WorkState::Returned => "returned",
        WorkState::OutcomeUnknown => "outcome_unknown",
    }
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
