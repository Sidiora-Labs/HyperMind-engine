#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc)]

pub mod admission;
pub mod anthropic;
pub mod cost;
pub mod gemini;
pub mod ollama;
pub mod openai_compat;
pub mod registry;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::Mutex;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum ModelTier {
    Economy,
    Standard,
    Capable,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Pricing {
    pub input_microusd_per_million_tokens: u64,
    pub output_microusd_per_million_tokens: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub cost_microusd: u64,
}

impl Usage {
    pub fn with_cost(mut self, pricing: Pricing) -> Result<Self, LlmError> {
        let input = self
            .input_tokens
            .checked_mul(pricing.input_microusd_per_million_tokens)
            .ok_or(LlmError::Capacity)?;
        let output = self
            .output_tokens
            .checked_mul(pricing.output_microusd_per_million_tokens)
            .ok_or(LlmError::Capacity)?;
        self.cost_microusd = input
            .checked_add(output)
            .ok_or(LlmError::Capacity)?
            .div_ceil(1_000_000);
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructuredRequest {
    pub prompt_id: String,
    pub system: String,
    pub prompt: String,
    pub json_schema: Value,
    pub maximum_output_tokens: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StructuredResponse {
    pub model_id: String,
    pub tier: ModelTier,
    pub value: Value,
    pub usage: Usage,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LlmError {
    InvalidArgument(&'static str),
    Network(String),
    Wire(String),
    Schema(String),
    Capacity,
    Admission(&'static str),
}

pub trait LlmProvider: Send + Sync {
    fn model_id(&self) -> &str;
    fn tier(&self) -> ModelTier;
    fn generate_structured(
        &self,
        request: &StructuredRequest,
    ) -> Result<StructuredResponse, LlmError>;
}

impl<P: LlmProvider + ?Sized> LlmProvider for std::sync::Arc<P> {
    fn model_id(&self) -> &str {
        (**self).model_id()
    }

    fn tier(&self) -> ModelTier {
        (**self).tier()
    }

    fn generate_structured(
        &self,
        request: &StructuredRequest,
    ) -> Result<StructuredResponse, LlmError> {
        (**self).generate_structured(request)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderConfig {
    pub endpoint: String,
    pub api_key: Option<String>,
    pub model: String,
    pub tier: ModelTier,
    pub pricing: Pricing,
}

impl ProviderConfig {
    pub fn validate(&self) -> Result<(), LlmError> {
        if self.endpoint.is_empty() || self.model.is_empty() {
            Err(LlmError::InvalidArgument(
                "provider configuration is incomplete",
            ))
        } else {
            Ok(())
        }
    }
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
    fn send(&self, request: &WireRequest) -> Result<WireResponse, LlmError>;
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

    pub fn from_json(encoded: &str) -> Result<Self, LlmError> {
        serde_json::from_str(encoded)
            .map(Self::new)
            .map_err(|error| LlmError::Wire(error.to_string()))
    }

    #[must_use]
    pub fn remaining(&self) -> usize {
        self.fixtures.lock().map_or(0, |fixtures| fixtures.len())
    }
}

impl WireTransport for RecordedTransport {
    fn send(&self, request: &WireRequest) -> Result<WireResponse, LlmError> {
        let mut fixtures = self
            .fixtures
            .lock()
            .map_err(|_| LlmError::Wire("wire fixture lock poisoned".to_owned()))?;
        if fixtures.is_empty() {
            return Err(LlmError::Wire("no recorded response remains".to_owned()));
        }
        let fixture = fixtures.remove(0);
        if fixture.request != *request {
            return Err(LlmError::Wire(
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
    fn send(&self, request: &WireRequest) -> Result<WireResponse, LlmError> {
        let method = reqwest::Method::from_bytes(request.method.as_bytes())
            .map_err(|error| LlmError::Network(error.to_string()))?;
        let mut call = self
            .client
            .request(method, &request.url)
            .json(&request.body);
        for (name, value) in &request.headers {
            call = call.header(name, value);
        }
        let response = call
            .send()
            .map_err(|error| LlmError::Network(error.to_string()))?;
        let status = response.status().as_u16();
        let body = response
            .text()
            .map_err(|error| LlmError::Network(error.to_string()))?;
        let body = decode_wire_response(&body)?;
        Ok(WireResponse { status, body })
    }
}

fn decode_wire_response(body: &str) -> Result<Value, LlmError> {
    let body = body.trim_end();
    let body = body.strip_suffix("data: [DONE]").unwrap_or(body);
    serde_json::from_str(body).map_err(|error| LlmError::Wire(error.to_string()))
}

pub(crate) fn headers(config: &ProviderConfig) -> BTreeMap<String, String> {
    let mut headers = BTreeMap::from([("content-type".to_owned(), "application/json".to_owned())]);
    if let Some(key) = &config.api_key {
        headers.insert("authorization".to_owned(), format!("Bearer {key}"));
    }
    headers
}

pub(crate) fn validate_request(request: &StructuredRequest) -> Result<(), LlmError> {
    if request.prompt_id.is_empty()
        || request.prompt.is_empty()
        || request.maximum_output_tokens == 0
        || !request.json_schema.is_object()
    {
        Err(LlmError::InvalidArgument(
            "structured request is incomplete",
        ))
    } else {
        Ok(())
    }
}

pub(crate) fn require_success(response: WireResponse) -> Result<Value, LlmError> {
    if (200..300).contains(&response.status) {
        Ok(response.body)
    } else {
        Err(LlmError::Network(format!(
            "provider returned status {}",
            response.status
        )))
    }
}

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod wire_tests {
    use super::decode_wire_response;

    #[test]
    fn centra_terminal_done_marker_is_not_json_content() {
        let recorded = include_str!("../tests/fixtures/centra-json-done.txt");
        let decoded = decode_wire_response(recorded).unwrap();
        assert_eq!(decoded["usage"]["prompt_tokens"], 283);
        assert_eq!(decoded["usage"]["completion_tokens"], 92);
        assert!(decode_wire_response("{\"ok\":true}").is_ok());
        assert!(decode_wire_response("{\"ok\":true}arbitrary trailing text").is_err());
        assert!(decode_wire_response("{\"ok\":true}data: [DONE] extra").is_err());
        assert!(decode_wire_response("data: {\"ok\":true}\n\ndata: [DONE]").is_err());
    }
}
