use hm_compose::tokens::FallbackWeights;
use hm_core::{ActorId, ConversationId, LSN};
use hm_ledger::frame::EventKind;
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{Authority, EventEnvelope, EventPayload, Retention, Sensitivity, UserMsg};
use hm_serve::actor::{ActivateRequest, ActorConfig, ActorEngine, IncomingEvent, RecallRequest};

fn config(path: &std::path::Path, actor: u16) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join(actor.to_string()),
        actor: ActorId::new(actor),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

fn message(conversation: ConversationId, content: &str) -> IncomingEvent {
    IncomingEvent {
        kind: EventKind::UserMsg,
        conversation,
        payload: encode_event_envelope(&EventEnvelope {
            schema_version: CURRENT_SCHEMA_VERSION,
            payload: EventPayload::UserMsg(Box::new(UserMsg {
                content: content.as_bytes().to_vec(),
            })),
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 0,
            origin_actor: 0,
            run_id: None,
            model_provenance: None,
            authority: Authority::UserAsserted,
            retention: Retention::Durable,
            sensitivity: Sensitivity::Personal,
            event_time_ns: 0,
        }),
    }
}

#[tokio::test]
async fn actor_appends_recalls_activates_and_rebuilds_identically() {
    let temporary = tempfile::tempdir().unwrap();
    let conversation = ConversationId::derive("primary");
    let engine = ActorEngine::open(config(temporary.path(), 7))
        .await
        .unwrap();
    let committed = engine
        .append(vec![
            message(conversation, "the launch code is heliotrope"),
            message(
                ConversationId::derive("other"),
                "heliotrope appears elsewhere",
            ),
        ])
        .await
        .unwrap();
    assert_eq!(committed.first_lsn.get(), 1);
    assert_eq!(committed.last_lsn.get(), 2);
    let recalled = engine
        .recall(RecallRequest::Lexical {
            query: "heliotrope".to_owned(),
            limit: 10,
        })
        .await
        .unwrap();
    assert_eq!(recalled.len(), 2);
    let bundle = engine
        .activate(ActivateRequest {
            conversation,
            query: "heliotrope".to_owned(),
            turn_text: String::new(),
            budget_tokens: 1024,
            token_weights: FallbackWeights::default(),
        })
        .await
        .unwrap();
    assert!(
        bundle
            .sections
            .iter()
            .flat_map(|section| &section.items)
            .any(|item| item.uri.starts_with("hm://7/"))
    );
    let before = engine.stats().await.unwrap();
    assert_eq!(before.applied.last_lsn, LSN::new(2));
    engine.shutdown().await.unwrap();

    let reopened = ActorEngine::open(config(temporary.path(), 7))
        .await
        .unwrap();
    let after = reopened.stats().await.unwrap();
    assert_eq!(before.applied, after.applied);
    assert_eq!(
        after
            .projections
            .iter()
            .find(|projection| projection.name == "bm25")
            .unwrap()
            .applied_lsn,
        LSN::new(2)
    );
    assert_eq!(
        reopened
            .recall(RecallRequest::Timeline {
                conversation,
                since_lsn: LSN::new(0),
                limit: 10,
            })
            .await
            .unwrap()
            .len(),
        1
    );
    reopened.shutdown().await.unwrap();
}

#[tokio::test]
async fn actors_are_isolated_by_directory_and_writer_task() {
    let temporary = tempfile::tempdir().unwrap();
    let conversation = ConversationId::derive("shared-name");
    let first = ActorEngine::open(config(temporary.path(), 1))
        .await
        .unwrap();
    let second = ActorEngine::open(config(temporary.path(), 2))
        .await
        .unwrap();
    first
        .append(vec![message(conversation, "only actor one")])
        .await
        .unwrap();
    assert_eq!(first.stats().await.unwrap().log_events, 1);
    assert_eq!(second.stats().await.unwrap().log_events, 0);
    first.shutdown().await.unwrap();
    second.shutdown().await.unwrap();
}
