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
        let body = request_body(&self.config.model, request);
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

fn request_body(model: &str, request: &StructuredRequest) -> Value {
    let mut body = json!({
        "model": model,
        "messages": [
            {"role": "system", "content": request.system},
            {"role": "user", "content": request.prompt}
        ],
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "name": request.prompt_id,
                "strict": true,
                "schema": request.json_schema
            }
        }
    });
    if model.rsplit('/').next() == Some("gpt-5.6-luna") {
        body["max_completion_tokens"] = json!(request.maximum_output_tokens);
        body["reasoning_effort"] = json!("none");
    } else {
        body["max_tokens"] = json!(request.maximum_output_tokens);
    }
    body
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luna_preserves_structured_contract_and_visible_output_budget() {
        let request = StructuredRequest {
            prompt_id: "reconstruct_1".into(),
            system: "Use only supplied anchors.".into(),
            prompt: "anchors".into(),
            json_schema: json!({"type":"object","additionalProperties":false}),
            maximum_output_tokens: 256,
        };
        for model in [
            "gpt-5.6-luna",
            "openai/gpt-5.6-luna",
            "openrouter/openai/gpt-5.6-luna",
        ] {
            let body = request_body(model, &request);
            assert_eq!(body["model"], model);
            assert_eq!(body["max_completion_tokens"], 256);
            assert_eq!(body["reasoning_effort"], "none");
            assert!(body.get("max_tokens").is_none());
            assert_eq!(
                body["response_format"]["json_schema"]["schema"],
                request.json_schema
            );
        }
        let historical = request_body("recorded-provider", &request);
        assert_eq!(historical["max_tokens"], 256);
        assert!(historical.get("max_completion_tokens").is_none());
        assert!(historical.get("reasoning_effort").is_none());
    }
}
