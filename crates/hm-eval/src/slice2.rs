#![allow(clippy::cast_precision_loss, clippy::missing_errors_doc)]

use crate::slice1::{GateResult, Metric};
use crate::suites::{continuity, degradation};
use hm_core::{Error, ErrorCode};
use std::fs;
use std::path::{Path, PathBuf};

pub async fn gate(compare: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let result = run().await?;
    if let Some(reference) = compare {
        compare_result(&result, reference)?;
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
    let degradation = degradation::run()?;
    let continuity = continuity::run().await?;
    if degradation.forced_conditions != degradation.visible_conditions
        || continuity.rust_trials != continuity::TRIALS
        || !continuity.typescript_passed
    {
        return Err(Error::new(ErrorCode::InvariantViolation).into());
    }
    Ok(GateResult {
        schema_version: 1,
        slice: "slice2".to_owned(),
        encoder: "lexical_only".to_owned(),
        judge_model: None,
        suites: vec![
            "sigkill_sequence_replay".to_owned(),
            "typescript_lost_ack".to_owned(),
            "silent_degradation".to_owned(),
        ],
        judge_free: vec![
            ratio(
                "continuity_rust",
                continuity.rust_trials,
                continuity::TRIALS,
            ),
            ratio(
                "continuity_typescript",
                usize::from(continuity.typescript_passed),
                1,
            ),
            ratio(
                "degradation_visibility",
                degradation.visible_conditions,
                degradation.forced_conditions,
            ),
        ],
        judged: Vec::new(),
    })
}

fn ratio(name: &str, numerator: usize, denominator: usize) -> Metric {
    Metric {
        name: name.to_owned(),
        value: numerator as f64 / denominator as f64,
        unit: "ratio".to_owned(),
        tolerance: 0.0,
        higher_is_better: true,
        judged: false,
    }
}

fn compare_result(result: &GateResult, reference: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(reference);
    let path = if path.is_dir() {
        path.join("slice2.json")
    } else {
        path.to_owned()
    };
    if !path.exists() {
        return Ok(());
    }
    let baseline: GateResult = serde_json::from_slice(&fs::read(path)?)?;
    for metric in &result.judge_free {
        let previous = baseline
            .judge_free
            .iter()
            .find(|candidate| candidate.name == metric.name)
            .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
        if metric.value + metric.tolerance < previous.value {
            return Err(format!(
                "metric {} regressed from {} to {}",
                metric.name, previous.value, metric.value
            )
            .into());
        }
    }
    Ok(())
}

fn result_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../eval/results/slice2.json")
}
