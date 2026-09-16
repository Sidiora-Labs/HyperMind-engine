#![allow(clippy::missing_errors_doc)]

use super::predict::{PredicateKindInput, bounded, scalar_bytes};
use crate::Envelope;
use hm_core::{Error, ErrorCode, LSN};
use hm_cortex::predict::{Observation, assess, revision_gap};
use hm_ledger::frame::EventKind;
use hm_schema::events::{Authority, EventPayload, OutcomeAssessment, Predicted, ResultStatus};
use hm_serve::actor::ActorEngine;
use rmcp::schemars;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::time::{SystemTime, UNIX_EPOCH};

const EVALUATOR_VERSION: &str = "hm-predicate-evaluator@1";

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OutcomeInput {
    pub conversation: String,
    pub prediction_id: String,
    pub revision: u32,
    pub observation_lsns: Vec<u64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservationDocument {
    schema_version: u32,
    observations: Vec<ObservedPredicate>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservedPredicate {
    kind: PredicateKindInput,
    scope: String,
    #[serde(default)]
    property: Option<String>,
    #[serde(default)]
    value: Option<Value>,
    executed: bool,
    resolvable: bool,
}

pub async fn run(actor: &ActorEngine, input: OutcomeInput) -> Result<Envelope, Error> {
    if input.conversation.is_empty()
        || !bounded(&input.prediction_id)
        || input.observation_lsns.is_empty()
        || input.observation_lsns.len() > 5
        || input.observation_lsns.contains(&0)
    {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let record = actor
        .prediction(input.prediction_id.as_bytes().to_vec())
        .await?
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
    if record.revision != input.revision {
        return Err(Error::new(ErrorCode::IdempotencyConflict));
    }
    let mut lsns = input
        .observation_lsns
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    lsns.extend(record.observation_lsns.iter().copied());
    if lsns.len() > 5 {
        return Err(Error::new(ErrorCode::CapacityExceeded));
    }
    let mut observations_by_predicate = BTreeMap::new();
    for lsn in &lsns {
        if *lsn <= record.predicted_lsn {
            return Err(Error::new(ErrorCode::CitationInvalid).at_lsn(LSN::new(*lsn)));
        }
        for observation in read_observations(actor, *lsn).await? {
            if record.predicates.iter().any(|predicate| {
                predicate.kind == observation.kind
                    && predicate.scope == observation.scope
                    && predicate.property == observation.property
            }) {
                observations_by_predicate.insert(
                    (
                        observation.kind,
                        observation.scope.clone(),
                        observation.property.clone(),
                    ),
                    observation,
                );
            }
        }
    }
    let observations = observations_by_predicate.into_values().collect::<Vec<_>>();
    if observations.is_empty() {
        return Err(Error::new(ErrorCode::CitationInvalid));
    }
    let prediction = Predicted {
        prediction_id: record.prediction_id,
        revision: record.revision,
        task_id: record.task_id,
        attempt_id: record.attempt_id,
        operation_id: record.operation_id,
        mechanism: record.mechanism,
        predicates: record.predicates,
        deadline_ns: record.deadline_ns,
        uncertainty: record.uncertainty,
    };
    let now_ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_nanos()).ok())
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
    let mut assessed = assess(&prediction, &observations, now_ns, EVALUATOR_VERSION)?;
    assessed.observation_lsns = lsns.iter().copied().collect();
    let duplicate = record.assessment == Some(assessed.assessment)
        && record.observation_lsns == assessed.observation_lsns;
    if record
        .assessment
        .is_some_and(|value| value != OutcomeAssessment::Pending)
        && !duplicate
    {
        return Err(Error::new(ErrorCode::IdempotencyConflict));
    }
    let assessment = assessed.assessment;
    let lsn = if duplicate {
        record.outcome_lsn
    } else {
        super::believe::append(
            actor,
            &input.conversation,
            EventKind::OutcomeObserved,
            EventPayload::OutcomeObserved(Box::new(assessed)),
            Authority::RuntimeFact,
            None,
        )
        .await?
        .first_lsn
        .get()
    };
    let failures = actor.mechanism_failures(prediction.mechanism).await?;
    let gap = revision_gap(
        failures.consecutive,
        u8::try_from(prediction.revision.saturating_sub(1)).unwrap_or(u8::MAX),
        u8::try_from(lsns.len()).unwrap_or(u8::MAX),
    );
    let mut envelope = Envelope::empty();
    envelope.items.push(json!({
        "prediction_id": input.prediction_id, "revision": input.revision,
        "assessment": assessment_name(assessment), "observation_lsns": lsns,
        "evaluator_version": EVALUATOR_VERSION, "lsn": lsn, "duplicate": duplicate,
    }));
    envelope
        .provenance
        .push(format!("hm://{}/lsn/{lsn}", actor.actor()));
    if gap.revision_required {
        envelope.gaps.push(json!({"kind": "revision_required",
            "revision_rounds_remaining": gap.revision_rounds_remaining,
            "probes_remaining": gap.probes_remaining}));
    }
    Ok(envelope)
}

async fn read_observations(actor: &ActorEngine, lsn: u64) -> Result<Vec<Observation>, Error> {
    let verified = actor.verified_event(LSN::new(lsn)).await?;
    let (bytes, successful) = match (verified.envelope.authority, verified.envelope.payload) {
        (Authority::ToolObserved, EventPayload::ToolResult(value)) => {
            (value.result, value.status == ResultStatus::Ok)
        }
        (Authority::ExternalObserved, EventPayload::ProviderFrame(value)) => {
            (value.api_content, true)
        }
        (Authority::RuntimeFact, EventPayload::Outcome(value)) => {
            (value.detail, value.status == ResultStatus::Ok)
        }
        _ => return Err(Error::new(ErrorCode::CitationInvalid).at_lsn(LSN::new(lsn))),
    };
    if bytes.len() > 65_536 {
        return Err(Error::new(ErrorCode::CapacityExceeded));
    }
    let document: ObservationDocument = serde_json::from_slice(&bytes)
        .map_err(|_| Error::new(ErrorCode::SchemaInvalid).at_lsn(LSN::new(lsn)))?;
    if document.schema_version != 1
        || document.observations.is_empty()
        || document.observations.len() > 16
    {
        return Err(Error::new(ErrorCode::SchemaInvalid).at_lsn(LSN::new(lsn)));
    }
    document
        .observations
        .into_iter()
        .map(|value| {
            if !bounded(&value.scope)
                || value.property.as_ref().is_some_and(|value| !bounded(value))
            {
                return Err(Error::new(ErrorCode::SchemaInvalid));
            }
            Ok(Observation {
                lsn,
                kind: value.kind.into(),
                scope: value.scope,
                property: value.property,
                value: value.value.as_ref().map(scalar_bytes).transpose()?,
                executed: value.executed,
                resolvable: value.resolvable && successful,
            })
        })
        .collect()
}

pub(crate) const fn assessment_name(value: OutcomeAssessment) -> &'static str {
    match value {
        OutcomeAssessment::Supported => "supported",
        OutcomeAssessment::Contradicted => "contradicted",
        OutcomeAssessment::Pending => "pending",
        OutcomeAssessment::Unresolvable => "unresolvable",
        OutcomeAssessment::NotExecuted => "not_executed",
    }
}
