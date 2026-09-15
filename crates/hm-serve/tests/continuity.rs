#![allow(clippy::similar_names)]

use hm_core::ErrorCode;
use hm_schema::protocol::{encode_wire_envelope, verify_wire_envelope};
use hm_schema::wire::{
    Checkpoint, Hello, LatestCheckpoint, MutationEffectState, Request, RequestPayload,
    ResponsePayload, Subscribe, WireEnvelope, WirePayload,
};
use hm_serve::config::{ActorCapability, ServerConfig};
use hm_serve::protocol::{FrameParser, encode_frame};
use hm_serve::uds::UdsServer;
use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

fn config(path: &Path) -> ServerConfig {
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
    }
}

async fn send(stream: &mut UnixStream, envelope: WireEnvelope) {
    stream
        .write_all(&encode_frame(&encode_wire_envelope(&envelope)).unwrap())
        .await
        .unwrap();
}

async fn receive(stream: &mut UnixStream) -> WireEnvelope {
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

async fn exchange(stream: &mut UnixStream, envelope: WireEnvelope) -> WireEnvelope {
    send(stream, envelope).await;
    receive(stream).await
}

fn hello(connection_id: u8) -> WireEnvelope {
    WireEnvelope {
        proto_version: 3,
        payload: WirePayload::Hello(Box::new(Hello {
            proto_version: 3,
            connection_id: vec![connection_id; 16],
            capability_token: vec![4; 32],
        })),
    }
}

fn request(request_id: u64, payload: RequestPayload) -> WireEnvelope {
    WireEnvelope {
        proto_version: 3,
        payload: WirePayload::Request(Box::new(Request {
            request_id,
            payload,
        })),
    }
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn checkpoint_is_idempotent_and_subscribe_replays_then_streams() {
    let temporary = tempfile::tempdir().unwrap();
    let config = config(temporary.path());
    let socket = config.socket_path.clone();
    let server = UdsServer::bind(config).await.unwrap();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let server_task = tokio::spawn(server.serve_until(async {
        let _ = shutdown_rx.await;
    }));

    let mut client = UnixStream::connect(&socket).await.unwrap();
    let welcomed = exchange(&mut client, hello(9)).await;
    let WirePayload::Welcome(welcome) = welcomed.payload else {
        panic!("expected welcome");
    };
    assert_eq!(welcome.next_client_seq, 1);

    let checkpoint = || {
        request(
            1,
            RequestPayload::Checkpoint(Box::new(Checkpoint {
                turn_id: b"turn-17".to_vec(),
                blob: b"opaque-state".to_vec(),
                client_seq: 1,
            })),
        )
    };
    let first = exchange(&mut client, checkpoint()).await;
    let WirePayload::Response(first) = first.payload else {
        panic!("expected checkpoint response");
    };
    let Some(ResponsePayload::CheckpointAck(first_ack)) = first.payload else {
        panic!("expected checkpoint ack");
    };
    assert_eq!(first_ack.lsn, 1);

    let duplicate = exchange(&mut client, checkpoint()).await;
    let WirePayload::Response(duplicate) = duplicate.payload else {
        panic!("expected duplicate response");
    };
    let Some(ResponsePayload::CheckpointAck(duplicate_ack)) = duplicate.payload else {
        panic!("expected duplicate checkpoint ack");
    };
    assert_eq!(duplicate_ack.lsn, first_ack.lsn);

    let conflict = exchange(
        &mut client,
        request(
            2,
            RequestPayload::Checkpoint(Box::new(Checkpoint {
                turn_id: b"turn-17".to_vec(),
                blob: b"different-state".to_vec(),
                client_seq: 1,
            })),
        ),
    )
    .await;
    let WirePayload::Response(conflict) = conflict.payload else {
        panic!("expected conflict response");
    };
    let Some(ResponsePayload::ErrorDetail(detail)) = conflict.payload else {
        panic!("expected error detail");
    };
    assert_eq!(detail.code, ErrorCode::IdempotencyConflict as u8);
    assert_eq!(detail.effect_state, MutationEffectState::Rejected);

    let latest = exchange(
        &mut client,
        request(
            3,
            RequestPayload::LatestCheckpoint(Box::new(LatestCheckpoint {
                turn_id: b"turn-17".to_vec(),
            })),
        ),
    )
    .await;
    let WirePayload::Response(latest) = latest.payload else {
        panic!("expected latest response");
    };
    let Some(ResponsePayload::CheckpointResult(latest)) = latest.payload else {
        panic!("expected latest checkpoint");
    };
    assert!(latest.present);
    assert_eq!(latest.lsn, 1);
    assert_eq!(latest.blob.as_deref(), Some(b"opaque-state".as_slice()));

    drop(client);
    let mut subscriber = UnixStream::connect(&socket).await.unwrap();
    let welcomed = exchange(&mut subscriber, hello(9)).await;
    let WirePayload::Welcome(welcome) = welcomed.payload else {
        panic!("expected reconnect welcome");
    };
    assert_eq!(welcome.next_client_seq, 2);

    send(
        &mut subscriber,
        request(
            4,
            RequestPayload::Subscribe(Box::new(Subscribe {
                conversation: None,
                since_lsn: 0,
            })),
        ),
    )
    .await;
    let subscribed = receive(&mut subscriber).await;
    let WirePayload::Response(subscribed) = subscribed.payload else {
        panic!("expected subscription response");
    };
    let Some(ResponsePayload::SubscriptionAck(subscription)) = subscribed.payload else {
        panic!("expected subscription ack");
    };
    let replay = receive(&mut subscriber).await;
    let WirePayload::Event(replay) = replay.payload else {
        panic!("expected replay event");
    };
    assert_eq!(replay.subscription_id, subscription.subscription_id);
    assert_eq!(replay.lsn, 1);

    let mut producer = UnixStream::connect(&socket).await.unwrap();
    assert!(matches!(
        exchange(&mut producer, hello(8)).await.payload,
        WirePayload::Welcome(_)
    ));
    let produced = exchange(
        &mut producer,
        request(
            5,
            RequestPayload::Checkpoint(Box::new(Checkpoint {
                turn_id: b"turn-18".to_vec(),
                blob: b"new-live-state".to_vec(),
                client_seq: 1,
            })),
        ),
    )
    .await;
    match produced.payload {
        WirePayload::Response(response)
            if matches!(response.payload, Some(ResponsePayload::CheckpointAck(_))) => {}
        other => panic!("expected checkpoint ack, got {other:?}"),
    }
    let live = receive(&mut subscriber).await;
    let WirePayload::Event(live) = live.payload else {
        panic!("expected live event");
    };
    assert_eq!(live.subscription_id, subscription.subscription_id);
    assert_eq!(live.lsn, 2);
    assert_eq!(live.kind, hm_schema::event::EventKind::Checkpoint as u8);

    drop(producer);
    drop(subscriber);
    shutdown_tx.send(()).unwrap();
    server_task.await.unwrap().unwrap();
}

#[tokio::test]
async fn invalid_mutation_reports_not_dispatched() {
    let temporary = tempfile::tempdir().unwrap();
    let config = config(temporary.path());
    let socket = config.socket_path.clone();
    let server = UdsServer::bind(config).await.unwrap();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let server_task = tokio::spawn(server.serve_until(async {
        let _ = shutdown_rx.await;
    }));
    let mut client = UnixStream::connect(&socket).await.unwrap();
    let _ = exchange(&mut client, hello(6)).await;
    let invalid = exchange(
        &mut client,
        request(
            1,
            RequestPayload::Checkpoint(Box::new(Checkpoint {
                turn_id: Vec::new(),
                blob: b"state".to_vec(),
                client_seq: 1,
            })),
        ),
    )
    .await;
    let WirePayload::Response(invalid) = invalid.payload else {
        panic!("expected error response");
    };
    let Some(ResponsePayload::ErrorDetail(detail)) = invalid.payload else {
        panic!("expected error detail");
    };
    assert_eq!(detail.effect_state, MutationEffectState::NotDispatched);

    drop(client);
    shutdown_tx.send(()).unwrap();
    server_task.await.unwrap().unwrap();
}

#[tokio::test]
async fn saturated_subscriber_is_disconnected() {
    let temporary = tempfile::tempdir().unwrap();
    let mut config = config(temporary.path());
    config.maximum_output_bytes = 2048;
    let socket = config.socket_path.clone();
    let server = UdsServer::bind(config).await.unwrap();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let server_task = tokio::spawn(server.serve_until(async {
        let _ = shutdown_rx.await;
    }));

    let mut subscriber = UnixStream::connect(&socket).await.unwrap();
    let _ = exchange(&mut subscriber, hello(5)).await;
    let subscribed = exchange(
        &mut subscriber,
        request(
            1,
            RequestPayload::Subscribe(Box::new(Subscribe {
                conversation: None,
                since_lsn: 0,
            })),
        ),
    )
    .await;
    assert!(matches!(
        subscribed.payload,
        WirePayload::Response(response)
            if matches!(response.payload, Some(ResponsePayload::SubscriptionAck(_)))
    ));

    let mut producer = UnixStream::connect(&socket).await.unwrap();
    let _ = exchange(&mut producer, hello(4)).await;
    let produced = exchange(
        &mut producer,
        request(
            2,
            RequestPayload::Checkpoint(Box::new(Checkpoint {
                turn_id: b"large-turn".to_vec(),
                blob: vec![b'x'; 4096],
                client_seq: 1,
            })),
        ),
    )
    .await;
    match produced.payload {
        WirePayload::Response(response)
            if matches!(response.payload, Some(ResponsePayload::CheckpointAck(_))) => {}
        other => panic!("expected checkpoint ack, got {other:?}"),
    }

    let mut byte = [0; 1];
    let read = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        subscriber.read(&mut byte),
    )
    .await
    .expect("subscriber was not disconnected")
    .unwrap();
    assert_eq!(read, 0);

    drop(producer);
    drop(subscriber);
    shutdown_tx.send(()).unwrap();
    server_task.await.unwrap().unwrap();
}
