use hm_context::retrieval::SourceKind;
use hm_context::{
    Authority, MessagePart, MessageRole, Scope, SourceMessage, SourceSpan, digest_bytes,
};
use hm_core::ActorId;
use hm_mcp::{EmbeddingRuntime, McpServer};
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_retrieval::SourceRecord,
};
use serde_json::{Value, json};

fn scope() -> Scope {
    Scope {
        owner_id: "model-owner".into(),
        project_id: "model-project".into(),
        workspace_id: None,
    }
}
fn config(root: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: root.join("actor"),
        actor: ActorId::new(43),
        user: [5; 16],
        kek: [9; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}
fn record(id: &str, text: &str) -> SourceRecord {
    SourceRecord {
        scope: scope(),
        kind: SourceKind::Document,
        id: id.into(),
        revision: 1,
        text: text.into(),
        content_digest: digest_bytes(text.as_bytes()),
        authority: Authority::ExternalObserved,
        provenance: vec![SourceSpan {
            source_id: format!("source:{id}"),
            source_digest: digest_bytes(text.as_bytes()),
            byte_start: 0,
            byte_end: text.len() as u64,
        }],
        occurred_at_ns: None,
        recorded_at_ns: 200,
        expires_at_ns: None,
        tombstoned: false,
    }
}
async fn operation(server: &McpServer, input: Value) -> hm_mcp::Envelope {
    server.remember_envelope(serde_json::from_value(json!({"conversation":"model-conversation","content":"","kind":"user","context":input})).unwrap()).await
}
async fn activate(server: &McpServer) -> hm_mcp::Envelope {
    server.activate_envelope(serde_json::from_value(json!({"conversation":"model-conversation","query":"Where is the database recovery backup?","budget_tokens":4096,"context":{"version":1,"scope":scope(),"session_id":"model-session","budget":{"context_tokens":4096,"reserved_output_tokens":256,"required_tokens":0}}})).unwrap()).await
}

#[test]
fn environment_selected_local_model_reaches_real_mcp_and_restart() {
    let cache = std::env::var("HM_EMBEDDING_MODEL_DIRECTORY")
        .expect("operator-provisioned model directory is required for actual model qualification");
    let missing = tempfile::tempdir().unwrap();
    for case in ["off", "missing", "local"] {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args(["environment_case", "--ignored", "--exact", "--nocapture"])
            .env_clear()
            .env("HM_CONTEXT_EMBEDDING_CASE", case);
        for name in ["PATH", "LD_LIBRARY_PATH", "ORT_DYLIB_PATH"] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        if case == "off" {
            command.env("HM_EMBEDDING_PROVIDER", "off");
        } else {
            command
                .env("HM_EMBEDDING_PROVIDER", "local")
                .env("HM_EMBEDDING_MODEL", "bge-small-en-v1.5")
                .env(
                    "HM_EMBEDDING_MODEL_DIRECTORY",
                    if case == "missing" {
                        missing.path().as_os_str()
                    } else {
                        std::ffi::OsStr::new(&cache)
                    },
                );
        }
        let status = command.status().unwrap();
        assert!(status.success(), "constructor/MCP case failed: {case}");
    }
    assert_eq!(
        std::fs::read_dir(missing.path()).unwrap().count(),
        0,
        "startup must not download or create model artifacts"
    );
}

#[test]
#[ignore = "isolated environment case invoked by the configured runtime journey"]
fn environment_case() {
    let case = std::env::var("HM_CONTEXT_EMBEDDING_CASE").expect("isolated test case required");
    if case == "off" {
        assert!(EmbeddingRuntime::from_env().unwrap().is_none());
        assert_eq!(
            EmbeddingRuntime::configuration_metadata(None),
            json!({"mode":"off","readiness":"disabled"})
        );
        return;
    }
    if case == "missing" {
        assert!(matches!(
            EmbeddingRuntime::from_env(),
            Err("local embedding artifact is absent; provision the pinned model before startup")
        ));
        return;
    }
    assert_eq!(case, "local");
    let runtime = EmbeddingRuntime::from_env()
        .unwrap()
        .expect("actual local encoder");
    let metadata = EmbeddingRuntime::configuration_metadata(Some(&runtime));
    assert_eq!(metadata["mode"], "local");
    assert_eq!(metadata["readiness"], "loaded");
    assert_eq!(metadata["model"], "BAAI/bge-small-en-v1.5");
    assert_eq!(
        metadata["revision"],
        hm_embed::ModelKind::BgeSmallEnV15.spec().revision
    );
    assert_eq!(metadata["dimensions"], 384);
    assert!(
        !metadata
            .to_string()
            .contains(&std::env::var("HM_EMBEDDING_MODEL_DIRECTORY").unwrap())
    );
    assert!(metadata.get("endpoint").is_none());
    assert!(metadata.get("api_key").is_none());
    drop(runtime);
    let executor = tokio::runtime::Runtime::new().unwrap();
    executor.block_on(async {
    let directory=tempfile::tempdir().unwrap();let actor_config=config(directory.path());let actor=ActorEngine::open(actor_config.clone()).await.unwrap();
    let server=McpServer::configured(actor.clone(),None).await.unwrap().with_context_scope(scope());
    let text="Locate the requested recovery record.";
    let mut message=SourceMessage{id:"source-message".into(),ordinal:1,role:MessageRole::User,parts:vec![MessagePart::Text{text:text.into()}],occurred_at_ns:None,recorded_at_ns:200,authority:Authority::UserAsserted,source_digest:String::new()};message.source_digest=message.computed_digest().unwrap();
    let ingested=operation(&server,json!({"operation":"source","request":{"version":1,"scope":scope(),"session_id":"model-session","conversation":"model-conversation","message":message,"original_bytes":text.as_bytes()}})).await;assert!(ingested.ok,"{ingested:?}");
    for source in [record("vault","The Atlas disaster recovery copy is stored in the Berlin vault."),record("fruit","Ripe oranges and bananas are stored in the kitchen.")] {
        let accepted=operation(&server,json!({"operation":"retrieval_source","expected_revision":0,"source":source})).await;assert!(accepted.ok,"{accepted:?}");
    }
    let configured=operation(&server,json!({"operation":"embedding","expected_revision":0,"enabled":true})).await;assert!(configured.ok,"{configured:?}");assert_eq!(configured.items[0]["registration"]["mode"],"local");assert_eq!(configured.items[0]["registration"]["fingerprint"]["dimensions"],384);
    let first=operation(&server,json!({"operation":"backfill","maximum_items":1,"maximum_bytes":4096})).await;assert!(first.ok,"{first:?}");assert_eq!(first.items[0]["embedded"],1);assert_eq!(first.items[0]["remaining"],2);assert!(first.items[0]["unavailable"].is_null());let checkpoint=first.items[0]["checkpoint"].clone();
    drop(server);actor.shutdown().await.unwrap();
    let actor=ActorEngine::open(actor_config).await.unwrap();let server=McpServer::configured(actor.clone(),None).await.unwrap().with_context_scope(scope());
    let state=server.inspect_envelope(serde_json::from_value(json!({"uri":"hm://43/context-retrieval"})).unwrap()).await;assert!(state.ok,"{state:?}");assert_eq!(state.items[0]["checkpoint"],checkpoint);assert_eq!(state.items[0]["vector_count"],1);
    let second=operation(&server,json!({"operation":"backfill","maximum_items":2,"maximum_bytes":4096})).await;assert!(second.ok,"{second:?}");assert_eq!(second.items[0]["embedded"],2);assert_eq!(second.items[0]["remaining"],0);
    let context=activate(&server).await;assert!(context.ok,"{context:?}");assert_eq!(context.items[0]["retrieval"]["semantic"]["state"],"available");
    let results=context.items[0]["retrieval"]["results"].as_array().unwrap();
    let vault=results.iter().find(|r|r["candidate"]["id"]=="vault").unwrap()["semantic_score"].as_f64().unwrap();let fruit=results.iter().find(|r|r["candidate"]["id"]=="fruit").unwrap()["semantic_score"].as_f64().unwrap();println!("MCP actual local model: vault={vault} fruit={fruit}");assert!(vault>fruit);
    assert!(context.items[0]["report"]["blocks"].as_array().unwrap().iter().any(|b|b["text"]=="The Atlas disaster recovery copy is stored in the Berlin vault."));
    let completed=operation(&server,json!({"operation":"backfill","maximum_items":2,"maximum_bytes":4096})).await;assert!(completed.ok,"{completed:?}");assert_eq!(completed.items[0]["embedded"],0);
    drop(server);actor.shutdown().await.unwrap();
 });
}
