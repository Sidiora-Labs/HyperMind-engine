#![allow(clippy::missing_errors_doc)]

use crate::bundle::RetrievalLane;
use crate::fusion::{LaneRanking, Q16_ONE, RankedCandidate};
use hm_core::{Error, ErrorCode, LSN};
use hm_proj::graph::{EdgeRecord, GraphProjection};
use hm_proj::store::ReadSnapshot;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const MAXIMUM_VISITED_NODES: usize = 200;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphSeed {
    pub node_id: Vec<u8>,
    pub source_lsn: LSN,
}

pub fn search(
    snapshot: &ReadSnapshot<'_>,
    generation: u64,
    seeds: &[GraphSeed],
    query: &str,
    valid_at_ns: i64,
    maximum_hops: u8,
    limit: usize,
) -> Result<LaneRanking, Error> {
    if seeds.is_empty()
        || query.trim().is_empty()
        || !(1..=2).contains(&maximum_hops)
        || limit == 0
        || limit > MAXIMUM_VISITED_NODES
        || seeds.iter().any(|seed| seed.node_id.is_empty())
    {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let query_terms = terms(query);
    let seed_nodes = seeds
        .iter()
        .map(|seed| seed.node_id.clone())
        .collect::<BTreeSet<_>>();
    let mut visited = seed_nodes.clone();
    if visited.len() > MAXIMUM_VISITED_NODES {
        return Err(Error::new(ErrorCode::CapacityExceeded));
    }
    let mut queue = seeds
        .iter()
        .map(|seed| (seed.node_id.clone(), 0_u8))
        .collect::<VecDeque<_>>();
    let mut hits = BTreeMap::<Vec<u8>, GraphHit>::new();
    while let Some((node, depth)) = queue.pop_front() {
        if depth >= maximum_hops || visited.len() >= MAXIMUM_VISITED_NODES {
            continue;
        }
        let neighbours = GraphProjection::neighbours(
            snapshot,
            generation,
            &node,
            valid_at_ns,
            MAXIMUM_VISITED_NODES - visited.len(),
        )?;
        for edge in neighbours {
            let Some(target) = other_endpoint(&edge, &node) else {
                continue;
            };
            if seed_nodes.contains(target) {
                continue;
            }
            let overlap = relation_overlap(&query_terms, &edge.relation);
            if overlap == 0 {
                continue;
            }
            let hop = depth + 1;
            let candidate = GraphHit {
                node_id: target.to_vec(),
                event_lsn: LSN::new(edge.event_lsn),
                hop,
                relation_overlap: overlap,
                weight_micros: edge.weight_micros,
            };
            hits.entry(candidate.node_id.clone())
                .and_modify(|existing| {
                    if candidate.rank_key() < existing.rank_key() {
                        *existing = candidate.clone();
                    }
                })
                .or_insert(candidate);
            if visited.insert(target.to_vec()) && visited.len() <= MAXIMUM_VISITED_NODES {
                queue.push_back((target.to_vec(), hop));
            }
        }
    }
    let mut hits = hits.into_values().collect::<Vec<_>>();
    hits.sort_by(|left, right| left.rank_key().cmp(&right.rank_key()));
    hits.truncate(limit);
    Ok(LaneRanking {
        lane: RetrievalLane::Graph,
        weight_q16: Q16_ONE,
        candidates: hits
            .into_iter()
            .map(|hit| RankedCandidate {
                canonical_id: hit.node_id,
                lsn: hit.event_lsn,
                fsrs_retrievability_q16: Q16_ONE,
                salience_q16: u32::try_from(
                    u64::from(hit.weight_micros) * u64::from(Q16_ONE) / 1_000_000,
                )
                .unwrap_or(Q16_ONE)
                .max(1),
                recency_q16: Q16_ONE,
            })
            .collect(),
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GraphHit {
    node_id: Vec<u8>,
    event_lsn: LSN,
    hop: u8,
    relation_overlap: usize,
    weight_micros: u32,
}

impl GraphHit {
    fn rank_key(&self) -> (u8, std::cmp::Reverse<usize>, std::cmp::Reverse<u32>, &[u8]) {
        (
            self.hop,
            std::cmp::Reverse(self.relation_overlap),
            std::cmp::Reverse(self.weight_micros),
            &self.node_id,
        )
    }
}

fn other_endpoint<'a>(edge: &'a EdgeRecord, node: &[u8]) -> Option<&'a [u8]> {
    if edge.source_id == node {
        Some(&edge.target_id)
    } else if edge.target_id == node {
        Some(&edge.source_id)
    } else {
        None
    }
}

fn relation_overlap(query_terms: &BTreeSet<String>, relation: &str) -> usize {
    terms(relation).intersection(query_terms).count()
}

fn terms(value: &str) -> BTreeSet<String> {
    value
        .to_lowercase()
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|term| term.len() >= 3)
        .map(str::to_owned)
        .collect()
}
