use hm_context::{
    Authority, MessagePart, MessageRole, Scope, SourceMessage, TokenBudget,
    history::SourceRelation, source_continuity::ContributionLimits,
};
use hm_core::ActorId;
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_history::{self, RelationIngestion, SourceIngestion},
    continuity_service::*,
    session_context::{self, SessionContextRequest},
};
use serde_json::json;
fn scope() -> Scope {
    Scope {
        owner_id: "continuity-owner".into(),
        project_id: "continuity-project".into(),
        workspace_id: None,
    }
}
fn config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.into(),
        actor: ActorId::new(73),
        user: [3; 16],
        kek: [4; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}
fn budget() -> TokenBudget {
    TokenBudget {
        context_tokens: 1800,
        reserved_output_tokens: 256,
        required_tokens: 0,
    }
}
fn request(id: &str, generation: u64, action: ContinuityAction) -> ContinuityRequest {
    ContinuityRequest {
        version: 1,
        scope: scope(),
        session_id: "session".into(),
        request_id: id.into(),
        expected_generation: generation,
        action,
    }
}
async fn ingest(actor: &ActorEngine, id: &str, ordinal: u64, text: String) -> Vec<u8> {
    let mut message = SourceMessage {
        id: id.into(),
        ordinal,
        role: MessageRole::User,
        parts: vec![MessagePart::Text { text: text.clone() }],
        occurred_at_ns: None,
        recorded_at_ns: 200 + ordinal as i64,
        authority: Authority::UserAsserted,
        source_digest: String::new(),
    };
    message.source_digest = message.computed_digest().unwrap();
    let bytes = serde_json::to_vec(&json!({"original":text,"id":id,"ordinal":ordinal})).unwrap();
    context_history::ingest(
        actor,
        &scope(),
        &SourceIngestion {
            version: 1,
            scope: scope(),
            session_id: "session".into(),
            conversation: "conversation".into(),
            message,
            original_bytes: bytes.clone(),
        },
    )
    .await
    .unwrap();
    bytes
}
async fn activate(actor: &ActorEngine) -> u64 {
    let request:SessionContextRequest=serde_json::from_value(json!({"version":1,"scope":scope(),"session_id":"session","budget":{"context_tokens":12000,"reserved_output_tokens":256,"required_tokens":0}})).unwrap();
    session_context::activate(actor, "conversation", &scope(), request)
        .await
        .unwrap()["cache"]["generation"]
        .as_u64()
        .unwrap()
}
#[tokio::test]
async fn actual_restart_pressure_hooks_and_source_edit_are_fenced() {
    let directory = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    let mut original = Vec::new();
    for n in 0..8 {
        let bytes = ingest(
            &actor,
            &format!("message-{n}"),
            n + 1,
            if n == 7 {
                "Current required user question".into()
            } else {
                "The original instrument recorded stable pressure and temperature. ".repeat(300)
            },
        )
        .await;
        if n == 0 {
            original = bytes
        }
    }
    let generation = activate(&actor).await;
    let off = execute(
        &actor,
        &scope(),
        &scope(),
        request(
            "off",
            generation,
            ContinuityAction::PreHook {
                budget: budget(),
                required_ids: vec![],
            },
        ),
    )
    .await
    .unwrap();
    assert_eq!(off["provider_input_changed"], false);
    execute(
        &actor,
        &scope(),
        &scope(),
        request(
            "shadow",
            generation,
            ContinuityAction::Configure {
                mode: ContinuityMode::Shadow,
                strict: false,
                expected_revision: 0,
            },
        ),
    )
    .await
    .unwrap();
    let before = actor.stats().await.unwrap().applied.last_lsn;
    let shadow = execute(
        &actor,
        &scope(),
        &scope(),
        request(
            "shadow-plan",
            generation,
            ContinuityAction::PreHook {
                budget: budget(),
                required_ids: vec![],
            },
        ),
    )
    .await
    .unwrap();
    assert_eq!(shadow["published"], false);
    assert!(
        !shadow["plan"]["reduction"]["omitted"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        shadow["plan"]["rendered"]["report"]["token_count"]
            .as_u64()
            .unwrap()
            <= 1544
    );
    assert_eq!(actor.stats().await.unwrap().applied.last_lsn, before);
    let mut foreign = scope();
    foreign.owner_id = "foreign".into();
    assert!(
        execute(
            &actor,
            &scope(),
            &foreign,
            request("foreign", generation, ContinuityAction::Inspect)
        )
        .await
        .is_err()
    );
    execute(
        &actor,
        &scope(),
        &scope(),
        request(
            "primary",
            generation,
            ContinuityAction::Configure {
                mode: ContinuityMode::Primary,
                strict: false,
                expected_revision: 1,
            },
        ),
    )
    .await
    .unwrap();
    let pre = execute(
        &actor,
        &scope(),
        &scope(),
        request(
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
    let ticket: HookTicket = serde_json::from_value(pre["ticket"].clone()).unwrap();
    assert_eq!(pre["published"], true);
    assert!(
        execute(
            &actor,
            &scope(),
            &scope(),
            request(
                "mid-turn",
                generation,
                ContinuityAction::Configure {
                    mode: ContinuityMode::Off,
                    strict: false,
                    expected_revision: 2
                }
            )
        )
        .await
        .is_err()
    );
    let replay = execute(
        &actor,
        &scope(),
        &scope(),
        request(
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
    assert_eq!(
        replay["plan"]["rendered"]["digest"],
        pre["plan"]["rendered"]["digest"]
    );
    let post = execute(
        &actor,
        &scope(),
        &scope(),
        request("post", generation, ContinuityAction::PostHook { ticket }),
    )
    .await
    .unwrap();
    assert_eq!(post["advisory"], true);
    execute(
        &actor,
        &scope(),
        &scope(),
        request(
            "strict",
            generation,
            ContinuityAction::Configure {
                mode: ContinuityMode::Primary,
                strict: true,
                expected_revision: 2,
            },
        ),
    )
    .await
    .unwrap();
    let before = actor.stats().await.unwrap().applied.last_lsn;
    assert!(
        execute(
            &actor,
            &scope(),
            &scope(),
            request(
                "strict-wall",
                generation,
                ContinuityAction::Pressure {
                    budget: budget(),
                    required_ids: vec![]
                }
            )
        )
        .await
        .is_err()
    );
    assert_eq!(actor.stats().await.unwrap().applied.last_lsn, before);
    execute(
        &actor,
        &scope(),
        &scope(),
        request(
            "relax",
            generation,
            ContinuityAction::Configure {
                mode: ContinuityMode::Primary,
                strict: false,
                expected_revision: 3,
            },
        ),
    )
    .await
    .unwrap();
    let fork = execute(
        &actor,
        &scope(),
        &scope(),
        request(
            "fork-plan",
            generation,
            ContinuityAction::PlanFork {
                child_session_id: "child".into(),
                limits: ContributionLimits {
                    max_blocks: 2,
                    max_sources: 4,
                    max_bytes: 4096,
                    max_output_tokens: 512,
                    max_child_tokens: 1024,
                    max_child_calls: 2,
                },
            },
        ),
    )
    .await
    .unwrap();
    assert_eq!(fork["parent"]["session_id"], "session");
    ingest(
        &actor,
        "replacement",
        9,
        "Corrected instrument result".into(),
    )
    .await;
    let plan = execute(
        &actor,
        &scope(),
        &scope(),
        request(
            "edit-plan",
            generation,
            ContinuityAction::PlanEdit {
                relation: SourceRelation::Edit {
                    id: "correction".into(),
                    original_id: "message-0".into(),
                    replacement_id: "replacement".into(),
                },
            },
        ),
    )
    .await
    .unwrap();
    assert_eq!(plan["original"]["id"], "message-0");
    context_history::relate(
        &actor,
        &scope(),
        &RelationIngestion {
            version: 1,
            scope: scope(),
            session_id: "session".into(),
            conversation: "conversation".into(),
            relation: serde_json::from_value(plan["relation"].clone()).unwrap(),
        },
    )
    .await
    .unwrap();
    assert!(
        execute(
            &actor,
            &scope(),
            &scope(),
            request(
                "stale-cache",
                generation,
                ContinuityAction::Pressure {
                    budget: budget(),
                    required_ids: vec![]
                }
            )
        )
        .await
        .is_err()
    );
    let history = context_history::replay(&actor, &scope(), "session", "conversation")
        .await
        .unwrap()
        .history;
    assert_eq!(
        history
            .recover(&scope(), &history.source_span("message-0").unwrap())
            .unwrap(),
        original
    );
    let generation = activate(&actor).await;
    let pre = execute(
        &actor,
        &scope(),
        &scope(),
        request(
            "restart-pre",
            generation,
            ContinuityAction::PreHook {
                budget: budget(),
                required_ids: vec![],
            },
        ),
    )
    .await
    .unwrap();
    let ticket: HookTicket = serde_json::from_value(pre["ticket"].clone()).unwrap();
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    let view = execute(
        &actor,
        &scope(),
        &scope(),
        request("inspect", generation, ContinuityAction::Inspect),
    )
    .await
    .unwrap();
    assert_eq!(view["active_hook"]["trace_id"], "restart-pre");
    let post = execute(
        &actor,
        &scope(),
        &scope(),
        request(
            "restart-post",
            generation,
            ContinuityAction::PostHook { ticket },
        ),
    )
    .await
    .unwrap();
    assert_eq!(post["advisory"], true);
    let enlarged = TokenBudget {
        context_tokens: 12001,
        ..budget()
    };
    assert!(
        execute(
            &actor,
            &scope(),
            &scope(),
            request(
                "too-large",
                generation,
                ContinuityAction::Pressure {
                    budget: enlarged,
                    required_ids: vec![]
                }
            )
        )
        .await
        .is_err()
    );
    actor.shutdown().await.unwrap();
}
