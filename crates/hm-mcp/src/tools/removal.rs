#![allow(clippy::missing_errors_doc)]

use super::inspect::{InspectHistory, references};
use crate::Envelope;
use hm_core::{Error, ErrorCode, LSN};
use hm_schema::event::{self, Boundary};
use hm_schema::events::{AttestationDisposition, EventPayload};
use hm_serve::actor::ActorEngine;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const MAXIMUM_REMOVAL_FRAMES: usize = 65_536;
pub(crate) const MINIMUM_COMPONENT_RECORDS: usize = 3;
pub(crate) const MAXIMUM_REMOVAL_ITEMS: usize = 256;

const COVERAGE: &str = "This preview walks the evidence reference graph recorded in ledger frames; consolidation graph edges are generation scoped and are not part of it.";

#[derive(Default)]
struct AttestationCounts {
    used: u64,
    ignored: u64,
    helpful: u64,
    harmful: u64,
}

pub(crate) async fn run(actor: &ActorEngine, lsn: LSN) -> Result<Envelope, Error> {
    let frames = actor
        .frames_since(LSN::new(0), None, MAXIMUM_REMOVAL_FRAMES + 1)
        .await?;
    if frames.len() > MAXIMUM_REMOVAL_FRAMES {
        return Err(Error::new(ErrorCode::CapacityExceeded));
    }
    let target = lsn.get();
    let mut history = InspectHistory::default();
    let mut present: BTreeSet<u64> = BTreeSet::new();
    let mut adjacency: BTreeMap<u64, BTreeSet<u64>> = BTreeMap::new();
    let mut dependents: BTreeSet<u64> = BTreeSet::new();
    let mut counts = AttestationCounts::default();
    for frame in &frames {
        let kind = event::EventKind::try_from(frame.header.kind as u8)
            .map_err(|()| Error::new(ErrorCode::InvalidKind).at_lsn(frame.header.lsn))?;
        let verified = event::verify_event_with_history(
            &frame.sealed_payload,
            kind,
            Boundary::Disk,
            &history,
        )?;
        history.record(frame.header.lsn, kind, verified.envelope.authority);
        let from = frame.header.lsn.get();
        present.insert(from);
        for (referenced, _) in references(&verified.envelope.payload) {
            let to = referenced.get();
            if to == from || !present.contains(&to) {
                continue;
            }
            adjacency.entry(from).or_default().insert(to);
            adjacency.entry(to).or_default().insert(from);
            if to == target {
                dependents.insert(from);
            }
        }
        if let EventPayload::Attestation(attestation) = &verified.envelope.payload
            && attestation.target_lsn == target
        {
            match attestation.disposition {
                AttestationDisposition::Used => counts.used += 1,
                AttestationDisposition::Ignored => counts.ignored += 1,
                AttestationDisposition::Helpful => counts.helpful += 1,
                AttestationDisposition::Harmful => counts.harmful += 1,
            }
        }
    }
    if !present.contains(&target) {
        return Err(Error::new(ErrorCode::InvalidArgument).at_lsn(lsn));
    }

    let component = component_of(&adjacency, target);
    let orphaned = if component.len() < MINIMUM_COMPONENT_RECORDS {
        Vec::new()
    } else {
        orphaned_records(&adjacency, &component, target)
    };
    let island_count = orphaned.len();
    let orphaned_lsns: Vec<u64> = orphaned
        .into_iter()
        .flatten()
        .collect::<BTreeSet<u64>>()
        .into_iter()
        .collect();
    let truncated =
        dependents.len() > MAXIMUM_REMOVAL_ITEMS || orphaned_lsns.len() > MAXIMUM_REMOVAL_ITEMS;
    let direct_dependents: Vec<u64> = dependents.into_iter().take(MAXIMUM_REMOVAL_ITEMS).collect();
    let orphaned_lsns: Vec<u64> = orphaned_lsns
        .into_iter()
        .take(MAXIMUM_REMOVAL_ITEMS)
        .collect();

    let mut envelope = Envelope::empty();
    envelope
        .provenance
        .push(format!("hm://{}/lsn/{target}", actor.actor()));
    for dependent in &direct_dependents {
        envelope
            .provenance
            .push(format!("hm://{}/lsn/{dependent}", actor.actor()));
    }
    envelope.items.push(json!({
        "surface": "removal",
        "diagnostic": "removal_preview",
        "performs_retraction": false,
        "target_lsn": target,
        "direct_dependents": direct_dependents,
        "orphaned_lsns": orphaned_lsns,
        "island_count": island_count,
        "component_records": component.len(),
        "attestation_counts": {
            "used": counts.used,
            "ignored": counts.ignored,
            "helpful": counts.helpful,
            "harmful": counts.harmful,
        },
        "scanned_frames": frames.len(),
        "truncated": truncated,
        "coverage": COVERAGE,
    }));
    Ok(envelope)
}

fn component_of(adjacency: &BTreeMap<u64, BTreeSet<u64>>, target: u64) -> BTreeSet<u64> {
    let mut component = BTreeSet::new();
    let mut pending = vec![target];
    while let Some(record) = pending.pop() {
        if !component.insert(record) {
            continue;
        }
        if let Some(neighbours) = adjacency.get(&record) {
            pending.extend(neighbours.iter().copied());
        }
    }
    component
}

fn orphaned_records(
    adjacency: &BTreeMap<u64, BTreeSet<u64>>,
    component: &BTreeSet<u64>,
    target: u64,
) -> Vec<BTreeSet<u64>> {
    let mut remaining: BTreeSet<u64> = component
        .iter()
        .copied()
        .filter(|record| *record != target)
        .collect();
    let mut fragments: Vec<BTreeSet<u64>> = Vec::new();
    while let Some(seed) = remaining.iter().next().copied() {
        let mut fragment = BTreeSet::new();
        let mut pending = vec![seed];
        while let Some(record) = pending.pop() {
            if !remaining.remove(&record) {
                continue;
            }
            fragment.insert(record);
            if let Some(neighbours) = adjacency.get(&record) {
                pending.extend(
                    neighbours
                        .iter()
                        .copied()
                        .filter(|neighbour| *neighbour != target),
                );
            }
        }
        fragments.push(fragment);
    }
    fragments.sort_by_key(|fragment| {
        (
            std::cmp::Reverse(fragment.len()),
            fragment.iter().next().copied().unwrap_or(0),
        )
    });
    if fragments.first().is_some_and(|largest| largest.len() > 1) {
        fragments.remove(0);
    }
    fragments
}
