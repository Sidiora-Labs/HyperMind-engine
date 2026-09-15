#![allow(clippy::cast_precision_loss, clippy::missing_errors_doc)]

use crate::slice1::{GateResult, Metric};
use crate::suites::{protected, temporal};
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

pub async fn run() -> Result<GateResult, Box<dyn std::error::Error>> {
    let temporal = temporal::run().await?;
    let protected = protected::run().await?;
    if temporal.axis_correct != temporal.axis_cases
        || temporal.stale_claims != 0
        || protected.rejected != protected.attempts
        || protected.surfaced_proposals != protected.attempts
    {
        return Err(Error::new(ErrorCode::InvariantViolation).into());
    }
    Ok(GateResult {
        schema_version: 1,
        slice: "slice5".to_owned(),
        encoder: "lexical_only".to_owned(),
        judge_model: None,
        suites: vec![
            "temporal_supersession".to_owned(),
            "protected_belief_types".to_owned(),
        ],
        judge_free: vec![
            Metric {
                name: "temporal_asof_accuracy".to_owned(),
                value: temporal.axis_correct as f64 / temporal.axis_cases as f64,
                unit: "ratio".to_owned(),
                tolerance: 0.0,
                higher_is_better: true,
                judged: false,
            },
            Metric {
                name: "stale_fact_rate".to_owned(),
                value: temporal.stale_fact_rate(),
                unit: "ratio".to_owned(),
                tolerance: 0.0,
                higher_is_better: false,
                judged: false,
            },
            Metric {
                name: "protected_write_rejection".to_owned(),
                value: protected.rejected as f64 / protected.attempts as f64,
                unit: "ratio".to_owned(),
                tolerance: 0.0,
                higher_is_better: true,
                judged: false,
            },
            Metric {
                name: "protected_proposal_surfacing".to_owned(),
                value: protected.surfaced_proposals as f64 / protected.attempts as f64,
                unit: "ratio".to_owned(),
                tolerance: 0.0,
                higher_is_better: true,
                judged: false,
            },
        ],
        judged: Vec::new(),
    })
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
            .args(["show", &format!("{candidate}:eval/results/slice5.json")])
            .output()?;
        if output.status.success() {
            return Ok(Some(serde_json::from_slice(&output.stdout)?));
        }
    }
    eprintln!("no slice5 baseline found at {reference}; recording initial baseline");
    Ok(None)
}

fn result_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../eval/results/slice5.json")
}
