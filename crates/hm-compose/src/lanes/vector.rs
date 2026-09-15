#![allow(clippy::missing_errors_doc)]

use crate::fusion::RankedCandidate;
use hm_core::{Error, ErrorCode};
use hm_proj::vectors::VectorLane;

pub fn search(
    lane: &VectorLane,
    query: &[i8],
    binary_prefilter: &[u8],
    maximum_candidates: usize,
) -> Result<Vec<RankedCandidate>, Error> {
    if maximum_candidates == 0 {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    lane.search(
        query,
        binary_prefilter,
        maximum_candidates.saturating_mul(4),
        maximum_candidates,
    )
    .map(|hits| {
        hits.into_iter()
            .map(|hit| {
                RankedCandidate::neutral(
                    hit.target_lsn.get().to_be_bytes().to_vec(),
                    hit.target_lsn,
                )
            })
            .collect()
    })
}
