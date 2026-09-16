#![forbid(unsafe_code)]

use base64::Engine as _;
use hm_core::ActorId;
use hm_cortex::connectors::{hmac_sha256, signing_base};
use hm_mcp::{BindInput, ConnectorAction, ConnectorInput, InspectInput, McpServer};
use hm_serve::actor::{ActorConfig, ActorEngine, SourceSignatureRequest};
use std::time::{SystemTime, UNIX_EPOCH};

const PROVIDER: &str = "repository-host";
const CONNECTOR_HEX: &str = "4c4c4c4c4c4c4c4c4c4c4c4c4c4c4c4c";
const CONNECTOR_BYTES: [u8; 16] = [0x4c; 16];
const OTHER_CONNECTOR_HEX: &str = "7d7d7d7d7d7d7d7d7d7d7d7d7d7d7d7d";
const EXTERNAL_ACCOUNT: &str = "fleet-engineering";
const SECRET_ONE: &[u8] = b"connector-signing-secret-one";
const SECRET_TWO: &[u8] = b"connector-signing-secret-two";
const SCOPE: &str = "contents.read";

fn actor_config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("7"),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

fn encoded(secret: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(secret)
}

fn now_ns() -> i64 {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    )
    .unwrap()
}

fn connector(action: ConnectorAction, connector_id: &str) -> ConnectorInput {
    ConnectorInput {
        action,
        provider: PROVIDER.to_owned(),
        connector_id: connector_id.to_owned(),
        external_account: None,
        consent_state: None,
        consent_ttl_ns: None,
        credential_version: None,
        signing_secret: None,
        scopes: None,
    }
}

fn request(connector: ConnectorInput) -> BindInput {
    BindInput {
        conversation: "ops".to_owned(),
        task: None,
        scope: None,
        canonical_entity: String::new(),
        property: String::new(),
        evidence_lsn: 0,
        revision: String::new(),
        freshness_requirement_ns: 0,
        connector: Some(connector),
    }
}

fn bind_request(connector_id: &str, version: u32, secret: &[u8], state: String) -> BindInput {
    let mut input = connector(ConnectorAction::Bind, connector_id);
    input.external_account = Some(EXTERNAL_ACCOUNT.to_owned());
    input.consent_state = Some(state);
    input.credential_version = Some(version);
    input.signing_secret = Some(encoded(secret));
    input.scopes = Some(vec![SCOPE.to_owned()]);
    request(input)
}

async fn consent_state(server: &McpServer, connector_id: &str) -> String {
    let envelope = server
        .bind_envelope(request(connector(ConnectorAction::Consent, connector_id)))
        .await;
    assert!(envelope.ok, "{envelope:?}");
    envelope.items[0]["consent_state"]
        .as_str()
        .unwrap()
        .to_owned()
}

async fn bind_connector(server: &McpServer, connector_id: &str) -> u64 {
    let state = consent_state(server, connector_id).await;
    let envelope = server
        .bind_envelope(bind_request(connector_id, 1, SECRET_ONE, state))
        .await;
    assert!(envelope.ok, "{envelope:?}");
    envelope.items[0]["lsn"].as_u64().unwrap()
}

#[tokio::test]
async fn connector_consent_then_bind_appends_one_record() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());

    let before = actor.stats().await.unwrap().log_events;
    let minted = server
        .bind_envelope(request(connector(ConnectorAction::Consent, CONNECTOR_HEX)))
        .await;
    assert!(minted.ok, "{minted:?}");
    assert_eq!(minted.items.len(), 1);
    assert!(minted.effect_state.is_none());
    assert!(minted.provenance.is_empty());
    let state = minted.items[0]["consent_state"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(minted.items[0]["consent_nonce"].as_str().unwrap().len(), 32);
    assert!(minted.items[0]["expires_at_ns"].as_i64().unwrap() > now_ns());
    assert!(state.starts_with(minted.items[0]["consent_nonce"].as_str().unwrap()));
    assert_eq!(actor.stats().await.unwrap().log_events, before);

    let bound = server
        .bind_envelope(bind_request(CONNECTOR_HEX, 1, SECRET_ONE, state))
        .await;
    assert!(bound.ok, "{bound:?}");
    assert_eq!(actor.stats().await.unwrap().log_events, before + 1);
    let lsn = bound.items[0]["lsn"].as_u64().unwrap();
    assert_eq!(bound.provenance, vec![format!("hm://7/lsn/{lsn}")]);
    assert_eq!(bound.items[0]["action"], "bind");
    assert_eq!(bound.items[0]["provider"], PROVIDER);
    assert_eq!(bound.items[0]["connector_id"], CONNECTOR_HEX);
    assert_eq!(bound.items[0]["external_account"], EXTERNAL_ACCOUNT);
    assert_eq!(bound.items[0]["credential_version"], 1);
    assert_eq!(bound.items[0]["scopes"][0], SCOPE);
    assert_eq!(bound.items[0]["state"], "bound");
    assert_eq!(bound.items[0]["signature_scheme"], "hmac_sha256_v0");

    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn connector_consent_state_is_single_use() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());

    let state = consent_state(&server, CONNECTOR_HEX).await;
    let first = server
        .bind_envelope(bind_request(CONNECTOR_HEX, 1, SECRET_ONE, state.clone()))
        .await;
    assert!(first.ok, "{first:?}");

    let replayed = server
        .bind_envelope(bind_request(CONNECTOR_HEX, 2, SECRET_TWO, state))
        .await;
    assert!(!replayed.ok);
    assert_eq!(replayed.items[0]["error"], "kIdempotencyConflict");
    assert_eq!(replayed.effect_state.as_deref(), Some("rejected"));

    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn connector_rotation_is_additive() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());

    bind_connector(&server, CONNECTOR_HEX).await;

    let mut rotation = connector(ConnectorAction::Rotate, CONNECTOR_HEX);
    rotation.credential_version = Some(2);
    rotation.signing_secret = Some(encoded(SECRET_TWO));
    let rotated = server.bind_envelope(request(rotation)).await;
    assert!(rotated.ok, "{rotated:?}");
    assert_eq!(rotated.items[0]["action"], "rotate");
    assert_eq!(rotated.items[0]["credential_version"], 2);
    assert_eq!(rotated.items[0]["state"], "bound");
    assert_eq!(rotated.items[0]["external_account"], EXTERNAL_ACCOUNT);
    assert_eq!(rotated.items[0]["scopes"][0], SCOPE);

    let signed_at_ns = now_ns();
    let body = b"{\"revision\":\"aa11\"}";
    let delivery_id = b"delivery-rotation".to_vec();
    let signature = hmac_sha256(
        SECRET_ONE,
        &signing_base(&CONNECTOR_BYTES, &delivery_id, signed_at_ns, body),
    );
    let verified = actor
        .verify_source_delivery(SourceSignatureRequest {
            connector_id: CONNECTOR_BYTES,
            provider: PROVIDER.to_owned(),
            credential_version: 1,
            delivery_id,
            event_name: "push".to_owned(),
            signed_at_ns,
            body: body.to_vec(),
            signature: signature.to_vec(),
        })
        .await
        .unwrap();
    assert_eq!(verified.body_bytes, u64::try_from(body.len()).unwrap());

    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn connector_revoke_marks_the_registry() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());

    bind_connector(&server, CONNECTOR_HEX).await;
    let revoked = server
        .bind_envelope(request(connector(ConnectorAction::Revoke, CONNECTOR_HEX)))
        .await;
    assert!(revoked.ok, "{revoked:?}");
    assert_eq!(revoked.items[0]["state"], "revoked");
    assert_eq!(revoked.items[0]["credential_version"], 1);

    let records = actor.connectors(16).await.unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].state, 1);
    assert_eq!(records[0].credential_version, 1);

    let again = server
        .bind_envelope(request(connector(ConnectorAction::Revoke, CONNECTOR_HEX)))
        .await;
    assert!(!again.ok);
    assert_eq!(again.items[0]["error"], "kCapabilityDenied");

    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn connector_arguments_fail_closed() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());

    let state = consent_state(&server, CONNECTOR_HEX).await;
    let mut rejected = Vec::new();

    let mut empty_provider = bind_request(CONNECTOR_HEX, 1, SECRET_ONE, state.clone());
    empty_provider.connector.as_mut().unwrap().provider = String::new();
    rejected.push(empty_provider);

    rejected.push(bind_request("4c4c4c", 1, SECRET_ONE, state.clone()));

    let mut zero_version = bind_request(CONNECTOR_HEX, 1, SECRET_ONE, state.clone());
    zero_version.connector.as_mut().unwrap().credential_version = Some(0);
    rejected.push(zero_version);

    let mut empty_secret = bind_request(CONNECTOR_HEX, 1, SECRET_ONE, state.clone());
    empty_secret.connector.as_mut().unwrap().signing_secret = Some(String::new());
    rejected.push(empty_secret);

    let mut empty_scopes = bind_request(CONNECTOR_HEX, 1, SECRET_ONE, state);
    empty_scopes.connector.as_mut().unwrap().scopes = Some(Vec::new());
    rejected.push(empty_scopes);

    for input in rejected {
        let envelope = server.bind_envelope(input).await;
        assert!(!envelope.ok);
        assert_eq!(envelope.items[0]["error"], "kInvalidArgument");
        assert_eq!(envelope.effect_state.as_deref(), Some("not_dispatched"));
    }
    assert_eq!(actor.stats().await.unwrap().log_events, 0);

    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn connector_envelope_never_echoes_secrets() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());

    let state = consent_state(&server, CONNECTOR_HEX).await;
    let tag = state.rsplit('.').next().unwrap().to_owned();
    let secret = encoded(SECRET_ONE);
    let bound = server
        .bind_envelope(bind_request(CONNECTOR_HEX, 1, SECRET_ONE, state.clone()))
        .await;
    assert!(bound.ok, "{bound:?}");

    let rendered = serde_json::to_string(&bound).unwrap();
    assert!(!rendered.contains(&secret));
    assert!(!rendered.contains(&tag));
    assert!(!rendered.contains(&state));
    assert!(!rendered.contains(std::str::from_utf8(SECRET_ONE).unwrap()));

    let mut rotation = connector(ConnectorAction::Rotate, CONNECTOR_HEX);
    rotation.credential_version = Some(2);
    rotation.signing_secret = Some(encoded(SECRET_TWO));
    let rotated = server.bind_envelope(request(rotation)).await;
    assert!(rotated.ok, "{rotated:?}");
    let rendered = serde_json::to_string(&rotated).unwrap();
    assert!(!rendered.contains(&encoded(SECRET_TWO)));
    assert!(!rendered.contains(std::str::from_utf8(SECRET_TWO).unwrap()));

    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn inspect_lists_bound_connectors() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());

    let first = bind_connector(&server, CONNECTOR_HEX).await;
    let second = bind_connector(&server, OTHER_CONNECTOR_HEX).await;

    let envelope = server
        .inspect_envelope(InspectInput {
            uri: Some("hm://7/connectors".to_owned()),
            ..InspectInput::default()
        })
        .await;
    assert!(envelope.ok, "{envelope:?}");
    let listed = envelope.items[0]["connectors"].as_array().unwrap();
    assert_eq!(listed.len(), 2);
    for record in listed {
        assert_eq!(record["provider"], PROVIDER);
        assert_eq!(record["external_account"], EXTERNAL_ACCOUNT);
        assert_eq!(record["state"], "bound");
        assert_eq!(record["credential_version"], 1);
        assert_eq!(record["signature_scheme"], "hmac_sha256_v0");
        assert_eq!(record["scopes"][0], SCOPE);
    }
    assert_eq!(
        envelope.provenance,
        vec![
            format!("hm://7/lsn/{first}"),
            format!("hm://7/lsn/{second}")
        ]
    );

    let walked = server
        .inspect_envelope(InspectInput {
            uri: Some(format!("hm://7/lsn/{first}")),
            ..InspectInput::default()
        })
        .await;
    assert!(walked.ok, "{walked:?}");
    assert_eq!(walked.items.len(), 2);
    assert_eq!(walked.items[1]["lsn"].as_u64().unwrap(), first);
    assert_eq!(walked.items[0]["evidence_path"]["root_lsn"], first);

    actor.shutdown().await.unwrap();
}
