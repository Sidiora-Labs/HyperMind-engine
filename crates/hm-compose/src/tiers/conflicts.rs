#![allow(clippy::missing_errors_doc)]

use crate::bundle::{ActivationItem, Gap, GapKind, Tier, WhyCode};
use crate::tiers::resident::provenance;
use crate::tokens::TokenCounter;
use hm_core::{ActorId, ConversationId, Error, LSN};
use hm_proj::beliefs::{BeliefAsOf, BeliefProjection, BeliefRecord, PendingProposal};
use hm_proj::store::{ProjectionId, ReadSnapshot};
use std::collections::BTreeSet;

pub struct ConflictTier {
    pub items: Vec<ActivationItem>,
    pub gaps: Vec<Gap>,
}

pub fn read(
    snapshot: &ReadSnapshot<'_>,
    actor: ActorId,
    conversation: ConversationId,
    counter: &TokenCounter,
) -> Result<ConflictTier, Error> {
    let known_lsn = snapshot.checkpoint(ProjectionId::BeliefStore)?;
    let heads = BeliefProjection::read_heads(snapshot, 4096)?;
    let mut seen = BTreeSet::new();
    let mut records = Vec::new();
    for head in heads.iter().filter(|head| {
        head.conflict_edges
            .iter()
            .any(|edge| edge.obligated_surfacing)
    }) {
        push_unique(&mut records, &mut seen, head.clone());
        for edge in head
            .conflict_edges
            .iter()
            .filter(|edge| edge.obligated_surfacing)
        {
            if let Some(counterpart) = BeliefProjection::read_as_of(
                snapshot,
                edge.other_type,
                &edge.other_canonical_identity,
                BeliefAsOf::KnownAt(known_lsn),
            )?
            .record
            {
                push_unique(&mut records, &mut seen, counterpart);
            }
        }
    }
    let mut items = records
        .into_iter()
        .map(|record| belief_item(actor, conversation, &record, counter))
        .collect::<Result<Vec<_>, _>>()?;
    let pending = BeliefProjection::read_pending(snapshot, 4096)?;
    let gaps = pending
        .iter()
        .map(|proposal| Gap {
            kind: GapKind::PendingProtectedProposal,
            tier: Some(Tier::Conflicts),
            lane: None,
            detail: format!("pending protected proposal {}", proposal.canonical_identity),
        })
        .collect();
    items.extend(
        pending
            .into_iter()
            .map(|proposal| proposal_item(actor, conversation, &proposal, counter))
            .collect::<Result<Vec<_>, _>>()?,
    );
    Ok(ConflictTier { items, gaps })
}

fn push_unique(
    records: &mut Vec<BeliefRecord>,
    seen: &mut BTreeSet<Vec<u8>>,
    record: BeliefRecord,
) {
    if seen.insert(record.belief_id.clone()) {
        records.push(record);
    }
}

fn belief_item(
    actor: ActorId,
    conversation: ConversationId,
    record: &BeliefRecord,
    counter: &TokenCounter,
) -> Result<ActivationItem, Error> {
    let mut content = format!("conflict {} = ", record.canonical_identity).into_bytes();
    content.extend_from_slice(&record.value);
    let lsn = LSN::new(record.observation_lsn);
    Ok(ActivationItem {
        tier: Tier::Conflicts,
        uri: format!("hm://{actor}/{conversation}/{lsn}?src=belief&why=conflict"),
        provenance: provenance(record),
        tokens: counter.count(&content)?,
        content,
        authority: record.authority,
        coarsened: false,
        vector_rank: 0,
        lexical_rank: 0,
        why: WhyCode::Conflict,
    })
}

fn proposal_item(
    actor: ActorId,
    conversation: ConversationId,
    proposal: &PendingProposal,
    counter: &TokenCounter,
) -> Result<ActivationItem, Error> {
    let mut content = format!(
        "pending {:?} {} = ",
        proposal.belief_type, proposal.canonical_identity
    )
    .into_bytes();
    content.extend_from_slice(&proposal.value);
    let lsn = LSN::new(proposal.observation_lsn);
    let provenance = proposal
        .provenance
        .iter()
        .flat_map(|range| [range.first_lsn, range.last_lsn])
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(LSN::new)
        .collect();
    Ok(ActivationItem {
        tier: Tier::Conflicts,
        uri: format!("hm://{actor}/{conversation}/{lsn}?src=proposal&why=pending"),
        provenance,
        tokens: counter.count(&content)?,
        content,
        authority: proposal.authority,
        coarsened: false,
        vector_rank: 0,
        lexical_rank: 0,
        why: WhyCode::Conflict,
    })
}
