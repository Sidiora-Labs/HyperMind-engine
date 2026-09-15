#![allow(clippy::missing_errors_doc)]

use crate::bundle::{ActivationItem, Tier, WhyCode};
use crate::tokens::TokenCounter;
use hm_core::{ActorId, ConversationId, Error, LSN, UtcNanos};
use hm_proj::ladder::{TemporalLadder, TemporalLevel, TemporalWindow};
use hm_proj::store::ReadSnapshot;
use hm_schema::events::Authority;

const WEEK_NS: i64 = 604_800_000_000_000;
const WEEKS_TO_SURFACE: i64 = 12;

pub fn read(
    snapshot: &ReadSnapshot<'_>,
    actor: ActorId,
    conversation: ConversationId,
    now_ns: UtcNanos,
    counter: &TokenCounter,
) -> Result<Vec<ActivationItem>, Error> {
    let from_ns = now_ns
        .get()
        .saturating_sub(WEEK_NS.saturating_mul(WEEKS_TO_SURFACE));
    let to_ns = now_ns.get().saturating_add(1);
    TemporalLadder::list_windows(snapshot, TemporalLevel::Week, from_ns, to_ns, 64)?
        .into_iter()
        .map(|window| item(snapshot, actor, conversation, window, counter))
        .collect()
}

fn item(
    snapshot: &ReadSnapshot<'_>,
    actor: ActorId,
    conversation: ConversationId,
    window: TemporalWindow,
    counter: &TokenCounter,
) -> Result<ActivationItem, Error> {
    let members = TemporalLadder::resolve_members(snapshot, window, 4096)?;
    let content = format!(
        "week {}..{} contains {} events",
        window.start_ns, window.end_ns, window.member_count
    )
    .into_bytes();
    let anchor = members.first().copied().unwrap_or(LSN::new(0));
    Ok(ActivationItem {
        tier: Tier::Temporal,
        uri: format!(
            "hm://{actor}/{conversation}/{anchor}?src=temporal&at={}..{}&why=window",
            window.start_ns, window.end_ns
        ),
        provenance: members,
        tokens: counter.count(&content)?,
        content,
        authority: Authority::RuntimeFact,
        coarsened: false,
        vector_rank: 0,
        lexical_rank: 0,
        why: WhyCode::Temporal,
    })
}
