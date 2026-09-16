#![allow(clippy::missing_errors_doc)]

use ort::session::Session;
use ort::value::Tensor;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::Mutex;
use tokenizers::{EncodeInput, Tokenizer, TruncationParams};

pub const MODEL_REVISION: &str = "233902d25c440f23af6f7d6e94d2946bac0bee0a";
pub const MODEL_SHA256: &str = "c80a8b34256ea453093d612e3ac48d3d965a0c0a48c7906709af8b8e28461bf9";
pub const TOKENIZER_SHA256: &str =
    "d241a60d5e8f04cc1b2b3e9ef7a4921b27bf526d9f6050ab90f9267a1f9e5c66";
pub const TOP_K: usize = 32;
const MAXIMUM_TOKENS: usize = 512;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DeploymentConfig {
    pub enabled: bool,
    pub deployment_id: String,
    pub evidence: Option<EvaluationEvidence>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvaluationEvidence {
    pub producer: String,
    pub deployment_id: String,
    pub model_sha256: String,
    pub tokenizer_sha256: String,
    pub evaluation_id: String,
    pub evaluated_queries: u64,
    pub baseline_mrr_at_10: f64,
    pub reranked_mrr_at_10: f64,
}

impl EvaluationEvidence {
    #[must_use]
    pub fn justifies(&self, deployment_id: &str) -> bool {
        self.producer == "hm-eval"
            && !deployment_id.is_empty()
            && self.deployment_id == deployment_id
            && self.model_sha256 == MODEL_SHA256
            && self.tokenizer_sha256 == TOKENIZER_SHA256
            && !self.evaluation_id.is_empty()
            && self.evaluated_queries > 0
            && (0.0..=1.0).contains(&self.baseline_mrr_at_10)
            && (0.0..=1.0).contains(&self.reranked_mrr_at_10)
            && self.reranked_mrr_at_10 > self.baseline_mrr_at_10
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RerankError {
    UnqualifiedDeployment,
    InvalidArgument,
    Artifact(String),
    DigestMismatch,
    Tokenizer(String),
    Runtime(String),
    Shape,
}

pub struct DeploymentReranker {
    model: Option<CrossEncoder>,
}

impl DeploymentReranker {
    pub fn open(
        config: &DeploymentConfig,
        model_path: &Path,
        tokenizer_path: &Path,
    ) -> Result<Self, RerankError> {
        if !config.enabled {
            return Ok(Self { model: None });
        }
        if !config
            .evidence
            .as_ref()
            .is_some_and(|evidence| evidence.justifies(&config.deployment_id))
        {
            return Err(RerankError::UnqualifiedDeployment);
        }
        Ok(Self {
            model: Some(CrossEncoder::open_for_evaluation(
                model_path,
                tokenizer_path,
            )?),
        })
    }

    #[must_use]
    pub fn enabled(&self) -> bool {
        self.model.is_some()
    }

    pub fn rank(&self, query: &str, passages: &[&str]) -> Result<Vec<usize>, RerankError> {
        match &self.model {
            Some(model) => model.rank_top_32(query, passages),
            None => Ok((0..passages.len()).collect()),
        }
    }
}

pub struct CrossEncoder {
    tokenizer: Tokenizer,
    session: Mutex<Session>,
}

impl CrossEncoder {
    pub fn open_for_evaluation(
        model_path: &Path,
        tokenizer_path: &Path,
    ) -> Result<Self, RerankError> {
        verify_digest(model_path, MODEL_SHA256)?;
        verify_digest(tokenizer_path, TOKENIZER_SHA256)?;
        let mut tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|error| RerankError::Tokenizer(error.to_string()))?;
        tokenizer
            .with_truncation(Some(TruncationParams {
                max_length: MAXIMUM_TOKENS,
                ..TruncationParams::default()
            }))
            .map_err(|error| RerankError::Tokenizer(error.to_string()))?;
        let session = Session::builder()
            .map_err(|error| RerankError::Runtime(error.to_string()))?
            .with_intra_threads(1)
            .map_err(|error| RerankError::Runtime(error.to_string()))?
            .commit_from_file(model_path)
            .map_err(|error| RerankError::Runtime(error.to_string()))?;
        Ok(Self {
            tokenizer,
            session: Mutex::new(session),
        })
    }

    pub fn rank_top_32(&self, query: &str, passages: &[&str]) -> Result<Vec<usize>, RerankError> {
        if query.trim().is_empty()
            || passages
                .iter()
                .take(TOP_K)
                .any(|text| text.trim().is_empty())
        {
            return Err(RerankError::InvalidArgument);
        }
        let mut scored = passages
            .iter()
            .take(TOP_K)
            .enumerate()
            .map(|(index, passage)| self.score(query, passage).map(|score| (index, score)))
            .collect::<Result<Vec<_>, _>>()?;
        scored.sort_by(|left, right| {
            right
                .1
                .total_cmp(&left.1)
                .then_with(|| left.0.cmp(&right.0))
        });
        Ok(scored
            .into_iter()
            .map(|(index, _)| index)
            .chain(TOP_K..passages.len())
            .collect())
    }

    fn score(&self, query: &str, passage: &str) -> Result<f32, RerankError> {
        let encoding = self
            .tokenizer
            .encode(EncodeInput::Dual(query.into(), passage.into()), true)
            .map_err(|error| RerankError::Tokenizer(error.to_string()))?;
        let length = encoding.get_ids().len();
        if length == 0 || length > MAXIMUM_TOKENS {
            return Err(RerankError::Shape);
        }
        let tensor = |values: &[u32]| {
            Tensor::from_array((
                [1, length],
                values
                    .iter()
                    .map(|value| i64::from(*value))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ))
            .map_err(|error| RerankError::Runtime(error.to_string()))
        };
        let mut session = self
            .session
            .lock()
            .map_err(|_| RerankError::Runtime("ONNX session lock poisoned".to_owned()))?;
        let outputs = session
            .run(ort::inputs! {
                "input_ids" => tensor(encoding.get_ids())?,
                "attention_mask" => tensor(encoding.get_attention_mask())?,
                "token_type_ids" => tensor(encoding.get_type_ids())?,
            })
            .map_err(|error| RerankError::Runtime(error.to_string()))?;
        let (shape, logits) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|error| RerankError::Runtime(error.to_string()))?;
        if shape.len() != 2
            || shape[0] != 1
            || shape[1] != 1
            || logits.len() != 1
            || !logits[0].is_finite()
        {
            return Err(RerankError::Shape);
        }
        Ok(logits[0])
    }
}

fn verify_digest(path: &Path, expected: &str) -> Result<(), RerankError> {
    let bytes = std::fs::read(path).map_err(|error| RerankError::Artifact(error.to_string()))?;
    if format!("{:x}", Sha256::digest(bytes)) != expected {
        return Err(RerankError::DigestMismatch);
    }
    Ok(())
}
