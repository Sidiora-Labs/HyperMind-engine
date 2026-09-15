#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::missing_errors_doc
)]

use hm_compose::tokens::FallbackWeights;
use hm_core::{ActorId, ConversationId, Error, ErrorCode};
use hm_ledger::frame::EventKind;
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{Authority, EventEnvelope, EventPayload, Retention, Sensitivity, UserMsg};
use hm_serve::actor::{ActivateRequest, ActorConfig, ActorEngine, IncomingEvent, RecallRequest};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

const BATCH_EVENTS: usize = 256;
const PROBE_COUNT: usize = 20;
const WARM_ACTIVATIONS: usize = 100;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Metric {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub tolerance: f64,
    pub higher_is_better: bool,
    pub judged: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GateResult {
    pub schema_version: u32,
    pub slice: String,
    pub encoder: String,
    pub judge_model: Option<String>,
    pub suites: Vec<String>,
    pub judge_free: Vec<Metric>,
    pub judged: Vec<Metric>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SeedManifest {
    pub seed: u64,
    pub event_counts: Vec<usize>,
    pub probe_count: usize,
}

pub async fn gate(compare: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let result = run().await?;
    if let Some(reference) = compare
        && let Some(baseline) = read_baseline(reference)?
    {
        compare_results(&result, &baseline)?;
    }
    let output = result_path();
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output, serde_json::to_vec_pretty(&result)?)?;
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}

pub async fn run() -> Result<GateResult, Error> {
    let temporary = tempfile::tempdir().map_err(|_| Error::new(ErrorCode::OpenFailed))?;
    let mut metrics = Vec::new();
    for count in [1_000, 10_000] {
        let directory = temporary.path().join(count.to_string());
        let actor = seed_store(&directory, count, 0x5eed).await?;
        let probes = probe_indices(count, 0x5eed);
        let (recall_at_10, mrr) = recall_metrics(&actor, &probes).await?;
        metrics.push(Metric {
            name: format!("recall_at_10_{count}"),
            value: recall_at_10,
            unit: "ratio".to_owned(),
            tolerance: 0.01,
            higher_is_better: true,
            judged: false,
        });
        metrics.push(Metric {
            name: format!("mrr_{count}"),
            value: mrr,
            unit: "ratio".to_owned(),
            tolerance: 0.01,
            higher_is_better: true,
            judged: false,
        });
        if count == 10_000 {
            let (cold, warm) = activation_latencies(&actor, &probes).await?;
            metrics.push(latency_metric(
                "activate_p50_cold_10000",
                percentile(&cold, 50),
            ));
            metrics.push(latency_metric(
                "activate_p99_cold_10000",
                percentile(&cold, 99),
            ));
            metrics.push(latency_metric(
                "activate_p50_warm_10000",
                percentile(&warm, 50),
            ));
            metrics.push(latency_metric(
                "activate_p99_warm_10000",
                percentile(&warm, 99),
            ));
        }
        actor.shutdown().await?;
    }
    let recall = metrics
        .iter()
        .find(|metric| metric.name == "recall_at_10_10000")
        .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))?;
    if recall.value < 0.95 {
        return Err(Error::new(ErrorCode::InvariantViolation));
    }
    Ok(GateResult {
        schema_version: 1,
        slice: "slice1".to_owned(),
        encoder: "lexical_only".to_owned(),
        judge_model: None,
        suites: vec![
            "seeded_recall_1k".to_owned(),
            "seeded_recall_10k".to_owned(),
            "activation_latency_10k".to_owned(),
        ],
        judge_free: metrics,
        judged: Vec::new(),
    })
}

pub async fn seed_store(root: &Path, count: usize, seed: u64) -> Result<ActorEngine, Error> {
    if count == 0 {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let actor = ActorEngine::open(ActorConfig {
        actor_directory: root.join("1"),
        actor: ActorId::new(1),
        user: seed_bytes::<16>(seed),
        kek: seed_bytes::<32>(seed.rotate_left(17)),
        projection_map_bytes: projection_bytes(count)?,
    })
    .await?;
    let mut batch = Vec::with_capacity(BATCH_EVENTS);
    for index in 0..count {
        batch.push(seed_event(index)?);
        if batch.len() == BATCH_EVENTS || index + 1 == count {
            actor.append(std::mem::take(&mut batch)).await?;
            batch.reserve(BATCH_EVENTS);
        }
    }
    Ok(actor)
}

async fn recall_metrics(actor: &ActorEngine, probes: &[usize]) -> Result<(f64, f64), Error> {
    let mut recalled = 0_u32;
    let mut reciprocal_rank = 0.0;
    for probe in probes {
        let expected_lsn = *probe as u64 + 1;
        let items = actor
            .recall(RecallRequest::Lexical {
                query: probe_token(*probe),
                limit: 10,
            })
            .await?;
        if let Some(rank) = items.iter().position(|item| item.lsn.get() == expected_lsn) {
            recalled += 1;
            reciprocal_rank += 1.0 / (rank + 1) as f64;
        }
    }
    let denominator = probes.len() as f64;
    Ok((
        f64::from(recalled) / denominator,
        reciprocal_rank / denominator,
    ))
}

async fn activation_latencies(
    actor: &ActorEngine,
    probes: &[usize],
) -> Result<(Vec<u64>, Vec<u64>), Error> {
    let request = |probe| ActivateRequest {
        conversation: ConversationId::derive("held-out-activation"),
        query: probe_token(probe),
        turn_text: String::new(),
        budget_tokens: 2_048,
        token_weights: FallbackWeights::default(),
    };
    let mut cold = Vec::with_capacity(probes.len());
    for probe in probes {
        let started = Instant::now();
        actor.activate(request(*probe)).await?;
        cold.push(duration_ns(started)?);
    }
    let mut warm = Vec::with_capacity(WARM_ACTIVATIONS);
    for index in 0..WARM_ACTIVATIONS {
        let started = Instant::now();
        actor
            .activate(request(probes[index % probes.len()]))
            .await?;
        warm.push(duration_ns(started)?);
    }
    Ok((cold, warm))
}

fn seed_event(index: usize) -> Result<IncomingEvent, Error> {
    let content = format!(
        "seeded memory record {index} contains unique anchor {} and common context",
        probe_token(index)
    );
    Ok(IncomingEvent {
        kind: EventKind::UserMsg,
        conversation: ConversationId::derive(&format!("seed-conversation-{}", index % 97)),
        payload: encode_event_envelope(&EventEnvelope {
            schema_version: CURRENT_SCHEMA_VERSION,
            payload: EventPayload::UserMsg(Box::new(UserMsg {
                content: content.into_bytes(),
            })),
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 0,
            origin_actor: 0,
            run_id: None,
            model_provenance: None,
            authority: Authority::ExternalObserved,
            retention: Retention::Daily,
            sensitivity: Sensitivity::Public,
            event_time_ns: i64::try_from(index)
                .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?,
        }),
    })
}

#[must_use]
pub fn probe_indices(count: usize, seed: u64) -> Vec<usize> {
    (0..PROBE_COUNT)
        .map(|index| {
            let mixed = seed
                .wrapping_add(index as u64)
                .wrapping_mul(6_364_136_223_846_793_005)
                .rotate_left(23);
            mixed as usize % count
        })
        .collect()
}

fn probe_token(index: usize) -> String {
    format!("anchor{index}hypermind")
}

fn projection_bytes(count: usize) -> Result<usize, Error> {
    count
        .checked_mul(16 * 1024)
        .and_then(|bytes| bytes.checked_add(64 * 1024 * 1024))
        .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))
}

fn seed_bytes<const N: usize>(mut seed: u64) -> [u8; N] {
    let mut bytes = [0; N];
    for byte in &mut bytes {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        *byte = seed as u8;
    }
    bytes
}

fn duration_ns(started: Instant) -> Result<u64, Error> {
    u64::try_from(started.elapsed().as_nanos()).map_err(|_| Error::new(ErrorCode::CapacityExceeded))
}

fn percentile(values: &[u64], percentile: usize) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let index = (sorted.len() - 1) * percentile / 100;
    sorted[index] as f64
}

fn latency_metric(name: &str, value: f64) -> Metric {
    Metric {
        name: name.to_owned(),
        value,
        unit: "nanoseconds".to_owned(),
        tolerance: 0.25,
        higher_is_better: false,
        judged: false,
    }
}

fn compare_results(current: &GateResult, baseline: &GateResult) -> Result<(), Error> {
    for metric in &current.judge_free {
        let Some(previous) = baseline
            .judge_free
            .iter()
            .find(|candidate| candidate.name == metric.name)
        else {
            continue;
        };
        let regressed = if metric.higher_is_better {
            metric.value + metric.tolerance < previous.value
        } else {
            metric.value > previous.value * (1.0 + metric.tolerance)
                && metric.value - previous.value > 1_000_000.0
        };
        if regressed {
            return Err(Error::new(ErrorCode::InvariantViolation));
        }
    }
    Ok(())
}

fn read_baseline(reference: &str) -> Result<Option<GateResult>, Box<dyn std::error::Error>> {
    for candidate in [reference.to_owned(), format!("origin/{reference}")] {
        let output = Command::new("git")
            .args(["show", &format!("{candidate}:eval/results/slice1.json")])
            .output()?;
        if output.status.success() {
            return Ok(Some(serde_json::from_slice(&output.stdout)?));
        }
    }
    eprintln!("no slice1 baseline found at {reference}; recording initial baseline");
    Ok(None)
}

fn result_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("eval/results/slice1.json")
}
