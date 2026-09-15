#![allow(clippy::missing_errors_doc)]

use hm_core::{Error, ErrorCode, LSN};
use hm_schema::events::{
    Assertion, AssertionClaim, Authority, BeliefType, EventEnvelope, EventPayload,
    ProposedAssertion, ProvenanceRange, ResultStatus,
};

pub trait BeliefEvidence {
    fn has_successful_tool_observed_result(&self, first_lsn: LSN, last_lsn: LSN) -> bool;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiveBelief {
    pub belief_id: Vec<u8>,
    pub belief_type: BeliefType,
    pub canonical_identity: String,
    pub conflict_domain: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BeliefConflict {
    pub event_index: usize,
    pub incoming_belief_id: Vec<u8>,
    pub existing_belief_id: Vec<u8>,
    pub belief_type: BeliefType,
    pub conflict_domain: String,
    pub incoming_identity: String,
    pub existing_identity: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GateRejection {
    pub event_index: usize,
    pub error: Error,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BatchAdmission {
    pub conflicts: Vec<BeliefConflict>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ApplyAdmission {
    pub admitted_indices: Vec<usize>,
    pub skipped: Vec<GateRejection>,
    pub conflicts: Vec<BeliefConflict>,
}

pub fn admit_batch(
    first_lsn: LSN,
    events: &[EventEnvelope],
    evidence: &impl BeliefEvidence,
    live_beliefs: &[LiveBelief],
) -> Result<BatchAdmission, GateRejection> {
    if first_lsn.get() == 0 || events.is_empty() {
        return Err(GateRejection {
            event_index: 0,
            error: Error::new(ErrorCode::InvalidArgument),
        });
    }
    let mut active = live_beliefs.to_vec();
    let mut admission = BatchAdmission::default();
    for (event_index, event) in events.iter().enumerate() {
        let event_lsn = checked_event_lsn(first_lsn, event_index).ok_or(GateRejection {
            event_index,
            error: Error::new(ErrorCode::CapacityExceeded),
        })?;
        inspect_event(
            event,
            event_index,
            event_lsn,
            first_lsn,
            events,
            evidence,
            &mut active,
            &mut admission.conflicts,
        )
        .map_err(|error| GateRejection { event_index, error })?;
    }
    Ok(admission)
}

pub fn apply_batch(
    first_lsn: LSN,
    events: &[EventEnvelope],
    evidence: &impl BeliefEvidence,
    live_beliefs: &[LiveBelief],
) -> ApplyAdmission {
    let mut active = live_beliefs.to_vec();
    let mut result = ApplyAdmission::default();
    if first_lsn.get() == 0 {
        result.skipped.extend(
            events
                .iter()
                .enumerate()
                .map(|(event_index, _)| GateRejection {
                    event_index,
                    error: Error::new(ErrorCode::InvalidArgument),
                }),
        );
        return result;
    }
    for (event_index, event) in events.iter().enumerate() {
        let Some(event_lsn) = checked_event_lsn(first_lsn, event_index) else {
            result.skipped.push(GateRejection {
                event_index,
                error: Error::new(ErrorCode::CapacityExceeded),
            });
            continue;
        };
        let mut staged_active = active.clone();
        let mut staged_conflicts = Vec::new();
        match inspect_event(
            event,
            event_index,
            event_lsn,
            first_lsn,
            events,
            evidence,
            &mut staged_active,
            &mut staged_conflicts,
        ) {
            Ok(()) => {
                active = staged_active;
                result.conflicts.extend(staged_conflicts);
                result.admitted_indices.push(event_index);
            }
            Err(error) => result.skipped.push(GateRejection { event_index, error }),
        }
    }
    result
}

#[allow(clippy::too_many_arguments)]
fn inspect_event(
    event: &EventEnvelope,
    event_index: usize,
    event_lsn: LSN,
    first_lsn: LSN,
    batch: &[EventEnvelope],
    evidence: &impl BeliefEvidence,
    active: &mut Vec<LiveBelief>,
    conflicts: &mut Vec<BeliefConflict>,
) -> Result<(), Error> {
    match &event.payload {
        EventPayload::Assertion(assertion) => {
            inspect_assertion(
                assertion,
                event_index,
                event_lsn,
                first_lsn,
                batch,
                evidence,
                active,
                conflicts,
            )?;
            upsert(active, assertion)?;
        }
        EventPayload::ProposedAssertion(assertion) => {
            inspect_proposed(assertion, event_lsn, first_lsn, batch, evidence)?;
            list_conflicts(
                event_index,
                &assertion.belief_id,
                assertion.belief_type,
                &assertion.canonical_identity,
                assertion.conflict_domain.as_deref(),
                active,
                conflicts,
            );
        }
        EventPayload::Consolidation(consolidation) => {
            for assertion in &consolidation.assertions {
                inspect_assertion(
                    assertion,
                    event_index,
                    event_lsn,
                    first_lsn,
                    batch,
                    evidence,
                    active,
                    conflicts,
                )?;
                upsert(active, assertion)?;
            }
        }
        EventPayload::Retract(retract) => {
            let old_len = active.len();
            active.retain(|belief| belief.belief_id != retract.belief_id);
            if active.len() == old_len {
                return Err(Error::new(ErrorCode::BeliefNotFound).at_lsn(event_lsn));
            }
        }
        _ => {}
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn inspect_assertion(
    assertion: &Assertion,
    event_index: usize,
    event_lsn: LSN,
    first_lsn: LSN,
    batch: &[EventEnvelope],
    evidence: &impl BeliefEvidence,
    active: &[LiveBelief],
    conflicts: &mut Vec<BeliefConflict>,
) -> Result<(), Error> {
    require_negative_evidence(
        assertion.claim,
        &assertion.provenance,
        event_lsn,
        first_lsn,
        batch,
        evidence,
    )?;
    list_conflicts(
        event_index,
        &assertion.belief_id,
        assertion.belief_type,
        &assertion.canonical_identity,
        assertion.conflict_domain.as_deref(),
        active,
        conflicts,
    );
    Ok(())
}

fn inspect_proposed(
    assertion: &ProposedAssertion,
    event_lsn: LSN,
    first_lsn: LSN,
    batch: &[EventEnvelope],
    evidence: &impl BeliefEvidence,
) -> Result<(), Error> {
    require_negative_evidence(
        assertion.claim,
        &assertion.provenance,
        event_lsn,
        first_lsn,
        batch,
        evidence,
    )
}

fn require_negative_evidence(
    claim: AssertionClaim,
    provenance: &[ProvenanceRange],
    event_lsn: LSN,
    first_lsn: LSN,
    batch: &[EventEnvelope],
    evidence: &impl BeliefEvidence,
) -> Result<(), Error> {
    if claim != AssertionClaim::NegativeExistence {
        return Ok(());
    }
    let corroborated = provenance.iter().any(|range| {
        historical_corroboration(range, event_lsn, first_lsn, evidence)
            || batch_corroboration(range, event_lsn, first_lsn, batch)
    });
    if corroborated {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::NegativeExistenceUncorroborated).at_lsn(event_lsn))
    }
}

fn historical_corroboration(
    range: &ProvenanceRange,
    event_lsn: LSN,
    first_lsn: LSN,
    evidence: &impl BeliefEvidence,
) -> bool {
    let Some(last_before_event) = event_lsn.get().checked_sub(1) else {
        return false;
    };
    let Some(last_before_batch) = first_lsn.get().checked_sub(1) else {
        return false;
    };
    let last_lsn = range.last_lsn.min(last_before_event).min(last_before_batch);
    range.first_lsn <= last_lsn
        && evidence
            .has_successful_tool_observed_result(LSN::new(range.first_lsn), LSN::new(last_lsn))
}

fn batch_corroboration(
    range: &ProvenanceRange,
    event_lsn: LSN,
    first_lsn: LSN,
    batch: &[EventEnvelope],
) -> bool {
    batch.iter().enumerate().any(|(index, event)| {
        checked_event_lsn(first_lsn, index).is_some_and(|candidate_lsn| {
            candidate_lsn.get() < event_lsn.get()
                && range.first_lsn <= candidate_lsn.get()
                && candidate_lsn.get() <= range.last_lsn
                && event.authority == Authority::ToolObserved
                && matches!(
                    &event.payload,
                    EventPayload::ToolResult(result) if result.status == ResultStatus::Ok
                )
        })
    })
}

#[allow(clippy::too_many_arguments)]
fn list_conflicts(
    event_index: usize,
    belief_id: &[u8],
    belief_type: BeliefType,
    canonical_identity: &str,
    conflict_domain: Option<&str>,
    active: &[LiveBelief],
    conflicts: &mut Vec<BeliefConflict>,
) {
    let Some(conflict_domain) = conflict_domain.filter(|domain| !domain.is_empty()) else {
        return;
    };
    conflicts.extend(
        active
            .iter()
            .filter(|existing| {
                existing.belief_type == belief_type
                    && existing.conflict_domain == conflict_domain
                    && existing.canonical_identity != canonical_identity
            })
            .map(|existing| BeliefConflict {
                event_index,
                incoming_belief_id: belief_id.to_vec(),
                existing_belief_id: existing.belief_id.clone(),
                belief_type,
                conflict_domain: conflict_domain.to_owned(),
                incoming_identity: canonical_identity.to_owned(),
                existing_identity: existing.canonical_identity.clone(),
            }),
    );
}

fn upsert(active: &mut Vec<LiveBelief>, assertion: &Assertion) -> Result<(), Error> {
    if active.iter().any(|belief| {
        belief.belief_id == assertion.belief_id
            && (belief.belief_type != assertion.belief_type
                || belief.canonical_identity != assertion.canonical_identity)
    }) {
        return Err(Error::new(ErrorCode::BeliefIdConflict));
    }
    active.retain(|belief| {
        belief.belief_id != assertion.belief_id
            && !(belief.belief_type == assertion.belief_type
                && belief.canonical_identity == assertion.canonical_identity)
    });
    active.push(LiveBelief {
        belief_id: assertion.belief_id.clone(),
        belief_type: assertion.belief_type,
        canonical_identity: assertion.canonical_identity.clone(),
        conflict_domain: assertion.conflict_domain.clone().unwrap_or_default(),
    });
    Ok(())
}

fn checked_event_lsn(first_lsn: LSN, event_index: usize) -> Option<LSN> {
    first_lsn
        .get()
        .checked_add(u64::try_from(event_index).ok()?)
        .map(LSN::new)
}
