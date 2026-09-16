#![allow(clippy::missing_errors_doc)]

use crate::bundle::{ActivationItem, Tier, WhyCode};
use crate::tokens::TokenCounter;
use hm_core::{ActorId, ConversationId, Error, LSN, UtcNanos};
use hm_proj::attention::{AttentionProjection, MAXIMUM_ATTENTION_HISTORY};
use hm_proj::intentions::{IntentionStatus, IntentionsProjection};
use hm_proj::store::ReadSnapshot;
use hm_proj::timeline::read_conversation_record;
use hm_schema::events::{AttentionDecision, Authority};
use std::collections::BTreeSet;

pub fn read(
    snapshot: &ReadSnapshot<'_>,
    actor: ActorId,
    conversation: ConversationId,
    now_ns: UtcNanos,
    counter: &TokenCounter,
) -> Result<Vec<ActivationItem>, Error> {
    let mut items = Vec::new();
    let mut seen = BTreeSet::new();
    let mut batch_lines = Vec::new();
    let mut batch_provenance = BTreeSet::new();
    for decision in AttentionProjection::recent(snapshot, MAXIMUM_ATTENTION_HISTORY)? {
        if !seen.insert((decision.intention_id.clone(), decision.wake_id.clone())) {
            continue;
        }
        let Some(intention) = IntentionsProjection::get(snapshot, &decision.intention_id)? else {
            continue;
        };
        if intention.status != IntentionStatus::Fired
            || intention.wake_id.as_deref() != Some(decision.wake_id.as_slice())
            || decision.lsn <= intention.status_lsn
            || (intention.expires_at_ns != 0 && intention.expires_at_ns <= now_ns.get())
        {
            continue;
        }
        let Some(source) = read_conversation_record(snapshot, LSN::new(intention.set_lsn))? else {
            continue;
        };
        if source.conversation != conversation {
            continue;
        }
        let provenance = [
            intention.set_lsn,
            intention.trigger_lsn,
            intention.status_lsn,
            decision.lsn,
        ]
        .into_iter()
        .map(LSN::new)
        .collect::<BTreeSet<_>>();
        let objective = String::from_utf8_lossy(&intention.objective);
        let action = match decision.decision {
            AttentionDecision::Notify => "notify",
            AttentionDecision::AskUser => "ask_user",
            AttentionDecision::StartWork => "start_work",
            AttentionDecision::Batch => {
                batch_lines.push(format!("{objective} — {}", decision.reason));
                batch_provenance.extend(provenance);
                continue;
            }
            AttentionDecision::Ignore
            | AttentionDecision::Remember
            | AttentionDecision::Schedule => {
                continue;
            }
        };
        items.push(item(
            actor,
            conversation,
            decision.lsn,
            format!("{action}: {objective}\nReason: {}", decision.reason),
            provenance,
            counter,
        )?);
    }
    if !batch_lines.is_empty() {
        let anchor = batch_provenance
            .iter()
            .next_back()
            .map_or(0, |lsn| lsn.get());
        items.push(item(
            actor,
            conversation,
            anchor,
            format!(
                "BATCH DIGEST ({} deferred items)\n{}",
                batch_lines.len(),
                batch_lines.join("\n")
            ),
            batch_provenance,
            counter,
        )?);
    }
    Ok(items)
}

fn item(
    actor: ActorId,
    conversation: ConversationId,
    anchor: u64,
    content: String,
    provenance: BTreeSet<LSN>,
    counter: &TokenCounter,
) -> Result<ActivationItem, Error> {
    let content = content.into_bytes();
    Ok(ActivationItem {
        tier: Tier::Prospective,
        uri: format!("hm://{actor}/{conversation}/{anchor}?src=attention&why=prospective"),
        provenance: provenance.into_iter().collect(),
        tokens: counter.count(&content)?,
        content,
        authority: Authority::DerivedInference,
        coarsened: false,
        vector_rank: 0,
        lexical_rank: 0,
        why: WhyCode::Prospective,
    })
}
