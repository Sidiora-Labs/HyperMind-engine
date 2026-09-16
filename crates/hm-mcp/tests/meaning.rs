#![forbid(unsafe_code)]

use hm_core::{ActorId, ConversationId, LSN};
use hm_embed::{
    EmbedError, Embedder, Embedding, HashFeatureEmbedder, InputRole, SpaceIdentity, quantize,
};
use hm_mcp::{
    AnchorFacet, McpServer, RecallFilters, RecallInput, RecallMode, RememberAnchor, RememberInput,
    RememberKind, RetentionInput, SensitivityInput,
};
use hm_schema::event::{Boundary, EventKind, verify_event};
use hm_schema::events::{Retention, Sensitivity};
use hm_serve::actor::{ActorConfig, ActorEngine, RecallRequest};
use hm_serve::requests::activate::{QueryEmbeddingSource, prepare_query_embedding};
use std::sync::atomic::{AtomicUsize, Ordering};

fn actor_config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("7"),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

#[tokio::test]
async fn anchored_remember_and_entity_near_recall_preserve_policy_fields() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .expect("actor");
    let server = McpServer::new(actor.clone());
    let remembered = server
        .remember_envelope(RememberInput {
            conversation: "meaning".to_owned(),
            content: "Alice Smith changed src/main.rs for GH-123".to_owned(),
            kind: RememberKind::User,
            chunk_bytes: None,
            anchor: Some(RememberAnchor {
                facet: AnchorFacet::Path,
                value: "src/main.rs".to_owned(),
            }),
            retention: Some(RetentionInput::Daily),
            sensitivity: Some(SensitivityInput::Public),
            vocabulary: None,
            source: None,
            derive: None,
            source_delivery: None,
            source_settlement: None,
            document: None,
            source_sync: None,
        })
        .await;
    assert!(remembered.ok);
    assert_eq!(remembered.items[0]["anchor"]["facet"], "path");

    let raw = actor
        .recall(RecallRequest::Timeline {
            conversation: ConversationId::derive("meaning"),
            since_lsn: LSN::new(0),
            limit: 10,
        })
        .await
        .expect("timeline");
    let event = verify_event(&raw[0].payload, EventKind::UserMsg, Boundary::Disk).expect("event");
    assert_eq!(event.envelope.retention, Retention::Daily);
    assert_eq!(event.envelope.sensitivity, Sensitivity::Public);

    for input in [
        RecallInput {
            mode: RecallMode::Entity,
            query: "GH-123".to_owned(),
            conversation: String::new(),
            limit: 10,
            since_lsn: 0,
            filters: RecallFilters::default(),
        },
        RecallInput {
            mode: RecallMode::Near,
            query: String::new(),
            conversation: String::new(),
            limit: 10,
            since_lsn: 0,
            filters: RecallFilters {
                anchor: Some("src/main.rs".to_owned()),
                ..RecallFilters::default()
            },
        },
    ] {
        let recalled = server.recall_envelope(input).await;
        assert!(recalled.ok);
        assert_eq!(recalled.items[0]["lsn"], 1);
    }

    let before = actor.stats().await.expect("stats").log_events;
    let discarded = server
        .remember_envelope(RememberInput {
            conversation: "meaning".to_owned(),
            content: "never persist this".to_owned(),
            kind: RememberKind::User,
            chunk_bytes: None,
            anchor: None,
            retention: Some(RetentionInput::DoNotStore),
            sensitivity: Some(SensitivityInput::Secret),
            vocabulary: None,
            source: None,
            derive: None,
            source_delivery: None,
            source_settlement: None,
            document: None,
            source_sync: None,
        })
        .await;
    assert!(discarded.ok);
    assert_eq!(discarded.items[0]["stored"], false);
    assert!(discarded.provenance.is_empty());
    assert_eq!(actor.stats().await.expect("stats").log_events, before);
    actor.shutdown().await.expect("shutdown");
}

#[test]
fn activation_embeds_one_query_or_accepts_a_precomputed_query_vector() {
    let embedder = CountingEmbedder {
        inner: HashFeatureEmbedder::new(16).expect("embedder"),
        queries: AtomicUsize::new(0),
        documents: AtomicUsize::new(0),
    };
    let prepared = prepare_query_embedding(Some(&embedder), "meaning query", None)
        .expect("configured embedding");
    assert_eq!(prepared.source, QueryEmbeddingSource::ConfiguredEmbedder);
    assert_eq!(embedder.queries.load(Ordering::Relaxed), 1);
    assert_eq!(embedder.documents.load(Ordering::Relaxed), 0);

    let embedding = embedder.inner.embed_query("precomputed").expect("query");
    let quantized = quantize(&embedding).expect("quantized");
    let precomputed =
        prepare_query_embedding::<CountingEmbedder>(None, "must not be embedded", Some(quantized))
            .expect("precomputed embedding");
    assert_eq!(precomputed.source, QueryEmbeddingSource::Precomputed);
    assert_eq!(embedder.queries.load(Ordering::Relaxed), 1);
    assert_eq!(embedder.documents.load(Ordering::Relaxed), 0);
}

struct CountingEmbedder {
    inner: HashFeatureEmbedder,
    queries: AtomicUsize,
    documents: AtomicUsize,
}

impl Embedder for CountingEmbedder {
    fn identity(&self, role: InputRole) -> SpaceIdentity {
        self.inner.identity(role)
    }

    fn embed_query(&self, query: &str) -> Result<Embedding, EmbedError> {
        self.queries.fetch_add(1, Ordering::Relaxed);
        self.inner.embed_query(query)
    }

    fn embed_documents(&self, documents: &[&str]) -> Result<Vec<Embedding>, EmbedError> {
        self.documents.fetch_add(1, Ordering::Relaxed);
        self.inner.embed_documents(documents)
    }
}
