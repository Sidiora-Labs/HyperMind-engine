#![allow(clippy::cast_precision_loss, clippy::missing_errors_doc)]

use super::gateway::{
    Completion, DynError, Gateway, JUDGE_SETTINGS, MODEL_IDENTITY_KIND, READER_CANONICAL_MODEL,
    READER_CANONICAL_SNAPSHOT, READER_MODEL, READER_SETTINGS,
};
use super::pipeline::{BenchAnswer, BenchDocument, BenchQuestion, BenchRole, MemoryPipeline};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const PINNED_JUDGE: &str = super::gateway::JUDGE_MODEL;
const PROMPT_VERSION: &str = "longmemeval-binary-v1";
pub const DATASET_REVISION: &str = "98d7416c24c778c2fee6e6f3006e7a073259d48f";
pub const DATASET_SHA256: &str = "d6f21ea9d60a0d56f34a05b609c79c88a451d2ae03597821ea3d5a9678c3a442";
pub const SCORER_REVISION: &str = "9e0b455f4ef0e2ab8f2e582289761153549043fc";
pub const OFFICIAL_PROMPT_VERSION: &str = "longmemeval-official-9e0b455f-v1";
pub const EXPECTED_QUESTIONS: usize = 500;
const COST_BASIS: &str = "accounted_budget: gateway-reported charges plus retained conservative reservations; reservations are not actual billed cost";

#[derive(Clone, Debug, Deserialize)]
pub struct Example {
    pub question_id: String,
    pub question: String,
    #[serde(deserialize_with = "answer_text")]
    pub answer: String,
}

fn answer_text<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    match Value::deserialize(deserializer)? {
        Value::String(value) => Ok(value),
        Value::Number(value) => Ok(value.to_string()),
        _ => Err(serde::de::Error::custom(
            "benchmark answer must be text or a number",
        )),
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Prediction {
    pub question_id: String,
    #[serde(alias = "response", alias = "answer")]
    pub hypothesis: String,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BenchmarkResult {
    pub total: usize,
    pub evaluated: usize,
    pub correct: usize,
    pub cache_hits: usize,
}

impl BenchmarkResult {
    #[must_use]
    pub fn coverage(self) -> f64 {
        ratio(self.evaluated, self.total)
    }

    #[must_use]
    pub fn accuracy(self) -> f64 {
        ratio(self.correct, self.evaluated)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CachedResponse {
    model: String,
    prompt_version: String,
    correct: bool,
    response: String,
}

pub fn run_baseline() -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
    run_named_predictions("LONGMEMEVAL_PREDICTIONS", "wave4-no-consolidation.jsonl")
}

pub fn run_consolidation() -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
    run_named_predictions(
        "LONGMEMEVAL_CONSOLIDATED_PREDICTIONS",
        "wave6-consolidation.jsonl",
    )
}

fn run_named_predictions(
    environment_name: &str,
    default_name: &str,
) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../eval/datasets/longmemeval");
    let dataset = std::env::var_os("LONGMEMEVAL_DATASET")
        .map_or_else(|| root.join("longmemeval_s_cleaned.json"), PathBuf::from);
    let predictions =
        std::env::var_os(environment_name).map_or_else(|| root.join(default_name), PathBuf::from);
    if !dataset.exists() || !predictions.exists() {
        return Ok(BenchmarkResult::default());
    }
    let examples: Vec<Example> = serde_json::from_slice(&fs::read(dataset)?)?;
    let predictions = read_predictions(&predictions)?;
    let cache = root.join("judge-cache");
    fs::create_dir_all(&cache)?;
    run(&examples, &predictions, &cache, None)
}

pub fn run(
    examples: &[Example],
    predictions: &[Prediction],
    cache_directory: &Path,
    judge: Option<&GatewayJudge>,
) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
    let indexed: BTreeMap<_, _> = predictions
        .iter()
        .map(|prediction| {
            (
                prediction.question_id.as_str(),
                prediction.hypothesis.as_str(),
            )
        })
        .collect();
    if indexed.len() != predictions.len()
        || predictions
            .iter()
            .any(|prediction| prediction.hypothesis.trim().is_empty())
        || examples
            .iter()
            .map(|example| &example.question_id)
            .collect::<BTreeSet<_>>()
            .len()
            != examples.len()
    {
        return Err("benchmark contains duplicate question IDs or empty predictions".into());
    }
    let mut result = BenchmarkResult {
        total: examples.len(),
        ..BenchmarkResult::default()
    };
    for example in examples {
        let Some(hypothesis) = indexed.get(example.question_id.as_str()) else {
            continue;
        };
        let cache_key = cache_key(example, hypothesis);
        let path = cache_directory.join(format!("{cache_key}.json"));
        let response = if path.exists() {
            result.cache_hits += 1;
            serde_json::from_slice::<CachedResponse>(&fs::read(&path)?)?
        } else {
            let Some(judge) = judge else {
                continue;
            };
            let response = judge.evaluate(example, hypothesis)?;
            fs::write(&path, serde_json::to_vec_pretty(&response)?)?;
            response
        };
        if response.model != PINNED_JUDGE || response.prompt_version != PROMPT_VERSION {
            return Err("judge cache namespace mismatch".into());
        }
        result.evaluated += 1;
        result.correct += usize::from(response.correct);
    }
    Ok(result)
}

pub struct GatewayJudge {
    endpoint: String,
    api_key: String,
    client: reqwest::blocking::Client,
}

impl GatewayJudge {
    fn evaluate(
        &self,
        example: &Example,
        hypothesis: &str,
    ) -> Result<CachedResponse, Box<dyn std::error::Error>> {
        let prompt = format!(
            "Question: {}\nReference answer: {}\nCandidate answer: {}\nIs the candidate answer factually consistent with the reference? Reply only YES or NO.",
            example.question, example.answer, hypothesis
        );
        let response: serde_json::Value = self
            .client
            .post(&self.endpoint)
            .bearer_auth(&self.api_key)
            .json(&json!({
                "model": PINNED_JUDGE,
                "reasoning_effort": "none",
                "max_completion_tokens": 4,
                "messages": [{"role": "user", "content": prompt}],
            }))
            .send()?
            .error_for_status()?
            .json()?;
        let content = response["choices"][0]["message"]["content"]
            .as_str()
            .ok_or("judge response omitted content")?
            .trim()
            .to_owned();
        Ok(CachedResponse {
            model: PINNED_JUDGE.to_owned(),
            prompt_version: PROMPT_VERSION.to_owned(),
            correct: content.eq_ignore_ascii_case("yes"),
            response: content,
        })
    }
}

fn read_predictions(path: &Path) -> Result<Vec<Prediction>, Box<dyn std::error::Error>> {
    let bytes = fs::read(path)?;
    if path
        .extension()
        .is_some_and(|extension| extension == "json")
    {
        return Ok(serde_json::from_slice(&bytes)?);
    }
    String::from_utf8(bytes)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).map_err(Into::into))
        .collect()
}

fn cache_key(example: &Example, hypothesis: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    for field in [
        PINNED_JUDGE,
        PROMPT_VERSION,
        &example.question_id,
        &example.question,
        &example.answer,
        hypothesis,
    ] {
        hasher.update(&(field.len() as u64).to_le_bytes());
        hasher.update(field.as_bytes());
    }
    hasher.finalize().to_hex().to_string()
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct HistoryTurn {
    pub role: String,
    pub content: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FullExample {
    #[serde(flatten)]
    pub example: Example,
    pub question_type: String,
    pub question_date: String,
    pub haystack_dates: Vec<String>,
    pub haystack_session_ids: Vec<String>,
    pub haystack_sessions: Vec<Vec<HistoryTurn>>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct CategoryResult {
    pub total: usize,
    pub evaluated: usize,
    pub correct: usize,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct FullBenchmarkResult {
    pub diagnostic: bool,
    pub selected_question_ids: Vec<String>,
    pub question_outcomes: BTreeMap<String, bool>,
    pub dataset_revision: String,
    pub dataset_sha256: String,
    pub dataset_url: String,
    pub scorer_revision: String,
    pub prompt_version: String,
    pub reader_model: String,
    pub reader_canonical_model: String,
    pub reader_canonical_snapshot: String,
    pub model_identity_kind: String,
    pub judge_model: String,
    pub reader_settings: Value,
    pub judge_settings: Value,
    pub retrieval_mode: String,
    pub total: usize,
    pub generated: usize,
    pub evaluated: usize,
    pub correct: usize,
    pub reader_cache_hits: usize,
    pub judge_cache_hits: usize,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub reasoning_tokens: u64,
    pub cached_input_tokens: u64,
    pub cost_microusd: u64,
    pub new_cost_microusd: u64,
    pub cost_basis: String,
    pub reported_cost_microusd: u64,
    pub reserved_cost_microusd: u64,
    pub new_reported_cost_microusd: u64,
    pub new_reserved_cost_microusd: u64,
    pub by_category: BTreeMap<String, CategoryResult>,
    pub failure: Option<String>,
}

impl FullBenchmarkResult {
    #[must_use]
    pub fn coverage(&self) -> f64 {
        ratio(self.evaluated, self.total)
    }

    #[must_use]
    pub fn accuracy(&self) -> f64 {
        ratio(self.correct, self.evaluated)
    }

    #[must_use]
    pub fn qualified(&self) -> bool {
        !self.diagnostic
            && self.failure.is_none()
            && self.total == EXPECTED_QUESTIONS
            && self.generated == self.total
            && self.evaluated == self.total
            && self.accuracy() >= 0.90
    }

    fn record(&mut self, completion: &Completion, judge: bool) {
        COST_BASIS.clone_into(&mut self.cost_basis);
        self.input_tokens += completion.usage.input_tokens;
        self.output_tokens += completion.usage.output_tokens;
        self.reasoning_tokens += completion.usage.reasoning_tokens;
        self.cached_input_tokens += completion.usage.cached_input_tokens;
        self.cost_microusd += completion.usage.cost_microusd;
        if completion.usage.cost_source == "gateway_reported" {
            self.reported_cost_microusd += completion.usage.cost_microusd;
            if !completion.cached {
                self.new_reported_cost_microusd += completion.usage.cost_microusd;
            }
        } else {
            self.reserved_cost_microusd += completion.usage.cost_microusd;
            if !completion.cached {
                self.new_reserved_cost_microusd += completion.usage.cost_microusd;
            }
        }
        if !completion.cached {
            self.new_cost_microusd += completion.usage.cost_microusd;
        } else if judge {
            self.judge_cache_hits += 1;
        } else {
            self.reader_cache_hits += 1;
        }
    }
}

pub fn load_full_dataset(path: &Path) -> Result<Vec<FullExample>, DynError> {
    let bytes = fs::read(path)?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    if digest != DATASET_SHA256 {
        return Err(format!("LongMemEval-S artifact digest mismatch: {digest}").into());
    }
    let examples: Vec<FullExample> = serde_json::from_slice(&bytes)?;
    validate_full_dataset(&examples)?;
    Ok(examples)
}

pub fn validate_full_dataset(examples: &[FullExample]) -> Result<(), DynError> {
    let mut ids = BTreeSet::new();
    if examples.len() != EXPECTED_QUESTIONS {
        return Err("LongMemEval-S requires all 500 questions".into());
    }
    for example in examples {
        if example.example.question_id.is_empty()
            || !ids.insert(&example.example.question_id)
            || example.example.question.trim().is_empty()
            || example.example.answer.trim().is_empty()
            || example.question_date.is_empty()
        {
            return Err("LongMemEval question metadata is missing or duplicated".into());
        }
        official_judge_prompt(example, "validation")?;
        history_documents(example)?;
    }
    Ok(())
}

pub fn history_documents(example: &FullExample) -> Result<Vec<BenchDocument>, DynError> {
    let count = example.haystack_sessions.len();
    if count == 0
        || count != example.haystack_dates.len()
        || count != example.haystack_session_ids.len()
    {
        return Err("LongMemEval history session alignment is invalid".into());
    }
    let mut documents = Vec::new();
    for (session_index, session) in example.haystack_sessions.iter().enumerate() {
        let source_id = &example.haystack_session_ids[session_index];
        let conversation = format!("{session_index}:{source_id}");
        let date = &example.haystack_dates[session_index];
        if session.is_empty() || source_id.is_empty() || date.is_empty() {
            return Err("LongMemEval history session is empty".into());
        }
        for (turn_index, turn) in session.iter().enumerate() {
            let role = match turn.role.as_str() {
                "user" => BenchRole::User,
                "assistant" => BenchRole::Assistant,
                _ => return Err("unsupported LongMemEval history role".into()),
            };
            documents.push(BenchDocument {
                id: format!("{conversation}:{turn_index}"),
                conversation: conversation.clone(),
                date: date.clone(),
                role,
                speaker: turn.role.clone(),
                text: turn.content.clone(),
            });
        }
    }
    Ok(documents)
}

pub async fn run_full(gateway: &Gateway) -> Result<FullBenchmarkResult, DynError> {
    run_full_or_selected(gateway, None).await
}

pub async fn run_selected(
    gateway: &Gateway,
    question_ids: &[String],
) -> Result<FullBenchmarkResult, DynError> {
    if question_ids.is_empty()
        || question_ids.len() > 32
        || question_ids.iter().collect::<BTreeSet<_>>().len() != question_ids.len()
    {
        return Err("diagnostic requires between one and 32 unique question IDs".into());
    }
    run_full_or_selected(gateway, Some(question_ids)).await
}

#[allow(clippy::too_many_lines)]
async fn run_full_or_selected(
    gateway: &Gateway,
    question_ids: Option<&[String]>,
) -> Result<FullBenchmarkResult, DynError> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../eval/datasets/longmemeval");
    let dataset = std::env::var_os("LONGMEMEVAL_DATASET")
        .map_or_else(|| root.join("longmemeval_s_cleaned.json"), PathBuf::from);
    let mut examples = load_full_dataset(&dataset)?;
    if let Some(ids) = question_ids {
        examples.retain(|example| ids.contains(&example.example.question_id));
        if examples.len() != ids.len() {
            return Err("diagnostic contains an unknown question ID".into());
        }
    }
    let mut report = FullBenchmarkResult {
        diagnostic: question_ids.is_some(),
        selected_question_ids: question_ids.unwrap_or_default().to_vec(),
        question_outcomes: BTreeMap::new(),
        dataset_revision: DATASET_REVISION.to_owned(),
        dataset_sha256: DATASET_SHA256.to_owned(),
        dataset_url: format!(
            "https://huggingface.co/datasets/xiaowu0162/longmemeval-cleaned/resolve/{DATASET_REVISION}/longmemeval_s_cleaned.json"
        ),
        scorer_revision: SCORER_REVISION.to_owned(),
        prompt_version: OFFICIAL_PROMPT_VERSION.to_owned(),
        reader_model: READER_MODEL.to_owned(),
        reader_canonical_model: READER_CANONICAL_MODEL.to_owned(),
        reader_canonical_snapshot: READER_CANONICAL_SNAPSHOT.to_owned(),
        model_identity_kind: MODEL_IDENTITY_KIND.to_owned(),
        judge_model: PINNED_JUDGE.to_owned(),
        reader_settings: serde_json::to_value(READER_SETTINGS)?,
        judge_settings: serde_json::to_value(JUDGE_SETTINGS)?,
        retrieval_mode: "lexical_only".to_owned(),
        total: examples.len(),
        generated: 0,
        evaluated: 0,
        correct: 0,
        reader_cache_hits: 0,
        judge_cache_hits: 0,
        input_tokens: 0,
        output_tokens: 0,
        reasoning_tokens: 0,
        cached_input_tokens: 0,
        cost_microusd: 0,
        new_cost_microusd: 0,
        cost_basis: COST_BASIS.to_owned(),
        reported_cost_microusd: 0,
        reserved_cost_microusd: 0,
        new_reported_cost_microusd: 0,
        new_reserved_cost_microusd: 0,
        by_category: BTreeMap::new(),
        failure: None,
    };
    for example in &examples {
        report
            .by_category
            .entry(category(example))
            .or_default()
            .total += 1;
    }
    fs::create_dir_all(&root)?;
    fs::create_dir_all(root.join("wave7-records"))?;
    let mut remaining = examples.into_iter();
    let mut active = tokio::task::JoinSet::new();
    let mut completed = BTreeMap::new();
    loop {
        while report.failure.is_none() && active.len() < 4 {
            let Some(example) = remaining.next() else {
                break;
            };
            let gateway = gateway.clone();
            let root = root.clone();
            active.spawn(async move { run_question(&gateway, &example, &root).await });
        }
        let Some(result) = active.join_next().await else {
            break;
        };
        match result {
            Ok(progress) => {
                if let Some(answer) = &progress.answer {
                    report.record(&answer.reader, false);
                    report.generated += usize::from(
                        answer.question_id == progress.question_id
                            && !answer.answer.trim().is_empty(),
                    );
                }
                if let Some(judged) = &progress.judged {
                    report.record(judged, true);
                    if !judged.text.trim().is_empty() {
                        report.evaluated += 1;
                        let correct =
                            usize::from(judged.text.trim().to_lowercase().contains("yes"));
                        report.correct += correct;
                        report
                            .question_outcomes
                            .insert(progress.question_id.clone(), correct == 1);
                        let category = report
                            .by_category
                            .get_mut(&progress.category)
                            .expect("validated category");
                        category.evaluated += 1;
                        category.correct += correct;
                    }
                }
                if let Some(error) = &progress.failure {
                    report
                        .failure
                        .get_or_insert_with(|| format!("{}: {error}", progress.question_id));
                }
                completed.insert(progress.question_id.clone(), progress);
            }
            Err(error) => {
                report
                    .failure
                    .get_or_insert_with(|| format!("benchmark worker failed: {error}"));
            }
        }
        fs::write(
            root.join(if report.diagnostic {
                "wave7-diagnostic-progress.json"
            } else {
                "wave7-progress.json"
            }),
            serde_json::to_vec_pretty(&report)?,
        )?;
        eprintln!(
            "LongMemEval{}: generated={}/{} judged={}/{} correct={} new_accounted_budget_microusd={}",
            if report.diagnostic { " diagnostic" } else { "" },
            report.generated,
            report.total,
            report.evaluated,
            report.total,
            report.correct,
            report.new_cost_microusd
        );
    }
    if report.failure.is_none() && report.evaluated == EXPECTED_QUESTIONS {
        write_aggregate(
            &root.join("wave7-predictions.jsonl"),
            completed
                .values()
                .map(|progress| {
                    serde_json::to_value(progress.answer.as_ref().expect("completed answer"))
                })
                .collect::<Result<Vec<_>, _>>()?,
        )?;
        write_aggregate(
            &root.join("wave7-judgments.jsonl"),
            completed.values().map(judgment_record).collect(),
        )?;
    }
    Ok(report)
}

struct QuestionProgress {
    question_id: String,
    category: String,
    answer: Option<BenchAnswer>,
    judged: Option<Completion>,
    failure: Option<String>,
}

async fn run_question(gateway: &Gateway, example: &FullExample, root: &Path) -> QuestionProgress {
    let mut progress = QuestionProgress {
        question_id: example.example.question_id.clone(),
        category: category(example),
        answer: None,
        judged: None,
        failure: None,
    };
    if let Err(error) = evaluate_question(gateway, example, root, &mut progress).await {
        progress.failure = Some(error.to_string());
    }
    progress
}

async fn evaluate_question(
    gateway: &Gateway,
    example: &FullExample,
    root: &Path,
    progress: &mut QuestionProgress,
) -> Result<(), DynError> {
    let documents = history_documents(example)?;
    let pipeline = MemoryPipeline::open(
        root.join("stores"),
        DATASET_SHA256,
        &example.example.question_id,
        &documents,
    )
    .await?;
    let answer = pipeline
        .answer(
            gateway,
            &BenchQuestion {
                id: example.example.question_id.clone(),
                text: example.example.question.clone(),
                date: Some(example.question_date.clone()),
            },
        )
        .await;
    let closed = pipeline.close().await;
    progress.answer = Some(answer?);
    closed?;
    let answer = progress.answer.as_ref().expect("reader answer");
    if answer.question_id != example.example.question_id || answer.answer.trim().is_empty() {
        return Err("reader returned an empty answer or wrong question ID".into());
    }
    let prompt = official_judge_prompt(example, &answer.answer)?;
    let prediction_path = root.join("wave7-records").join(format!(
        "{}-{}-{}.prediction.json",
        example.example.question_id, answer.question_digest, answer.reader.request_hash
    ));
    write_immutable(
        &prediction_path,
        &serde_json::to_value(progress.answer.as_ref().expect("reader answer"))?,
    )?;
    let judged = gateway
        .complete_with_settings(
            PINNED_JUDGE,
            OFFICIAL_PROMPT_VERSION,
            "",
            &prompt,
            JUDGE_SETTINGS,
        )
        .await?;
    let judgment_path = root.join("wave7-records").join(format!(
        "{}-{}.judgment.json",
        example.example.question_id, judged.request_hash
    ));
    progress.judged = Some(judged);
    if progress
        .judged
        .as_ref()
        .expect("judge response")
        .text
        .trim()
        .is_empty()
    {
        return Err("judge returned an empty response".into());
    }
    write_immutable(&judgment_path, &judgment_record(progress))?;
    Ok(())
}

fn judgment_record(progress: &QuestionProgress) -> Value {
    let judged = progress.judged.as_ref().expect("completed judgment");
    json!({
        "question_id": progress.question_id,
        "correct": judged.text.trim().to_lowercase().contains("yes"),
        "judge": judged, "prompt_version": OFFICIAL_PROMPT_VERSION,
        "dataset_sha256": DATASET_SHA256,
    })
}

fn write_immutable(path: &Path, value: &Value) -> Result<(), DynError> {
    let mut temporary =
        tempfile::NamedTempFile::new_in(path.parent().ok_or("record parent is missing")?)?;
    temporary.write_all(&serde_json::to_vec_pretty(value)?)?;
    temporary.as_file().sync_all()?;
    match temporary.persist_noclobber(path) {
        Ok(_) => Ok(()),
        Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
            let mut previous: Value = serde_json::from_slice(&fs::read(path)?)?;
            let mut current = value.clone();
            for record in [&mut previous, &mut current] {
                for field in ["reader", "judge"] {
                    if let Some(completion) = record.get_mut(field).and_then(Value::as_object_mut) {
                        completion.remove("cached");
                    }
                }
            }
            if previous != current {
                return Err("immutable benchmark record differs from cached response".into());
            }
            Ok(())
        }
        Err(error) => Err(error.error.into()),
    }
}

fn write_aggregate(path: &Path, records: Vec<Value>) -> Result<(), DynError> {
    let temporary = path.with_extension("jsonl.pending");
    let mut file = fs::File::create(&temporary)?;
    for record in records {
        writeln!(file, "{}", serde_json::to_string(&record)?)?;
    }
    file.sync_all()?;
    fs::rename(temporary, path)?;
    Ok(())
}

fn category(example: &FullExample) -> String {
    if example.example.question_id.contains("_abs") {
        "abstention".to_owned()
    } else {
        example.question_type.clone()
    }
}

// Ported from LongMemEval src/evaluation/evaluate_qa.py, revision
// 9e0b455f4ef0e2ab8f2e582289761153549043fc. MIT License, Copyright (c) 2024 Di Wu.
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
pub fn official_judge_prompt(example: &FullExample, hypothesis: &str) -> Result<String, DynError> {
    let (instruction, answer_label, conclusion) = if example.example.question_id.contains("_abs") {
        (
            "I will give you an unanswerable question, an explanation, and a response from a model. Please answer yes if the model correctly identifies the question as unanswerable. The model could say that the information is incomplete, or some other information is given but the asked information is not.",
            "Explanation",
            "Does the model correctly identify the question as unanswerable? Answer yes or no only.",
        )
    } else {
        let instruction = match example.question_type.as_str() {
            "single-session-user" | "single-session-assistant" | "multi-session" => {
                "I will give you a question, a correct answer, and a response from a model. Please answer yes if the response contains the correct answer. Otherwise, answer no. If the response is equivalent to the correct answer or contains all the intermediate steps to get the correct answer, you should also answer yes. If the response only contains a subset of the information required by the answer, answer no. "
            }
            "temporal-reasoning" => {
                "I will give you a question, a correct answer, and a response from a model. Please answer yes if the response contains the correct answer. Otherwise, answer no. If the response is equivalent to the correct answer or contains all the intermediate steps to get the correct answer, you should also answer yes. If the response only contains a subset of the information required by the answer, answer no. In addition, do not penalize off-by-one errors for the number of days. If the question asks for the number of days/weeks/months, etc., and the model makes off-by-one errors (e.g., predicting 19 days when the answer is 18), the model's response is still correct. "
            }
            "knowledge-update" => {
                "I will give you a question, a correct answer, and a response from a model. Please answer yes if the response contains the correct answer. Otherwise, answer no. If the response contains some previous information along with an updated answer, the response should be considered as correct as long as the updated answer is the required answer."
            }
            "single-session-preference" => {
                "I will give you a question, a rubric for desired personalized response, and a response from a model. Please answer yes if the response satisfies the desired response. Otherwise, answer no. The model does not need to reflect all the points in the rubric. The response is correct as long as it recalls and utilizes the user's personal information correctly."
            }
            _ => return Err("unknown LongMemEval question type".into()),
        };
        (
            instruction,
            if example.question_type == "single-session-preference" {
                "Rubric"
            } else {
                "Correct Answer"
            },
            "Is the model response correct? Answer yes or no only.",
        )
    };
    Ok(format!(
        "{instruction}\n\nQuestion: {}\n\n{answer_label}: {}\n\nModel Response: {hypothesis}\n\n{conclusion}",
        example.example.question, example.example.answer
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn example() -> FullExample {
        serde_json::from_value(json!({
            "question_id": "case", "question_type": "multi-session",
            "question": "How many days?", "answer": 18,
            "question_date": "2023/05/30 (Tue) 23:40",
            "answer_session_ids": ["oracle_annotation_not_history"],
            "haystack_dates": ["2023/05/20 (Sat) 10:00"],
            "haystack_session_ids": ["history"],
            "haystack_sessions": [[{"role": "user", "content": "I left on Tuesday.",
                "has_answer": "oracle_annotation_not_history"}]],
        }))
        .unwrap()
    }

    #[test]
    fn scorer_matches_seven_verbatim_upstream_vectors() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/longmemeval-official-prompts.json"
        ))
        .unwrap();
        assert!(
            fixture["source"]
                .as_str()
                .unwrap()
                .contains(SCORER_REVISION)
        );
        let vectors = fixture["vectors"].as_array().unwrap();
        assert_eq!(vectors.len(), 7);
        for vector in vectors {
            let mut example = example();
            example.example.question_id = vector["question_id"].as_str().unwrap().into();
            example.example.question = vector["question"].as_str().unwrap().into();
            example.example.answer = vector["answer"].as_str().unwrap().into();
            example.question_type = vector["question_type"].as_str().unwrap().into();
            assert_eq!(
                official_judge_prompt(&example, vector["hypothesis"].as_str().unwrap()).unwrap(),
                vector["prompt"].as_str().unwrap()
            );
        }
    }

    #[test]
    fn reader_history_excludes_oracle_annotations_and_preserves_empty_and_repeated_sessions() {
        let mut example = example();
        assert_eq!(example.example.answer, "18");
        example.example.answer = "reference_answer_not_history".into();
        example
            .haystack_dates
            .push(example.haystack_dates[0].clone());
        example
            .haystack_session_ids
            .push(example.haystack_session_ids[0].clone());
        example.haystack_sessions.push(vec![HistoryTurn {
            role: "assistant".into(),
            content: String::new(),
        }]);
        let documents = history_documents(&example).unwrap();
        assert_eq!(documents.len(), 2);
        assert_ne!(documents[0].id, documents[1].id);
        assert_ne!(documents[0].conversation, documents[1].conversation);
        assert_eq!(documents[0].text, "I left on Tuesday.");
        assert!(documents[1].text.is_empty());
        let encoded = serde_json::to_string(&documents).unwrap();
        assert!(!encoded.contains("oracle_annotation_not_history"));
        assert!(!encoded.contains("reference_answer_not_history"));
    }

    #[test]
    fn full_coverage_requires_exactly_five_hundred_unique_nonempty_questions() {
        let mut examples: Vec<_> = (0..EXPECTED_QUESTIONS)
            .map(|index| {
                let mut example = example();
                example.example.question_id = format!("question-{index}");
                example
            })
            .collect();
        validate_full_dataset(&examples).unwrap();
        assert!(validate_full_dataset(&examples[..499]).is_err());
        examples[499].example.question_id = examples[0].example.question_id.clone();
        assert!(validate_full_dataset(&examples).is_err());
        examples[499].example.question_id = "question-499".into();
        examples[499].example.question.clear();
        assert!(validate_full_dataset(&examples).is_err());
        let mut report = FullBenchmarkResult::default();
        assert!(!report.qualified());
        report.total = 500;
        report.generated = 500;
        report.evaluated = 499;
        report.correct = 499;
        assert!(!report.qualified());
        report.evaluated = 500;
        report.correct = 450;
        assert!(report.qualified());
        report.diagnostic = true;
        assert!(!report.qualified());
        report.diagnostic = false;
        report.correct = 449;
        assert!(!report.qualified());
        report.correct = 450;
        report.failure = Some("interrupted".into());
        assert!(!report.qualified());
    }

    #[test]
    fn duplicate_predictions_and_unpinned_artifacts_fail_closed() {
        let directory = tempfile::tempdir().unwrap();
        let example = example().example;
        let prediction = Prediction {
            question_id: example.question_id.clone(),
            hypothesis: "19 days".into(),
        };
        assert!(
            run(
                std::slice::from_ref(&example),
                &[prediction.clone(), prediction],
                directory.path(),
                None
            )
            .is_err()
        );
        assert!(
            run(
                &[example],
                &[Prediction {
                    question_id: "case".into(),
                    hypothesis: String::new()
                }],
                directory.path(),
                None
            )
            .is_err()
        );
        let path = directory.path().join("dataset.json");
        fs::write(&path, "[]").unwrap();
        assert!(
            load_full_dataset(&path)
                .unwrap_err()
                .to_string()
                .contains("digest mismatch")
        );
    }

    #[test]
    fn immutable_records_preserve_original_bytes_and_allow_cache_replay() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("question.json");
        let original = json!({"question_id":"case", "reader":{"text":"answer", "cached":false}});
        write_immutable(&path, &original).unwrap();
        let bytes = fs::read(&path).unwrap();
        let mut replay = original;
        replay["reader"]["cached"] = json!(true);
        write_immutable(&path, &replay).unwrap();
        assert_eq!(fs::read(&path).unwrap(), bytes);
        replay["reader"]["text"] = json!("changed answer");
        assert!(write_immutable(&path, &replay).is_err());
        assert_eq!(fs::read(&path).unwrap(), bytes);
    }

    #[test]
    fn reported_cost_and_retained_reservations_remain_distinct_on_cache_replay() {
        let mut report = FullBenchmarkResult::default();
        for (cost, source, cached, judge) in [
            (7, "gateway_reported", false, false),
            (
                100,
                "retained_reservation_missing_gateway_cost",
                false,
                true,
            ),
            (11, "gateway_reported", true, true),
            (
                200,
                "retained_reservation_missing_gateway_cost",
                true,
                false,
            ),
        ] {
            report.record(
                &Completion {
                    text: "cost accounting vector".into(),
                    model: READER_MODEL.into(),
                    response_model: READER_CANONICAL_MODEL.into(),
                    canonical_model: READER_CANONICAL_MODEL.into(),
                    canonical_snapshot: READER_CANONICAL_SNAPSHOT.into(),
                    model_identity_kind: MODEL_IDENTITY_KIND.into(),
                    model_catalog_source: String::new(),
                    model_catalog_observed_at: String::new(),
                    request_hash: String::new(),
                    cached,
                    usage: super::super::gateway::Usage {
                        cost_microusd: cost,
                        cost_source: source.into(),
                        ..super::super::gateway::Usage::default()
                    },
                },
                judge,
            );
        }
        assert_eq!(report.cost_microusd, 318);
        assert_eq!(report.new_cost_microusd, 107);
        assert_eq!(report.reported_cost_microusd, 18);
        assert_eq!(report.reserved_cost_microusd, 300);
        assert_eq!(report.new_reported_cost_microusd, 7);
        assert_eq!(report.new_reserved_cost_microusd, 100);
        assert_eq!(report.reader_cache_hits, 1);
        assert_eq!(report.judge_cache_hits, 1);
        assert!(
            report
                .cost_basis
                .contains("reservations are not actual billed cost")
        );
        let encoded = serde_json::to_value(&report).unwrap();
        assert_eq!(encoded["reported_cost_microusd"], 18);
        assert_eq!(encoded["reserved_cost_microusd"], 300);
    }

    #[test]
    #[ignore = "requires the pinned 264 MB official LongMemEval-S artifact"]
    fn pinned_official_dataset_has_full_coverage_and_valid_reader_histories() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../eval/datasets/longmemeval/longmemeval_s_cleaned.json");
        let examples = load_full_dataset(&path).unwrap();
        assert_eq!(examples.len(), 500);
        let mut turns = 0;
        let mut empty = 0;
        for example in &examples {
            let documents = history_documents(example).unwrap();
            turns += documents.len();
            empty += documents
                .iter()
                .filter(|document| document.text.trim().is_empty())
                .count();
            assert_eq!(
                documents
                    .iter()
                    .map(|document| &document.id)
                    .collect::<BTreeSet<_>>()
                    .len(),
                documents.len()
            );
        }
        assert_eq!(turns, 246_750);
        assert_eq!(empty, 12);
    }
}
