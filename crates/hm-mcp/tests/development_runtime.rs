use hm_context::{
    Authority, MessagePart, MessageRole, Scope, SourceMessage, development::*,
    development_schedule::*,
};
use hm_core::ActorId;
use hm_mcp::dispatcher::McpToolDispatcher;
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    development_service::{DevelopmentAction, DevelopmentRequest, ServiceSchedule},
    uds::ToolDispatcher,
};
use serde_json::{Value, json};
use std::collections::BTreeSet;
#[derive(Debug, serde::Deserialize)]
struct Envelope {
    ok: bool,
    items: Vec<Value>,
}
fn scope() -> Scope {
    Scope {
        owner_id: "runtime-owner".into(),
        project_id: "context".into(),
        workspace_id: None,
    }
}
fn config(root: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: root.join("1"),
        actor: ActorId::new(1),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}
fn dispatcher() -> McpToolDispatcher {
    McpToolDispatcher::from_env().unwrap().with_context_scope(
        hm_serve::context_config::TrustedContextConfig {
            version: 1,
            actor: 1,
            scope: scope(),
        },
    )
}
async fn call(d: &McpToolDispatcher, a: &ActorEngine, verb: &str, args: Value) -> Envelope {
    serde_json::from_slice(
        &d.dispatch(a.clone(), verb.into(), serde_json::to_vec(&args).unwrap())
            .await
            .unwrap(),
    )
    .unwrap()
}
async fn operation(d: &McpToolDispatcher, a: &ActorEngine, op: Value) -> Envelope {
    call(
        d,
        a,
        "remember",
        json!({"conversation":"conversation","kind":"user","content":"","context":op}),
    )
    .await
}
async fn development(
    d: &McpToolDispatcher,
    a: &ActorEngine,
    id: &str,
    action: DevelopmentAction,
) -> Envelope {
    operation(d,a,json!({"operation":"development","request":DevelopmentRequest{version:1,scope:scope(),request_id:id.into(),action}})).await
}
async fn activate(d: &McpToolDispatcher, a: &ActorEngine, defer: bool) -> Envelope {
    call(d,a,"activate",json!({"conversation":"conversation","budget_tokens":4096,"context":{"version":1,"scope":scope(),"session_id":"session","budget":{"context_tokens":4096,"reserved_output_tokens":256,"required_tokens":0},"required_message_ids":["tail"],"defer_reductions":defer}})).await
}
fn source(id: &str, ordinal: u64, text: &str) -> SourceMessage {
    let mut m = SourceMessage {
        id: id.into(),
        ordinal,
        role: MessageRole::User,
        parts: vec![MessagePart::Text { text: text.into() }],
        occurred_at_ns: Some(100),
        recorded_at_ns: 200,
        authority: Authority::UserAsserted,
        source_digest: String::new(),
    };
    m.source_digest = m.computed_digest().unwrap();
    m
}
#[tokio::test]
async fn configured_native_dispatch_summary_cache_and_restart() {
    assert_eq!(
        std::env::var("HM_DEVELOPMENT_PROVIDER").unwrap(),
        "ollama",
        "actual configured provider required"
    );
    let dir = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(dir.path())).await.unwrap();
    let d = dispatcher();
    for (id, n, text) in [
        (
            "reading",
            1,
            "The deployment region is eu-central-1. The release date is 2026-10-06.",
        ),
        ("tail", 2, "Keep the current request verbatim."),
    ] {
        let result=operation(&d,&actor,json!({"operation":"source","request":{"version":1,"scope":scope(),"session_id":"session","conversation":"conversation","message":source(id,n,text),"original_bytes":text.as_bytes()}})).await;
        assert!(result.ok, "{result:?}");
    }
    let initial = activate(&d, &actor, false).await;
    assert!(initial.ok, "{initial:?}");
    let view = call(
        &d,
        &actor,
        "inspect",
        json!({"uri":"hm://1/context-development"}),
    )
    .await;
    assert!(view.ok, "{view:?}");
    assert_eq!(
        view.items[0]["runtime"]["registered_families"],
        json!(["historian", "extraction"])
    );
    let off = McpToolDispatcher::default().with_context_scope(
        hm_serve::context_config::TrustedContextConfig {
            version: 1,
            actor: 1,
            scope: scope(),
        },
    );
    let disabled = call(
        &off,
        &actor,
        "inspect",
        json!({"uri":"hm://1/context-development"}),
    )
    .await;
    assert!(disabled.ok, "{disabled:?}");
    assert_eq!(disabled.items[0]["runtime"]["provider"], "off");
    let malicious=operation(&d,&actor,json!({"operation":"development","request":{"version":1,"scope":scope(),"request_id":"credential","action":{"action":"inspect"},"endpoint":"http://127.0.0.1:9"}})).await;
    assert!(!malicious.ok);
    let mut foreign = scope();
    foreign.owner_id = "foreign".into();
    let refused=operation(&d,&actor,json!({"operation":"development","request":{"version":1,"scope":foreign,"request_id":"foreign","action":{"action":"inspect"}}})).await;
    assert!(!refused.ok);
    let register = DevelopmentAction::Register {
        worker_id: "historian".into(),
        capability_id: "historian-capability".into(),
        session_id: "session".into(),
        conversation: "conversation".into(),
        source_ids: BTreeSet::from(["reading".into()]),
        record_ids: BTreeSet::new(),
        new_record_ids: BTreeSet::from(["a-summary".into(), "b-fact".into(), "c-fact".into()]),
        budget: DevelopmentBudget {
            reserved_tokens: 8192,
            max_input_bytes: 65536,
            max_output_bytes: 65536,
            max_mutations: 3,
        },
        lease_ms: 600_000,
    };
    let registered = development(&d, &actor, "register", register.clone()).await;
    assert!(registered.ok, "{registered:?}");
    let mut unsupported = register.clone();
    if let DevelopmentAction::Register {
        worker_id,
        capability_id,
        ..
    } = &mut unsupported
    {
        *worker_id = "verification".into();
        *capability_id = "unavailable".into();
    }
    let refused = development(&d, &actor, "unsupported", unsupported).await;
    assert!(!refused.ok);
    let cap: WorkerCapability =
        serde_json::from_value(registered.items[0]["capability"].clone()).unwrap();
    let schedule = ServiceSchedule {
        id: "manual".into(),
        mode: ScheduleMode::Manual,
        worker_id: "historian".into(),
        snapshot: SnapshotRequest {
            capability_id: cap.id,
            source_ids: cap.source_ids,
            record_ids: BTreeSet::new(),
        },
        reservation: 8192,
        timeout_ms: 60_000,
        backoff_ms: 0,
        max_attempts: 2,
        identical_failure_limit: 2,
    };
    let configured = development(
        &d,
        &actor,
        "configure",
        DevelopmentAction::Configure { schedule },
    )
    .await;
    assert!(configured.ok, "{configured:?}");
    let queued = development(
        &d,
        &actor,
        "enqueue",
        DevelopmentAction::Enqueue {
            schedule_id: "manual".into(),
        },
    )
    .await;
    assert!(queued.ok, "{queued:?}");
    let job = queued.items[0]["job_id"].as_str().unwrap().to_owned();
    let completed = development(
        &d,
        &actor,
        "dispatch",
        DevelopmentAction::Dispatch {
            job_id: job.clone(),
        },
    )
    .await;
    assert!(completed.ok, "{completed:?}");
    let state: DevelopmentSchedules = serde_json::from_value(completed.items[0].clone()).unwrap();
    assert!(
        matches!(state.jobs[&job].status, DispatchStatus::Complete { .. }),
        "{state:?}"
    );
    assert!(state.accounting.spent > 0);
    let memory = hm_serve::context_memory::rebuild(&actor, &scope())
        .await
        .unwrap();
    assert_eq!(
        memory.records["a-summary"].kind,
        hm_serve::context_memory::RecordKind::Summary
    );
    let deferred = activate(&d, &actor, true).await;
    assert!(deferred.ok, "{deferred:?}");
    assert_eq!(
        deferred.items[0]["cache"]["generation"],
        initial.items[0]["cache"]["generation"]
    );
    assert_eq!(
        deferred.items[0]["cache"]["materialization_digest"],
        initial.items[0]["cache"]["materialization_digest"]
    );
    assert!(
        deferred.items[0]["cache"]["pending_reductions"]
            .as_u64()
            .unwrap()
            > 0
    );
    let folded = activate(&d, &actor, false).await;
    assert!(folded.ok, "{folded:?}");
    assert!(
        folded.items[0]["report"]["blocks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|b| b["id"]
                .as_str()
                .is_some_and(|id| id.starts_with("summary:")))
    );
    assert!(
        folded.items[0]["cache"]["generation"].as_u64().unwrap()
            > initial.items[0]["cache"]["generation"].as_u64().unwrap()
    );
    drop(d);
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(dir.path())).await.unwrap();
    let d = dispatcher();
    let recovered = activate(&d, &actor, false).await;
    assert!(recovered.ok, "{recovered:?}");
    assert_eq!(
        recovered.items[0]["cache"]["materialization_digest"],
        folded.items[0]["cache"]["materialization_digest"]
    );
    let view = call(
        &d,
        &actor,
        "inspect",
        json!({"uri":"hm://1/context-development"}),
    )
    .await;
    assert!(view.ok, "{view:?}");
    assert_eq!(
        view.items[0]["scheduler"]["jobs"][&job]["status"]["state"],
        "complete"
    );
    let history = hm_serve::context_history::replay(&actor, &scope(), "session", "conversation")
        .await
        .unwrap();
    let summaries = hm_serve::development_service::current_summaries(
        &hm_serve::context_memory::rebuild(&actor, &scope())
            .await
            .unwrap(),
        &scope(),
        &history.history,
        2,
        300,
    )
    .unwrap();
    assert!(summaries.summaries.is_empty());
    assert!(
        !summaries.omitted.is_empty(),
        "stale policy must be explicit"
    );
    actor.shutdown().await.unwrap();
}
