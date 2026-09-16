#![forbid(unsafe_code)]

use hm_core::ErrorCode;
use hm_schema::event::{Boundary, EventKind, encode_event_envelope, verify_event};
use hm_schema::events::{
    Authority, ConnectorState, EventEnvelope, EventPayload, Retention, Sensitivity,
    SourceConnectorBound, SourceDeliveryAccepted, SourceDeliverySettled, SourceDeliveryState,
    SourceRevisionObserved, SourceSignatureScheme,
};

const CONNECTOR_ID: [u8; 16] = [0x21; 16];
const CONSENT_NONCE: [u8; 16] = [0x37; 16];
const BODY_DIGEST: [u8; 32] = [0x5a; 32];
const CONTENT_DIGEST: [u8; 32] = [0x6b; 32];

fn bound() -> SourceConnectorBound {
    SourceConnectorBound {
        connector_id: CONNECTOR_ID.to_vec(),
        provider: "repository-host".to_owned(),
        external_account: "engineering".to_owned(),
        consent_nonce: CONSENT_NONCE.to_vec(),
        consent_expires_at_ns: 1_700_000_000_000_000_000,
        credential_version: 1,
        signature_scheme: SourceSignatureScheme::HmacSha256V0,
        scopes: vec!["contents:read".to_owned()],
        state: ConnectorState::Bound,
    }
}

fn accepted() -> SourceDeliveryAccepted {
    SourceDeliveryAccepted {
        connector_id: CONNECTOR_ID.to_vec(),
        delivery_id: b"delivery-0001".to_vec(),
        signature_scheme: SourceSignatureScheme::HmacSha256V0,
        credential_version: 1,
        signed_at_ns: 1_700_000_000_000_000_001,
        body_digest: BODY_DIGEST.to_vec(),
        body_bytes: 4_096,
        event_name: "push".to_owned(),
    }
}

fn settled() -> SourceDeliverySettled {
    SourceDeliverySettled {
        connector_id: CONNECTOR_ID.to_vec(),
        delivery_id: b"delivery-0001".to_vec(),
        accepted_lsn: 9,
        attempt: 1,
        state: SourceDeliveryState::Applied,
        next_attempt_at_ns: 0,
        detail: "applied".to_owned(),
    }
}

fn observed() -> SourceRevisionObserved {
    SourceRevisionObserved {
        connector_id: CONNECTOR_ID.to_vec(),
        source_id: "repository/main".to_owned(),
        revision: b"0123456789abcdef".to_vec(),
        content_digest: CONTENT_DIGEST.to_vec(),
        observed_at_ns: 1_700_000_000_000_000_002,
        delivery_lsn: 11,
    }
}

fn envelope(payload: EventPayload, authority: Authority) -> EventEnvelope {
    EventEnvelope {
        schema_version: 2,
        payload,
        connection_id: None,
        client_seq: 0,
        client_event_index: 0,
        client_event_count: 0,
        origin_actor: 41,
        run_id: None,
        model_provenance: None,
        authority,
        retention: Retention::Durable,
        sensitivity: Sensitivity::Personal,
        event_time_ns: 1_700_000_000_000_000_000,
    }
}

fn pinned_authority(kind: EventKind) -> Authority {
    match kind {
        EventKind::SourceConnectorBound => Authority::UserAsserted,
        EventKind::SourceDeliverySettled => Authority::RuntimeFact,
        _ => Authority::ExternalObserved,
    }
}

fn source_records() -> Vec<(EventKind, EventPayload)> {
    vec![
        (
            EventKind::SourceConnectorBound,
            EventPayload::SourceConnectorBound(Box::new(bound())),
        ),
        (
            EventKind::SourceDeliveryAccepted,
            EventPayload::SourceDeliveryAccepted(Box::new(accepted())),
        ),
        (
            EventKind::SourceDeliverySettled,
            EventPayload::SourceDeliverySettled(Box::new(settled())),
        ),
        (
            EventKind::SourceRevisionObserved,
            EventPayload::SourceRevisionObserved(Box::new(observed())),
        ),
    ]
}

fn payload_source_kind(payload: &EventPayload) -> EventKind {
    match payload {
        EventPayload::SourceConnectorBound(_) => EventKind::SourceConnectorBound,
        EventPayload::SourceDeliveryAccepted(_) => EventKind::SourceDeliveryAccepted,
        EventPayload::SourceDeliverySettled(_) => EventKind::SourceDeliverySettled,
        EventPayload::SourceRevisionObserved(_) => EventKind::SourceRevisionObserved,
        _ => panic!("only source connector payloads are under test"),
    }
}

#[test]
fn source_connector_records_round_trip_at_both_boundaries() {
    for (kind, payload) in source_records() {
        let encoded = encode_event_envelope(&envelope(payload, pinned_authority(kind)));
        for boundary in [Boundary::Disk, Boundary::Socket] {
            let verified = verify_event(&encoded, kind, boundary).expect("source record verifies");
            assert_eq!(verified.kind, kind);
            assert_eq!(verified.boundary, boundary);
        }
    }
}

fn malformed_source_payloads() -> Vec<EventPayload> {
    let mut cases = Vec::new();
    for mutate in [
        |value: &mut SourceConnectorBound| value.connector_id = vec![0x21; 15],
        |value: &mut SourceConnectorBound| value.consent_nonce = vec![0x37; 17],
        |value: &mut SourceConnectorBound| value.provider = String::new(),
        |value: &mut SourceConnectorBound| value.external_account = String::new(),
        |value: &mut SourceConnectorBound| value.credential_version = 0,
    ] {
        let mut value = bound();
        mutate(&mut value);
        cases.push(EventPayload::SourceConnectorBound(Box::new(value)));
    }
    for mutate in [
        |value: &mut SourceDeliveryAccepted| value.connector_id = vec![0x21; 17],
        |value: &mut SourceDeliveryAccepted| value.delivery_id = Vec::new(),
        |value: &mut SourceDeliveryAccepted| value.body_digest = vec![0x5a; 31],
        |value: &mut SourceDeliveryAccepted| value.event_name = String::new(),
        |value: &mut SourceDeliveryAccepted| value.credential_version = 0,
        |value: &mut SourceDeliveryAccepted| value.signed_at_ns = 0,
    ] {
        let mut value = accepted();
        mutate(&mut value);
        cases.push(EventPayload::SourceDeliveryAccepted(Box::new(value)));
    }
    for mutate in [
        |value: &mut SourceDeliverySettled| value.connector_id = vec![0x21; 15],
        |value: &mut SourceDeliverySettled| value.delivery_id = Vec::new(),
        |value: &mut SourceDeliverySettled| value.state = SourceDeliveryState::Accepted,
        |value: &mut SourceDeliverySettled| value.detail = String::new(),
    ] {
        let mut value = settled();
        mutate(&mut value);
        cases.push(EventPayload::SourceDeliverySettled(Box::new(value)));
    }
    for mutate in [
        |value: &mut SourceRevisionObserved| value.connector_id = vec![0x21; 15],
        |value: &mut SourceRevisionObserved| value.source_id = String::new(),
        |value: &mut SourceRevisionObserved| value.revision = Vec::new(),
        |value: &mut SourceRevisionObserved| value.content_digest = vec![0x6b; 31],
        |value: &mut SourceRevisionObserved| value.observed_at_ns = 0,
    ] {
        let mut value = observed();
        mutate(&mut value);
        cases.push(EventPayload::SourceRevisionObserved(Box::new(value)));
    }
    cases
}

#[test]
fn source_connector_records_fail_closed_on_shape() {
    let cases = malformed_source_payloads();
    assert_eq!(cases.len(), 20);
    for payload in cases {
        let kind = payload_source_kind(&payload);
        let encoded = encode_event_envelope(&envelope(payload, pinned_authority(kind)));
        let error = verify_event(&encoded, kind, Boundary::Socket)
            .expect_err("malformed source payload must fail closed");
        assert_eq!(error.code, ErrorCode::SchemaInvalid);
    }
}

#[test]
fn source_connector_authority_is_pinned() {
    let wrong = [
        (EventKind::SourceConnectorBound, Authority::ExternalObserved),
        (EventKind::SourceDeliveryAccepted, Authority::UserAsserted),
        (
            EventKind::SourceDeliverySettled,
            Authority::ExternalObserved,
        ),
        (EventKind::SourceRevisionObserved, Authority::RuntimeFact),
    ];
    for ((kind, payload), (pinned_kind, authority)) in source_records().into_iter().zip(wrong) {
        assert_eq!(kind, pinned_kind);
        let encoded = encode_event_envelope(&envelope(payload, authority));
        let error = verify_event(&encoded, kind, Boundary::Socket)
            .expect_err("source records refuse unpinned authority");
        assert_eq!(error.code, ErrorCode::ProtectedTypeWrite);
    }
}

#[test]
fn source_connector_kinds_are_wave_seven() {
    for value in 48..=51_u8 {
        let kind = EventKind::try_from(value).expect("source connector kind");
        assert!(kind.is_wave_seven());
        assert!(!kind.is_llm_derived());
        assert!(!kind.requires_run_id());
    }
    assert_eq!(EventKind::try_from(48), Ok(EventKind::SourceConnectorBound));
    assert_eq!(
        EventKind::try_from(49),
        Ok(EventKind::SourceDeliveryAccepted)
    );
    assert_eq!(
        EventKind::try_from(50),
        Ok(EventKind::SourceDeliverySettled)
    );
    assert_eq!(
        EventKind::try_from(51),
        Ok(EventKind::SourceRevisionObserved)
    );
    assert_eq!(EventKind::try_from(52), Err(()));
}
