use hm_core::ActorId;
use hm_mcp::{
    ConsolidateAction, ConsolidateInput, ConsolidateMode, McpServer, RecallFilters, RecallInput,
    RecallMode, RememberInput, RememberKind,
};
use hm_serve::actor::{ActorConfig, ActorEngine};
use serde_json::Value;
use std::collections::BTreeSet;

const SNAPSHOT: &str = concat!(
    "{\"contract\":\"hypermind.repository-graph.v1\",\"repository\":\"orbit\",\"fact_count\":5}\n",
    "{\"kind\":\"file\",\"name\":\"src/gateway.rs\",\"relations\":[{\"relation\":\"contains\",\"target\":\"forward\"}]}\n",
    "{\"kind\":\"file\",\"name\":\"src/queue.rs\"}\n",
    "{\"kind\":\"symbol\",\"name\":\"forward\",\"path\":\"src/gateway.rs\",\"relations\":[{\"relation\":\"calls\",\"target\":\"src/queue.rs\"}]}\n",
    "{\"kind\":\"test\",\"name\":\"forwards_once\",\"path\":\"src/queue.rs\",\"relations\":[{\"relation\":\"covers\",\"target\":\"forward\"}]}\n",
    "{\"kind\":\"storage\",\"name\":\"outbox\",\"path\":\"src/queue.rs\"}\n",
);

fn actor_config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("5"),
        actor: ActorId::new(5),
        user: [7; 16],
        kek: [8; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

fn snapshot_input() -> RememberInput {
    RememberInput {
        conversation: "repository:orbit".to_owned(),
        content: SNAPSHOT.to_owned(),
        kind: RememberKind::RepositorySnapshot,
        chunk_bytes: Some(256),
        anchor: None,
        retention: None,
        sensitivity: None,
        vocabulary: None,
        source: None,
        derive: None,
        source_delivery: None,
        source_settlement: None,
        document: None,
    }
}

fn extract_input() -> ConsolidateInput {
    ConsolidateInput {
        action: ConsolidateAction::Run,
        mode: Some(ConsolidateMode::Repository),
        scope: Some("orbit".to_owned()),
        cadence_key: None,
        budget: None,
        run_id: None,
        reason: None,
    }
}

fn traversal(anchor: &str, query: &str, limit: usize) -> RecallInput {
    RecallInput {
        mode: RecallMode::Graph,
        query: query.to_owned(),
        conversation: String::new(),
        limit,
        since_lsn: 0,
        filters: RecallFilters {
            anchor: Some(anchor.to_owned()),
            temporal_from_ns: Some(0),
            ..RecallFilters::default()
        },
    }
}

fn shape(items: &[Value]) -> BTreeSet<(String, String, String)> {
    items
        .iter()
        .map(|item| {
            (
                item["relation"].as_str().unwrap().to_owned(),
                item["direction"].as_str().unwrap().to_owned(),
                item["name"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

fn assert_item_contract(item: &Value, actor: u16) {
    assert_eq!(item["authority"].as_str().unwrap(), "derived_inference");
    assert_eq!(item["node_id"].as_str().map(str::len), Some(64));
    assert_eq!(item["edge_id"].as_str().map(str::len), Some(64));
    assert_eq!(item["weight_micros"].as_u64().unwrap(), 250_000);
    assert_eq!(item["valid_from_ns"].as_i64().unwrap(), 0);
    assert_eq!(item["valid_to_ns"].as_i64().unwrap(), 0);
    let edge_lsn = item["edge_lsn"].as_u64().unwrap();
    assert!(edge_lsn > 0);
    assert_eq!(
        item["uri"].as_str().unwrap(),
        format!("hm://{actor}/lsn/{edge_lsn}")
    );
    let support = item["support_lsns"].as_array().unwrap();
    assert!(
        !support.is_empty(),
        "an edge cites the snapshot it came from"
    );
}

fn expected_symbol_neighbours() -> BTreeSet<(String, String, String)> {
    [
        (
            "contains".to_owned(),
            "incoming".to_owned(),
            "file src/gateway.rs".to_owned(),
        ),
        (
            "calls".to_owned(),
            "outgoing".to_owned(),
            "file src/queue.rs".to_owned(),
        ),
        (
            "covers".to_owned(),
            "incoming".to_owned(),
            "test forwards_once".to_owned(),
        ),
    ]
    .into_iter()
    .collect()
}

#[tokio::test]
async fn repository_graph_answers_and_survives_restart() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());

    let remembered = server.remember_envelope(snapshot_input()).await;
    assert!(remembered.ok, "{:?}", remembered.items);
    let snapshot_lsns = (remembered.items[0]["first_lsn"].as_u64().unwrap()
        ..=remembered.items[0]["last_lsn"].as_u64().unwrap())
        .collect::<BTreeSet<_>>();

    let run = server.consolidate_envelope(extract_input()).await;
    assert!(run.ok, "{:?}", run.items);
    assert_eq!(run.items[0]["nodes"].as_u64().unwrap(), 5);
    assert_eq!(run.items[0]["edges"].as_u64().unwrap(), 3);
    assert_eq!(run.items[0]["generation"].as_u64().unwrap(), 1);

    let first = server
        .recall_envelope(traversal("orbit", "forward", 8))
        .await;
    assert!(first.ok, "{:?}", first.items);
    assert_eq!(first.items.len(), 3);
    assert_eq!(shape(&first.items), expected_symbol_neighbours());
    assert_eq!(first.health["generation"].as_u64().unwrap(), 1);
    assert_eq!(first.health["anchor"].as_str().map(str::len), Some(64));
    assert_eq!(
        first.health["anchor_name"].as_str().unwrap(),
        "symbol forward"
    );
    for item in &first.items {
        assert_item_contract(item, 5);
        assert!(
            first
                .provenance
                .contains(&item["uri"].as_str().unwrap().to_owned())
        );
        for lsn in item["support_lsns"].as_array().unwrap() {
            let lsn = lsn.as_u64().unwrap();
            assert!(snapshot_lsns.contains(&lsn), "{lsn} is a snapshot shard");
            assert!(first.provenance.contains(&format!("hm://5/lsn/{lsn}")));
        }
    }

    let queue = first
        .items
        .iter()
        .find(|item| item["relation"].as_str() == Some("calls"))
        .expect("the symbol calls a file")["node_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let second = server.recall_envelope(traversal(&queue, "", 8)).await;
    assert!(second.ok, "{:?}", second.items);
    assert_eq!(
        shape(&second.items),
        [(
            "calls".to_owned(),
            "incoming".to_owned(),
            "symbol forward".to_owned(),
        )]
        .into_iter()
        .collect()
    );
    assert_eq!(second.health["anchor"].as_str().unwrap(), queue);
    assert_eq!(
        second.health["anchor_name"].as_str().unwrap(),
        "file src/queue.rs"
    );

    let clamped = server
        .recall_envelope(traversal("orbit", "forward", 4_096))
        .await;
    assert!(clamped.ok, "{:?}", clamped.items);
    assert_eq!(clamped.items, first.items);

    drop(server);
    actor.shutdown().await.unwrap();

    let reopened = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(reopened.clone());
    let again = server
        .recall_envelope(traversal("orbit", "forward", 8))
        .await;
    assert!(again.ok, "{:?}", again.items);
    assert_eq!(again.items, first.items);
    assert_eq!(again.provenance, first.provenance);
    assert_eq!(again.health, first.health);
    drop(server);
    reopened.shutdown().await.unwrap();
}

#[tokio::test]
async fn unresolved_graph_anchor_is_visible_not_silent() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());
    assert!(server.remember_envelope(snapshot_input()).await.ok);
    assert!(server.consolidate_envelope(extract_input()).await.ok);

    let missing_node = server.recall_envelope(traversal("orbit", "retry", 8)).await;
    assert!(missing_node.ok);
    assert!(missing_node.items.is_empty());
    assert_eq!(missing_node.gaps.len(), 1);
    assert_eq!(
        missing_node.gaps[0]["kind"].as_str().unwrap(),
        "graph_anchor_unresolved"
    );
    assert_eq!(missing_node.gaps[0]["anchor"].as_str().unwrap(), "orbit");

    let unknown_id = server
        .recall_envelope(traversal(&"ab".repeat(32), "", 8))
        .await;
    assert!(unknown_id.ok);
    assert!(unknown_id.items.is_empty());
    assert_eq!(
        unknown_id.gaps[0]["kind"].as_str().unwrap(),
        "graph_anchor_unresolved"
    );

    let mut without_anchor = traversal("orbit", "forward", 8);
    without_anchor.filters.anchor = None;
    let refused = server.recall_envelope(without_anchor).await;
    assert!(!refused.ok);
    assert_eq!(
        refused.items[0]["error"].as_str().unwrap(),
        "kInvalidArgument"
    );

    let mut without_name = traversal("orbit", "", 8);
    without_name.query = String::new();
    let refused = server.recall_envelope(without_name).await;
    assert!(!refused.ok);
    assert_eq!(
        refused.items[0]["error"].as_str().unwrap(),
        "kInvalidArgument"
    );

    drop(server);
    actor.shutdown().await.unwrap();
}
