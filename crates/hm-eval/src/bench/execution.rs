#![allow(clippy::missing_errors_doc)]

use super::gateway::{DynError, Gateway, JUDGE_MODEL, MAXIMUM_BUDGET_MICROUSD, READER_MODEL};
use super::{beam, beam_run, judge_diagnostic, locomo, longmemeval};
use std::path::{Path, PathBuf};

#[must_use]
pub fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn gateway(live: bool) -> Result<Gateway, DynError> {
    Gateway::open(
        repository_root().join("eval/cache/slice7-gateway"),
        live,
        MAXIMUM_BUDGET_MICROUSD,
    )
}

pub async fn provider_check() -> Result<(), DynError> {
    let gateway = gateway(true)?;
    let models = std::collections::BTreeSet::from([READER_MODEL, JUDGE_MODEL]);
    for model in models {
        let response = gateway
            .complete(
                model,
                "slice7-provider-check-v1",
                "",
                "Reply with the single word READY.",
                10,
            )
            .await?;
        if response.text.trim().is_empty() {
            return Err("provider check returned empty content".into());
        }
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "model":response.model,"response_model":response.response_model,
                "cached":response.cached,"usage":response.usage,
            }))?
        );
    }
    println!("{}", serde_json::to_string(&gateway.budget()?)?);
    Ok(())
}

pub async fn benchmark(name: &str, live: bool) -> Result<(), DynError> {
    let gateway = gateway(live)?;
    let root = repository_root();
    let (report, complete) = match name {
        "longmemeval" => {
            let result = longmemeval::run_full(&gateway).await?;
            let complete =
                result.total == 500 && result.evaluated == 500 && result.failure.is_none();
            (serde_json::to_value(result)?, complete)
        }
        "locomo" => {
            let dataset_path = root.join("eval/datasets/locomo/locomo10.json");
            let dataset =
                tokio::task::spawn_blocking(move || locomo::ensure_dataset(&dataset_path))
                    .await??;
            let result = locomo::run_live(
                &dataset,
                &gateway,
                &root.join("eval/datasets/locomo/run"),
                true,
            )
            .await?;
            let complete = result.complete;
            (serde_json::to_value(result)?, complete)
        }
        "beam" => {
            let (set, _) = beam_run::load_or_fixture()?;
            let result =
                beam_run::run(&gateway, &set, &root.join("eval/datasets/beam/run")).await?;
            let complete = result.complete;
            (serde_json::to_value(result)?, complete)
        }
        _ => return Err("benchmark must be longmemeval, locomo or beam".into()),
    };
    let output = root.join(format!("eval/results/slice7-{name}.json"));
    super::gateway::write_json(&output, &report)?;
    println!("{}", serde_json::to_string(&report)?);
    println!("{}", serde_json::to_string(&gateway.budget()?)?);
    if !complete {
        return Err("public benchmark incomplete; see the recorded failure and coverage".into());
    }
    Ok(())
}

pub async fn adapt_beam(artifact: Option<&str>) -> Result<(), DynError> {
    let source = artifact.map_or_else(
        || repository_root().join("eval/datasets/beam/conversations.json"),
        PathBuf::from,
    );
    let read = source.clone();
    let bytes = tokio::task::spawn_blocking(move || {
        std::fs::read(&read)
            .map_err(|error| format!("probe set artifact {}: {error}", read.display()))
    })
    .await??;
    let set = beam::adapt_published_artifact(&bytes)?;
    let output = beam_run::default_probe_set_path();
    super::gateway::write_json(&output, &set)?;
    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({
            "artifact": source.display().to_string(),
            "probe_set": output.display().to_string(),
            "probe_set_digest": beam::probe_set_digest(&set)?,
            "source_digest": set.source_digest,
            "conversations": set.conversations.len(),
            "probes": set.conversations.iter().map(|conversation| conversation.probes.len()).sum::<usize>(),
        }))?
    );
    Ok(())
}

pub async fn diagnostic_longmemeval(ids: &str, live: bool) -> Result<(), DynError> {
    let question_ids: Vec<String> = ids.split(',').map(str::to_owned).collect();
    let gateway = gateway(live)?;
    let result = longmemeval::run_selected(&gateway, &question_ids).await?;
    let complete = result.evaluated == result.total && result.failure.is_none();
    let report = serde_json::to_value(result)?;
    super::gateway::write_json(
        &repository_root().join("eval/results/slice7-longmemeval-diagnostic.json"),
        &report,
    )?;
    println!("{}", serde_json::to_string(&report)?);
    println!("{}", serde_json::to_string(&gateway.budget()?)?);
    if !complete {
        return Err(
            "diagnostic incomplete; failures recorded; this is not a qualification gate".into(),
        );
    }
    Ok(())
}

pub async fn diagnostic_judge(
    question_id: &str,
    prediction_path: &str,
    live: bool,
) -> Result<(), DynError> {
    let gateway = gateway(live)?;
    let report = judge_diagnostic::run(&gateway, question_id, Path::new(prediction_path)).await?;
    super::gateway::write_json(
        &repository_root().join("eval/results/slice7-longmemeval-judge-diagnostic.json"),
        &report,
    )?;
    println!("{}", serde_json::to_string(&report)?);
    println!("{}", serde_json::to_string(&gateway.budget()?)?);
    Ok(())
}
