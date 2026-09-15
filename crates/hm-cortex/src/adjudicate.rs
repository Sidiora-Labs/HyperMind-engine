#![allow(clippy::missing_errors_doc)]

use crate::nli::{BidirectionalNli, NliError, NliModel, NliVerdict};
use hm_llm::{LlmError, LlmProvider, ModelTier, StructuredRequest, Usage};
use hm_schema::events::{Assertion, ProvenanceRange, Retract};
use serde_json::{Value, json};

pub const SUPERSESSION_PROMPT: &str = include_str!("../../../prompts/supersession@1.md");

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdjudicationConflict {
    pub left_belief_id: Vec<u8>,
    pub right_belief_id: Vec<u8>,
    pub conflict_domain: String,
    pub obligated_surfacing: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdjudicationOutcome {
    Replace {
        retract: Retract,
        assertion: Assertion,
    },
    Conflict(AdjudicationConflict),
    UnverifiedTension,
    NoChange,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdjudicationReport {
    pub nli: BidirectionalNli,
    pub outcome: AdjudicationOutcome,
    pub model_id: Option<String>,
    pub usage: Usage,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdjudicationError {
    InvalidUtf8,
    InvalidBelief,
    Nli(NliError),
    Llm(LlmError),
    InvalidStructuredOutput,
}

impl From<NliError> for AdjudicationError {
    fn from(value: NliError) -> Self {
        Self::Nli(value)
    }
}

impl From<LlmError> for AdjudicationError {
    fn from(value: LlmError) -> Self {
        Self::Llm(value)
    }
}

pub fn dispute(
    nli: &NliModel,
    provider: Option<&dyn LlmProvider>,
    minimum_tier: ModelTier,
    existing: &Assertion,
    incoming: &Assertion,
) -> Result<AdjudicationReport, AdjudicationError> {
    if existing.belief_id.is_empty()
        || incoming.belief_id.is_empty()
        || existing.belief_type != incoming.belief_type
        || existing.conflict_domain != incoming.conflict_domain
    {
        return Err(AdjudicationError::InvalidBelief);
    }
    let existing_text =
        std::str::from_utf8(&existing.value).map_err(|_| AdjudicationError::InvalidUtf8)?;
    let incoming_text =
        std::str::from_utf8(&incoming.value).map_err(|_| AdjudicationError::InvalidUtf8)?;
    let nli_report = nli.classify_both(existing_text, incoming_text)?;
    if nli_report.verdict != NliVerdict::Genuine || nli_report.confidence < 0.8 {
        let outcome = if nli_report.verdict == NliVerdict::Tension {
            AdjudicationOutcome::UnverifiedTension
        } else {
            AdjudicationOutcome::NoChange
        };
        return Ok(AdjudicationReport {
            nli: nli_report,
            outcome,
            model_id: None,
            usage: Usage::default(),
        });
    }
    let Some(provider) = provider.filter(|provider| provider.tier() >= minimum_tier) else {
        return Ok(AdjudicationReport {
            nli: nli_report,
            outcome: AdjudicationOutcome::UnverifiedTension,
            model_id: None,
            usage: Usage::default(),
        });
    };
    let response = provider.generate_structured(&StructuredRequest {
        prompt_id: "supersession_1".to_owned(),
        system: SUPERSESSION_PROMPT.to_owned(),
        prompt: format!(
            "existing_identity: {}\nexisting_valid: {}..{}\nexisting_value: {}\n\nincoming_identity: {}\nincoming_valid: {}..{}\nincoming_value: {}",
            existing.canonical_identity,
            existing.valid_from_ns,
            existing.valid_to_ns,
            existing_text,
            incoming.canonical_identity,
            incoming.valid_from_ns,
            incoming.valid_to_ns,
            incoming_text,
        ),
        json_schema: supersession_schema(),
        maximum_output_tokens: 256,
    })?;
    let supersedes = response
        .value
        .get("supersedes")
        .and_then(Value::as_bool)
        .ok_or(AdjudicationError::InvalidStructuredOutput)?;
    let outcome = if supersedes {
        AdjudicationOutcome::Replace {
            retract: Retract {
                belief_id: existing.belief_id.clone(),
                provenance: incoming.provenance.clone(),
            },
            assertion: incoming.clone(),
        }
    } else {
        AdjudicationOutcome::Conflict(conflict(existing, incoming)?)
    };
    Ok(AdjudicationReport {
        nli: nli_report,
        outcome,
        model_id: Some(response.model_id),
        usage: response.usage,
    })
}

fn conflict(
    existing: &Assertion,
    incoming: &Assertion,
) -> Result<AdjudicationConflict, AdjudicationError> {
    let conflict_domain = existing
        .conflict_domain
        .clone()
        .filter(|domain| !domain.is_empty())
        .ok_or(AdjudicationError::InvalidBelief)?;
    Ok(AdjudicationConflict {
        left_belief_id: existing.belief_id.clone(),
        right_belief_id: incoming.belief_id.clone(),
        conflict_domain,
        obligated_surfacing: true,
    })
}

fn supersession_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "supersedes": {"type": "boolean"},
            "reason": {"type": "string"}
        },
        "required": ["supersedes", "reason"],
        "additionalProperties": false
    })
}

#[must_use]
pub fn cited_range(first_lsn: u64, last_lsn: u64, byte_end: u32) -> ProvenanceRange {
    ProvenanceRange {
        first_lsn,
        last_lsn,
        byte_start: 0,
        byte_end,
    }
}
