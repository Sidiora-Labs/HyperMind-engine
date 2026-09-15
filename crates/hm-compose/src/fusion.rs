#![allow(clippy::missing_errors_doc)]

use crate::bundle::{RetrievalLane, WhyCode};
use hm_core::{ActorId, ConversationId, Error, ErrorCode, LSN};
use std::collections::{BTreeMap, BTreeSet};

pub const Q16_ONE: u32 = 1 << 16;
const RRF_K: u64 = 60;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RankedCandidate {
    pub canonical_id: Vec<u8>,
    pub lsn: LSN,
    pub fsrs_retrievability_q16: u32,
    pub salience_q16: u32,
    pub recency_q16: u32,
}

impl RankedCandidate {
    #[must_use]
    pub fn neutral(canonical_id: Vec<u8>, lsn: LSN) -> Self {
        Self {
            canonical_id,
            lsn,
            fsrs_retrievability_q16: Q16_ONE,
            salience_q16: Q16_ONE,
            recency_q16: Q16_ONE,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LaneRanking {
    pub lane: RetrievalLane,
    pub weight_q16: u32,
    pub candidates: Vec<RankedCandidate>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FusedHit {
    pub canonical_id: Vec<u8>,
    pub lsn: LSN,
    pub score_q32: u64,
    pub lane_ranks: BTreeMap<RetrievalLane, u32>,
    pub why: WhyCode,
    pub uri: String,
}

pub fn fuse(
    actor: ActorId,
    conversation: ConversationId,
    rankings: &[LaneRanking],
    limit: usize,
) -> Result<Vec<FusedHit>, Error> {
    if actor.get() == 0 || rankings.is_empty() || limit == 0 {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let mut accumulated = BTreeMap::<Vec<u8>, Accumulator>::new();
    for ranking in rankings {
        if ranking.weight_q16 == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let mut seen = BTreeSet::new();
        for (index, candidate) in ranking.candidates.iter().enumerate() {
            if candidate.canonical_id.is_empty()
                || candidate.lsn.get() == 0
                || candidate.fsrs_retrievability_q16 > Q16_ONE
                || candidate.recency_q16 > Q16_ONE
                || candidate.salience_q16 == 0
                || !seen.insert(candidate.canonical_id.as_slice())
            {
                return Err(Error::new(ErrorCode::InvalidArgument));
            }
            let rank =
                u32::try_from(index + 1).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
            let denominator = RRF_K + u64::from(rank);
            let contribution = (u64::from(ranking.weight_q16) << 16) / denominator;
            let entry = accumulated
                .entry(candidate.canonical_id.clone())
                .or_insert_with(|| Accumulator::new(candidate));
            if entry.lsn != candidate.lsn {
                return Err(Error::new(ErrorCode::InvariantViolation));
            }
            entry.rrf_q32 = entry
                .rrf_q32
                .checked_add(contribution)
                .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?;
            entry.ranks.insert(ranking.lane, rank);
            entry.fsrs = entry.fsrs.max(candidate.fsrs_retrievability_q16);
            entry.salience = entry.salience.max(candidate.salience_q16);
            entry.recency = entry.recency.max(candidate.recency_q16);
        }
    }
    let mut hits = accumulated
        .into_iter()
        .map(|(canonical_id, entry)| {
            let score_q32 = composite(entry.rrf_q32, entry.fsrs, entry.salience, entry.recency)?;
            let why = why_code(&entry.ranks);
            let uri = provenance_uri(actor, conversation, entry.lsn, score_q32, &entry.ranks, why);
            Ok(FusedHit {
                canonical_id,
                lsn: entry.lsn,
                score_q32,
                lane_ranks: entry.ranks,
                why,
                uri,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    hits.sort_by(|left, right| {
        right
            .score_q32
            .cmp(&left.score_q32)
            .then_with(|| left.canonical_id.cmp(&right.canonical_id))
    });
    hits.truncate(limit);
    Ok(hits)
}

struct Accumulator {
    lsn: LSN,
    rrf_q32: u64,
    ranks: BTreeMap<RetrievalLane, u32>,
    fsrs: u32,
    salience: u32,
    recency: u32,
}

impl Accumulator {
    fn new(candidate: &RankedCandidate) -> Self {
        Self {
            lsn: candidate.lsn,
            rrf_q32: 0,
            ranks: BTreeMap::new(),
            fsrs: candidate.fsrs_retrievability_q16,
            salience: candidate.salience_q16,
            recency: candidate.recency_q16,
        }
    }
}

fn composite(rrf: u64, fsrs: u32, salience: u32, recency: u32) -> Result<u64, Error> {
    let denominator = u128::from(Q16_ONE).pow(3);
    let value = u128::from(rrf)
        .checked_mul(u128::from(fsrs))
        .and_then(|value| value.checked_mul(u128::from(salience)))
        .and_then(|value| value.checked_mul(u128::from(recency)))
        .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?
        / denominator;
    u64::try_from(value).map_err(|_| Error::new(ErrorCode::CapacityExceeded))
}

fn why_code(ranks: &BTreeMap<RetrievalLane, u32>) -> WhyCode {
    if ranks.len() != 1 {
        return WhyCode::Fused;
    }
    match ranks.first_key_value().map(|(lane, _)| *lane) {
        Some(RetrievalLane::Lexical) => WhyCode::Lexical,
        Some(RetrievalLane::Vector) => WhyCode::Vector,
        Some(RetrievalLane::Entity) => WhyCode::Entity,
        Some(RetrievalLane::Temporal) => WhyCode::Temporal,
        _ => WhyCode::Fused,
    }
}

fn provenance_uri(
    actor: ActorId,
    conversation: ConversationId,
    lsn: LSN,
    score: u64,
    ranks: &BTreeMap<RetrievalLane, u32>,
    why: WhyCode,
) -> String {
    let ranks = ranks
        .iter()
        .map(|(lane, rank)| format!("{}:{rank}", lane_name(*lane)))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "hm://{actor}/{conversation}/{lsn}?score={score}&ranks={ranks}&why={}",
        why_name(why)
    )
}

const fn lane_name(lane: RetrievalLane) -> &'static str {
    match lane {
        RetrievalLane::Lexical => "lexical",
        RetrievalLane::Vector => "vector",
        RetrievalLane::Entity => "entity",
        RetrievalLane::Temporal => "temporal",
        RetrievalLane::Graph => "graph",
        RetrievalLane::Belief => "belief",
        RetrievalLane::Timeline => "timeline",
        RetrievalLane::Reconstruct => "reconstruct",
    }
}

const fn why_name(why: WhyCode) -> &'static str {
    match why {
        WhyCode::Conversation => "conversation",
        WhyCode::Lexical => "lexical",
        WhyCode::Vector => "vector",
        WhyCode::Entity => "entity",
        WhyCode::Temporal => "temporal",
        WhyCode::Fused => "fused",
        WhyCode::Intent => "intent",
        WhyCode::Binding => "binding",
        WhyCode::WorkLedger => "work_ledger",
        WhyCode::Belief => "belief",
        WhyCode::Conflict => "conflict",
    }
}
