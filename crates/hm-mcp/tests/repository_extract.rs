use hm_core::{ActorId, ConversationId, LSN};
use hm_cortex::repograph::{RepoFactKind, node_display_name, node_id};
use hm_mcp::{
    ConsolidateAction, ConsolidateInput, ConsolidateMode, McpServer, RememberInput, RememberKind,
};
use hm_schema::event::{Boundary, EventKind, REPOSITORY_SNAPSHOT_PROVIDER, verify_event};
use hm_schema::events::{Authority, ConsolidationClosed, EventPayload};
use hm_serve::actor::{ActorConfig, ActorEngine};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

const FIRST: &str = concat!(
    "{\"contract\":\"hypermind.repository-graph.v1\",\"repository\":\"atlas\",\"fact_count\":6}\n",
    "{\"kind\":\"file\",\"name\":\"src/router.rs\",\"relations\":[{\"relation\":\"contains\",\"target\":\"dispatch\"}]}\n",
    "{\"kind\":\"file\",\"name\":\"src/store.rs\"}\n",
    "{\"kind\":\"symbol\",\"name\":\"dispatch\",\"path\":\"src/router.rs\",\"relations\":[{\"relation\":\"calls\",\"target\":\"src/store.rs\"}]}\n",
    "{\"kind\":\"route\",\"name\":\"GET /health\",\"path\":\"src/router.rs\",\"relations\":[{\"relation\":\"routes_to\",\"target\":\"dispatch\"}]}\n",
    "{\"kind\":\"test\",\"name\":\"dispatches\",\"path\":\"src/store.rs\",\"relations\":[{\"relation\":\"covers\",\"target\":\"dispatch\"}]}\n",
    "{\"kind\":\"storage\",\"name\":\"sessions\",\"path\":\"src/store.rs\"}\n",
);

const SECOND: &str = concat!(
    "{\"contract\":\"hypermind.repository-graph.v1\",\"repository\":\"atlas\",\"fact_count\":6}\n",
    "{\"kind\":\"file\",\"name\":\"src/router.rs\",\"path\":\"src/router.rs\",\"relations\":[{\"relation\":\"contains\",\"target\":\"dispatch\"},{\"relation\":\"imports\",\"target\":\"src/store.rs\"}]}\n",
    "{\"kind\":\"file\",\"name\":\"src/store.rs\"}\n",
    "{\"kind\":\"symbol\",\"name\":\"dispatch\",\"path\":\"src/router.rs\",\"relations\":[{\"relation\":\"calls\",\"target\":\"src/store.rs\"}]}\n",
    "{\"kind\":\"route\",\"name\":\"GET /health\",\"path\":\"src/router.rs\",\"relations\":[{\"relation\":\"routes_to\",\"target\":\"dispatch\"}]}\n",
    "{\"kind\":\"test\",\"name\":\"dispatches\",\"path\":\"src/store.rs\",\"relations\":[{\"relation\":\"covers\",\"target\":\"dispatch\"}]}\n",
    "{\"kind\":\"storage\",\"name\":\"sessions\",\"path\":\"src/store.rs\"}\n",
);

const WRONG_CONTRACT: &str = concat!(
    "{\"contract\":\"example.other-graph.v1\",\"repository\":\"atlas\",\"fact_count\":1}\n",
    "{\"kind\":\"file\",\"name\":\"src/router.rs\"}\n",
);

fn actor_config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("9"),
        actor: ActorId::new(9),
        user: [3; 16],
        kek: [4; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

fn snapshot_input(content: &str) -> RememberInput {
    RememberInput {
        conversation: "repository:atlas".to_owned(),
        content: content.to_owned(),
        kind: RememberKind::RepositorySnapshot,
        chunk_bytes: Some(256),
        anchor: None,
        retention: None,
        sensitivity: None,
        vocabulary: None,
        source: None,
        derive: None,
    }
}

fn extract_input() -> ConsolidateInput {
    ConsolidateInput {
        action: ConsolidateAction::Run,
        mode: Some(ConsolidateMode::Repository),
        scope: Some("atlas".to_owned()),
        cadence_key: None,
        budget: None,
        run_id: None,
        reason: None,
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(output, "{byte:02x}").unwrap();
    }
    output
}

fn expected_run_id(digest_hex: &str) -> Vec<u8> {
    format!("repository-graph/{digest_hex}").into_bytes()
}

async fn shards(actor: &ActorEngine) -> BTreeMap<u64, Vec<u8>> {
    let mut output = BTreeMap::new();
    for frame in actor
        .frames_since(
            LSN::new(0),
            Some(ConversationId::derive("repository:atlas")),
            usize::MAX,
        )
        .await
        .unwrap()
    {
        if frame.header.kind != hm_ledger::frame::EventKind::ProviderFrame {
            continue;
        }
        let verified = verify_event(
            &frame.sealed_payload,
            EventKind::ProviderFrame,
            Boundary::Disk,
        )
        .unwrap();
        let EventPayload::ProviderFrame(observed) = verified.envelope.payload else {
            panic!("a snapshot shard is a provider frame");
        };
        assert_eq!(observed.provider, REPOSITORY_SNAPSHOT_PROVIDER);
        output.insert(frame.header.lsn.get(), observed.api_content);
    }
    output
}

async fn derived_frames(
    actor: &ActorEngine,
    run_id: &[u8],
    digest: &str,
) -> (usize, usize, ConsolidationClosed) {
    let contents = shards(actor).await;
    let mut minted = 0;
    let mut asserted = 0;
    let mut closed = None;
    for frame in actor
        .frames_since(LSN::new(0), None, usize::MAX)
        .await
        .unwrap()
    {
        let kind = match frame.header.kind {
            hm_ledger::frame::EventKind::MemoryMinted => EventKind::MemoryMinted,
            hm_ledger::frame::EventKind::EdgeAsserted => EventKind::EdgeAsserted,
            hm_ledger::frame::EventKind::ConsolidationClosed => EventKind::ConsolidationClosed,
            _ => continue,
        };
        let verified = verify_event(&frame.sealed_payload, kind, Boundary::Disk).unwrap();
        assert_eq!(verified.envelope.run_id.as_deref(), Some(run_id));
        if kind == EventKind::ConsolidationClosed {
            let EventPayload::ConsolidationClosed(payload) = verified.envelope.payload else {
                panic!("a closed frame carries a closed payload");
            };
            closed = Some(*payload);
            continue;
        }
        assert_eq!(verified.envelope.authority, Authority::DerivedInference);
        let model = verified.envelope.model_provenance.clone().unwrap();
        assert_eq!(model.model_id, "repository-graph-extract");
        assert_eq!(model.prompt_id, "repository-graph-extract/v1");
        assert_eq!(model.prompt_version, 1);
        assert!((model.temperature - 0.0).abs() < f32::EPSILON);
        assert_eq!(model.call_id.as_deref().map(hex).unwrap(), digest);
        assert_eq!(model.input_tokens, 0);
        assert_eq!(model.output_tokens, 0);
        assert_eq!(model.cache_read_tokens, 0);
        assert_eq!(model.cache_write_tokens, 0);
        assert_eq!(model.cost_microusd, 0);
        let citations = match verified.envelope.payload {
            EventPayload::MemoryMinted(memory) => {
                minted += 1;
                memory.citations.clone()
            }
            EventPayload::EdgeAsserted(edge) => {
                asserted += 1;
                assert!(edge.weight_micros >= 250_000);
                edge.citations.clone()
            }
            _ => panic!("unexpected derived payload"),
        };
        assert_eq!(citations.len(), 1);
        let range = citations[0];
        assert_eq!(range.first_lsn, range.last_lsn);
        let shard = contents
            .get(&range.first_lsn)
            .expect("a citation names a snapshot shard");
        let start = usize::try_from(range.byte_start).unwrap();
        let end = usize::try_from(range.byte_end).unwrap();
        let parsed: serde_json::Value = serde_json::from_slice(&shard[start..end]).unwrap();
        assert!(parsed["kind"].is_string(), "a cited range is a fact line");
    }
    (minted, asserted, closed.expect("the run closes"))
}

#[tokio::test]
async fn repository_snapshot_extracts_nodes_and_edges_without_a_provider() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());

    let remembered = server.remember_envelope(snapshot_input(FIRST)).await;
    assert!(remembered.ok, "{:?}", remembered.items);
    let digest = remembered.items[0]["snapshot_digest"]
        .as_str()
        .unwrap()
        .to_owned();
    let shard_count = remembered.items[0]["shards"].as_u64().unwrap();
    assert!(shard_count > 1, "the fixture should span several shards");

    let run = server.consolidate_envelope(extract_input()).await;
    assert!(run.ok, "{:?}", run.items);
    let item = &run.items[0];
    assert_eq!(item["repository"].as_str().unwrap(), "atlas");
    assert_eq!(item["snapshot_digest"].as_str().unwrap(), digest);
    assert_eq!(item["shards"].as_u64().unwrap(), shard_count);
    assert_eq!(item["facts"].as_u64().unwrap(), 6);
    assert_eq!(item["skipped"].as_u64().unwrap(), 0);
    assert_eq!(item["nodes"].as_u64().unwrap(), 6);
    assert_eq!(item["edges"].as_u64().unwrap(), 4);
    assert_eq!(item["dropped"].as_u64().unwrap(), 0);
    assert_eq!(item["generation"].as_u64().unwrap(), 1);
    assert_eq!(item["parent_generation"].as_u64().unwrap(), 0);
    assert!(!item["duplicate"].as_bool().unwrap());
    assert_eq!(item["cost"]["llm_calls"].as_u64().unwrap(), 0);
    assert_eq!(item["stats"]["derived_records"].as_u64().unwrap(), 10);

    let run_id = expected_run_id(&digest);
    assert_eq!(item["run_id"].as_str().unwrap(), hex(&run_id));

    let (minted, asserted, closed) = derived_frames(&actor, &run_id, &digest).await;
    assert_eq!(minted, 6);
    assert_eq!(asserted, 4);
    assert_eq!(closed.llm_calls, 0);
    assert_eq!(closed.input_tokens, 0);
    assert_eq!(closed.output_tokens, 0);
    assert_eq!(closed.cost_microusd, 0);
    assert_eq!(closed.derived_records, 10);
    assert_eq!(closed.dropped_candidates, 0);

    let neighbourhood = actor
        .graph_neighbourhood(
            node_id("atlas", RepoFactKind::Symbol, "dispatch").to_vec(),
            i64::MAX,
            8,
        )
        .await
        .unwrap();
    assert_eq!(
        neighbourhood.node.as_ref().map(|node| node.name.as_str()),
        Some(node_display_name(RepoFactKind::Symbol, "dispatch").as_str())
    );
    let observed = neighbourhood
        .neighbours
        .iter()
        .map(|neighbour| {
            (
                neighbour.edge.relation.clone(),
                neighbour
                    .endpoint
                    .as_ref()
                    .map(|record| record.name.clone())
                    .unwrap_or_default(),
            )
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        observed,
        [
            ("calls".to_owned(), "file src/store.rs".to_owned()),
            ("contains".to_owned(), "file src/router.rs".to_owned()),
            ("covers".to_owned(), "test dispatches".to_owned()),
            ("routes_to".to_owned(), "route GET /health".to_owned()),
        ]
        .into_iter()
        .collect::<BTreeSet<_>>()
    );
}

#[tokio::test]
async fn re_running_the_same_snapshot_is_a_duplicate() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());

    assert!(server.remember_envelope(snapshot_input(FIRST)).await.ok);
    let first = server.consolidate_envelope(extract_input()).await;
    assert!(first.ok, "{:?}", first.items);
    let settled = actor.stats().await.unwrap().applied.last_lsn;

    let second = server.consolidate_envelope(extract_input()).await;
    assert!(second.ok, "{:?}", second.items);
    assert!(second.items[0]["duplicate"].as_bool().unwrap());
    assert_eq!(
        second.items[0]["generation"].as_u64().unwrap(),
        first.items[0]["generation"].as_u64().unwrap()
    );
    assert_eq!(
        second.items[0]["run_id"].as_str().unwrap(),
        first.items[0]["run_id"].as_str().unwrap()
    );
    assert_eq!(actor.stats().await.unwrap().applied.last_lsn, settled);
}

#[tokio::test]
async fn a_changed_snapshot_revises_nodes_and_adds_edges() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());

    assert!(server.remember_envelope(snapshot_input(FIRST)).await.ok);
    let first = server.consolidate_envelope(extract_input()).await;
    assert!(first.ok, "{:?}", first.items);
    let before = actor
        .memories(64)
        .await
        .unwrap()
        .into_iter()
        .map(|record| (record.memory_id, record.version_lsn))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(before.len(), 6);

    assert!(server.remember_envelope(snapshot_input(SECOND)).await.ok);
    let second = server.consolidate_envelope(extract_input()).await;
    assert!(second.ok, "{:?}", second.items);
    assert!(!second.items[0]["duplicate"].as_bool().unwrap());
    assert_eq!(second.items[0]["generation"].as_u64().unwrap(), 2);
    assert_eq!(second.items[0]["parent_generation"].as_u64().unwrap(), 1);
    assert_eq!(second.items[0]["nodes"].as_u64().unwrap(), 6);
    assert_eq!(second.items[0]["edges"].as_u64().unwrap(), 5);

    let router = node_id("atlas", RepoFactKind::File, "src/router.rs");
    let after = actor.memories(64).await.unwrap();
    assert_eq!(after.len(), 6);
    assert_eq!(
        after
            .iter()
            .map(|record| record.memory_id.clone())
            .collect::<BTreeSet<_>>()
            .len(),
        6
    );
    let revised = after
        .iter()
        .find(|record| record.memory_id == router)
        .expect("the changed node stays visible");
    assert_eq!(revised.previous_lsn, before[router.as_slice()]);
    assert!(revised.version_lsn > revised.previous_lsn);
    assert!(
        String::from_utf8(revised.definition.clone())
            .unwrap()
            .contains("at src/router.rs")
    );

    let neighbourhood = actor
        .graph_neighbourhood(router.to_vec(), i64::MAX, 8)
        .await
        .unwrap();
    let imported = neighbourhood
        .neighbours
        .iter()
        .find(|neighbour| neighbour.edge.relation == "imports")
        .expect("the new relation is visible");
    assert!(imported.outgoing);
    assert_eq!(
        imported
            .endpoint
            .as_ref()
            .map(|record| record.name.as_str())
            .unwrap(),
        "file src/store.rs"
    );
}

#[tokio::test]
async fn missing_and_malformed_snapshots_fail_closed() {
    let empty = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(empty.path())).await.unwrap();
    let server = McpServer::new(actor.clone());
    let before = actor.stats().await.unwrap().applied.last_lsn;
    let refused = server.consolidate_envelope(extract_input()).await;
    assert!(!refused.ok);
    assert_eq!(
        refused.items[0]["error"].as_str().unwrap(),
        "kOperationUnavailable"
    );
    assert_eq!(actor.stats().await.unwrap().applied.last_lsn, before);

    let malformed = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(malformed.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());
    assert!(
        server
            .remember_envelope(snapshot_input(WRONG_CONTRACT))
            .await
            .ok
    );
    let before = actor.stats().await.unwrap().applied.last_lsn;
    let refused = server.consolidate_envelope(extract_input()).await;
    assert!(!refused.ok);
    assert_eq!(
        refused.items[0]["error"].as_str().unwrap(),
        "kSchemaInvalid"
    );
    assert_eq!(actor.stats().await.unwrap().applied.last_lsn, before);
}
