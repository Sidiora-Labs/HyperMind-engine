#![forbid(unsafe_code)]
#![allow(clippy::too_many_lines)]

use hm_core::{ActorId, ConversationId, ErrorCode, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_proj::connectors::{ConnectorRegistryProjection, MAXIMUM_REGISTRY_SCAN};
use hm_proj::rebuild::rebuild_projection_stream;
use hm_proj::store::{ProjectionId, ProjectionStore};
use hm_schema::event::encode_event_envelope;
use hm_schema::events::{
    Authority, ConnectorState, EventEnvelope, EventPayload, Retention, Sensitivity,
    SourceConnectorBound, SourceDeliveryAccepted, SourceDeliverySettled, SourceDeliveryState,
    SourceRevisionObserved, SourceSignatureScheme, UserMsg,
};
use tempfile::tempdir;

const CONNECTOR_ID: [u8; 16] = [0x4c; 16];
const FIRST_NONCE: [u8; 16] = [0x11; 16];
const SECOND_NONCE: [u8; 16] = [0x22; 16];
const FIRST_DELIVERY: &[u8] = b"delivery-0001";
const SECOND_DELIVERY: &[u8] = b"delivery-0002";
const SOURCE_ID: &str = "engine/src/main.rs";

fn frame(lsn: u64, kind: EventKind, authority: Authority, payload: EventPayload) -> Frame {
    Frame {
        header: FrameHeader {
            lsn: LSN::new(lsn),
            kind,
            wall_timestamp_ns: UtcNanos::new(i64::try_from(lsn).unwrap() * 1_000),
            actor: ActorId::new(41),
            conversation: ConversationId::new([0x63; 16]),
        },
        sealed_payload: encode_event_envelope(&EventEnvelope {
            schema_version: 2,
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
            event_time_ns: i64::try_from(lsn).unwrap() * 1_000,
        }),
    }
}

fn bound(lsn: u64, nonce: [u8; 16], state: ConnectorState) -> Frame {
    frame(
        lsn,
        EventKind::SourceConnectorBound,
        Authority::UserAsserted,
        EventPayload::SourceConnectorBound(Box::new(SourceConnectorBound {
            connector_id: CONNECTOR_ID.to_vec(),
            provider: "repository-host".to_owned(),
            external_account: "fleet-engineering".to_owned(),
            consent_nonce: nonce.to_vec(),
            consent_expires_at_ns: 1_700_000_000_000_000_000,
            credential_version: 3,
            signature_scheme: SourceSignatureScheme::HmacSha256V0,
            scopes: vec!["contents:read".to_owned(), "metadata:read".to_owned()],
            state,
        })),
    )
}

fn accepted(lsn: u64, delivery_id: &[u8], body_bytes: u64) -> Frame {
    frame(
        lsn,
        EventKind::SourceDeliveryAccepted,
        Authority::ExternalObserved,
        EventPayload::SourceDeliveryAccepted(Box::new(SourceDeliveryAccepted {
            connector_id: CONNECTOR_ID.to_vec(),
            delivery_id: delivery_id.to_vec(),
            signature_scheme: SourceSignatureScheme::HmacSha256V0,
            credential_version: 3,
            signed_at_ns: 1_699_000_000_000_000_000,
            body_digest: vec![0x5b; 32],
            body_bytes,
            event_name: "push".to_owned(),
        })),
    )
}

fn settled(lsn: u64, delivery_id: &[u8], accepted_lsn: u64) -> Frame {
    frame(
        lsn,
        EventKind::SourceDeliverySettled,
        Authority::RuntimeFact,
        EventPayload::SourceDeliverySettled(Box::new(SourceDeliverySettled {
            connector_id: CONNECTOR_ID.to_vec(),
            delivery_id: delivery_id.to_vec(),
            accepted_lsn,
            attempt: 2,
            state: SourceDeliveryState::Applied,
            next_attempt_at_ns: 1_699_000_000_000_000_500,
            detail: "ingested 4 revisions".to_owned(),
        })),
    )
}

fn observed(lsn: u64, revision: &[u8], digest: u8) -> Frame {
    frame(
        lsn,
        EventKind::SourceRevisionObserved,
        Authority::ExternalObserved,
        EventPayload::SourceRevisionObserved(Box::new(SourceRevisionObserved {
            connector_id: CONNECTOR_ID.to_vec(),
            source_id: SOURCE_ID.to_owned(),
            revision: revision.to_vec(),
            content_digest: vec![digest; 32],
            observed_at_ns: 1_699_000_000_000_000_000 + i64::try_from(lsn).unwrap(),
            delivery_lsn: 3,
        })),
    )
}

fn frames() -> Vec<Frame> {
    vec![
        frame(
            1,
            EventKind::UserMsg,
            Authority::UserAsserted,
            EventPayload::UserMsg(Box::new(UserMsg {
                content: b"bind the repository host".to_vec(),
            })),
        ),
        bound(2, FIRST_NONCE, ConnectorState::Bound),
        accepted(3, FIRST_DELIVERY, 2_048),
        accepted(4, SECOND_DELIVERY, 4_096),
        settled(5, FIRST_DELIVERY, 3),
        observed(6, b"9f1c0a", 0x71),
        observed(7, b"3ed4b8", 0x72),
        bound(8, SECOND_NONCE, ConnectorState::Revoked),
    ]
}

fn open_store(directory: &std::path::Path) -> ProjectionStore {
    ProjectionStore::open(directory, 16 * 1024 * 1024).unwrap()
}

#[test]
fn connector_registry_projection_is_registered() {
    assert_eq!(ProjectionId::COUNT, 22);
    assert_eq!(ProjectionId::SourceConnectors as usize, 20);
    assert_eq!(ProjectionId::SourceConnectors.name(), "source_connectors");
}

#[test]
fn connector_binding_records_registry_state_and_redeems_consent() {
    let temporary = tempdir().unwrap();
    let store = open_store(temporary.path());
    let frames = frames();

    rebuild_projection_stream(&store, &frames[..2], false, usize::MAX).unwrap();
    let snapshot = store.begin_snapshot().unwrap();
    assert_eq!(
        snapshot
            .checkpoint(ProjectionId::SourceConnectors)
            .unwrap()
            .get(),
        2
    );
    let record = ConnectorRegistryProjection::connector(&snapshot, &CONNECTOR_ID)
        .unwrap()
        .expect("connector record");
    assert_eq!(record.connector_id, CONNECTOR_ID.to_vec());
    assert_eq!(record.provider, "repository-host");
    assert_eq!(record.external_account, "fleet-engineering");
    assert_eq!(record.credential_version, 3);
    assert_eq!(
        record.signature_scheme,
        u8::from(SourceSignatureScheme::HmacSha256V0)
    );
    assert_eq!(
        record.scopes,
        vec!["contents:read".to_owned(), "metadata:read".to_owned()]
    );
    assert_eq!(record.state, u8::from(ConnectorState::Bound));
    assert_eq!(record.bound_lsn, 2);
    assert!(ConnectorRegistryProjection::consent_redeemed(&snapshot, &FIRST_NONCE).unwrap());
    assert!(!ConnectorRegistryProjection::consent_redeemed(&snapshot, &SECOND_NONCE).unwrap());
    assert_eq!(
        ConnectorRegistryProjection::list_connectors(&snapshot, 8).unwrap(),
        vec![record]
    );
    drop(snapshot);

    rebuild_projection_stream(&store, &frames, false, usize::MAX).unwrap();
    let snapshot = store.begin_snapshot().unwrap();
    let revoked = ConnectorRegistryProjection::connector(&snapshot, &CONNECTOR_ID)
        .unwrap()
        .expect("connector record");
    assert_eq!(revoked.state, u8::from(ConnectorState::Revoked));
    assert_eq!(revoked.bound_lsn, 8);
    assert_eq!(
        ConnectorRegistryProjection::list_connectors(&snapshot, MAXIMUM_REGISTRY_SCAN)
            .unwrap()
            .len(),
        1
    );
    assert!(ConnectorRegistryProjection::consent_redeemed(&snapshot, &SECOND_NONCE).unwrap());
    assert!(!ConnectorRegistryProjection::consent_redeemed(&snapshot, &[0x33; 16]).unwrap());
}

#[test]
fn delivery_inbox_records_acceptance_then_settlement() {
    let temporary = tempdir().unwrap();
    let store = open_store(temporary.path());
    let frames = frames();

    rebuild_projection_stream(&store, &frames[..4], false, usize::MAX).unwrap();
    let snapshot = store.begin_snapshot().unwrap();
    let accepted_record =
        ConnectorRegistryProjection::delivery(&snapshot, &CONNECTOR_ID, FIRST_DELIVERY)
            .unwrap()
            .expect("delivery record");
    assert_eq!(accepted_record.delivery_id, FIRST_DELIVERY.to_vec());
    assert_eq!(accepted_record.accepted_lsn, 3);
    assert_eq!(accepted_record.body_digest, vec![0x5b; 32]);
    assert_eq!(accepted_record.body_bytes, 2_048);
    assert_eq!(accepted_record.event_name, "push");
    assert_eq!(accepted_record.attempt, 0);
    assert_eq!(
        accepted_record.state,
        u8::from(SourceDeliveryState::Accepted)
    );
    assert_eq!(accepted_record.next_attempt_at_ns, 0);
    assert_eq!(accepted_record.detail, "");
    assert_eq!(accepted_record.settled_lsn, 0);
    let recent =
        ConnectorRegistryProjection::recent_deliveries(&snapshot, &CONNECTOR_ID, 8).unwrap();
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0].delivery_id, SECOND_DELIVERY.to_vec());
    assert_eq!(recent[1].delivery_id, FIRST_DELIVERY.to_vec());
    drop(snapshot);

    rebuild_projection_stream(&store, &frames[..5], false, usize::MAX).unwrap();
    let snapshot = store.begin_snapshot().unwrap();
    let settled_record =
        ConnectorRegistryProjection::delivery(&snapshot, &CONNECTOR_ID, FIRST_DELIVERY)
            .unwrap()
            .expect("delivery record");
    assert_eq!(settled_record.accepted_lsn, 3);
    assert_eq!(settled_record.body_digest, accepted_record.body_digest);
    assert_eq!(settled_record.body_bytes, accepted_record.body_bytes);
    assert_eq!(settled_record.attempt, 2);
    assert_eq!(settled_record.state, u8::from(SourceDeliveryState::Applied));
    assert_eq!(settled_record.next_attempt_at_ns, 1_699_000_000_000_000_500);
    assert_eq!(settled_record.detail, "ingested 4 revisions");
    assert_eq!(settled_record.settled_lsn, 5);
    let untouched =
        ConnectorRegistryProjection::delivery(&snapshot, &CONNECTOR_ID, SECOND_DELIVERY)
            .unwrap()
            .expect("delivery record");
    assert_eq!(untouched.accepted_lsn, 4);
    assert_eq!(untouched.settled_lsn, 0);
    assert_eq!(
        ConnectorRegistryProjection::delivery(&snapshot, &CONNECTOR_ID, b"delivery-0009").unwrap(),
        None
    );
}

#[test]
fn revision_head_is_replaced_not_accumulated() {
    let temporary = tempdir().unwrap();
    let store = open_store(temporary.path());
    let frames = frames();

    rebuild_projection_stream(&store, &frames[..6], false, usize::MAX).unwrap();
    let snapshot = store.begin_snapshot().unwrap();
    let head = ConnectorRegistryProjection::revision(&snapshot, &CONNECTOR_ID, SOURCE_ID)
        .unwrap()
        .expect("revision record");
    assert_eq!(head.revision, b"9f1c0a".to_vec());
    assert_eq!(head.content_digest, vec![0x71; 32]);
    assert_eq!(head.lsn, 6);
    drop(snapshot);

    rebuild_projection_stream(&store, &frames[..7], false, usize::MAX).unwrap();
    let snapshot = store.begin_snapshot().unwrap();
    let head = ConnectorRegistryProjection::revision(&snapshot, &CONNECTOR_ID, SOURCE_ID)
        .unwrap()
        .expect("revision record");
    assert_eq!(head.source_id, SOURCE_ID);
    assert_eq!(head.revision, b"3ed4b8".to_vec());
    assert_eq!(head.content_digest, vec![0x72; 32]);
    assert_eq!(head.observed_at_ns, 1_699_000_000_000_000_007);
    assert_eq!(head.lsn, 7);
    let heads = ConnectorRegistryProjection::recent_revisions(
        &snapshot,
        &CONNECTOR_ID,
        MAXIMUM_REGISTRY_SCAN,
    )
    .unwrap();
    assert_eq!(heads, vec![head]);
    assert_eq!(
        ConnectorRegistryProjection::revision(&snapshot, &CONNECTOR_ID, "engine/src/other.rs")
            .unwrap(),
        None
    );
}

#[test]
fn bounded_readers_reject_zero_limit() {
    let temporary = tempdir().unwrap();
    let store = open_store(temporary.path());
    let frames = frames();
    rebuild_projection_stream(&store, &frames, false, usize::MAX).unwrap();
    let snapshot = store.begin_snapshot().unwrap();

    for code in [
        ConnectorRegistryProjection::list_connectors(&snapshot, 0)
            .unwrap_err()
            .code,
        ConnectorRegistryProjection::recent_deliveries(&snapshot, &CONNECTOR_ID, 0)
            .unwrap_err()
            .code,
        ConnectorRegistryProjection::recent_revisions(&snapshot, &CONNECTOR_ID, 0)
            .unwrap_err()
            .code,
        ConnectorRegistryProjection::connector(&snapshot, &[])
            .unwrap_err()
            .code,
        ConnectorRegistryProjection::delivery(&snapshot, &CONNECTOR_ID, &[])
            .unwrap_err()
            .code,
        ConnectorRegistryProjection::revision(&snapshot, &CONNECTOR_ID, "")
            .unwrap_err()
            .code,
        ConnectorRegistryProjection::consent_redeemed(&snapshot, &[])
            .unwrap_err()
            .code,
    ] {
        assert_eq!(code, ErrorCode::InvalidArgument);
    }

    assert!(
        ConnectorRegistryProjection::list_connectors(&snapshot, usize::MAX)
            .unwrap()
            .len()
            <= MAXIMUM_REGISTRY_SCAN
    );
    assert!(
        ConnectorRegistryProjection::recent_deliveries(&snapshot, &CONNECTOR_ID, usize::MAX)
            .unwrap()
            .len()
            <= MAXIMUM_REGISTRY_SCAN
    );
}

#[test]
fn registry_rebuild_is_byte_identical() {
    let temporary = tempdir().unwrap();
    let store = open_store(temporary.path());
    let frames = frames();

    let progress = rebuild_projection_stream(&store, &frames, false, usize::MAX).unwrap();
    assert!(progress.complete);
    assert_eq!(progress.applied_lsn.get(), 8);
    let snapshot = store.begin_snapshot().unwrap();
    assert_eq!(
        snapshot
            .checkpoint(ProjectionId::SourceConnectors)
            .unwrap()
            .get(),
        8
    );
    let incremental = snapshot
        .canonical_dump(ProjectionId::SourceConnectors)
        .unwrap();
    drop(snapshot);

    let progress = rebuild_projection_stream(&store, &frames, true, usize::MAX).unwrap();
    assert!(progress.complete);
    let snapshot = store.begin_snapshot().unwrap();
    let rebuilt = snapshot
        .canonical_dump(ProjectionId::SourceConnectors)
        .unwrap();
    assert_eq!(rebuilt, incremental);
}
