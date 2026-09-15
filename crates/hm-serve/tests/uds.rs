use hm_core::ConversationId;
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{Authority, EventEnvelope, EventPayload, Retention, Sensitivity, UserMsg};
use hm_schema::protocol::{encode_wire_envelope, verify_wire_envelope};
use hm_schema::wire::{
    Append, AppendEvent, Hello, Recall, RecallMode, Request, RequestPayload, ResponsePayload,
    WireEnvelope, WirePayload,
};
use hm_serve::config::{ActorCapability, ServerConfig};
use hm_serve::protocol::{FrameParser, encode_frame};
use hm_serve::uds::UdsServer;
use std::fs;
use std::os::unix::fs::PermissionsExt;
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
        maximum_output_frames: 8,
        maximum_output_bytes: 1024 * 1024,
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

async fn exchange(stream: &mut UnixStream, envelope: WireEnvelope) -> WireEnvelope {
    let framed = encode_frame(&encode_wire_envelope(&envelope)).unwrap();
    stream.write_all(&framed).await.unwrap();
    let mut header = [0; 8];
    stream.read_exact(&mut header).await.unwrap();
    let length = u32::from_le_bytes(header[..4].try_into().unwrap()) as usize;
    let mut payload = vec![0; length];
    stream.read_exact(&mut payload).await.unwrap();
    let mut parser = FrameParser::default();
    let parsed = parser
        .push(&[header.as_slice(), payload.as_slice()].concat())
        .unwrap();
    verify_wire_envelope(&parsed[0]).unwrap()
}

#[tokio::test]
async fn real_socket_accepts_v2_append_and_lexical_recall() {
    let temporary = tempfile::tempdir().unwrap();
    let config = config(temporary.path());
    let socket = config.socket_path.clone();
    let server = UdsServer::bind(config).await.unwrap();
    assert_eq!(
        fs::metadata(&socket).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(server.serve_until(async {
        let _ = shutdown_rx.await;
    }));
    let mut stream = UnixStream::connect(&socket).await.unwrap();
    let welcome = exchange(
        &mut stream,
        WireEnvelope {
            proto_version: 2,
            payload: WirePayload::Hello(Box::new(Hello {
                proto_version: 2,
                connection_id: vec![9; 16],
                capability_token: vec![4; 32],
            })),
        },
    )
    .await;
    assert!(matches!(welcome.payload, WirePayload::Welcome(_)));

    let conversation = ConversationId::derive("socket-test");
    let payload = encode_event_envelope(&EventEnvelope {
        schema_version: CURRENT_SCHEMA_VERSION,
        payload: EventPayload::UserMsg(Box::new(UserMsg {
            content: b"socket heliotrope".to_vec(),
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
    });
    let appended = exchange(
        &mut stream,
        WireEnvelope {
            proto_version: 2,
            payload: WirePayload::Request(Box::new(Request {
                request_id: 1,
                payload: RequestPayload::Append(Box::new(Append {
                    client_seq: 1,
                    events: vec![AppendEvent {
                        kind: 1,
                        conversation: conversation.into_bytes().to_vec(),
                        payload,
                    }],
                })),
            })),
        },
    )
    .await;
    let WirePayload::Response(response) = appended.payload else {
        panic!("expected response");
    };
    assert!(matches!(
        response.payload,
        Some(ResponsePayload::AppendAck(_))
    ));

    let recalled = exchange(
        &mut stream,
        WireEnvelope {
            proto_version: 2,
            payload: WirePayload::Request(Box::new(Request {
                request_id: 2,
                payload: RequestPayload::Recall(Box::new(Recall {
                    query: b"heliotrope".to_vec(),
                    limit: 10,
                    mode: RecallMode::ListWindows,
                    level: 0,
                    start_ns: 0,
                    end_ns: 0,
                })),
            })),
        },
    )
    .await;
    let WirePayload::Response(response) = recalled.payload else {
        panic!("expected response");
    };
    let Some(ResponsePayload::RecallResult(result)) = response.payload else {
        panic!("expected recall result");
    };
    assert_eq!(result.members.unwrap(), vec![1]);
    drop(stream);
    shutdown_tx.send(()).unwrap();
    task.await.unwrap().unwrap();
}
