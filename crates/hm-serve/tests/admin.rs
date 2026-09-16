#![forbid(unsafe_code)]
#![allow(clippy::similar_names, clippy::too_many_lines)]

use hm_compose::tokens::FallbackWeights;
use hm_core::{ActorId, ConversationId, ErrorCode, LSN};
use hm_ledger::checkpoint::load_checkpoint;
use hm_ledger::frame::EventKind;
use hm_ledger::shred::{decode_deletion_receipt, verify_deletion_receipt};
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{Authority, EventEnvelope, EventPayload, Retention, Sensitivity, UserMsg};
use hm_schema::protocol::{encode_wire_envelope, verify_wire_envelope};
use hm_schema::wire::{
    Append, AppendEvent, CryptoDelete, Hello, LatencyHistograms, MutationEffectState,
    RebuildProjection, Request, RequestPayload, ResponsePayload, Stats, VerifyStatus, WireEnvelope,
    WirePayload,
};
use hm_serve::actor::{ActivateRequest, ActorConfig, ActorEngine, IncomingEvent};
use hm_serve::admin;
use hm_serve::config::{ActorCapability, ServerConfig};
use hm_serve::protocol::{FrameParser, encode_frame};
use hm_serve::uds::UdsServer;
use std::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

fn config(path: &std::path::Path) -> ServerConfig {
    ServerConfig {
        socket_path: path.join("hypermind.sock"),
        data_directory: path.join("data"),
        user: [1; 16],
        kek: [2; 32],
        admin_token: [3; 32],
        actors: vec![ActorCapability {
            actor: 7,
            token: [4; 32],
        }],
        maximum_connections: 4,
        maximum_output_frames: 16,
        maximum_output_bytes: 1024 * 1024,
        projection_map_bytes: 16 * 1024 * 1024,
        maximum_active_actors: 64,
        maximum_heavy_jobs: 2,
        lease_wait_ms: 250,
    }
}

fn message(conversation: ConversationId) -> IncomingEvent {
    IncomingEvent {
        kind: EventKind::UserMsg,
        conversation,
        payload: encode_event_envelope(&EventEnvelope {
            schema_version: CURRENT_SCHEMA_VERSION,
            payload: EventPayload::UserMsg(Box::new(UserMsg {
                content: b"admin trust path".to_vec(),
            })),
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 0,
            origin_actor: 0,
            run_id: None,
            model_provenance: None,
            authority: Authority::UserAsserted,
            retention: Retention::Durable,
            sensitivity: Sensitivity::Personal,
            event_time_ns: 0,
        }),
    }
}

async fn exchange(stream: &mut UnixStream, payload: WirePayload) -> WireEnvelope {
    let framed = encode_frame(&encode_wire_envelope(&WireEnvelope {
        proto_version: 3,
        payload,
    }))
    .unwrap();
    stream.write_all(&framed).await.unwrap();
    let mut header = [0; 8];
    stream.read_exact(&mut header).await.unwrap();
    let length = u32::from_le_bytes(header[..4].try_into().unwrap()) as usize;
    let mut payload = vec![0; length];
    stream.read_exact(&mut payload).await.unwrap();
    let mut parser = FrameParser::default();
    let frames = parser
        .push(&[header.as_slice(), payload.as_slice()].concat())
        .unwrap();
    verify_wire_envelope(&frames[0]).unwrap()
}

async fn connect(socket: &std::path::Path, token: [u8; 32]) -> UnixStream {
    let mut stream = UnixStream::connect(socket).await.unwrap();
    let welcome = exchange(
        &mut stream,
        WirePayload::Hello(Box::new(Hello {
            proto_version: 3,
            connection_id: vec![9; 16],
            capability_token: token.to_vec(),
        })),
    )
    .await;
    assert!(matches!(welcome.payload, WirePayload::Welcome(_)));
    stream
}

fn request(request_id: u64, payload: RequestPayload) -> WirePayload {
    WirePayload::Request(Box::new(Request {
        request_id,
        payload,
    }))
}

#[tokio::test]
async fn admin_and_actor_capabilities_never_mix_and_admin_operations_are_real() {
    let temporary = tempfile::tempdir().unwrap();
    let config = config(temporary.path());
    let socket = config.socket_path.clone();
    let actor_directory = config.actor_directory(7);
    let server = UdsServer::bind(config).await.unwrap();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(server.serve_until(async {
        let _ = shutdown_rx.await;
    }));

    let conversation = ConversationId::derive("admin-test");
    let payload = message(conversation).payload;
    let mut actor = connect(&socket, [4; 32]).await;
    let appended = exchange(
        &mut actor,
        request(
            1,
            RequestPayload::Append(Box::new(Append {
                client_seq: 1,
                events: vec![AppendEvent {
                    kind: EventKind::UserMsg as u8,
                    conversation: conversation.into_bytes().to_vec(),
                    payload: payload.clone(),
                }],
            })),
        ),
    )
    .await;
    assert!(matches!(
        appended.payload,
        WirePayload::Response(response)
            if matches!(response.payload, Some(ResponsePayload::AppendAck(_)))
    ));
    let denied = exchange(
        &mut actor,
        request(
            2,
            RequestPayload::VerifyStatus(Box::new(VerifyStatus { actor: 7 })),
        ),
    )
    .await;
    assert_capability_denied(denied, MutationEffectState::None);

    let mut admin = connect(&socket, [3; 32]).await;
    let denied = exchange(
        &mut admin,
        request(
            3,
            RequestPayload::Append(Box::new(Append {
                client_seq: 1,
                events: vec![AppendEvent {
                    kind: EventKind::UserMsg as u8,
                    conversation: conversation.into_bytes().to_vec(),
                    payload,
                }],
            })),
        ),
    )
    .await;
    assert_capability_denied(denied, MutationEffectState::NotDispatched);

    let verified = exchange(
        &mut admin,
        request(
            4,
            RequestPayload::VerifyStatus(Box::new(VerifyStatus { actor: 7 })),
        ),
    )
    .await;
    let WirePayload::Response(response) = verified.payload else {
        panic!("verify response");
    };
    let Some(ResponsePayload::VerifyResult(verified)) = response.payload else {
        panic!("verify result");
    };
    assert!(verified.verified);
    assert_eq!(verified.leaf_count, 1);
    assert_eq!(verified.root.len(), 32);

    let stats = exchange(
        &mut admin,
        request(5, RequestPayload::Stats(Box::new(Stats { actor: 7 }))),
    )
    .await;
    let WirePayload::Response(response) = stats.payload else {
        panic!("stats response");
    };
    let Some(ResponsePayload::StatsResult(stats)) = response.payload else {
        panic!("stats result");
    };
    assert_eq!(stats.log_events, 1);
    assert!(
        stats
            .projection_stats
            .iter()
            .any(|item| item.name == "bm25")
    );

    let rebuilt = exchange(
        &mut admin,
        request(
            6,
            RequestPayload::RebuildProjection(Box::new(RebuildProjection {
                actor: 7,
                name: "bm25".to_owned(),
            })),
        ),
    )
    .await;
    assert!(matches!(
        rebuilt.payload,
        WirePayload::Response(response)
            if matches!(&response.payload, Some(ResponsePayload::RebuildResult(result)) if result.applied_lsn == 1)
    ));

    let latencies = exchange(
        &mut admin,
        request(
            7,
            RequestPayload::LatencyHistograms(Box::new(LatencyHistograms {})),
        ),
    )
    .await;
    assert!(matches!(
        latencies.payload,
        WirePayload::Response(response)
            if matches!(&response.payload, Some(ResponsePayload::LatencyResult(result)) if !result.buckets.is_empty())
    ));

    let checkpoint_path = fs::read_dir(actor_directory.join("mmr/checkpoints"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let checkpoint = load_checkpoint(&checkpoint_path).unwrap();
    let deleted = exchange(
        &mut admin,
        request(
            8,
            RequestPayload::CryptoDelete(Box::new(CryptoDelete { actor: 7 })),
        ),
    )
    .await;
    let WirePayload::Response(response) = deleted.payload else {
        panic!("delete response");
    };
    let Some(ResponsePayload::DeleteResult(deleted)) = response.payload else {
        panic!("delete result");
    };
    let receipt = decode_deletion_receipt(&deleted.receipt).unwrap();
    verify_deletion_receipt(&receipt, &checkpoint.public_key).unwrap();
    assert_eq!(receipt.checkpoint_root, checkpoint.root);
    assert!(!actor_directory.join("keys/KEYRING").exists());

    drop(actor);
    drop(admin);
    shutdown_tx.send(()).unwrap();
    task.await.unwrap().unwrap();
}

#[tokio::test]
async fn rotation_survives_restart_and_tripwire_blocks_activation() {
    let temporary = tempfile::tempdir().unwrap();
    let actor_directory = temporary.path().join("actor");
    let base = ActorConfig {
        actor_directory: actor_directory.clone(),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    };
    let conversation = ConversationId::derive("tripwire");
    let engine = ActorEngine::open_with_tripwires(base.clone(), [LSN::new(1)])
        .await
        .unwrap();
    engine.append(vec![message(conversation)]).await.unwrap();
    let error = engine
        .activate(ActivateRequest {
            conversation,
            query: "admin".to_owned(),
            turn_text: String::new(),
            budget_tokens: 1_000,
            token_weights: FallbackWeights::default(),
        })
        .await
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Tripwire);
    assert_eq!(
        admin::rotate_keys(&engine, [8; 32]).await.unwrap(),
        LSN::new(1)
    );
    engine.shutdown().await.unwrap();
    assert_eq!(
        ActorEngine::open(base.clone()).await.err().unwrap().code,
        ErrorCode::CryptoAuthentication
    );
    let reopened = ActorEngine::open(ActorConfig {
        kek: [8; 32],
        ..base
    })
    .await
    .unwrap();
    assert_eq!(reopened.stats().await.unwrap().log_events, 1);
    reopened.shutdown().await.unwrap();
}

fn assert_capability_denied(envelope: WireEnvelope, effect_state: MutationEffectState) {
    let WirePayload::Response(response) = envelope.payload else {
        panic!("error response");
    };
    let Some(ResponsePayload::ErrorDetail(error)) = response.payload else {
        panic!("error detail");
    };
    assert_eq!(error.code, ErrorCode::CapabilityDenied as u8);
    assert_eq!(error.effect_state, effect_state);
}
