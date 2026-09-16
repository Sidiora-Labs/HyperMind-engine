#![forbid(unsafe_code)]

use base64::Engine as _;
use hm_core::{ActorId, LSN};
use hm_cortex::connectors::{MAXIMUM_DELIVERY_BYTES, hmac_sha256, retry_delay_ns, signing_base};
use hm_mcp::{
    BindInput, ConnectorAction, ConnectorInput, InspectInput, McpServer, RememberInput,
    RememberKind, SourceDeliveryInput, SourceOutcome, SourceSettlementInput,
};
use hm_schema::events::{Authority, EventPayload};
use hm_serve::actor::{ActorConfig, ActorEngine};
use std::time::{SystemTime, UNIX_EPOCH};

const PROVIDER: &str = "repository-host";
const CONNECTOR_HEX: &str = "4c4c4c4c4c4c4c4c4c4c4c4c4c4c4c4c";
const CONNECTOR_BYTES: [u8; 16] = [0x4c; 16];
const UNBOUND_HEX: &str = "7d7d7d7d7d7d7d7d7d7d7d7d7d7d7d7d";
const EXTERNAL_ACCOUNT: &str = "fleet-engineering";
const SECRET_ONE: &[u8] = b"delivery-signing-secret-one";
const SECRET_TWO: &[u8] = b"delivery-signing-secret-two";
const SCOPE: &str = "contents.read";
const EVENT_NAME: &str = "revision.pushed";
const BODY: &[u8] = b"{\"revision\":\"aa11bb22\",\"paths\":[\"crates/hm-mcp/src/lib.rs\"]}";
const STALE_SKEW_NS: i64 = 900_000_000_000;

fn actor_config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("7"),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
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

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut output, byte| {
        let _ = write!(output, "{byte:02x}");
        output
    })
}

fn signature(secret: &[u8], delivery_id: &str, signed_at_ns: i64, body: &[u8]) -> String {
    hex(&hmac_sha256(
        secret,
        &signing_base(&CONNECTOR_BYTES, delivery_id.as_bytes(), signed_at_ns, body),
    ))
}

fn delivery(
    delivery_id: &str,
    signed_at_ns: i64,
    body: &[u8],
    secret: &[u8],
) -> SourceDeliveryInput {
    SourceDeliveryInput {
        connector_id: CONNECTOR_HEX.to_owned(),
        delivery_id: delivery_id.to_owned(),
        event_name: EVENT_NAME.to_owned(),
        credential_version: 1,
        signed_at_ns,
        signature: signature(secret, delivery_id, signed_at_ns, body),
        body_base64: base64::engine::general_purpose::STANDARD.encode(body),
        conversation: None,
    }
}

fn remember(
    source_delivery: Option<SourceDeliveryInput>,
    source_settlement: Option<SourceSettlementInput>,
) -> RememberInput {
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
        source_delivery,
        source_settlement,
        document: None,
    }
}

fn settlement(delivery_id: &str, attempt: u32, outcome: SourceOutcome) -> SourceSettlementInput {
    SourceSettlementInput {
        connector_id: CONNECTOR_HEX.to_owned(),
        delivery_id: delivery_id.to_owned(),
        attempt,
        outcome,
        detail: None,
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
    input.signing_secret = Some(base64::engine::general_purpose::STANDARD.encode(SECRET_ONE));
    input.scopes = Some(vec![SCOPE.to_owned()]);
    let bound = server.bind_envelope(bind_input(input)).await;
    assert!(bound.ok, "{bound:?}");
}

#[tokio::test]
async fn signed_delivery_is_admitted_with_body_and_digest() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());
    bind_connector(&server).await;

    let before = actor.stats().await.unwrap().log_events;
    let envelope = server
        .remember_envelope(remember(
            Some(delivery("delivery-one", now_ns(), BODY, SECRET_ONE)),
            None,
        ))
        .await;
    assert!(envelope.ok, "{envelope:?}");
    assert_eq!(actor.stats().await.unwrap().log_events, before + 2);

    let accepted_lsn = envelope.items[0]["accepted_lsn"].as_u64().unwrap();
    let body_lsn = envelope.items[0]["body_lsn"].as_u64().unwrap();
    assert_eq!(body_lsn, accepted_lsn + 1);
    assert_eq!(envelope.items[0]["duplicate"], false);
    assert_eq!(envelope.items[0]["delivery_id"], "delivery-one");
    assert_eq!(envelope.items[0]["event_name"], EVENT_NAME);
    assert_eq!(envelope.items[0]["state"], "accepted");
    assert_eq!(
        envelope.items[0]["body_digest"].as_str().unwrap(),
        hex(blake3::hash(BODY).as_bytes())
    );
    assert_eq!(
        envelope.items[0]["body_bytes"].as_u64().unwrap(),
        u64::try_from(BODY.len()).unwrap()
    );
    assert_eq!(
        envelope.provenance,
        vec![
            format!("hm://7/lsn/{accepted_lsn}"),
            format!("hm://7/lsn/{body_lsn}")
        ]
    );

    let accepted = actor
        .verified_event(LSN::new(accepted_lsn))
        .await
        .unwrap()
        .envelope;
    assert_eq!(accepted.authority, Authority::ExternalObserved);
    match accepted.payload {
        EventPayload::SourceDeliveryAccepted(value) => {
            assert_eq!(value.connector_id, CONNECTOR_BYTES.to_vec());
            assert_eq!(value.delivery_id, b"delivery-one".to_vec());
            assert_eq!(value.body_digest, blake3::hash(BODY).as_bytes().to_vec());
            assert_eq!(value.body_bytes, u64::try_from(BODY.len()).unwrap());
            assert_eq!(value.credential_version, 1);
            assert_eq!(value.event_name, EVENT_NAME);
        }
        other => panic!("unexpected accepted payload {other:?}"),
    }

    let body = actor
        .verified_event(LSN::new(body_lsn))
        .await
        .unwrap()
        .envelope;
    assert_eq!(body.authority, Authority::ExternalObserved);
    match body.payload {
        EventPayload::ProviderFrame(value) => {
            assert_eq!(value.provider, PROVIDER);
            assert_eq!(value.api_content, BODY.to_vec());
        }
        other => panic!("unexpected body payload {other:?}"),
    }

    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn repeat_delivery_is_a_duplicate_not_an_error() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());
    bind_connector(&server).await;

    let signed_at_ns = now_ns();
    let first = server
        .remember_envelope(remember(
            Some(delivery("delivery-repeat", signed_at_ns, BODY, SECRET_ONE)),
            None,
        ))
        .await;
    assert!(first.ok, "{first:?}");
    let accepted_lsn = first.items[0]["accepted_lsn"].as_u64().unwrap();
    let after_first = actor.stats().await.unwrap().log_events;

    let repeated = server
        .remember_envelope(remember(
            Some(delivery("delivery-repeat", signed_at_ns, BODY, SECRET_ONE)),
            None,
        ))
        .await;
    assert!(repeated.ok, "{repeated:?}");
    assert_eq!(repeated.items[0]["duplicate"], true);
    assert_eq!(
        repeated.items[0]["accepted_lsn"].as_u64().unwrap(),
        accepted_lsn
    );
    assert_eq!(
        repeated.items[0]["body_digest"].as_str().unwrap(),
        hex(blake3::hash(BODY).as_bytes())
    );
    assert_eq!(
        repeated.provenance,
        vec![format!("hm://7/lsn/{accepted_lsn}")]
    );
    assert_eq!(actor.stats().await.unwrap().log_events, after_first);

    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn delivery_fails_closed() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());
    bind_connector(&server).await;

    let mut rotation = connector(ConnectorAction::Rotate);
    rotation.credential_version = Some(2);
    rotation.signing_secret = Some(base64::engine::general_purpose::STANDARD.encode(SECRET_TWO));
    let rotated = server.bind_envelope(bind_input(rotation)).await;
    assert!(rotated.ok, "{rotated:?}");
    let before = actor.stats().await.unwrap().log_events;

    let signed_at_ns = now_ns();
    let mut tampered_body = delivery("delivery-tampered-body", signed_at_ns, BODY, SECRET_ONE);
    tampered_body.body_base64 =
        base64::engine::general_purpose::STANDARD.encode(b"{\"revision\":\"forged\"}");

    let mut tampered_signature = delivery(
        "delivery-tampered-signature",
        signed_at_ns,
        BODY,
        SECRET_ONE,
    );
    let flipped = if tampered_signature.signature.starts_with('a') {
        format!("b{}", &tampered_signature.signature[1..])
    } else {
        format!("a{}", &tampered_signature.signature[1..])
    };
    tampered_signature.signature = flipped;

    let mut wrong_version = delivery("delivery-wrong-version", signed_at_ns, BODY, SECRET_ONE);
    wrong_version.credential_version = 2;

    let stale = delivery(
        "delivery-stale",
        signed_at_ns - STALE_SKEW_NS,
        BODY,
        SECRET_ONE,
    );

    let mut unbound = delivery("delivery-unbound", signed_at_ns, BODY, SECRET_ONE);
    unbound.connector_id = UNBOUND_HEX.to_owned();

    let oversized_body = vec![b'x'; MAXIMUM_DELIVERY_BYTES + 1];
    let oversized = delivery(
        "delivery-oversized",
        signed_at_ns,
        &oversized_body,
        SECRET_ONE,
    );

    let expected = [
        (tampered_body, "kSignatureInvalid", "not_dispatched"),
        (tampered_signature, "kSignatureInvalid", "not_dispatched"),
        (wrong_version, "kSignatureInvalid", "not_dispatched"),
        (stale, "kOrderingViolation", "rejected"),
        (unbound, "kCapabilityDenied", "not_dispatched"),
        (oversized, "kInvalidArgument", "not_dispatched"),
    ];
    for (input, code, effect_state) in expected {
        let envelope = server.remember_envelope(remember(Some(input), None)).await;
        assert!(!envelope.ok, "{envelope:?}");
        assert_eq!(envelope.items[0]["error"], code);
        assert_eq!(envelope.effect_state.as_deref(), Some(effect_state));
    }
    assert_eq!(actor.stats().await.unwrap().log_events, before);

    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn settlement_records_attempt_and_backoff() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());
    bind_connector(&server).await;

    let applied_delivery = server
        .remember_envelope(remember(
            Some(delivery("delivery-applied", now_ns(), BODY, SECRET_ONE)),
            None,
        ))
        .await;
    assert!(applied_delivery.ok, "{applied_delivery:?}");
    let accepted_lsn = applied_delivery.items[0]["accepted_lsn"].as_u64().unwrap();

    let before = actor.stats().await.unwrap().log_events;
    let applied = server
        .remember_envelope(remember(
            None,
            Some(settlement("delivery-applied", 1, SourceOutcome::Applied)),
        ))
        .await;
    assert!(applied.ok, "{applied:?}");
    assert_eq!(actor.stats().await.unwrap().log_events, before + 1);
    assert_eq!(applied.items[0]["state"], "applied");
    assert_eq!(applied.items[0]["attempt"], 1);
    assert_eq!(applied.items[0]["next_attempt_at_ns"], 0);
    assert_eq!(
        applied.items[0]["accepted_lsn"].as_u64().unwrap(),
        accepted_lsn
    );
    let settled_lsn = applied.items[0]["settled_lsn"].as_u64().unwrap();
    assert_eq!(
        applied.provenance,
        vec![format!("hm://7/lsn/{settled_lsn}")]
    );

    let failing = server
        .remember_envelope(remember(
            Some(delivery("delivery-failing", now_ns(), BODY, SECRET_ONE)),
            None,
        ))
        .await;
    assert!(failing.ok, "{failing:?}");

    let opened = now_ns();
    let failed = server
        .remember_envelope(remember(
            None,
            Some(settlement("delivery-failing", 1, SourceOutcome::Failed)),
        ))
        .await;
    let closed = now_ns();
    assert!(failed.ok, "{failed:?}");
    assert_eq!(failed.items[0]["state"], "failed");
    let next_attempt_at_ns = failed.items[0]["next_attempt_at_ns"].as_i64().unwrap();
    assert!(next_attempt_at_ns >= opened + retry_delay_ns(1));
    assert!(next_attempt_at_ns <= closed + retry_delay_ns(1));

    let abandoned = server
        .remember_envelope(remember(
            None,
            Some(settlement("delivery-failing", 6, SourceOutcome::Failed)),
        ))
        .await;
    assert!(abandoned.ok, "{abandoned:?}");
    assert_eq!(abandoned.items[0]["state"], "abandoned");
    assert_eq!(abandoned.items[0]["attempt"], 6);
    assert_eq!(abandoned.items[0]["next_attempt_at_ns"], 0);

    let records = actor.source_deliveries(CONNECTOR_BYTES, 16).await.unwrap();
    let held = records
        .iter()
        .find(|record| record.delivery_id == b"delivery-failing".to_vec())
        .unwrap();
    assert_eq!(held.attempt, 6);
    assert_eq!(held.state, 3);
    assert_eq!(held.detail, "abandoned");

    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn inspect_lists_the_inbox() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());
    bind_connector(&server).await;

    let first = server
        .remember_envelope(remember(
            Some(delivery("delivery-first", now_ns(), BODY, SECRET_ONE)),
            None,
        ))
        .await;
    assert!(first.ok, "{first:?}");
    let second = server
        .remember_envelope(remember(
            Some(delivery("delivery-second", now_ns(), BODY, SECRET_ONE)),
            None,
        ))
        .await;
    assert!(second.ok, "{second:?}");
    let settled = server
        .remember_envelope(remember(
            None,
            Some(settlement("delivery-first", 1, SourceOutcome::Applied)),
        ))
        .await;
    assert!(settled.ok, "{settled:?}");

    let first_lsn = first.items[0]["accepted_lsn"].as_u64().unwrap();
    let second_lsn = second.items[0]["accepted_lsn"].as_u64().unwrap();

    let envelope = server
        .inspect_envelope(InspectInput {
            uri: Some("hm://7/inbox".to_owned()),
            ..InspectInput::default()
        })
        .await;
    assert!(envelope.ok, "{envelope:?}");
    let listed = envelope.items[0]["inbox"].as_array().unwrap();
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0]["delivery_id"], hex(b"delivery-second"));
    assert_eq!(listed[0]["state"], "accepted");
    assert_eq!(listed[0]["attempt"], 0);
    assert_eq!(listed[0]["next_attempt_at_ns"], 0);
    assert_eq!(listed[0]["accepted_lsn"].as_u64().unwrap(), second_lsn);
    assert_eq!(listed[0]["event_name"], EVENT_NAME);
    assert_eq!(listed[0]["connector_id"], CONNECTOR_HEX);
    assert_eq!(listed[1]["delivery_id"], hex(b"delivery-first"));
    assert_eq!(listed[1]["state"], "applied");
    assert_eq!(listed[1]["attempt"], 1);
    assert_eq!(listed[1]["accepted_lsn"].as_u64().unwrap(), first_lsn);
    assert_eq!(
        envelope.provenance,
        vec![
            format!("hm://7/lsn/{second_lsn}"),
            format!("hm://7/lsn/{first_lsn}")
        ]
    );

    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn inbox_survives_restart() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());
    bind_connector(&server).await;

    let signed_at_ns = now_ns();
    let admitted = server
        .remember_envelope(remember(
            Some(delivery("delivery-durable", signed_at_ns, BODY, SECRET_ONE)),
            None,
        ))
        .await;
    assert!(admitted.ok, "{admitted:?}");
    let accepted_lsn = admitted.items[0]["accepted_lsn"].as_u64().unwrap();
    let events = actor.stats().await.unwrap().log_events;
    actor.shutdown().await.unwrap();

    let reopened = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(reopened.clone());
    let repeated = server
        .remember_envelope(remember(
            Some(delivery("delivery-durable", signed_at_ns, BODY, SECRET_ONE)),
            None,
        ))
        .await;
    assert!(repeated.ok, "{repeated:?}");
    assert_eq!(repeated.items[0]["duplicate"], true);
    assert_eq!(
        repeated.items[0]["accepted_lsn"].as_u64().unwrap(),
        accepted_lsn
    );
    assert_eq!(reopened.stats().await.unwrap().log_events, events);

    reopened.shutdown().await.unwrap();
}
