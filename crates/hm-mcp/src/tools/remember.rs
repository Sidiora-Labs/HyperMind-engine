use super::source::{SourceDeliveryInput, SourceSettlementInput, SourceSyncInput};
use hm_core::{Error, ErrorCode};
use hm_embed::{
    CachedEmbedder, Embedder, HttpTransport, InputRole, Provider, QuantizedEmbedding, RemoteConfig,
    RemoteEmbedder, SpaceIdentity, quantize,
};
use hm_schema::events::{Retention, Sensitivity};
use rmcp::schemars;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Clone)]
pub struct EmbeddingRuntime {
    embedder: Arc<dyn Embedder>,
    mode: hm_serve::context_retrieval::EmbeddingMode,
}

impl EmbeddingRuntime {
    #[must_use]
    pub fn new(embedder: Arc<dyn Embedder>) -> Self {
        Self {
            embedder,
            mode: hm_serve::context_retrieval::EmbeddingMode::Managed,
        }
    }

    pub fn context_mode(&self) -> hm_serve::context_retrieval::EmbeddingMode {
        self.mode
    }

    pub fn embedder(&self) -> Arc<dyn Embedder> {
        self.embedder.clone()
    }

    pub(crate) fn health(&self) -> &'static str {
        if self.embedder.report_label() == "lexical_only" {
            "lexical_only"
        } else {
            "semantic_ready"
        }
    }

    pub fn configuration_metadata(runtime: Option<&Self>) -> serde_json::Value {
        match runtime {
            None => serde_json::json!({"mode":"off","readiness":"disabled"}),
            Some(runtime) => {
                let identity = runtime.embedder.identity(InputRole::Document);
                let readiness = match runtime.mode {
                    hm_serve::context_retrieval::EmbeddingMode::Off => "disabled",
                    hm_serve::context_retrieval::EmbeddingMode::Local => "loaded",
                    hm_serve::context_retrieval::EmbeddingMode::RemoteCompatible => "configured",
                    hm_serve::context_retrieval::EmbeddingMode::Managed => "host_injected",
                };
                serde_json::json!({"mode":runtime.mode,"readiness":readiness,"model":identity.encoder_id,"revision":identity.revision,"dimensions":identity.dimensions})
            }
        }
    }

    pub fn from_env() -> Result<Option<Self>, &'static str> {
        let selected = match std::env::var("HM_EMBEDDING_PROVIDER") {
            Ok(value) => value,
            Err(std::env::VarError::NotPresent) => return Ok(None),
            Err(_) => return Err("HM_EMBEDDING_PROVIDER must be valid text"),
        };
        match selected.as_str() {
            "" | "off" => Ok(None),
            "local" => {
                let model = required_embedding_env("HM_EMBEDDING_MODEL")?;
                let kind = match model.as_str() {
                    "bge-small-en-v1.5" | "BAAI/bge-small-en-v1.5" => {
                        hm_embed::ModelKind::BgeSmallEnV15
                    }
                    "nomic-embed-text-v1.5" | "nomic-ai/nomic-embed-text-v1.5" => {
                        hm_embed::ModelKind::NomicEmbedTextV15
                    }
                    _ => {
                        return Err("HM_EMBEDDING_MODEL must name a registered pinned local model");
                    }
                };
                let root = std::path::PathBuf::from(required_embedding_env(
                    "HM_EMBEDDING_MODEL_DIRECTORY",
                )?);
                let spec = kind.spec();
                let directory = root
                    .join(spec.encoder_id.replace('/', "--"))
                    .join(spec.revision);
                let model_path = directory.join(spec.model.name);
                let tokenizer_path = directory.join(spec.tokenizer.name);
                verify_cached_embedding_artifact(&model_path, spec.model.sha256)?;
                verify_cached_embedding_artifact(&tokenizer_path, spec.tokenizer.sha256)?;
                let encoder = hm_embed::OnnxEmbedder::open(kind, &model_path, &tokenizer_path)
                    .map_err(|_| "verified local embedding model could not be loaded")?;
                let cached = CachedEmbedder::new(encoder, 16)
                    .map_err(|_| "embedding cache configuration is invalid")?;
                Ok(Some(Self {
                    embedder: Arc::new(cached),
                    mode: hm_serve::context_retrieval::EmbeddingMode::Local,
                }))
            }
            "remote-compatible" => {
                let endpoint = required_embedding_env("HM_EMBEDDING_ENDPOINT")?;
                let parsed = url::Url::parse(&endpoint)
                    .map_err(|_| "HM_EMBEDDING_ENDPOINT must be an HTTP endpoint")?;
                if !matches!(parsed.scheme(), "http" | "https")
                    || parsed.host_str().is_none()
                    || !parsed.username().is_empty()
                    || parsed.password().is_some()
                    || parsed.fragment().is_some()
                {
                    return Err(
                        "HM_EMBEDDING_ENDPOINT must be an HTTP endpoint without embedded credentials",
                    );
                }
                let model = required_embedding_env("HM_EMBEDDING_MODEL")?;
                let revision = required_embedding_env("HM_EMBEDDING_REVISION")?;
                hm_context::validate_id(&model).map_err(|_| "HM_EMBEDDING_MODEL is invalid")?;
                hm_context::validate_id(&revision)
                    .map_err(|_| "HM_EMBEDDING_REVISION is invalid")?;
                let dimensions = required_embedding_env("HM_EMBEDDING_DIMENSIONS")?
                    .parse::<usize>()
                    .map_err(|_| "HM_EMBEDDING_DIMENSIONS must be a positive integer")?;
                if dimensions == 0 || dimensions > 65_536 {
                    return Err("HM_EMBEDDING_DIMENSIONS is outside supported bounds");
                }
                let maximum_batch = match std::env::var("HM_EMBEDDING_MAXIMUM_BATCH") {
                    Ok(value) => value
                        .parse::<usize>()
                        .map_err(|_| "HM_EMBEDDING_MAXIMUM_BATCH must be an integer")?,
                    Err(std::env::VarError::NotPresent) => 16,
                    Err(_) => return Err("HM_EMBEDDING_MAXIMUM_BATCH is invalid"),
                };
                if maximum_batch == 0 || maximum_batch > 16 {
                    return Err("HM_EMBEDDING_MAXIMUM_BATCH must be between one and sixteen");
                }
                let api_key = match std::env::var("HM_EMBEDDING_API_KEY") {
                    Ok(value) => Some(value).filter(|v| !v.is_empty()),
                    Err(std::env::VarError::NotPresent) => None,
                    Err(_) => return Err("HM_EMBEDDING_API_KEY is invalid"),
                };
                Self::remote_configured(RemoteConfig {
                    endpoint,
                    api_key,
                    model,
                    revision,
                    dimensions,
                    maximum_batch,
                })
                .map(Some)
            }
            "centra" => {
                let api_key = required_embedding_env("CENTRA_GATEWAY_API_KEY")?;
                let gateway_url = std::env::var("CENTRA_GATEWAY_URL")
                    .unwrap_or_else(|_| "https://gateway.centra.ag/v1".to_owned());
                Self::remote_configured(RemoteConfig {
                    endpoint: format!("{}/embeddings", gateway_url.trim_end_matches('/')),
                    api_key: Some(api_key),
                    model: "openrouter/openai/text-embedding-3-large".to_owned(),
                    revision: "centra-openrouter-live".to_owned(),
                    dimensions: 3072,
                    maximum_batch: 16,
                })
                .map(Some)
            }
            _ => {
                Err("HM_EMBEDDING_PROVIDER must be off, local, remote-compatible, centra or unset")
            }
        }
    }

    fn remote_configured(config: RemoteConfig) -> Result<Self, &'static str> {
        let maximum_batch = config.maximum_batch;
        let encoder = RemoteEmbedder::new(Provider::OpenAi, config, HttpTransport::default())
            .map_err(|_| "embedding provider configuration is invalid")?;
        let cached = CachedEmbedder::new(encoder, maximum_batch)
            .map_err(|_| "embedding cache configuration is invalid")?;
        Ok(Self {
            embedder: Arc::new(cached),
            mode: hm_serve::context_retrieval::EmbeddingMode::RemoteCompatible,
        })
    }

    pub(crate) fn document_space(&self) -> SpaceIdentity {
        self.embedder.identity(InputRole::Document)
    }

    pub(crate) async fn documents(
        &self,
        documents: Vec<String>,
    ) -> Result<Vec<QuantizedEmbedding>, Error> {
        let embedder = self.embedder.clone();
        tokio::task::spawn_blocking(move || {
            let identity = embedder.identity(InputRole::Document);
            let inputs = documents.iter().map(String::as_str).collect::<Vec<_>>();
            let embeddings = embedder
                .embed_documents(&inputs)
                .map_err(|_| Error::new(ErrorCode::OperationUnavailable))?;
            if embeddings.len() != inputs.len() {
                return Err(Error::new(ErrorCode::SchemaInvalid));
            }
            embeddings
                .into_iter()
                .map(|embedding| {
                    if embedding.space != identity {
                        return Err(Error::new(ErrorCode::SchemaInvalid));
                    }
                    quantize(&embedding).map_err(|_| Error::new(ErrorCode::SchemaInvalid))
                })
                .collect()
        })
        .await
        .map_err(|_| Error::new(ErrorCode::OperationUnavailable))?
    }

    pub(crate) async fn query(&self, query: String) -> Result<QuantizedEmbedding, Error> {
        let embedder = self.embedder.clone();
        tokio::task::spawn_blocking(move || {
            let document_identity = embedder.identity(InputRole::Document);
            let mut query_identity = embedder.identity(InputRole::Query);
            let embedding = embedder
                .embed_query(&query)
                .map_err(|_| Error::new(ErrorCode::OperationUnavailable))?;
            if embedding.space != query_identity || query_identity.input_role != InputRole::Query {
                return Err(Error::new(ErrorCode::SchemaInvalid));
            }
            query_identity.input_role = InputRole::Document;
            if query_identity != document_identity {
                return Err(Error::new(ErrorCode::SchemaInvalid));
            }
            quantize(&embedding).map_err(|_| Error::new(ErrorCode::SchemaInvalid))
        })
        .await
        .map_err(|_| Error::new(ErrorCode::OperationUnavailable))?
    }
}

fn required_embedding_env(name: &'static str) -> Result<String, &'static str> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or(name)
}

fn verify_cached_embedding_artifact(
    path: &std::path::Path,
    expected_digest: &str,
) -> Result<(), &'static str> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(
        |_| "local embedding artifact is absent; provision the pinned model before startup",
    )?;
    if !file
        .metadata()
        .map_err(|_| "local embedding artifact cannot be inspected")?
        .is_file()
    {
        return Err("local embedding artifact is not a regular file");
    }
    let mut bytes = Vec::new();
    file.take(512 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "local embedding artifact cannot be read")?;
    if bytes.len() > 512 * 1024 * 1024 || hm_context::digest_bytes(&bytes) != expected_digest {
        return Err("local embedding artifact failed pinned digest verification");
    }
    Ok(())
}

pub(crate) fn space_id(identity: &SpaceIdentity) -> String {
    let canonical = serde_json::json!({
        "encoder": identity.encoder_id,
        "revision": identity.revision,
        "dimensions": identity.dimensions,
        "distance": format!("{:?}", identity.distance),
        "normalization": format!("{:?}", identity.normalization),
        "input_role": "document",
    });
    format!(
        "hm-space-v1:{}",
        blake3::hash(canonical.to_string().as_bytes()).to_hex()
    )
}

pub(crate) fn relation_space_id(identity: &SpaceIdentity) -> String {
    let canonical = serde_json::json!({
        "encoder": identity.encoder_id,
        "revision": identity.revision,
        "dimensions": identity.dimensions,
        "distance": format!("{:?}", identity.distance),
        "normalization": format!("{:?}", identity.normalization),
        "input_role": "document",
        "lane": "relation",
    });
    format!(
        "hm-space-v1:{}",
        blake3::hash(canonical.to_string().as_bytes()).to_hex()
    )
}

#[derive(Clone, Copy, Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RememberKind {
    User,
    Assistant,
    Document,
    Vocabulary,
    RepositorySnapshot,
}

#[derive(Clone, Copy, Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AnchorFacet {
    Path,
    Symbol,
    Url,
    Entity,
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct RememberAnchor {
    pub facet: AnchorFacet,
    pub value: String,
}

#[derive(Clone, Copy, Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RetentionInput {
    CurrentState,
    Daily,
    Durable,
    DoNotStore,
}

impl From<RetentionInput> for Retention {
    fn from(value: RetentionInput) -> Self {
        match value {
            RetentionInput::CurrentState => Self::CurrentState,
            RetentionInput::Daily => Self::Daily,
            RetentionInput::Durable => Self::Durable,
            RetentionInput::DoNotStore => Self::DoNotStore,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SensitivityInput {
    Public,
    Personal,
    Secret,
}

impl From<SensitivityInput> for Sensitivity {
    fn from(value: SensitivityInput) -> Self {
        match value {
            SensitivityInput::Public => Self::Public,
            SensitivityInput::Personal => Self::Personal,
            SensitivityInput::Secret => Self::Secret,
        }
    }
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct VocabularyInput {
    pub vocabulary_id: String,
    pub version: u16,
    pub source_uri: String,
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct RememberSource {
    pub url: String,
}

#[derive(Clone, Copy, Debug, Deserialize, schemars::JsonSchema)]
pub struct RememberDerive {
    pub media_lsn: u64,
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct RememberDocument {
    pub name: String,
    pub media_type: String,
    pub content_base64: String,
    #[serde(default)]
    pub loader: Option<String>,
    #[serde(default)]
    pub token_budget: Option<u32>,
    #[serde(default)]
    pub plan_only: bool,
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct RememberInput {
    pub conversation: String,
    #[serde(default)]
    pub content: String,
    pub kind: RememberKind,
    #[serde(default)]
    pub chunk_bytes: Option<usize>,
    #[serde(default)]
    pub anchor: Option<RememberAnchor>,
    #[serde(default)]
    pub retention: Option<RetentionInput>,
    #[serde(default)]
    pub sensitivity: Option<SensitivityInput>,
    #[serde(default)]
    pub vocabulary: Option<VocabularyInput>,
    #[serde(default)]
    pub source: Option<RememberSource>,
    #[serde(default)]
    pub derive: Option<RememberDerive>,
    #[serde(default)]
    pub source_delivery: Option<SourceDeliveryInput>,
    #[serde(default)]
    pub source_settlement: Option<SourceSettlementInput>,
    #[serde(default)]
    pub document: Option<RememberDocument>,
    #[serde(default)]
    pub source_sync: Option<SourceSyncInput>,
    #[serde(default)]
    pub context: Option<serde_json::Value>,
}
