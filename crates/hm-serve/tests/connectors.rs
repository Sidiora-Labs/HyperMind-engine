#![forbid(unsafe_code)]
#![allow(clippy::too_many_lines)]

use hm_core::{ConversationId, ErrorCode};
use hm_cortex::connectors::{hmac_sha256, signing_base};
use hm_ledger::frame::EventKind;
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    Authority, ConnectorState, EventEnvelope, EventPayload, Retention, Sensitivity,
    SourceConnectorBound, SourceDeliveryAccepted, SourceDeliverySettled, SourceDeliveryState,
    SourceRevisionObserved, SourceSignatureScheme,
};
use hm_serve::actor::{ActorConfig, ActorEngine, IncomingEvent, SourceSignatureRequest};
use std::time::{SystemTime, UNIX_EPOCH};

const CONNECTOR_ID: [u8; 16] = [0x4c; 16];
const OTHER_CONNECTOR_ID: [u8; 16] = [0x7d; 16];
const PROVIDER: &str = "repository-host";
const CREDENTIAL_VERSION: u32 = 1;
const CONSENT_TTL_NS: i64 = 600_000_000_000;

fn actor_config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("7"),
        actor: hm_core::ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

fn event(kind: EventKind, authority: Authority, payload: EventPayload) -> IncomingEvent {
    IncomingEvent {
        kind,
        conversation: ConversationId::derive("source-connectors"),
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

fn bound(connector_id: [u8; 16], nonce: [u8; 16], state: ConnectorState) -> IncomingEvent {
    event(
        EventKind::SourceConnectorBound,
        Authority::UserAsserted,
        EventPayload::SourceConnectorBound(Box::new(SourceConnectorBound {
            connector_id: connector_id.to_vec(),
            provider: PROVIDER.to_owned(),
            external_account: "fleet-engineering".to_owned(),
            consent_nonce: nonce.to_vec(),
            consent_expires_at_ns: 1_700_000_000_000_000_000,
            credential_version: CREDENTIAL_VERSION,
            signature_scheme: SourceSignatureScheme::HmacSha256V0,
            scopes: vec!["contents:read".to_owned()],
            state,
        })),
    )
}

fn accepted(connector_id: [u8; 16], delivery_id: &[u8]) -> IncomingEvent {
    event(
        EventKind::SourceDeliveryAccepted,
        Authority::ExternalObserved,
        EventPayload::SourceDeliveryAccepted(Box::new(SourceDeliveryAccepted {
            connector_id: connector_id.to_vec(),
            delivery_id: delivery_id.to_vec(),
            signature_scheme: SourceSignatureScheme::HmacSha256V0,
            credential_version: CREDENTIAL_VERSION,
            signed_at_ns: 1_699_000_000_000_000_000,
            body_digest: vec![0x5b; 32],
            body_bytes: 64,
            event_name: "push".to_owned(),
        })),
    )
}

fn settled(delivery_id: &[u8], accepted_lsn: u64, attempt: u32) -> IncomingEvent {
    event(
        EventKind::SourceDeliverySettled,
        Authority::RuntimeFact,
        EventPayload::SourceDeliverySettled(Box::new(SourceDeliverySettled {
            connector_id: CONNECTOR_ID.to_vec(),
            delivery_id: delivery_id.to_vec(),
            accepted_lsn,
            attempt,
            state: SourceDeliveryState::Applied,
            next_attempt_at_ns: 0,
            detail: "applied".to_owned(),
        })),
    )
}

fn observed(source_id: &str, revision: &[u8]) -> IncomingEvent {
    event(
        EventKind::SourceRevisionObserved,
        Authority::ExternalObserved,
        EventPayload::SourceRevisionObserved(Box::new(SourceRevisionObserved {
            connector_id: CONNECTOR_ID.to_vec(),
            source_id: source_id.to_owned(),
            revision: revision.to_vec(),
            content_digest: vec![0x3a; 32],
            observed_at_ns: 1_699_000_000_000_000_001,
            delivery_lsn: 0,
        })),
    )
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

async fn bind_connector(engine: &ActorEngine, connector_id: [u8; 16]) -> [u8; 16] {
    let mint = engine
        .mint_consent_state(PROVIDER.to_owned(), connector_id, CONSENT_TTL_NS)
        .await
        .unwrap();
    let grant = engine
        .redeem_consent_state(PROVIDER.to_owned(), connector_id, mint.state.clone())
        .await
        .unwrap();
    assert_eq!(grant.nonce, mint.nonce);
    assert_eq!(grant.expires_at_ns, mint.expires_at_ns);
    engine
        .append(vec![bound(
            connector_id,
            grant.nonce,
            ConnectorState::Bound,
        )])
        .await
        .unwrap();
    grant.nonce
}

#[tokio::test]
async fn consent_state_is_single_use() {
    let temporary = tempfile::tempdir().unwrap();
    let engine = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();

    let mint = engine
        .mint_consent_state(PROVIDER.to_owned(), CONNECTOR_ID, CONSENT_TTL_NS)
        .await
        .unwrap();
    assert!(mint.state.contains('.'));
    assert_eq!(
        engine
            .redeem_consent_state(
                "workspace-host".to_owned(),
                CONNECTOR_ID,
                mint.state.clone()
            )
            .await
            .unwrap_err()
            .code,
        ErrorCode::CapabilityDenied
    );
    let expired = engine
        .mint_consent_state(PROVIDER.to_owned(), CONNECTOR_ID, 1)
        .await
        .unwrap();
    assert_eq!(
        engine
            .redeem_consent_state(PROVIDER.to_owned(), CONNECTOR_ID, expired.state)
            .await
            .unwrap_err()
            .code,
        ErrorCode::CapabilityDenied
    );

    let grant = engine
        .redeem_consent_state(PROVIDER.to_owned(), CONNECTOR_ID, mint.state)
        .await
        .unwrap();
    engine
        .append(vec![bound(
            CONNECTOR_ID,
            grant.nonce,
            ConnectorState::Bound,
        )])
        .await
        .unwrap();
    let after_bind = engine.stats().await.unwrap().log_events;

    assert_eq!(
        engine
            .append(vec![bound(
                OTHER_CONNECTOR_ID,
                grant.nonce,
                ConnectorState::Bound
            )])
            .await
            .unwrap_err()
            .code,
        ErrorCode::IdempotencyConflict
    );
    assert_eq!(engine.stats().await.unwrap().log_events, after_bind);
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn delivery_inbox_is_idempotent_and_authorized() {
    let temporary = tempfile::tempdir().unwrap();
    let engine = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();

    assert_eq!(
        engine
            .append(vec![accepted(CONNECTOR_ID, b"delivery-0001")])
            .await
            .unwrap_err()
            .code,
        ErrorCode::CapabilityDenied
    );

    bind_connector(&engine, CONNECTOR_ID).await;
    engine
        .append(vec![accepted(CONNECTOR_ID, b"delivery-0001")])
        .await
        .unwrap();
    assert_eq!(
        engine
            .append(vec![accepted(CONNECTOR_ID, b"delivery-0001")])
            .await
            .unwrap_err()
            .code,
        ErrorCode::IdempotencyConflict
    );
    assert_eq!(
        engine
            .append(vec![
                accepted(CONNECTOR_ID, b"delivery-0002"),
                accepted(CONNECTOR_ID, b"delivery-0002"),
            ])
            .await
            .unwrap_err()
            .code,
        ErrorCode::IdempotencyConflict
    );

    let deliveries = engine.source_deliveries(CONNECTOR_ID, 8).await.unwrap();
    assert_eq!(deliveries.len(), 1);
    assert_eq!(deliveries[0].delivery_id, b"delivery-0001".to_vec());
    assert_eq!(
        engine
            .source_deliveries(CONNECTOR_ID, 0)
            .await
            .unwrap_err()
            .code,
        ErrorCode::InvalidArgument
    );
    assert_eq!(
        engine.connectors(0).await.unwrap_err().code,
        ErrorCode::InvalidArgument
    );
    assert_eq!(
        engine
            .source_revisions(CONNECTOR_ID, 0)
            .await
            .unwrap_err()
            .code,
        ErrorCode::InvalidArgument
    );

    let revoke_nonce = {
        let mint = engine
            .mint_consent_state(PROVIDER.to_owned(), CONNECTOR_ID, CONSENT_TTL_NS)
            .await
            .unwrap();
        engine
            .redeem_consent_state(PROVIDER.to_owned(), CONNECTOR_ID, mint.state)
            .await
            .unwrap()
            .nonce
    };
    engine
        .append(vec![bound(
            CONNECTOR_ID,
            revoke_nonce,
            ConnectorState::Revoked,
        )])
        .await
        .unwrap();
    assert_eq!(
        engine
            .append(vec![accepted(CONNECTOR_ID, b"delivery-0003")])
            .await
            .unwrap_err()
            .code,
        ErrorCode::CapabilityDenied
    );
    assert_eq!(
        engine
            .append(vec![observed("engine/src/main.rs", b"3f1c")])
            .await
            .unwrap_err()
            .code,
        ErrorCode::CapabilityDenied
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn settlement_requires_an_accepted_delivery_and_advancing_attempt() {
    let temporary = tempfile::tempdir().unwrap();
    let engine = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    bind_connector(&engine, CONNECTOR_ID).await;

    assert_eq!(
        engine
            .append(vec![settled(b"delivery-0001", 1, 1)])
            .await
            .unwrap_err()
            .code,
        ErrorCode::OrderingViolation
    );

    let committed = engine
        .append(vec![accepted(CONNECTOR_ID, b"delivery-0001")])
        .await
        .unwrap();
    engine
        .append(vec![settled(b"delivery-0001", committed.last_lsn.get(), 1)])
        .await
        .unwrap();
    assert_eq!(
        engine
            .append(vec![settled(b"delivery-0001", committed.last_lsn.get(), 1)])
            .await
            .unwrap_err()
            .code,
        ErrorCode::OrderingViolation
    );

    let deliveries = engine.source_deliveries(CONNECTOR_ID, 8).await.unwrap();
    assert_eq!(deliveries[0].attempt, 1);
    assert_eq!(deliveries[0].state, u8::from(SourceDeliveryState::Applied));
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn credential_secrets_never_leave_the_writer() {
    let temporary = tempfile::tempdir().unwrap();
    let engine = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let secret = b"connector-shared-secret".to_vec();
    engine
        .store_connector_credential(
            PROVIDER.to_owned(),
            CONNECTOR_ID,
            CREDENTIAL_VERSION,
            secret.clone(),
        )
        .await
        .unwrap();
    assert_eq!(
        engine
            .store_connector_credential(
                PROVIDER.to_owned(),
                CONNECTOR_ID,
                CREDENTIAL_VERSION,
                secret.clone(),
            )
            .await
            .unwrap_err()
            .code,
        ErrorCode::AlreadyExists
    );

    let delivery_id = b"delivery-0001".to_vec();
    let body = b"{\"ref\":\"refs/heads/main\"}".to_vec();
    let signed_at_ns = now_ns();
    let signature = hmac_sha256(
        &secret,
        &signing_base(&CONNECTOR_ID, &delivery_id, signed_at_ns, &body),
    )
    .to_vec();
    let request = SourceSignatureRequest {
        connector_id: CONNECTOR_ID,
        provider: PROVIDER.to_owned(),
        credential_version: CREDENTIAL_VERSION,
        delivery_id,
        event_name: "push".to_owned(),
        signed_at_ns,
        body: body.clone(),
        signature,
    };
    let verified = engine
        .verify_source_delivery(request.clone())
        .await
        .unwrap();
    assert_eq!(verified.body_digest, *blake3::hash(&body).as_bytes());
    assert_eq!(verified.body_bytes, u64::try_from(body.len()).unwrap());

    let mut tampered = request.clone();
    tampered.body[0] ^= 0x01;
    assert_eq!(
        engine
            .verify_source_delivery(tampered)
            .await
            .unwrap_err()
            .code,
        ErrorCode::SignatureInvalid
    );

    let mut unknown_version = request;
    unknown_version.credential_version = CREDENTIAL_VERSION + 1;
    assert_eq!(
        engine
            .verify_source_delivery(unknown_version)
            .await
            .unwrap_err()
            .code,
        ErrorCode::InvalidArgument
    );
    engine.shutdown().await.unwrap();
}

#[tokio::test]
async fn registry_survives_restart() {
    let temporary = tempfile::tempdir().unwrap();
    let engine = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let nonce = bind_connector(&engine, CONNECTOR_ID).await;
    engine
        .append(vec![accepted(CONNECTOR_ID, b"delivery-0001")])
        .await
        .unwrap();
    engine
        .append(vec![observed("engine/src/main.rs", b"3f1c")])
        .await
        .unwrap();
    let connectors = engine.connectors(8).await.unwrap();
    let deliveries = engine.source_deliveries(CONNECTOR_ID, 8).await.unwrap();
    let revisions = engine.source_revisions(CONNECTOR_ID, 8).await.unwrap();
    assert_eq!(connectors.len(), 1);
    assert_eq!(deliveries.len(), 1);
    assert_eq!(revisions.len(), 1);
    engine.shutdown().await.unwrap();

    let reopened = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    assert_eq!(reopened.connectors(8).await.unwrap(), connectors);
    assert_eq!(
        reopened.source_deliveries(CONNECTOR_ID, 8).await.unwrap(),
        deliveries
    );
    assert_eq!(
        reopened.source_revisions(CONNECTOR_ID, 8).await.unwrap(),
        revisions
    );
    assert_eq!(
        reopened
            .append(vec![bound(CONNECTOR_ID, nonce, ConnectorState::Bound)])
            .await
            .unwrap_err()
            .code,
        ErrorCode::IdempotencyConflict
    );
    reopened.shutdown().await.unwrap();
}
