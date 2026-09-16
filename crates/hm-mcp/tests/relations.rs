use hm_core::{ActorId, ConversationId};
use hm_embed::HashFeatureEmbedder;
use hm_ledger::frame::EventKind as LedgerEventKind;
use hm_mcp::{
    ConsolidateAction, ConsolidateBudget, ConsolidateInput, ConsolidateMode, EmbeddingRuntime,
    McpServer, RememberInput, RememberKind,
};
use hm_schema::event::{
    Boundary, CURRENT_SCHEMA_VERSION, EventKind, encode_event_envelope, verify_event,
};
use hm_schema::events::{
    Authority, ConsolidationBudget, ConsolidationClosed, ConsolidationOpened,
    ConsolidationPhaseName, EdgeAsserted, EventEnvelope, EventPayload, ModelProvenance,
    PromptVersion, ProvenanceRange, Retention, Sensitivity,
};
use hm_serve::actor::{ActorConfig, ActorEngine, IncomingEvent};
use std::sync::Arc;

const RUN: &[u8] = b"relations/seed";
const SCOPE: &str = "relations";

fn actor_config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("11"),
        actor: ActorId::new(11),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

fn runtime() -> EmbeddingRuntime {
    EmbeddingRuntime::new(Arc::new(HashFeatureEmbedder::new(64).unwrap()))
}

fn remember(content: &str) -> RememberInput {
    RememberInput {
        conversation: "relations".to_owned(),
        content: content.to_owned(),
        kind: RememberKind::Document,
        chunk_bytes: None,
        anchor: None,
        retention: None,
        sensitivity: None,
        vocabulary: None,
    }
}

fn consolidate(cadence_key: &str) -> ConsolidateInput {
    ConsolidateInput {
        action: ConsolidateAction::Run,
        mode: Some(ConsolidateMode::Nrem),
        scope: Some(SCOPE.to_owned()),
        cadence_key: Some(cadence_key.to_owned()),
        budget: Some(ConsolidateBudget {
            max_llm_calls: 1,
            max_tokens: 1_000,
            max_microusd: 1_000,
            max_wall_ms: 1_000,
        }),
        run_id: None,
        reason: None,
    }
}

fn provenance() -> ModelProvenance {
    ModelProvenance {
        model_id: "relation-seed".to_owned(),
        prompt_id: "relation-seed/v1".to_owned(),
        prompt_version: 1,
        temperature: 0.0,
        call_id: Some(vec![7; 32]),
        input_tokens: 4,
        output_tokens: 2,
        cache_read_tokens: 0,
        cache_write_tokens: 0,
        cost_microusd: 0,
    }
}

fn seed_event(
    kind: LedgerEventKind,
    payload: EventPayload,
    model_provenance: Option<ModelProvenance>,
) -> IncomingEvent {
    IncomingEvent {
        kind,
        conversation: ConversationId::derive("relations"),
        payload: encode_event_envelope(&EventEnvelope {
            schema_version: CURRENT_SCHEMA_VERSION,
            payload,
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 0,
            origin_actor: 0,
            run_id: Some(RUN.to_vec()),
            model_provenance: model_provenance.map(Box::new),
            authority: Authority::DerivedInference,
            retention: Retention::Durable,
            sensitivity: Sensitivity::Personal,
            event_time_ns: 0,
        }),
    }
}

fn edge(
    edge_id: &[u8],
    source: &[u8],
    relation: &str,
    target: &[u8],
    support: u64,
) -> IncomingEvent {
    seed_event(
        LedgerEventKind::EdgeAsserted,
        EventPayload::EdgeAsserted(Box::new(EdgeAsserted {
            edge_id: edge_id.to_vec(),
            source_id: source.to_vec(),
            target_id: target.to_vec(),
            relation: relation.to_owned(),
            weight_micros: 400_000,
            valid_from_ns: 0,
            valid_to_ns: 0,
            citations: vec![ProvenanceRange {
                first_lsn: support,
                last_lsn: support,
                byte_start: 0,
                byte_end: 16,
            }],
        })),
        Some(provenance()),
    )
}

fn seed_generation(first_support: u64, second_support: u64) -> Vec<IncomingEvent> {
    vec![
        seed_event(
            LedgerEventKind::ConsolidationOpened,
            EventPayload::ConsolidationOpened(Box::new(ConsolidationOpened {
                scope_digest: vec![5; 32],
                cadence_key: "relations:seed".to_owned(),
                generation: 1,
                expected_active_generation: 0,
                phases: vec![ConsolidationPhaseName::Publish],
                prompts: vec![PromptVersion {
                    prompt_id: "relation-seed/v1".to_owned(),
                    version: 1,
                    model_id: "relation-seed".to_owned(),
                }],
                budget: Box::new(ConsolidationBudget {
                    max_llm_calls: 1,
                    max_tokens: 1_000,
                    max_microusd: 1_000,
                    max_wall_ms: 1_000,
                }),
                source_first_lsn: 0,
                source_last_lsn: 0,
            })),
            None,
        ),
        edge(
            b"edge-vault-holds-backup",
            b"berlin vault",
            "holds",
            b"atlas backup",
            first_support,
        ),
        edge(
            b"edge-backup-covers-ledger",
            b"atlas backup",
            "covers",
            b"atlas ledger",
            second_support,
        ),
        seed_event(
            LedgerEventKind::ConsolidationClosed,
            EventPayload::ConsolidationClosed(Box::new(ConsolidationClosed {
                generation: 1,
                expected_active_generation: 0,
                derived_records: 2,
                dropped_candidates: 0,
                llm_calls: 0,
                input_tokens: 0,
                output_tokens: 0,
                cost_microusd: 0,
            })),
            None,
        ),
    ]
}

struct Vector {
    lsn: u64,
    target_lsn: u64,
    space_id: String,
    authority: Authority,
    prompt_id: String,
}

async fn vectors(actor: &ActorEngine) -> Vec<Vector> {
    let mut output = Vec::new();
    for frame in actor
        .frames_since(hm_core::LSN::new(0), None, usize::MAX)
        .await
        .unwrap()
    {
        if frame.header.kind != LedgerEventKind::Embedding {
            continue;
        }
        let verified =
            verify_event(&frame.sealed_payload, EventKind::Embedding, Boundary::Disk).unwrap();
        let prompt_id = verified
            .envelope
            .model_provenance
            .as_ref()
            .map(|provenance| provenance.prompt_id.clone())
            .unwrap_or_default();
        let authority = verified.envelope.authority;
        let EventPayload::Embedding(embedding) = verified.envelope.payload else {
            panic!("embedding payload")
        };
        output.push(Vector {
            lsn: frame.header.lsn.get(),
            target_lsn: embedding.target_lsn,
            space_id: embedding.space_id,
            authority,
            prompt_id,
        });
    }
    output
}

async fn edge_lsns(actor: &ActorEngine) -> Vec<u64> {
    actor
        .frames_since(hm_core::LSN::new(0), None, usize::MAX)
        .await
        .unwrap()
        .iter()
        .filter(|frame| frame.header.kind == LedgerEventKind::EdgeAsserted)
        .map(|frame| frame.header.lsn.get())
        .collect()
}

#[tokio::test]
async fn relationship_embeddings_are_built_in_the_background_with_edge_and_source_provenance() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone()).with_embedding_runtime(runtime());
    let first = server
        .remember_envelope(remember("The Berlin vault holds the atlas backup."))
        .await;
    assert!(first.ok, "{first:?}");
    let second = server
        .remember_envelope(remember("The atlas backup covers the atlas ledger."))
        .await;
    assert!(second.ok, "{second:?}");
    let first_support = first.items[0]["first_lsn"].as_u64().unwrap();
    let second_support = second.items[0]["first_lsn"].as_u64().unwrap();
    actor
        .append(seed_generation(first_support, second_support))
        .await
        .unwrap();
    let asserted = edge_lsns(&actor).await;
    assert_eq!(asserted.len(), 2);
    let document_space = vectors(&actor).await[0].space_id.clone();

    let envelope = server.consolidate_envelope(consolidate("first")).await;
    assert!(envelope.ok, "{envelope:?}");
    assert!(envelope.gaps.is_empty(), "{envelope:?}");
    assert_eq!(envelope.items[0]["relations_embedded"], 2);
    let relation_space = envelope.items[0]["relation_space"].as_str().unwrap();
    assert_ne!(relation_space, document_space);

    let relation_vectors = vectors(&actor)
        .await
        .into_iter()
        .filter(|vector| vector.space_id == relation_space)
        .collect::<Vec<_>>();
    assert_eq!(relation_vectors.len(), 2);
    let mut targets = relation_vectors
        .iter()
        .map(|vector| vector.target_lsn)
        .collect::<Vec<_>>();
    targets.sort_unstable();
    assert_eq!(targets, asserted);
    for vector in &relation_vectors {
        assert!(vector.target_lsn < vector.lsn);
        assert_eq!(vector.authority, Authority::DerivedInference);
        assert_eq!(vector.prompt_id, "embedding/relation");
    }
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_second_background_pass_embeds_no_relationship_twice() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone()).with_embedding_runtime(runtime());
    let first = server
        .remember_envelope(remember("The Berlin vault holds the atlas backup."))
        .await;
    let second = server
        .remember_envelope(remember("The atlas backup covers the atlas ledger."))
        .await;
    actor
        .append(seed_generation(
            first.items[0]["first_lsn"].as_u64().unwrap(),
            second.items[0]["first_lsn"].as_u64().unwrap(),
        ))
        .await
        .unwrap();

    let opening = server.consolidate_envelope(consolidate("first")).await;
    assert_eq!(opening.items[0]["relations_embedded"], 2);
    let before = vectors(&actor).await.len();

    let repeated = server.consolidate_envelope(consolidate("second")).await;
    assert!(repeated.ok, "{repeated:?}");
    assert_eq!(repeated.items[0]["relations_embedded"], 0);
    assert_eq!(
        repeated.items[0]["relation_space"],
        opening.items[0]["relation_space"]
    );
    assert_eq!(vectors(&actor).await.len(), before);
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn without_an_embedding_runtime_a_published_run_stays_lexical_only() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());
    let first = server
        .remember_envelope(remember("The Berlin vault holds the atlas backup."))
        .await;
    let second = server
        .remember_envelope(remember("The atlas backup covers the atlas ledger."))
        .await;
    actor
        .append(seed_generation(
            first.items[0]["first_lsn"].as_u64().unwrap(),
            second.items[0]["first_lsn"].as_u64().unwrap(),
        ))
        .await
        .unwrap();

    let envelope = server.consolidate_envelope(consolidate("first")).await;
    assert!(envelope.ok, "{envelope:?}");
    assert!(envelope.items[0].get("relations_embedded").is_none());
    assert!(envelope.items[0].get("relation_space").is_none());
    assert!(vectors(&actor).await.is_empty());
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn an_empty_relationship_limit_is_rejected() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let rejection = actor.relations(0).await.unwrap_err();
    assert_eq!(rejection.code, hm_core::ErrorCode::InvalidArgument);
    assert!(actor.relations(16).await.unwrap().is_empty());
    actor.shutdown().await.unwrap();
}
