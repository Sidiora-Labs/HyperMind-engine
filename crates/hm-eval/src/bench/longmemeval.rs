#![allow(clippy::cast_precision_loss, clippy::missing_errors_doc)]

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const PINNED_JUDGE: &str = "openrouter/openai/gpt-4o-2024-08-06";
const PROMPT_VERSION: &str = "longmemeval-binary-v1";

#[derive(Clone, Debug, Deserialize)]
pub struct Example {
    pub question_id: String,
    pub question: String,
    pub answer: String,
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
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../eval/datasets/longmemeval");
    let dataset = std::env::var_os("LONGMEMEVAL_DATASET")
        .map_or_else(|| root.join("longmemeval_s_cleaned.json"), PathBuf::from);
    let predictions = std::env::var_os("LONGMEMEVAL_PREDICTIONS")
        .map_or_else(|| root.join("wave4-no-consolidation.jsonl"), PathBuf::from);
    if !dataset.exists() || !predictions.exists() {
        return Ok(BenchmarkResult::default());
    }
    let examples: Vec<Example> = serde_json::from_slice(&fs::read(dataset)?)?;
    let predictions = read_predictions(&predictions)?;
    let cache = root.join("judge-cache");
    fs::create_dir_all(&cache)?;
    let judge = GatewayJudge::from_environment()?;
    run(&examples, &predictions, &cache, judge.as_ref())
}

pub fn run(
    examples: &[Example],
    predictions: &[Prediction],
    cache_directory: &Path,
    judge: Option<&GatewayJudge>,
) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
    let predictions: BTreeMap<_, _> = predictions
        .iter()
        .map(|prediction| {
            (
                prediction.question_id.as_str(),
                prediction.hypothesis.as_str(),
            )
        })
        .collect();
    let mut result = BenchmarkResult {
        total: examples.len(),
        ..BenchmarkResult::default()
    };
    for example in examples {
        let Some(hypothesis) = predictions.get(example.question_id.as_str()) else {
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
    fn from_environment() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let Ok(api_key) = std::env::var("CENTRA_GATEWAY_API_KEY") else {
            return Ok(None);
        };
        let base = std::env::var("CENTRA_GATEWAY_URL")
            .unwrap_or_else(|_| "https://gateway.centra.ag/v1".to_owned());
        Ok(Some(Self {
            endpoint: format!("{}/chat/completions", base.trim_end_matches('/')),
            api_key,
            client: reqwest::blocking::Client::builder().build()?,
        }))
    }

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
                "temperature": 0,
                "max_tokens": 4,
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
