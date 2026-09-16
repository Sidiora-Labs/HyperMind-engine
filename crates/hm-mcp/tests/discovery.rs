use hm_core::ActorId;
use hm_embed::HashFeatureEmbedder;
use hm_mcp::{EmbeddingRuntime, InspectInput, McpServer};
use hm_serve::actor::{ActorConfig, ActorEngine};
use serde_json::{Value, json};
use std::sync::Arc;

const ITEM_KEYS: [&str; 8] = [
    "verb",
    "surface",
    "summary",
    "arguments",
    "mutation",
    "requires",
    "available",
    "unavailable_reason",
];

fn actor_config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("7"),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

fn arguments(value: Value) -> InspectInput {
    serde_json::from_value(value).expect("inspect arguments")
}

fn surface<'a>(envelope: &'a hm_mcp::Envelope, id: &str) -> &'a Value {
    envelope
        .items
        .iter()
        .find(|item| item["surface"] == id)
        .unwrap_or_else(|| panic!("{id} is absent from the discovery envelope"))
}

#[tokio::test]
async fn discovery_enumerates_surfaces_without_widening_the_tool_surface() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());

    let status = server.inspect_envelope(arguments(json!({}))).await;
    assert!(status.ok, "{status:?}");
    assert!(status.items[0].get("log_events").is_some(), "{status:?}");
    assert_eq!(status.items[0]["actor"], 7);

    let catalog = server
        .inspect_envelope(arguments(json!({"mode": "discover"})))
        .await;
    assert!(catalog.ok, "{catalog:?}");
    assert_eq!(catalog.items.len(), 12);
    assert_eq!(catalog.health["advertised_tools"], 14);
    assert_eq!(catalog.health["discoverable_surfaces"], 28);
    assert_eq!(catalog.health["returned"], 12);
    assert_eq!(catalog.health["truncated"], true);
    assert!(catalog.provenance.is_empty());
    assert!(catalog.gaps.is_empty());
    assert!(
        catalog
            .warnings
            .contains(&"discovery_is_not_authorization".to_owned()),
        "{catalog:?}"
    );
    for item in &catalog.items {
        for key in ITEM_KEYS {
            assert!(item.get(key).is_some(), "{key} is absent from {item}");
        }
    }

    let shred = server
        .inspect_envelope(arguments(
            json!({"mode": "discover", "query": "shred", "limit": 5}),
        ))
        .await;
    assert_eq!(shred.items.len(), 1, "{shred:?}");
    assert_eq!(shred.items[0]["surface"], "forget.crypto_shred");
    assert_eq!(shred.items[0]["requires"], "admin_token");
    assert_eq!(shred.items[0]["mutation"], true);
    assert_eq!(shred.items[0]["available"], false);
    assert_eq!(shred.items[0]["unavailable_reason"], "admin_token_absent");

    let semantic = server
        .inspect_envelope(arguments(
            json!({"mode": "discover", "query": "semantic vector"}),
        ))
        .await;
    let found = surface(&semantic, "recall.semantic");
    assert_eq!(found["available"], false);
    assert_eq!(found["unavailable_reason"], "embedding_runtime_absent");

    let everything = server
        .inspect_envelope(arguments(json!({"mode": "discover", "limit": 9999})))
        .await;
    assert!(everything.ok, "{everything:?}");
    assert_eq!(everything.items.len(), 28);
    assert_eq!(everything.health["returned"], 28);
    assert_eq!(everything.health["truncated"], false);

    let empty = server
        .inspect_envelope(arguments(
            json!({"mode": "discover", "query": "zzzznotaword"}),
        ))
        .await;
    assert!(empty.ok, "{empty:?}");
    assert!(empty.items.is_empty(), "{empty:?}");
    assert!(
        empty
            .warnings
            .contains(&"discovery_is_not_authorization".to_owned()),
        "{empty:?}"
    );

    let verbs = everything
        .items
        .iter()
        .map(|item| item["verb"].as_str().expect("verb").to_owned())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(verbs.len(), 14);
    assert_eq!(hm_serve::rest::VERBS.len(), 14);
    for verb in &verbs {
        assert!(
            hm_serve::rest::VERBS.contains(&verb.as_str()),
            "{verb} is not an advertised verb"
        );
    }
    assert_eq!(actor.stats().await.unwrap().log_events, 0);

    let encoded = McpServer::new(actor.clone()).with_embedding_runtime(EmbeddingRuntime::new(
        Arc::new(HashFeatureEmbedder::new(16).unwrap()),
    ));
    let ready = encoded
        .inspect_envelope(arguments(
            json!({"mode": "discover", "query": "semantic vector"}),
        ))
        .await;
    let found = surface(&ready, "recall.semantic");
    assert_eq!(found["available"], true);
    assert_eq!(found["unavailable_reason"], Value::Null);

    actor.shutdown().await.unwrap();
}
