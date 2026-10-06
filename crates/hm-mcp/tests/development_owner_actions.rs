use hm_context::{Authority, MessagePart, MessageRole, Scope, SourceMessage};
use hm_core::ActorId;
use hm_mcp::dispatcher::McpToolDispatcher;
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_history::SourceIngestion,
    context_memory,
    development_service::{DevelopmentAction, DevelopmentRequest},
    uds::ToolDispatcher,
};
use serde_json::{Value, json};
fn scope() -> Scope {
    Scope {
        owner_id: "owner".into(),
        project_id: "owner-actions".into(),
        workspace_id: None,
    }
}
fn config(root: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: root.join("actor"),
        actor: ActorId::new(1),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}
fn dispatcher() -> McpToolDispatcher {
    let mut d = McpToolDispatcher::from_env().unwrap().with_context_scope(
        hm_serve::context_config::TrustedContextConfig {
            version: 1,
            actor: 1,
            scope: scope(),
        },
    );
    d.development_runtime = None;
    d
}
async fn op(d: &McpToolDispatcher, a: &ActorEngine, conversation: &str, context: Value) -> Value {
    serde_json::from_slice(
        &d.dispatch(
            a.clone(),
            "remember".into(),
            serde_json::to_vec(
                &json!({"conversation":conversation,"content":"","kind":"user","context":context}),
            )
            .unwrap(),
        )
        .await
        .unwrap(),
    )
    .unwrap()
}
async fn action(
    d: &McpToolDispatcher,
    a: &ActorEngine,
    id: &str,
    action: DevelopmentAction,
) -> Value {
    op(d,a,"room",json!({"operation":"development","request":DevelopmentRequest{version:1,scope:scope(),request_id:id.into(),action}})).await
}
async fn ingest(d: &McpToolDispatcher, a: &ActorEngine, session: &str, text: &str) {
    let mut message = SourceMessage {
        id: format!("source-{session}"),
        ordinal: 1,
        role: MessageRole::User,
        parts: vec![MessagePart::Text { text: text.into() }],
        authority: Authority::UserAsserted,
        occurred_at_ns: None,
        recorded_at_ns: 100,
        source_digest: String::new(),
    };
    message.source_digest = message.computed_digest().unwrap();
    let room = format!("room-{session}");
    let r=op(d,a,&room,json!({"operation":"source","request":SourceIngestion{version:1,scope:scope(),session_id:session.into(),conversation:room.clone(),message,original_bytes:text.as_bytes().to_vec()}})).await;
    assert_eq!(r["ok"], true, "{r}");
}
fn collect(profile: bool, session: &str) -> DevelopmentAction {
    if profile {
        DevelopmentAction::ProfileCollectSession {
            session_id: session.into(),
            conversation: format!("room-{session}"),
        }
    } else {
        DevelopmentAction::PrimerCollectSession {
            session_id: session.into(),
            conversation: format!("room-{session}"),
        }
    }
}
#[tokio::test]
async fn public_owner_consent_collection_backlog_and_restart() {
    let root = tempfile::tempdir().unwrap();
    let a = ActorEngine::open(config(root.path())).await.unwrap();
    let d = dispatcher();
    let empty = a.stats().await.unwrap();
    for (id, inspect) in [
        ("profile-empty", DevelopmentAction::ProfileInspect),
        ("primer-empty", DevelopmentAction::PrimerInspect),
    ] {
        let r = action(&d, &a, id, inspect).await;
        assert_eq!(r["ok"], true, "{r}");
    }
    assert_eq!(a.stats().await.unwrap(), empty);
    ingest(
        &d,
        &a,
        "first",
        "I prefer concise responses. Who approves a release? The owner approves.",
    )
    .await;
    ingest(
        &d,
        &a,
        "second",
        "Keep replies brief. Every release requires owner approval.",
    )
    .await;
    let before = a.stats().await.unwrap();
    assert_eq!(
        action(&d, &a, "no-consent", collect(true, "first")).await["ok"],
        false
    );
    assert_eq!(a.stats().await.unwrap(), before);
    assert_eq!(
        action(
            &d,
            &a,
            "consent",
            DevelopmentAction::ProfileSetEnabled { enabled: true }
        )
        .await["ok"],
        true
    );
    let mut evidence = Vec::new();
    for session in ["first", "second"] {
        for profile in [true, false] {
            let r = action(
                &d,
                &a,
                &format!("collect-{session}-{profile}"),
                collect(profile, session),
            )
            .await;
            assert_eq!(r["ok"], true, "{r}");
            evidence.extend(
                r["items"][0]["source_ids"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap().to_string()),
            );
            let before = a.stats().await.unwrap();
            let retry = action(
                &d,
                &a,
                &format!("retry-{session}-{profile}"),
                collect(profile, session),
            )
            .await;
            assert_eq!(retry["items"][0]["source_ids"], r["items"][0]["source_ids"]);
            assert_eq!(a.stats().await.unwrap(), before);
        }
    }
    let before = a.stats().await.unwrap();
    let mismatch = DevelopmentAction::ProfileCollectSession {
        session_id: "first".into(),
        conversation: "room-second".into(),
    };
    assert_eq!(action(&d, &a, "mismatch", mismatch).await["ok"], false);
    assert_eq!(a.stats().await.unwrap(), before);
    let mut wrong = scope();
    wrong.owner_id = "another-owner".into();
    let r=op(&d,&a,"room",json!({"operation":"development","request":DevelopmentRequest{version:1,scope:wrong,request_id:"wrong-owner".into(),action:DevelopmentAction::ProfileSetEnabled{enabled:false}}})).await;
    assert_eq!(r["ok"], false);
    let r=op(&d,&a,"room",json!({"operation":"development","request":DevelopmentRequest{version:2,scope:scope(),request_id:"wrong-version".into(),action:DevelopmentAction::ProfileSetEnabled{enabled:false}}})).await;
    assert_eq!(r["ok"], false);
    assert_eq!(a.stats().await.unwrap(), before);
    let r = action(
        &d,
        &a,
        "queue",
        DevelopmentAction::PrimerEnqueue {
            job_id: "release-job".into(),
            target_id: "release-primer".into(),
            refresh: false,
        },
    )
    .await;
    assert_eq!(r["ok"], true, "{r}");
    let state = context_memory::rebuild(&a, &scope()).await.unwrap();
    for id in evidence {
        assert!(!state.sources[&id].tombstoned);
        assert!(
            state.sources[&id].locator.ends_with("first")
                || state.sources[&id].locator.ends_with("second")
        );
    }
    assert_eq!(
        state.records["profile-collection-policy"].authority,
        Authority::UserAsserted
    );
    assert!(state.records["profile-collection-policy"].pinned);
    assert_eq!(
        state.records["release-job"].metadata["target_id"],
        "release-primer"
    );
    assert!(!state.records.contains_key("release-primer"));
    let backlog_digest = state.records["release-job"].revision_digest.clone();
    assert_eq!(
        action(
            &d,
            &a,
            "disable",
            DevelopmentAction::ProfileSetEnabled { enabled: false }
        )
        .await["ok"],
        true
    );
    assert_eq!(
        action(&d, &a, "disabled-collect", collect(true, "second")).await["ok"],
        false
    );
    a.shutdown().await.unwrap();
    let a = ActorEngine::open(config(root.path())).await.unwrap();
    let d = dispatcher();
    let before = a.stats().await.unwrap();
    let profile = action(&d, &a, "profile-restart", DevelopmentAction::ProfileInspect).await;
    assert_eq!(profile["ok"], true);
    assert_eq!(profile["items"][0]["enabled"], false);
    assert_eq!(profile["items"][0]["source_count"], 2);
    let primer = action(&d, &a, "primer-restart", DevelopmentAction::PrimerInspect).await;
    assert_eq!(primer["ok"], true);
    assert_eq!(primer["items"][0]["pending"].as_array().unwrap().len(), 1);
    assert_eq!(
        primer["items"][0]["pending"][0]["revision_digest"],
        backlog_digest
    );
    assert_eq!(a.stats().await.unwrap(), before);
    a.shutdown().await.unwrap();
}
