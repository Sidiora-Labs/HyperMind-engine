#![allow(clippy::missing_errors_doc)]

use crate::checkpoint::verify_frame;
use crate::generation::{decode, encode};
use crate::store::{Mutation, ProjectionId, ProjectionStore, ReadSnapshot};
use hm_core::{Error, ErrorCode};
use hm_ledger::frame::{EventKind, Frame};
use hm_schema::events::{EventPayload, ExpectedPredicate, OutcomeAssessment, PredicateKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const PREDICTION_PREFIX: u8 = b'P';
const CALIBRATION_PREFIX: u8 = b'C';
const MECHANISM_PREFIX: u8 = b'M';

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PredictionRecord {
    pub prediction_id: Vec<u8>,
    pub revision: u32,
    pub task_id: Option<Vec<u8>>,
    pub attempt_id: Option<Vec<u8>>,
    pub operation_id: Option<Vec<u8>>,
    pub mechanism: String,
    pub predicates: Vec<ExpectedPredicate>,
    pub deadline_ns: i64,
    pub uncertainty: String,
    pub predicted_lsn: u64,
    pub assessment: Option<OutcomeAssessment>,
    pub observation_lsns: Vec<u64>,
    pub evaluator_version: Option<String>,
    pub outcome_lsn: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct CalibrationCounters {
    pub supported: u64,
    pub contradicted: u64,
    pub pending: u64,
    pub unresolvable: u64,
    pub not_executed: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct MechanismFailures {
    pub consecutive: u32,
    pub revision_required: bool,
}

pub struct PredictionsProjection;

impl PredictionsProjection {
    pub fn apply_event(store: &ProjectionStore, frame: &Frame) -> Result<(), Error> {
        if !matches!(
            frame.header.kind,
            EventKind::Predicted | EventKind::OutcomeObserved
        ) {
            return store.apply(ProjectionId::Predictions, frame.header.lsn, &[]);
        }
        let snapshot = store.begin_snapshot()?;
        let envelope = verify_frame(&snapshot, frame)?;
        let mut mutations = Vec::new();
        match envelope.payload {
            EventPayload::Predicted(value) => {
                let key = prediction_key(&value.prediction_id);
                let previous = snapshot
                    .get(ProjectionId::Predictions, &key)?
                    .map(|bytes| decode::<PredictionRecord>(&bytes))
                    .transpose()?;
                if previous.as_ref().map_or(value.revision != 1, |record| {
                    value.revision != record.revision.saturating_add(1)
                }) {
                    return Err(Error::new(ErrorCode::OrderingViolation).at_lsn(frame.header.lsn));
                }
                let record = PredictionRecord {
                    prediction_id: value.prediction_id,
                    revision: value.revision,
                    task_id: value.task_id,
                    attempt_id: value.attempt_id,
                    operation_id: value.operation_id,
                    mechanism: value.mechanism,
                    predicates: value.predicates,
                    deadline_ns: value.deadline_ns,
                    uncertainty: value.uncertainty,
                    predicted_lsn: frame.header.lsn.get(),
                    assessment: None,
                    observation_lsns: Vec::new(),
                    evaluator_version: None,
                    outcome_lsn: 0,
                };
                mutations.push(Mutation::put(key, encode(&record)?));
            }
            EventPayload::OutcomeObserved(value) => {
                let key = prediction_key(&value.prediction_id);
                let mut record = snapshot
                    .get(ProjectionId::Predictions, &key)?
                    .ok_or_else(|| {
                        Error::new(ErrorCode::OrderingViolation).at_lsn(frame.header.lsn)
                    })
                    .and_then(|bytes| decode::<PredictionRecord>(&bytes))?;
                if record.revision != value.revision
                    || record
                        .assessment
                        .is_some_and(|assessment| assessment != OutcomeAssessment::Pending)
                {
                    return Err(Error::new(ErrorCode::IdempotencyConflict).at_lsn(frame.header.lsn));
                }
                let previous_assessment = record.assessment;
                record.assessment = Some(value.assessment);
                record.observation_lsns = value.observation_lsns;
                record.evaluator_version = Some(value.evaluator_version);
                record.outcome_lsn = frame.header.lsn.get();
                mutations.push(Mutation::put(key, encode(&record)?));
                let kinds = record
                    .predicates
                    .iter()
                    .map(|predicate| predicate.kind)
                    .collect::<BTreeSet<_>>();
                for kind in kinds {
                    let key = calibration_key(kind);
                    let mut counters = snapshot
                        .get(ProjectionId::Predictions, &key)?
                        .map(|bytes| decode::<CalibrationCounters>(&bytes))
                        .transpose()?
                        .unwrap_or_default();
                    if previous_assessment == Some(OutcomeAssessment::Pending) {
                        counters.pending = counters.pending.saturating_sub(1);
                    }
                    counters.increment(value.assessment);
                    mutations.push(Mutation::put(key, encode(&counters)?));
                }
                let key = mechanism_key(&record.mechanism);
                let mut failures = snapshot
                    .get(ProjectionId::Predictions, &key)?
                    .map(|bytes| decode::<MechanismFailures>(&bytes))
                    .transpose()?
                    .unwrap_or_default();
                match value.assessment {
                    OutcomeAssessment::Contradicted => {
                        failures.consecutive = failures.consecutive.saturating_add(1);
                        failures.revision_required = failures.consecutive >= 3;
                    }
                    OutcomeAssessment::Supported => failures = MechanismFailures::default(),
                    _ => {}
                }
                mutations.push(Mutation::put(key, encode(&failures)?));
            }
            _ => return Err(Error::new(ErrorCode::InvalidKind).at_lsn(frame.header.lsn)),
        }
        drop(snapshot);
        store.apply(ProjectionId::Predictions, frame.header.lsn, &mutations)
    }

    pub fn get(
        snapshot: &ReadSnapshot<'_>,
        prediction_id: &[u8],
    ) -> Result<Option<PredictionRecord>, Error> {
        snapshot
            .get(ProjectionId::Predictions, &prediction_key(prediction_id))?
            .map(|bytes| decode(&bytes))
            .transpose()
    }

    pub fn calibration(
        snapshot: &ReadSnapshot<'_>,
        kind: PredicateKind,
    ) -> Result<CalibrationCounters, Error> {
        snapshot
            .get(ProjectionId::Predictions, &calibration_key(kind))?
            .map(|bytes| decode(&bytes))
            .transpose()
            .map(Option::unwrap_or_default)
    }

    pub fn mechanism_failures(
        snapshot: &ReadSnapshot<'_>,
        mechanism: &str,
    ) -> Result<MechanismFailures, Error> {
        snapshot
            .get(ProjectionId::Predictions, &mechanism_key(mechanism))?
            .map(|bytes| decode(&bytes))
            .transpose()
            .map(Option::unwrap_or_default)
    }
}

impl CalibrationCounters {
    fn increment(&mut self, assessment: OutcomeAssessment) {
        let counter = match assessment {
            OutcomeAssessment::Supported => &mut self.supported,
            OutcomeAssessment::Contradicted => &mut self.contradicted,
            OutcomeAssessment::Pending => &mut self.pending,
            OutcomeAssessment::Unresolvable => &mut self.unresolvable,
            OutcomeAssessment::NotExecuted => &mut self.not_executed,
        };
        *counter = counter.saturating_add(1);
    }
}

fn prediction_key(id: &[u8]) -> Vec<u8> {
    prefixed(PREDICTION_PREFIX, id)
}

fn calibration_key(kind: PredicateKind) -> [u8; 2] {
    [CALIBRATION_PREFIX, kind as u8]
}

fn mechanism_key(mechanism: &str) -> Vec<u8> {
    prefixed(
        MECHANISM_PREFIX,
        blake3::hash(mechanism.as_bytes()).as_bytes(),
    )
}

fn prefixed(prefix: u8, value: &[u8]) -> Vec<u8> {
    let mut key = Vec::with_capacity(value.len() + 1);
    key.push(prefix);
    key.extend_from_slice(value);
    key
}
