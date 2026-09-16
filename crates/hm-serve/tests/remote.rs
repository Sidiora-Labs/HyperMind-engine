#![allow(clippy::too_many_lines)]

use base64::Engine as _;
use hm_core::ConversationId;
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{Authority, EventEnvelope, EventPayload, Retention, Sensitivity, UserMsg};
use hm_schema::protocol::{encode_wire_envelope, verify_wire_envelope};
use hm_schema::wire::{
    Append, AppendEvent, Hello, Recall, RecallMode, Request, RequestPayload, ResponsePayload,
    Subscribe, WireEnvelope, WirePayload,
};
use hm_serve::config::{ActorCapability, ServerConfig};
use hm_serve::grpc::wire::hyper_mind_client::HyperMindClient;
use hm_serve::grpc::wire::{Envelope, ExchangeRequest};
use hm_serve::grpc::{Gateway, GrpcServer, ListenerRole, TlsIdentity};
use hm_serve::rest::RestServer;
use hm_serve::uds::UdsServer;
use rcgen::{
    BasicConstraints, CertificateParams, CertifiedIssuer, ExtendedKeyUsagePurpose, IsCa, KeyPair,
    KeyUsagePurpose,
};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Endpoint, Identity};

struct Certificates {
    tls: TlsIdentity,
    client_cert: String,
    client_key: String,
}

fn certificates() -> Certificates {
    let mut ca_params = CertificateParams::new(Vec::<String>::new()).unwrap();
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca_params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    let ca = CertifiedIssuer::self_signed(ca_params, KeyPair::generate().unwrap()).unwrap();
    let mut server_params = CertificateParams::new(vec!["localhost".to_owned()]).unwrap();
    server_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    let server_key = KeyPair::generate().unwrap();
    let server = server_params.signed_by(&server_key, &ca).unwrap();
    let mut client_params = CertificateParams::new(vec!["remote-client".to_owned()]).unwrap();
    client_params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ClientAuth];
    let client_key = KeyPair::generate().unwrap();
    let client = client_params.signed_by(&client_key, &ca).unwrap();
    Certificates {
        tls: TlsIdentity {
            certificate_pem: server.pem().into_bytes(),
            private_key_pem: server_key.serialize_pem().into_bytes(),
            client_ca_pem: ca.pem().into_bytes(),
        },
        client_cert: client.pem(),
        client_key: client_key.serialize_pem(),
    }
}

fn config(path: &std::path::Path) -> ServerConfig {
    ServerConfig {
        socket_path: path.join("daemon.sock"),
        data_directory: path.join("data"),
        user: [1; 16],
        kek: [2; 32],
        admin_token: [3; 32],
        actors: vec![
            ActorCapability {
                actor: 7,
                token: [4; 32],
            },
            ActorCapability {
                actor: 8,
                token: [5; 32],
            },
        ],
        maximum_connections: 32,
        maximum_output_frames: 16,
        maximum_output_bytes: 1024 * 1024,
        projection_map_bytes: 16 * 1024 * 1024,
        maximum_active_actors: 64,
        maximum_heavy_jobs: 2,
        lease_wait_ms: 250,
    }
}

fn hello(token: u8, connection: u8) -> Vec<u8> {
    encode_wire_envelope(&WireEnvelope {
        proto_version: 3,
        payload: WirePayload::Hello(Box::new(Hello {
            proto_version: 3,
            connection_id: vec![connection; 16],
            capability_token: vec![token; 32],
        })),
    })
}

fn request(id: u64, payload: RequestPayload) -> Vec<u8> {
    encode_wire_envelope(&WireEnvelope {
        proto_version: 3,
        payload: WirePayload::Request(Box::new(Request {
            request_id: id,
            payload,
        })),
    })
}

fn append(sequence: u64, content: &str) -> Vec<u8> {
    let payload = encode_event_envelope(&EventEnvelope {
        schema_version: CURRENT_SCHEMA_VERSION,
        payload: EventPayload::UserMsg(Box::new(UserMsg {
            content: content.as_bytes().to_vec(),
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
    request(
        sequence,
        RequestPayload::Append(Box::new(Append {
            client_seq: sequence,
            events: vec![AppendEvent {
                kind: 1,
                conversation: ConversationId::derive("remote").into_bytes().to_vec(),
                payload,
            }],
        })),
    )
}

async fn grpc(
    address: std::net::SocketAddr,
    certs: &Certificates,
    client_identity: bool,
) -> Result<HyperMindClient<Channel>, tonic::transport::Error> {
    let mut tls = ClientTlsConfig::new()
        .domain_name("localhost")
        .ca_certificate(Certificate::from_pem(&certs.tls.client_ca_pem));
    if client_identity {
        tls = tls.identity(Identity::from_pem(&certs.client_cert, &certs.client_key));
    }
    let channel = Endpoint::from_shared(format!("https://{address}"))
        .unwrap()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .tls_config(tls)?
        .connect()
        .await?;
    Ok(HyperMindClient::new(channel))
}

fn http(certs: &Certificates, client_identity: bool) -> reqwest::Client {
    let mut builder = reqwest::Client::builder()
        .use_rustls_tls()
        .tls_built_in_root_certs(false)
        .add_root_certificate(reqwest::Certificate::from_pem(&certs.tls.client_ca_pem).unwrap())
        .timeout(Duration::from_secs(10));
    if client_identity {
        builder = builder.identity(
            reqwest::Identity::from_pem(
                format!("{}{}", certs.client_cert, certs.client_key).as_bytes(),
            )
            .unwrap(),
        );
    }
    builder.build().unwrap()
}

async fn post(
    client: &reqwest::Client,
    base: &str,
    verb: &str,
    token: u8,
    arguments: Value,
) -> (reqwest::StatusCode, Value) {
    let response = client
        .post(format!("{base}/v1/{verb}"))
        .bearer_auth(format!("{token:02x}").repeat(32))
        .json(&json!({"connection_id":"ab".repeat(16),"request_id":1,"arguments":arguments}))
        .send()
        .await
        .unwrap();
    (response.status(), response.json().await.unwrap())
}

#[tokio::test]
async fn remote_mtls_capabilities_streaming_rest_and_restart() {
    let temporary = tempfile::tempdir().unwrap();
    let config = Arc::new(config(temporary.path()));
    let daemon = UdsServer::bind((*config).clone())
        .await
        .unwrap()
        .with_tool_dispatcher(Arc::new(hm_mcp::dispatcher::McpToolDispatcher::default()));
    let (stop, stopped) = tokio::sync::watch::channel(false);
    let shutdown = |mut receiver: tokio::sync::watch::Receiver<bool>| async move {
        let _ = receiver.changed().await;
    };
    let daemon_task = tokio::spawn(daemon.serve_until(shutdown(stopped.clone())));
    let certs = certificates();
    let actor_gateway = Gateway::new(config.clone(), ListenerRole::Actor);
    let admin_gateway = Gateway::new(config.clone(), ListenerRole::Admin);
    let actor_grpc = GrpcServer::bind(
        "127.0.0.1:0".parse().unwrap(),
        actor_gateway.clone(),
        certs.tls.clone(),
    )
    .await
    .unwrap();
    let grpc_address = actor_grpc.local_addr().unwrap();
    let grpc_task = tokio::spawn(actor_grpc.serve_until(shutdown(stopped.clone())));
    let admin_grpc = GrpcServer::bind(
        "127.0.0.1:0".parse().unwrap(),
        admin_gateway.clone(),
        certs.tls.clone(),
    )
    .await
    .unwrap();
    let admin_grpc_address = admin_grpc.local_addr().unwrap();
    let admin_grpc_task = tokio::spawn(admin_grpc.serve_until(shutdown(stopped.clone())));
    let rest = RestServer::bind(
        "127.0.0.1:0".parse().unwrap(),
        actor_gateway,
        certs.tls.clone(),
    )
    .await
    .unwrap();
    let rest_base = format!("https://localhost:{}", rest.local_addr().unwrap().port());
    let rest_task = tokio::spawn(rest.serve_until(shutdown(stopped.clone())));
    let admin_rest = RestServer::bind(
        "127.0.0.1:0".parse().unwrap(),
        admin_gateway,
        certs.tls.clone(),
    )
    .await
    .unwrap();
    let admin_base = format!(
        "https://localhost:{}",
        admin_rest.local_addr().unwrap().port()
    );
    let admin_rest_task = tokio::spawn(admin_rest.serve_until(shutdown(stopped.clone())));

    match grpc(grpc_address, &certs, false).await {
        Err(_) => {}
        Ok(mut client) => assert!(
            client
                .connect(Envelope { ncpr: hello(4, 9) })
                .await
                .is_err()
        ),
    }
    assert!(
        http(&certs, false)
            .get(format!("{rest_base}/openapi.json"))
            .send()
            .await
            .is_err()
    );
    let untrusted = certificates();
    let wrong_identity = Endpoint::from_shared(format!("https://{grpc_address}"))
        .unwrap()
        .connect_timeout(Duration::from_secs(5))
        .tls_config(
            ClientTlsConfig::new()
                .domain_name("localhost")
                .ca_certificate(Certificate::from_pem(&certs.tls.client_ca_pem))
                .identity(Identity::from_pem(
                    &untrusted.client_cert,
                    &untrusted.client_key,
                )),
        )
        .unwrap()
        .connect()
        .await;
    if let Ok(channel) = wrong_identity {
        assert!(
            HyperMindClient::new(channel)
                .connect(Envelope { ncpr: hello(4, 9) })
                .await
                .is_err()
        );
    }
    let mut client = grpc(grpc_address, &certs, true).await.unwrap();
    assert_eq!(
        client
            .connect(Envelope { ncpr: hello(0, 9) })
            .await
            .unwrap_err()
            .code(),
        tonic::Code::Unauthenticated
    );
    assert_eq!(
        client
            .connect(Envelope { ncpr: hello(3, 9) })
            .await
            .unwrap_err()
            .code(),
        tonic::Code::PermissionDenied
    );
    let welcome = client
        .connect(Envelope { ncpr: hello(4, 9) })
        .await
        .unwrap()
        .into_inner();
    assert!(
        matches!(verify_wire_envelope(&welcome.ncpr).unwrap().payload,WirePayload::Welcome(value) if value.actor_ns==7 && value.next_client_seq==1)
    );
    let malformed = client
        .exchange(ExchangeRequest {
            hello: hello(4, 9),
            request: b"invalid NCPR".to_vec(),
        })
        .await
        .unwrap_err();
    assert_eq!(malformed.code(), tonic::Code::InvalidArgument);
    assert_eq!(
        malformed.metadata().get("effect-state").unwrap(),
        "not_dispatched"
    );
    let result = client
        .exchange(ExchangeRequest {
            hello: hello(4, 9),
            request: append(1, "remote heliotrope"),
        })
        .await
        .unwrap()
        .into_inner();
    assert!(
        matches!(verify_wire_envelope(&result.ncpr).unwrap().payload,WirePayload::Response(value) if matches!(value.payload,Some(ResponsePayload::AppendAck(ref ack)) if ack.first_lsn==1 && !ack.duplicate))
    );
    let duplicate = client
        .exchange(ExchangeRequest {
            hello: hello(4, 9),
            request: append(1, "remote heliotrope"),
        })
        .await
        .unwrap()
        .into_inner();
    assert!(
        matches!(verify_wire_envelope(&duplicate.ncpr).unwrap().payload,WirePayload::Response(value) if matches!(value.payload,Some(ResponsePayload::AppendAck(ref ack)) if ack.duplicate))
    );
    let subscription = request(
        10,
        RequestPayload::Subscribe(Box::new(Subscribe {
            conversation: None,
            since_lsn: 0,
        })),
    );
    let mut stream = client
        .subscribe(ExchangeRequest {
            hello: hello(4, 10),
            request: subscription,
        })
        .await
        .unwrap()
        .into_inner();
    let ack = stream.message().await.unwrap().unwrap();
    assert!(
        matches!(verify_wire_envelope(&ack.ncpr).unwrap().payload,WirePayload::Response(value) if matches!(value.payload,Some(ResponsePayload::SubscriptionAck(_))))
    );
    let replay = stream.message().await.unwrap().unwrap();
    assert!(
        matches!(verify_wire_envelope(&replay.ncpr).unwrap().payload,WirePayload::Event(value) if value.actor==7 && value.lsn==1)
    );
    client
        .exchange(ExchangeRequest {
            hello: hello(4, 9),
            request: append(2, "remote iris"),
        })
        .await
        .unwrap();
    let pushed = tokio::time::timeout(Duration::from_secs(5), stream.message())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(
        matches!(verify_wire_envelope(&pushed.ncpr).unwrap().payload,WirePayload::Event(value) if value.actor==7 && value.lsn==2)
    );
    drop(stream);
    let mut bounded_config = (*config).clone();
    bounded_config.maximum_output_bytes = 1024;
    let bounded_server = GrpcServer::bind(
        "127.0.0.1:0".parse().unwrap(),
        Gateway::new(Arc::new(bounded_config), ListenerRole::Actor),
        certs.tls.clone(),
    )
    .await
    .unwrap();
    let bounded_client = grpc(bounded_server.local_addr().unwrap(), &certs, true);
    let bounded_task = tokio::spawn(bounded_server.serve_until(shutdown(stopped.clone())));
    let mut bounded_client = bounded_client.await.unwrap();
    let mut bounded_stream = bounded_client
        .subscribe(ExchangeRequest {
            hello: hello(4, 70),
            request: request(
                10,
                RequestPayload::Subscribe(Box::new(Subscribe {
                    conversation: None,
                    since_lsn: 2,
                })),
            ),
        })
        .await
        .unwrap()
        .into_inner();
    assert!(bounded_stream.message().await.unwrap().is_some());
    client
        .exchange(ExchangeRequest {
            hello: hello(4, 71),
            request: append(1, &"boundedstream ".repeat(1024)),
        })
        .await
        .unwrap();
    let capacity = tokio::time::timeout(Duration::from_secs(5), bounded_stream.message())
        .await
        .unwrap()
        .unwrap_err();
    assert_eq!(capacity.code(), tonic::Code::ResourceExhausted);
    drop(bounded_stream);
    drop(bounded_client);
    let recall = request(
        20,
        RequestPayload::Recall(Box::new(Recall {
            query: b"heliotrope".to_vec(),
            limit: 10,
            mode: RecallMode::ListWindows,
            level: 1,
            start_ns: 0,
            end_ns: 0,
        })),
    );
    let isolated = client
        .exchange(ExchangeRequest {
            hello: hello(5, 11),
            request: recall.clone(),
        })
        .await
        .unwrap()
        .into_inner();
    assert!(
        matches!(verify_wire_envelope(&isolated.ncpr).unwrap().payload,WirePayload::Response(value) if matches!(value.payload,Some(ResponsePayload::RecallResult(ref result)) if result.members.as_ref().unwrap().is_empty()))
    );
    let forbidden = client
        .exchange(ExchangeRequest {
            hello: hello(4, 9),
            request: request(21, RequestPayload::Health(Box::default())),
        })
        .await
        .unwrap_err();
    assert_eq!(forbidden.code(), tonic::Code::PermissionDenied);
    let mut admin_client = grpc(admin_grpc_address, &certs, true).await.unwrap();
    assert_eq!(
        admin_client
            .connect(Envelope { ncpr: hello(4, 9) })
            .await
            .unwrap_err()
            .code(),
        tonic::Code::PermissionDenied
    );
    let admin_health = admin_client
        .exchange(ExchangeRequest {
            hello: hello(3, 12),
            request: request(1, RequestPayload::Health(Box::default())),
        })
        .await
        .unwrap()
        .into_inner();
    assert!(
        matches!(verify_wire_envelope(&admin_health.ncpr).unwrap().payload,WirePayload::Response(value) if matches!(value.payload,Some(ResponsePayload::HealthResult(_))))
    );

    let http = http(&certs, true);
    let openapi: Value = http
        .get(format!("{rest_base}/openapi.json"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    for verb in hm_serve::rest::VERBS {
        assert!(openapi["paths"].get(format!("/v1/{verb}")).is_some());
        assert!(include_str!("../../../schemas/openapi.yaml").contains(&format!("/v1/{verb}:")));
        let (status, error) =
            post(&http, &rest_base, verb, 4, json!("invalid typed arguments")).await;
        assert_eq!(status, reqwest::StatusCode::OK, "{verb}: {error}");
        assert_eq!(error["ok"], false, "{verb}: {error}");
        assert_eq!(
            error["items"][0]["error"], "kInvalidArgument",
            "{verb}: {error}"
        );
        if !matches!(verb, "recall" | "activate" | "inspect") {
            assert!(error["effect_state"].is_string(), "{verb}: {error}");
        }
    }
    let (status, remembered) = post(
        &http,
        &rest_base,
        "remember",
        4,
        json!({"conversation":"remote-rest","kind":"user","content":"rest silverbeam"}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK);
    assert_eq!(remembered["ok"], true, "{remembered}");
    let (_, recalled) = post(
        &http,
        &rest_base,
        "recall",
        4,
        json!({"mode":"lexical","query":"silverbeam"}),
    )
    .await;
    assert_eq!(recalled["ok"], true, "{recalled}");
    assert!(!recalled["items"].as_array().unwrap().is_empty());
    let (_, other_actor) = post(
        &http,
        &rest_base,
        "recall",
        5,
        json!({"mode":"lexical","query":"silverbeam"}),
    )
    .await;
    assert!(other_actor["items"].as_array().unwrap().is_empty());
    let (status, denied) = post(
        &http,
        &rest_base,
        "forget",
        4,
        json!({"action":"crypto_shred","admin_token":"03".repeat(32)}),
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::FORBIDDEN);
    assert_eq!(denied["effect_state"], "not_dispatched");
    let (status, _) = post(&http, &rest_base, "inspect", 3, json!({})).await;
    assert_eq!(status, reqwest::StatusCode::FORBIDDEN);
    client
        .exchange(ExchangeRequest {
            hello: hello(5, 90),
            request: append(1, "temporary actor-eight record to shred"),
        })
        .await
        .unwrap();
    let checkpoint_path = std::fs::read_dir(config.actor_directory(8).join("mmr/checkpoints"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let checkpoint = hm_ledger::checkpoint::load_checkpoint(&checkpoint_path).unwrap();
    let admin_body = json!({"connection_id":"cd".repeat(16),"request_id":1,"actor":8,"arguments":{"action":"crypto_shred"}});
    let denied = http
        .post(format!("{admin_base}/v1/admin/forget"))
        .bearer_auth("04".repeat(32))
        .json(&admin_body)
        .send()
        .await
        .unwrap();
    assert_eq!(denied.status(), reqwest::StatusCode::FORBIDDEN);
    let deleted: Value = http
        .post(format!("{admin_base}/v1/admin/forget"))
        .bearer_auth("03".repeat(32))
        .json(&admin_body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(deleted["ok"], true, "{deleted}");
    let receipt_bytes = base64::engine::general_purpose::STANDARD
        .decode(deleted["items"][0]["receipt_base64"].as_str().unwrap())
        .unwrap();
    let receipt = hm_ledger::shred::decode_deletion_receipt(&receipt_bytes).unwrap();
    hm_ledger::shred::verify_deletion_receipt(&receipt, &checkpoint.public_key).unwrap();
    assert_eq!(receipt.checkpoint_root, checkpoint.root);
    assert!(!config.actor_directory(8).join("keys/KEYRING").exists());
    drop(client);
    drop(admin_client);
    drop(http);
    stop.send(true).unwrap();
    grpc_task.await.unwrap().unwrap();
    bounded_task.await.unwrap().unwrap();
    admin_grpc_task.await.unwrap().unwrap();
    rest_task.await.unwrap().unwrap();
    admin_rest_task.await.unwrap().unwrap();
    daemon_task.await.unwrap().unwrap();

    let mut restarted_config = (*config).clone();
    restarted_config.actors.retain(|actor| actor.actor == 7);
    let restarted_config = Arc::new(restarted_config);
    let daemon = UdsServer::bind((*restarted_config).clone()).await.unwrap();
    let (stop, stopped) = tokio::sync::watch::channel(false);
    let daemon_task = tokio::spawn(daemon.serve_until(shutdown(stopped.clone())));
    let grpc_server = GrpcServer::bind(
        "127.0.0.1:0".parse().unwrap(),
        Gateway::new(restarted_config, ListenerRole::Actor),
        certs.tls.clone(),
    )
    .await
    .unwrap();
    let address = grpc_server.local_addr().unwrap();
    let grpc_task = tokio::spawn(grpc_server.serve_until(shutdown(stopped)));
    let mut client = grpc(address, &certs, true).await.unwrap();
    let welcome = client
        .connect(Envelope { ncpr: hello(4, 9) })
        .await
        .unwrap()
        .into_inner();
    assert!(
        matches!(verify_wire_envelope(&welcome.ncpr).unwrap().payload,WirePayload::Welcome(value) if value.next_client_seq==3)
    );
    let result = client
        .exchange(ExchangeRequest {
            hello: hello(4, 9),
            request: recall,
        })
        .await
        .unwrap()
        .into_inner();
    assert!(
        matches!(verify_wire_envelope(&result.ncpr).unwrap().payload,WirePayload::Response(value) if matches!(value.payload,Some(ResponsePayload::RecallResult(ref result)) if result.members.as_deref()==Some(&[1])))
    );
    drop(client);
    stop.send(true).unwrap();
    grpc_task.await.unwrap().unwrap();
    daemon_task.await.unwrap().unwrap();
}
