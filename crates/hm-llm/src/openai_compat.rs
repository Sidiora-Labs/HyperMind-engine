use crate::media::{
    MediaAttachment, MediaModality, MediaProvider, MediaRequest, audio_format_token,
    validate_media_request,
};
use crate::outcome::{ResponseFault, ResponseOutcome, decode_structured_text};
use crate::{
    LlmError, LlmProvider, ModelTier, Pricing, ProviderConfig, StructuredRequest,
    StructuredResponse, Usage, WireRequest, WireTransport, headers, require_success,
    validate_request,
};
use base64::Engine as _;
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

impl<T: WireTransport> OpenAiCompatible<T> {
    fn complete(
        &self,
        body: Value,
        maximum_output_tokens: u32,
    ) -> Result<StructuredResponse, LlmError> {
        let response = require_success(self.transport.send(&WireRequest {
            method: "POST".to_owned(),
            url: self.config.endpoint.clone(),
            headers: headers(&self.config),
            body,
        })?)?;
        parse_completion(&self.config, &response, maximum_output_tokens)
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
        self.complete(body, request.maximum_output_tokens)
    }
}

impl<T: WireTransport> MediaProvider for OpenAiCompatible<T> {
    fn model_id(&self) -> &str {
        &self.config.model
    }

    fn tier(&self) -> ModelTier {
        self.config.tier
    }

    fn describe_media(&self, request: &MediaRequest) -> Result<StructuredResponse, LlmError> {
        validate_media_request(request)?;
        let body = media_request_body(&self.config.model, request);
        self.complete(body, request.maximum_output_tokens)
    }
}

fn parse_completion(
    config: &ProviderConfig,
    response: &Value,
    maximum_output_tokens: u32,
) -> Result<StructuredResponse, LlmError> {
    if refused(response) {
        return Err(ResponseFault::new(
            ResponseOutcome::Refused,
            &config.model,
            "provider declined the structured request".to_owned(),
            maximum_output_tokens,
            observed_usage(response, config.pricing),
        )
        .into());
    }
    if truncated(response) {
        return Err(ResponseFault::new(
            ResponseOutcome::Truncated,
            &config.model,
            "provider stopped the generation at the output token ceiling".to_owned(),
            maximum_output_tokens,
            observed_usage(response, config.pricing),
        )
        .into());
    }
    let value = decode_structured_text(
        response
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str),
        &config.model,
        maximum_output_tokens,
        observed_usage(response, config.pricing),
    )?;
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
    .with_cost(config.pricing)?;
    Ok(StructuredResponse {
        model_id: config.model.clone(),
        tier: config.tier,
        value,
        usage,
    })
}

fn refused(response: &Value) -> bool {
    response
        .pointer("/choices/0/message/refusal")
        .and_then(Value::as_str)
        .is_some_and(|refusal| !refusal.trim().is_empty())
}

fn truncated(response: &Value) -> bool {
    matches!(
        response
            .pointer("/choices/0/finish_reason")
            .and_then(Value::as_str),
        Some("length" | "max_tokens")
    )
}

fn observed_usage(response: &Value, pricing: Pricing) -> Usage {
    let observed = Usage {
        input_tokens: response
            .pointer("/usage/prompt_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        output_tokens: response
            .pointer("/usage/completion_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        cache_read_tokens: response
            .pointer("/usage/prompt_tokens_details/cached_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        cache_write_tokens: 0,
        cost_microusd: 0,
    };
    observed.with_cost(pricing).unwrap_or(observed)
}

fn completion_body(
    model: &str,
    system: &str,
    user_content: &Value,
    schema_name: &str,
    json_schema: &Value,
    maximum_output_tokens: u32,
) -> Value {
    let mut body = json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user_content}
        ],
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "name": schema_name,
                "strict": true,
                "schema": json_schema
            }
        }
    });
    if model.rsplit('/').next() == Some("gpt-5.6-luna") {
        body["max_completion_tokens"] = json!(maximum_output_tokens);
        body["reasoning_effort"] = json!("none");
    } else {
        body["max_tokens"] = json!(maximum_output_tokens);
    }
    body
}

fn request_body(model: &str, request: &StructuredRequest) -> Value {
    completion_body(
        model,
        &request.system,
        &json!(request.prompt),
        &request.prompt_id,
        &request.json_schema,
        request.maximum_output_tokens,
    )
}

fn media_request_body(model: &str, request: &MediaRequest) -> Value {
    completion_body(
        model,
        &request.system,
        &media_content(&request.prompt, &request.attachment),
        &request.prompt_id,
        &request.json_schema,
        request.maximum_output_tokens,
    )
}

fn media_content(prompt: &str, attachment: &MediaAttachment) -> Value {
    let encoded = base64::engine::general_purpose::STANDARD.encode(&attachment.bytes);
    let attached = match attachment.modality {
        MediaModality::Audio => json!({
            "type": "input_audio",
            "input_audio": {
                "data": encoded,
                "format": audio_format_token(&attachment.media_type).unwrap_or_default()
            }
        }),
        MediaModality::Image => json!({
            "type": "image_url",
            "image_url": {
                "url": format!("data:{};base64,{encoded}", attachment.media_type)
            }
        }),
    };
    json!([{"type": "text", "text": prompt}, attached])
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
