#![allow(clippy::cast_precision_loss, clippy::missing_errors_doc)]

use hm_core::{Error, ErrorCode, LSN};
use hm_index::hnsw::HNSW_THRESHOLD;
use hm_proj::vectors::{VectorEntry, VectorLane};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Debug, Serialize)]
pub struct ParityResult {
    pub encoder: &'static str,
    pub seed: u64,
    pub vectors: usize,
    pub queries: usize,
    pub matched: usize,
    pub expected: usize,
    pub recall_at_10: f64,
}

pub fn run() -> Result<ParityResult, Error> {
    let temporary = tempfile::tempdir().map_err(|_| Error::new(ErrorCode::OpenFailed))?;
    let lane = VectorLane::open(temporary.path(), "slice7-parity", "synthetic-int8-32", 32)?;
    let seed = 0xa826_7182_cd61_0183_u64;
    let mut state = seed;
    let mut queries = Vec::new();
    let mut entries = Vec::with_capacity(HNSW_THRESHOLD + 65);
    for ordinal in 1..=HNSW_THRESHOLD + 65 {
        let vector = vector(&mut state, 32);
        if ordinal % 997 == 0 {
            queries.push(vector.clone());
        }
        entries.push(VectorEntry {
            target_lsn: LSN::new(ordinal as u64),
            binary_prefilter: binary(&vector),
            quantized: vector,
        });
    }
    lane.replay(&entries)?;
    if lane.hnsw_checkpoint_count()? != Some(entries.len()) {
        return Err(Error::new(ErrorCode::InvariantViolation));
    }
    for _ in 0..50 {
        queries.push(vector(&mut state, 32));
    }
    let mut matched = 0;
    let mut expected = 0;
    for query in &queries {
        let prefilter = binary(query);
        let flat = lane.search_flat(query, &prefilter, 1_024, 10)?;
        let approximate = lane.search(query, &prefilter, 1_024, 10)?;
        let reference: BTreeSet<_> = flat.iter().map(|hit| hit.target_lsn).collect();
        matched += approximate
            .iter()
            .filter(|hit| reference.contains(&hit.target_lsn))
            .count();
        expected += reference.len();
        for hit in &approximate {
            if let Some(exact) = flat.iter().find(|exact| exact.target_lsn == hit.target_lsn)
                && hit != exact
            {
                return Err(Error::new(ErrorCode::InvariantViolation));
            }
        }
    }
    Ok(ParityResult {
        encoder: "synthetic_int8_32_seeded_parity",
        seed,
        vectors: entries.len(),
        queries: queries.len(),
        matched,
        expected,
        recall_at_10: if expected == 0 {
            0.0
        } else {
            matched as f64 / expected as f64
        },
    })
}

fn vector(state: &mut u64, dimensions: usize) -> Vec<i8> {
    (0..dimensions)
        .map(|_| {
            *state ^= *state << 13;
            *state ^= *state >> 7;
            *state ^= *state << 17;
            i8::try_from(*state % 127).unwrap() - 63
        })
        .collect()
}

fn binary(vector: &[i8]) -> Vec<u8> {
    let mut result = vec![0; vector.len().div_ceil(8)];
    for (index, value) in vector.iter().enumerate() {
        if *value >= 0 {
            result[index / 8] |= 1 << (index % 8);
        }
    }
    result
}
