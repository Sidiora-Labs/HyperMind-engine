#![allow(clippy::missing_errors_doc)]

use crate::fusion::{Q16_ONE, RankedCandidate};
use hm_core::{Error, ErrorCode, LSN, UtcNanos};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemporalCandidate {
    pub canonical_id: Vec<u8>,
    pub lsn: LSN,
    pub event_time_ns: UtcNanos,
}

pub fn search(
    candidates: &[TemporalCandidate],
    now_ns: UtcNanos,
    half_life_ns: u64,
    limit: usize,
) -> Result<Vec<RankedCandidate>, Error> {
    if half_life_ns == 0 || limit == 0 {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let mut ranked = candidates
        .iter()
        .map(|candidate| {
            if candidate.canonical_id.is_empty() || candidate.lsn.get() == 0 {
                return Err(Error::new(ErrorCode::InvalidArgument));
            }
            let age = now_ns.get().saturating_sub(candidate.event_time_ns.get());
            let age = u64::try_from(age).unwrap_or(0);
            let denominator = half_life_ns.saturating_add(age);
            let prior = if denominator == 0 {
                Q16_ONE
            } else {
                u32::try_from(
                    (u128::from(half_life_ns) * u128::from(Q16_ONE)) / u128::from(denominator),
                )
                .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?
            };
            Ok(RankedCandidate {
                canonical_id: candidate.canonical_id.clone(),
                lsn: candidate.lsn,
                fsrs_retrievability_q16: Q16_ONE,
                salience_q16: Q16_ONE,
                recency_q16: prior,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    ranked.sort_by(|left, right| {
        right
            .recency_q16
            .cmp(&left.recency_q16)
            .then_with(|| left.lsn.cmp(&right.lsn))
    });
    ranked.truncate(limit);
    Ok(ranked)
}
