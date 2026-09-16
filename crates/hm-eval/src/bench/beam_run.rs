#![allow(clippy::cast_precision_loss, clippy::missing_errors_doc)]

//! Executes a probe set against the real memory pipeline.
//!
//! The report is split in two. `judge_free` needs no provider at all: it is
//! retrieval arithmetic over the citations the ledger returned. `judged` is
//! present only when a gateway pass actually graded the answers, so a run that
//! never reached a provider cannot be mistaken for a graded one.

use super::beam::{self, Conversation, Probe, ProbeKind, ProbeSet};
use super::gateway::{DynError, Gateway, JUDGE_MODEL, READER_MODEL};
use super::pipeline::{BenchCitation, MemoryPipeline};
use super::rubric::{self, CRITERION_PROMPT_ID, GradeSummary, ProbeGrade};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub const RESULT_FORMAT: &str = "hypermind.beam-result.v1";
pub const PROBE_SET_ENVIRONMENT: &str = "HM_BEAM_PROBE_SET";
pub const ENCODER: &str = "lexical_only";
pub const FIXTURE_FAILURE: &str = "the probe set is the in-repo fixture and not a published benchmark artifact, so this report records wiring and is never a benchmark score";
pub const JUDGE_FREE_FAILURE: &str =
    "no judged pass ran, so this report carries retrieval-side numbers only";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProbeRetrieval {
    pub probe_id: String,
    pub kind: ProbeKind,
    pub evidence_expected: usize,
    pub evidence_retrieved: usize,
    pub first_evidence_rank: Option<usize>,
    pub citations: usize,
    pub context_characters: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RetrievalSummary {
    pub probes: usize,
    pub evidence_recall: f64,
    pub mean_reciprocal_rank: f64,
    pub probes_without_evidence: usize,
    pub per_kind_probes: Vec<(ProbeKind, usize)>,
    pub rows: Vec<ProbeRetrieval>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BeamResult {
    pub format: String,
    pub probe_set_digest: String,
    pub source_digest: String,
    pub conversations: usize,
    pub total: usize,
    pub answered: usize,
    pub encoder: String,
    pub reader_model: String,
    pub judge_model: String,
    pub prompt_id: String,
    pub judge_free: RetrievalSummary,
    pub judged: Option<GradeSummary>,
    pub failure: Option<String>,
    pub complete: bool,
}

pub async fn run_judge_free(set: &ProbeSet, root: &Path) -> Result<BeamResult, DynError> {
    let digest = validated_digest(set)?;
    let mut rows = Vec::new();
    for conversation in &set.conversations {
        let pipeline = MemoryPipeline::open(
            root,
            &digest,
            &conversation.id,
            &beam::to_documents(conversation),
        )
        .await?;
        let mut failure = None;
        for probe in &conversation.probes {
            match pipeline
                .retrieve_context(&beam::to_question(conversation, probe))
                .await
            {
                Ok(context) => rows.push(retrieval_row(
                    conversation,
                    probe,
                    &context.provenance,
                    context.coverage.context_characters,
                )),
                Err(error) => {
                    failure = Some(error);
                    break;
                }
            }
        }
        pipeline.close().await?;
        if let Some(error) = failure {
            return Err(error);
        }
    }
    report(set, digest, rows, 0, None)
}

pub async fn run(gateway: &Gateway, set: &ProbeSet, root: &Path) -> Result<BeamResult, DynError> {
    let digest = validated_digest(set)?;
    let mut rows = Vec::new();
    let mut grades: Vec<ProbeGrade> = Vec::new();
    let mut answered = 0;
    for conversation in &set.conversations {
        let pipeline = MemoryPipeline::open(
            root,
            &digest,
            &conversation.id,
            &beam::to_documents(conversation),
        )
        .await?;
        let mut failure = None;
        for probe in &conversation.probes {
            match probe_pass(gateway, &pipeline, conversation, probe).await {
                Ok((row, grade)) => {
                    answered += 1;
                    rows.push(row);
                    grades.push(grade);
                }
                Err(error) => {
                    failure = Some(error);
                    break;
                }
            }
        }
        pipeline.close().await?;
        if let Some(error) = failure {
            return Err(error);
        }
    }
    let judged = rubric::summarize(&grades, JUDGE_MODEL);
    report(set, digest, rows, answered, Some(judged))
}

#[must_use]
pub fn default_probe_set_path() -> PathBuf {
    super::execution::repository_root().join("eval/datasets/beam/probe-set.json")
}

pub fn load_or_fixture() -> Result<(ProbeSet, bool), DynError> {
    let path = match std::env::var(PROBE_SET_ENVIRONMENT) {
        Ok(value) if !value.trim().is_empty() => PathBuf::from(value.trim()),
        _ => default_probe_set_path(),
    };
    if path.exists() {
        return Ok((beam::load_probe_set(&path)?, false));
    }
    Ok((beam::fixture_probe_set()?, true))
}

async fn probe_pass(
    gateway: &Gateway,
    pipeline: &MemoryPipeline,
    conversation: &Conversation,
    probe: &Probe,
) -> Result<(ProbeRetrieval, ProbeGrade), DynError> {
    let answer = pipeline
        .answer(gateway, &beam::to_question(conversation, probe))
        .await?;
    let row = retrieval_row(
        conversation,
        probe,
        &answer.provenance,
        answer.retrieved_characters,
    );
    let grade = rubric::grade_probe(gateway, probe, &answer.answer).await?;
    Ok((row, grade))
}

fn validated_digest(set: &ProbeSet) -> Result<String, DynError> {
    if set.format != beam::PROBE_SET_FORMAT {
        return Err(format!(
            "probe set format tag is {} and not {}",
            set.format,
            beam::PROBE_SET_FORMAT
        )
        .into());
    }
    if set.conversations.is_empty() {
        return Err("probe set carries no conversations".into());
    }
    beam::probe_set_digest(set)
}

fn retrieval_row(
    conversation: &Conversation,
    probe: &Probe,
    provenance: &[BenchCitation],
    context_characters: usize,
) -> ProbeRetrieval {
    let expected: BTreeSet<String> = beam::evidence_document_ids(conversation, probe)
        .into_iter()
        .collect();
    let mut retrieved = BTreeSet::new();
    let mut first_evidence_rank = None;
    for citation in provenance {
        for document in &citation.document_ids {
            if !expected.contains(document) {
                continue;
            }
            retrieved.insert(document.clone());
            first_evidence_rank = Some(
                first_evidence_rank.map_or(citation.rank, |rank: usize| rank.min(citation.rank)),
            );
        }
    }
    ProbeRetrieval {
        probe_id: probe.id.clone(),
        kind: probe.kind,
        evidence_expected: expected.len(),
        evidence_retrieved: retrieved.len(),
        first_evidence_rank,
        citations: provenance.len(),
        context_characters,
    }
}

fn summarize_retrieval(rows: Vec<ProbeRetrieval>) -> RetrievalSummary {
    let mut per_kind: BTreeMap<ProbeKind, usize> = BTreeMap::new();
    let mut recall_total = 0.0;
    let mut reciprocal_total = 0.0;
    let mut with_evidence = 0;
    let mut probes_without_evidence = 0;
    for row in &rows {
        *per_kind.entry(row.kind).or_default() += 1;
        if row.evidence_expected == 0 {
            continue;
        }
        with_evidence += 1;
        recall_total += row.evidence_retrieved as f64 / row.evidence_expected as f64;
        match row.first_evidence_rank {
            Some(rank) if rank > 0 => reciprocal_total += 1.0 / rank as f64,
            Some(_) | None => probes_without_evidence += 1,
        }
    }
    RetrievalSummary {
        probes: rows.len(),
        evidence_recall: mean(recall_total, with_evidence),
        mean_reciprocal_rank: mean(reciprocal_total, with_evidence),
        probes_without_evidence,
        per_kind_probes: per_kind.into_iter().collect(),
        rows,
    }
}

fn report(
    set: &ProbeSet,
    probe_set_digest: String,
    rows: Vec<ProbeRetrieval>,
    answered: usize,
    judged: Option<GradeSummary>,
) -> Result<BeamResult, DynError> {
    let total = rows.len();
    let judge_free = summarize_retrieval(rows);
    let failure = if probe_set_digest == fixture_digest()? {
        Some(FIXTURE_FAILURE.to_owned())
    } else if judged.is_none() {
        Some(JUDGE_FREE_FAILURE.to_owned())
    } else if answered != total {
        Some(format!("{answered} of {total} probes were answered"))
    } else {
        None
    };
    Ok(BeamResult {
        format: RESULT_FORMAT.to_owned(),
        probe_set_digest,
        source_digest: set.source_digest.clone(),
        conversations: set.conversations.len(),
        total,
        answered,
        encoder: ENCODER.to_owned(),
        reader_model: READER_MODEL.to_owned(),
        judge_model: JUDGE_MODEL.to_owned(),
        prompt_id: CRITERION_PROMPT_ID.to_owned(),
        judge_free,
        judged,
        complete: failure.is_none(),
        failure,
    })
}

fn fixture_digest() -> Result<String, DynError> {
    beam::probe_set_digest(&beam::fixture_probe_set()?)
}

fn mean(total: f64, count: usize) -> f64 {
    if count == 0 {
        0.0
    } else {
        total / count as f64
    }
}
