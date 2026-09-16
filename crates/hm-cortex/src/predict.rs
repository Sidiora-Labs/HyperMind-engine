#![allow(clippy::missing_errors_doc)]

use hm_core::{Error, ErrorCode};
use hm_schema::events::{OutcomeAssessment, OutcomeObserved, PredicateKind, Predicted};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Observation {
    pub lsn: u64,
    pub kind: PredicateKind,
    pub scope: String,
    pub property: Option<String>,
    pub value: Option<Vec<u8>>,
    pub executed: bool,
    pub resolvable: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RevisionGap {
    pub revision_required: bool,
    pub revision_rounds_remaining: u8,
    pub probes_remaining: u8,
}

pub fn assess(
    prediction: &Predicted,
    observations: &[Observation],
    now_ns: i64,
    evaluator_version: &str,
) -> Result<OutcomeObserved, Error> {
    if observations.is_empty() || evaluator_version.is_empty() {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let assessment = if observations.iter().all(|value| !value.executed) {
        OutcomeAssessment::NotExecuted
    } else if observations.iter().any(|value| !value.resolvable) {
        OutcomeAssessment::Unresolvable
    } else {
        let mut matched = 0;
        let mut contradicted = false;
        for predicate in &prediction.predicates {
            if let Some(observation) = observations.iter().find(|observation| {
                observation.kind == predicate.kind
                    && observation.scope == predicate.scope
                    && observation.property == predicate.property
            }) {
                matched += 1;
                contradicted |= observation.value != predicate.expected;
            }
        }
        if contradicted {
            OutcomeAssessment::Contradicted
        } else if matched == prediction.predicates.len() {
            OutcomeAssessment::Supported
        } else if now_ns <= prediction.deadline_ns {
            OutcomeAssessment::Pending
        } else {
            OutcomeAssessment::Unresolvable
        }
    };
    Ok(OutcomeObserved {
        prediction_id: prediction.prediction_id.clone(),
        revision: prediction.revision,
        assessment,
        observation_lsns: observations.iter().map(|value| value.lsn).collect(),
        evaluator_version: evaluator_version.to_owned(),
    })
}

#[must_use]
pub const fn revision_gap(same_mechanism_failures: u32, revisions: u8, probes: u8) -> RevisionGap {
    RevisionGap {
        revision_required: same_mechanism_failures >= 3,
        revision_rounds_remaining: 2_u8.saturating_sub(revisions),
        probes_remaining: 5_u8.saturating_sub(probes),
    }
}
