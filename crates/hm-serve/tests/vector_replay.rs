use hm_context::{
    Authority, Scope, SourceSpan, digest_bytes,
    retrieval::{EmbeddedVector, RetrievalRequest, SemanticStatus, SourceKind},
};
use hm_core::{ActorId, ConversationId, LSN};
use hm_embed::{HttpFetcher, ModelKind, ModelStore, OnnxEmbedder};
use hm_schema::{
    event::{self, CURRENT_SCHEMA_VERSION},
    events::{EventEnvelope, EventPayload, ProviderFrame, Retention, Sensitivity},
};
use hm_serve::{
    actor::{ActorConfig, ActorEngine, IncomingEvent},
    context_retrieval::{self, EmbeddingMode, EmbeddingProvider, RETRIEVAL_PROVIDER, SourceRecord},
};
use std::sync::Arc;
fn scope() -> Scope {
    Scope {
        owner_id: "owner".into(),
        project_id: "vector-replay".into(),
        workspace_id: None,
    }
}
fn config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.into(),
        actor: ActorId::new(42),
        user: [9; 16],
        kek: [8; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}
async fn persisted(actor: &ActorEngine) -> Vec<u8> {
    for frame in actor.frames_since(LSN::new(0), None, 100).await.unwrap() {
        if let EventPayload::ProviderFrame(provider) = actor
            .verified_event(frame.header.lsn)
            .await
            .unwrap()
            .envelope
            .payload
        {
            if provider.provider == RETRIEVAL_PROVIDER {
                let value: serde_json::Value =
                    serde_json::from_slice(&provider.api_content).unwrap();
                if value["operation"]["operation"] == "backfill" {
                    return provider.api_content;
                }
            }
        }
    }
    panic!("native backfill frame missing")
}
#[tokio::test]
async fn actual_bge_float_vectors_replay_exactly_with_decimal_precision_and_restart() {
    let exact = "0.000000000000000000123456789123456789123456789123456789";
    let number: serde_json::Number = exact.parse().unwrap();
    assert_eq!(number.to_string(), exact);
    let directory = std::env::var("HM_RETRIEVAL_MODEL_DIRECTORY")
        .expect("verified actual model directory required");
    let embedder = tokio::task::spawn_blocking(move || {
        OnnxEmbedder::download(
            ModelKind::BgeSmallEnV15,
            &ModelStore::new(directory),
            &HttpFetcher,
        )
    })
    .await
    .unwrap()
    .unwrap();
    let provider =
        EmbeddingProvider::new("replay-bge", 1, EmbeddingMode::Local, Arc::new(embedder)).unwrap();
    assert_eq!(
        provider
            .registration
            .fingerprint
            .as_ref()
            .unwrap()
            .dimensions,
        384
    );
    let dir = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(dir.path())).await.unwrap();
    let text = "The database recovery backup is stored in the Paris vault.";
    let digest = digest_bytes(text.as_bytes());
    let span = SourceSpan {
        source_id: "backup-document".into(),
        source_digest: digest.clone(),
        byte_start: 0,
        byte_end: text.len() as u64,
    };
    context_retrieval::register_source(
        &actor,
        &scope(),
        0,
        SourceRecord {
            scope: scope(),
            kind: SourceKind::Document,
            id: "backup-document".into(),
            revision: 1,
            text: text.into(),
            content_digest: digest.clone(),
            authority: Authority::ExternalObserved,
            provenance: vec![span.clone()],
            occurred_at_ns: None,
            recorded_at_ns: 1791288000123456789,
            expires_at_ns: None,
            tombstoned: false,
        },
    )
    .await
    .unwrap();
    context_retrieval::register_embedding(&actor, &scope(), 0, provider.registration.clone())
        .await
        .unwrap();
    let backfill = context_retrieval::backfill(
        &actor,
        &scope(),
        Some(&provider),
        4,
        16384,
        1791288000123456790,
    )
    .await
    .unwrap();
    assert_eq!(backfill.embedded, 1);
    assert!(backfill.unavailable.is_none());
    let bytes = persisted(&actor).await;
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let vector: EmbeddedVector =
        serde_json::from_value(value["operation"]["vectors"][0]["vector"].clone()).unwrap();
    vector.validate().unwrap();
    assert_eq!(vector.values.len(), 384);
    assert_eq!(vector.content_digest, digest);
    assert!(vector.values.iter().any(|v| v.fract() != 0.0));
    let bits: Vec<_> = vector.values.iter().map(|v| v.to_bits()).collect();
    let inspected = context_retrieval::inspect(&actor, &scope()).await.unwrap();
    assert_eq!(inspected["stored_vector_count"], 1);
    assert_eq!(inspected["vector_count"], 1);
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(dir.path())).await.unwrap();
    let replay = context_retrieval::inspect(&actor, &scope()).await.unwrap();
    assert_eq!(replay["vector_count"], 1);
    assert_eq!(persisted(&actor).await, bytes);
    let restored: serde_json::Value = serde_json::from_slice(&persisted(&actor).await).unwrap();
    let restored: EmbeddedVector =
        serde_json::from_value(restored["operation"]["vectors"][0]["vector"].clone()).unwrap();
    assert_eq!(
        restored
            .values
            .iter()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>(),
        bits
    );
    assert_eq!(restored, vector);
    let before = actor.stats().await.unwrap().log_events;
    let retry = context_retrieval::backfill(
        &actor,
        &scope(),
        Some(&provider),
        4,
        16384,
        1791288000123456790,
    )
    .await
    .unwrap();
    assert_eq!(retry.embedded, 0);
    assert_eq!(actor.stats().await.unwrap().log_events, before);
    let tokenizer =
        hm_compose::tokens::TokenCounter::for_model("gpt-4o", None, Default::default()).unwrap();
    let request = RetrievalRequest {
        scope: scope(),
        now_ns: 1791288000123456790,
        grants: vec![],
        visible: vec![],
        time_filter: None,
        query_vector: None,
        max_candidates: 8,
        max_results: 2,
        max_tokens: 256,
    };
    let retrieved = context_retrieval::retrieve(
        &actor,
        &scope(),
        "Where is the recovery backup stored?",
        &request,
        &tokenizer,
        Some(&provider),
    )
    .await
    .unwrap();
    assert!(matches!(
        retrieved.semantic,
        SemanticStatus::Available { scored: 1, .. }
    ));
    assert_eq!(retrieved.results[0].candidate.id, "backup-document");
    assert!(retrieved.results[0].semantic_score.unwrap().is_finite());
    let mut malformed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    malformed["operation"]["vectors"][0]["vector"]["fingerprint"]["dimensions"] =
        serde_json::json!(385);
    let content = serde_json::to_vec(&malformed).unwrap();
    let envelope = EventEnvelope {
        schema_version: CURRENT_SCHEMA_VERSION,
        payload: EventPayload::ProviderFrame(Box::new(ProviderFrame {
            provider: RETRIEVAL_PROVIDER.into(),
            api_content: content,
        })),
        connection_id: None,
        client_seq: 0,
        client_event_index: 0,
        client_event_count: 1,
        origin_actor: 0,
        run_id: None,
        model_provenance: None,
        authority: hm_schema::events::Authority::RuntimeFact,
        retention: Retention::Durable,
        sensitivity: Sensitivity::Personal,
        event_time_ns: 0,
    };
    actor
        .append(vec![IncomingEvent {
            kind: hm_ledger::frame::EventKind::ProviderFrame,
            conversation: ConversationId::derive(&format!(
                "context-retrieval:{}",
                scope().digest().unwrap()
            )),
            payload: event::encode_event_envelope(&envelope),
        }])
        .await
        .unwrap();
    assert!(
        matches!(context_retrieval::inspect(&actor, &scope()).await, Err(context_retrieval::RetrievalError::Context(hm_context::ContextError::Invalid(message))) if message == "invalid embedding vector")
    );
    actor.shutdown().await.unwrap();
}
