#![forbid(unsafe_code)]

use hm_llm::anthropic::Anthropic;
use hm_llm::ollama::Ollama;
use hm_llm::openai_compat::OpenAiCompatible;
use hm_llm::{
    LlmProvider, ModelTier, Pricing, ProviderConfig, RecordedTransport, StructuredRequest,
};
use serde_json::json;

fn request() -> StructuredRequest {
    StructuredRequest {
        prompt_id: "supersession_1".to_owned(),
        system: "Decide temporal supersession.".to_owned(),
        prompt: "old: Europe\nnew: America".to_owned(),
        json_schema: json!({
            "type": "object",
            "properties": {
                "supersedes": {"type": "boolean"},
                "reason": {"type": "string"}
            },
            "required": ["supersedes", "reason"],
            "additionalProperties": false
        }),
        maximum_output_tokens: 64,
    }
}

fn config(endpoint: &str, api_key: Option<&str>) -> ProviderConfig {
    ProviderConfig {
        endpoint: endpoint.to_owned(),
        api_key: api_key.map(str::to_owned),
        model: "fixture-model".to_owned(),
        tier: ModelTier::Capable,
        pricing: Pricing {
            input_microusd_per_million_tokens: 1_000_000,
            output_microusd_per_million_tokens: 2_000_000,
        },
    }
}

#[test]
fn anthropic_native_messages_use_prompt_caching_and_report_usage() {
    let transport = RecordedTransport::from_json(include_str!("fixtures/anthropic.json")).unwrap();
    let provider = Anthropic::new(
        config("https://fixture.invalid/v1/messages", Some("fixture-key")),
        transport,
    )
    .unwrap();
    let response = provider.generate_structured(&request()).unwrap();
    assert_eq!(response.value["supersedes"], true);
    assert_eq!(response.usage.input_tokens, 10);
    assert_eq!(response.usage.output_tokens, 4);
    assert_eq!(response.usage.cache_read_tokens, 6);
    assert_eq!(response.usage.cache_write_tokens, 3);
    assert_eq!(response.usage.cost_microusd, 18);
    assert_eq!(provider.transport().remaining(), 0);
}

#[test]
fn openai_compatible_json_schema_matches_recorded_wire() {
    let transport = RecordedTransport::from_json(include_str!("fixtures/openai.json")).unwrap();
    let provider = OpenAiCompatible::new(
        config(
            "https://fixture.invalid/v1/chat/completions",
            Some("fixture-key"),
        ),
        transport,
    )
    .unwrap();
    let response = provider.generate_structured(&request()).unwrap();
    assert_eq!(response.value["reason"], "later interval");
    assert_eq!(response.usage.cost_microusd, 18);
    assert_eq!(response.usage.cache_read_tokens, 2);
    assert_eq!(provider.transport().remaining(), 0);
}

#[test]
fn ollama_native_schema_matches_recorded_wire() {
    let transport = RecordedTransport::from_json(include_str!("fixtures/ollama.json")).unwrap();
    let provider = Ollama::new(config("http://fixture.invalid/api/chat", None), transport).unwrap();
    let response = provider.generate_structured(&request()).unwrap();
    assert_eq!(response.value["supersedes"], true);
    assert_eq!(response.usage.cost_microusd, 18);
    assert_eq!(provider.transport().remaining(), 0);
}
