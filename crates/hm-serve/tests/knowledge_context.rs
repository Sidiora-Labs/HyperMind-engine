use hm_context::{Authority, MessagePart, MessageRole, Scope, SourceMessage};
use hm_core::ActorId;
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_history::{self, SourceIngestion},
    context_memory::{self, MemoryCommand, MemoryGrant, MemoryRecord, MemoryRequest, RecordKind},
    knowledge_injection,
    session_context::{self, SessionContextRequest},
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
fn owner() -> Scope {
    Scope {
        owner_id: "consumer".into(),
        project_id: "project".into(),
        workspace_id: None,
    }
}
fn shared() -> Scope {
    Scope {
        owner_id: "author".into(),
        ..owner()
    }
}
fn config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.into(),
        actor: ActorId::new(19),
        user: [3; 16],
        kek: [4; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}
async fn command(actor: &ActorEngine, scope: &Scope, id: &str, command: MemoryCommand) {
    context_memory::execute(
        actor,
        scope,
        scope,
        MemoryRequest {
            version: 1,
            scope: scope.clone(),
            request_id: id.into(),
            command,
        },
    )
    .await
    .unwrap();
}
fn request(generation: u64, defer: bool) -> SessionContextRequest {
    serde_json::from_value(json!({"version":1,"scope":owner(),"session_id":"session","budget":{"context_tokens":16000,"reserved_output_tokens":1000,"required_tokens":0},"generation":generation,"model_id":"gpt-4o","memory_scopes":[owner(),shared()],"defer_reductions":defer})).unwrap()
}
fn text(view: &Value) -> String {
    view["messages"].to_string()
}
#[tokio::test]
async fn real_activation_knowledge_cues_frozen_generation_grants_and_restart() {
    let root = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(root.path())).await.unwrap();
    let mut message = SourceMessage {
        id: "work".into(),
        ordinal: 0,
        role: MessageRole::User,
        parts: vec![MessagePart::Text {
            text: "Current protected work".into(),
        }],
        occurred_at_ns: Some(1791287999123456789),
        recorded_at_ns: 1791288000123456789,
        authority: Authority::UserAsserted,
        source_digest: String::new(),
    };
    message.source_digest = message.computed_digest().unwrap();
    context_history::ingest(
        &actor,
        &owner(),
        &SourceIngestion {
            version: 1,
            scope: owner(),
            session_id: "session".into(),
            conversation: "conversation".into(),
            message,
            original_bytes: b"Current protected work".to_vec(),
        },
    )
    .await
    .unwrap();
    for (id, kind, content, pinned) in [
        (
            "fact",
            RecordKind::Fact,
            "Deployment region eu-central-1".into(),
            false,
        ),
        (
            "episode",
            RecordKind::Episode,
            "Previous deploy completed".into(),
            false,
        ),
        (
            "primer",
            RecordKind::Primer,
            "Preserve exact source authority".into(),
            false,
        ),
        (
            "pinned",
            RecordKind::Fact,
            "Protected pinned constraint".into(),
            true,
        ),
        (
            "cue",
            RecordKind::Fact,
            format!("Cue original {} END-OF-ORIGINAL", "calibration ".repeat(70)),
            false,
        ),
    ] {
        let mut record = MemoryRecord::new(id, kind, content, 100);
        record.pinned = pinned;
        if id == "cue" {
            record.metadata = json!({"context_cue":true});
        }
        command(&actor, &owner(), id, MemoryCommand::Create { record }).await;
    }
    let foreign = MemoryRecord::new(
        "shared",
        RecordKind::Fact,
        "Authorized shared configuration",
        100,
    );
    command(
        &actor,
        &shared(),
        "shared",
        MemoryCommand::Create { record: foreign },
    )
    .await;
    let revision = context_memory::rebuild(&actor, &shared())
        .await
        .unwrap()
        .records["shared"]
        .revision_digest
        .clone();
    command(
        &actor,
        &shared(),
        "grant",
        MemoryCommand::SetGrant {
            grant: MemoryGrant {
                principal_digest: None,
                id: "share".into(),
                principal: owner(),
                record_ids: BTreeSet::from(["shared".into()]),
                categories: BTreeSet::new(),
                read: true,
                expires_at_ns: None,
                revoked: false,
                revision: 1,
                record_revisions: BTreeMap::from([("shared".into(), revision)]),
            },
        },
    )
    .await;
    let first = session_context::activate(&actor, "conversation", &owner(), request(0, false))
        .await
        .unwrap();
    let generation = first["report"]["generation"].as_u64().unwrap();
    let firsttext = text(&first);
    for expected in [
        "Current protected work",
        "Deployment region",
        "Previous deploy",
        "Preserve exact source",
        "Protected pinned",
        "Authorized shared",
        "Memory cue",
    ] {
        assert!(
            firsttext.contains(expected),
            "missing {expected}: {firsttext}"
        );
    }
    assert!(!firsttext.contains("END-OF-ORIGINAL"));
    assert_eq!(
        first["knowledge"]["accepted"]["cues"][0]["accounting"]["image_tokens"],
        0
    );
    assert!(
        knowledge_injection::exact_recall(&actor, &owner(), &owner(), "cue")
            .await
            .unwrap()
            .unwrap()
            .content
            .contains("END-OF-ORIGINAL")
    );
    let selected: hm_context::knowledge_injection::KnowledgePlan = serde_json::from_value(
        first["knowledge"]["accepted"]["plans"]
            .as_array()
            .unwrap()
            .iter()
            .find(|plan| plan["scope"] == json!(owner()))
            .unwrap()
            .clone(),
    )
    .unwrap();
    knowledge_injection::observe_use(
        &actor,
        &selected,
        generation,
        "actual-delivered-turn",
        &BTreeSet::from(["fact".into()]),
    )
    .await
    .unwrap();
    command(
        &actor,
        &owner(),
        "new",
        MemoryCommand::Create {
            record: MemoryRecord::new("new", RecordKind::Fact, "New generation knowledge", 200),
        },
    )
    .await;
    let deferred =
        session_context::activate(&actor, "conversation", &owner(), request(generation, true))
            .await
            .unwrap();
    assert_eq!(
        deferred["knowledge"]["accepted"]["blocks"],
        first["knowledge"]["accepted"]["blocks"]
    );
    assert_eq!(
        deferred["cache"]["materialization_digest"],
        first["cache"]["materialization_digest"]
    );
    assert!(!text(&deferred).contains("New generation knowledge"));
    let folded = session_context::activate(&actor, "conversation", &owner(), request(0, false))
        .await
        .unwrap();
    assert!(text(&folded).contains("New generation knowledge"));
    assert!(folded["report"]["generation"].as_u64().unwrap() > generation);
    let accepted = folded["knowledge"]["accepted"]["blocks"].clone();
    actor.shutdown().await.unwrap();
    let reopened = ActorEngine::open(config(root.path())).await.unwrap();
    let resumed = session_context::inspect(&reopened, &owner(), Some("session"))
        .await
        .unwrap();
    assert_eq!(resumed["knowledge"]["accepted"]["blocks"], accepted);
    command(
        &reopened,
        &shared(),
        "revoke",
        MemoryCommand::RevokeGrant { id: "share".into() },
    )
    .await;
    assert!(
        session_context::inspect(&reopened, &owner(), Some("session"))
            .await
            .is_err()
    );
    let clean = session_context::activate(&reopened, "conversation", &owner(), request(0, false))
        .await
        .unwrap();
    assert!(!text(&clean).contains("Authorized shared"));
    command(
        &reopened,
        &owner(),
        "delete",
        MemoryCommand::Tombstone {
            id: "fact".into(),
            expected_revision: 1,
        },
    )
    .await;
    assert!(
        session_context::inspect(&reopened, &owner(), Some("session"))
            .await
            .is_err()
    );
    let finalview =
        session_context::activate(&reopened, "conversation", &owner(), request(0, false))
            .await
            .unwrap();
    assert!(!text(&finalview).contains("Deployment region"));
    assert!(text(&finalview).contains("Protected pinned"));
    reopened.shutdown().await.unwrap();
}
