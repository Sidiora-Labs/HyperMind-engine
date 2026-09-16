#![forbid(unsafe_code)]

use base64::Engine as _;
use hm_core::{ActorId, ConversationId, ErrorCode, LSN};
use hm_ledger::frame::EventKind;
use hm_ledger::shred::{DELETION_RECEIPT_BYTES, decode_deletion_receipt};
use hm_mcp::{
    ActivateInput, ForgetAction, ForgetInput, InspectInput, McpServer, RememberInput, RememberKind,
};
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    Authority, EventEnvelope, EventPayload, Retention, Sensitivity, ToolCall, ToolResult,
};
use hm_serve::actor::{ActorConfig, ActorEngine, IncomingEvent};

fn config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("7"),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

fn event(
    kind: EventKind,
    conversation: ConversationId,
    payload: EventPayload,
    authority: Authority,
    run_id: Option<&[u8]>,
) -> IncomingEvent {
    IncomingEvent {
        kind,
        conversation,
        payload: encode_event_envelope(&EventEnvelope {
            schema_version: CURRENT_SCHEMA_VERSION,
            payload,
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 0,
            origin_actor: 7,
            run_id: run_id.map(<[u8]>::to_vec),
            model_provenance: None,
            authority,
            retention: Retention::Durable,
            sensitivity: Sensitivity::Personal,
            event_time_ns: 0,
        }),
    }
}

#[tokio::test]
async fn inspect_walks_provenance_and_mmr_receipts_and_forget_is_ledgered() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(temporary.path())).await.unwrap();
    let server = McpServer::new_with_admin(actor.clone(), [0x5a; 32]);
    let conversation = ConversationId::derive("trust");
    actor
        .append(vec![
            event(
                EventKind::ToolCall,
                conversation,
                EventPayload::ToolCall(Box::new(ToolCall {
                    call_id: b"call-1".to_vec(),
                    tool_name: "read".to_owned(),
                    arguments: b"file".to_vec(),
                })),
                Authority::RuntimeFact,
                Some(b"run-a"),
            ),
            event(
                EventKind::ToolResult,
                conversation,
                EventPayload::ToolResult(Box::new(ToolResult {
                    call_id: b"call-1".to_vec(),
                    tool_call_lsn: 1,
                    result: b"observed".to_vec(),
                    ..ToolResult::default()
                })),
                Authority::ToolObserved,
                Some(b"run-a"),
            ),
        ])
        .await
        .unwrap();

    let inspected = server
        .inspect_envelope(InspectInput {
            uri: Some("hm://7/lsn/2".to_owned()),
            ..InspectInput::default()
        })
        .await;
    assert!(inspected.ok);
    assert_eq!(inspected.provenance, ["hm://7/lsn/2", "hm://7/lsn/1"]);
    assert_eq!(inspected.items[1]["authority"], "tool_observed");
    assert_eq!(inspected.items[2]["authority"], "runtime_fact");
    assert!(
        inspected.items[1]["integrity"]["leaf_hash"]
            .as_str()
            .is_some_and(|value| value.len() == 64)
    );
    assert_eq!(inspected.items[0]["verification"]["leaf_count"], 2);

    let retracted = server
        .forget_envelope(ForgetInput {
            action: ForgetAction::RetractRun,
            lsn: None,
            run_id: Some("run-a".to_owned()),
            admin_token: None,
        })
        .await;
    assert!(retracted.ok);
    assert_eq!(retracted.items[0]["targets"], serde_json::json!([1, 2]));
    assert_eq!(actor.stats().await.unwrap().log_events, 4);

    let faded = server
        .forget_envelope(ForgetInput {
            action: ForgetAction::Fade,
            lsn: Some(2),
            run_id: None,
            admin_token: None,
        })
        .await;
    assert!(faded.ok);
    assert_eq!(actor.stats().await.unwrap().log_events, 5);
    drop(server);
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn tripwire_is_a_warning_and_crypto_shred_requires_the_admin_token() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open_with_tripwires(config(temporary.path()), [LSN::new(1)])
        .await
        .unwrap();
    let server = McpServer::new_with_admin(actor.clone(), [0x5a; 32]);
    let remembered = server
        .remember_envelope(RememberInput {
            conversation: "tripwire".to_owned(),
            content: "tripwire content".to_owned(),
            kind: RememberKind::User,
            chunk_bytes: None,
            anchor: None,
            retention: None,
            sensitivity: None,
        })
        .await;
    assert!(remembered.ok);
    let activation = server
        .activate_envelope(ActivateInput {
            conversation: "tripwire".to_owned(),
            query: String::new(),
            turn_text: String::new(),
            budget_tokens: 1_000,
        })
        .await;
    assert!(!activation.ok);
    assert_eq!(activation.items[0]["error"], ErrorCode::Tripwire.as_str());
    assert_eq!(activation.warnings, ["security_event:tripwire:lsn=1"]);
    let blocked = server
        .forget_envelope(ForgetInput {
            action: ForgetAction::Fade,
            lsn: Some(1),
            run_id: None,
            admin_token: None,
        })
        .await;
    assert!(!blocked.ok);
    assert_eq!(blocked.warnings, ["security_event:tripwire:lsn=1"]);

    let denied = server
        .forget_envelope(ForgetInput {
            action: ForgetAction::CryptoShred,
            lsn: None,
            run_id: None,
            admin_token: Some("00".repeat(32)),
        })
        .await;
    assert!(!denied.ok);
    assert_eq!(
        denied.items[0]["error"],
        ErrorCode::CapabilityDenied.as_str()
    );
    let deleted = server
        .forget_envelope(ForgetInput {
            action: ForgetAction::CryptoShred,
            lsn: None,
            run_id: None,
            admin_token: Some("5a".repeat(32)),
        })
        .await;
    assert!(deleted.ok);
    let receipt = base64::engine::general_purpose::STANDARD
        .decode(deleted.items[0]["receipt_base64"].as_str().unwrap())
        .unwrap();
    assert_eq!(receipt.len(), DELETION_RECEIPT_BYTES);
    assert_eq!(
        decode_deletion_receipt(&receipt).unwrap().actor,
        ActorId::new(7)
    );
}
