#![allow(clippy::cast_precision_loss, clippy::missing_errors_doc)]

use crate::bench::longmemeval;
use crate::slice1::{GateResult, Metric};
use crate::suites::{geometry, latency, recall};
use hm_core::{Error, ErrorCode};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn gate(compare: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let local = local_encoder_run()?;
    let result = run()?;
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
    fs::write(local_result_path(), serde_json::to_vec_pretty(&local)?)?;
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}

fn local_encoder_run() -> Result<GateResult, Box<dyn std::error::Error>> {
    let run = recall::run_local()?;
    require(&run.metrics, "semantic_recall_at_10_10000", |value| {
        value >= 0.95
    })?;
    Ok(GateResult {
        schema_version: 1,
        slice: "slice4-local".to_owned(),
        encoder: run.encoder,
        judge_model: None,
        suites: vec!["semantic_recall_1k_10k_100k".to_owned()],
        judge_free: run.metrics,
        judged: Vec::new(),
    })
}

pub fn run() -> Result<GateResult, Box<dyn std::error::Error>> {
    let mut judge_free = recall::run()?;
    judge_free.extend(latency::run()?);
    let longmemeval = longmemeval::run_baseline()?;
    judge_free.push(Metric {
        name: "longmemeval_coverage".to_owned(),
        value: longmemeval.coverage(),
        unit: "ratio".to_owned(),
        tolerance: 0.0,
        higher_is_better: true,
        judged: false,
    });
    judge_free.push(Metric {
        name: "longmemeval_judge_cache_hit_rate".to_owned(),
        value: if longmemeval.evaluated == 0 {
            0.0
        } else {
            longmemeval.cache_hits as f64 / longmemeval.evaluated as f64
        },
        unit: "ratio".to_owned(),
        tolerance: 0.0,
        higher_is_better: true,
        judged: false,
    });
    let geometry = geometry::run()?;
    judge_free.push(Metric {
        name: "geometry_baseline_mrr_at_10".to_owned(),
        value: geometry.baseline_mrr_at_10,
        unit: "ratio".to_owned(),
        tolerance: 0.0,
        higher_is_better: true,
        judged: false,
    });
    judge_free.push(Metric {
        name: "geometry_boosted_mrr_at_10".to_owned(),
        value: geometry.boosted_mrr_at_10,
        unit: "ratio".to_owned(),
        tolerance: 0.0,
        higher_is_better: true,
        judged: false,
    });
    judge_free.push(Metric {
        name: "geometry_boost_enabled".to_owned(),
        value: if geometry.enabled_by_default {
            1.0
        } else {
            0.0
        },
        unit: "boolean".to_owned(),
        tolerance: 0.0,
        higher_is_better: true,
        judged: false,
    });
    require(&judge_free, "recall_at_10_10000", |value| value >= 0.95)?;
    let latency_target_met = judge_free
        .iter()
        .find(|metric| metric.name == "activate_p99_warm_100000")
        .is_some_and(|metric| metric.value < 10_000_000.0);
    judge_free.push(Metric {
        name: "activate_p99_target_met_100000".to_owned(),
        value: if latency_target_met { 1.0 } else { 0.0 },
        unit: "boolean".to_owned(),
        tolerance: 0.0,
        higher_is_better: true,
        judged: false,
    });
    Ok(GateResult {
        schema_version: 1,
        slice: "slice4".to_owned(),
        encoder: "lexical_only".to_owned(),
        judge_model: Some(longmemeval::PINNED_JUDGE.to_owned()),
        suites: vec![
            "seeded_vector_recall_1k_10k_100k".to_owned(),
            "activation_latency_10k_100k".to_owned(),
            "longmemeval_s_no_consolidation".to_owned(),
            "geometry_alignment_ablation".to_owned(),
        ],
        judge_free,
        judged: vec![Metric {
            name: "longmemeval_accuracy_no_consolidation".to_owned(),
            value: longmemeval.accuracy(),
            unit: "ratio".to_owned(),
            tolerance: 0.0,
            higher_is_better: true,
            judged: true,
        }],
    })
}

fn require(
    metrics: &[Metric],
    name: &str,
    predicate: impl FnOnce(f64) -> bool,
) -> Result<(), Error> {
    let value = metrics
        .iter()
        .find(|metric| metric.name == name)
        .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))?
        .value;
    if predicate(value) {
        Ok(())
    } else {
        eprintln!("slice4 gate failed: {name}={value}");
        Err(Error::new(ErrorCode::InvariantViolation))
    }
}

fn compare_results(current: &GateResult, baseline: &GateResult) -> Result<(), Error> {
    for metric in current.judge_free.iter().chain(&current.judged) {
        let Some(previous) = baseline
            .judge_free
            .iter()
            .chain(&baseline.judged)
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
            .args(["show", &format!("{candidate}:eval/results/slice4.json")])
            .output()?;
        if output.status.success() {
            return Ok(Some(serde_json::from_slice(&output.stdout)?));
        }
    }
    eprintln!("no slice4 baseline found at {reference}; recording initial baseline");
    Ok(None)
}

fn result_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../eval/results/slice4.json")
}

fn local_result_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../eval/results/slice4-local.json")
}
