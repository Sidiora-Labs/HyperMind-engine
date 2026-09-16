#![allow(clippy::cast_precision_loss, clippy::missing_errors_doc)]

use hm_core::{ActorId, ConversationId};
use hm_ledger::frame::EventKind;
use hm_mcp::{Envelope, McpServer, OutcomeInput};
use hm_proj::predictions::CalibrationCounters;
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    Authority, EventEnvelope, EventPayload, ProviderFrame, Retention, Sensitivity,
};
use hm_serve::actor::{ActorConfig, ActorEngine, IncomingEvent};
use serde::Serialize;
use serde_json::{Value, json};
use std::fs;

type EvaluationError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Serialize)]
pub struct CalibrationRow {
    pub predicate_kind: String,
    pub counts: CalibrationCounters,
    pub supported_among_resolved: f64,
}

#[derive(Debug, Serialize)]
pub struct CalibrationResult {
    pub cases: usize,
    pub assessment_mismatches: usize,
    pub duplicate_writes: usize,
    pub restart_identical: bool,
    pub revision_required: bool,
    pub per_kind: Vec<CalibrationRow>,
}

#[allow(clippy::too_many_lines)]
pub async fn run() -> Result<CalibrationResult, EvaluationError> {
    let temporary = tempfile::tempdir()?;
    let config = ActorConfig {
        actor_directory: temporary.path().join("7"),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 32 * 1024 * 1024,
    };
    let actor = ActorEngine::open(config.clone()).await?;
    let server = McpServer::new(actor.clone());
    let revision_file = temporary.path().join("revision");
    fs::write(&revision_file, "revision-2")?;
    let revision = fs::read_to_string(&revision_file)?;
    let digest = blake3::hash(revision.as_bytes()).to_hex().to_string();
    let receipt_file = temporary.path().join("receipt");
    fs::write(&receipt_file, &digest)?;
    let receipt = fs::read_to_string(&receipt_file)?;
    let process = std::process::Command::new("sh")
        .args(["-c", "exit 0"])
        .status()?;
    let answer_file = temporary.path().join("answer");
    fs::write(&answer_file, "The observed revision is revision-2.")?;
    fs::File::open(&answer_file)?.sync_all()?;
    let committed = fs::read_to_string(&answer_file)?;
    let values = [
        ("object_exists", json!(revision_file.exists())),
        ("revision_equals", json!(revision)),
        ("digest_equals", json!(digest)),
        ("receipt_matches", json!(receipt)),
        (
            "property_satisfies",
            json!(fs::metadata(&revision_file)?.len()),
        ),
        (
            "process_terminated",
            json!(process.code().ok_or("process has no exit code")?),
        ),
        ("answer_committed", json!(!committed.is_empty())),
    ];
    let mut result = CalibrationResult {
        cases: 0,
        assessment_mismatches: 0,
        duplicate_writes: 0,
        restart_identical: false,
        revision_required: false,
        per_kind: Vec::new(),
    };
    for (kind, value) in values {
        for assessment in [
            "supported",
            "contradicted",
            "pending",
            "unresolvable",
            "not_executed",
        ] {
            let id = format!("{kind}-{assessment}");
            let expected = if assessment == "contradicted" {
                different(&value)
            } else {
                value.clone()
            };
            let mut predicates = vec![
                json!({"kind":kind,"scope":"artifact","property":"value","expected":expected}),
            ];
            if assessment == "pending" {
                predicates.push(json!({"kind":kind,"scope":"future-artifact","property":"value","expected":value}));
            }
            require_ok(server.predict_envelope(serde_json::from_value(json!({
                "conversation":"calibration", "prediction_id":id,"revision":1,
                "mechanism":format!("calibration_{kind}"),
                "predicates":predicates,
                "deadline_ns":i64::MAX,"uncertainty":"Observed operations may differ from the declared predicate."
            }))?).await)?;
            let (executed, resolvable, observed) = match assessment {
                "not_executed" => (false, false, Value::Null),
                "unresolvable" => {
                    let missing = fs::read(temporary.path().join("missing-artifact"));
                    if missing.is_ok() {
                        return Err("missing artifact unexpectedly resolved".into());
                    }
                    (true, false, Value::Null)
                }
                _ => (true, true, value.clone()),
            };
            let lsn = observe(
                &actor,
                json!({"kind":kind,
                "scope":"artifact",
                    "property":"value","value":observed,"executed":executed,"resolvable":resolvable
                }),
            )
            .await?;
            let input = OutcomeInput {
                conversation: "calibration".to_owned(),
                prediction_id: id,
                revision: 1,
                observation_lsns: vec![lsn],
            };
            let outcome = require_ok(server.outcome_envelope(input.clone()).await)?;
            result.cases += 1;
            result.assessment_mismatches +=
                usize::from(outcome.items[0]["assessment"] != assessment);
            let before = actor.stats().await?.log_events;
            require_ok(server.outcome_envelope(input).await)?;
            result.duplicate_writes += usize::from(actor.stats().await?.log_events != before);
        }
    }
    for index in 0..2 {
        let id = format!("revision-failure-{index}");
        require_ok(server.predict_envelope(serde_json::from_value(json!({
            "conversation":"calibration","prediction_id":id,"revision":1,
            "mechanism":"calibration_revision_equals",
            "predicates":[{"kind":"revision_equals","scope":"artifact","property":"value","expected":"revision-3"}],
            "deadline_ns":i64::MAX,"uncertainty":"The write may not have reached the expected revision."
        }))?).await)?;
        let lsn = observe(&actor, json!({"kind":"revision_equals","scope":"artifact",
            "property":"value","value":fs::read_to_string(&revision_file)?,"executed":true,"resolvable":true})).await?;
        let outcome = require_ok(
            server
                .outcome_envelope(OutcomeInput {
                    conversation: "calibration".to_owned(),
                    prediction_id: id,
                    revision: 1,
                    observation_lsns: vec![lsn],
                })
                .await,
        )?;
        result.cases += 1;
        result.assessment_mismatches +=
            usize::from(outcome.items[0]["assessment"] != "contradicted");
    }
    result.revision_required = actor
        .mechanism_failures("calibration_revision_equals".to_owned())
        .await?
        .revision_required;
    let before = actor.calibration().await?;
    drop(server);
    actor.shutdown().await?;
    let reopened = ActorEngine::open(config).await?;
    let after = reopened.calibration().await?;
    result.restart_identical = before == after;
    for (kind, counts) in after {
        let resolved = counts.supported + counts.contradicted;
        result.per_kind.push(CalibrationRow {
            predicate_kind: format!("{kind:?}"),
            counts,
            supported_among_resolved: if resolved == 0 {
                0.0
            } else {
                counts.supported as f64 / resolved as f64
            },
        });
    }
    reopened.shutdown().await?;
    Ok(result)
}

fn different(value: &Value) -> Value {
    match value {
        Value::Bool(value) => json!(!value),
        Value::Number(value) => json!(value.as_i64().unwrap_or_default() + 1),
        _ => json!("deliberately different predicted value"),
    }
}

fn require_ok(envelope: Envelope) -> Result<Envelope, EvaluationError> {
    if envelope.ok {
        Ok(envelope)
    } else {
        Err(format!("calibration operation failed: {envelope:?}").into())
    }
}

async fn observe(actor: &ActorEngine, observation: Value) -> Result<u64, EvaluationError> {
    Ok(actor
        .append(vec![IncomingEvent {
            kind: EventKind::ProviderFrame,
            conversation: ConversationId::derive("calibration"),
            payload: encode_event_envelope(&EventEnvelope {
                schema_version: CURRENT_SCHEMA_VERSION,
                payload: EventPayload::ProviderFrame(Box::new(ProviderFrame {
                    provider: "local-filesystem-and-process".to_owned(),
                    api_content: serde_json::to_vec(
                        &json!({"schema_version":1,"observations":[observation]}),
                    )?,
                })),
                connection_id: None,
                client_seq: 0,
                client_event_index: 0,
                client_event_count: 1,
                origin_actor: 0,
                run_id: None,
                model_provenance: None,
                authority: Authority::ExternalObserved,
                retention: Retention::CurrentState,
                sensitivity: Sensitivity::Personal,
                event_time_ns: 0,
            }),
        }])
        .await?
        .first_lsn
        .get())
}
