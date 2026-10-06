use hm_context::{Authority, MessagePart, MessageRole, Scope, SourceMessage, TokenBudget};
use hm_core::ActorId;
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_history::{self, SourceIngestion},
    context_memory::{self, MemoryCommand, MemoryGrant, MemoryRecord, MemoryRequest, RecordKind},
    continuity_service::{self, ContinuityAction, ContinuityMode, ContinuityRequest, HookTicket},
    session_context::{self, SessionContextRequest},
};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
fn scope() -> Scope {
    Scope {
        owner_id: "continuity-reader".into(),
        project_id: "project".into(),
        workspace_id: None,
    }
}
fn author() -> Scope {
    Scope {
        owner_id: "continuity-author".into(),
        ..scope()
    }
}
fn config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.into(),
        actor: ActorId::new(77),
        user: [3; 16],
        kek: [4; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}
fn budget() -> TokenBudget {
    TokenBudget {
        context_tokens: 12000,
        reserved_output_tokens: 1000,
        required_tokens: 0,
    }
}
fn hook(id: &str, generation: u64, action: ContinuityAction) -> ContinuityRequest {
    ContinuityRequest {
        version: 1,
        scope: scope(),
        session_id: "session".into(),
        request_id: id.into(),
        expected_generation: generation,
        action,
    }
}
async fn command(actor: &ActorEngine, owner: &Scope, id: &str, command: MemoryCommand) {
    context_memory::execute(
        actor,
        owner,
        owner,
        MemoryRequest {
            version: 1,
            scope: owner.clone(),
            request_id: id.into(),
            command,
        },
    )
    .await
    .unwrap();
}
#[tokio::test]
async fn native_primary_accepted_knowledge_fence_survives_restart_and_refuses_revocation() {
    let root = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(root.path())).await.unwrap();
    let mut message = SourceMessage {
        id: "work".into(),
        ordinal: 0,
        role: MessageRole::User,
        parts: vec![MessagePart::Text {
            text: "Current required work".into(),
        }],
        occurred_at_ns: None,
        recorded_at_ns: 1791288000123456789,
        authority: Authority::UserAsserted,
        source_digest: String::new(),
    };
    message.source_digest = message.computed_digest().unwrap();
    context_history::ingest(
        &actor,
        &scope(),
        &SourceIngestion {
            version: 1,
            scope: scope(),
            session_id: "session".into(),
            conversation: "conversation".into(),
            message,
            original_bytes: b"Current required work".to_vec(),
        },
    )
    .await
    .unwrap();
    command(
        &actor,
        &scope(),
        "required",
        MemoryCommand::Create {
            record: MemoryRecord::new("required", RecordKind::Note, "Protected native rule", 100),
        },
    )
    .await;
    let mut cue = MemoryRecord::new(
        "cue",
        RecordKind::Fact,
        "Calibration observation ".repeat(40),
        100,
    );
    cue.metadata = json!({"context_cue":true});
    command(
        &actor,
        &scope(),
        "cue",
        MemoryCommand::Create { record: cue },
    )
    .await;
    command(
        &actor,
        &author(),
        "shared",
        MemoryCommand::Create {
            record: MemoryRecord::new(
                "shared",
                RecordKind::Fact,
                "Authorized cross-session fact",
                100,
            ),
        },
    )
    .await;
    let revision = context_memory::rebuild(&actor, &author())
        .await
        .unwrap()
        .records["shared"]
        .revision_digest
        .clone();
    command(
        &actor,
        &author(),
        "share",
        MemoryCommand::SetGrant {
            grant: MemoryGrant {
                principal_digest: None,
                id: "share".into(),
                principal: scope(),
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
    let request:SessionContextRequest=serde_json::from_value(json!({"version":1,"scope":scope(),"session_id":"session","budget":budget(),"memory_scopes":[scope(),author()]})).unwrap();
    let activated = session_context::activate(&actor, "conversation", &scope(), request)
        .await
        .unwrap();
    let generation = activated["report"]["generation"].as_u64().unwrap();
    let frozen = activated["knowledge"]["accepted"]["blocks"].clone();
    assert!(
        continuity_service::execute(
            &actor,
            &scope(),
            &scope(),
            hook(
                "wrong-generation",
                generation + 1,
                ContinuityAction::PreHook {
                    budget: budget(),
                    required_ids: vec![]
                }
            )
        )
        .await
        .is_err()
    );
    continuity_service::execute(
        &actor,
        &scope(),
        &scope(),
        hook(
            "configure",
            generation,
            ContinuityAction::Configure {
                mode: ContinuityMode::Primary,
                strict: true,
                expected_revision: 0,
            },
        ),
    )
    .await
    .unwrap();
    let pre = continuity_service::execute(
        &actor,
        &scope(),
        &scope(),
        hook(
            "pre",
            generation,
            ContinuityAction::PreHook {
                budget: budget(),
                required_ids: vec![],
            },
        ),
    )
    .await
    .unwrap();
    assert_eq!(pre["published"], true);
    let rendered = pre["plan"]["rendered"].to_string();
    for expected in [
        "Current required work",
        "Protected native rule",
        "Authorized cross-session fact",
        "Memory cue",
    ] {
        assert!(
            rendered.contains(expected),
            "missing {expected}: {rendered}"
        );
    }
    let ticket: HookTicket = serde_json::from_value(pre["ticket"].clone()).unwrap();
    actor.shutdown().await.unwrap();
    let reopened = ActorEngine::open(config(root.path())).await.unwrap();
    command(
        &reopened,
        &scope(),
        "new-statistical-candidate",
        MemoryCommand::Create {
            record: MemoryRecord::new(
                "later",
                RecordKind::Fact,
                "Not accepted into old generation",
                200,
            ),
        },
    )
    .await;
    let unchanged = session_context::inspect(&reopened, &scope(), Some("session"))
        .await
        .unwrap();
    assert_eq!(unchanged["knowledge"]["accepted"]["blocks"], frozen);
    assert!(
        !unchanged["messages"]
            .to_string()
            .contains("Not accepted into old generation")
    );
    let post = continuity_service::execute(
        &reopened,
        &scope(),
        &scope(),
        hook("post", generation, ContinuityAction::PostHook { ticket }),
    )
    .await
    .unwrap();
    assert_eq!(post["advisory"], true);
    let next = continuity_service::execute(
        &reopened,
        &scope(),
        &scope(),
        hook(
            "next-pre",
            generation,
            ContinuityAction::PreHook {
                budget: budget(),
                required_ids: vec![],
            },
        ),
    )
    .await
    .unwrap();
    let stale: HookTicket = serde_json::from_value(next["ticket"].clone()).unwrap();
    command(
        &reopened,
        &author(),
        "revoke",
        MemoryCommand::RevokeGrant { id: "share".into() },
    )
    .await;
    assert!(
        continuity_service::execute(
            &reopened,
            &scope(),
            &scope(),
            hook(
                "revoked-post",
                generation,
                ContinuityAction::PostHook { ticket: stale }
            )
        )
        .await
        .is_err()
    );
    assert!(
        continuity_service::execute(
            &reopened,
            &scope(),
            &scope(),
            hook(
                "revoked-pre",
                generation,
                ContinuityAction::PreHook {
                    budget: budget(),
                    required_ids: vec![]
                }
            )
        )
        .await
        .is_err()
    );
    assert!(
        session_context::inspect(&reopened, &scope(), Some("session"))
            .await
            .is_err()
    );
    reopened.shutdown().await.unwrap();
}
