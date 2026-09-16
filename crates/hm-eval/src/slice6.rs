#![allow(clippy::cast_precision_loss, clippy::missing_errors_doc)]

use crate::bench::longmemeval;
use crate::slice1::{GateResult, Metric};
use crate::suites::{citations, dream, generations};
use hm_core::{Error, ErrorCode};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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

#[allow(clippy::too_many_lines)]
pub async fn run() -> Result<GateResult, Box<dyn std::error::Error>> {
    let dream = dream::run()?;
    let citations = citations::run()?;
    let generations = generations::run().await?;
    let longmemeval = longmemeval::run_consolidation()?;
    if dream.regression_cases == 0
        || dream.lossy_rewrites != 0
        || dream.preserved_cases != dream.regression_cases
        || dream.duplicate_abstractions != 0
        || dream.valid_mints != 1
        || dream.ungrounded_mints != 0
        || citations.escaped_invalid != 0
        || citations.accepted != 1
        || citations.dropped + citations.accepted != citations.attempted
        || !generations.staged_hidden_after_kill
        || !generations.publish_after_restart
        || !generations.lost_ack_deduplicated
        || generations.published_generation_count != 2
        || !generations.rollback_immediate
        || !generations.leased_view_overridden
        || (longmemeval.evaluated != 0 && longmemeval.accuracy() < 0.80)
    {
        eprintln!(
            "slice6 gate failed: dream={dream:?} citations={citations:?} generations={generations:?} longmemeval={longmemeval:?}"
        );
        return Err(Error::new(ErrorCode::InvariantViolation).into());
    }
    let longmemeval_cache_rate = if longmemeval.evaluated == 0 {
        0.0
    } else {
        longmemeval.cache_hits as f64 / longmemeval.evaluated as f64
    };
    Ok(GateResult {
        schema_version: 1,
        slice: "slice6".to_owned(),
        encoder: "lexical_only".to_owned(),
        judge_model: Some(longmemeval::PINNED_JUDGE.to_owned()),
        suites: vec![
            "dream_regression".to_owned(),
            "citation_validity".to_owned(),
            "generation_publish_rollback".to_owned(),
            "longmemeval_s_consolidation".to_owned(),
        ],
        judge_free: vec![
            metric(
                "dream_lossy_rewrites",
                dream.lossy_rewrites as f64,
                "count",
                false,
            ),
            metric(
                "dream_preservation_rate",
                ratio(dream.preserved_cases, dream.regression_cases),
                "ratio",
                true,
            ),
            metric(
                "duplicate_abstraction_rate",
                dream.duplicate_abstractions as f64,
                "count",
                false,
            ),
            metric(
                "ungrounded_mint_rate",
                dream.ungrounded_mints as f64,
                "count",
                false,
            ),
            metric(
                "citation_invalid_drop_rate",
                citations.drop_rate(),
                "ratio",
                true,
            ),
            metric(
                "citation_escape_rate",
                ratio(citations.escaped_invalid, citations.attempted),
                "ratio",
                false,
            ),
            metric(
                "generation_crash_recovery",
                boolean(
                    generations.staged_hidden_after_kill
                        && generations.publish_after_restart
                        && generations.lost_ack_deduplicated,
                ),
                "boolean",
                true,
            ),
            metric(
                "generation_rollback_override",
                boolean(generations.rollback_immediate && generations.leased_view_overridden),
                "boolean",
                true,
            ),
            metric(
                "longmemeval_consolidation_coverage",
                longmemeval.coverage(),
                "ratio",
                true,
            ),
            metric(
                "longmemeval_judge_cache_hit_rate",
                longmemeval_cache_rate,
                "ratio",
                true,
            ),
        ],
        judged: vec![Metric {
            name: "longmemeval_accuracy_consolidation".to_owned(),
            value: longmemeval.accuracy(),
            unit: "ratio".to_owned(),
            tolerance: 0.0,
            higher_is_better: true,
            judged: true,
        }],
    })
}

fn metric(name: &str, value: f64, unit: &str, higher_is_better: bool) -> Metric {
    Metric {
        name: name.to_owned(),
        value,
        unit: unit.to_owned(),
        tolerance: 0.0,
        higher_is_better,
        judged: false,
    }
}

fn boolean(value: bool) -> f64 {
    if value { 1.0 } else { 0.0 }
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
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
            metric.value > previous.value + metric.tolerance
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
            .args(["show", &format!("{candidate}:eval/results/slice6.json")])
            .output()?;
        if output.status.success() {
            return Ok(Some(serde_json::from_slice(&output.stdout)?));
        }
    }
    eprintln!("no slice6 baseline found at {reference}; recording initial baseline");
    Ok(None)
}

fn result_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../eval/results/slice6.json")
}
