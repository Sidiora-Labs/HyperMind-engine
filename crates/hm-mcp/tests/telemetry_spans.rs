#![allow(clippy::too_many_lines)]

use hm_core::{ActorId, ConversationId};
use hm_ledger::frame::EventKind;
use hm_llm::openai_compat::OpenAiCompatible;
use hm_llm::{
    LlmProvider, ModelTier, Pricing, ProviderConfig, RecordedTransport, StructuredRequest,
};
use hm_mcp::McpServer;
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{Authority, EventEnvelope, EventPayload, Retention, Sensitivity, UserMsg};
use hm_serve::actor::{ActorConfig, ActorEngine, IncomingEvent};
use hm_serve::telemetry::TelemetryMode;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

fn config(path: &Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("7"),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

fn event(conversation: &str, content: &str) -> IncomingEvent {
    IncomingEvent {
        kind: EventKind::UserMsg,
        conversation: ConversationId::derive(conversation),
        payload: encode_event_envelope(&EventEnvelope {
            schema_version: CURRENT_SCHEMA_VERSION,
            payload: EventPayload::UserMsg(Box::new(UserMsg {
                content: content.as_bytes().to_vec(),
            })),
            authority: Authority::UserAsserted,
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 1,
            origin_actor: 0,
            run_id: None,
            model_provenance: None,
            retention: Retention::Durable,
            sensitivity: Sensitivity::Personal,
            event_time_ns: 0,
        }),
    }
}

fn provider() -> Arc<dyn LlmProvider> {
    hm_mcp::telemetry::observed(Arc::new(
        OpenAiCompatible::new(
            ProviderConfig {
                endpoint: "https://gateway.centra.ag/v1/chat/completions".to_owned(),
                api_key: None,
                model: "openrouter/openai/gpt-4o-mini".to_owned(),
                tier: ModelTier::Economy,
                pricing: Pricing {
                    input_microusd_per_million_tokens: 150_000,
                    output_microusd_per_million_tokens: 600_000,
                },
            },
            RecordedTransport::from_json("[]").unwrap(),
        )
        .unwrap(),
    ))
}

fn spans(text: &str, name: &str) -> Vec<Value> {
    text.lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .map(|value| value["resourceSpans"][0]["scopeSpans"][0]["spans"][0].clone())
        .filter(|span| span["name"] == name)
        .collect()
}

fn integer(span: &Value, key: &str) -> Option<i64> {
    attribute(span, key).and_then(|value| value["intValue"].as_i64())
}

fn text(span: &Value, key: &str) -> Option<String> {
    attribute(span, key).and_then(|value| value["stringValue"].as_str().map(str::to_owned))
}

fn attribute(span: &Value, key: &str) -> Option<Value> {
    span["attributes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|attribute| attribute["key"] == key)
        .map(|attribute| attribute["value"].clone())
}

#[tokio::test]
async fn ingestion_extraction_and_provider_spans_carry_only_metadata() {
    let directory = tempfile::tempdir().unwrap();
    let recorded = directory.path().join("spans.jsonl");
    assert_eq!(
        hm_serve::telemetry::configure(TelemetryMode::File, Some(&recorded), "hypermind-test"),
        Ok(true)
    );

    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    actor
        .append(vec![
            event(
                "telemetry-alpha",
                "Sillimanite entered the ingestion batch.",
            ),
            event(
                "telemetry-alpha",
                "Sillimanite was weighed during ingestion.",
            ),
        ])
        .await
        .unwrap();
    actor
        .append(vec![event(
            "telemetry-beta",
            "Sillimanite surfaced in a second conversation.",
        )])
        .await
        .unwrap();

    let server = McpServer::new(actor.clone());
    let envelope = server
        .consolidate_envelope(
            serde_json::from_value(json!({
                "action": "run",
                "mode": "nrem",
                "cadence_key": "telemetry-cadence",
                "budget": {
                    "max_llm_calls": 1,
                    "max_tokens": 128,
                    "max_microusd": 1000,
                    "max_wall_ms": 1000
                }
            }))
            .unwrap(),
        )
        .await;
    assert!(!envelope.ok, "{envelope:?}");

    let structured = provider().generate_structured(&StructuredRequest {
        prompt_id: "telemetry-probe".to_owned(),
        system: "provider stromatolite system".to_owned(),
        prompt: "provider stromatolite prompt".to_owned(),
        json_schema: json!({"type": "object"}),
        maximum_output_tokens: 64,
    });
    assert!(structured.is_err());

    hm_serve::telemetry::flush();
    drop(server);
    actor.shutdown().await.unwrap();

    let exported = std::fs::read_to_string(&recorded).unwrap();
    let names = exported
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .map(|value| {
            value["resourceSpans"][0]["scopeSpans"][0]["spans"][0]["name"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        names,
        BTreeSet::from([
            "hypermind.ingestion".to_owned(),
            "hypermind.extraction".to_owned(),
            "hypermind.provider".to_owned(),
        ])
    );

    let ingestion = spans(&exported, "hypermind.ingestion");
    assert_eq!(ingestion.len(), 2);
    let batch = ingestion
        .iter()
        .find(|span| integer(span, "hypermind.ingestion.events") == Some(2))
        .unwrap();
    assert_eq!(integer(batch, "hypermind.actor"), Some(7));
    assert_eq!(batch["status"]["code"], 1);

    let extraction = spans(&exported, "hypermind.extraction");
    assert_eq!(extraction.len(), 1);
    assert_eq!(
        text(&extraction[0], "hypermind.extraction.mode").as_deref(),
        Some("nrem")
    );
    assert_eq!(integer(&extraction[0], "hypermind.actor"), Some(7));
    assert_eq!(extraction[0]["status"]["code"], 2);

    let provider_spans = spans(&exported, "hypermind.provider");
    assert_eq!(provider_spans.len(), 1);
    let tier = text(&provider_spans[0], "hypermind.provider.tier").unwrap();
    assert!(matches!(tier.as_str(), "economy" | "standard" | "capable"));
    assert_eq!(tier, "economy");
    assert_eq!(provider_spans[0]["status"]["code"], 2);

    let lowercased = exported.to_lowercase();
    assert!(!lowercased.contains("sillimanite"));
    assert!(!lowercased.contains("stromatolite"));
    assert!(!lowercased.contains("telemetry-cadence"));
    assert!(!lowercased.contains("gpt-4o-mini"));
}
