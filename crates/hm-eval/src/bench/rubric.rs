#![allow(clippy::cast_precision_loss, clippy::missing_errors_doc)]

use super::beam::{Probe, ProbeKind};
use super::gateway::{DynError, Gateway, JUDGE_MODEL, JUDGE_SETTINGS};
use super::ordering::{self, OrderingScore};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::fmt;

pub const CRITERION_PROMPT_ID: &str = "criterion-grade@1";
pub const CRITERION_PROMPT: &str = include_str!("../../../../prompts/criterion-grade@1.md");

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Compliance {
    Full,
    Partial,
    None,
}

impl Compliance {
    #[must_use]
    pub const fn weight(self) -> f64 {
        match self {
            Self::Full => 1.0,
            Self::Partial => 0.5,
            Self::None => 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerdictFailure {
    EmptyResponse,
    Unparsable,
    UnknownCompliance,
    MissingEvidence,
}

impl fmt::Display for VerdictFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::EmptyResponse => "the judge returned an empty response",
            Self::Unparsable => "the judge response carries no JSON verdict object",
            Self::UnknownCompliance => "the verdict compliance is not one of full, partial or none",
            Self::MissingEvidence => "the verdict carries no evidence quote",
        };
        formatter.write_str(text)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CriterionGrade {
    pub criterion_index: usize,
    pub criterion: String,
    pub compliance: Option<Compliance>,
    pub evidence: String,
    pub judge_model: String,
    pub prompt_id: String,
    pub response_digest: String,
    pub failure: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProbeGrade {
    pub probe_id: String,
    pub kind: ProbeKind,
    pub question: String,
    pub answer: String,
    pub answer_digest: String,
    pub grades: Vec<CriterionGrade>,
    pub graded_criteria: usize,
    pub judge_failures: usize,
    pub score: Option<f64>,
    pub ordering: Option<OrderingScore>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KindScore {
    pub kind: ProbeKind,
    pub probes: usize,
    pub scored: usize,
    pub mean_score: f64,
    pub judge_failures: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GradeSummary {
    pub probes: usize,
    pub scored_probes: usize,
    pub unscored_probes: usize,
    pub ordering_probes: usize,
    pub judge_scored_probes: usize,
    pub judge_failures: usize,
    pub mean_score: f64,
    pub mean_ordering_score: f64,
    pub per_kind: Vec<KindScore>,
    pub judge_model: String,
    pub prompt_id: String,
}

pub fn parse_verdict(raw: &str) -> Result<(Compliance, String), VerdictFailure> {
    if raw.trim().is_empty() {
        return Err(VerdictFailure::EmptyResponse);
    }
    let verdict = verdict_object(raw).ok_or(VerdictFailure::Unparsable)?;
    let compliance = match verdict.get("compliance").and_then(Value::as_str) {
        Some("full") => Compliance::Full,
        Some("partial") => Compliance::Partial,
        Some("none") => Compliance::None,
        Some(_) | None => return Err(VerdictFailure::UnknownCompliance),
    };
    let evidence = verdict
        .get("evidence")
        .and_then(Value::as_str)
        .ok_or(VerdictFailure::MissingEvidence)?;
    if evidence.trim().is_empty() {
        return Err(VerdictFailure::MissingEvidence);
    }
    Ok((compliance, evidence.to_owned()))
}

#[must_use]
pub fn criterion_prompt(question: &str, criterion: &str, answer: &str) -> String {
    json!({"question": question, "criterion": criterion, "answer": answer}).to_string()
}

pub fn grade_from_responses(
    probe: &Probe,
    answer: &str,
    judge_model: &str,
    responses: &[String],
) -> Result<ProbeGrade, DynError> {
    if probe.criteria.is_empty() {
        return Err(format!("probe {} carries no grading criteria", probe.id).into());
    }
    if probe.kind == ProbeKind::EventOrdering {
        return grade_ordering(probe, answer);
    }
    if responses.len() != probe.criteria.len() {
        return Err(format!(
            "probe {} has {} criteria but {} judge responses",
            probe.id,
            probe.criteria.len(),
            responses.len()
        )
        .into());
    }
    let mut grades = Vec::with_capacity(probe.criteria.len());
    let mut weight = 0.0;
    let mut graded_criteria = 0;
    let mut judge_failures = 0;
    for (criterion_index, (criterion, response)) in
        probe.criteria.iter().zip(responses.iter()).enumerate()
    {
        let mut grade = CriterionGrade {
            criterion_index,
            criterion: criterion.clone(),
            compliance: None,
            evidence: String::new(),
            judge_model: judge_model.to_owned(),
            prompt_id: CRITERION_PROMPT_ID.to_owned(),
            response_digest: blake3::hash(response.as_bytes()).to_hex().to_string(),
            failure: None,
        };
        match parse_verdict(response) {
            Ok((compliance, evidence)) => {
                weight += compliance.weight();
                graded_criteria += 1;
                grade.compliance = Some(compliance);
                grade.evidence = evidence;
            }
            Err(failure) => {
                judge_failures += 1;
                grade.failure = Some(failure.to_string());
            }
        }
        grades.push(grade);
    }
    let score = if judge_failures == 0 {
        Some(weight / probe.criteria.len() as f64)
    } else {
        None
    };
    Ok(ProbeGrade {
        probe_id: probe.id.clone(),
        kind: probe.kind,
        question: probe.question.clone(),
        answer: answer.to_owned(),
        answer_digest: blake3::hash(answer.as_bytes()).to_hex().to_string(),
        grades,
        graded_criteria,
        judge_failures,
        score,
        ordering: None,
    })
}

fn grade_ordering(probe: &Probe, answer: &str) -> Result<ProbeGrade, DynError> {
    let observed = ordering::split_ordered_answer(answer);
    let alignment = ordering::align_by_terms(
        &probe.criteria,
        &observed,
        ordering::DEFAULT_MINIMUM_OVERLAP,
    );
    let ordering = ordering::score_ordering(&probe.criteria, &observed, &alignment)?;
    Ok(ProbeGrade {
        probe_id: probe.id.clone(),
        kind: probe.kind,
        question: probe.question.clone(),
        answer: answer.to_owned(),
        answer_digest: blake3::hash(answer.as_bytes()).to_hex().to_string(),
        grades: Vec::new(),
        graded_criteria: 0,
        judge_failures: 0,
        score: Some(ordering.score),
        ordering: Some(ordering),
    })
}

pub async fn grade_probe(
    gateway: &Gateway,
    probe: &Probe,
    answer: &str,
) -> Result<ProbeGrade, DynError> {
    if probe.kind == ProbeKind::EventOrdering {
        return grade_from_responses(probe, answer, JUDGE_MODEL, &[]);
    }
    let mut responses = Vec::with_capacity(probe.criteria.len());
    for criterion in &probe.criteria {
        let completion = gateway
            .complete_with_settings(
                JUDGE_MODEL,
                CRITERION_PROMPT_ID,
                CRITERION_PROMPT,
                &criterion_prompt(&probe.question, criterion, answer),
                JUDGE_SETTINGS,
            )
            .await?;
        if completion.model != JUDGE_MODEL {
            return Err("criterion judge model differs from the pinned judge model".into());
        }
        responses.push(completion.text);
    }
    grade_from_responses(probe, answer, JUDGE_MODEL, &responses)
}

#[must_use]
pub fn summarize(grades: &[ProbeGrade], judge_model: &str) -> GradeSummary {
    let mut kinds: BTreeMap<ProbeKind, (usize, usize, f64, usize)> = BTreeMap::new();
    let mut scored_probes = 0;
    let mut ordering_probes = 0;
    let mut judge_scored_probes = 0;
    let mut judge_failures = 0;
    let mut total = 0.0;
    let mut ordering_total = 0.0;
    for grade in grades {
        let row = kinds.entry(grade.kind).or_insert((0, 0, 0.0, 0));
        row.0 += 1;
        row.3 += grade.judge_failures;
        judge_failures += grade.judge_failures;
        if let Some(score) = grade.score {
            scored_probes += 1;
            total += score;
            row.1 += 1;
            row.2 += score;
            if grade.ordering.is_some() {
                ordering_probes += 1;
                ordering_total += score;
            } else {
                judge_scored_probes += 1;
            }
        }
    }
    let per_kind = kinds
        .into_iter()
        .map(|(kind, (probes, scored, sum, failures))| KindScore {
            kind,
            probes,
            scored,
            mean_score: mean(sum, scored),
            judge_failures: failures,
        })
        .collect();
    GradeSummary {
        probes: grades.len(),
        scored_probes,
        unscored_probes: grades.len() - scored_probes,
        ordering_probes,
        judge_scored_probes,
        judge_failures,
        mean_score: mean(total, scored_probes),
        mean_ordering_score: mean(ordering_total, ordering_probes),
        per_kind,
        judge_model: judge_model.to_owned(),
        prompt_id: CRITERION_PROMPT_ID.to_owned(),
    }
}

fn mean(total: f64, count: usize) -> f64 {
    if count == 0 {
        0.0
    } else {
        total / count as f64
    }
}

fn verdict_object(raw: &str) -> Option<Map<String, Value>> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    if end < start {
        return None;
    }
    match serde_json::from_str(raw.get(start..=end)?) {
        Ok(Value::Object(verdict)) => Some(verdict),
        _ => None,
    }
}
