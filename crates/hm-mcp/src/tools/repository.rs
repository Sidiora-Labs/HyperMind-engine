#![allow(clippy::missing_errors_doc)]

use crate::Envelope;
use crate::tools::consolidate::{
    ConsolidateBudget, ConsolidateInput, ConsolidateMode, RunHistory, derived_incoming, hex,
    incoming, phases, prompts,
};
use hm_core::{ConversationId, Error, ErrorCode, LSN};
use hm_cortex::budget::BudgetUsage;
use hm_cortex::repograph::{
    self, RepoDropReason, RepoEdge, RepoGraph, RepoNode, RepoSnapshotShard,
};
use hm_cortex::run::PhaseMachine;
use hm_ledger::frame::EventKind;
use hm_schema::event::{
    self, Boundary, REPOSITORY_EXTRACT_MODEL_ID, REPOSITORY_EXTRACT_PROMPT_ID,
    REPOSITORY_EXTRACT_PROMPT_VERSION, REPOSITORY_EXTRACT_RUN_PREFIX, REPOSITORY_SNAPSHOT_PROVIDER,
};
use hm_schema::events::{
    Authority, ConsolidationClosed, ConsolidationOpened, ConsolidationPhaseState, EdgeAsserted,
    EventPayload, MemoryMinted, MemoryRevised, ModelProvenance, ProvenanceRange,
};
use hm_serve::actor::{ActorEngine, IncomingEvent, MAXIMUM_GRAPH_NEIGHBOURS};
use serde_json::json;
use std::collections::BTreeMap;

pub const MAXIMUM_STAGED_BATCH: usize = 200;

const NODE_SALIENCE_MICROS: u32 = 800_000;
const MAXIMUM_REPORTED_DROPS: usize = 8;
const DECLARED_BUDGET: ConsolidateBudget = ConsolidateBudget {
    max_llm_calls: 1,
    max_tokens: 1,
    max_microusd: 1,
    max_wall_ms: 1,
};

#[allow(clippy::too_many_lines)]
pub(crate) async fn run(
    actor: &ActorEngine,
    history: RunHistory,
    input: ConsolidateInput,
) -> Result<Envelope, Error> {
    let RunHistory {
        runs,
        maximum_generation,
        active_generation,
        ..
    } = history;
    let repository = input
        .scope
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?
        .to_owned();
    let budget = input.budget.unwrap_or(DECLARED_BUDGET);
    if budget.max_llm_calls == 0
        || budget.max_tokens == 0
        || budget.max_microusd == 0
        || budget.max_wall_ms == 0
    {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let cadence_key = input
        .cadence_key
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| scope_label(&repository));

    let shards = latest_snapshot(actor, &repository).await?;
    let source_first_lsn = shards
        .iter()
        .map(|shard| shard.lsn)
        .min()
        .unwrap_or_default();
    let source_last_lsn = shards
        .iter()
        .map(|shard| shard.lsn)
        .max()
        .unwrap_or_default();
    let snapshot =
        repograph::parse_snapshot(&shards).map_err(|_| Error::new(ErrorCode::SchemaInvalid))?;
    let facts = snapshot.facts.len();
    let graph = repograph::resolve(&snapshot);
    let id = extraction_run_id(&graph.digest);

    if let Some(existing) = runs.get(&id) {
        let mut envelope = report(
            &id,
            &graph,
            shards.len(),
            facts,
            existing.generation,
            existing.parent_generation,
            true,
        );
        envelope.items[0]["status"] = json!(existing.status.name());
        envelope
            .provenance
            .push(format!("hm://{}/lsn/{}", actor.actor(), existing.last_lsn));
        return Ok(envelope);
    }

    let generation = maximum_generation.saturating_add(1).max(1);
    let parent = active_generation;
    let visible = actor.memories(MAXIMUM_GRAPH_NEIGHBOURS).await?;
    let saturated = visible.len() == MAXIMUM_GRAPH_NEIGHBOURS;
    let versions = visible
        .into_iter()
        .map(|record| (record.memory_id, record.version_lsn))
        .collect::<BTreeMap<_, _>>();

    let provenance = extraction_provenance(&graph.digest);
    let mut derived = Vec::with_capacity(graph.nodes.len() + graph.edges.len());
    for node in &graph.nodes {
        derived.push(node_event(
            actor,
            &id,
            &provenance,
            node,
            versions.get(node.node_id.as_slice()).copied(),
        ));
    }
    for edge in &graph.edges {
        derived.push(edge_event(actor, &id, &provenance, edge));
    }
    let derived_records =
        u64::try_from(derived.len()).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
    let dropped_candidates = u64::try_from(graph.dropped.len())
        .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?
        .saturating_add(graph.skipped);

    let opened = ConsolidationOpened {
        scope_digest: blake3::hash(scope_label(&repository).as_bytes())
            .as_bytes()
            .to_vec(),
        cadence_key,
        generation,
        expected_active_generation: parent,
        phases: phases(ConsolidateMode::Repository),
        prompts: prompts(ConsolidateMode::Repository),
        budget: Box::new(budget.into()),
        source_first_lsn,
        source_last_lsn,
    };
    let mut machine = PhaseMachine::resume(id.clone(), opened.phases.clone(), []);
    let work = machine
        .next()
        .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))?;
    let started = machine.event(
        &work,
        ConsolidationPhaseState::Started,
        None,
        BudgetUsage::default(),
        0,
    );
    machine.record(&work, &started);
    actor
        .append(vec![
            incoming(
                actor,
                &id,
                EventKind::ConsolidationOpened,
                EventPayload::ConsolidationOpened(Box::new(opened)),
            ),
            incoming(
                actor,
                &id,
                EventKind::ConsolidationPhase,
                EventPayload::ConsolidationPhase(Box::new(started)),
            ),
        ])
        .await?;
    for batch in derived.chunks(MAXIMUM_STAGED_BATCH) {
        actor.append(batch.to_vec()).await?;
    }
    let completed = machine.event(
        &work,
        ConsolidationPhaseState::Completed,
        None,
        BudgetUsage::default(),
        dropped_candidates,
    );
    let outcome = actor
        .append(vec![
            incoming(
                actor,
                &id,
                EventKind::ConsolidationPhase,
                EventPayload::ConsolidationPhase(Box::new(completed)),
            ),
            incoming(
                actor,
                &id,
                EventKind::ConsolidationClosed,
                EventPayload::ConsolidationClosed(Box::new(ConsolidationClosed {
                    generation,
                    expected_active_generation: parent,
                    derived_records,
                    dropped_candidates,
                    llm_calls: 0,
                    input_tokens: 0,
                    output_tokens: 0,
                    cost_microusd: 0,
                })),
            ),
        ])
        .await?;

    let mut envelope = report(&id, &graph, shards.len(), facts, generation, parent, false);
    envelope.items[0]["status"] = json!("published");
    envelope.provenance.push(format!(
        "hm://{}/lsn/{}",
        actor.actor(),
        outcome.last_lsn.get()
    ));
    if saturated {
        envelope
            .gaps
            .push(json!({"kind": "visible_memories_truncated", "limit": MAXIMUM_GRAPH_NEIGHBOURS}));
        envelope.warnings.push(
            "The visible memory read was truncated; a node beyond that bound was minted rather than revised."
                .to_owned(),
        );
    }
    Ok(envelope)
}

fn report(
    id: &[u8],
    graph: &RepoGraph,
    shards: usize,
    facts: usize,
    generation: u64,
    parent_generation: u64,
    duplicate: bool,
) -> Envelope {
    let dropped_candidates = u64::try_from(graph.dropped.len())
        .unwrap_or(u64::MAX)
        .saturating_add(graph.skipped);
    let derived_records = u64::try_from(graph.nodes.len() + graph.edges.len()).unwrap_or(u64::MAX);
    let mut envelope = Envelope::empty();
    envelope.items.push(json!({
        "run_id": hex(id),
        "repository": graph.repository,
        "snapshot_digest": hex(&graph.digest),
        "shards": shards,
        "facts": facts,
        "skipped": graph.skipped,
        "nodes": graph.nodes.len(),
        "edges": graph.edges.len(),
        "dropped": graph.dropped.len(),
        "generation": generation,
        "parent_generation": parent_generation,
        "duplicate": duplicate,
        "cost": {"llm_calls": 0, "input_tokens": 0, "output_tokens": 0, "microusd": 0},
        "stats": {"derived_records": derived_records, "dropped_candidates": dropped_candidates},
    }));
    for drop in graph.dropped.iter().take(MAXIMUM_REPORTED_DROPS) {
        envelope.gaps.push(json!({
            "kind": "repository_relation_dropped",
            "source": drop.source,
            "relation": drop.relation,
            "target": drop.target,
            "reason": drop_reason(drop.reason),
        }));
    }
    envelope.warnings.push(
        "Facts a newer snapshot no longer declares are not retracted by this run; the older records stay visible."
            .to_owned(),
    );
    envelope
}

const fn drop_reason(reason: RepoDropReason) -> &'static str {
    match reason {
        RepoDropReason::UnknownRelation => "unknown_relation",
        RepoDropReason::UnresolvedTarget => "unresolved_target",
        RepoDropReason::AmbiguousTarget => "ambiguous_target",
        RepoDropReason::SelfReference => "self_reference",
        RepoDropReason::NodeLimit => "node_limit",
        RepoDropReason::EdgeLimit => "edge_limit",
    }
}

fn scope_label(repository: &str) -> String {
    format!("repository:{repository}")
}

fn extraction_provenance(digest: &[u8; 32]) -> ModelProvenance {
    ModelProvenance {
        model_id: REPOSITORY_EXTRACT_MODEL_ID.to_owned(),
        prompt_id: REPOSITORY_EXTRACT_PROMPT_ID.to_owned(),
        prompt_version: REPOSITORY_EXTRACT_PROMPT_VERSION,
        temperature: 0.0,
        call_id: Some(digest.to_vec()),
        input_tokens: 0,
        output_tokens: 0,
        cache_read_tokens: 0,
        cache_write_tokens: 0,
        cost_microusd: 0,
    }
}

fn extraction_run_id(digest: &[u8; 32]) -> Vec<u8> {
    let mut id = String::from(REPOSITORY_EXTRACT_RUN_PREFIX);
    id.push_str(&hex(digest));
    id.into_bytes()
}

async fn latest_snapshot(
    actor: &ActorEngine,
    repository: &str,
) -> Result<Vec<RepoSnapshotShard>, Error> {
    let conversation = ConversationId::derive(&scope_label(repository));
    let mut current: Vec<RepoSnapshotShard> = Vec::new();
    let mut expected = 0_u32;
    let mut latest: Option<Vec<RepoSnapshotShard>> = None;
    for frame in actor
        .frames_since(LSN::new(0), Some(conversation), usize::MAX)
        .await?
    {
        if frame.header.kind != EventKind::ProviderFrame {
            continue;
        }
        let kind = event::EventKind::try_from(frame.header.kind as u8)
            .map_err(|()| Error::new(ErrorCode::InvalidKind))?;
        let verified = event::verify_event(&frame.sealed_payload, kind, Boundary::Disk)?;
        let index = verified.envelope.client_event_index;
        let count = verified.envelope.client_event_count.max(1);
        let EventPayload::ProviderFrame(observed) = verified.envelope.payload else {
            continue;
        };
        if observed.provider != REPOSITORY_SNAPSHOT_PROVIDER {
            continue;
        }
        if index == 0 {
            current.clear();
            expected = count;
        }
        if count != expected || u32::try_from(current.len()).unwrap_or(u32::MAX) != index {
            current.clear();
            continue;
        }
        current.push(RepoSnapshotShard {
            lsn: frame.header.lsn.get(),
            content: observed.api_content,
        });
        if u32::try_from(current.len()).unwrap_or(u32::MAX) == expected {
            latest = Some(std::mem::take(&mut current));
        }
    }
    latest
        .filter(|shards| !shards.is_empty())
        .ok_or_else(|| Error::new(ErrorCode::OperationUnavailable))
}

fn node_event(
    actor: &ActorEngine,
    id: &[u8],
    provenance: &ModelProvenance,
    node: &RepoNode,
    previous_lsn: Option<u64>,
) -> IncomingEvent {
    let citations = vec![citation(node.citation)];
    if let Some(previous_lsn) = previous_lsn {
        derived_incoming(
            actor,
            id,
            EventKind::MemoryRevised,
            EventPayload::MemoryRevised(Box::new(MemoryRevised {
                memory_id: node.node_id.to_vec(),
                previous_lsn,
                name: node.display_name.clone(),
                definition: node.definition.clone(),
                tags: node.tags.clone(),
                salience_micros: NODE_SALIENCE_MICROS,
                citations,
            })),
            Authority::DerivedInference,
            provenance.clone(),
        )
    } else {
        derived_incoming(
            actor,
            id,
            EventKind::MemoryMinted,
            EventPayload::MemoryMinted(Box::new(MemoryMinted {
                memory_id: node.node_id.to_vec(),
                name: node.display_name.clone(),
                definition: node.definition.clone(),
                tags: node.tags.clone(),
                salience_micros: NODE_SALIENCE_MICROS,
                citations,
            })),
            Authority::DerivedInference,
            provenance.clone(),
        )
    }
}

fn edge_event(
    actor: &ActorEngine,
    id: &[u8],
    provenance: &ModelProvenance,
    edge: &RepoEdge,
) -> IncomingEvent {
    derived_incoming(
        actor,
        id,
        EventKind::EdgeAsserted,
        EventPayload::EdgeAsserted(Box::new(EdgeAsserted {
            edge_id: edge.edge_id.to_vec(),
            source_id: edge.source_id.to_vec(),
            target_id: edge.target_id.to_vec(),
            relation: edge.relation.clone(),
            weight_micros: edge.weight_micros,
            valid_from_ns: 0,
            valid_to_ns: 0,
            citations: vec![citation(edge.citation)],
        })),
        Authority::DerivedInference,
        provenance.clone(),
    )
}

const fn citation(source: repograph::RepoCitation) -> ProvenanceRange {
    ProvenanceRange {
        first_lsn: source.lsn,
        last_lsn: source.lsn,
        byte_start: source.byte_start,
        byte_end: source.byte_end,
    }
}
