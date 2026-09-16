#![allow(clippy::missing_errors_doc)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use crate::citations::FrozenCandidate;
use hm_core::{Error, ErrorCode};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingObservation {
    pub source: FrozenCandidate,
    pub salience_micros: u32,
    pub event_time_ns: i64,
    pub embedding: Vec<i8>,
    pub entities: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClusterOptions {
    pub maximum_observations: usize,
    pub cosine_threshold_micros: u32,
    pub entity_overlap_threshold_micros: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservationCluster {
    pub cluster_id: [u8; 32],
    pub priority: u64,
    pub observations: Vec<PendingObservation>,
}

pub fn cluster_observations(
    observations: &[PendingObservation],
    options: ClusterOptions,
) -> Result<Vec<ObservationCluster>, Error> {
    validate(observations, options)?;
    if observations.is_empty() {
        return Ok(Vec::new());
    }
    let minimum_time = observations
        .iter()
        .map(|observation| observation.event_time_ns)
        .min()
        .unwrap_or(0);
    let maximum_time = observations
        .iter()
        .map(|observation| observation.event_time_ns)
        .max()
        .unwrap_or(0);
    let mut sampled = observations
        .iter()
        .cloned()
        .map(|observation| {
            let priority = priority(&observation, minimum_time, maximum_time);
            let tie_break = *blake3::hash(&observation.source.content).as_bytes();
            (Reverse(priority), tie_break, observation)
        })
        .collect::<Vec<_>>();
    sampled.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
    sampled.truncate(options.maximum_observations.min(sampled.len()));
    let sampled = sampled
        .into_iter()
        .map(|(_, _, observation)| observation)
        .collect::<Vec<_>>();

    let mut parents = (0..sampled.len()).collect::<Vec<_>>();
    for left in 0..sampled.len() {
        for right in left + 1..sampled.len() {
            if connected(&sampled[left], &sampled[right], options) {
                union(&mut parents, left, right);
            }
        }
    }
    let mut grouped = BTreeMap::<usize, Vec<PendingObservation>>::new();
    for (index, observation) in sampled.into_iter().enumerate() {
        let root = find(&mut parents, index);
        grouped.entry(root).or_default().push(observation);
    }
    let mut clusters = grouped
        .into_values()
        .map(|mut observations| {
            observations.sort_by_key(|observation| {
                (
                    Reverse(priority(observation, minimum_time, maximum_time)),
                    *blake3::hash(&observation.source.content).as_bytes(),
                )
            });
            let priority = observations
                .iter()
                .map(|observation| priority(observation, minimum_time, maximum_time))
                .max()
                .unwrap_or(0);
            ObservationCluster {
                cluster_id: cluster_id(&observations),
                priority,
                observations,
            }
        })
        .collect::<Vec<_>>();
    clusters.sort_by_key(|cluster| (Reverse(cluster.priority), cluster.cluster_id));
    Ok(clusters)
}

fn validate(observations: &[PendingObservation], options: ClusterOptions) -> Result<(), Error> {
    if options.maximum_observations == 0
        || options.cosine_threshold_micros == 0
        || options.entity_overlap_threshold_micros == 0
        || options.cosine_threshold_micros > 1_000_000
        || options.entity_overlap_threshold_micros > 1_000_000
        || observations.iter().any(|observation| {
            observation.source.lsn == 0
                || observation.source.content.is_empty()
                || observation.salience_micros > 1_000_000
        })
    {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let mut lsns = BTreeSet::new();
    if observations
        .iter()
        .any(|observation| !lsns.insert(observation.source.lsn))
    {
        return Err(Error::new(ErrorCode::AlreadyExists));
    }
    Ok(())
}

fn priority(observation: &PendingObservation, minimum_time: i64, maximum_time: i64) -> u64 {
    let recency = if maximum_time == minimum_time {
        1_000_000_u64
    } else {
        let elapsed = i128::from(observation.event_time_ns) - i128::from(minimum_time);
        let range = i128::from(maximum_time) - i128::from(minimum_time);
        u64::try_from(elapsed.saturating_mul(1_000_000).div_euclid(range)).unwrap_or(0)
    };
    u64::from(observation.salience_micros)
        .saturating_mul(3)
        .saturating_add(recency)
}

fn connected(
    left: &PendingObservation,
    right: &PendingObservation,
    options: ClusterOptions,
) -> bool {
    cosine_micros(&left.embedding, &right.embedding)
        .is_some_and(|score| score >= options.cosine_threshold_micros)
        || entity_overlap_micros(&left.entities, &right.entities)
            >= options.entity_overlap_threshold_micros
}

fn cosine_micros(left: &[i8], right: &[i8]) -> Option<u32> {
    if left.is_empty() || left.len() != right.len() {
        return None;
    }
    let dot = left
        .iter()
        .zip(right)
        .map(|(left, right)| i64::from(*left) * i64::from(*right))
        .sum::<i64>();
    let left_norm = left
        .iter()
        .map(|value| i64::from(*value).pow(2))
        .sum::<i64>();
    let right_norm = right
        .iter()
        .map(|value| i64::from(*value).pow(2))
        .sum::<i64>();
    if dot <= 0 || left_norm == 0 || right_norm == 0 {
        return Some(0);
    }
    let denominator = (left_norm as f64).sqrt() * (right_norm as f64).sqrt();
    Some(((dot as f64 / denominator) * 1_000_000.0).clamp(0.0, 1_000_000.0) as u32)
}

fn entity_overlap_micros(left: &[String], right: &[String]) -> u32 {
    let left = left
        .iter()
        .map(|entity| entity.to_lowercase())
        .collect::<BTreeSet<_>>();
    let right = right
        .iter()
        .map(|entity| entity.to_lowercase())
        .collect::<BTreeSet<_>>();
    let denominator = left.len().min(right.len());
    if denominator == 0 {
        return 0;
    }
    let intersection = left.intersection(&right).count();
    u32::try_from(intersection.saturating_mul(1_000_000) / denominator).unwrap_or(1_000_000)
}

fn find(parents: &mut [usize], index: usize) -> usize {
    if parents[index] != index {
        parents[index] = find(parents, parents[index]);
    }
    parents[index]
}

fn union(parents: &mut [usize], left: usize, right: usize) {
    let left = find(parents, left);
    let right = find(parents, right);
    let root = left.min(right);
    parents[left] = root;
    parents[right] = root;
}

fn cluster_id(observations: &[PendingObservation]) -> [u8; 32] {
    let mut roots = observations
        .iter()
        .map(|observation| {
            (
                observation.source.source_root,
                observation.source.conversation,
                observation.source.lsn,
            )
        })
        .collect::<Vec<_>>();
    roots.sort_unstable();
    let mut hasher = blake3::Hasher::new();
    for (root, conversation, lsn) in roots {
        hasher.update(&root);
        hasher.update(&conversation);
        hasher.update(&lsn.to_le_bytes());
    }
    *hasher.finalize().as_bytes()
}
