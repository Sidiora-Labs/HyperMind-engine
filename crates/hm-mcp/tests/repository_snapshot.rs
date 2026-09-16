use hm_core::{ActorId, ConversationId, LSN};
use hm_mcp::{McpServer, RecallInput, RecallMode, RememberInput, RememberKind};
use hm_schema::event::{Boundary, EventKind, REPOSITORY_SNAPSHOT_PROVIDER, verify_event};
use hm_schema::events::{Authority, EventPayload};
use hm_serve::actor::{ActorConfig, ActorEngine, RecallRequest};

const DOCUMENT: &str = concat!(
    "{\"contract\": \"hypermind.repository-graph.v1\", \"repository\": \"atlas\"}\n",
    "{\"kind\": \"file\", \"name\": \"src/router.rs\"}\n",
    "{\"kind\": \"file\", \"name\": \"src/store.rs\"}\n",
    "{\"kind\": \"symbol\", \"name\": \"dispatch\", \"path\": \"src/router.rs\"}\n",
    "{\"kind\": \"route\", \"name\": \"GET /health\", \"path\": \"src/router.rs\"}\n",
    "{\"kind\": \"test\", \"name\": \"dispatches\", \"path\": \"src/store.rs\"}\n",
);

fn actor_config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("7"),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

fn snapshot_input(content: &str, chunk_bytes: usize) -> RememberInput {
    RememberInput {
        conversation: "repository:atlas".to_owned(),
        content: content.to_owned(),
        kind: RememberKind::RepositorySnapshot,
        chunk_bytes: Some(chunk_bytes),
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

async fn shards(actor: &ActorEngine) -> Vec<(LSN, String)> {
    actor
        .recall(RecallRequest::Timeline {
            conversation: ConversationId::derive("repository:atlas"),
            since_lsn: LSN::new(0),
            limit: 512,
        })
        .await
        .unwrap()
        .into_iter()
        .map(|record| {
            let decoded =
                verify_event(&record.payload, EventKind::ProviderFrame, Boundary::Disk).unwrap();
            assert_eq!(decoded.envelope.authority, Authority::ExternalObserved);
            let EventPayload::ProviderFrame(frame) = decoded.envelope.payload else {
                panic!("a repository snapshot shard is a provider frame");
            };
            assert_eq!(frame.provider, REPOSITORY_SNAPSHOT_PROVIDER);
            (record.lsn, String::from_utf8(frame.api_content).unwrap())
        })
        .collect()
}

#[tokio::test]
async fn repository_snapshot_appends_line_aligned_external_frames() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());

    let remembered = server.remember_envelope(snapshot_input(DOCUMENT, 96)).await;
    assert!(remembered.ok, "{:?}", remembered.items);

    let shard_count = usize::try_from(remembered.items[0]["shards"].as_u64().unwrap()).unwrap();
    assert!(shard_count > 1);
    assert_eq!(remembered.provenance.len(), shard_count);

    let mut hasher = blake3::Hasher::new();
    hasher.update(b"hypermind.repository-snapshot.v1");
    hasher.update(&[0u8]);
    hasher.update(DOCUMENT.as_bytes());
    assert_eq!(
        remembered.items[0]["snapshot_digest"].as_str().unwrap(),
        hasher.finalize().to_hex().as_str()
    );

    let observed = shards(&actor).await;
    assert_eq!(observed.len(), shard_count);
    let mut rebuilt = String::new();
    for (index, (_, content)) in observed.iter().enumerate() {
        assert!(content.ends_with('\n'), "shard {index} ends mid line");
        rebuilt.push_str(content);
    }
    assert_eq!(rebuilt, DOCUMENT);

    for (index, (lsn, _)) in observed.iter().enumerate() {
        let raw = actor.verified_event(*lsn).await.unwrap();
        assert_eq!(
            raw.envelope.client_event_index,
            u32::try_from(index).unwrap()
        );
        assert_eq!(
            raw.envelope.client_event_count,
            u32::try_from(observed.len()).unwrap()
        );
    }
    assert!(remembered.items[0]["embedding_first_lsn"].is_null());
}

#[tokio::test]
async fn repository_snapshot_is_reachable_by_entity_recall() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());

    assert!(
        server
            .remember_envelope(snapshot_input(DOCUMENT, 96))
            .await
            .ok
    );
    let observed = shards(&actor).await;

    let recalled = server
        .recall_envelope(RecallInput {
            mode: RecallMode::Entity,
            query: "src/router.rs".to_owned(),
            conversation: String::new(),
            limit: 10,
            since_lsn: 0,
            filters: hm_mcp::RecallFilters::default(),
        })
        .await;
    assert!(recalled.ok, "{:?}", recalled.items);
    assert!(!recalled.items.is_empty());

    let mut matched = 0;
    for item in &recalled.items {
        let lsn = item["lsn"].as_u64().unwrap();
        let (_, expected) = observed
            .iter()
            .find(|(shard, _)| shard.get() == lsn)
            .expect("an entity hit names a shard of the snapshot");
        assert_eq!(item["content"].as_str().unwrap(), expected);
        assert_eq!(item["authority"].as_str().unwrap(), "external_observed");
        if expected.contains("src/router.rs") {
            matched += 1;
        }
    }
    assert!(matched > 0, "the queried path is in no recalled shard");
}

#[tokio::test]
async fn oversized_line_and_shard_count_are_refused() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());
    let before = actor.stats().await.unwrap().applied.last_lsn;

    let long_line = format!("{}\n", "n".repeat(200));
    let refused = server
        .remember_envelope(snapshot_input(&long_line, 64))
        .await;
    assert!(!refused.ok);
    assert_eq!(
        refused.items[0]["error"].as_str().unwrap(),
        "kCapacityExceeded"
    );
    assert!(refused.effect_state.is_some());
    assert_eq!(actor.stats().await.unwrap().applied.last_lsn, before);

    let many_lines = "0123456789abcdef\n".repeat(300);
    let refused = server
        .remember_envelope(snapshot_input(&many_lines, 24))
        .await;
    assert!(!refused.ok);
    assert_eq!(
        refused.items[0]["error"].as_str().unwrap(),
        "kCapacityExceeded"
    );
    assert!(refused.effect_state.is_some());
    assert_eq!(actor.stats().await.unwrap().applied.last_lsn, before);
}
