#![allow(clippy::missing_errors_doc)]

use hm_core::{Error, ErrorCode, LSN};
use hm_proj::attestations::{
    AttestationsProjection, PREFERENCE_MAXIMUM_Q16, PREFERENCE_MINIMUM_Q16, PREFERENCE_NEUTRAL_Q16,
};
use hm_proj::store::ReadSnapshot;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreferenceProfile {
    weights: BTreeMap<u64, u32>,
}

impl PreferenceProfile {
    #[must_use]
    pub fn neutral() -> Self {
        Self {
            weights: BTreeMap::new(),
        }
    }

    pub fn load(snapshot: &ReadSnapshot<'_>, targets: &[LSN]) -> Result<Self, Error> {
        let weights = AttestationsProjection::preferences(snapshot, targets)?;
        if weights
            .values()
            .any(|weight| *weight < PREFERENCE_MINIMUM_Q16 || *weight > PREFERENCE_MAXIMUM_Q16)
        {
            return Err(Error::new(ErrorCode::InvariantViolation));
        }
        Ok(Self { weights })
    }

    #[must_use]
    pub fn is_neutral(&self) -> bool {
        self.weights
            .values()
            .all(|weight| *weight == PREFERENCE_NEUTRAL_Q16)
    }

    #[must_use]
    pub fn weight_q16(&self, lsn: LSN) -> u32 {
        self.weights
            .get(&lsn.get())
            .copied()
            .unwrap_or(PREFERENCE_NEUTRAL_Q16)
    }
}

pub fn adjust_q32(score_q32: u64, preference_q16: u32) -> Result<u64, Error> {
    if !(PREFERENCE_MINIMUM_Q16..=PREFERENCE_MAXIMUM_Q16).contains(&preference_q16) {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    if preference_q16 == PREFERENCE_NEUTRAL_Q16 {
        return Ok(score_q32);
    }
    let value = u128::from(score_q32)
        .checked_mul(u128::from(preference_q16))
        .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?
        / u128::from(PREFERENCE_NEUTRAL_Q16);
    u64::try_from(value).map_err(|_| Error::new(ErrorCode::CapacityExceeded))
}
