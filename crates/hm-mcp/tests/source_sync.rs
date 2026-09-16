#![forbid(unsafe_code)]

use base64::Engine as _;
use hm_core::{ActorId, LSN};
use hm_cortex::connectors::sync::{SourceRequest, SourceResponse};
use hm_mcp::{
    BindInput, ConnectorAction, ConnectorInput, InspectInput, McpServer, RememberInput,
    RememberKind, SourceRuntime, SourceSyncInput,
};
use hm_schema::events::{Authority, EventPayload};
use hm_serve::actor::{ActorConfig, ActorEngine};
use std::collections::BTreeMap;

const PROVIDER: &str = "repository-host";
const CONNECTOR_HEX: &str = "5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e";
const CONNECTOR_BYTES: [u8; 16] = [0x5e; 16];
const UNBOUND_HEX: &str = "2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b";
const EXTERNAL_ACCOUNT: &str = "fleet-engineering";
const SECRET: &[u8] = b"delivery-signing-secret";
const SCOPE: &str = "contents.read";
const ACCESS_TOKEN: &str = "source-index-access-token";
const BASE_URL: &str = "https://index.invalid/v1";
const FIRST_SOURCE: &str = "crates/hm-mcp";
const SECOND_SOURCE: &str = "docs/reference";
const FIRST_REVISION: &[u8] = b"rev-one";
const SECOND_REVISION: &[u8] = b"rev-two";
const FIRST_DIGEST: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const SECOND_DIGEST: &str = "2222222222222222222222222222222222222222222222222222222222222222";

fn actor_config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("7"),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut output, byte| {
        let _ = write!(output, "{byte:02x}");
        output
    })
}

fn listing_request() -> SourceRequest {
    SourceRequest {
        url: format!("{BASE_URL}/sources"),
        headers: BTreeMap::from([
            ("accept".to_owned(), "application/json".to_owned()),
            ("authorization".to_owned(), format!("Bearer {ACCESS_TOKEN}")),
        ]),
    }
}

fn listing_response() -> SourceResponse {
    let body = serde_json::json!({
        "sources": [
            {
                "source_id": FIRST_SOURCE,
                "revision": String::from_utf8(FIRST_REVISION.to_vec()).unwrap(),
                "content_digest": FIRST_DIGEST,
            },
            {
                "source_id": SECOND_SOURCE,
                "revision": String::from_utf8(SECOND_REVISION.to_vec()).unwrap(),
                "content_digest": SECOND_DIGEST,
            }
        ]
    });
    SourceResponse {
        status: 200,
        body: serde_json::to_vec(&body).unwrap(),
    }
}

fn runtime(pages: usize) -> SourceRuntime {
    let exchanges = (0..pages)
        .map(|_| (listing_request(), listing_response()))
        .collect();
    SourceRuntime::recorded(exchanges, ACCESS_TOKEN)
}

fn sync_input(connector_id: &str) -> SourceSyncInput {
    SourceSyncInput {
        connector_id: connector_id.to_owned(),
        base_url: BASE_URL.to_owned(),
        credential_version: 1,
    }
}

fn remember(source_sync: SourceSyncInput) -> RememberInput {
    RememberInput {
        conversation: "ops".to_owned(),
        content: String::new(),
        kind: RememberKind::Document,
        chunk_bytes: None,
        anchor: None,
        retention: None,
        sensitivity: None,
        vocabulary: None,
        source: None,
        derive: None,
        source_delivery: None,
        source_settlement: None,
        document: None,
        source_sync: Some(source_sync),
    }
}

fn connector(action: ConnectorAction) -> ConnectorInput {
    ConnectorInput {
        action,
        provider: PROVIDER.to_owned(),
        connector_id: CONNECTOR_HEX.to_owned(),
        external_account: None,
        consent_state: None,
        consent_ttl_ns: None,
        credential_version: None,
        signing_secret: None,
        scopes: None,
    }
}

fn bind_input(connector: ConnectorInput) -> BindInput {
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

async fn bind_connector(server: &McpServer) {
    let minted = server
        .bind_envelope(bind_input(connector(ConnectorAction::Consent)))
        .await;
    assert!(minted.ok, "{minted:?}");
    let state = minted.items[0]["consent_state"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut input = connector(ConnectorAction::Bind);
    input.external_account = Some(EXTERNAL_ACCOUNT.to_owned());
    input.consent_state = Some(state);
    input.credential_version = Some(1);
    input.signing_secret = Some(base64::engine::general_purpose::STANDARD.encode(SECRET));
    input.scopes = Some(vec![SCOPE.to_owned()]);
    let bound = server.bind_envelope(bind_input(input)).await;
    assert!(bound.ok, "{bound:?}");
}

#[tokio::test]
async fn source_sync_is_unavailable_without_a_runtime() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());
    bind_connector(&server).await;

    let before = actor.stats().await.unwrap().log_events;
    let envelope = server
        .remember_envelope(remember(sync_input(CONNECTOR_HEX)))
        .await;
    assert!(!envelope.ok, "{envelope:?}");
    assert_eq!(envelope.items[0]["error"], "kOperationUnavailable");
    assert_eq!(envelope.effect_state.as_deref(), Some("not_dispatched"));
    assert_eq!(actor.stats().await.unwrap().log_events, before);

    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn source_sync_records_changed_revisions() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone()).with_source_runtime(runtime(1));
    bind_connector(&server).await;

    let before = actor.stats().await.unwrap().log_events;
    let envelope = server
        .remember_envelope(remember(sync_input(CONNECTOR_HEX)))
        .await;
    assert!(envelope.ok, "{envelope:?}");
    assert_eq!(envelope.items.len(), 2);
    assert_eq!(actor.stats().await.unwrap().log_events, before + 2);

    assert_eq!(envelope.items[0]["source_id"], FIRST_SOURCE);
    assert_eq!(envelope.items[0]["revision"], hex(FIRST_REVISION));
    assert_eq!(envelope.items[0]["content_digest"], FIRST_DIGEST);
    assert_eq!(envelope.items[0]["changed"], true);
    assert_eq!(envelope.items[0]["connector_id"], CONNECTOR_HEX);
    assert_eq!(envelope.items[1]["source_id"], SECOND_SOURCE);
    assert_eq!(envelope.items[1]["revision"], hex(SECOND_REVISION));
    assert_eq!(envelope.items[1]["content_digest"], SECOND_DIGEST);
    assert_eq!(envelope.items[1]["changed"], true);

    let first_lsn = envelope.items[0]["lsn"].as_u64().unwrap();
    let second_lsn = envelope.items[1]["lsn"].as_u64().unwrap();
    assert_eq!(second_lsn, first_lsn + 1);
    assert_eq!(
        envelope.provenance,
        vec![
            format!("hm://7/lsn/{first_lsn}"),
            format!("hm://7/lsn/{second_lsn}")
        ]
    );

    let observed = actor
        .verified_event(LSN::new(first_lsn))
        .await
        .unwrap()
        .envelope;
    assert_eq!(observed.authority, Authority::ExternalObserved);
    match observed.payload {
        EventPayload::SourceRevisionObserved(value) => {
            assert_eq!(value.connector_id, CONNECTOR_BYTES.to_vec());
            assert_eq!(value.source_id, FIRST_SOURCE);
            assert_eq!(value.revision, FIRST_REVISION.to_vec());
            assert_eq!(hex(&value.content_digest), FIRST_DIGEST);
            assert_eq!(value.delivery_lsn, 0);
            assert!(value.observed_at_ns > 0);
        }
        other => panic!("unexpected payload {other:?}"),
    }

    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn source_sync_skips_unchanged_revisions() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone()).with_source_runtime(runtime(2));
    bind_connector(&server).await;

    let first = server
        .remember_envelope(remember(sync_input(CONNECTOR_HEX)))
        .await;
    assert!(first.ok, "{first:?}");
    let first_lsn = first.items[0]["lsn"].as_u64().unwrap();
    let second_lsn = first.items[1]["lsn"].as_u64().unwrap();
    let after_first = actor.stats().await.unwrap().log_events;

    let repeated = server
        .remember_envelope(remember(sync_input(CONNECTOR_HEX)))
        .await;
    assert!(repeated.ok, "{repeated:?}");
    assert_eq!(repeated.items.len(), 2);
    assert_eq!(repeated.items[0]["changed"], false);
    assert_eq!(repeated.items[1]["changed"], false);
    assert_eq!(repeated.items[0]["lsn"].as_u64().unwrap(), first_lsn);
    assert_eq!(repeated.items[1]["lsn"].as_u64().unwrap(), second_lsn);
    assert_eq!(repeated.items[0]["content_digest"], FIRST_DIGEST);
    assert!(repeated.provenance.is_empty(), "{repeated:?}");
    assert_eq!(actor.stats().await.unwrap().log_events, after_first);

    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn source_sync_refuses_a_revoked_connector() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone()).with_source_runtime(runtime(1));
    bind_connector(&server).await;

    let unknown = server
        .remember_envelope(remember(sync_input(UNBOUND_HEX)))
        .await;
    assert!(!unknown.ok, "{unknown:?}");
    assert_eq!(unknown.items[0]["error"], "kCapabilityDenied");

    let revoked = server
        .bind_envelope(bind_input(connector(ConnectorAction::Revoke)))
        .await;
    assert!(revoked.ok, "{revoked:?}");

    let before = actor.stats().await.unwrap().log_events;
    let envelope = server
        .remember_envelope(remember(sync_input(CONNECTOR_HEX)))
        .await;
    assert!(!envelope.ok, "{envelope:?}");
    assert_eq!(envelope.items[0]["error"], "kCapabilityDenied");
    assert_eq!(actor.stats().await.unwrap().log_events, before);

    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn source_sync_never_echoes_the_token() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone()).with_source_runtime(runtime(1));
    bind_connector(&server).await;

    let envelope = server
        .remember_envelope(remember(sync_input(CONNECTOR_HEX)))
        .await;
    assert!(envelope.ok, "{envelope:?}");
    let serialized = serde_json::to_string(&envelope).unwrap();
    assert!(!serialized.contains(ACCESS_TOKEN), "{serialized}");
    assert!(envelope.warnings.is_empty(), "{envelope:?}");

    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn inspect_lists_revision_heads() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone()).with_source_runtime(runtime(1));
    bind_connector(&server).await;

    let synced = server
        .remember_envelope(remember(sync_input(CONNECTOR_HEX)))
        .await;
    assert!(synced.ok, "{synced:?}");
    let first_lsn = synced.items[0]["lsn"].as_u64().unwrap();
    let second_lsn = synced.items[1]["lsn"].as_u64().unwrap();

    let envelope = server
        .inspect_envelope(InspectInput {
            uri: Some("hm://7/revisions".to_owned()),
            ..InspectInput::default()
        })
        .await;
    assert!(envelope.ok, "{envelope:?}");
    let revisions = envelope.items[0]["revisions"].as_array().unwrap();
    assert_eq!(revisions.len(), 2);
    assert_eq!(revisions[0]["source_id"], SECOND_SOURCE);
    assert_eq!(revisions[0]["revision"], hex(SECOND_REVISION));
    assert_eq!(revisions[0]["content_digest"], SECOND_DIGEST);
    assert_eq!(revisions[0]["lsn"].as_u64().unwrap(), second_lsn);
    assert!(revisions[0]["observed_at_ns"].as_i64().unwrap() > 0);
    assert_eq!(revisions[1]["source_id"], FIRST_SOURCE);
    assert_eq!(revisions[1]["lsn"].as_u64().unwrap(), first_lsn);
    assert_eq!(
        envelope.provenance,
        vec![
            format!("hm://7/lsn/{second_lsn}"),
            format!("hm://7/lsn/{first_lsn}")
        ]
    );

    actor.shutdown().await.unwrap();
}
