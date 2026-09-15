#![no_main]

use hm_ledger::mmr::{Hash, Mmr};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut mmr = Mmr::default();
    for chunk in data.chunks(32).take(4096) {
        let mut leaf = Hash::default();
        leaf[..chunk.len()].copy_from_slice(chunk);
        let _ = mmr.append(leaf);
    }
    if mmr.leaf_count() > 0 {
        let start = u64::from(data.first().copied().unwrap_or_default()) % mmr.leaf_count();
        let count = 1 + u64::from(data.get(1).copied().unwrap_or_default())
            % (mmr.leaf_count() - start);
        if let Ok(proof) = mmr.prove_range(start, count) {
            let first = usize::try_from(start).unwrap_or_default();
            let last = usize::try_from(start + count).unwrap_or(first);
            let _ = Mmr::verify_range(&mmr.leaves()[first..last], &proof);
        }
    }
});
