use crate::{
    LlmError, LlmProvider, ModelTier, ProviderConfig, StructuredRequest, StructuredResponse, Usage,
    WireRequest, WireTransport, headers, require_success, validate_request,
};
use serde_json::{Value, json};

pub struct OpenAiCompatible<T> {
    config: ProviderConfig,
    transport: T,
}

impl<T> OpenAiCompatible<T> {
    pub fn new(config: ProviderConfig, transport: T) -> Result<Self, LlmError> {
        config.validate()?;
        Ok(Self { config, transport })
    }

    #[must_use]
    pub const fn transport(&self) -> &T {
        &self.transport
    }
}

impl<T: WireTransport> LlmProvider for OpenAiCompatible<T> {
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
            "model": self.config.model,
            "messages": [
                {"role": "system", "content": request.system},
                {"role": "user", "content": request.prompt}
            ],
            "max_tokens": request.maximum_output_tokens,
            "response_format": {
                "type": "json_schema",
                "json_schema": {
                    "name": request.prompt_id,
                    "strict": true,
                    "schema": request.json_schema
                }
            }
        });
        let response = require_success(self.transport.send(&WireRequest {
            method: "POST".to_owned(),
            url: self.config.endpoint.clone(),
            headers: headers(&self.config),
            body,
        })?)?;
        let content = response
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str)
            .ok_or_else(|| LlmError::Wire("OpenAI response has no content".to_owned()))?;
        let value =
            serde_json::from_str(content).map_err(|error| LlmError::Schema(error.to_string()))?;
        let usage = response
            .get("usage")
            .ok_or_else(|| LlmError::Wire("OpenAI response has no usage".to_owned()))?;
        let usage = Usage {
            input_tokens: usage
                .get("prompt_tokens")
                .and_then(Value::as_u64)
                .ok_or_else(|| LlmError::Wire("OpenAI prompt usage is missing".to_owned()))?,
            output_tokens: usage
                .get("completion_tokens")
                .and_then(Value::as_u64)
                .ok_or_else(|| LlmError::Wire("OpenAI completion usage is missing".to_owned()))?,
            cache_read_tokens: usage
                .pointer("/prompt_tokens_details/cached_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0),
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
