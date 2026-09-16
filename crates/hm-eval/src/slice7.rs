#![allow(clippy::cast_precision_loss, clippy::missing_errors_doc)]

use crate::bench::{execution, gateway, locomo, longmemeval};
use crate::suites::{attention, calibration, hnsw_parity};
use serde_json::{Value, json};
use std::process::Command;

pub async fn gate(compare: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    run(compare)
        .await
        .map_err(|error| error as Box<dyn std::error::Error>)
}

#[allow(clippy::too_many_lines)]
async fn run(compare: Option<&str>) -> Result<(), gateway::DynError> {
    let root = execution::repository_root();
    let attention = attention::run()?;
    let calibration = calibration::run().await?;
    let parity = tokio::task::spawn_blocking(hnsw_parity::run).await??;
    let gateway = execution::gateway(false)?;
    let mut failures = Vec::new();
    if attention.cases < 20
        || attention.precision() < 0.90
        || attention.recall() < 0.90
        || attention.reasons_present != attention.cases
        || attention.true_interruptions == 0
    {
        failures.push("attention policy precision/recall/coverage failed".to_owned());
    }
    if calibration.cases != 37
        || calibration.assessment_mismatches != 0
        || calibration.duplicate_writes != 0
        || !calibration.restart_identical
        || !calibration.revision_required
        || calibration.per_kind.len() != 7
    {
        failures.push("prediction calibration, replay, or restart failed".to_owned());
    }
    for row in &calibration.per_kind {
        if row.counts.supported != 1
            || row.counts.pending != 1
            || row.counts.unresolvable != 1
            || row.counts.not_executed != 1
            || row.counts.contradicted
                != if row.predicate_kind == "RevisionEquals" {
                    3
                } else {
                    1
                }
        {
            failures.push(format!(
                "incorrect calibration counts for {}",
                row.predicate_kind
            ));
        }
    }
    if parity.vectors <= 50_000 || parity.queries < 100 || parity.recall_at_10 < 0.98 {
        failures.push("HNSW parity below 0.98 or insufficient coverage".to_owned());
    }
    let mut long_accuracy = 0.0;
    let long_result = match longmemeval::run_full(&gateway).await {
        Ok(result) => {
            long_accuracy = result.accuracy();
            if !result.qualified() {
                failures.push(format!(
                "LongMemEval coverage={:.3} accuracy={:.3}; requires 500/500 and accuracy >= 0.90; {:?}",
                result.coverage(), result.accuracy(), result.failure));
            }
            serde_json::to_value(result)?
        }
        Err(error) => {
            failures.push(format!("LongMemEval unavailable: {error}"));
            Value::Null
        }
    };
    let dataset_path = root.join("eval/datasets/locomo/locomo10.json");
    let dataset = tokio::task::spawn_blocking(move || locomo::load_dataset(&dataset_path)).await?;
    let mut locomo_f1 = 0.0;
    let locomo_result = match dataset {
        Ok(dataset) => match locomo::run_live(
            &dataset,
            &gateway,
            &root.join("eval/datasets/locomo/run"),
            true,
        )
        .await
        {
            Ok(result) => {
                locomo_f1 = result.judge_free.non_adversarial_f1;
                if !result.complete
                    || result.total != 1_986
                    || result.evaluated != result.total
                    || result.judge_free.non_adversarial_count != 1_540
                    || result.judge_free.adversarial_count != 446
                    || locomo_f1 < 0.75
                    || !result.judged.as_ref().is_some_and(|judge| {
                        judge.total == 1_986
                            && judge.evaluated == judge.total
                            && judge.model == gateway::JUDGE_MODEL
                    })
                {
                    failures.push(format!("LoCoMo coverage={:.3} non-adversarial F1={locomo_f1:.3}; requires complete coverage and F1 >= 0.75", result.coverage()));
                }
                serde_json::to_value(result)?
            }
            Err(error) => {
                failures.push(format!("LoCoMo incomplete: {error}"));
                Value::Null
            }
        },
        Err(error) => {
            failures.push(format!("LoCoMo unavailable: {error}"));
            Value::Null
        }
    };
    let metrics = json!({"attention_suppression_precision":attention.precision(),
        "attention_suppression_recall":attention.recall(),"hnsw_recall_at_10":parity.recall_at_10,
        "longmemeval_accuracy":long_accuracy,"locomo_non_adversarial_f1":locomo_f1});
    if let Some(reference) = compare {
        let baseline = Command::new("git")
            .args(["show", &format!("{reference}:eval/results/slice7.json")])
            .output()?;
        if baseline.status.success() {
            let baseline: Value = serde_json::from_slice(&baseline.stdout)?;
            for (name, current) in metrics.as_object().ok_or("missing metrics")? {
                if let (Some(current), Some(previous)) =
                    (current.as_f64(), baseline["metrics"][name].as_f64())
                    && current < previous
                {
                    failures.push(format!("regression in {name}: {current} < {previous}"));
                }
            }
        } else {
            eprintln!("No slice7 baseline at {reference}; absolute thresholds remain mandatory.");
        }
    }
    let report = json!({"schema_version":1,"slice":"slice7","passed":failures.is_empty(),
        "encoder":"lexical_only","metrics":metrics,"failures":failures,
        "attention":attention,"calibration":calibration,"synthetic_vector_parity":parity,
        "longmemeval":long_result,"locomo":locomo_result,"provider_budget":gateway.budget()?});
    gateway::write_json(&root.join("eval/results/slice7.json"), &report)?;
    println!("{}", serde_json::to_string(&report)?);
    if report["passed"] != true {
        return Err("slice7 acceptance thresholds not met; measured failures recorded".into());
    }
    Ok(())
}
