use crate::hash_feature::l2_normalize;
use crate::types::{
    Distance, EmbedError, Embedder, Embedding, InputRole, Normalization, SpaceIdentity,
    checked_embeddings,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::Mutex;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Provider {
    OpenAi,
    Voyage,
    Vertex,
    Ollama,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoteConfig {
    pub endpoint: String,
    pub api_key: Option<String>,
    pub model: String,
    pub revision: String,
    pub dimensions: usize,
    pub maximum_batch: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WireRequest {
    pub method: String,
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub body: Value,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WireResponse {
    pub status: u16,
    pub body: Value,
}

pub trait WireTransport: Send + Sync {
    fn send(&self, request: &WireRequest) -> Result<WireResponse, EmbedError>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WireFixture {
    pub request: WireRequest,
    pub response: WireResponse,
}

pub struct RecordedTransport {
    fixtures: Mutex<Vec<WireFixture>>,
}

impl RecordedTransport {
    #[must_use]
    pub fn new(fixtures: Vec<WireFixture>) -> Self {
        Self {
            fixtures: Mutex::new(fixtures),
        }
    }

    pub fn from_json(encoded: &str) -> Result<Self, EmbedError> {
        serde_json::from_str::<Vec<WireFixture>>(encoded)
            .map(Self::new)
            .map_err(|error| EmbedError::Wire(error.to_string()))
    }

    #[must_use]
    pub fn remaining(&self) -> usize {
        self.fixtures.lock().map_or(0, |fixtures| fixtures.len())
    }
}

impl WireTransport for RecordedTransport {
    fn send(&self, request: &WireRequest) -> Result<WireResponse, EmbedError> {
        let mut fixtures = self
            .fixtures
            .lock()
            .map_err(|_| EmbedError::Runtime("wire fixture lock poisoned".to_owned()))?;
        if fixtures.is_empty() {
            return Err(EmbedError::Wire("no recorded response remains".to_owned()));
        }
        let fixture = fixtures.remove(0);
        if fixture.request != *request {
            return Err(EmbedError::Wire(
                "request did not match recorded wire fixture".to_owned(),
            ));
        }
        Ok(fixture.response)
    }
}

#[derive(Clone, Debug, Default)]
pub struct HttpTransport {
    client: reqwest::blocking::Client,
}

impl WireTransport for HttpTransport {
    fn send(&self, request: &WireRequest) -> Result<WireResponse, EmbedError> {
        let method = reqwest::Method::from_bytes(request.method.as_bytes())
            .map_err(|error| EmbedError::Network(error.to_string()))?;
        let mut call = self
            .client
            .request(method, &request.url)
            .json(&request.body);
        for (name, value) in &request.headers {
            call = call.header(name, value);
        }
        let response = call
            .send()
            .map_err(|error| EmbedError::Network(error.to_string()))?;
        let status = response.status().as_u16();
        let body = response
            .json()
            .map_err(|error| EmbedError::Network(error.to_string()))?;
        Ok(WireResponse { status, body })
    }
}

pub struct RemoteEmbedder<T> {
    provider: Provider,
    config: RemoteConfig,
    transport: T,
}

impl<T> RemoteEmbedder<T> {
    pub fn new(provider: Provider, config: RemoteConfig, transport: T) -> Result<Self, EmbedError> {
        if config.endpoint.is_empty()
            || config.model.is_empty()
            || config.revision.is_empty()
            || config.dimensions == 0
            || config.maximum_batch == 0
        {
            return Err(EmbedError::InvalidArgument(
                "remote embedder configuration is incomplete",
            ));
        }
        Ok(Self {
            provider,
            config,
            transport,
        })
    }

    #[must_use]
    pub const fn transport(&self) -> &T {
        &self.transport
    }
}

impl<T: WireTransport> RemoteEmbedder<T> {
    fn embed(&self, role: InputRole, inputs: &[&str]) -> Result<Vec<Embedding>, EmbedError> {
        if inputs.is_empty() {
            return Ok(Vec::new());
        }
        let mut output = Vec::with_capacity(inputs.len());
        for batch in inputs.chunks(self.config.maximum_batch) {
            let request = self.request(role, batch);
            let response = self.transport.send(&request)?;
            if !(200..300).contains(&response.status) {
                return Err(EmbedError::Network(format!(
                    "embedding provider returned status {}",
                    response.status
                )));
            }
            let mut vectors = parse_vectors(self.provider, &response.body)?;
            for vector in &mut vectors {
                l2_normalize(vector);
            }
            output.extend(checked_embeddings(&self.identity(role), vectors)?);
        }
        if output.len() != inputs.len() {
            return Err(EmbedError::Wire(
                "embedding provider returned the wrong batch size".to_owned(),
            ));
        }
        Ok(output)
    }

    fn request(&self, role: InputRole, inputs: &[&str]) -> WireRequest {
        let mut headers =
            BTreeMap::from([("content-type".to_owned(), "application/json".to_owned())]);
        if let Some(key) = &self.config.api_key {
            headers.insert("authorization".to_owned(), format!("Bearer {key}"));
        }
        let body = match self.provider {
            Provider::OpenAi => json!({
                "model": self.config.model,
                "input": inputs,
                "dimensions": self.config.dimensions,
                "encoding_format": "float"
            }),
            Provider::Voyage => json!({
                "model": self.config.model,
                "input": inputs,
                "input_type": role_name(role),
                "output_dimension": self.config.dimensions,
                "output_dtype": "float"
            }),
            Provider::Vertex => json!({
                "instances": inputs.iter().map(|text| json!({
                    "content": text,
                    "task_type": vertex_role(role)
                })).collect::<Vec<_>>(),
                "parameters": {"outputDimensionality": self.config.dimensions}
            }),
            Provider::Ollama => json!({"model": self.config.model, "input": inputs}),
        };
        WireRequest {
            method: "POST".to_owned(),
            url: self.config.endpoint.clone(),
            headers,
            body,
        }
    }
}

impl<T: WireTransport> Embedder for RemoteEmbedder<T> {
    fn identity(&self, role: InputRole) -> SpaceIdentity {
        SpaceIdentity {
            encoder_id: format!("{}:{}", provider_name(self.provider), self.config.model),
            revision: self.config.revision.clone(),
            dimensions: self.config.dimensions,
            distance: Distance::Cosine,
            normalization: Normalization::L2,
            input_role: role,
        }
    }

    fn embed_query(&self, query: &str) -> Result<Embedding, EmbedError> {
        self.embed(InputRole::Query, &[query])
            .map(|mut values| values.remove(0))
    }

    fn embed_documents(&self, documents: &[&str]) -> Result<Vec<Embedding>, EmbedError> {
        self.embed(InputRole::Document, documents)
    }
}

fn parse_vectors(provider: Provider, body: &Value) -> Result<Vec<Vec<f32>>, EmbedError> {
    let values = match provider {
        Provider::OpenAi | Provider::Voyage => {
            body.get("data").and_then(Value::as_array).map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.get("embedding"))
                    .collect::<Vec<_>>()
            })
        }
        Provider::Vertex => body
            .get("predictions")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.pointer("/embeddings/values"))
                    .collect()
            }),
        Provider::Ollama => body
            .get("embeddings")
            .and_then(Value::as_array)
            .map(|items| items.iter().collect()),
    }
    .ok_or_else(|| EmbedError::Wire("embedding response has no vectors".to_owned()))?;
    values.into_iter().map(vector).collect()
}

#[allow(clippy::cast_possible_truncation)]
fn vector(value: &Value) -> Result<Vec<f32>, EmbedError> {
    value
        .as_array()
        .ok_or_else(|| EmbedError::Wire("embedding vector is not an array".to_owned()))?
        .iter()
        .map(|component| {
            component
                .as_f64()
                .map(|value| value as f32)
                .filter(|value| value.is_finite())
                .ok_or_else(|| EmbedError::Wire("embedding component is invalid".to_owned()))
        })
        .collect()
}

const fn provider_name(provider: Provider) -> &'static str {
    match provider {
        Provider::OpenAi => "openai",
        Provider::Voyage => "voyage",
        Provider::Vertex => "vertex",
        Provider::Ollama => "ollama",
    }
}

const fn role_name(role: InputRole) -> &'static str {
    match role {
        InputRole::Query => "query",
        InputRole::Document => "document",
    }
}

const fn vertex_role(role: InputRole) -> &'static str {
    match role {
        InputRole::Query => "RETRIEVAL_QUERY",
        InputRole::Document => "RETRIEVAL_DOCUMENT",
    }
}
