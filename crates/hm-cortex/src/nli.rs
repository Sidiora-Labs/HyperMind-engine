#![allow(clippy::missing_errors_doc)]

use ort::session::Session;
use ort::value::Tensor;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tokenizers::{EncodeInput, Tokenizer};

pub const MODEL_REVISION: &str = "80dd65bdcda723aee9ed855dc1c3082d8b3e6842";
pub const MODEL_SHA256: &str = "03c2221313dc0c3eac9cec1f746d1319d33f2c2901fcce1c0f08f4daac9b6dae";
pub const TOKENIZER_SHA256: &str =
    "4b4f60231058db4b5794e7b124bb7945bc8ade6719282de4d2e0372ee527b929";
const MODEL_URL: &str = "https://huggingface.co/cross-encoder/nli-deberta-v3-small/resolve/80dd65bdcda723aee9ed855dc1c3082d8b3e6842/onnx/model_quint8_avx2.onnx";
const TOKENIZER_URL: &str = "https://huggingface.co/cross-encoder/nli-deberta-v3-small/resolve/80dd65bdcda723aee9ed855dc1c3082d8b3e6842/tokenizer.json";
const MAXIMUM_TOKENS: usize = 512;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NliScores {
    pub contradiction: f32,
    pub entailment: f32,
    pub neutral: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NliVerdict {
    Genuine,
    Tension,
    Complementary,
    Unrelated,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BidirectionalNli {
    pub forward: NliScores,
    pub reverse: NliScores,
    pub verdict: NliVerdict,
    pub confidence: f32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NliError {
    InvalidArgument,
    Download(String),
    Digest { path: PathBuf },
    Tokenizer(String),
    Runtime(String),
    Shape,
}

pub struct NliModel {
    tokenizer: Tokenizer,
    session: Mutex<Session>,
}

impl NliModel {
    pub fn download(cache_root: &Path) -> Result<Self, NliError> {
        let directory = cache_root
            .join("cross-encoder--nli-deberta-v3-small")
            .join(MODEL_REVISION);
        std::fs::create_dir_all(&directory)
            .map_err(|error| NliError::Download(error.to_string()))?;
        let model = directory.join("model_quint8_avx2.onnx");
        let tokenizer = directory.join("tokenizer.json");
        ensure_artifact(&model, MODEL_URL, MODEL_SHA256)?;
        ensure_artifact(&tokenizer, TOKENIZER_URL, TOKENIZER_SHA256)?;
        Self::open(&model, &tokenizer)
    }

    pub fn open(model_path: &Path, tokenizer_path: &Path) -> Result<Self, NliError> {
        verify_digest(model_path, MODEL_SHA256)?;
        verify_digest(tokenizer_path, TOKENIZER_SHA256)?;
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|error| NliError::Tokenizer(error.to_string()))?;
        let session = Session::builder()
            .and_then(|mut builder| builder.commit_from_file(model_path))
            .map_err(|error| NliError::Runtime(error.to_string()))?;
        Ok(Self {
            tokenizer,
            session: Mutex::new(session),
        })
    }

    pub fn classify(&self, premise: &str, hypothesis: &str) -> Result<NliScores, NliError> {
        if premise.is_empty() || hypothesis.is_empty() {
            return Err(NliError::InvalidArgument);
        }
        let encoding = self
            .tokenizer
            .encode(EncodeInput::Dual(premise.into(), hypothesis.into()), true)
            .map_err(|error| NliError::Tokenizer(error.to_string()))?;
        let length = encoding.get_ids().len().min(MAXIMUM_TOKENS);
        if length == 0 {
            return Err(NliError::Shape);
        }
        let input_ids: Vec<i64> = encoding.get_ids()[..length]
            .iter()
            .map(|value| i64::from(*value))
            .collect();
        let attention_mask: Vec<i64> = encoding.get_attention_mask()[..length]
            .iter()
            .map(|value| i64::from(*value))
            .collect();
        let mut session = self
            .session
            .lock()
            .map_err(|_| NliError::Runtime("ONNX session lock poisoned".to_owned()))?;
        let outputs = session
            .run(ort::inputs! {
                "input_ids" => Tensor::from_array(([1, length], input_ids.into_boxed_slice()))
                    .map_err(|error| NliError::Runtime(error.to_string()))?,
                "attention_mask" => Tensor::from_array(([1, length], attention_mask.into_boxed_slice()))
                    .map_err(|error| NliError::Runtime(error.to_string()))?,
            })
            .map_err(|error| NliError::Runtime(error.to_string()))?;
        let (shape, logits) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|error| NliError::Runtime(error.to_string()))?;
        if shape.len() != 2 || shape[0] != 1 || shape[1] != 3 || logits.len() != 3 {
            return Err(NliError::Shape);
        }
        let probabilities = softmax([logits[0], logits[1], logits[2]]);
        Ok(NliScores {
            contradiction: probabilities[0],
            entailment: probabilities[1],
            neutral: probabilities[2],
        })
    }

    pub fn classify_both(&self, left: &str, right: &str) -> Result<BidirectionalNli, NliError> {
        let forward = self.classify(left, right)?;
        let reverse = self.classify(right, left)?;
        Ok(combine(forward, reverse))
    }
}

fn combine(forward: NliScores, reverse: NliScores) -> BidirectionalNli {
    let contradiction_floor = forward.contradiction.min(reverse.contradiction);
    let contradiction_peak = forward.contradiction.max(reverse.contradiction);
    let entailment_peak = forward.entailment.max(reverse.entailment);
    let neutral_floor = forward.neutral.min(reverse.neutral);
    let (verdict, confidence) = if contradiction_floor >= 0.8 {
        (NliVerdict::Genuine, contradiction_floor)
    } else if contradiction_peak >= 0.5 {
        (NliVerdict::Tension, contradiction_peak)
    } else if entailment_peak >= 0.5 {
        (NliVerdict::Complementary, entailment_peak)
    } else {
        (NliVerdict::Unrelated, neutral_floor)
    };
    BidirectionalNli {
        forward,
        reverse,
        verdict,
        confidence,
    }
}

fn softmax(logits: [f32; 3]) -> [f32; 3] {
    let maximum = logits.into_iter().fold(f32::NEG_INFINITY, f32::max);
    let values = logits.map(|value| (value - maximum).exp());
    let sum = values.into_iter().sum::<f32>();
    values.map(|value| value / sum)
}

fn ensure_artifact(path: &Path, url: &str, expected_sha256: &str) -> Result<(), NliError> {
    if path.exists() {
        return verify_digest(path, expected_sha256);
    }
    let response = reqwest::blocking::get(url)
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(|error| NliError::Download(error.to_string()))?;
    let bytes = response
        .bytes()
        .map_err(|error| NliError::Download(error.to_string()))?;
    let temporary = path.with_extension("partial");
    std::fs::write(&temporary, &bytes).map_err(|error| NliError::Download(error.to_string()))?;
    verify_digest(&temporary, expected_sha256)?;
    std::fs::rename(&temporary, path).map_err(|error| NliError::Download(error.to_string()))?;
    Ok(())
}

fn verify_digest(path: &Path, expected_sha256: &str) -> Result<(), NliError> {
    let bytes = std::fs::read(path).map_err(|error| NliError::Download(error.to_string()))?;
    let digest = format!("{:x}", Sha256::digest(bytes));
    if digest == expected_sha256 {
        Ok(())
    } else {
        Err(NliError::Digest {
            path: path.to_path_buf(),
        })
    }
}
