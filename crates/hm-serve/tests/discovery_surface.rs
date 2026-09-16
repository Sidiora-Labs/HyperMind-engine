#![allow(clippy::too_many_lines)]

use hm_core::ErrorCode;
use hm_schema::protocol::{encode_wire_envelope, verify_wire_envelope};
use hm_schema::wire::{
    Hello, Request, RequestPayload, ResponsePayload, ToolRequest, WireEnvelope, WirePayload,
};
use hm_serve::config::{ActorCapability, ServerConfig};
use hm_serve::protocol::{FrameParser, encode_frame};
use hm_serve::uds::UdsServer;
use serde_json::Value;
use std::sync::Arc;
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
        maximum_active_actors: 2,
        maximum_heavy_jobs: 1,
        lease_wait_ms: 250,
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

fn hello(token: u8, connection: u8) -> WireEnvelope {
    WireEnvelope {
        proto_version: 3,
        payload: WirePayload::Hello(Box::new(Hello {
            proto_version: 3,
            connection_id: vec![connection; 16],
            capability_token: vec![token; 32],
        })),
    }
}

fn tool_request(request_id: u64, verb: &str, arguments: &[u8]) -> WireEnvelope {
    WireEnvelope {
        proto_version: 3,
        payload: WirePayload::Request(Box::new(Request {
            request_id,
            payload: RequestPayload::ToolRequest(Box::new(ToolRequest {
                verb: verb.to_owned(),
                arguments_json: arguments.to_vec(),
            })),
        })),
    }
}

async fn call(stream: &mut UnixStream, request_id: u64, verb: &str, arguments: &[u8]) -> Value {
    let response = exchange(stream, tool_request(request_id, verb, arguments)).await;
    let WirePayload::Response(response) = response.payload else {
        panic!("expected a response for {verb}");
    };
    let Some(ResponsePayload::BytesResult(result)) = response.payload else {
        panic!("expected a bytes result for {verb}: {:?}", response.payload);
    };
    serde_json::from_slice(&result.bytes).unwrap()
}

fn surface<'a>(envelope: &'a Value, id: &str) -> &'a Value {
    envelope["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["surface"] == id)
        .unwrap_or_else(|| panic!("discovery did not list {id}: {envelope}"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn discovery_advertises_surfaces_without_granting_them() {
    let temporary = tempfile::tempdir().unwrap();
    let config = config(temporary.path());
    let socket = config.socket_path.clone();
    let server = UdsServer::bind(config)
        .await
        .unwrap()
        .with_tool_dispatcher(Arc::new(hm_mcp::dispatcher::McpToolDispatcher::default()));
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(server.serve_until(async {
        let _ = shutdown_rx.await;
    }));

    let mut stream = UnixStream::connect(&socket).await.unwrap();
    let welcome = exchange(&mut stream, hello(4, 9)).await;
    let WirePayload::Welcome(welcome) = welcome.payload else {
        panic!("expected a welcome");
    };
    assert_eq!(welcome.actor_ns, 7);
    assert!(!welcome.admin);

    let discovered = call(
        &mut stream,
        1,
        "inspect",
        br#"{"mode":"discover","query":"forget"}"#,
    )
    .await;
    assert_eq!(discovered["ok"], true, "{discovered}");
    let shred = surface(&discovered, "forget.crypto_shred");
    assert_eq!(shred["verb"], "forget", "{discovered}");
    assert_eq!(shred["requires"], "admin_token", "{discovered}");
    assert_eq!(shred["available"], false, "{discovered}");
    assert_eq!(
        shred["unavailable_reason"], "admin_token_absent",
        "{discovered}"
    );
    assert!(
        discovered["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning == "discovery_is_not_authorization"),
        "{discovered}"
    );
    assert_eq!(discovered["health"]["advertised_tools"], 14, "{discovered}");
    assert_eq!(
        discovered["health"]["discoverable_surfaces"],
        hm_mcp::tools::surfaces::SURFACES.len(),
        "{discovered}"
    );

    let denied = call(&mut stream, 2, "forget", br#"{"action":"crypto_shred"}"#).await;
    assert_eq!(denied["ok"], false, "{denied}");
    assert_eq!(denied["items"][0]["error"], "kCapabilityDenied", "{denied}");
    let effect_state = denied["effect_state"]
        .as_str()
        .unwrap_or_else(|| panic!("a refused mutation must report an effect state: {denied}"));
    assert!(
        matches!(effect_state, "not_dispatched" | "rejected"),
        "{denied}"
    );

    let rediscovered = call(
        &mut stream,
        3,
        "inspect",
        br#"{"mode":"discover","query":"crypto_shred"}"#,
    )
    .await;
    assert_eq!(rediscovered["ok"], true, "{rediscovered}");
    assert_eq!(
        surface(&rediscovered, "forget.crypto_shred")["available"],
        false,
        "{rediscovered}"
    );

    assert_eq!(
        hm_serve::rest::VERBS.len(),
        14,
        "the advertised surface is fourteen verbs; a new capability is a surface row, never a verb"
    );
    let advertised = hm_mcp::tools::surfaces::SURFACES
        .iter()
        .map(|surface| surface.verb)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        advertised.len(),
        14,
        "the discovery catalogue covers fourteen verbs; a new capability is a surface row, never a verb"
    );
    assert_eq!(
        advertised,
        hm_serve::rest::VERBS.into_iter().collect(),
        "every discoverable surface belongs to an already advertised verb"
    );
    assert_eq!(
        hm_mcp::tools::surfaces::SURFACES.len(),
        28,
        "adding a capability adds a row to the discovery catalogue; update this count deliberately"
    );

    let mut admin = UnixStream::connect(&socket).await.unwrap();
    let welcome = exchange(&mut admin, hello(3, 10)).await;
    let WirePayload::Welcome(welcome) = welcome.payload else {
        panic!("expected a welcome");
    };
    assert!(welcome.admin);
    let refused = exchange(
        &mut admin,
        tool_request(4, "inspect", br#"{"mode":"discover","query":"forget"}"#),
    )
    .await;
    let WirePayload::Response(response) = refused.payload else {
        panic!("expected a response");
    };
    let Some(ResponsePayload::ErrorDetail(detail)) = response.payload else {
        panic!("expected an error detail for a tool request on an admin connection");
    };
    assert_eq!(detail.code, ErrorCode::CapabilityDenied as u8);

    drop(stream);
    drop(admin);
    shutdown_tx.send(()).unwrap();
    task.await.unwrap().unwrap();
}
