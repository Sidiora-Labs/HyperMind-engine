#![allow(clippy::missing_errors_doc)]

use crate::actor::ActorEngine;
use hm_core::{Error, LSN};
use hm_schema::wire::{DeleteResult, LatencyBucket, LatencyResult, RebuildResult, VerifyResult};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const LATENCY_BOUNDS_NS: [u64; 4] = [100_000, 1_000_000, 10_000_000, u64::MAX];

#[derive(Clone, Default)]
pub struct LatencyHistograms {
    counts: Arc<Mutex<BTreeMap<String, [u64; LATENCY_BOUNDS_NS.len()]>>>,
}

impl LatencyHistograms {
    pub fn observe(&self, operation: &str, elapsed: Duration) {
        let nanos = u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX);
        let mut counts = self
            .counts
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let buckets = counts.entry(operation.to_owned()).or_default();
        let index = LATENCY_BOUNDS_NS
            .iter()
            .position(|bound| nanos <= *bound)
            .unwrap_or(LATENCY_BOUNDS_NS.len() - 1);
        buckets[index] = buckets[index].saturating_add(1);
    }

    #[must_use]
    pub fn snapshot(&self) -> LatencyResult {
        let counts = self
            .counts
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut buckets = Vec::new();
        for (operation, operation_counts) in counts.iter() {
            for (upper_bound_ns, count) in LATENCY_BOUNDS_NS.into_iter().zip(operation_counts) {
                buckets.push(LatencyBucket {
                    operation: operation.clone(),
                    upper_bound_ns,
                    count: *count,
                });
            }
        }
        LatencyResult { buckets }
    }
}

pub async fn verify(actor: &ActorEngine) -> Result<VerifyResult, Error> {
    let status = actor.verification_status().await?;
    Ok(VerifyResult {
        actor: actor.actor().get(),
        root: status.root.to_vec(),
        leaf_count: status.leaf_count,
        last_checkpoint_lsn: status.last_checkpoint_lsn.get(),
        verified: status.verified,
    })
}

pub async fn rebuild(actor: &ActorEngine, name: String) -> Result<RebuildResult, Error> {
    let applied_lsn = actor.rebuild_projection(name.clone()).await?;
    Ok(RebuildResult {
        actor: actor.actor().get(),
        name,
        applied_lsn: applied_lsn.get(),
    })
}

pub async fn crypto_delete(actor: &ActorEngine) -> Result<DeleteResult, Error> {
    Ok(DeleteResult {
        actor: actor.actor().get(),
        receipt: actor.crypto_delete().await?,
    })
}

pub async fn rotate_keys(
    actor: &ActorEngine,
    new_kek: hm_ledger::keyring::KeyEncryptionKey,
) -> Result<LSN, Error> {
    actor.rotate_keys(new_kek).await?;
    Ok(actor.stats().await?.applied.last_lsn)
}
