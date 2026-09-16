#![allow(clippy::missing_errors_doc)]

use crate::Envelope;
use crate::admission::admission_from_env;
use crate::tools::relation;
use crate::tools::remember::EmbeddingRuntime;
use hm_core::telemetry::{Attribute, SpanBuilder, SpanKind, SpanOutcome};
use hm_core::{ConversationId, Error, ErrorCode, LSN};
use hm_cortex::budget::BudgetUsage;
use hm_cortex::citations::{FrozenCandidate, SourceKind};
use hm_cortex::nrem::cluster::{
    ClusterOptions, ObservationCluster, PendingObservation, cluster_observations,
};
use hm_cortex::nrem::merge::{MergeAction, NremReport, consolidate_clusters};
use hm_cortex::run::{PhaseMachine, retraction_event, run_id};
use hm_ledger::frame::EventKind;
use hm_llm::LlmProvider;
use hm_llm::admission::{AdmissionLimits, AdmittedProvider, CallAdmission};
use hm_schema::event::{self, Boundary, CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    Authority, ConsolidationBudget, ConsolidationClosed, ConsolidationOpened,
    ConsolidationPhaseName, ConsolidationPhaseState, EventEnvelope, EventPayload, MemoryMinted,
    PromptVersion, Retention, Sensitivity,
};
use hm_serve::actor::{ActorEngine, IncomingEvent};
use rmcp::schemars;
use serde::Deserialize;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

const MAXIMUM_RELATIONS: usize = 256;

#[derive(Clone)]
pub struct ConsolidationRuntime {
    provider: Arc<dyn LlmProvider>,
    pub admission: Arc<CallAdmission>,
}

impl ConsolidationRuntime {
    pub fn from_env() -> Result<Option<Self>, Error> {
        match std::env::var("HM_CONSOLIDATION_PROVIDER").as_deref() {
            Err(_) | Ok("") => return Ok(None),
            Ok("centra") => {}
            _ => return Err(Error::new(ErrorCode::InvalidArgument)),
        }
        let api_key = std::env::var("CENTRA_GATEWAY_API_KEY")
            .ok()
            .filter(|key| !key.is_empty())
            .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
        let base = std::env::var("CENTRA_GATEWAY_URL")
            .unwrap_or_else(|_| "https://gateway.centra.ag/v1".into());
        let provider = hm_llm::openai_compat::OpenAiCompatible::new(
            hm_llm::ProviderConfig {
                endpoint: format!("{}/chat/completions", base.trim_end_matches('/')),
                api_key: Some(api_key),
                model: "openrouter/openai/gpt-5.6-luna".into(),
                tier: hm_llm::ModelTier::Economy,
                pricing: hm_llm::Pricing {
                    input_microusd_per_million_tokens: 200_000,
                    output_microusd_per_million_tokens: 1_200_000,
                },
            },
            hm_llm::HttpTransport::default(),
        )
        .map_err(|_| Error::new(ErrorCode::InvalidArgument))?;
        Ok(Some(Self {
            provider: crate::telemetry::observed(Arc::new(provider)),
            admission: admission_from_env(),
        }))
    }

    #[must_use]
    pub fn new(provider: Arc<dyn LlmProvider>) -> Self {
        Self {
            provider: crate::telemetry::observed(provider),
            admission: Arc::new(CallAdmission::new(AdmissionLimits::default())),
        }
    }

    #[must_use]
    pub fn provider_for(&self, actor: u16) -> Arc<dyn LlmProvider> {
        Arc::new(AdmittedProvider::new(
            Arc::clone(&self.admission),
            actor,
            Arc::clone(&self.provider),
        ))
    }
}

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

pub async fn run(
    actor: &ActorEngine,
    runtime: Option<&ConsolidationRuntime>,
    embedding: Option<&EmbeddingRuntime>,
    input: ConsolidateInput,
) -> Result<Envelope, Error> {
    let history = read_history(actor).await?;
    match input.action {
        ConsolidateAction::List => Ok(list(history)),
        ConsolidateAction::Run => {
            let mode = input.mode;
            let mut span =
                hm_core::telemetry::start_span(SpanKind::Extraction, "hypermind.extraction");
            if let Some(span) = span.as_mut() {
                span.attribute(Attribute::Integer(
                    "hypermind.actor",
                    i64::from(actor.actor().get()),
                ));
                if let Some(mode) = mode {
                    span.attribute(Attribute::Text(
                        "hypermind.extraction.mode",
                        mode_label(mode),
                    ));
                }
            }
            let started = start(actor, runtime, embedding, history, input).await;
            if let Some(mut span) = span {
                match started.as_ref() {
                    Ok(envelope) => {
                        record_extraction_cost(&mut span, envelope);
                        span.finish(SpanOutcome::Ok);
                    }
                    Err(_) => span.finish(SpanOutcome::Error),
                }
            }
            started
        }
        ConsolidateAction::Retract => retract(actor, history, input).await,
    }
}

const fn mode_label(mode: ConsolidateMode) -> &'static str {
    match mode {
        ConsolidateMode::Nrem => "nrem",
        ConsolidateMode::Rem => "rem",
        ConsolidateMode::Both => "both",
    }
}

fn record_extraction_cost(span: &mut SpanBuilder, envelope: &Envelope) {
    let Some(item) = envelope.items.first() else {
        return;
    };
    for (key, source) in [
        ("hypermind.extraction.llm_calls", &item["cost"]["llm_calls"]),
        (
            "hypermind.extraction.input_tokens",
            &item["cost"]["input_tokens"],
        ),
        (
            "hypermind.extraction.output_tokens",
            &item["cost"]["output_tokens"],
        ),
        ("hypermind.extraction.microusd", &item["cost"]["microusd"]),
        (
            "hypermind.extraction.derived_records",
            &item["stats"]["derived_records"],
        ),
    ] {
        if let Some(value) = source.as_u64() {
            span.attribute(Attribute::Integer(
                key,
                i64::try_from(value).unwrap_or(i64::MAX),
            ));
        }
    }
}

#[allow(clippy::too_many_lines)]
async fn start(
    actor: &ActorEngine,
    runtime: Option<&ConsolidationRuntime>,
    embedding: Option<&EmbeddingRuntime>,
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
        let source_events =
            window_source_events(actor, existing.source_first_lsn, existing.source_last_lsn)
                .await?;
        return Ok(run_envelope(
            actor,
            id,
            existing,
            true,
            source_events,
            false,
        ));
    }
    let generation = history.maximum_generation.saturating_add(1).max(1);
    let parent = history.active_generation;
    let watermark = history
        .watermarks
        .get(&scope_digest)
        .copied()
        .unwrap_or_default();
    let head = actor.stats().await?.applied.last_lsn.get();
    let source_first_lsn = watermark.saturating_add(1);
    let source_last_lsn = head;
    if head <= watermark {
        let mut envelope = Envelope::empty();
        envelope.items.push(json!({
            "generation": generation,
            "parent_generation": parent,
            "status": "skipped",
            "duplicate": false,
            "extracted": false,
            "source_first_lsn": source_first_lsn,
            "source_last_lsn": source_last_lsn,
            "source_events": 0,
        }));
        envelope
            .warnings
            .push("no source material above the extraction watermark".to_owned());
        return Ok(envelope);
    }
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
        source_first_lsn,
        source_last_lsn,
    };
    let mut events = vec![incoming(
        actor,
        &id,
        EventKind::ConsolidationOpened,
        EventPayload::ConsolidationOpened(Box::new(opened.clone())),
    )];
    let mut machine = PhaseMachine::resume(id.to_vec(), phases, []);
    let mut derived = Vec::new();
    let mut dropped_candidates = 0_u64;
    let mut llm_calls = 0_u64;
    let mut input_tokens = 0_u64;
    let mut output_tokens = 0_u64;
    let mut cost_microusd = 0_u64;
    let (mut window_clusters, source_events) =
        nrem_clusters_in_window(actor, source_first_lsn, source_last_lsn).await?;
    while let Some(work) = machine.next() {
        let started = machine.event(
            &work,
            ConsolidationPhaseState::Started,
            None,
            BudgetUsage {
                llm_calls,
                input_tokens,
                output_tokens,
                cost_microusd,
                wall_ms: 0,
            },
            dropped_candidates,
        );
        events.push(incoming(
            actor,
            &id,
            EventKind::ConsolidationPhase,
            EventPayload::ConsolidationPhase(Box::new(started.clone())),
        ));
        machine.record(&work, &started);
        if work.phase == ConsolidationPhaseName::Nrem {
            let mut clusters = std::mem::take(&mut window_clusters);
            let cluster_count = clusters.len();
            clusters.retain(mint_eligible);
            dropped_candidates = dropped_candidates.saturating_add(
                u64::try_from(cluster_count.saturating_sub(clusters.len()))
                    .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?,
            );
            clusters.truncate(usize::try_from(budget.max_llm_calls).unwrap_or(usize::MAX));
            let report = if clusters.is_empty() {
                NremReport::default()
            } else {
                let runtime = runtime.ok_or_else(|| Error::new(ErrorCode::OperationUnavailable))?;
                let provider = runtime.provider_for(actor.actor().get());
                consolidate_clusters(provider.as_ref(), &id, &clusters, &[])
                    .map_err(|_| Error::new(ErrorCode::OperationUnavailable))?
            };
            if report.llm_calls > budget.max_llm_calls
                || report.cost.tokens() > budget.max_tokens
                || report.cost.cost_microusd > budget.max_microusd
            {
                return Err(Error::new(ErrorCode::CapacityExceeded));
            }
            dropped_candidates = dropped_candidates.saturating_add(
                u64::try_from(report.dropped.len())
                    .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?,
            );
            llm_calls = llm_calls.saturating_add(report.llm_calls);
            input_tokens = input_tokens.saturating_add(report.cost.input_tokens);
            output_tokens = output_tokens.saturating_add(report.cost.output_tokens);
            cost_microusd = cost_microusd.saturating_add(report.cost.cost_microusd);
            for decision in report.decisions {
                if decision.action != MergeAction::Mint {
                    dropped_candidates = dropped_candidates.saturating_add(1);
                    continue;
                }
                let memory_id = memory_id(&id, &decision.cluster_id);
                derived.push(derived_incoming(
                    actor,
                    &id,
                    EventKind::MemoryMinted,
                    EventPayload::MemoryMinted(Box::new(MemoryMinted {
                        memory_id,
                        name: decision.name,
                        definition: decision.definition,
                        tags: decision.tags,
                        salience_micros: decision.salience_micros,
                        citations: decision.citations,
                    })),
                    decision.authority,
                    decision.model_provenance,
                ));
            }
        }
        let completed = machine.event(
            &work,
            ConsolidationPhaseState::Completed,
            None,
            BudgetUsage {
                llm_calls,
                input_tokens,
                output_tokens,
                cost_microusd,
                wall_ms: 0,
            },
            dropped_candidates,
        );
        events.push(incoming(
            actor,
            &id,
            EventKind::ConsolidationPhase,
            EventPayload::ConsolidationPhase(Box::new(completed.clone())),
        ));
        machine.record(&work, &completed);
    }
    let derived_records = derived.len() as u64;
    events.extend(derived);
    events.push(incoming(
        actor,
        &id,
        EventKind::ConsolidationClosed,
        EventPayload::ConsolidationClosed(Box::new(ConsolidationClosed {
            generation,
            expected_active_generation: parent,
            derived_records,
            dropped_candidates,
            llm_calls,
            input_tokens,
            output_tokens,
            cost_microusd,
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
        source_first_lsn,
        source_last_lsn,
        derived_records,
        dropped_candidates,
        llm_calls,
        input_tokens,
        output_tokens,
        cost_microusd,
    };
    let mut envelope = run_envelope(actor, &id, &summary, false, source_events, true);
    envelope.provenance.push(format!(
        "hm://{}/lsn/{}",
        actor.actor(),
        outcome.last_lsn.get()
    ));
    if let Some(embedding) = embedding {
        if let Ok(report) = relation::build(actor, embedding, MAXIMUM_RELATIONS).await {
            envelope.items[0]["relations_embedded"] = json!(report.embedded);
            envelope.items[0]["relation_space"] = json!(report.space_id);
        } else {
            envelope
                .gaps
                .push(json!({"kind": "relation_embedding_pending"}));
            envelope.warnings.push(
                "Run published; its relationship embeddings were not confirmed committed."
                    .to_owned(),
            );
        }
    }
    Ok(envelope)
}

fn mint_eligible(cluster: &ObservationCluster) -> bool {
    let roots = cluster
        .observations
        .iter()
        .map(|observation| observation.source.source_root)
        .collect::<BTreeSet<_>>();
    let conversations = cluster
        .observations
        .iter()
        .map(|observation| observation.source.conversation)
        .collect::<BTreeSet<_>>();
    roots.len() >= 3 && conversations.len() >= 2
}

pub async fn nrem_clusters(actor: &ActorEngine) -> Result<Vec<ObservationCluster>, Error> {
    Ok(nrem_clusters_in_window(actor, 1, u64::MAX).await?.0)
}

pub async fn nrem_clusters_in_window(
    actor: &ActorEngine,
    first_lsn: u64,
    last_lsn: u64,
) -> Result<(Vec<ObservationCluster>, u64), Error> {
    let observations = window_observations(actor, first_lsn, last_lsn).await?;
    let source_events =
        u64::try_from(observations.len()).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
    let clusters = cluster_observations(
        &observations,
        ClusterOptions {
            maximum_observations: 4_096,
            cosine_threshold_micros: 900_000,
            entity_overlap_threshold_micros: 1_000_000,
        },
    )?;
    Ok((clusters, source_events))
}

async fn window_source_events(
    actor: &ActorEngine,
    first_lsn: u64,
    last_lsn: u64,
) -> Result<u64, Error> {
    let observations = window_observations(actor, first_lsn, last_lsn).await?;
    u64::try_from(observations.len()).map_err(|_| Error::new(ErrorCode::CapacityExceeded))
}

async fn window_observations(
    actor: &ActorEngine,
    first_lsn: u64,
    last_lsn: u64,
) -> Result<Vec<PendingObservation>, Error> {
    let mut observations = Vec::new();
    for frame in actor
        .frames_since(LSN::new(first_lsn.saturating_sub(1)), None, usize::MAX)
        .await?
    {
        if frame.header.lsn.get() > last_lsn {
            break;
        }
        if !matches!(
            frame.header.kind,
            EventKind::UserMsg | EventKind::DeliveredMsg
        ) {
            continue;
        }
        let kind = event::EventKind::try_from(frame.header.kind as u8)
            .map_err(|()| Error::new(ErrorCode::InvalidKind))?;
        let verified = event::verify_event(&frame.sealed_payload, kind, Boundary::Disk)?;
        let content = match verified.envelope.payload {
            EventPayload::UserMsg(message) => message.content,
            EventPayload::DeliveredMsg(message) => message.content,
            _ => return Err(Error::new(ErrorCode::InvalidKind)),
        };
        let mut root = blake3::Hasher::new();
        root.update(b"hypermind.observation-root.v1\0");
        root.update(&frame.header.lsn.get().to_le_bytes());
        root.update(frame.header.conversation.as_bytes());
        root.update(&content);
        let entities = entities(&content);
        observations.push(PendingObservation {
            source: FrozenCandidate {
                lsn: frame.header.lsn.get(),
                conversation: frame.header.conversation.into_bytes(),
                source_root: *root.finalize().as_bytes(),
                content,
                kind: SourceKind::Declarative,
                authority: verified.envelope.authority,
            },
            salience_micros: 800_000,
            event_time_ns: frame.header.wall_timestamp_ns.get(),
            embedding: Vec::new(),
            entities,
        });
    }
    Ok(observations)
}

fn entities(content: &[u8]) -> Vec<String> {
    let text = String::from_utf8_lossy(content);
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|word| word.chars().next().is_some_and(char::is_uppercase))
        .map(str::to_owned)
        .collect()
}

fn memory_id(run_id: &[u8], cluster_id: &[u8; 32]) -> Vec<u8> {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"hypermind.memory.v1\0");
    hasher.update(run_id);
    hasher.update(cluster_id);
    hasher.finalize().as_bytes().to_vec()
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

fn list(history: RunHistory) -> Envelope {
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
    envelope.health = json!({
        "projection": "ready",
        "active_generation": history.active_generation,
        "watermarks": history
            .watermarks
            .iter()
            .map(|(scope_digest, through_lsn)| {
                json!({"scope_digest": hex(scope_digest), "through_lsn": through_lsn})
            })
            .collect::<Vec<_>>(),
    });
    envelope
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
    source_first_lsn: u64,
    source_last_lsn: u64,
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
    watermarks: BTreeMap<[u8; 32], u64>,
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
                        source_first_lsn: opened.source_first_lsn,
                        source_last_lsn: opened.source_last_lsn,
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
    let mut watermarks: BTreeMap<[u8; 32], u64> = BTreeMap::new();
    for run in history.runs.values() {
        if run.status != RunStatus::Published {
            continue;
        }
        let entry = watermarks.entry(run.scope_digest).or_default();
        *entry = (*entry).max(run.source_last_lsn);
    }
    history.watermarks = watermarks;
    Ok(history)
}

fn run_envelope(
    actor: &ActorEngine,
    id: &[u8],
    run: &RunSummary,
    duplicate: bool,
    source_events: u64,
    extracted: bool,
) -> Envelope {
    let mut envelope = Envelope::empty();
    envelope.items.push(json!({
        "run_id": hex(id),
        "generation": run.generation,
        "parent_generation": run.parent_generation,
        "status": run.status.name(),
        "first_lsn": run.first_lsn,
        "last_lsn": run.last_lsn,
        "source_first_lsn": run.source_first_lsn,
        "source_last_lsn": run.source_last_lsn,
        "source_events": source_events,
        "extracted": extracted,
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

fn derived_incoming(
    actor: &ActorEngine,
    run_id: &[u8],
    kind: EventKind,
    payload: EventPayload,
    authority: Authority,
    model_provenance: hm_schema::events::ModelProvenance,
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
            model_provenance: Some(Box::new(model_provenance)),
            authority,
            retention: Retention::Durable,
            sensitivity: Sensitivity::Personal,
            event_time_ns: 0,
        }),
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
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
