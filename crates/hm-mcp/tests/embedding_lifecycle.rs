use hm_core::{ActorId, ConversationId, LSN};
use hm_mcp::{
    EmbeddingRuntime, McpServer, RecallFilters, RecallInput, RecallMode, RememberInput,
    RememberKind, RetentionInput,
};
use hm_schema::event::{Boundary, EventKind, verify_event};
use hm_schema::events::EventPayload;
use hm_serve::actor::{ActorConfig, ActorEngine, RecallRequest};

fn actor_config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("7"),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

fn remember(content: &str) -> RememberInput {
    RememberInput {
        conversation: "embedding-lifecycle".to_owned(),
        content: content.to_owned(),
        kind: RememberKind::Document,
        chunk_bytes: None,
        anchor: None,
        retention: None,
        sensitivity: None,
    }
}

fn recall(query: &str) -> RecallInput {
    RecallInput {
        mode: RecallMode::Semantic,
        query: query.to_owned(),
        conversation: String::new(),
        limit: 2,
        since_lsn: 0,
        filters: RecallFilters::default(),
    }
}

#[tokio::test]
async fn unconfigured_embeddings_are_unavailable_and_reconstruction_is_rejected() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());
    let rejection = server
        .remember_envelope(remember("RECONSTRUCTION\nUnverified narrative"))
        .await;
    assert!(!rejection.ok);
    assert_eq!(actor.stats().await.unwrap().log_events, 0);
    let observed = server
        .remember_envelope(remember("A durable observation"))
        .await;
    assert!(observed.ok);
    assert_eq!(observed.health["encoder"], "lexical_only");
    let response = server.recall_envelope(recall("observation")).await;
    assert!(!response.ok);
    assert_eq!(response.items[0]["error"], "kOperationUnavailable");
    actor.shutdown().await.unwrap();
}

#[test]
#[ignore = "requires HM_EMBEDDING_PROVIDER=centra and CENTRA_GATEWAY_API_KEY; incurs four live embedding calls"]
fn centra_remember_recall_restart_uses_ledger_backed_vectors() {
    let runtime = EmbeddingRuntime::from_env()
        .expect("valid opt-in provider configuration")
        .expect("HM_EMBEDDING_PROVIDER=centra is required");
    let executor = tokio::runtime::Runtime::new().unwrap();
    executor.block_on(async move {
        let temporary = tempfile::tempdir().unwrap();
        let config = actor_config(temporary.path());
        let actor = ActorEngine::open(config.clone()).await.unwrap();
        let server = McpServer::new(actor.clone()).with_embedding_runtime(runtime.clone());
        let content = "The durable Atlas database backup is stored in the Berlin vault.";
        let first = server.remember_envelope(remember(content)).await;
        assert!(first.ok, "{first:?}");
        assert!(first.gaps.is_empty(), "{first:?}");
        assert_eq!(first.health["encoder"], "semantic_ready");
        assert_eq!(first.items[0]["embedding_first_lsn"], 2);
        let second = server
            .remember_envelope(remember(
                "Ripe oranges and bananas are stored in the kitchen.",
            ))
            .await;
        assert!(second.ok && second.gaps.is_empty(), "{second:?}");
        let mut discarded = remember("Do not persist or transmit this observation.");
        discarded.retention = Some(RetentionInput::DoNotStore);
        assert!(server.remember_envelope(discarded).await.ok);
        assert_eq!(actor.stats().await.unwrap().log_events, 4);

        let raw = actor
            .recall(RecallRequest::Timeline {
                conversation: ConversationId::derive("embedding-lifecycle"),
                since_lsn: LSN::new(0),
                limit: 10,
            })
            .await
            .unwrap();
        let vectors = raw
            .iter()
            .filter(|item| item.kind == hm_ledger::frame::EventKind::Embedding)
            .collect::<Vec<_>>();
        assert_eq!(vectors.len(), 2);
        let event =
            verify_event(&vectors[0].payload, EventKind::Embedding, Boundary::Disk).unwrap();
        let EventPayload::Embedding(embedding) = event.envelope.payload else {
            panic!("embedding event")
        };
        assert_eq!(embedding.target_lsn, 1);
        assert_eq!(embedding.dimension, 3072);
        assert_eq!(embedding.quantized.len(), 3072);
        assert_eq!(embedding.binary_prefilter.len(), 384);
        assert!(event.envelope.model_provenance.is_some());

        let found = server
            .recall_envelope(recall("In which city is the database recovery copy kept?"))
            .await;
        assert!(found.ok, "{found:?}");
        assert_eq!(found.items[0]["lsn"], 1);
        assert_eq!(found.items[0]["content"], content);
        assert_eq!(actor.stats().await.unwrap().log_events, 4);
        drop(server);
        actor.shutdown().await.unwrap();

        let actor = ActorEngine::open(config).await.unwrap();
        let server = McpServer::new(actor.clone()).with_embedding_runtime(runtime);
        let found = server
            .recall_envelope(recall(
                "Where does Atlas keep its disaster recovery backup?",
            ))
            .await;
        assert!(found.ok, "{found:?}");
        assert_eq!(found.items[0]["lsn"], 1);
        assert_eq!(found.items[0]["content"], content);
        assert_eq!(actor.stats().await.unwrap().log_events, 4);
        actor.shutdown().await.unwrap();
    });
}
