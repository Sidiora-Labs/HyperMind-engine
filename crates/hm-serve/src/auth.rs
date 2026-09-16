#![allow(clippy::missing_errors_doc)]

use crate::config::{ServerConfig, capability_equal};
use hm_core::{Error, ErrorCode};
use hm_schema::wire::RequestPayload;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Principal {
    Actor(u16),
    Admin,
}

pub fn authenticate(config: &ServerConfig, token: &[u8]) -> Result<Principal, Error> {
    if capability_equal(&config.admin_token, token) {
        return Ok(Principal::Admin);
    }
    config
        .actors
        .iter()
        .find(|capability| capability_equal(&capability.token, token))
        .map(|capability| Principal::Actor(capability.actor))
        .ok_or_else(|| Error::new(ErrorCode::CapabilityDenied))
}

pub const ADMIN_OPERATIONS: [&str; 6] = [
    "health",
    "stats",
    "latency_histograms",
    "verify",
    "rebuild",
    "crypto_delete",
];

#[must_use]
pub const fn is_admin_request(request: &RequestPayload) -> bool {
    matches!(
        request,
        RequestPayload::Health(_)
            | RequestPayload::Stats(_)
            | RequestPayload::LatencyHistograms(_)
            | RequestPayload::VerifyStatus(_)
            | RequestPayload::RebuildProjection(_)
            | RequestPayload::CryptoDelete(_)
    )
}

pub fn authorize(principal: Principal, request: &RequestPayload) -> Result<(), Error> {
    if matches!(principal, Principal::Admin) == is_admin_request(request) {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::CapabilityDenied))
    }
}
