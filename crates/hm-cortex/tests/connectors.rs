#![forbid(unsafe_code)]

use hm_core::ErrorCode;
use hm_cortex::connectors::{
    DEFAULT_FRESHNESS_WINDOW_NS, DeliveryEnvelope, MAXIMUM_DELIVERY_ATTEMPTS,
    MAXIMUM_DELIVERY_BYTES, hmac_sha256, mint_consent_state, retry_delay_ns, retry_exhausted,
    signing_base, verify_consent_state, verify_delivery,
};

const SECRET: &[u8] = b"connector-shared-secret";
const CONNECTOR_ID: [u8; 16] = [
    0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00,
];
const NONCE: [u8; 16] = [
    0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
];
const DELIVERY_ID: &[u8] = b"delivery-0001";
const BODY: &[u8] = b"{\"action\":\"synchronize\",\"revision\":\"a1b2c3\"}";
const SIGNED_AT_NS: i64 = 1_700_000_000_000_000_000;
const PROVIDER: &str = "repository-host";

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn signature_for(
    secret: &[u8],
    connector_id: &[u8; 16],
    delivery_id: &[u8],
    body: &[u8],
) -> Vec<u8> {
    hmac_sha256(
        secret,
        &signing_base(connector_id, delivery_id, SIGNED_AT_NS, body),
    )
    .to_vec()
}

fn envelope<'a>(
    delivery_id: &'a [u8],
    body: &'a [u8],
    signature: &'a [u8],
) -> DeliveryEnvelope<'a> {
    DeliveryEnvelope {
        connector_id: CONNECTOR_ID,
        delivery_id,
        event_name: "repository.push",
        signed_at_ns: SIGNED_AT_NS,
        body,
        signature,
    }
}

#[test]
fn hmac_sha256_matches_published_vectors() {
    assert_eq!(
        hex(&hmac_sha256(&[0x0b; 20], b"Hi There")),
        "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
    );
    assert_eq!(
        hex(&hmac_sha256(b"Jefe", b"what do ya want for nothing?")),
        "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
    );
    assert_eq!(
        hex(&hmac_sha256(
            &[0xaa; 131],
            b"Test Using Larger Than Block-Size Key - Hash Key First"
        )),
        "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
    );
    assert_eq!(
        hex(&hmac_sha256(
            &[0xaa; 131],
            b"This is a test using a larger than block-size key and a larger than block-size data. The key needs to be hashed before being used by the HMAC algorithm."
        )),
        "9b09ffa71b942fcb27635fbcd5b0e944bfdc63644f0713938a7f51535c3a35e2"
    );
}

#[test]
fn delivery_signature_verifies_and_fails_closed() {
    let signature = signature_for(SECRET, &CONNECTOR_ID, DELIVERY_ID, BODY);
    let accepted = verify_delivery(
        SECRET,
        &envelope(DELIVERY_ID, BODY, &signature),
        SIGNED_AT_NS,
        DEFAULT_FRESHNESS_WINDOW_NS,
    )
    .unwrap();
    assert_eq!(accepted.body_digest, *blake3::hash(BODY).as_bytes());
    assert_eq!(accepted.body_bytes, BODY.len() as u64);

    let mut tampered_body = BODY.to_vec();
    tampered_body[3] ^= 0x01;
    let failure = verify_delivery(
        SECRET,
        &envelope(DELIVERY_ID, &tampered_body, &signature),
        SIGNED_AT_NS,
        DEFAULT_FRESHNESS_WINDOW_NS,
    )
    .unwrap_err();
    assert_eq!(failure.code, ErrorCode::SignatureInvalid);

    let mut tampered_delivery_id = DELIVERY_ID.to_vec();
    tampered_delivery_id[0] ^= 0x01;
    let failure = verify_delivery(
        SECRET,
        &envelope(&tampered_delivery_id, BODY, &signature),
        SIGNED_AT_NS,
        DEFAULT_FRESHNESS_WINDOW_NS,
    )
    .unwrap_err();
    assert_eq!(failure.code, ErrorCode::SignatureInvalid);

    let mut tampered_connector = envelope(DELIVERY_ID, BODY, &signature);
    tampered_connector.connector_id[15] ^= 0x01;
    let failure = verify_delivery(
        SECRET,
        &tampered_connector,
        SIGNED_AT_NS,
        DEFAULT_FRESHNESS_WINDOW_NS,
    )
    .unwrap_err();
    assert_eq!(failure.code, ErrorCode::SignatureInvalid);

    let mut tampered_time = envelope(DELIVERY_ID, BODY, &signature);
    tampered_time.signed_at_ns += 1;
    let failure = verify_delivery(
        SECRET,
        &tampered_time,
        SIGNED_AT_NS,
        DEFAULT_FRESHNESS_WINDOW_NS,
    )
    .unwrap_err();
    assert_eq!(failure.code, ErrorCode::SignatureInvalid);

    let mut tampered_signature = signature.clone();
    tampered_signature[31] ^= 0x01;
    let failure = verify_delivery(
        SECRET,
        &envelope(DELIVERY_ID, BODY, &tampered_signature),
        SIGNED_AT_NS,
        DEFAULT_FRESHNESS_WINDOW_NS,
    )
    .unwrap_err();
    assert_eq!(failure.code, ErrorCode::SignatureInvalid);

    let failure = verify_delivery(
        b"a-different-connector-secret",
        &envelope(DELIVERY_ID, BODY, &signature),
        SIGNED_AT_NS,
        DEFAULT_FRESHNESS_WINDOW_NS,
    )
    .unwrap_err();
    assert_eq!(failure.code, ErrorCode::SignatureInvalid);
}

#[test]
fn delivery_freshness_window_is_enforced() {
    let signature = signature_for(SECRET, &CONNECTOR_ID, DELIVERY_ID, BODY);
    let window = DEFAULT_FRESHNESS_WINDOW_NS;
    for now_ns in [SIGNED_AT_NS - window, SIGNED_AT_NS + window] {
        assert!(
            verify_delivery(
                SECRET,
                &envelope(DELIVERY_ID, BODY, &signature),
                now_ns,
                window
            )
            .is_ok()
        );
    }
    for now_ns in [SIGNED_AT_NS - window - 1, SIGNED_AT_NS + window + 1] {
        let failure = verify_delivery(
            SECRET,
            &envelope(DELIVERY_ID, BODY, &signature),
            now_ns,
            window,
        )
        .unwrap_err();
        assert_eq!(failure.code, ErrorCode::OrderingViolation);
    }
}

#[test]
fn delivery_shape_is_bounded() {
    let empty_id_signature = signature_for(SECRET, &CONNECTOR_ID, b"", BODY);
    let failure = verify_delivery(
        SECRET,
        &envelope(b"", BODY, &empty_id_signature),
        SIGNED_AT_NS,
        DEFAULT_FRESHNESS_WINDOW_NS,
    )
    .unwrap_err();
    assert_eq!(failure.code, ErrorCode::InvalidArgument);

    let empty_body_signature = signature_for(SECRET, &CONNECTOR_ID, DELIVERY_ID, b"");
    let failure = verify_delivery(
        SECRET,
        &envelope(DELIVERY_ID, b"", &empty_body_signature),
        SIGNED_AT_NS,
        DEFAULT_FRESHNESS_WINDOW_NS,
    )
    .unwrap_err();
    assert_eq!(failure.code, ErrorCode::InvalidArgument);

    let oversized = vec![0x7a; MAXIMUM_DELIVERY_BYTES + 1];
    let oversized_signature = signature_for(SECRET, &CONNECTOR_ID, DELIVERY_ID, &oversized);
    let failure = verify_delivery(
        SECRET,
        &envelope(DELIVERY_ID, &oversized, &oversized_signature),
        SIGNED_AT_NS,
        DEFAULT_FRESHNESS_WINDOW_NS,
    )
    .unwrap_err();
    assert_eq!(failure.code, ErrorCode::InvalidArgument);

    let largest = vec![0x7a; MAXIMUM_DELIVERY_BYTES];
    let largest_signature = signature_for(SECRET, &CONNECTOR_ID, DELIVERY_ID, &largest);
    let accepted = verify_delivery(
        SECRET,
        &envelope(DELIVERY_ID, &largest, &largest_signature),
        SIGNED_AT_NS,
        DEFAULT_FRESHNESS_WINDOW_NS,
    )
    .unwrap();
    assert_eq!(accepted.body_bytes, MAXIMUM_DELIVERY_BYTES as u64);

    let signature = signature_for(SECRET, &CONNECTOR_ID, DELIVERY_ID, BODY);
    let failure = verify_delivery(
        SECRET,
        &envelope(DELIVERY_ID, BODY, &signature[..31]),
        SIGNED_AT_NS,
        DEFAULT_FRESHNESS_WINDOW_NS,
    )
    .unwrap_err();
    assert_eq!(failure.code, ErrorCode::InvalidArgument);
}

#[test]
fn consent_state_is_provider_and_connector_bound() {
    let expires_at_ns = SIGNED_AT_NS + DEFAULT_FRESHNESS_WINDOW_NS;
    let state = mint_consent_state(SECRET, PROVIDER, &CONNECTOR_ID, NONCE, expires_at_ns);
    assert_eq!(state.nonce, NONCE);
    assert_eq!(state.expires_at_ns, expires_at_ns);
    let grant = verify_consent_state(
        SECRET,
        PROVIDER,
        &CONNECTOR_ID,
        &state.encoded,
        SIGNED_AT_NS,
    )
    .unwrap();
    assert_eq!(grant.nonce, NONCE);
    assert_eq!(grant.expires_at_ns, expires_at_ns);

    let failure = verify_consent_state(
        SECRET,
        "workspace-host",
        &CONNECTOR_ID,
        &state.encoded,
        SIGNED_AT_NS,
    )
    .unwrap_err();
    assert_eq!(failure.code, ErrorCode::CapabilityDenied);

    let mut other_connector = CONNECTOR_ID;
    other_connector[0] ^= 0x01;
    let failure = verify_consent_state(
        SECRET,
        PROVIDER,
        &other_connector,
        &state.encoded,
        SIGNED_AT_NS,
    )
    .unwrap_err();
    assert_eq!(failure.code, ErrorCode::CapabilityDenied);

    let mut tampered_nonce = NONCE;
    tampered_nonce[7] ^= 0x01;
    let forged = format!(
        "{}.{expires_at_ns}.{}",
        hex(&tampered_nonce),
        &state.encoded[state.encoded.len() - 64..]
    );
    let failure =
        verify_consent_state(SECRET, PROVIDER, &CONNECTOR_ID, &forged, SIGNED_AT_NS).unwrap_err();
    assert_eq!(failure.code, ErrorCode::CapabilityDenied);

    let forged = format!(
        "{}.{}.{}",
        hex(&NONCE),
        expires_at_ns + 1,
        &state.encoded[state.encoded.len() - 64..]
    );
    let failure =
        verify_consent_state(SECRET, PROVIDER, &CONNECTOR_ID, &forged, SIGNED_AT_NS).unwrap_err();
    assert_eq!(failure.code, ErrorCode::CapabilityDenied);

    let mut tampered_tag = state.encoded.clone();
    tampered_tag.pop();
    tampered_tag.push(if state.encoded.ends_with('0') {
        '1'
    } else {
        '0'
    });
    let failure =
        verify_consent_state(SECRET, PROVIDER, &CONNECTOR_ID, &tampered_tag, SIGNED_AT_NS)
            .unwrap_err();
    assert_eq!(failure.code, ErrorCode::CapabilityDenied);

    let tag_field = state.encoded[state.encoded.len() - 64..].to_owned();
    for malformed in [
        String::new(),
        "not-a-consent-state".to_owned(),
        format!("{}.{expires_at_ns}", hex(&NONCE)),
        format!("{}.{expires_at_ns}.{tag_field}.extra", hex(&NONCE)),
        format!("{}.{expires_at_ns}.{tag_field}", "zz".repeat(16)),
        format!("{}.not-a-number.{tag_field}", hex(&NONCE)),
        format!("{}.{expires_at_ns}.{}", hex(&NONCE), hex(&NONCE)),
    ] {
        let failure =
            verify_consent_state(SECRET, PROVIDER, &CONNECTOR_ID, &malformed, SIGNED_AT_NS)
                .unwrap_err();
        assert_eq!(failure.code, ErrorCode::CapabilityDenied);
    }

    for now_ns in [expires_at_ns, expires_at_ns + 1] {
        let failure = verify_consent_state(SECRET, PROVIDER, &CONNECTOR_ID, &state.encoded, now_ns)
            .unwrap_err();
        assert_eq!(failure.code, ErrorCode::CapabilityDenied);
    }
}

#[test]
fn retry_schedule_is_bounded() {
    assert_eq!(retry_delay_ns(1), 1_000_000_000);
    assert_eq!(retry_delay_ns(2), 2_000_000_000);
    assert_eq!(retry_delay_ns(3), 4_000_000_000);
    assert_eq!(retry_delay_ns(4), 8_000_000_000);
    assert_eq!(retry_delay_ns(5), 16_000_000_000);
    assert_eq!(retry_delay_ns(6), 32_000_000_000);
    assert_eq!(retry_delay_ns(7), 60_000_000_000);
    assert_eq!(retry_delay_ns(8), 60_000_000_000);
    assert_eq!(retry_delay_ns(u32::MAX), 60_000_000_000);
    assert_eq!(retry_delay_ns(0), 0);

    assert_eq!(MAXIMUM_DELIVERY_ATTEMPTS, 5);
    for attempt in 0..=MAXIMUM_DELIVERY_ATTEMPTS {
        assert!(!retry_exhausted(attempt));
    }
    assert!(retry_exhausted(MAXIMUM_DELIVERY_ATTEMPTS + 1));
    assert!(retry_exhausted(u32::MAX));
}
