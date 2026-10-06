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
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub server_version: Option<String>,
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
    pub provider_endpoint_sha256: Option<String>,
    pub provider_version: Option<String>,
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
        if text.endpoint.is_some() != text.server_version.is_some() {
            return Err(error(ErrorCode::InvalidArgument));
        }
        if let Some(endpoint) = &text.endpoint {
            let url =
                reqwest::Url::parse(endpoint).map_err(|_| error(ErrorCode::InvalidArgument))?;
            if !matches!(url.scheme(), "http" | "https")
                || !url.username().is_empty()
                || url.password().is_some()
                || url.query().is_some()
                || url.fragment().is_some()
                || endpoint.ends_with('/')
                || text.server_version.as_ref().is_none_or(String::is_empty)
            {
                return Err(error(ErrorCode::InvalidArgument));
            }
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
            provider_endpoint_sha256: config
                .ollama_text
                .as_ref()
                .and_then(|t| t.endpoint.as_ref())
                .map(|s| sha(s.as_bytes())),
            provider_version: config
                .ollama_text
                .as_ref()
                .and_then(|t| t.server_version.clone()),
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
            provider_endpoint_sha256: None,
            provider_version: None,
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

struct UniqueJson;
impl<'de> Deserialize<'de> for UniqueJson {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = UniqueJson;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("JSON without duplicate keys")
            }
            fn visit_bool<E: serde::de::Error>(self, _: bool) -> Result<Self::Value, E> {
                Ok(UniqueJson)
            }
            fn visit_i64<E: serde::de::Error>(self, _: i64) -> Result<Self::Value, E> {
                Ok(UniqueJson)
            }
            fn visit_u64<E: serde::de::Error>(self, _: u64) -> Result<Self::Value, E> {
                Ok(UniqueJson)
            }
            fn visit_f64<E: serde::de::Error>(self, _: f64) -> Result<Self::Value, E> {
                Ok(UniqueJson)
            }
            fn visit_str<E: serde::de::Error>(self, _: &str) -> Result<Self::Value, E> {
                Ok(UniqueJson)
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueJson)
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut values: A,
            ) -> Result<Self::Value, A::Error> {
                while values.next_element::<UniqueJson>()?.is_some() {}
                Ok(UniqueJson)
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut values: A,
            ) -> Result<Self::Value, A::Error> {
                let mut keys = std::collections::BTreeSet::new();
                while let Some(key) = values.next_key::<String>()? {
                    if !keys.insert(key) {
                        return Err(serde::de::Error::custom("duplicate JSON key"));
                    }
                    values.next_value::<UniqueJson>()?;
                }
                Ok(UniqueJson)
            }
        }
        deserializer.deserialize_any(Visitor)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToolOccurrence {
    pub message_index: usize,
    pub call_index: usize,
    pub call_id: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChatMeasurement {
    pub identity: TokenizerIdentity,
    pub tokens: u64,
    pub input_sha256: String,
    pub payload_sha256: String,
    pub tool_occurrences: Vec<ToolOccurrence>,
    pub context_tokens: u64,
    pub reserved_output_tokens: u64,
}

fn validate_chat_payload(
    model_id: &str,
    payload: &serde_json::Value,
    budget: hm_context::TokenBudget,
) -> Result<Vec<ToolOccurrence>, Error> {
    let invalid = || error(ErrorCode::InvalidArgument);
    let unsupported = || error(ErrorCode::OperationUnavailable);
    let object = payload.as_object().ok_or_else(invalid)?;
    if object.keys().any(|key| {
        !matches!(
            key.as_str(),
            "model" | "messages" | "tools" | "stream" | "options" | "format" | "truncate"
        )
    }) || payload["model"] != model_id
        || payload["truncate"] != false
    {
        return Err(unsupported());
    }
    if payload["options"]["num_ctx"].as_u64() != Some(budget.context_tokens)
        || payload["options"]["num_predict"]
            .as_u64()
            .is_none_or(|n| n == 0 || n > budget.reserved_output_tokens)
    {
        return Err(error(ErrorCode::CapacityExceeded));
    }
    budget
        .available()
        .map_err(|_| error(ErrorCode::CapacityExceeded))?;
    let messages = payload["messages"].as_array().ok_or_else(invalid)?;
    if messages.is_empty() || messages.len() > 100_000 {
        return Err(invalid());
    }
    let mut pending = std::collections::BTreeSet::new();
    let mut occurrences = Vec::new();
    for (message_index, message) in messages.iter().enumerate() {
        let object = message.as_object().ok_or_else(invalid)?;
        if object.keys().any(|key| {
            !matches!(
                key.as_str(),
                "role"
                    | "content"
                    | "tool_calls"
                    | "tool_call_id"
                    | "tool_name"
                    | "_tool_status"
                    | "_volatile"
                    | "cache_control"
            )
        }) {
            return Err(unsupported());
        }
        let role = message["role"].as_str().ok_or_else(invalid)?;
        let content = message["content"].as_str().ok_or_else(unsupported)?;
        if !pending.is_empty() && role != "tool" {
            return Err(invalid());
        }
        match role {
            "system" | "user" | "assistant" => {
                if let Some(calls) = message.get("tool_calls") {
                    let calls = calls.as_array().ok_or_else(invalid)?;
                    if role != "assistant" || !content.is_empty() && !calls.is_empty() {
                        return Err(unsupported());
                    }
                    for (call_index, call) in calls.iter().enumerate() {
                        let id = call["id"]
                            .as_str()
                            .filter(|s| !s.is_empty())
                            .ok_or_else(invalid)?;
                        if !pending.insert(id.to_owned())
                            || call["function"]["name"].as_str().is_none_or(str::is_empty)
                            || !call["function"]["arguments"].is_object()
                        {
                            return Err(invalid());
                        }
                        occurrences.push(ToolOccurrence {
                            message_index,
                            call_index,
                            call_id: id.into(),
                        });
                    }
                }
            }
            "tool" => {
                let id = message["tool_call_id"].as_str().ok_or_else(invalid)?;
                if !pending.remove(id) || message.get("tool_calls").is_some() {
                    return Err(invalid());
                }
            }
            _ => return Err(unsupported()),
        }
    }
    if !pending.is_empty() {
        return Err(invalid());
    }
    Ok(occurrences)
}

async fn bounded_json(response: reqwest::Response) -> Result<serde_json::Value, Error> {
    let mut response = response
        .error_for_status()
        .map_err(|_| error(ErrorCode::OperationUnavailable))?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| error(ErrorCode::OperationUnavailable))?
    {
        if bytes.len().saturating_add(chunk.len()) > MAX_ARTIFACT_BYTES as usize {
            return Err(error(ErrorCode::CapacityExceeded));
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| error(ErrorCode::SchemaInvalid))
}

async fn verify_ollama_observation(
    client: &reqwest::Client,
    text: &VerifiedOllamaTextConfig,
    identity: &TokenizerIdentity,
) -> Result<(), Error> {
    let endpoint = text
        .endpoint
        .as_ref()
        .ok_or_else(|| error(ErrorCode::OperationUnavailable))?;
    let version = bounded_json(
        client
            .get(format!("{endpoint}/api/version"))
            .send()
            .await
            .map_err(|_| error(ErrorCode::OperationUnavailable))?,
    )
    .await?;
    let tags = bounded_json(
        client
            .get(format!("{endpoint}/api/tags"))
            .send()
            .await
            .map_err(|_| error(ErrorCode::OperationUnavailable))?,
    )
    .await?;
    let model = tags["models"]
        .as_array()
        .and_then(|models| models.iter().find(|m| m["name"] == identity.model_id))
        .ok_or_else(|| error(ErrorCode::SequenceViolation))?;
    let show = bounded_json(
        client
            .post(format!("{endpoint}/api/show"))
            .json(&serde_json::json!({"model":identity.model_id}))
            .send()
            .await
            .map_err(|_| error(ErrorCode::OperationUnavailable))?,
    )
    .await?;
    if version["version"].as_str() != text.server_version.as_deref()
        || model["digest"].as_str() != Some(identity.model_revision.as_str())
        || show["template"].as_str().map(|s| sha(s.as_bytes()))
            != Some(text.template_sha256.clone())
        || show["system"].as_str() != Some(text.system.as_str())
    {
        return Err(error(ErrorCode::SequenceViolation));
    }
    Ok(())
}

/// Uses the serving provider's render-only path. The supplied inference payload
/// is forwarded unchanged, with only its non-generating debug flag appended.
pub async fn count_ollama_chat_input(
    model_id: &str,
    expected_generation: u64,
    payload_bytes: &[u8],
    budget: hm_context::TokenBudget,
) -> Result<ChatMeasurement, Error> {
    if payload_bytes.len() > 16 * 1024 * 1024 {
        return Err(error(ErrorCode::CapacityExceeded));
    }
    serde_json::from_slice::<UniqueJson>(payload_bytes)
        .map_err(|_| error(ErrorCode::InvalidArgument))?;
    let payload: serde_json::Value =
        serde_json::from_slice(payload_bytes).map_err(|_| error(ErrorCode::InvalidArgument))?;
    let occurrences = validate_chat_payload(model_id, &payload, budget)?;
    let counter = counter_for_model(model_id)?;
    if counter.identity.generation != expected_generation {
        return Err(error(ErrorCode::SequenceViolation));
    }
    let text = counter
        .ollama_text
        .as_ref()
        .ok_or_else(|| error(ErrorCode::OperationUnavailable))?;
    let endpoint = text
        .endpoint
        .as_ref()
        .ok_or_else(|| error(ErrorCode::OperationUnavailable))?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(45))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| error(ErrorCode::OperationUnavailable))?;
    verify_ollama_observation(&client, text, &counter.identity).await?;
    let end = payload_bytes
        .iter()
        .rposition(|b| !b.is_ascii_whitespace())
        .ok_or_else(|| error(ErrorCode::InvalidArgument))?;
    if payload_bytes[end] != b'}' {
        return Err(error(ErrorCode::InvalidArgument));
    }
    let mut debug = payload_bytes[..end].to_vec();
    debug.extend_from_slice(b",\"_debug_render_only\":true}");
    let rendered = bounded_json(
        client
            .post(format!("{endpoint}/api/chat"))
            .header("Content-Type", "application/json")
            .body(debug)
            .send()
            .await
            .map_err(|_| error(ErrorCode::OperationUnavailable))?,
    )
    .await?;
    if rendered["model"] != model_id
        || rendered["done"] != false
        || rendered.get("eval_count").is_some()
        || rendered.get("prompt_eval_count").is_some()
        || rendered["_debug_info"]["image_count"]
            .as_u64()
            .is_some_and(|n| n != 0)
    {
        return Err(error(ErrorCode::OperationUnavailable));
    }
    let prompt = rendered["_debug_info"]["rendered_template"]
        .as_str()
        .ok_or_else(|| error(ErrorCode::OperationUnavailable))?;
    let tokens = counter.count(prompt.as_bytes())? as u64;
    if tokens
        > budget
            .available()
            .map_err(|_| error(ErrorCode::CapacityExceeded))?
    {
        return Err(error(ErrorCode::CapacityExceeded));
    }
    verify_ollama_observation(&client, text, &counter.identity).await?;
    counter.ensure_current()?;
    Ok(ChatMeasurement {
        identity: counter.identity,
        tokens,
        input_sha256: sha(prompt.as_bytes()),
        payload_sha256: sha(payload_bytes),
        tool_occurrences: occurrences,
        context_tokens: budget.context_tokens,
        reserved_output_tokens: budget.reserved_output_tokens,
    })
}
