use crate::{
    LlmError, LlmProvider, ModelTier, ProviderConfig, StructuredRequest, StructuredResponse, Usage,
    WireRequest, WireTransport, require_success, validate_request,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub struct Gemini<T> {
    config: ProviderConfig,
    transport: T,
}

impl<T> Gemini<T> {
    pub fn new(config: ProviderConfig, transport: T) -> Result<Self, LlmError> {
        config.validate()?;
        Ok(Self { config, transport })
    }

    #[must_use]
    pub const fn transport(&self) -> &T {
        &self.transport
    }
}

impl<T: WireTransport> LlmProvider for Gemini<T> {
    fn model_id(&self) -> &str {
        &self.config.model
    }

    fn tier(&self) -> ModelTier {
        self.config.tier
    }

    fn generate_structured(
        &self,
        request: &StructuredRequest,
    ) -> Result<StructuredResponse, LlmError> {
        validate_request(request)?;
        let body = json!({
            "systemInstruction": {"parts": [{"text": request.system}]},
            "contents": [{"role": "user", "parts": [{"text": request.prompt}]}],
            "generationConfig": {
                "responseMimeType": "application/json",
                "responseJsonSchema": request.json_schema,
                "maxOutputTokens": request.maximum_output_tokens
            }
        });
        let mut headers =
            BTreeMap::from([("content-type".to_owned(), "application/json".to_owned())]);
        if let Some(api_key) = &self.config.api_key {
            headers.insert("x-goog-api-key".to_owned(), api_key.clone());
        }
        let response = require_success(self.transport.send(&WireRequest {
            method: "POST".to_owned(),
            url: self.config.endpoint.clone(),
            headers,
            body,
        })?)?;
        let content = response
            .pointer("/candidates/0/content/parts/0/text")
            .and_then(Value::as_str)
            .ok_or_else(|| LlmError::Wire("Gemini response has no content".to_owned()))?;
        let value =
            serde_json::from_str(content).map_err(|error| LlmError::Schema(error.to_string()))?;
        let usage = response
            .get("usageMetadata")
            .ok_or_else(|| LlmError::Wire("Gemini response has no usage metadata".to_owned()))?;
        let usage = Usage {
            input_tokens: required_u64(usage, "promptTokenCount")?,
            output_tokens: required_u64(usage, "candidatesTokenCount")?,
            cache_read_tokens: optional_u64(usage, "cachedContentTokenCount"),
            cache_write_tokens: 0,
            cost_microusd: 0,
        }
        .with_cost(self.config.pricing)?;
        Ok(StructuredResponse {
            model_id: self.config.model.clone(),
            tier: self.config.tier,
            value,
            usage,
        })
    }
}

fn required_u64(value: &Value, field: &str) -> Result<u64, LlmError> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| LlmError::Wire(format!("Gemini usage is missing {field}")))
}

fn optional_u64(value: &Value, field: &str) -> u64 {
    value.get(field).and_then(Value::as_u64).unwrap_or(0)
}
