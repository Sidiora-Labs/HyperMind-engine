#![allow(clippy::needless_pass_by_value, clippy::too_many_lines)]

use base64::Engine as _;
use hm_core::{ActorId, ConversationId, LSN};
use hm_ledger::frame::EventKind;
use hm_mcp::{ActivateInput, IntendInput, McpServer};
use hm_schema::event::encode_event_envelope;
use hm_schema::events::{
    Authority, EventEnvelope, EventPayload, ProviderFrame, Retention, Sensitivity,
};
use hm_serve::actor::{ActorConfig, ActorEngine, IncomingEvent};
use hm_serve::uds::ToolDispatcher;
use serde_json::{Value, json};

fn config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("7"),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 32 * 1024 * 1024,
    }
}

fn intend(action: Value) -> IntendInput {
    serde_json::from_value(json!({"conversation":"anticipation", "action":action})).unwrap()
}

#[tokio::test]
async fn every_wake_trigger_is_persisted_and_observed_change_is_batched_across_restart() {
    let temporary = tempfile::tempdir().unwrap();
    let configuration = config(temporary.path());
    let actor = ActorEngine::open(configuration.clone()).await.unwrap();
    let server = McpServer::new(actor.clone());
    let triggers = [
        json!({"kind":"at","at_ns":1}),
        json!({"kind":"schedule","schedule":"daily"}),
        json!({"kind":"child_terminal","child_id":"child"}),
        json!({"kind":"process_exit","process_id":"process"}),
        json!({"kind":"file_changed","path":"revision.txt"}),
        json!({"kind":"repository_changed","repository":"repo"}),
        json!({"kind":"channel_message","channel":"updates"}),
        json!({"kind":"external_condition","condition":"ready"}),
        json!({"kind":"user_response","reply_to":"question"}),
        json!({"kind":"entity_mentioned","entity_id":"release"}),
        json!({"kind":"loop_closed","loop_id":"deploy"}),
        json!({"kind":"prediction_resolved","prediction_id":"revision"}),
        json!({"kind":"belief_changed","canonical_identity":"release"}),
    ];
    for (index, trigger) in triggers.into_iter().enumerate() {
        let result = server
            .intend_envelope(intend(json!({"kind":"set_intention",
            "intention_id":format!("intention-{index}"), "objective":format!("follow up {index}"),
            "trigger":trigger,"expires_at_ns":i64::MAX,"reply_route":"conversation"})))
            .await;
        assert!(result.ok, "{result:?}");
        assert!(
            actor
                .intention(format!("intention-{index}").into_bytes())
                .await
                .unwrap()
                .is_some()
        );
    }
    let file = temporary.path().join("revision.txt");
    std::fs::write(&file, "revision-2").unwrap();
    let observed = std::fs::read_to_string(file).unwrap();
    let change = actor
        .append(vec![IncomingEvent {
            kind: EventKind::ProviderFrame,
            conversation: ConversationId::derive("anticipation"),
            payload: encode_event_envelope(&EventEnvelope {
                schema_version: 2,
                payload: EventPayload::ProviderFrame(Box::new(ProviderFrame {
                    provider: "filesystem".to_owned(),
                    api_content: serde_json::to_vec(&json!({
                    "wake":{"kind":"repository_changed","key":"repo"},"revision":observed}))
                    .unwrap(),
                })),
                connection_id: None,
                client_seq: 0,
                client_event_index: 0,
                client_event_count: 1,
                origin_actor: 0,
                run_id: None,
                model_provenance: None,
                authority: Authority::ExternalObserved,
                retention: Retention::CurrentState,
                sensitivity: Sensitivity::Personal,
                event_time_ns: 0,
            }),
        }])
        .await
        .unwrap();
    let wake = json!({"kind":"evaluate_wake","observation_lsn":change.first_lsn.get(),"factors":{
        "urgency":900_000,"expected_value":900_000,"confidence":900_000,"interruption_cost":0,
        "resource_cost":0,"duplication_penalty":0,"quiet_hours":true,"notifications_remaining":10,"workload":0}});
    let result = server.intend_envelope(intend(wake.clone())).await;
    assert!(result.ok, "{result:?}");
    assert_eq!(result.items[0]["fired"].as_array().unwrap().len(), 1);
    let count = actor.stats().await.unwrap().log_events;
    assert!(server.intend_envelope(intend(wake)).await.ok);
    assert_eq!(actor.stats().await.unwrap().log_events, count);
    let bundle = server
        .activate_envelope(ActivateInput {
            conversation: "anticipation".to_owned(),
            query: String::new(),
            turn_text: "new turn".to_owned(),
            budget_tokens: 4096,
        })
        .await;
    assert!(bundle.ok, "{bundle:?}");
    assert!(bundle.items.iter().any(|item| {
        item["tier"] == "prospective"
            && item["content_base64"].as_str().is_some_and(|encoded| {
                base64::engine::general_purpose::STANDARD
                    .decode(encoded)
                    .is_ok_and(|content| content.starts_with(b"BATCH DIGEST"))
            })
    }));
    let cancelled = server
        .intend_envelope(intend(json!({"kind":"cancel_intention",
        "intention_id":"intention-0","reason":"superseded"})))
        .await;
    assert!(cancelled.ok, "{cancelled:?}");
    drop(server);
    actor.shutdown().await.unwrap();
    let reopened = ActorEngine::open(configuration).await.unwrap();
    let server = McpServer::new(reopened.clone());
    let inspected = server
        .inspect_envelope(hm_mcp::InspectInput {
            uri: Some("hm://7/attention".to_owned()),
            ..hm_mcp::InspectInput::default()
        })
        .await;
    assert!(inspected.ok, "{inspected:?}");
    assert!(
        serde_json::to_string(&inspected)
            .unwrap()
            .contains("quiet hours")
    );
    assert_eq!(
        reopened
            .intention(b"intention-0".to_vec())
            .await
            .unwrap()
            .unwrap()
            .status,
        hm_proj::intentions::IntentionStatus::Cancelled
    );
    reopened.shutdown().await.unwrap();
}

#[tokio::test]
async fn wire_tool_dispatch_is_actor_scoped_and_preserves_mutation_error_state() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(temporary.path())).await.unwrap();
    let dispatcher = hm_mcp::dispatcher::McpToolDispatcher::default();
    let invalid = dispatcher
        .dispatch(actor.clone(), "predict".to_owned(), b"{}".to_vec())
        .await
        .unwrap();
    let envelope: Value = serde_json::from_slice(&invalid).unwrap();
    assert_eq!(envelope["ok"], false);
    assert!(envelope["effect_state"].is_string());
    assert!(
        dispatcher
            .dispatch(actor.clone(), "forget".to_owned(), b"{}".to_vec())
            .await
            .is_err()
    );
    assert_eq!(actor.stats().await.unwrap().log_events, 0);
    assert!(actor.verified_event(LSN::new(0)).await.is_err());
    actor.shutdown().await.unwrap();
}
