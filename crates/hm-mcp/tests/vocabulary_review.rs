#![forbid(unsafe_code)]

use hm_core::ActorId;
use hm_index::vocabulary::Q16_ONE;
use hm_mcp::{
    InspectInput, McpServer, RememberInput, RememberKind, RetentionInput, VocabularyInput,
};
use hm_proj::store::ProjectionStore;
use hm_proj::vocabulary::VocabularyProjection;
use hm_serve::actor::{ActorConfig, ActorEngine};

const MAP_BYTES: usize = 16 * 1024 * 1024;

const DOCUMENT: &str = concat!(
    "<https://acme.example/crm#works_for> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2002/07/owl#ObjectProperty> .\n",
    "<https://acme.example/crm#works_for> <http://www.w3.org/2004/02/skos/core#altLabel> \"employs\" .\n",
);

fn actor_config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("7"),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: MAP_BYTES,
    }
}

fn import() -> RememberInput {
    RememberInput {
        conversation: "ops".to_owned(),
        content: DOCUMENT.to_owned(),
        kind: RememberKind::Vocabulary,
        chunk_bytes: None,
        anchor: None,
        retention: Some(RetentionInput::Durable),
        sensitivity: None,
        vocabulary: Some(VocabularyInput {
            vocabulary_id: "acme-crm".to_owned(),
            version: 1,
            source_uri: "file:///vocab/acme-crm.nt".to_owned(),
        }),
        source: None,
        derive: None,
    }
}

fn inspect(uri: &str) -> InspectInput {
    InspectInput {
        uri: Some(uri.to_owned()),
        ..InspectInput::default()
    }
}

#[tokio::test]
async fn inspect_lists_imported_vocabularies() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());

    let imported = server.remember_envelope(import()).await;
    assert!(imported.ok, "{:?}", imported.items);
    let first_lsn = imported.items[0]["first_lsn"].as_u64().unwrap();

    let envelope = server.inspect_envelope(inspect("hm://7/vocabulary")).await;
    assert!(envelope.ok, "{:?}", envelope.items);
    let listed = envelope.items[0]["vocabulary"].as_array().unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0]["vocabulary_id"].as_str(), Some("acme-crm"));
    assert_eq!(listed[0]["version"].as_u64(), Some(1));
    assert_eq!(
        listed[0]["source_uri"].as_str(),
        Some("file:///vocab/acme-crm.nt")
    );
    assert_eq!(
        listed[0]["source_media_type"].as_str(),
        Some("application/n-triples")
    );
    assert_eq!(listed[0]["source_digest"].as_str().map(str::len), Some(64));
    assert_eq!(listed[0]["term_count"].as_u64(), Some(1));
    assert_eq!(listed[0]["ignored_triples"].as_u64(), Some(0));
    assert_eq!(listed[0]["event_lsn"].as_u64(), Some(first_lsn));
    assert_eq!(envelope.provenance, vec![format!("hm://7/lsn/{first_lsn}")]);
    assert!(
        envelope
            .warnings
            .iter()
            .any(|warning| warning.contains("importing a new vocabulary version"))
    );

    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn inspect_proposes_aliases_without_merging() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());
    assert!(server.remember_envelope(import()).await.ok);
    let baseline = actor.stats().await.unwrap().applied.last_lsn.get();

    let exact = server
        .inspect_envelope(inspect("hm://7/vocabulary/aliases?name=employs"))
        .await;
    assert!(exact.ok, "{:?}", exact.items);
    let proposals = exact.items[0]["alias_proposals"].as_array().unwrap();
    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0]["vocabulary_id"].as_str(), Some("acme-crm"));
    assert_eq!(proposals[0]["version"].as_u64(), Some(1));
    assert_eq!(
        proposals[0]["term_id"].as_str(),
        Some("https://acme.example/crm#works_for")
    );
    assert_eq!(proposals[0]["canonical_name"].as_str(), Some("works_for"));
    assert_eq!(proposals[0]["category"].as_str(), Some("relation"));
    assert_eq!(proposals[0]["matched_candidate"].as_str(), Some("employs"));
    assert_eq!(
        proposals[0]["similarity_q16"].as_u64(),
        Some(u64::from(Q16_ONE))
    );
    assert_eq!(proposals[0]["accepted"].as_bool(), Some(false));
    assert!(
        exact
            .warnings
            .iter()
            .any(|warning| warning.contains("changes nothing"))
    );

    let near = server
        .inspect_envelope(inspect("hm://7/vocabulary/aliases?name=employ"))
        .await;
    assert!(near.ok, "{:?}", near.items);
    let near_proposals = near.items[0]["alias_proposals"].as_array().unwrap();
    assert_eq!(near_proposals.len(), 1);
    assert!(near_proposals[0]["similarity_q16"].as_u64().unwrap() < u64::from(Q16_ONE));
    assert_eq!(near_proposals[0]["accepted"].as_bool(), Some(false));

    let strict = server
        .inspect_envelope(inspect(
            "hm://7/vocabulary/aliases?name=employ&threshold_q16=65536",
        ))
        .await;
    assert!(strict.ok, "{:?}", strict.items);
    assert!(
        strict.items[0]["alias_proposals"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(
        strict
            .warnings
            .iter()
            .any(|warning| warning.contains("changes nothing"))
    );

    let malformed = server
        .inspect_envelope(inspect("hm://7/vocabulary/aliases?name="))
        .await;
    assert!(!malformed.ok);
    assert_eq!(
        malformed.items[0]["error"].as_str(),
        Some("kInvalidArgument")
    );

    assert_eq!(
        actor.stats().await.unwrap().applied.last_lsn.get(),
        baseline
    );

    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn proposals_never_reach_retrieval() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());
    assert!(server.remember_envelope(import()).await.ok);

    let proposed = server
        .inspect_envelope(inspect("hm://7/vocabulary/aliases?name=employ"))
        .await;
    assert_eq!(
        proposed.items[0]["alias_proposals"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    actor.shutdown().await.unwrap();

    let store = ProjectionStore::open(temporary.path().join("7"), MAP_BYTES).unwrap();
    let snapshot = store.begin_snapshot().unwrap();
    assert!(
        VocabularyProjection::resolve(&snapshot, "employ", 16)
            .unwrap()
            .is_empty()
    );
    let reviewed = VocabularyProjection::resolve(&snapshot, "employs", 16).unwrap();
    assert_eq!(reviewed.len(), 1);
    assert_eq!(reviewed[0].canonical_name, "works_for");
}
