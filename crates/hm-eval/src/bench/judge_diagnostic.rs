#![allow(clippy::missing_errors_doc)]

use super::gateway::{DynError, Gateway, JUDGE_MODEL, JUDGE_SETTINGS, READER_MODEL};
use super::longmemeval::{
    DATASET_SHA256, OFFICIAL_PROMPT_VERSION, load_full_dataset, official_judge_prompt,
};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

pub async fn run(
    gateway: &Gateway,
    question_id: &str,
    prediction_path: &Path,
) -> Result<Value, DynError> {
    let source_path = prediction_path.canonicalize()?;
    let prediction_bytes = fs::read(&source_path)?;
    let prediction: Value = serde_json::from_slice(&prediction_bytes)?;
    if prediction.get("question_id").and_then(Value::as_str) != Some(question_id)
        || prediction.get("dataset_hash").and_then(Value::as_str) != Some(DATASET_SHA256)
        || prediction.pointer("/reader/model").and_then(Value::as_str) != Some(READER_MODEL)
    {
        return Err(
            "judge diagnostic prediction identity, pinned dataset, or Luna reader mismatch".into(),
        );
    }
    let hypothesis = prediction
        .get("answer")
        .and_then(Value::as_str)
        .filter(|answer| !answer.trim().is_empty())
        .ok_or("judge diagnostic requires an existing nonempty immutable prediction")?;
    if prediction.pointer("/reader/text").and_then(Value::as_str) != Some(hypothesis) {
        return Err("judge diagnostic hypothesis differs from the recorded reader response".into());
    }
    let dataset_root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../eval/datasets/longmemeval");
    let dataset_path = std::env::var_os("LONGMEMEVAL_DATASET").map_or_else(
        || dataset_root.join("longmemeval_s_cleaned.json"),
        PathBuf::from,
    );
    let example = load_full_dataset(&dataset_path)?
        .into_iter()
        .find(|example| example.example.question_id == question_id)
        .ok_or("judge diagnostic question ID is absent from the pinned dataset")?;
    let prompt = official_judge_prompt(&example, hypothesis)?;
    let judge = gateway
        .complete_with_settings(
            JUDGE_MODEL,
            OFFICIAL_PROMPT_VERSION,
            "",
            &prompt,
            JUDGE_SETTINGS,
        )
        .await?;
    let current_correct = judge.text.trim().to_lowercase().contains("yes");
    Ok(json!({
        "diagnostic": true,
        "not_qualification": true,
        "question_id": question_id,
        "source_prediction": {
            "path": source_path.display().to_string(),
            "digest_algorithm": "blake3",
            "digest": blake3::hash(&prediction_bytes).to_hex().to_string(),
        },
        "dataset_sha256": DATASET_SHA256,
        "prompt_version": OFFICIAL_PROMPT_VERSION,
        "hypothesis": hypothesis,
        "prior_reader_model": READER_MODEL,
        "judge_settings": JUDGE_SETTINGS,
        "judge": judge,
        "current_correct": current_correct,
    }))
}
