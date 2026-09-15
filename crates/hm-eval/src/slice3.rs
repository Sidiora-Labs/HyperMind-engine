#![allow(clippy::cast_precision_loss, clippy::missing_errors_doc)]

use crate::slice1::{GateResult, Metric};
use crate::suites::laundering;
use hm_core::{Error, ErrorCode};
use std::fs;
use std::path::{Path, PathBuf};

pub fn gate(compare: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let result = run()?;
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

pub fn run() -> Result<GateResult, Box<dyn std::error::Error>> {
    let result = laundering::run()?;
    if result.violations != 0 || result.attempts != laundering::ATTEMPTS {
        return Err(Error::new(ErrorCode::InvariantViolation).into());
    }
    Ok(GateResult {
        schema_version: 1,
        slice: "slice3".to_owned(),
        encoder: "lexical_only".to_owned(),
        judge_model: None,
        suites: vec!["authority_laundering".to_owned()],
        judge_free: vec![Metric {
            name: "laundering_rejection".to_owned(),
            value: (result.attempts - result.violations) as f64 / result.attempts as f64,
            unit: "ratio".to_owned(),
            tolerance: 0.0,
            higher_is_better: true,
            judged: false,
        }],
        judged: Vec::new(),
    })
}

fn compare_result(result: &GateResult, reference: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(reference);
    let path = if path.is_dir() {
        path.join("slice3.json")
    } else {
        path.to_owned()
    };
    if !path.exists() {
        return Ok(());
    }
    let baseline: GateResult = serde_json::from_slice(&fs::read(path)?)?;
    let current = &result.judge_free[0];
    let previous = baseline
        .judge_free
        .iter()
        .find(|metric| metric.name == current.name)
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
    if current.value + current.tolerance < previous.value {
        return Err("laundering rejection regressed".into());
    }
    Ok(())
}

fn result_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../eval/results/slice3.json")
}
