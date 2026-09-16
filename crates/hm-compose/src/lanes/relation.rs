#![allow(clippy::missing_errors_doc)]

use crate::bundle::RetrievalLane;
use crate::fusion::{LaneRanking, Q16_ONE, RankedCandidate};
use hm_core::{Error, ErrorCode, LSN};

pub const MAXIMUM_RELATION_CANDIDATES: usize = 256;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationHit {
    pub edge_id: Vec<u8>,
    pub event_lsn: LSN,
    pub weight_micros: u32,
    pub score: i64,
    pub support_lsns: Vec<LSN>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationRanking {
    pub ranking: LaneRanking,
    pub dropped: usize,
}

pub fn rank(hits: &[RelationHit], limit: usize) -> Result<RelationRanking, Error> {
    if limit == 0 || limit > MAXIMUM_RELATION_CANDIDATES {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let mut usable = Vec::with_capacity(hits.len());
    let mut dropped = 0;
    for hit in hits {
        if hit.edge_id.is_empty() || hit.event_lsn.get() == 0 {
            dropped += 1;
            continue;
        }
        usable.push(hit);
    }
    usable.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.event_lsn.get().cmp(&right.event_lsn.get()))
            .then_with(|| left.edge_id.cmp(&right.edge_id))
    });
    usable.truncate(limit);
    Ok(RelationRanking {
        ranking: LaneRanking {
            lane: RetrievalLane::Relation,
            weight_q16: Q16_ONE,
            candidates: usable
                .into_iter()
                .map(|hit| RankedCandidate {
                    canonical_id: hit.edge_id.clone(),
                    lsn: hit.event_lsn,
                    fsrs_retrievability_q16: Q16_ONE,
                    salience_q16: salience(hit.weight_micros),
                    recency_q16: Q16_ONE,
                })
                .collect(),
        },
        dropped,
    })
}

fn salience(weight_micros: u32) -> u32 {
    u32::try_from(u64::from(weight_micros) * u64::from(Q16_ONE) / 1_000_000)
        .unwrap_or(Q16_ONE)
        .max(1)
}
