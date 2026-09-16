#![allow(clippy::missing_errors_doc)]

use crate::Envelope;
use hm_core::{Error, ErrorCode};
use hm_ledger::frame::EventKind;
use hm_schema::events::{Authority, EventPayload, ExpectedPredicate, PredicateKind, Predicted};
use hm_serve::actor::ActorEngine;
use rmcp::schemars;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PredicateKindInput {
    ObjectExists,
    RevisionEquals,
    DigestEquals,
    ReceiptMatches,
    PropertySatisfies,
    ProcessTerminated,
    AnswerCommitted,
}

impl From<PredicateKindInput> for PredicateKind {
    fn from(value: PredicateKindInput) -> Self {
        match value {
            PredicateKindInput::ObjectExists => Self::ObjectExists,
            PredicateKindInput::RevisionEquals => Self::RevisionEquals,
            PredicateKindInput::DigestEquals => Self::DigestEquals,
            PredicateKindInput::ReceiptMatches => Self::ReceiptMatches,
            PredicateKindInput::PropertySatisfies => Self::PropertySatisfies,
            PredicateKindInput::ProcessTerminated => Self::ProcessTerminated,
            PredicateKindInput::AnswerCommitted => Self::AnswerCommitted,
        }
    }
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExpectedPredicateInput {
    pub kind: PredicateKindInput,
    pub scope: String,
    #[serde(default)]
    pub property: Option<String>,
    #[serde(default)]
    pub expected: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PredictInput {
    pub conversation: String,
    pub prediction_id: String,
    pub revision: u32,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub attempt_id: Option<String>,
    #[serde(default)]
    pub operation_id: Option<String>,
    pub mechanism: String,
    pub predicates: Vec<ExpectedPredicateInput>,
    pub deadline_ns: i64,
    pub uncertainty: String,
}

pub async fn run(actor: &ActorEngine, input: PredictInput) -> Result<Envelope, Error> {
    if input.conversation.is_empty()
        || !bounded(&input.prediction_id)
        || !(1..=3).contains(&input.revision)
        || !bounded(&input.mechanism)
        || input.predicates.is_empty()
        || input.predicates.len() > 16
        || input.deadline_ns <= 0
        || input.uncertainty.is_empty()
        || input.uncertainty.len() > 4096
        || [&input.task_id, &input.attempt_id, &input.operation_id]
            .iter()
            .any(|value| value.as_ref().is_some_and(|value| !bounded(value)))
    {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let predicates = input
        .predicates
        .iter()
        .map(predicate)
        .collect::<Result<Vec<_>, _>>()?;
    let prediction = Predicted {
        prediction_id: input.prediction_id.as_bytes().to_vec(),
        revision: input.revision,
        task_id: input.task_id.map(String::into_bytes),
        attempt_id: input.attempt_id.map(String::into_bytes),
        operation_id: input.operation_id.map(String::into_bytes),
        mechanism: input.mechanism,
        predicates,
        deadline_ns: input.deadline_ns,
        uncertainty: input.uncertainty,
    };
    let previous = actor.prediction(prediction.prediction_id.clone()).await?;
    let duplicate_lsn = if let Some(previous) = previous {
        if previous.revision == prediction.revision {
            if previous.task_id != prediction.task_id
                || previous.attempt_id != prediction.attempt_id
                || previous.operation_id != prediction.operation_id
                || previous.mechanism != prediction.mechanism
                || previous.predicates != prediction.predicates
                || previous.deadline_ns != prediction.deadline_ns
                || previous.uncertainty != prediction.uncertainty
            {
                return Err(Error::new(ErrorCode::IdempotencyConflict));
            }
            Some(previous.predicted_lsn)
        } else {
            if prediction.revision != previous.revision.saturating_add(1) {
                return Err(Error::new(ErrorCode::OrderingViolation));
            }
            None
        }
    } else {
        if prediction.revision != 1 {
            return Err(Error::new(ErrorCode::OrderingViolation));
        }
        None
    };
    let lsn = if let Some(lsn) = duplicate_lsn {
        lsn
    } else {
        super::believe::append(
            actor,
            &input.conversation,
            EventKind::Predicted,
            EventPayload::Predicted(Box::new(prediction)),
            Authority::DerivedInference,
            None,
        )
        .await?
        .first_lsn
        .get()
    };
    let mut envelope = Envelope::empty();
    envelope.items.push(json!({
        "prediction_id": input.prediction_id,
        "revision": input.revision,
        "lsn": lsn,
        "duplicate": duplicate_lsn.is_some(),
    }));
    envelope
        .provenance
        .push(format!("hm://{}/lsn/{lsn}", actor.actor()));
    Ok(envelope)
}

fn predicate(input: &ExpectedPredicateInput) -> Result<ExpectedPredicate, Error> {
    if !bounded(&input.scope)
        || input.property.as_ref().is_some_and(|value| !bounded(value))
        || (matches!(input.kind, PredicateKindInput::PropertySatisfies) && input.property.is_none())
    {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let expected = input
        .expected
        .clone()
        .or_else(|| {
            matches!(
                input.kind,
                PredicateKindInput::ObjectExists | PredicateKindInput::AnswerCommitted
            )
            .then_some(Value::Bool(true))
        })
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
    if matches!(
        input.kind,
        PredicateKindInput::ObjectExists | PredicateKindInput::AnswerCommitted
    ) && !expected.is_boolean()
    {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    Ok(ExpectedPredicate {
        kind: input.kind.into(),
        scope: input.scope.clone(),
        property: input.property.clone(),
        expected: Some(scalar_bytes(&expected)?),
    })
}

pub(crate) fn scalar_bytes(value: &Value) -> Result<Vec<u8>, Error> {
    if !matches!(value, Value::Bool(_) | Value::Number(_) | Value::String(_)) {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let encoded = serde_json::to_vec(value).map_err(|_| Error::new(ErrorCode::InvalidArgument))?;
    if encoded.len() > 4096 {
        return Err(Error::new(ErrorCode::CapacityExceeded));
    }
    Ok(encoded)
}

pub(crate) fn bounded(value: &str) -> bool {
    !value.is_empty() && value.len() <= 1024
}

pub(crate) const fn kind_name(kind: PredicateKind) -> &'static str {
    match kind {
        PredicateKind::ObjectExists => "object_exists",
        PredicateKind::RevisionEquals => "revision_equals",
        PredicateKind::DigestEquals => "digest_equals",
        PredicateKind::ReceiptMatches => "receipt_matches",
        PredicateKind::PropertySatisfies => "property_satisfies",
        PredicateKind::ProcessTerminated => "process_terminated",
        PredicateKind::AnswerCommitted => "answer_committed",
    }
}
