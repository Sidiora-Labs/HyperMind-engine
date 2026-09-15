use crate::{
    LlmError, LlmProvider, ModelTier, ProviderConfig, StructuredRequest, StructuredResponse, Usage,
    WireRequest, WireTransport, headers, require_success, validate_request,
};
use serde_json::{Value, json};

pub struct Ollama<T> {
    config: ProviderConfig,
    transport: T,
}

impl<T> Ollama<T> {
    pub fn new(config: ProviderConfig, transport: T) -> Result<Self, LlmError> {
        config.validate()?;
        Ok(Self { config, transport })
    }

    #[must_use]
    pub const fn transport(&self) -> &T {
        &self.transport
    }
}

impl<T: WireTransport> LlmProvider for Ollama<T> {
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
            "format": request.json_schema,
            "stream": false,
            "options": {"num_predict": request.maximum_output_tokens}
        });
        let response = require_success(self.transport.send(&WireRequest {
            method: "POST".to_owned(),
            url: self.config.endpoint.clone(),
            headers: headers(&self.config),
            body,
        })?)?;
        let content = response
            .pointer("/message/content")
            .and_then(Value::as_str)
            .ok_or_else(|| LlmError::Wire("Ollama response has no content".to_owned()))?;
        let value =
            serde_json::from_str(content).map_err(|error| LlmError::Schema(error.to_string()))?;
        let usage = Usage {
            input_tokens: response
                .get("prompt_eval_count")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            output_tokens: response
                .get("eval_count")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            cache_read_tokens: 0,
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
