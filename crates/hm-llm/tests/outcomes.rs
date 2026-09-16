#![forbid(unsafe_code)]

use hm_llm::anthropic::Anthropic;
use hm_llm::gemini::Gemini;
use hm_llm::ollama::Ollama;
use hm_llm::openai_compat::OpenAiCompatible;
use hm_llm::outcome::{RESPONSE_CONTRACT_VERSION, ResponseFault, ResponseOutcome};
use hm_llm::{
    LlmError, LlmProvider, ModelTier, Pricing, ProviderConfig, RecordedTransport,
    StructuredRequest, WireFixture,
};
use serde_json::{Map, Value, json};

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

fn config() -> ProviderConfig {
    ProviderConfig {
        endpoint: "https://fixture.invalid/v1/chat/completions".to_owned(),
        api_key: Some("fixture-key".to_owned()),
        model: "fixture-model".to_owned(),
        tier: ModelTier::Capable,
        pricing: Pricing {
            input_microusd_per_million_tokens: 1_000_000,
            output_microusd_per_million_tokens: 2_000_000,
        },
    }
}

fn config_at(endpoint: &str, api_key: Option<&str>) -> ProviderConfig {
    ProviderConfig {
        endpoint: endpoint.to_owned(),
        api_key: api_key.map(str::to_owned),
        ..config()
    }
}

fn recorded() -> Vec<WireFixture> {
    serde_json::from_str(include_str!("fixtures/openai.json")).unwrap()
}

fn choice(fixtures: &mut [WireFixture]) -> &mut Value {
    &mut fixtures[0].response.body["choices"][0]
}

fn message(fixtures: &mut [WireFixture]) -> &mut Map<String, Value> {
    choice(fixtures)["message"].as_object_mut().unwrap()
}

fn build(fixtures: Vec<WireFixture>) -> OpenAiCompatible<RecordedTransport> {
    OpenAiCompatible::new(config(), RecordedTransport::new(fixtures)).unwrap()
}

fn classified<P: LlmProvider>(provider: &P) -> ResponseFault {
    match provider.generate_structured(&request()) {
        Err(LlmError::Response(fault)) => fault,
        other => panic!("expected a classified response fault, got {other:?}"),
    }
}

fn assert_fault_identity(fault: &ResponseFault) {
    assert_eq!(fault.version, RESPONSE_CONTRACT_VERSION);
    assert_eq!(fault.version, 1);
    assert_eq!(fault.model_id, "fixture-model");
    assert_eq!(fault.requested_output_tokens, 64);
}

fn assert_identity(fault: &ResponseFault, provider: &OpenAiCompatible<RecordedTransport>) {
    assert_fault_identity(fault);
    assert_eq!(provider.transport().remaining(), 0);
}

#[test]
fn openai_compatible_refusal_truncation_malformed_and_incomplete_are_distinct() {
    let mut refusal = recorded();
    message(&mut refusal).insert("refusal".to_owned(), json!("cannot comply"));
    message(&mut refusal).remove("content");
    let provider = build(refusal);
    let fault = classified(&provider);
    assert_eq!(fault.outcome, ResponseOutcome::Refused);
    assert_eq!(fault.outcome.as_str(), "refused");
    assert!(!fault.detail.contains("later interval"));
    assert!(!fault.detail.contains("cannot comply"));
    assert_eq!(fault.usage.input_tokens, 10);
    assert_eq!(fault.usage.cost_microusd, 18);
    assert_identity(&fault, &provider);

    let mut truncation = recorded();
    choice(&mut truncation)["finish_reason"] = json!("length");
    let provider = build(truncation);
    let fault = classified(&provider);
    assert_eq!(fault.outcome, ResponseOutcome::Truncated);
    assert_eq!(fault.outcome.as_str(), "truncated");
    assert!(!fault.detail.contains("later interval"));
    assert_eq!(fault.usage.input_tokens, 10);
    assert_eq!(fault.usage.cost_microusd, 18);
    assert_identity(&fault, &provider);

    let mut malformed = recorded();
    message(&mut malformed).insert("content".to_owned(), json!("{\"supersedes\":"));
    let provider = build(malformed);
    let fault = classified(&provider);
    assert_eq!(fault.outcome, ResponseOutcome::Malformed);
    assert_eq!(fault.outcome.as_str(), "malformed");
    assert_eq!(
        fault.detail,
        serde_json::from_str::<Value>("{\"supersedes\":")
            .unwrap_err()
            .to_string()
    );
    assert_eq!(fault.usage.input_tokens, 10);
    assert_eq!(fault.usage.cost_microusd, 18);
    assert_identity(&fault, &provider);

    let mut incomplete = recorded();
    message(&mut incomplete).remove("content");
    let provider = build(incomplete);
    let fault = classified(&provider);
    assert_eq!(fault.outcome, ResponseOutcome::Incomplete);
    assert_eq!(fault.outcome.as_str(), "incomplete");
    assert!(!fault.detail.contains("later interval"));
    assert_identity(&fault, &provider);

    let mut blank = recorded();
    message(&mut blank).insert("content".to_owned(), json!("   \n"));
    let provider = build(blank);
    let fault = classified(&provider);
    assert_eq!(fault.outcome, ResponseOutcome::Incomplete);
    assert_identity(&fault, &provider);
}

#[test]
fn a_clean_recorded_response_is_never_classified_as_a_fault() {
    let mut stopped = recorded();
    choice(&mut stopped)["finish_reason"] = json!("stop");
    for fixtures in [recorded(), stopped] {
        let provider = build(fixtures);
        let response = provider.generate_structured(&request()).unwrap();
        assert_eq!(response.value["reason"], "later interval");
        assert_eq!(response.usage.cost_microusd, 18);
        assert_eq!(provider.transport().remaining(), 0);
    }
}

fn anthropic_recorded() -> Vec<WireFixture> {
    serde_json::from_str(include_str!("fixtures/anthropic.json")).unwrap()
}

fn anthropic(fixtures: Vec<WireFixture>) -> Anthropic<RecordedTransport> {
    Anthropic::new(
        config_at("https://fixture.invalid/v1/messages", Some("fixture-key")),
        RecordedTransport::new(fixtures),
    )
    .unwrap()
}

fn gemini_recorded() -> Vec<WireFixture> {
    serde_json::from_str(include_str!("fixtures/gemini.json")).unwrap()
}

fn gemini(fixtures: Vec<WireFixture>) -> Gemini<RecordedTransport> {
    Gemini::new(
        config_at(
            "https://fixture.invalid/v1beta/models/gemini-2.5-flash:generateContent",
            Some("fixture-key"),
        ),
        RecordedTransport::new(fixtures),
    )
    .unwrap()
}

fn ollama_recorded() -> Vec<WireFixture> {
    serde_json::from_str(include_str!("fixtures/ollama.json")).unwrap()
}

fn ollama(fixtures: Vec<WireFixture>) -> Ollama<RecordedTransport> {
    Ollama::new(
        config_at("http://fixture.invalid/api/chat", None),
        RecordedTransport::new(fixtures),
    )
    .unwrap()
}

#[test]
fn anthropic_stop_reason_separates_refusal_from_truncation() {
    let mut refusal = anthropic_recorded();
    refusal[0].response.body["stop_reason"] = json!("refusal");
    let provider = anthropic(refusal);
    let fault = classified(&provider);
    assert_eq!(fault.outcome, ResponseOutcome::Refused);
    assert_eq!(fault.usage.input_tokens, 10);
    assert_eq!(fault.usage.cost_microusd, 18);
    assert!(!fault.detail.contains("later interval"));
    assert_fault_identity(&fault);
    assert_eq!(provider.transport().remaining(), 0);

    let mut truncation = anthropic_recorded();
    truncation[0].response.body["stop_reason"] = json!("max_tokens");
    let provider = anthropic(truncation);
    let fault = classified(&provider);
    assert_eq!(fault.outcome, ResponseOutcome::Truncated);
    assert_eq!(fault.usage.input_tokens, 10);
    assert_eq!(fault.usage.cost_microusd, 18);
    assert!(!fault.detail.contains("later interval"));
    assert_fault_identity(&fault);
    assert_eq!(provider.transport().remaining(), 0);

    let mut incomplete = anthropic_recorded();
    incomplete[0].response.body["content"] = json!([]);
    let provider = anthropic(incomplete);
    let fault = classified(&provider);
    assert_eq!(fault.outcome, ResponseOutcome::Incomplete);
    assert_eq!(fault.usage.input_tokens, 10);
    assert_eq!(fault.usage.cache_write_tokens, 3);
    assert_fault_identity(&fault);
    assert_eq!(provider.transport().remaining(), 0);
}

#[test]
fn gemini_finish_reason_and_block_reason_are_classified() {
    let mut truncation = gemini_recorded();
    truncation[0].response.body["candidates"][0]["finishReason"] = json!("MAX_TOKENS");
    let provider = gemini(truncation);
    let fault = classified(&provider);
    assert_eq!(fault.outcome, ResponseOutcome::Truncated);
    assert_eq!(fault.usage.input_tokens, 10);
    assert_eq!(fault.usage.cost_microusd, 18);
    assert_fault_identity(&fault);
    assert_eq!(provider.transport().remaining(), 0);

    let mut blocked_candidate = gemini_recorded();
    blocked_candidate[0].response.body["candidates"][0]["finishReason"] = json!("SAFETY");
    let provider = gemini(blocked_candidate);
    let fault = classified(&provider);
    assert_eq!(fault.outcome, ResponseOutcome::Refused);
    assert!(!fault.detail.contains("later interval"));
    assert_fault_identity(&fault);
    assert_eq!(provider.transport().remaining(), 0);

    let mut blocked_prompt = gemini_recorded();
    blocked_prompt[0].response.body["promptFeedback"]["blockReason"] = json!("OTHER");
    let provider = gemini(blocked_prompt);
    let fault = classified(&provider);
    assert_eq!(fault.outcome, ResponseOutcome::Refused);
    assert!(!fault.detail.contains("OTHER"));
    assert_fault_identity(&fault);
    assert_eq!(provider.transport().remaining(), 0);

    let mut malformed = gemini_recorded();
    malformed[0].response.body["candidates"][0]["content"]["parts"][0]["text"] = json!("{");
    let provider = gemini(malformed);
    let fault = classified(&provider);
    assert_eq!(fault.outcome, ResponseOutcome::Malformed);
    assert_eq!(
        fault.detail,
        serde_json::from_str::<Value>("{").unwrap_err().to_string()
    );
    assert_eq!(fault.usage.cache_read_tokens, 2);
    assert_fault_identity(&fault);
    assert_eq!(provider.transport().remaining(), 0);

    let mut incomplete = gemini_recorded();
    incomplete[0].response.body["candidates"][0]["content"]["parts"] = json!([]);
    let provider = gemini(incomplete);
    let fault = classified(&provider);
    assert_eq!(fault.outcome, ResponseOutcome::Incomplete);
    assert_eq!(fault.usage.input_tokens, 10);
    assert_fault_identity(&fault);
    assert_eq!(provider.transport().remaining(), 0);
}

#[test]
fn ollama_done_reason_and_body_shape_are_classified() {
    let mut truncation = ollama_recorded();
    truncation[0].response.body["done_reason"] = json!("length");
    let provider = ollama(truncation);
    let fault = classified(&provider);
    assert_eq!(fault.outcome, ResponseOutcome::Truncated);
    assert_eq!(fault.usage.input_tokens, 10);
    assert_eq!(fault.usage.output_tokens, 4);
    assert_eq!(fault.usage.cost_microusd, 18);
    assert_fault_identity(&fault);
    assert_eq!(provider.transport().remaining(), 0);

    let mut malformed = ollama_recorded();
    malformed[0].response.body["message"]["content"] = json!("not json");
    let provider = ollama(malformed);
    let fault = classified(&provider);
    assert_eq!(fault.outcome, ResponseOutcome::Malformed);
    assert_eq!(
        fault.detail,
        serde_json::from_str::<Value>("not json")
            .unwrap_err()
            .to_string()
    );
    assert!(!fault.detail.contains("later interval"));
    assert_fault_identity(&fault);
    assert_eq!(provider.transport().remaining(), 0);

    let mut incomplete = ollama_recorded();
    incomplete[0].response.body["message"]
        .as_object_mut()
        .unwrap()
        .remove("content");
    let provider = ollama(incomplete);
    let fault = classified(&provider);
    assert_eq!(fault.outcome, ResponseOutcome::Incomplete);
    assert_eq!(fault.usage.input_tokens, 10);
    assert_fault_identity(&fault);
    assert_eq!(provider.transport().remaining(), 0);
}
