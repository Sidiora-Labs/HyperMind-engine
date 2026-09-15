#![allow(clippy::missing_errors_doc)]

use crate::bundle::{ActivationItem, Tier, WhyCode};
use crate::tokens::TokenCounter;
use hm_core::{ActorId, ConversationId, Error, LSN};
use hm_proj::beliefs::{BeliefProjection, BeliefRecord};
use hm_proj::store::ReadSnapshot;
use hm_schema::events::BeliefType;
use std::collections::BTreeSet;

pub fn read(
    snapshot: &ReadSnapshot<'_>,
    actor: ActorId,
    conversation: ConversationId,
    counter: &TokenCounter,
) -> Result<Vec<ActivationItem>, Error> {
    BeliefProjection::read_heads(snapshot, 4096)?
        .into_iter()
        .filter(resident)
        .map(|record| item(actor, conversation, &record, counter))
        .collect()
}

fn resident(record: &BeliefRecord) -> bool {
    matches!(
        record.belief_type,
        BeliefType::Identity | BeliefType::Constraint | BeliefType::Preference | BeliefType::Goal
    ) && !excluded_domain(&record.conflict_domain)
}

fn excluded_domain(domain: &str) -> bool {
    matches!(domain, "structural-self" | "structural_self" | "failure")
        || domain.starts_with("structural-self:")
        || domain.starts_with("structural_self:")
        || domain.starts_with("failure:")
}

fn item(
    actor: ActorId,
    conversation: ConversationId,
    record: &BeliefRecord,
    counter: &TokenCounter,
) -> Result<ActivationItem, Error> {
    let mut content =
        format!("{:?} {} = ", record.belief_type, record.canonical_identity).into_bytes();
    content.extend_from_slice(&record.value);
    let provenance = provenance(record);
    let lsn = LSN::new(record.observation_lsn);
    Ok(ActivationItem {
        tier: Tier::Resident,
        uri: format!(
            "hm://{actor}/{conversation}/{lsn}?src=belief&at=known:{}&why=resident",
            record.observation_lsn
        ),
        provenance,
        tokens: counter.count(&content)?,
        content,
        authority: record.authority,
        coarsened: false,
        vector_rank: 0,
        lexical_rank: 0,
        why: WhyCode::Belief,
    })
}

pub(crate) fn provenance(record: &BeliefRecord) -> Vec<LSN> {
    record
        .provenance
        .iter()
        .flat_map(|range| [range.first_lsn, range.last_lsn])
        .filter(|lsn| *lsn != 0)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(LSN::new)
        .collect()
}
