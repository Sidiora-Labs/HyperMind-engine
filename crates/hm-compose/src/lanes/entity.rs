#![allow(clippy::missing_errors_doc)]

use crate::fusion::RankedCandidate;
use hm_core::Error;
use hm_proj::entities::EntityProjection;
use hm_proj::store::ReadSnapshot;

pub fn search(
    snapshot: &ReadSnapshot<'_>,
    query: &str,
    turn_text: &str,
    maximum_candidates: usize,
) -> Result<Vec<RankedCandidate>, Error> {
    EntityProjection::query(snapshot, query, turn_text, maximum_candidates).map(|hits| {
        hits.into_iter()
            .map(|hit| RankedCandidate::neutral(hit.lsn.get().to_be_bytes().to_vec(), hit.lsn))
            .collect()
    })
}
