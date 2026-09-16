use hm_core::{ActorId, ConversationId, Error, ErrorCode, LSN};
use hm_mcp::{
    BindInput, IntendAction, IntendCloseReason, IntendInput, McpServer, RememberInput, RememberKind,
};
use hm_schema::event::{Boundary, EventKind, verify_event};
use hm_schema::events::EventPayload;
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
#[allow(clippy::too_many_lines)]
async fn intend_and_bind_append_real_continuity_events() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());

    let objective = server
        .intend_envelope(IntendInput {
            conversation: "delivery".to_owned(),
            action: IntendAction::SetObjective {
                objective: "ship the continuity slice".to_owned(),
            },
        })
        .await;
    assert!(objective.ok);
    assert_eq!(objective.items[0]["action"], "set_objective");

    let opened = server
        .intend_envelope(IntendInput {
            conversation: "delivery".to_owned(),
            action: IntendAction::OpenLoop {
                loop_id: "task-2.5".to_owned(),
                objective: "wire checkpoint and subscription".to_owned(),
            },
        })
        .await;
    assert!(opened.ok);
    assert_eq!(opened.items[0]["loop_id"], "task-2.5");

    let evidence = server
        .remember_envelope(RememberInput {
            conversation: "delivery".to_owned(),
            content: "the continuity fixture is observed".to_owned(),
            kind: RememberKind::User,
            chunk_bytes: None,
            anchor: None,
            retention: None,
            sensitivity: None,
            vocabulary: None,
            source: None,
        })
        .await;
    assert!(evidence.ok);
    let evidence_lsn = evidence.items[0]["first_lsn"].as_u64().unwrap();

    let bound = server
        .bind_envelope(BindInput {
            conversation: "delivery".to_owned(),
            task: Some("task-2.5".to_owned()),
            scope: None,
            canonical_entity: "repository".to_owned(),
            property: "revision".to_owned(),
            evidence_lsn,
            revision: "abc123".to_owned(),
            freshness_requirement_ns: 60_000_000_000,
        })
        .await;
    assert!(bound.ok);
    assert_eq!(bound.items[0]["status"], "resolved");
    assert_eq!(bound.items[0]["evidence_lsn"], evidence_lsn);

    let closed = server
        .intend_envelope(IntendInput {
            conversation: "delivery".to_owned(),
            action: IntendAction::CloseLoop {
                loop_id: "task-2.5".to_owned(),
                reason: IntendCloseReason::Abandoned,
                cause: "fixture complete".to_owned(),
                evidence_lsns: Vec::new(),
            },
        })
        .await;
    assert!(closed.ok);

    let records = actor
        .recall(RecallRequest::Timeline {
            conversation: ConversationId::derive("delivery"),
            since_lsn: LSN::new(0),
            limit: 16,
        })
        .await
        .unwrap();
    assert_eq!(records.len(), 5);
    assert_eq!(records[0].kind as u8, EventKind::IntentSet as u8);
    assert_eq!(records[1].kind as u8, EventKind::LoopOpened as u8);
    assert_eq!(records[3].kind as u8, EventKind::Binding as u8);
    assert_eq!(records[4].kind as u8, EventKind::LoopClosed as u8);
    let verified = verify_event(&records[3].payload, EventKind::Binding, Boundary::Disk).unwrap();
    let EventPayload::Binding(binding) = verified.envelope.payload else {
        panic!("expected binding event");
    };
    assert_eq!(binding.task.as_deref(), Some(b"task-2.5".as_slice()));
    assert_eq!(binding.revision, b"abc123");

    let invalid = server
        .bind_envelope(BindInput {
            conversation: "delivery".to_owned(),
            task: Some("task-2.5".to_owned()),
            scope: Some("global".to_owned()),
            canonical_entity: "repository".to_owned(),
            property: "revision".to_owned(),
            evidence_lsn,
            revision: "abc123".to_owned(),
            freshness_requirement_ns: 1,
        })
        .await;
    assert!(!invalid.ok);
    assert_eq!(invalid.effect_state.as_deref(), Some("not_dispatched"));

    drop(server);
    actor.shutdown().await.unwrap();
}

#[test]
fn mutation_errors_cover_all_three_effect_states() {
    use hm_serve::errors::{mutation_effect_state, mutation_effect_state_name};

    assert_eq!(
        mutation_effect_state_name(mutation_effect_state(Error::new(
            ErrorCode::InvalidArgument
        ))),
        "not_dispatched"
    );
    assert_eq!(
        mutation_effect_state_name(mutation_effect_state(Error::new(ErrorCode::LoopNotFound))),
        "rejected"
    );
    assert_eq!(
        mutation_effect_state_name(mutation_effect_state(Error::new(ErrorCode::WriteFailed))),
        "unknown"
    );
}
