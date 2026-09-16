use hm_core::{ActorId, ConversationId, LSN};
use hm_mcp::{
    ActivateInput, InspectInput, McpServer, RecallInput, RecallMode, RememberInput, RememberKind,
};
use hm_schema::event::{Boundary, EventKind, verify_event};
use hm_schema::events::Authority;
use hm_serve::actor::{ActorConfig, ActorEngine, RecallRequest};

fn actor_config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("7"),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

#[tokio::test]
async fn four_tools_share_one_real_actor_and_one_envelope() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());

    let remembered = server
        .remember_envelope(RememberInput {
            conversation: "docs".to_owned(),
            content: "heliotrope alpha heliotrope beta".to_owned(),
            kind: RememberKind::Document,
            chunk_bytes: Some(18),
            anchor: None,
            retention: None,
            sensitivity: None,
            vocabulary: None,
            source: None,
            derive: None,
            source_delivery: None,
            source_settlement: None,
        })
        .await;
    assert!(remembered.ok);
    assert_eq!(remembered.provenance.len(), 2);
    assert!(remembered.effect_state.is_none());

    let raw = actor
        .recall(RecallRequest::Timeline {
            conversation: ConversationId::derive("docs"),
            since_lsn: LSN::new(0),
            limit: 10,
        })
        .await
        .unwrap();
    assert_eq!(raw.len(), 2);
    let decoded = verify_event(&raw[0].payload, EventKind::UserMsg, Boundary::Disk).unwrap();
    assert_eq!(decoded.envelope.authority, Authority::ExternalObserved);

    let recalled = server
        .recall_envelope(RecallInput {
            mode: RecallMode::Lexical,
            query: "heliotrope".to_owned(),
            conversation: String::new(),
            limit: 10,
            since_lsn: 0,
            filters: hm_mcp::RecallFilters::default(),
        })
        .await;
    assert!(recalled.ok);
    assert!(!recalled.items.is_empty());
    assert_eq!(recalled.items.len(), recalled.provenance.len());

    let activated = server
        .activate_envelope(ActivateInput {
            conversation: "new-turn".to_owned(),
            query: "heliotrope".to_owned(),
            turn_text: String::new(),
            budget_tokens: 1024,
        })
        .await;
    assert!(activated.ok);
    assert!(activated.budget.is_some());
    assert!(activated.health.get("bundle_hash").is_some());

    let inspected = server.inspect_envelope(InspectInput::default()).await;
    assert!(inspected.ok);
    assert_eq!(inspected.items[0]["log_events"], 2);

    let rejected = server
        .remember_envelope(RememberInput {
            conversation: "docs".to_owned(),
            content: String::new(),
            kind: RememberKind::User,
            chunk_bytes: None,
            anchor: None,
            retention: None,
            sensitivity: None,
            vocabulary: None,
            source: None,
            derive: None,
            source_delivery: None,
            source_settlement: None,
        })
        .await;
    assert!(!rejected.ok);
    assert_eq!(rejected.effect_state.as_deref(), Some("not_dispatched"));

    drop(server);
    actor.shutdown().await.unwrap();
}
