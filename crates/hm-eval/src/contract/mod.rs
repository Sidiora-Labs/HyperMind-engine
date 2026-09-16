#![allow(clippy::cast_precision_loss, clippy::missing_errors_doc)]

use crate::bench::gateway::DynError;
use hm_core::ActorId;
use hm_mcp::dispatcher::McpToolDispatcher;
use hm_serve::actor::{ActorConfig, ActorEngine};
use hm_serve::uds::ToolDispatcher as _;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub const CONTRACT_FORMAT: &str = "hypermind.cross-sdk-contract.v1";
pub const RAW_FORMAT: &str = "hypermind.cross-sdk-contract-raw.v1";
pub const CONTRACT_ACTOR: u16 = 7;
pub const REFERENCE_SDK: &str = "rust-embedded";
pub const REFERENCE_TRANSPORT: &str = "in-process";

const IDENTIFIER_KEYS: [&str; 6] = [
    "first_lsn",
    "last_lsn",
    "lsn",
    "belief_id",
    "run_id",
    "receipt",
];
const RETAINED_QUERY_KEYS: [&str; 2] = ["src", "score"];
const COMPARED_FIELDS: [&str; 10] = [
    "verb",
    "ok",
    "error_code",
    "effect_state",
    "item_identifiers",
    "provenance",
    "authority",
    "health",
    "gap_kinds",
    "warning_kinds",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    BeforeRestart,
    AfterRestart,
}

#[derive(Clone, Debug, Serialize)]
pub struct ScenarioCall {
    pub step: &'static str,
    pub verb: &'static str,
    pub arguments: Value,
    pub phase: Phase,
}

#[must_use]
pub fn scenario(conversation: &str) -> Vec<ScenarioCall> {
    vec![
        ScenarioCall {
            step: "remember-user",
            verb: "remember",
            arguments: json!({
                "conversation": conversation,
                "content": "The harbour telemetry beacon drifted nine metres off pier seventeen.",
                "kind": "user",
            }),
            phase: Phase::BeforeRestart,
        },
        ScenarioCall {
            step: "remember-assistant",
            verb: "remember",
            arguments: json!({
                "conversation": conversation,
                "content": "Logged the harbour telemetry beacon drift and scheduled a pier survey.",
                "kind": "assistant",
            }),
            phase: Phase::BeforeRestart,
        },
        ScenarioCall {
            step: "recall-lexical",
            verb: "recall",
            arguments: json!({
                "mode": "lexical",
                "query": "harbour telemetry beacon drift",
                "conversation": conversation,
                "limit": 8,
            }),
            phase: Phase::BeforeRestart,
        },
        ScenarioCall {
            step: "recall-rejected-argument",
            verb: "recall",
            arguments: json!({
                "mode": "lexical",
                "query": "harbour telemetry beacon drift",
                "conversation": conversation,
                "limit": 0,
            }),
            phase: Phase::BeforeRestart,
        },
        ScenarioCall {
            step: "remember-rejected-mutation",
            verb: "remember",
            arguments: json!({
                "conversation": conversation,
                "content": "",
                "kind": "user",
            }),
            phase: Phase::BeforeRestart,
        },
        ScenarioCall {
            step: "inspect-first-event",
            verb: "inspect",
            arguments: json!({ "uri": format!("hm://{CONTRACT_ACTOR}/lsn/1") }),
            phase: Phase::BeforeRestart,
        },
        ScenarioCall {
            step: "recall-after-restart",
            verb: "recall",
            arguments: json!({
                "mode": "lexical",
                "query": "harbour telemetry beacon drift",
                "conversation": conversation,
                "limit": 8,
            }),
            phase: Phase::AfterRestart,
        },
        ScenarioCall {
            step: "forget-fade",
            verb: "forget",
            arguments: json!({ "action": "fade", "lsn": 1 }),
            phase: Phase::AfterRestart,
        },
    ]
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct StepObservation {
    pub step: String,
    pub verb: String,
    pub ok: bool,
    pub error_code: Option<String>,
    pub effect_state: Option<String>,
    pub item_identifiers: Vec<String>,
    pub provenance: Vec<String>,
    pub authority: Vec<String>,
    pub health: BTreeMap<String, String>,
    pub gap_kinds: Vec<String>,
    pub warning_kinds: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContractObservation {
    pub format: String,
    pub sdk: String,
    pub transport: String,
    pub steps: Vec<StepObservation>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RawCall {
    pub step: String,
    pub verb: String,
    pub envelope: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RawDocument {
    pub format: String,
    pub sdk: String,
    pub transport: String,
    pub calls: Vec<RawCall>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Unavailable {
    pub sdk: String,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Unsupported {
    pub sdk: String,
    pub step: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Divergence {
    pub step: String,
    pub field: String,
    pub values: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContractComparison {
    pub format: String,
    pub participants: Vec<String>,
    pub unavailable: Vec<Unavailable>,
    pub unsupported: Vec<Unsupported>,
    pub steps_compared: usize,
    pub divergences: Vec<Divergence>,
    pub agreed: bool,
}

#[must_use]
pub fn normalize_provenance(uri: &str) -> String {
    let (path, query) = uri.split_once('?').unwrap_or((uri, ""));
    let mut retained = Vec::new();
    for key in RETAINED_QUERY_KEYS {
        for field in query.split('&') {
            if let Some(value) = field
                .strip_prefix(key)
                .and_then(|rest| rest.strip_prefix('='))
            {
                retained.push(format!("{key}={value}"));
            }
        }
    }
    if retained.is_empty() {
        path.to_owned()
    } else {
        format!("{path}?{}", retained.join("&"))
    }
}

#[must_use]
pub fn observe(step: &str, verb: &str, envelope: &Value) -> StepObservation {
    let items = array_of(envelope, "items");
    let mut item_identifiers = Vec::new();
    let mut authority = Vec::new();
    let mut error_code = None;
    for item in &items {
        for key in IDENTIFIER_KEYS {
            if let Some(value) = item.get(key).filter(|value| !value.is_null()) {
                item_identifiers.push(format!("{key}={}", scalar_text(value)));
            }
        }
        if let Some(value) = item.get("authority").and_then(Value::as_str) {
            authority.push(value.to_owned());
        }
        if error_code.is_none() {
            error_code = item.get("error").and_then(Value::as_str).map(str::to_owned);
        }
    }
    let provenance = array_of(envelope, "provenance")
        .iter()
        .filter_map(Value::as_str)
        .map(normalize_provenance)
        .collect();
    let gap_kinds = array_of(envelope, "gaps")
        .iter()
        .filter_map(|gap| gap.get("kind").and_then(Value::as_str))
        .map(str::to_owned)
        .collect();
    let warning_kinds = array_of(envelope, "warnings")
        .iter()
        .filter_map(Value::as_str)
        .map(warning_prefix)
        .collect();
    let health = envelope
        .get("health")
        .and_then(Value::as_object)
        .map(|health| {
            health
                .iter()
                .map(|(key, value)| (key.clone(), scalar_text(value)))
                .collect()
        })
        .unwrap_or_default();
    StepObservation {
        step: step.to_owned(),
        verb: verb.to_owned(),
        ok: envelope
            .get("ok")
            .and_then(Value::as_bool)
            .unwrap_or_default(),
        error_code,
        effect_state: envelope
            .get("effect_state")
            .and_then(Value::as_str)
            .map(str::to_owned),
        item_identifiers,
        provenance,
        authority,
        health,
        gap_kinds,
        warning_kinds,
    }
}

pub fn observe_raw(bytes: &[u8]) -> Result<ContractObservation, DynError> {
    let document: RawDocument = serde_json::from_slice(bytes)?;
    if document.format != RAW_FORMAT {
        return Err(format!(
            "raw leg format is {}, expected {RAW_FORMAT}",
            document.format
        )
        .into());
    }
    if document.sdk.is_empty() || document.transport.is_empty() {
        return Err("raw leg must name its sdk and transport".into());
    }
    let mut seen = BTreeSet::new();
    let mut steps = Vec::with_capacity(document.calls.len());
    for call in &document.calls {
        if !seen.insert(call.step.clone()) {
            return Err(format!("raw leg repeats step {}", call.step).into());
        }
        steps.push(observe(&call.step, &call.verb, &call.envelope));
    }
    Ok(ContractObservation {
        format: CONTRACT_FORMAT.to_owned(),
        sdk: document.sdk,
        transport: document.transport,
        steps,
    })
}

#[must_use]
pub fn compare(
    observations: &[ContractObservation],
    unavailable: Vec<Unavailable>,
    unsupported: Vec<Unsupported>,
) -> ContractComparison {
    let participants = observations
        .iter()
        .map(|observation| observation.sdk.clone())
        .collect();
    let skipped = unsupported
        .iter()
        .map(|entry| (entry.sdk.clone(), entry.step.clone()))
        .collect::<BTreeSet<_>>();
    let mut order = Vec::new();
    let mut seen = BTreeSet::new();
    for observation in observations {
        for step in &observation.steps {
            if seen.insert(step.step.clone()) {
                order.push(step.step.clone());
            }
        }
    }
    let mut divergences = Vec::new();
    let mut steps_compared = 0;
    for step_name in &order {
        let mut present = Vec::new();
        let mut absent = Vec::new();
        for observation in observations {
            if skipped.contains(&(observation.sdk.clone(), step_name.clone())) {
                continue;
            }
            match observation
                .steps
                .iter()
                .find(|step| &step.step == step_name)
            {
                Some(step) => present.push((
                    observation.sdk.clone(),
                    serde_json::to_value(step).unwrap_or_default(),
                )),
                None => absent.push(observation.sdk.clone()),
            }
        }
        if present.is_empty() {
            continue;
        }
        steps_compared += 1;
        if !absent.is_empty() {
            let mut values = BTreeMap::new();
            for (sdk, _) in &present {
                values.insert(sdk.clone(), "present".to_owned());
            }
            for sdk in absent {
                values.insert(sdk, "absent".to_owned());
            }
            divergences.push(Divergence {
                step: step_name.clone(),
                field: "presence".to_owned(),
                values,
            });
        }
        for field in COMPARED_FIELDS {
            let mut values = BTreeMap::new();
            for (sdk, step) in &present {
                values.insert(
                    sdk.clone(),
                    step.get(field)
                        .map_or_else(|| Value::Null.to_string(), std::string::ToString::to_string),
                );
            }
            if values.values().collect::<BTreeSet<_>>().len() > 1 {
                divergences.push(Divergence {
                    step: step_name.clone(),
                    field: field.to_owned(),
                    values,
                });
            }
        }
    }
    let agreed = divergences.is_empty();
    ContractComparison {
        format: CONTRACT_FORMAT.to_owned(),
        participants,
        unavailable,
        unsupported,
        steps_compared,
        divergences,
        agreed,
    }
}

pub async fn reference_raw(directory: &Path, conversation: &str) -> Result<RawDocument, DynError> {
    let calls = scenario(conversation);
    let actor_directory = directory.join("actor");
    let dispatcher = McpToolDispatcher::default();
    let mut recorded = Vec::with_capacity(calls.len());
    for phase in [Phase::BeforeRestart, Phase::AfterRestart] {
        let engine = open_engine(&actor_directory).await?;
        for call in calls.iter().filter(|call| call.phase == phase) {
            let arguments = serde_json::to_vec(&call.arguments)?;
            let bytes = match dispatcher
                .dispatch(engine.clone(), call.verb.to_owned(), arguments)
                .await
            {
                Ok(bytes) => bytes,
                Err(error) => {
                    engine.shutdown().await?;
                    return Err(format!(
                        "{}: {} was not dispatched: {error}",
                        call.step, call.verb
                    )
                    .into());
                }
            };
            recorded.push(RawCall {
                step: call.step.to_owned(),
                verb: call.verb.to_owned(),
                envelope: serde_json::from_slice(&bytes)?,
            });
        }
        engine.shutdown().await?;
    }
    Ok(RawDocument {
        format: RAW_FORMAT.to_owned(),
        sdk: REFERENCE_SDK.to_owned(),
        transport: REFERENCE_TRANSPORT.to_owned(),
        calls: recorded,
    })
}

pub async fn reference_leg(
    directory: &Path,
    conversation: &str,
) -> Result<ContractObservation, DynError> {
    let document = reference_raw(directory, conversation).await?;
    observe_raw(&serde_json::to_vec(&document)?)
}

async fn open_engine(actor_directory: &Path) -> Result<ActorEngine, DynError> {
    ActorEngine::open(ActorConfig {
        actor_directory: actor_directory.to_path_buf(),
        actor: ActorId::new(CONTRACT_ACTOR),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    })
    .await
    .map_err(Into::into)
}

fn array_of(envelope: &Value, key: &str) -> Vec<Value> {
    envelope
        .get(key)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn scalar_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

fn warning_prefix(warning: &str) -> String {
    let segments = warning.split(':').collect::<Vec<_>>();
    let kept = segments
        .iter()
        .rev()
        .skip_while(|segment| segment.contains('='))
        .count();
    if kept == 0 {
        warning.to_owned()
    } else {
        segments[..kept].join(":")
    }
}
