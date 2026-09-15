#![allow(clippy::cast_precision_loss, clippy::missing_errors_doc)]

use crate::slice1::Metric;
use crate::suites::recall::SeededVectorCorpus;
use hm_compose::bundle::{ActivationRequest, activate};
use hm_compose::tokens::{FallbackWeights, TokenCounter};
use hm_core::{ActorId, ConversationId, Error, ErrorCode};
use hm_proj::store::{ProjectionStore, ReadSnapshot};
use std::time::Instant;

const COLD_PROBES: usize = 8;
const WARM_PROBES: usize = 64;

pub fn run() -> Result<Vec<Metric>, Error> {
    let temporary = tempfile::tempdir().map_err(|_| Error::new(ErrorCode::OpenFailed))?;
    let mut metrics = Vec::new();
    for count in [10_000, 100_000] {
        let corpus = SeededVectorCorpus::build(count)
            .map_err(|_| Error::new(ErrorCode::BackendUnavailable))?;
        let store =
            ProjectionStore::open(temporary.path().join(count.to_string()), 16 * 1024 * 1024)?;
        let snapshot = store.begin_snapshot()?;
        let counter = TokenCounter::for_model("wire-fallback", None, FallbackWeights::default())?;
        let cold = samples(&corpus, &snapshot, &counter, COLD_PROBES, count)?;
        let warm = samples(&corpus, &snapshot, &counter, WARM_PROBES, count)?;
        metrics.push(metric(
            format!("activate_p50_cold_{count}"),
            percentile(&cold, 50),
        ));
        metrics.push(metric(
            format!("activate_p99_cold_{count}"),
            percentile(&cold, 99),
        ));
        metrics.push(metric(
            format!("activate_p50_warm_{count}"),
            percentile(&warm, 50),
        ));
        metrics.push(metric(
            format!("activate_p99_warm_{count}"),
            percentile(&warm, 99),
        ));
    }
    Ok(metrics)
}

fn samples(
    corpus: &SeededVectorCorpus,
    snapshot: &ReadSnapshot<'_>,
    counter: &TokenCounter,
    count: usize,
    corpus_size: usize,
) -> Result<Vec<u64>, Error> {
    let mut samples = Vec::with_capacity(count);
    for index in 0..count {
        let probe = index.wrapping_mul(7_919) % corpus_size;
        let query = (0..16)
            .map(|facet| format!("topic{probe}facet{facet}hypermind"))
            .collect::<Vec<_>>()
            .join(" ");
        let started = Instant::now();
        let _hits = corpus
            .query_index(probe, 10)
            .map_err(|_| Error::new(ErrorCode::BackendUnavailable))?;
        let _bundle = activate(
            snapshot,
            &ActivationRequest {
                actor: ActorId::new(1),
                conversation: ConversationId::derive("held-out-activation"),
                query,
                turn_text: String::new(),
                budget_tokens: 2_048,
                token_counter: counter,
                maximum_candidates: 256,
                maximum_conversation_records: 256,
            },
        )?;
        samples.push(
            u64::try_from(started.elapsed().as_nanos())
                .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?,
        );
    }
    Ok(samples)
}

fn percentile(values: &[u64], percentile: usize) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted[(sorted.len() - 1) * percentile / 100] as f64
}

fn metric(name: String, value: f64) -> Metric {
    Metric {
        name,
        value,
        unit: "nanoseconds".to_owned(),
        tolerance: 0.25,
        higher_is_better: false,
        judged: false,
    }
}
