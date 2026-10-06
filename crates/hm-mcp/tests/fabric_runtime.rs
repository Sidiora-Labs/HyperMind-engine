use hm_context::{Scope, digest_bytes};
use hm_core::ActorId;
use hm_fabric::{
    backend_config::*,
    backend_runtime::{BackendRuntime, RUNTIME_BACKEND_MIGRATIONS},
    runtime::{RuntimeConfig, worker_from_env},
    supervisor::{Probe, ProcessSpec, RestartPolicy},
};
use hm_mcp::dispatcher::McpToolDispatcher;
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_config::TrustedContextConfig,
    fabric_service::TrustedMetadata,
    uds::ToolDispatcher,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, time::Duration};
fn scope() -> Scope {
    Scope {
        owner_id: "mcp-owner".into(),
        project_id: "mcp-project".into(),
        workspace_id: None,
    }
}
#[test]
#[ignore = "actual signed digest worker subprocess"]
fn fabric_worker_entry() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(worker_from_env())
        .unwrap();
}
async fn call(
    dispatcher: &McpToolDispatcher,
    actor: &ActorEngine,
    verb: &str,
    input: Value,
) -> Value {
    serde_json::from_slice(
        &dispatcher
            .dispatch(
                actor.clone(),
                verb.into(),
                serde_json::to_vec(&input).unwrap(),
            )
            .await
            .unwrap(),
    )
    .unwrap()
}
#[tokio::test]
async fn retained_trusted_dispatcher_runs_actual_fabric_worker() {
    let home = tempfile::tempdir().unwrap();
    let ledger = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(ActorConfig {
        actor_directory: ledger.path().into(),
        actor: ActorId::new(74),
        user: [3; 16],
        kek: [4; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    })
    .await
    .unwrap();
    let selected = select_backend_async(
        validate_descriptor(&BackendDescriptor {
            version: 1,
            scope: scope(),
            home: HomeDescriptor {
                path: home.path().into(),
                base: None,
                missing: MissingHomePolicy::Refuse,
            },
            operational: OperationalBackendDescriptor::Sqlite,
            bus: BusBackendDescriptor::ProcessLocal {
                limits: Default::default(),
            },
        })
        .unwrap(),
        RUNTIME_BACKEND_MIGRATIONS,
        |_| Err(BackendConfigError::SecretUnavailable),
        |_| Err(BackendConfigError::SecretUnavailable),
    )
    .await
    .unwrap();
    let worker = ProcessSpec {
        module_id: "native-digest".into(),
        command: std::env::current_exe().unwrap(),
        args: vec![
            "--exact".into(),
            "fabric_worker_entry".into(),
            "--ignored".into(),
            "--nocapture".into(),
        ],
        env: BTreeMap::new(),
        cwd: home.path().into(),
        readiness: Probe::ProcessAlive,
        health: Probe::ProcessAlive,
        readiness_timeout: Duration::from_secs(3),
        shutdown_timeout: Duration::from_millis(100),
        stderr_bytes: 8192,
        drain_message: None,
        restart: RestartPolicy {
            max_restarts: 0,
            initial_backoff: Duration::ZERO,
            max_backoff: Duration::ZERO,
        },
    };
    let dispatcher = McpToolDispatcher::default()
        .with_fabric_startup(
            TrustedContextConfig {
                version: 1,
                actor: 74,
                scope: scope(),
            },
            RuntimeConfig::new(home.path(), scope(), [7; 32], [8; 32]),
            BackendRuntime::from_selected(selected).unwrap(),
            worker,
            TrustedMetadata::default(),
        )
        .await
        .unwrap();
    let input = json!({"conversation":"fabric","content":"","kind":"user","context":{"operation":"fabric","request":{"version":1,"scope":scope(),"request_id":"digest-one","action":{"action":"dispatch","operation":"digest","bytes":b"real mcp bytes".to_vec(),"timeout_ms":10000}}}});
    let first = call(&dispatcher, &actor, "remember", input.clone()).await;
    assert!(first["ok"] == true, "{first:?}");
    assert_eq!(
        first["items"][0]["result"]["digest"],
        digest_bytes(b"real mcp bytes")
    );
    let repeated = call(&dispatcher, &actor, "remember", input.clone()).await;
    assert!(repeated["ok"] == true);
    assert_eq!(first["items"], repeated["items"]);
    let inspect = call(
        &dispatcher,
        &actor,
        "inspect",
        json!({"uri":"hm://74/context-fabric"}),
    )
    .await;
    assert!(inspect["ok"] == true);
    assert_eq!(
        inspect["items"][0]["descriptors"]["effect_authority"],
        "selected_operational"
    );
    let mut foreign = input.clone();
    foreign["context"]["request"]["scope"]["owner_id"] = json!("foreign");
    assert_eq!(
        call(&dispatcher, &actor, "remember", foreign).await["ok"],
        false
    );
    let mut version = input.clone();
    version["context"]["request"]["version"] = json!(2);
    assert_eq!(
        call(&dispatcher, &actor, "remember", version).await["ok"],
        false
    );
    let mut injected = input;
    injected["context"]["request"]["executable"] = json!("/bin/true");
    assert_eq!(
        call(&dispatcher, &actor, "remember", injected).await["ok"],
        false
    );
    assert!(!home.path().join("effects.sqlite").exists());
    dispatcher.shutdown_fabric().await.unwrap();
}
