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
}

impl EmbeddingRuntime {
    #[must_use]
    pub fn new(embedder: Arc<dyn Embedder>) -> Self {
        Self { embedder }
    }

    pub(crate) fn health(&self) -> &'static str {
        if self.embedder.report_label() == "lexical_only" {
            "lexical_only"
        } else {
            "semantic_ready"
        }
    }

    pub fn from_env() -> Result<Option<Self>, &'static str> {
        match std::env::var("HM_EMBEDDING_PROVIDER").as_deref() {
            Err(_) | Ok("") => return Ok(None),
            Ok("centra") => {}
            _ => return Err("HM_EMBEDDING_PROVIDER must be centra or unset"),
        }
        let api_key = std::env::var("CENTRA_GATEWAY_API_KEY")
            .ok()
            .filter(|key| !key.is_empty())
            .ok_or("CENTRA_GATEWAY_API_KEY is required for the embedding provider")?;
        let gateway_url = std::env::var("CENTRA_GATEWAY_URL")
            .unwrap_or_else(|_| "https://gateway.centra.ag/v1".to_owned());
        let embedder = RemoteEmbedder::new(
            Provider::OpenAi,
            RemoteConfig {
                endpoint: format!("{}/embeddings", gateway_url.trim_end_matches('/')),
                api_key: Some(api_key),
                model: "openrouter/openai/text-embedding-3-large".to_owned(),
                revision: "centra-openrouter-live".to_owned(),
                dimensions: 3072,
                maximum_batch: 16,
            },
            HttpTransport::default(),
        )
        .map_err(|_| "embedding provider configuration is invalid")?;
        let cached = CachedEmbedder::new(embedder, 16)
            .map_err(|_| "embedding cache configuration is invalid")?;
        Ok(Some(Self::new(Arc::new(cached))))
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

#[derive(Clone, Copy, Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RememberKind {
    User,
    Assistant,
    Document,
    Vocabulary,
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
pub struct RememberInput {
    pub conversation: String,
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
}
