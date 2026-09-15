#![allow(clippy::missing_errors_doc)]

use crate::actor::ActorEngine;
use hm_core::{Error, ErrorCode, LSN};
use hm_proj::beliefs::{BeliefAsOf, BeliefRecord};
use hm_schema::events::BeliefType;
use hm_schema::wire::{AsOf, BeliefConflictRecord, BeliefProvenanceRecord, BeliefResult};

pub async fn read(actor: &ActorEngine, request: AsOf) -> Result<BeliefResult, Error> {
    let belief_type = BeliefType::try_from(request.belief_type)
        .map_err(|_| Error::new(ErrorCode::ProtocolInvalid))?;
    let axis = if request.valid_time_ns != 0 {
        BeliefAsOf::ValidAt(request.valid_time_ns)
    } else {
        BeliefAsOf::KnownAt(LSN::new(request.known_lsn))
    };
    actor
        .as_of(belief_type, request.canonical_identity, axis)
        .await
        .map(|result| encode(result.record, belief_type))
}

fn encode(record: Option<BeliefRecord>, belief_type: BeliefType) -> BeliefResult {
    let Some(record) = record else {
        return BeliefResult {
            belief_type: belief_type as u8,
            ..BeliefResult::default()
        };
    };
    BeliefResult {
        present: true,
        belief_type: record.belief_type as u8,
        belief_id: Some(record.belief_id),
        canonical_identity: Some(record.canonical_identity),
        conflict_domain: (!record.conflict_domain.is_empty()).then_some(record.conflict_domain),
        value: Some(record.value),
        claim: record.claim as u8,
        valid_from_ns: record.valid_from_ns,
        valid_to_ns: record.valid_to_ns,
        transaction_lsn: record.observation_lsn,
        version: record.version,
        supersedes_version: record.supersedes_version,
        provenance: Some(
            record
                .provenance
                .into_iter()
                .map(|range| BeliefProvenanceRecord {
                    first_lsn: range.first_lsn,
                    last_lsn: range.last_lsn,
                    byte_start: range.byte_start,
                    byte_end: range.byte_end,
                })
                .collect(),
        ),
        conflict_edges: Some(
            record
                .conflict_edges
                .into_iter()
                .map(|edge| BeliefConflictRecord {
                    other_type: edge.other_type as u8,
                    other_canonical_identity: edge.other_canonical_identity,
                    created_lsn: edge.created_lsn,
                    resolved_lsn: edge.resolved_lsn,
                    obligated_surfacing: edge.obligated_surfacing,
                })
                .collect(),
        ),
        tombstoned: record.tombstoned,
    }
}
