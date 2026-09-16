#![allow(clippy::cast_possible_truncation)]

use hm_core::{ActorId, ConversationId, ErrorCode, LSN};
use hm_ledger::frame::EventKind;
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    Authority, Embedding, EventEnvelope, EventPayload, Retention, Sensitivity, UserMsg,
};
use hm_serve::actor::{ActorConfig, ActorEngine, IncomingEvent, RecallRequest};

#[tokio::test]
async fn committed_embeddings_recall_and_replay_without_duplicate_vectors() {
    let directory = tempfile::tempdir().unwrap();
    let config = ActorConfig {
        actor_directory: directory.path().join("actor"),
        actor: ActorId::new(1),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    };
    let engine = ActorEngine::open(config.clone()).await.unwrap();
    engine
        .append(vec![message("violet orchid"), message("copper wire")])
        .await
        .unwrap();
    let first = embedding(1, "space-a", &[60, 3, 0, 0]);
    engine
        .append(vec![first.clone(), embedding(2, "space-a", &[0, 0, 3, 60])])
        .await
        .unwrap();
    engine.append(vec![first]).await.unwrap();
    let query = || RecallRequest::Vector {
        space_id: "space-a".into(),
        query: vec![60, 2, 0, 0],
        binary_prefilter: vec![0b1111],
        limit: 10,
    };
    let hits = engine.recall(query()).await.unwrap();
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].lsn, LSN::new(1));
    assert_eq!(
        engine
            .stats()
            .await
            .unwrap()
            .projections
            .iter()
            .find(|stat| stat.name == "vector_lane")
            .unwrap()
            .applied_lsn,
        LSN::new(5)
    );
    assert_eq!(
        engine
            .append(vec![embedding(1, "space-a", &[0, 0, 60, 0])])
            .await
            .unwrap_err()
            .code,
        ErrorCode::InvalidArgument
    );
    assert_eq!(
        engine
            .append(vec![embedding(2, "space-a", &[0, 60])])
            .await
            .unwrap_err()
            .code,
        ErrorCode::InvalidArgument
    );
    assert_eq!(
        engine
            .append(vec![embedding(99, "space-a", &[0, 60, 0, 0])])
            .await
            .unwrap_err()
            .code,
        ErrorCode::InvalidArgument
    );
    let before = engine.stats().await.unwrap();
    engine.shutdown().await.unwrap();
    let vector_path = config
        .actor_directory
        .join("vectors")
        .join(format!("{}.hmvec", blake3::hash(b"space-a").to_hex()));
    let canonical = std::fs::read(&vector_path).unwrap();
    let reopened = ActorEngine::open(config.clone()).await.unwrap();
    assert_eq!(reopened.recall(query()).await.unwrap(), hits);
    assert_eq!(reopened.stats().await.unwrap().applied, before.applied);
    assert_eq!(std::fs::read(&vector_path).unwrap(), canonical);
    reopened.shutdown().await.unwrap();
    std::fs::rename(&vector_path, vector_path.with_extension("saved"))
        .expect("simulate missing rebuildable vector projection");
    let rebuilt = ActorEngine::open(config).await.unwrap();
    assert_eq!(rebuilt.recall(query()).await.unwrap(), hits);
    assert_eq!(std::fs::read(vector_path).unwrap(), canonical);
    assert_eq!(
        rebuilt
            .recall(RecallRequest::Vector {
                space_id: "unknown-space".into(),
                query: vec![60, 2, 0, 0],
                binary_prefilter: vec![0b1111],
                limit: 10,
            })
            .await
            .unwrap_err()
            .code,
        ErrorCode::OperationUnavailable
    );
    rebuilt.shutdown().await.unwrap();
}

fn message(content: &str) -> IncomingEvent {
    incoming(
        EventKind::UserMsg,
        EventPayload::UserMsg(Box::new(UserMsg {
            content: content.as_bytes().to_vec(),
        })),
        Authority::UserAsserted,
    )
}

fn embedding(target_lsn: u64, space_id: &str, quantized: &[i8]) -> IncomingEvent {
    let mut binary_prefilter = vec![0; quantized.len().div_ceil(8)];
    for (index, value) in quantized.iter().enumerate() {
        if *value >= 0 {
            binary_prefilter[index / 8] |= 1 << (index % 8);
        }
    }
    incoming(
        EventKind::Embedding,
        EventPayload::Embedding(Box::new(Embedding {
            target_lsn,
            dimension: quantized.len() as u32,
            quantized: quantized.to_vec(),
            binary_prefilter,
            space_id: space_id.into(),
        })),
        Authority::DerivedInference,
    )
}

fn incoming(kind: EventKind, payload: EventPayload, authority: Authority) -> IncomingEvent {
    IncomingEvent {
        kind,
        conversation: ConversationId::derive("vector-life-cycle"),
        payload: encode_event_envelope(&EventEnvelope {
            schema_version: CURRENT_SCHEMA_VERSION,
            payload,
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 0,
            origin_actor: 0,
            run_id: None,
            model_provenance: None,
            authority,
            retention: Retention::Durable,
            sensitivity: Sensitivity::Personal,
            event_time_ns: 0,
        }),
    }
}
