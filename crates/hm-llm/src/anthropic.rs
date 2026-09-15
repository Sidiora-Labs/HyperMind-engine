use crate::{
    LlmError, LlmProvider, ModelTier, ProviderConfig, StructuredRequest, StructuredResponse, Usage,
    WireRequest, WireTransport, require_success, validate_request,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub struct Anthropic<T> {
    config: ProviderConfig,
    transport: T,
}

impl<T> Anthropic<T> {
    pub fn new(config: ProviderConfig, transport: T) -> Result<Self, LlmError> {
        config.validate()?;
        Ok(Self { config, transport })
    }

    #[must_use]
    pub const fn transport(&self) -> &T {
        &self.transport
    }
}

impl<T: WireTransport> LlmProvider for Anthropic<T> {
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
        let mut headers = BTreeMap::from([
            ("content-type".to_owned(), "application/json".to_owned()),
            ("anthropic-version".to_owned(), "2023-06-01".to_owned()),
        ]);
        if let Some(key) = &self.config.api_key {
            headers.insert("x-api-key".to_owned(), key.clone());
        }
        let body = json!({
            "model": self.config.model,
            "max_tokens": request.maximum_output_tokens,
            "system": [{
                "type": "text",
                "text": request.system,
                "cache_control": {"type": "ephemeral"}
            }],
            "messages": [{"role": "user", "content": request.prompt}],
            "tools": [{
                "name": "emit_structured_result",
                "description": request.prompt_id,
                "input_schema": request.json_schema
            }],
            "tool_choice": {"type": "tool", "name": "emit_structured_result"}
        });
        let response = require_success(self.transport.send(&WireRequest {
            method: "POST".to_owned(),
            url: self.config.endpoint.clone(),
            headers,
            body,
        })?)?;
        let value = response
            .get("content")
            .and_then(Value::as_array)
            .and_then(|items| {
                items
                    .iter()
                    .find(|item| item.get("type").and_then(Value::as_str) == Some("tool_use"))
            })
            .and_then(|item| item.get("input"))
            .cloned()
            .ok_or_else(|| LlmError::Wire("Anthropic response has no tool input".to_owned()))?;
        let usage = response
            .get("usage")
            .ok_or_else(|| LlmError::Wire("Anthropic response has no usage".to_owned()))?;
        let usage = Usage {
            input_tokens: u64_field(usage, "input_tokens")?,
            output_tokens: u64_field(usage, "output_tokens")?,
            cache_read_tokens: optional_u64(usage, "cache_read_input_tokens"),
            cache_write_tokens: optional_u64(usage, "cache_creation_input_tokens"),
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

fn u64_field(value: &Value, field: &str) -> Result<u64, LlmError> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| LlmError::Wire(format!("Anthropic usage is missing {field}")))
}

fn optional_u64(value: &Value, field: &str) -> u64 {
    value.get(field).and_then(Value::as_u64).unwrap_or(0)
}
