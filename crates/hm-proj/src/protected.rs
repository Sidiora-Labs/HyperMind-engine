#![allow(clippy::missing_errors_doc)]

use hm_core::{Error, ErrorCode, LSN};
use hm_schema::events::{Authority, BeliefType};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub observation_lsn: u64,
    pub code: String,
    pub belief_type: BeliefType,
    pub belief_id: Vec<u8>,
    pub authority: Authority,
    pub run_id: Option<Vec<u8>>,
}

#[must_use]
pub const fn is_protected(belief_type: BeliefType) -> bool {
    matches!(
        belief_type,
        BeliefType::Identity | BeliefType::Constraint | BeliefType::Preference
    )
}

pub fn enforce_direct_write(
    belief_type: BeliefType,
    belief_id: &[u8],
    authority: Authority,
    run_id: Option<&[u8]>,
    observation_lsn: LSN,
) -> Result<(), SecurityEvent> {
    if !is_protected(belief_type) || (authority == Authority::UserAsserted && run_id.is_none()) {
        return Ok(());
    }
    Err(SecurityEvent {
        observation_lsn: observation_lsn.get(),
        code: ErrorCode::ProtectedTypeWrite.as_str().to_owned(),
        belief_type,
        belief_id: belief_id.to_vec(),
        authority,
        run_id: run_id.map(<[u8]>::to_vec),
    })
}

#[must_use]
pub fn rejection_error(event: &SecurityEvent) -> Error {
    Error::new(ErrorCode::ProtectedTypeWrite).at_lsn(LSN::new(event.observation_lsn))
}
