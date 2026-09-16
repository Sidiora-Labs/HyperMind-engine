use crate::outcome::{ResponseFault, ResponseOutcome, decode_structured_text};
use crate::{
    LlmError, LlmProvider, ModelTier, Pricing, ProviderConfig, StructuredRequest,
    StructuredResponse, Usage, WireRequest, WireTransport, headers, require_success,
    validate_request,
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
        if truncated(&response) {
            return Err(ResponseFault::new(
                ResponseOutcome::Truncated,
                &self.config.model,
                "provider stopped the generation at the output token ceiling".to_owned(),
                request.maximum_output_tokens,
                observed_usage(&response, self.config.pricing),
            )
            .into());
        }
        let value = decode_structured_text(
            response.pointer("/message/content").and_then(Value::as_str),
            &self.config.model,
            request.maximum_output_tokens,
            observed_usage(&response, self.config.pricing),
        )?;
        let usage = reported_usage(&response).with_cost(self.config.pricing)?;
        Ok(StructuredResponse {
            model_id: self.config.model.clone(),
            tier: self.config.tier,
            value,
            usage,
        })
    }
}

fn truncated(response: &Value) -> bool {
    response.get("done_reason").and_then(Value::as_str) == Some("length")
}

fn reported_usage(response: &Value) -> Usage {
    Usage {
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
}

fn observed_usage(response: &Value, pricing: Pricing) -> Usage {
    let observed = reported_usage(response);
    observed.with_cost(pricing).unwrap_or(observed)
}
