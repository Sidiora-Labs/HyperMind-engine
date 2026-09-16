use hm_core::{ActorId, ConversationId, ErrorCode, LSN};
use hm_ledger::frame::EventKind;
use hm_llm::openai_compat::OpenAiCompatible;
use hm_llm::{ModelTier, Pricing, ProviderConfig, RecordedTransport};
use hm_mcp::{
    McpServer, RecallFilters, RecallInput, RecallMode, ReconstructionRuntime, RememberInput,
    RememberKind,
};
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    Authority, EventEnvelope, EventPayload, ResultStatus, Retention, Sensitivity, ToolCall,
    ToolResult, UserMsg,
};
use hm_serve::actor::{ActorConfig, ActorEngine, IncomingEvent};
use serde_json::json;
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

fn provider() -> Arc<OpenAiCompatible<RecordedTransport>> {
    Arc::new(
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
            RecordedTransport::from_json(include_str!("fixtures/reconstruction-centra.json"))
                .unwrap(),
        )
        .unwrap(),
    )
}

fn event(kind: EventKind, payload: EventPayload, authority: Authority) -> IncomingEvent {
    IncomingEvent {
        kind,
        conversation: ConversationId::new([0; 16]),
        payload: encode_event_envelope(&EventEnvelope {
            schema_version: CURRENT_SCHEMA_VERSION,
            payload,
            authority,
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

async fn seed(actor: &ActorEngine, directory: &Path) {
    actor
        .append(vec![event(
            EventKind::UserMsg,
            EventPayload::UserMsg(Box::new(UserMsg {
                content: b"At 09:00 release 7 was queued.".to_vec(),
            })),
            Authority::UserAsserted,
        )])
        .await
        .unwrap();
    let receipt = directory.join("release.receipt");
    std::fs::write(&receipt, "At 09:05 receipt confirms release 7 completed.").unwrap();
    actor
        .append(vec![event(
            EventKind::ToolCall,
            EventPayload::ToolCall(Box::new(ToolCall {
                call_id: b"release-check".to_vec(),
                tool_name: "read-receipt".to_owned(),
                arguments: receipt.to_str().unwrap().as_bytes().to_vec(),
            })),
            Authority::RuntimeFact,
        )])
        .await
        .unwrap();
    let result = std::fs::read(&receipt).unwrap();
    actor
        .append(vec![event(
            EventKind::ToolResult,
            EventPayload::ToolResult(Box::new(ToolResult {
                call_id: b"release-check".to_vec(),
                tool_call_lsn: 2,
                status: ResultStatus::Ok,
                result,
            })),
            Authority::ToolObserved,
        )])
        .await
        .unwrap();
}

fn recall(anchor_lsns: Vec<u64>) -> RecallInput {
    RecallInput {
        mode: RecallMode::Reconstruct,
        query: "release 7".to_owned(),
        conversation: String::new(),
        limit: 32,
        since_lsn: 0,
        filters: RecallFilters {
            anchor_lsns,
            maximum_output_tokens: Some(256),
            ..RecallFilters::default()
        },
    }
}

#[tokio::test]
async fn reconstruction_uses_verified_anchors_and_cannot_be_remembered() {
    let directory = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    seed(&actor, directory.path()).await;
    let recorded = provider();
    let server = McpServer::new(actor.clone())
        .with_reconstruction_runtime(ReconstructionRuntime::new(recorded.clone()));
    let initial_frames = actor.frames_since(LSN::new(0), None, 32).await.unwrap();
    for anchors in [vec![0, 3], vec![3, 1], vec![1, 999], vec![1, 2]] {
        assert!(!server.recall_envelope(recall(anchors)).await.ok);
        assert_eq!(recorded.transport().remaining(), 1);
    }
    let missing_runtime = McpServer::new(actor.clone())
        .recall_envelope(recall(vec![1, 3]))
        .await;
    assert!(!missing_runtime.ok);
    assert_eq!(
        missing_runtime.items[0]["error"],
        ErrorCode::OperationUnavailable.as_str()
    );
    assert_eq!(recorded.transport().remaining(), 1);
    let result = server.recall_envelope(recall(vec![1, 3])).await;
    assert!(result.ok, "{result:?}");
    assert_eq!(recorded.transport().remaining(), 0);
    assert_eq!(result.items[0]["authority"], "assistant_generated");
    assert_eq!(result.items[0]["label"], "RECONSTRUCTION");
    assert_eq!(result.items[0]["anchor_lsns"], json!([1, 3]));
    assert_eq!(
        result.provenance,
        [
            "hm://7/00000000000000000000000000000000/1",
            "hm://7/00000000000000000000000000000000/3",
        ]
    );
    assert_eq!(result.budget.as_ref().unwrap()["input_tokens"], 283);
    assert_eq!(result.budget.as_ref().unwrap()["output_tokens"], 88);
    let content = result.items[0]["content"].as_str().unwrap();
    assert!(content.starts_with("RECONSTRUCTION\n"));
    for kind in [
        RememberKind::User,
        RememberKind::Assistant,
        RememberKind::Document,
    ] {
        let remembered = server
            .remember_envelope(RememberInput {
                conversation: "reconstruction".to_owned(),
                content: content.to_owned(),
                kind,
                chunk_bytes: None,
                anchor: None,
                retention: None,
                sensitivity: None,
            })
            .await;
        assert!(!remembered.ok);
        assert_eq!(
            remembered.items[0]["error"],
            ErrorCode::ForbiddenKind.as_str()
        );
    }
    assert_eq!(
        actor.frames_since(LSN::new(0), None, 32).await.unwrap(),
        initial_frames
    );
    assert_eq!(
        actor
            .verified_event(LSN::new(1))
            .await
            .unwrap()
            .envelope
            .authority,
        Authority::UserAsserted
    );
    assert_eq!(
        actor
            .verified_event(LSN::new(3))
            .await
            .unwrap()
            .envelope
            .authority,
        Authority::ToolObserved
    );
    drop(server);
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn reconstruction_tripwire_fails_before_the_provider_call() {
    let directory = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open_with_tripwires(config(directory.path()), [LSN::new(3)])
        .await
        .unwrap();
    seed(&actor, directory.path()).await;
    let recorded = provider();
    let server = McpServer::new(actor.clone())
        .with_reconstruction_runtime(ReconstructionRuntime::new(recorded.clone()));
    let denied = server.recall_envelope(recall(vec![1, 3])).await;
    assert!(!denied.ok);
    assert_eq!(denied.items[0]["error"], ErrorCode::Tripwire.as_str());
    assert_eq!(recorded.transport().remaining(), 1);
    assert_eq!(actor.stats().await.unwrap().log_events, 3);
    drop(server);
    actor.shutdown().await.unwrap();
}
