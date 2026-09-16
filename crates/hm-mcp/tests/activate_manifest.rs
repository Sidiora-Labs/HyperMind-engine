use hm_core::ActorId;
use hm_mcp::{ActivateInput, InspectInput, McpServer, RememberInput, RememberKind};
use hm_serve::actor::{ActorConfig, ActorEngine};

fn actor_config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("11"),
        actor: ActorId::new(11),
        user: [3; 16],
        kek: [4; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

async fn seeded_server(path: &std::path::Path) -> McpServer {
    let actor = ActorEngine::open(actor_config(path)).await.unwrap();
    let server = McpServer::new(actor);
    for content in [
        "the heliotrope ledger records every activation",
        "a heliotrope bundle is trimmed to its budget",
        "heliotrope evidence separates retrieval from inclusion",
    ] {
        let remembered = server
            .remember_envelope(RememberInput {
                conversation: "manifest".to_owned(),
                content: content.to_owned(),
                kind: RememberKind::Document,
                chunk_bytes: None,
                anchor: None,
                retention: None,
                sensitivity: None,
                vocabulary: None,
                source: None,
            })
            .await;
        assert!(remembered.ok);
    }
    server
}

fn lsn_set(value: &serde_json::Value, key: &str) -> Vec<u64> {
    value[key]
        .as_array()
        .unwrap_or_else(|| panic!("{key} is an array"))
        .iter()
        .map(|entry| entry.as_u64().expect("lsn is an unsigned integer"))
        .collect()
}

#[tokio::test]
async fn activate_reports_retrieved_selected_and_included() {
    let temporary = tempfile::tempdir().unwrap();
    let server = seeded_server(temporary.path()).await;

    let activated = server
        .activate_envelope(ActivateInput {
            conversation: "manifest".to_owned(),
            query: "heliotrope".to_owned(),
            turn_text: String::new(),
            budget_tokens: 512,
        })
        .await;
    assert!(activated.ok);

    let manifest = activated
        .manifest
        .clone()
        .expect("activate carries manifest");
    assert_eq!(manifest["candidate_lanes"], serde_json::json!(["lexical"]));
    assert_eq!(manifest["manifest_id"].as_str().unwrap().len(), 64);
    assert!(
        manifest["manifest_id"]
            .as_str()
            .unwrap()
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    );
    assert_eq!(manifest["query_digest"].as_str().unwrap().len(), 64);
    assert!(!manifest["encoder"].as_str().unwrap().is_empty());
    assert!(manifest["snapshot_epoch"].is_u64());
    assert!(manifest["index_generation"].is_u64());

    let retrieved = lsn_set(&manifest, "retrieved");
    let selected = lsn_set(&manifest, "selected");
    let included = lsn_set(&manifest, "included");
    let used = lsn_set(&manifest, "used");
    assert!(!retrieved.is_empty());
    assert!(selected.iter().all(|lsn| retrieved.contains(lsn)));
    assert!(included.iter().all(|lsn| selected.contains(lsn)));
    assert!(used.is_empty());

    let serialized = serde_json::to_value(&activated).unwrap();
    assert_eq!(serialized["manifest"], manifest);
}

#[tokio::test]
async fn other_verbs_do_not_carry_a_manifest() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor);

    let remembered = server
        .remember_envelope(RememberInput {
            conversation: "manifest".to_owned(),
            content: "heliotrope alone".to_owned(),
            kind: RememberKind::Document,
            chunk_bytes: None,
            anchor: None,
            retention: None,
            sensitivity: None,
            vocabulary: None,
            source: None,
        })
        .await;
    assert!(remembered.ok);
    assert!(remembered.manifest.is_none());
    assert!(
        serde_json::to_value(&remembered)
            .unwrap()
            .get("manifest")
            .is_none()
    );

    let inspected = server.inspect_envelope(InspectInput::default()).await;
    assert!(inspected.ok);
    assert!(inspected.manifest.is_none());
    assert!(
        serde_json::to_value(&inspected)
            .unwrap()
            .get("manifest")
            .is_none()
    );
}
