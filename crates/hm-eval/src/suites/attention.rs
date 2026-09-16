#![allow(clippy::cast_precision_loss, clippy::missing_errors_doc)]

use hm_core::{Error, ErrorCode};
use hm_mcp::tools::intend::AttentionFactorsInput;
use hm_schema::events::AttentionDecision;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Deserialize)]
struct Case {
    id: String,
    rationale: String,
    suppress: bool,
    factors: AttentionFactorsInput,
}

#[derive(Debug, Serialize)]
pub struct AttentionResult {
    pub fixture_hash: String,
    pub cases: usize,
    pub true_suppression: usize,
    pub false_suppression: usize,
    pub missed_suppression: usize,
    pub true_interruptions: usize,
    pub reasons_present: usize,
}

impl AttentionResult {
    #[must_use]
    pub fn precision(&self) -> f64 {
        ratio(
            self.true_suppression,
            self.true_suppression + self.false_suppression,
        )
    }

    #[must_use]
    pub fn recall(&self) -> f64 {
        ratio(
            self.true_suppression,
            self.true_suppression + self.missed_suppression,
        )
    }
}

pub fn run() -> Result<AttentionResult, Error> {
    let bytes = include_bytes!("../../../../eval/fixtures/attention/interruption-labels.json");
    let cases: Vec<Case> =
        serde_json::from_slice(bytes).map_err(|_| Error::new(ErrorCode::SchemaInvalid))?;
    let mut ids = BTreeSet::new();
    let mut result = AttentionResult {
        fixture_hash: blake3::hash(bytes).to_hex().to_string(),
        cases: cases.len(),
        true_suppression: 0,
        false_suppression: 0,
        missed_suppression: 0,
        true_interruptions: 0,
        reasons_present: 0,
    };
    for case in cases {
        if case.rationale.is_empty() || !ids.insert(case.id.clone()) {
            return Err(Error::new(ErrorCode::SchemaInvalid));
        }
        let decision = hm_cortex::attention::decide(
            case.id.as_bytes(),
            case.id.as_bytes(),
            case.factors.into(),
        )?;
        let suppress = !matches!(
            decision.decision,
            AttentionDecision::Notify | AttentionDecision::AskUser | AttentionDecision::StartWork
        );
        match (case.suppress, suppress) {
            (true, true) => result.true_suppression += 1,
            (false, true) => result.false_suppression += 1,
            (true, false) => result.missed_suppression += 1,
            (false, false) => result.true_interruptions += 1,
        }
        result.reasons_present += usize::from(!decision.reason.is_empty());
    }
    Ok(result)
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}
