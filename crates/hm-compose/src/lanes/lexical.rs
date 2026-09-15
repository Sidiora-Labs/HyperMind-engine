#![allow(clippy::missing_errors_doc)]

use hm_core::Error;
use hm_proj::lexical::{LexicalHit, LexicalProjection};
use hm_proj::store::ReadSnapshot;

pub fn search(
    snapshot: &ReadSnapshot<'_>,
    query: &str,
    maximum_candidates: usize,
) -> Result<Vec<LexicalHit>, Error> {
    LexicalProjection::query(snapshot, query, maximum_candidates)
}
