use hm_compose::tokens::{FallbackWeights, TokenCounter};
use hm_core::{Error, ErrorCode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, OnceLock, RwLock},
};

const MAX_ARTIFACT_BYTES: u64 = 32 * 1024 * 1024;
static REGISTRY: OnceLock<Result<RwLock<BTreeMap<String, BoundTokenCounter>>, ErrorCode>> =
    OnceLock::new();
fn error(code: ErrorCode) -> Error {
    Error::new(code)
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifiedTokenizerConfig {
    pub model_id: String,
    pub model_revision: String,
    pub tokenizer_revision: String,
    pub tokenizer_path: PathBuf,
    pub tokenizer_sha256: String,
    #[serde(default)]
    pub ollama_text: Option<VerifiedOllamaTextConfig>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifiedOllamaTextConfig {
    pub template_path: PathBuf,
    pub template_sha256: String,
    pub system: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedTokenizerConfig {
    pub version: u32,
    pub tokenizers: Vec<VerifiedTokenizerConfig>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokenizerIdentity {
    pub model_id: String,
    pub model_revision: String,
    pub tokenizer_revision: String,
    pub tokenizer_sha256: Option<String>,
    pub generation: u64,
    pub text_serializer_sha256: Option<String>,
    pub system_sha256: Option<String>,
}
#[derive(Clone)]
pub struct BoundTokenCounter {
    counter: Arc<TokenCounter>,
    pub identity: TokenizerIdentity,
    ollama_text: Option<Arc<VerifiedOllamaTextConfig>>,
}
impl BoundTokenCounter {
    pub fn count(&self, bytes: &[u8]) -> Result<usize, Error> {
        self.ensure_current()?;
        let tokens = if bytes.is_empty() {
            0
        } else {
            self.counter.count(bytes)?
        };
        self.ensure_current()?;
        Ok(tokens)
    }
    pub(crate) fn native_counter(&self) -> &TokenCounter {
        &self.counter
    }
    pub fn ensure_current(&self) -> Result<(), Error> {
        if identity(&self.identity.model_id)? != self.identity {
            return Err(error(ErrorCode::SequenceViolation));
        }
        Ok(())
    }
}

fn verified(config: &VerifiedTokenizerConfig, generation: u64) -> Result<BoundTokenCounter, Error> {
    if [
        &config.model_id,
        &config.model_revision,
        &config.tokenizer_revision,
    ]
    .iter()
    .any(|s| s.is_empty())
        || config.tokenizer_sha256.len() != 64
        || !config
            .tokenizer_sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(error(ErrorCode::InvalidArgument));
    }
    let bytes = read_owned_file(&config.tokenizer_path, MAX_ARTIFACT_BYTES)?;
    if sha(&bytes) != config.tokenizer_sha256 {
        return Err(error(ErrorCode::InvalidArgument));
    }
    let counter =
        TokenCounter::for_model(&config.model_id, Some(&bytes), FallbackWeights::default())?;
    if !matches!(counter, TokenCounter::HuggingFace { .. }) {
        return Err(error(ErrorCode::OperationUnavailable));
    }
    let ollama_text = if let Some(text) = &config.ollama_text {
        let template = read_owned_file(&text.template_path, 64 * 1024)?;
        if text.template_sha256
            != "eb4402837c7829a690fa845de4d7f3fd842c2adee476d5341da8a46ea9255175"
            || sha(&template) != text.template_sha256
            || text.system.len() > 1024 * 1024
        {
            return Err(error(ErrorCode::OperationUnavailable));
        }
        Some(Arc::new(text.clone()))
    } else {
        None
    };
    Ok(BoundTokenCounter {
        counter: Arc::new(counter),
        ollama_text,
        identity: TokenizerIdentity {
            model_id: config.model_id.clone(),
            model_revision: config.model_revision.clone(),
            tokenizer_revision: config.tokenizer_revision.clone(),
            tokenizer_sha256: Some(config.tokenizer_sha256.clone()),
            generation,
            text_serializer_sha256: config
                .ollama_text
                .as_ref()
                .map(|t| t.template_sha256.clone()),
            system_sha256: config
                .ollama_text
                .as_ref()
                .map(|t| sha(t.system.as_bytes())),
        },
    })
}

fn read_owned_file(path: &std::path::Path, limit: u64) -> Result<Vec<u8>, Error> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(|_| error(ErrorCode::OperationUnavailable))?;
    let metadata = file
        .metadata()
        .map_err(|_| error(ErrorCode::OperationUnavailable))?;
    if !metadata.is_file() {
        return Err(error(ErrorCode::InvalidArgument));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.mode() & 0o022 != 0
            || (metadata.uid() != 0 && metadata.uid() != rustix::process::geteuid().as_raw())
        {
            return Err(error(ErrorCode::InvalidArgument));
        }
    }
    if metadata.len() > limit {
        return Err(error(ErrorCode::CapacityExceeded));
    }
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| error(ErrorCode::OperationUnavailable))?;
    if bytes.len() as u64 > limit {
        return Err(error(ErrorCode::CapacityExceeded));
    }
    Ok(bytes)
}

fn registry() -> Result<&'static RwLock<BTreeMap<String, BoundTokenCounter>>, Error> {
    REGISTRY
        .get_or_init(|| {
            let mut entries = BTreeMap::new();
            if let Some(path) = std::env::var_os("HYPERMIND_CONTEXT_TOKENIZERS") {
                let bytes = read_owned_file(std::path::Path::new(&path), 1024 * 1024)
                    .map_err(|e| e.code)?;
                let config: OwnedTokenizerConfig =
                    serde_json::from_slice(&bytes).map_err(|_| ErrorCode::InvalidArgument)?;
                if config.version != 1 || config.tokenizers.len() > 128 {
                    return Err(ErrorCode::InvalidArgument);
                }
                for tokenizer in config.tokenizers {
                    let value = verified(&tokenizer, 1).map_err(|e| e.code)?;
                    if entries.insert(tokenizer.model_id, value).is_some() {
                        return Err(ErrorCode::InvalidArgument);
                    }
                }
            }
            Ok(RwLock::new(entries))
        })
        .as_ref()
        .map_err(|code| error(*code))
}

/// Owner/bootstrap API. This configuration is never accepted from tool requests.
pub fn install_owned(
    config: VerifiedTokenizerConfig,
    expected_generation: u64,
) -> Result<TokenizerIdentity, Error> {
    let next = expected_generation
        .checked_add(1)
        .ok_or_else(|| error(ErrorCode::CapacityExceeded))?;
    let candidate = verified(&config, next)?;
    let mut entries = registry()?
        .write()
        .map_err(|_| error(ErrorCode::OperationUnavailable))?;
    if entries
        .get(&config.model_id)
        .map_or(0, |value| value.identity.generation)
        != expected_generation
    {
        return Err(error(ErrorCode::SequenceViolation));
    }
    entries.insert(config.model_id, candidate.clone());
    Ok(candidate.identity)
}

pub fn counter_for_model(model_id: &str) -> Result<BoundTokenCounter, Error> {
    if let Some(counter) = registry()?
        .read()
        .map_err(|_| error(ErrorCode::OperationUnavailable))?
        .get(model_id)
    {
        return Ok(counter.clone());
    }
    let native = TokenCounter::for_model(model_id, None, FallbackWeights::default())?;
    if !matches!(native, TokenCounter::Tiktoken { .. }) {
        return Err(error(ErrorCode::OperationUnavailable));
    }
    Ok(BoundTokenCounter {
        counter: Arc::new(native),
        ollama_text: None,
        identity: TokenizerIdentity {
            model_id: model_id.into(),
            model_revision: "native-model-binding-v1".into(),
            tokenizer_revision: "native-tiktoken-v1".into(),
            tokenizer_sha256: None,
            generation: 0,
            text_serializer_sha256: None,
            system_sha256: None,
        },
    })
}
pub fn identity(model_id: &str) -> Result<TokenizerIdentity, Error> {
    Ok(counter_for_model(model_id)?.identity)
}
pub fn fence_digest(model_id: &str) -> Result<String, Error> {
    Ok(sha(
        &serde_json::to_vec(&identity(model_id)?).map_err(|_| error(ErrorCode::SchemaInvalid))?
    ))
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokenMeasurement {
    pub identity: TokenizerIdentity,
    pub tokens: u64,
    pub input_sha256: String,
}
/// Counts explicitly supplied final UTF-8 provider input; adds no chat framing.
pub fn count_provider_input(
    model_id: &str,
    expected_generation: u64,
    bytes: &[u8],
) -> Result<TokenMeasurement, Error> {
    let counter = counter_for_model(model_id)?;
    if counter.identity.generation != expected_generation {
        return Err(error(ErrorCode::SequenceViolation));
    }
    let tokens = counter.count(bytes)? as u64;
    counter.ensure_current()?;
    Ok(TokenMeasurement {
        identity: counter.identity,
        tokens,
        input_sha256: sha(bytes),
    })
}

/// Only the verified template's one-user plain-text chat branch is supported.
/// Owner-supplied system text must match the serving model's configuration.
pub fn count_ollama_user_input(
    model_id: &str,
    expected_generation: u64,
    content: &str,
) -> Result<TokenMeasurement, Error> {
    let counter = counter_for_model(model_id)?;
    if counter.identity.generation != expected_generation {
        return Err(error(ErrorCode::SequenceViolation));
    }
    let text = counter
        .ollama_text
        .as_ref()
        .ok_or_else(|| error(ErrorCode::OperationUnavailable))?;
    let mut prompt = String::new();
    if !text.system.is_empty() {
        prompt.push_str("<|im_start|>system\n");
        prompt.push_str(&text.system);
        prompt.push_str("<|im_end|>\n");
    }
    prompt.push_str("<|im_start|>user\n");
    prompt.push_str(content);
    prompt.push_str("<|im_end|>\n<|im_start|>assistant\n");
    let tokens = counter.count(prompt.as_bytes())? as u64;
    counter.ensure_current()?;
    Ok(TokenMeasurement {
        identity: counter.identity,
        tokens,
        input_sha256: sha(prompt.as_bytes()),
    })
}
