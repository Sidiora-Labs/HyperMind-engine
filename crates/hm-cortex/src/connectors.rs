#![allow(clippy::missing_errors_doc)]

use hm_core::{Error, ErrorCode};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use subtle::ConstantTimeEq;

pub const SIGNATURE_DOMAIN: &[u8] = b"hypermind.source-delivery.v0\0";
pub const CONSENT_DOMAIN: &[u8] = b"hypermind.source-consent.v0\0";
pub const DEFAULT_FRESHNESS_WINDOW_NS: i64 = 300_000_000_000;
pub const MAXIMUM_DELIVERY_BYTES: usize = 1024 * 1024;
pub const MAXIMUM_DELIVERY_ATTEMPTS: u32 = 5;

const HMAC_BLOCK_BYTES: usize = 64;
const CONNECTOR_ID_BYTES: usize = 16;
const CONSENT_NONCE_BYTES: usize = 16;
const TAG_BYTES: usize = 32;
const BASE_RETRY_DELAY_NS: i64 = 1_000_000_000;
const MAXIMUM_RETRY_DELAY_NS: i64 = 60_000_000_000;
const MAXIMUM_RETRY_SHIFT: u32 = 6;

#[must_use]
pub fn hmac_sha256(secret: &[u8], message: &[u8]) -> [u8; 32] {
    let mut key = [0_u8; HMAC_BLOCK_BYTES];
    if secret.len() > HMAC_BLOCK_BYTES {
        key[..TAG_BYTES].copy_from_slice(&Sha256::digest(secret));
    } else {
        key[..secret.len()].copy_from_slice(secret);
    }
    let mut inner_key = [0x36_u8; HMAC_BLOCK_BYTES];
    let mut outer_key = [0x5c_u8; HMAC_BLOCK_BYTES];
    for ((inner, outer), byte) in inner_key.iter_mut().zip(outer_key.iter_mut()).zip(key) {
        *inner ^= byte;
        *outer ^= byte;
    }
    let mut inner = Sha256::new();
    inner.update(inner_key);
    inner.update(message);
    let mut outer = Sha256::new();
    outer.update(outer_key);
    outer.update(inner.finalize());
    outer.finalize().into()
}

#[must_use]
pub fn signing_base(
    connector_id: &[u8; 16],
    delivery_id: &[u8],
    signed_at_ns: i64,
    body: &[u8],
) -> Vec<u8> {
    let mut base = Vec::with_capacity(
        SIGNATURE_DOMAIN.len() + CONNECTOR_ID_BYTES + delivery_id.len() + body.len() + 11,
    );
    base.extend_from_slice(SIGNATURE_DOMAIN);
    base.extend_from_slice(connector_id);
    base.push(0);
    base.extend_from_slice(delivery_id);
    base.push(0);
    base.extend_from_slice(&signed_at_ns.to_le_bytes());
    base.push(0);
    base.extend_from_slice(body);
    base
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeliveryEnvelope<'a> {
    pub connector_id: [u8; 16],
    pub delivery_id: &'a [u8],
    pub event_name: &'a str,
    pub signed_at_ns: i64,
    pub body: &'a [u8],
    pub signature: &'a [u8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerifiedDelivery {
    pub body_digest: [u8; 32],
    pub body_bytes: u64,
}

pub fn verify_delivery(
    secret: &[u8],
    envelope: &DeliveryEnvelope<'_>,
    now_ns: i64,
    window_ns: i64,
) -> Result<VerifiedDelivery, Error> {
    if secret.is_empty()
        || window_ns < 0
        || envelope.delivery_id.is_empty()
        || envelope.body.is_empty()
        || envelope.body.len() > MAXIMUM_DELIVERY_BYTES
        || envelope.signature.len() != TAG_BYTES
    {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let base = signing_base(
        &envelope.connector_id,
        envelope.delivery_id,
        envelope.signed_at_ns,
        envelope.body,
    );
    let expected = hmac_sha256(secret, &base);
    if expected.ct_eq(envelope.signature).unwrap_u8() != 1 {
        return Err(Error::new(ErrorCode::SignatureInvalid));
    }
    let skew = now_ns.saturating_sub(envelope.signed_at_ns);
    if skew > window_ns || skew < -window_ns {
        return Err(Error::new(ErrorCode::OrderingViolation));
    }
    Ok(VerifiedDelivery {
        body_digest: *blake3::hash(envelope.body).as_bytes(),
        body_bytes: envelope.body.len() as u64,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConsentState {
    pub encoded: String,
    pub nonce: [u8; 16],
    pub expires_at_ns: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConsentGrant {
    pub nonce: [u8; 16],
    pub expires_at_ns: i64,
}

#[must_use]
pub fn mint_consent_state(
    secret: &[u8],
    provider: &str,
    connector_id: &[u8; 16],
    nonce: [u8; 16],
    expires_at_ns: i64,
) -> ConsentState {
    let tag = consent_tag(secret, provider, connector_id, &nonce, expires_at_ns);
    let mut encoded = encode_hex(&nonce);
    let _ = write!(encoded, ".{expires_at_ns}.");
    encoded.push_str(&encode_hex(&tag));
    ConsentState {
        encoded,
        nonce,
        expires_at_ns,
    }
}

pub fn verify_consent_state(
    secret: &[u8],
    provider: &str,
    connector_id: &[u8; 16],
    encoded: &str,
    now_ns: i64,
) -> Result<ConsentGrant, Error> {
    let denied = Error::new(ErrorCode::CapabilityDenied);
    let mut fields = encoded.split('.');
    let (Some(nonce_field), Some(expiry_field), Some(tag_field), None) =
        (fields.next(), fields.next(), fields.next(), fields.next())
    else {
        return Err(denied);
    };
    let Some(nonce) = decode_hex::<CONSENT_NONCE_BYTES>(nonce_field) else {
        return Err(denied);
    };
    let Ok(expires_at_ns) = expiry_field.parse::<i64>() else {
        return Err(denied);
    };
    let Some(tag) = decode_hex::<TAG_BYTES>(tag_field) else {
        return Err(denied);
    };
    let expected = consent_tag(secret, provider, connector_id, &nonce, expires_at_ns);
    if expected.ct_eq(&tag).unwrap_u8() != 1 {
        return Err(denied);
    }
    if expires_at_ns <= now_ns {
        return Err(denied);
    }
    Ok(ConsentGrant {
        nonce,
        expires_at_ns,
    })
}

#[must_use]
pub const fn retry_delay_ns(attempt: u32) -> i64 {
    if attempt == 0 {
        return 0;
    }
    let shift = if attempt - 1 > MAXIMUM_RETRY_SHIFT {
        MAXIMUM_RETRY_SHIFT
    } else {
        attempt - 1
    };
    let delay = BASE_RETRY_DELAY_NS << shift;
    if delay > MAXIMUM_RETRY_DELAY_NS {
        MAXIMUM_RETRY_DELAY_NS
    } else {
        delay
    }
}

#[must_use]
pub const fn retry_exhausted(attempt: u32) -> bool {
    attempt > MAXIMUM_DELIVERY_ATTEMPTS
}

fn consent_tag(
    secret: &[u8],
    provider: &str,
    connector_id: &[u8; CONNECTOR_ID_BYTES],
    nonce: &[u8; CONSENT_NONCE_BYTES],
    expires_at_ns: i64,
) -> [u8; TAG_BYTES] {
    let mut message = Vec::with_capacity(
        CONSENT_DOMAIN.len() + provider.len() + CONNECTOR_ID_BYTES + CONSENT_NONCE_BYTES + 11,
    );
    message.extend_from_slice(CONSENT_DOMAIN);
    message.extend_from_slice(provider.as_bytes());
    message.push(0);
    message.extend_from_slice(connector_id);
    message.push(0);
    message.extend_from_slice(nonce);
    message.push(0);
    message.extend_from_slice(&expires_at_ns.to_le_bytes());
    hmac_sha256(secret, &message)
}

fn encode_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn decode_hex<const N: usize>(encoded: &str) -> Option<[u8; N]> {
    let source = encoded.as_bytes();
    if source.len() != N * 2 {
        return None;
    }
    let mut output = [0_u8; N];
    for (index, byte) in output.iter_mut().enumerate() {
        let high = hex_digit(source[index * 2])?;
        let low = hex_digit(source[index * 2 + 1])?;
        *byte = (high << 4) | low;
    }
    Some(output)
}

const fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}
