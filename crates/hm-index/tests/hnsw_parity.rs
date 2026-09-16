#![cfg(feature = "hnsw")]
#![allow(clippy::cast_precision_loss)]
#![forbid(unsafe_code)]

use hm_core::{ErrorCode, LSN};
use hm_index::hnsw::{HNSW_THRESHOLD, HnswIndex};
use hm_proj::vectors::{VectorEntry, VectorLane};
use std::collections::BTreeSet;

#[test]
fn hnsw_parity_and_checkpoint_replay_above_fifty_thousand_vectors() {
    let directory = tempfile::tempdir().expect("real vector directory");
    let lane = VectorLane::open(directory.path(), "generation-a", "space-a", 32)
        .expect("create vector lane");
    let mut state = 0xa826_7182_cd61_0183_u64;
    let mut queries = Vec::new();
    let mut prefix = Vec::with_capacity(HNSW_THRESHOLD);
    for ordinal in 1..=HNSW_THRESHOLD + 65 {
        let vector = vector(&mut state, 32);
        if ordinal <= HNSW_THRESHOLD {
            prefix.push(VectorEntry {
                target_lsn: LSN::new(ordinal as u64),
                quantized: vector.clone(),
                binary_prefilter: binary(&vector),
            });
        } else {
            lane.append(LSN::new(ordinal as u64), &vector, &binary(&vector))
                .expect("append canonical vector");
        }
        if ordinal == HNSW_THRESHOLD {
            lane.replay(&prefix).expect("replay canonical flat prefix");
            assert_eq!(lane.hnsw_checkpoint_count().unwrap(), None);
        }
        if ordinal % 997 == 0 {
            queries.push(vector);
        }
    }
    assert_eq!(
        lane.hnsw_checkpoint_count().unwrap(),
        Some(HNSW_THRESHOLD + 1)
    );
    for _ in 0..50 {
        queries.push(vector(&mut state, 32));
    }
    let mut matched = 0;
    let mut expected = 0;
    let mut original_results = Vec::new();
    for query in &queries {
        let prefilter = binary(query);
        let flat = lane.search_flat(query, &prefilter, 1_024, 10).unwrap();
        let approximate = lane.search(query, &prefilter, 1_024, 10).unwrap();
        let reference: BTreeSet<_> = flat.iter().map(|hit| hit.target_lsn).collect();
        matched += approximate
            .iter()
            .filter(|hit| reference.contains(&hit.target_lsn))
            .count();
        expected += reference.len();
        for hit in &approximate {
            if let Some(exact) = flat.iter().find(|exact| exact.target_lsn == hit.target_lsn) {
                assert_eq!(hit, exact);
            }
        }
        original_results.push(approximate);
    }
    let recall_at_10 = matched as f64 / expected as f64;
    eprintln!(
        "HNSW parity: vectors={} queries={} matched={matched}/{expected} recall@10={recall_at_10:.4}",
        HNSW_THRESHOLD + 65,
        queries.len()
    );
    assert!(recall_at_10 >= 0.98, "HNSW recall@10 was {recall_at_10:.4}");
    let canonical = lane.canonical_bytes().unwrap();
    let checkpoint_path = lane.path().with_extension("hnsw");
    let before_restart = std::fs::read(&checkpoint_path).unwrap();
    drop(lane);
    let reopened = VectorLane::open(directory.path(), "generation-a", "space-a", 32)
        .expect("replay uncheckpointed tail");
    assert_eq!(reopened.canonical_bytes().unwrap(), canonical);
    assert_eq!(
        reopened.hnsw_checkpoint_count().unwrap(),
        Some(HNSW_THRESHOLD + 65)
    );
    for (query, original) in queries.iter().zip(original_results).take(10) {
        assert_eq!(
            reopened.search(query, &binary(query), 1_024, 10).unwrap(),
            original
        );
    }
    let other = VectorLane::open(directory.path(), "generation-b", "space-b", 32).unwrap();
    assert_eq!(other.hnsw_checkpoint_count().unwrap(), None);
    assert!(
        other
            .search(&queries[0], &binary(&queries[0]), 1_024, 10)
            .unwrap()
            .is_empty()
    );
    drop(reopened);
    let checkpoint = std::fs::read(&checkpoint_path).unwrap();
    assert_ne!(checkpoint, before_restart);
    let mut corrupted = checkpoint;
    *corrupted.last_mut().unwrap() ^= 1;
    std::fs::write(&checkpoint_path, corrupted).unwrap();
    assert_eq!(
        VectorLane::open(directory.path(), "generation-a", "space-a", 32)
            .unwrap_err()
            .code,
        ErrorCode::VectorIndexCorrupt
    );
}

#[test]
fn native_index_is_reproducible_and_rejects_invalid_inputs() {
    let first = HnswIndex::new(32, 1).unwrap();
    let second = HnswIndex::new(32, 1).unwrap();
    let mut state = 17;
    for key in 1..=256 {
        let vector = vector(&mut state, 32);
        first.add(key, &vector).unwrap();
        second.add(key, &vector).unwrap();
    }
    assert_eq!(first.checkpoint().unwrap(), second.checkpoint().unwrap());
    assert_eq!(
        first.add(1, &[0; 32]).unwrap_err().code,
        ErrorCode::InvalidArgument
    );
    assert_eq!(
        first.add(300, &[0; 31]).unwrap_err().code,
        ErrorCode::InvalidArgument
    );
    let restored = HnswIndex::restore(32, &first.checkpoint().unwrap()).unwrap();
    let admitted = (1..=256).collect();
    let query = vector(&mut state, 32);
    assert_eq!(
        first.search(&query, &admitted, 10).unwrap(),
        restored.search(&query, &admitted, 10).unwrap()
    );
    assert!(HnswIndex::restore(31, &first.checkpoint().unwrap()).is_err());
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
