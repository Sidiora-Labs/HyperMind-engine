#![allow(clippy::too_many_lines)]

use hm_core::telemetry::SpanKind;
use hm_schema::protocol::{encode_wire_envelope, verify_wire_envelope};
use hm_schema::wire::{
    Hello, Recall, RecallMode, Request, RequestPayload, WireEnvelope, WirePayload,
};
use hm_serve::config::{ActorCapability, ServerConfig};
use hm_serve::protocol::{FrameParser, encode_frame};
use hm_serve::telemetry::TelemetryMode;
use hm_serve::uds::UdsServer;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

const ACTOR_TOKEN: [u8; 32] = [4; 32];
const ADMIN_TOKEN: [u8; 32] = [3; 32];

fn config(path: &std::path::Path) -> ServerConfig {
    ServerConfig {
        socket_path: path.join("hypermind.sock"),
        data_directory: path.join("data"),
        user: [1; 16],
        kek: [2; 32],
        admin_token: ADMIN_TOKEN,
        actors: vec![ActorCapability {
            actor: 7,
            token: ACTOR_TOKEN,
        }],
        maximum_connections: 4,
        maximum_output_frames: 8,
        maximum_output_bytes: 1024 * 1024,
        projection_map_bytes: 16 * 1024 * 1024,
        maximum_active_actors: 64,
        maximum_heavy_jobs: 2,
        lease_wait_ms: 250,
    }
}

fn hello(connection_id: u8, token: [u8; 32]) -> WireEnvelope {
    WireEnvelope {
        proto_version: 2,
        payload: WirePayload::Hello(Box::new(Hello {
            proto_version: 2,
            connection_id: vec![connection_id; 16],
            capability_token: token.to_vec(),
        })),
    }
}

fn recall(request_id: u64, query: &str) -> WireEnvelope {
    WireEnvelope {
        proto_version: 2,
        payload: WirePayload::Request(Box::new(Request {
            request_id,
            payload: RequestPayload::Recall(Box::new(Recall {
                query: query.as_bytes().to_vec(),
                limit: 5,
                mode: RecallMode::ListWindows,
                level: 1,
                start_ns: 0,
                end_ns: 0,
            })),
        })),
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

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(encoded, "{byte:02x}").unwrap();
    }
    encoded
}

fn lowercase_hex(value: &str) -> bool {
    value
        .chars()
        .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn opt_in_spans_are_metadata_only_and_flush_on_shutdown() {
    let temporary = tempfile::tempdir().unwrap();
    let spans = temporary.path().join("spans.jsonl");
    assert_eq!(
        hm_serve::telemetry::configure(TelemetryMode::File, Some(&spans), "hypermind-test"),
        Ok(true)
    );

    let server_config = config(temporary.path());
    let socket = server_config.socket_path.clone();
    let server = UdsServer::bind(server_config)
        .await
        .unwrap()
        .with_tool_dispatcher(Arc::new(hm_mcp::dispatcher::McpToolDispatcher::default()));
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(server.serve_until(async {
        let _ = shutdown_rx.await;
    }));

    let mut actor_stream = UnixStream::connect(&socket).await.unwrap();
    let welcome = exchange(&mut actor_stream, hello(9, ACTOR_TOKEN)).await;
    assert!(matches!(welcome.payload, WirePayload::Welcome(_)));
    let recalled = exchange(&mut actor_stream, recall(1, "heliotrope quartzite")).await;
    assert!(matches!(recalled.payload, WirePayload::Response(_)));

    let mut admin_stream = UnixStream::connect(&socket).await.unwrap();
    let admin_welcome = exchange(&mut admin_stream, hello(10, ADMIN_TOKEN)).await;
    assert!(matches!(admin_welcome.payload, WirePayload::Welcome(_)));
    let health = exchange(
        &mut admin_stream,
        WireEnvelope {
            proto_version: 2,
            payload: WirePayload::Request(Box::new(Request {
                request_id: 2,
                payload: RequestPayload::Health(Box::default()),
            })),
        },
    )
    .await;
    assert!(matches!(health.payload, WirePayload::Response(_)));

    drop(actor_stream);
    drop(admin_stream);
    shutdown_tx.send(()).unwrap();
    task.await.unwrap().unwrap();

    assert_eq!(
        fs::metadata(&spans).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let text = fs::read_to_string(&spans).unwrap();
    assert!(!text.trim().is_empty());
    let mut names = BTreeSet::new();
    let mut kinds = BTreeSet::new();
    for line in text.lines() {
        let value: Value = serde_json::from_str(line).unwrap();
        let span = &value["resourceSpans"][0]["scopeSpans"][0]["spans"][0];
        assert!(span.is_object());
        let trace_id = span["traceId"].as_str().unwrap();
        assert_eq!(trace_id.len(), 32);
        assert!(lowercase_hex(trace_id));
        let span_id = span["spanId"].as_str().unwrap();
        assert_eq!(span_id.len(), 16);
        assert!(lowercase_hex(span_id));
        let start: u128 = span["startTimeUnixNano"].as_str().unwrap().parse().unwrap();
        let end: u128 = span["endTimeUnixNano"].as_str().unwrap().parse().unwrap();
        assert!(end > start);
        let status = span["status"]["code"].as_u64().unwrap();
        assert!(status == 1 || status == 2);
        names.insert(span["name"].as_str().unwrap().to_owned());
        let mut actor_recorded = false;
        let mut mutation_recorded = false;
        for attribute in span["attributes"].as_array().unwrap() {
            let value = &attribute["value"];
            match attribute["key"].as_str().unwrap() {
                "hypermind.request.kind" => {
                    kinds.insert(value["stringValue"].as_str().unwrap().to_owned());
                }
                "hypermind.actor" => {
                    assert!(value["intValue"].is_i64());
                    actor_recorded = true;
                }
                "hypermind.request.mutation" => {
                    assert!(value["boolValue"].is_boolean());
                    mutation_recorded = true;
                }
                _ => {}
            }
        }
        assert!(actor_recorded);
        assert!(mutation_recorded);
    }
    assert!(names.contains("hypermind.request"));
    assert!(kinds.contains("recall"));
    assert!(kinds.contains("health"));
    assert!(!text.contains("heliotrope"));
    assert!(!text.contains("quartzite"));
    assert!(!text.contains(&hex(&ACTOR_TOKEN)));
    assert!(!text.contains(&hex(&ADMIN_TOKEN)));

    assert_eq!(
        hm_serve::telemetry::configure(TelemetryMode::Off, None, "hypermind-test"),
        Ok(false)
    );
    assert!(hm_core::telemetry::start_span(SpanKind::Request, "hypermind.request").is_none());
    let recorded_length = fs::metadata(&spans).unwrap().len();

    let second_directory = tempfile::tempdir().unwrap();
    let second_config = config(second_directory.path());
    let second_socket = second_config.socket_path.clone();
    let second_server = UdsServer::bind(second_config)
        .await
        .unwrap()
        .with_tool_dispatcher(Arc::new(hm_mcp::dispatcher::McpToolDispatcher::default()));
    let (second_shutdown_tx, second_shutdown_rx) = tokio::sync::oneshot::channel();
    let second_task = tokio::spawn(second_server.serve_until(async {
        let _ = second_shutdown_rx.await;
    }));
    let mut second_stream = UnixStream::connect(&second_socket).await.unwrap();
    let second_welcome = exchange(&mut second_stream, hello(11, ACTOR_TOKEN)).await;
    assert!(matches!(second_welcome.payload, WirePayload::Welcome(_)));
    let second_recalled = exchange(&mut second_stream, recall(1, "heliotrope quartzite")).await;
    assert!(matches!(second_recalled.payload, WirePayload::Response(_)));
    drop(second_stream);
    second_shutdown_tx.send(()).unwrap();
    second_task.await.unwrap().unwrap();

    assert_eq!(fs::metadata(&spans).unwrap().len(), recorded_length);
}
