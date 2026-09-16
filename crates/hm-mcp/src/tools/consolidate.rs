#![allow(clippy::missing_errors_doc)]

use crate::Envelope;
use hm_core::{ConversationId, Error, ErrorCode, LSN};
use hm_cortex::budget::BudgetUsage;
use hm_cortex::run::{PhaseMachine, retraction_event, run_id};
use hm_ledger::frame::EventKind;
use hm_schema::event::{self, Boundary, CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    Authority, ConsolidationBudget, ConsolidationClosed, ConsolidationOpened,
    ConsolidationPhaseName, ConsolidationPhaseState, EventEnvelope, EventPayload, PromptVersion,
    Retention, Sensitivity,
};
use hm_serve::actor::{ActorEngine, IncomingEvent};
use rmcp::schemars;
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConsolidateAction {
    Run,
    List,
    Retract,
}

#[derive(Clone, Copy, Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConsolidateMode {
    Nrem,
    Rem,
    Both,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, schemars::JsonSchema)]
pub struct ConsolidateBudget {
    pub max_llm_calls: u64,
    pub max_tokens: u64,
    pub max_microusd: u64,
    pub max_wall_ms: u64,
}

impl From<ConsolidateBudget> for ConsolidationBudget {
    fn from(value: ConsolidateBudget) -> Self {
        Self {
            max_llm_calls: value.max_llm_calls,
            max_tokens: value.max_tokens,
            max_microusd: value.max_microusd,
            max_wall_ms: value.max_wall_ms,
        }
    }
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct ConsolidateInput {
    pub action: ConsolidateAction,
    #[serde(default)]
    pub mode: Option<ConsolidateMode>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub cadence_key: Option<String>,
    #[serde(default)]
    pub budget: Option<ConsolidateBudget>,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
}

pub async fn run(actor: &ActorEngine, input: ConsolidateInput) -> Result<Envelope, Error> {
    let history = read_history(actor).await?;
    match input.action {
        ConsolidateAction::List => list(history),
        ConsolidateAction::Run => start(actor, history, input).await,
        ConsolidateAction::Retract => retract(actor, history, input).await,
    }
}

async fn start(
    actor: &ActorEngine,
    history: RunHistory,
    input: ConsolidateInput,
) -> Result<Envelope, Error> {
    let mode = input
        .mode
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
    let scope = input.scope.unwrap_or_else(|| "actor".to_owned());
    let cadence_key = input
        .cadence_key
        .filter(|value| !value.is_empty())
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
    let budget = input
        .budget
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
    if budget.max_llm_calls == 0
        || budget.max_tokens == 0
        || budget.max_microusd == 0
        || budget.max_wall_ms == 0
    {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let scope_digest = *blake3::hash(scope.as_bytes()).as_bytes();
    let requested_phases = phases(mode);
    if let Some((id, existing)) = history
        .runs
        .iter()
        .find(|(_, run)| run.scope_digest == scope_digest && run.cadence_key == cadence_key)
    {
        if existing.phases != requested_phases || existing.budget != budget {
            return Err(Error::new(ErrorCode::IdempotencyConflict));
        }
        return Ok(run_envelope(actor, id, existing, true));
    }
    let generation = history.maximum_generation.saturating_add(1).max(1);
    let parent = history.active_generation;
    let id = run_id(scope.as_bytes(), &cadence_key, generation);
    let phases = requested_phases;
    let prompts = prompts(mode);
    let opened = ConsolidationOpened {
        scope_digest: scope_digest.to_vec(),
        cadence_key,
        generation,
        expected_active_generation: parent,
        phases: phases.clone(),
        prompts,
        budget: Box::new(budget.into()),
    };
    let mut events = vec![incoming(
        actor,
        &id,
        EventKind::ConsolidationOpened,
        EventPayload::ConsolidationOpened(Box::new(opened.clone())),
    )];
    let mut machine = PhaseMachine::resume(id.to_vec(), phases, []);
    while let Some(work) = machine.next() {
        let started = machine.event(
            &work,
            ConsolidationPhaseState::Started,
            None,
            BudgetUsage::default(),
            0,
        );
        events.push(incoming(
            actor,
            &id,
            EventKind::ConsolidationPhase,
            EventPayload::ConsolidationPhase(Box::new(started.clone())),
        ));
        machine.record(&work, &started);
        let completed = machine.event(
            &work,
            ConsolidationPhaseState::Completed,
            None,
            BudgetUsage::default(),
            0,
        );
        events.push(incoming(
            actor,
            &id,
            EventKind::ConsolidationPhase,
            EventPayload::ConsolidationPhase(Box::new(completed.clone())),
        ));
        machine.record(&work, &completed);
    }
    events.push(incoming(
        actor,
        &id,
        EventKind::ConsolidationClosed,
        EventPayload::ConsolidationClosed(Box::new(ConsolidationClosed {
            generation,
            expected_active_generation: parent,
            derived_records: 0,
            dropped_candidates: 0,
            llm_calls: 0,
            input_tokens: 0,
            output_tokens: 0,
            cost_microusd: 0,
        })),
    ));
    let outcome = actor.append(events).await?;
    let summary = RunSummary {
        generation,
        parent_generation: parent,
        status: RunStatus::Published,
        scope_digest,
        cadence_key: opened.cadence_key,
        phases: opened.phases,
        budget,
        first_lsn: outcome.first_lsn.get(),
        last_lsn: outcome.last_lsn.get(),
        derived_records: 0,
        dropped_candidates: 0,
        llm_calls: 0,
        input_tokens: 0,
        output_tokens: 0,
        cost_microusd: 0,
    };
    let mut envelope = run_envelope(actor, &id, &summary, false);
    envelope.provenance.push(format!(
        "hm://{}/lsn/{}",
        actor.actor(),
        outcome.last_lsn.get()
    ));
    Ok(envelope)
}

async fn retract(
    actor: &ActorEngine,
    history: RunHistory,
    input: ConsolidateInput,
) -> Result<Envelope, Error> {
    let target = input
        .run_id
        .as_deref()
        .and_then(decode_hex)
        .and_then(|id| history.runs.get(&id).cloned().map(|run| (id, run)))
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
    if target.1.status != RunStatus::Published || history.active_generation != target.1.generation {
        return Err(Error::new(ErrorCode::IdempotencyConflict));
    }
    let reason = input
        .reason
        .filter(|value| !value.is_empty())
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
    let event = retraction_event(target.0.clone(), target.1.parent_generation, reason);
    let outcome = actor
        .append(vec![incoming(
            actor,
            &target.0,
            EventKind::ConsolidationRetracted,
            EventPayload::ConsolidationRetracted(Box::new(event)),
        )])
        .await?;
    let mut envelope = Envelope::empty();
    envelope.items.push(json!({
        "run_id": hex(&target.0),
        "status": "retracted",
        "active_generation": target.1.parent_generation,
        "lsn": outcome.first_lsn.get(),
    }));
    Ok(envelope)
}

fn list(history: RunHistory) -> Result<Envelope, Error> {
    let mut envelope = Envelope::empty();
    envelope.items = history
        .runs
        .into_iter()
        .map(|(id, run)| {
            json!({
                "run_id": hex(&id),
                "generation": run.generation,
                "parent_generation": run.parent_generation,
                "status": run.status.name(),
                "cost": {"llm_calls": run.llm_calls, "input_tokens": run.input_tokens, "output_tokens": run.output_tokens, "microusd": run.cost_microusd},
                "stats": {"derived_records": run.derived_records, "dropped_candidates": run.dropped_candidates},
            })
        })
        .collect();
    envelope.health =
        json!({"projection": "ready", "active_generation": history.active_generation});
    Ok(envelope)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RunStatus {
    Open,
    Published,
    Retracted,
}

impl RunStatus {
    const fn name(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Published => "published",
            Self::Retracted => "retracted",
        }
    }
}

#[derive(Clone, Debug)]
struct RunSummary {
    generation: u64,
    parent_generation: u64,
    status: RunStatus,
    scope_digest: [u8; 32],
    cadence_key: String,
    phases: Vec<ConsolidationPhaseName>,
    budget: ConsolidateBudget,
    first_lsn: u64,
    last_lsn: u64,
    derived_records: u64,
    dropped_candidates: u64,
    llm_calls: u64,
    input_tokens: u64,
    output_tokens: u64,
    cost_microusd: u64,
}

#[derive(Default)]
struct RunHistory {
    runs: BTreeMap<Vec<u8>, RunSummary>,
    maximum_generation: u64,
    active_generation: u64,
}

async fn read_history(actor: &ActorEngine) -> Result<RunHistory, Error> {
    let mut history = RunHistory::default();
    for frame in actor.frames_since(LSN::new(0), None, usize::MAX).await? {
        if !matches!(
            frame.header.kind,
            EventKind::ConsolidationOpened
                | EventKind::ConsolidationClosed
                | EventKind::ConsolidationRetracted
        ) {
            continue;
        }
        let kind = event::EventKind::try_from(frame.header.kind as u8)
            .map_err(|()| Error::new(ErrorCode::InvalidKind))?;
        let verified = event::verify_event(&frame.sealed_payload, kind, Boundary::Disk)?;
        let envelope_run_id = verified
            .envelope
            .run_id
            .clone()
            .ok_or_else(|| Error::new(ErrorCode::SchemaInvalid))?;
        match verified.envelope.payload {
            EventPayload::ConsolidationOpened(opened) => {
                let scope_digest = opened
                    .scope_digest
                    .as_slice()
                    .try_into()
                    .map_err(|_| Error::new(ErrorCode::SchemaInvalid))?;
                history.maximum_generation = history.maximum_generation.max(opened.generation);
                history.runs.insert(
                    envelope_run_id,
                    RunSummary {
                        generation: opened.generation,
                        parent_generation: opened.expected_active_generation,
                        status: RunStatus::Open,
                        scope_digest,
                        cadence_key: opened.cadence_key,
                        phases: opened.phases,
                        budget: ConsolidateBudget {
                            max_llm_calls: opened.budget.max_llm_calls,
                            max_tokens: opened.budget.max_tokens,
                            max_microusd: opened.budget.max_microusd,
                            max_wall_ms: opened.budget.max_wall_ms,
                        },
                        first_lsn: frame.header.lsn.get(),
                        last_lsn: frame.header.lsn.get(),
                        derived_records: 0,
                        dropped_candidates: 0,
                        llm_calls: 0,
                        input_tokens: 0,
                        output_tokens: 0,
                        cost_microusd: 0,
                    },
                );
            }
            EventPayload::ConsolidationClosed(closed) => {
                let run = history
                    .runs
                    .get_mut(&envelope_run_id)
                    .ok_or_else(|| Error::new(ErrorCode::OrderingViolation))?;
                run.status = RunStatus::Published;
                run.derived_records = closed.derived_records;
                run.dropped_candidates = closed.dropped_candidates;
                run.llm_calls = closed.llm_calls;
                run.input_tokens = closed.input_tokens;
                run.output_tokens = closed.output_tokens;
                run.cost_microusd = closed.cost_microusd;
                run.last_lsn = frame.header.lsn.get();
                history.active_generation = closed.generation;
            }
            EventPayload::ConsolidationRetracted(retracted) => {
                let run = history
                    .runs
                    .get_mut(&retracted.target_run_id)
                    .ok_or_else(|| Error::new(ErrorCode::OrderingViolation))?;
                run.status = RunStatus::Retracted;
                run.last_lsn = frame.header.lsn.get();
                history.active_generation = retracted.previous_generation;
            }
            _ => return Err(Error::new(ErrorCode::InvalidKind)),
        }
    }
    Ok(history)
}

fn run_envelope(actor: &ActorEngine, id: &[u8], run: &RunSummary, duplicate: bool) -> Envelope {
    let mut envelope = Envelope::empty();
    envelope.items.push(json!({
        "run_id": hex(id),
        "generation": run.generation,
        "parent_generation": run.parent_generation,
        "status": run.status.name(),
        "first_lsn": run.first_lsn,
        "last_lsn": run.last_lsn,
        "duplicate": duplicate,
        "cost": {"llm_calls": run.llm_calls, "input_tokens": run.input_tokens, "output_tokens": run.output_tokens, "microusd": run.cost_microusd},
        "stats": {"derived_records": run.derived_records, "dropped_candidates": run.dropped_candidates},
    }));
    envelope
        .provenance
        .push(format!("hm://{}/lsn/{}", actor.actor(), run.last_lsn));
    envelope
}

fn phases(mode: ConsolidateMode) -> Vec<ConsolidationPhaseName> {
    match mode {
        ConsolidateMode::Nrem => vec![ConsolidationPhaseName::Nrem],
        ConsolidateMode::Rem => vec![
            ConsolidationPhaseName::Connect,
            ConsolidationPhaseName::Abstract,
            ConsolidationPhaseName::Hindsight,
            ConsolidationPhaseName::Review,
        ],
        ConsolidateMode::Both => vec![
            ConsolidationPhaseName::Nrem,
            ConsolidationPhaseName::Connect,
            ConsolidationPhaseName::Abstract,
            ConsolidationPhaseName::Hindsight,
            ConsolidationPhaseName::Review,
        ],
    }
}

fn prompts(mode: ConsolidateMode) -> Vec<PromptVersion> {
    let mut prompts = Vec::new();
    if matches!(mode, ConsolidateMode::Nrem | ConsolidateMode::Both) {
        prompts.push(prompt("merge-cluster", 1));
    }
    if matches!(mode, ConsolidateMode::Rem | ConsolidateMode::Both) {
        prompts.extend([
            prompt("connect-long-context", 1),
            prompt("abstract-synthesis", 2),
            prompt("hindsight-review", 3),
        ]);
    }
    prompts
}

fn prompt(id: &str, version: u16) -> PromptVersion {
    PromptVersion {
        prompt_id: id.to_owned(),
        version,
        model_id: "configured-provider".to_owned(),
    }
}

fn incoming(
    actor: &ActorEngine,
    run_id: &[u8],
    kind: EventKind,
    payload: EventPayload,
) -> IncomingEvent {
    IncomingEvent {
        kind,
        conversation: ConversationId::derive("hypermind.consolidation"),
        payload: encode_event_envelope(&EventEnvelope {
            schema_version: CURRENT_SCHEMA_VERSION,
            payload,
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 0,
            origin_actor: actor.actor().get(),
            run_id: Some(run_id.to_vec()),
            model_provenance: None,
            authority: Authority::RuntimeFact,
            retention: Retention::Durable,
            sensitivity: Sensitivity::Personal,
            event_time_ns: 0,
        }),
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn decode_hex(value: &str) -> Option<Vec<u8>> {
    if value.is_empty() || !value.len().is_multiple_of(2) {
        return None;
    }
    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).ok())
        .collect()
}
