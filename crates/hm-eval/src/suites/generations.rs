#![allow(clippy::missing_errors_doc)]

use hm_core::{ActorId, ConversationId};
use hm_cortex::run::retraction_event;
use hm_ledger::frame::EventKind;
use hm_proj::memories::MemoryProjection;
use hm_proj::runs::{RunStatus, RunsProjection};
use hm_proj::store::ProjectionStore;
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    Authority, ConsolidationBudget, ConsolidationClosed, ConsolidationOpened,
    ConsolidationPhaseName, EventEnvelope, EventPayload, MemoryMinted, ModelProvenance,
    PromptVersion, ProvenanceRange, Retention, Sensitivity,
};
use hm_serve::actor::{ActorConfig, ActorEngine, IncomingEvent};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const CONNECTION: [u8; 16] = [9; 16];

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GenerationResult {
    pub staged_hidden_after_kill: bool,
    pub publish_after_restart: bool,
    pub lost_ack_deduplicated: bool,
    pub published_generation_count: usize,
    pub rollback_immediate: bool,
    pub leased_view_overridden: bool,
}

pub async fn run() -> Result<GenerationResult, Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let actor_directory = temporary.path().join("actor");
    let stage_marker = temporary.path().join("stage-committed");
    kill_after_marker(
        spawn_child(&actor_directory, &stage_marker, "stage")?,
        &stage_marker,
    )?;

    let store = ProjectionStore::open(&actor_directory, 64 * 1024 * 1024)?;
    let snapshot = store.begin_snapshot()?;
    let staged_hidden_after_kill = RunsProjection::active_generation(&snapshot)? == 0
        && MemoryProjection::get_visible(&snapshot, 0, b"memory-one")?.is_none();
    drop(snapshot);
    drop(store);

    let actor = ActorEngine::open(config(&actor_directory)).await?;
    actor
        .append(vec![generation_events(1, 0, b"memory-one")[2].clone()])
        .await?;
    actor.shutdown().await?;
    let store = ProjectionStore::open(&actor_directory, 64 * 1024 * 1024)?;
    let snapshot = store.begin_snapshot()?;
    let publish_after_restart = RunsProjection::active_generation(&snapshot)? == 1
        && MemoryProjection::get_visible(&snapshot, 1, b"memory-one")?.is_some();
    drop(snapshot);
    drop(store);

    let ack_marker = temporary.path().join("ack-lost");
    kill_after_marker(
        spawn_child(&actor_directory, &ack_marker, "lost_ack")?,
        &ack_marker,
    )?;
    let actor = ActorEngine::open(config(&actor_directory)).await?;
    let replay = actor
        .append_idempotent(CONNECTION, 1, generation_events(2, 1, b"memory-two"))
        .await?;
    let stats = actor.stats().await?;
    actor.shutdown().await?;
    let lost_ack_deduplicated = replay.duplicate && stats.log_events == 6;

    let store = ProjectionStore::open(&actor_directory, 64 * 1024 * 1024)?;
    let snapshot = store.begin_snapshot()?;
    let published_generation_count = (1..=3)
        .filter(|generation| {
            RunsProjection::run_for_generation(&snapshot, *generation)
                .ok()
                .flatten()
                .is_some_and(|run| run.status == RunStatus::Published)
        })
        .count();
    let lease_generation = RunsProjection::active_generation(&snapshot)?;
    let run_two = RunsProjection::run_for_generation(&snapshot, 2)?
        .ok_or("generation two was not recorded")?;
    drop(snapshot);
    drop(store);

    let actor = ActorEngine::open(config(&actor_directory)).await?;
    let event = retraction_event(run_two.run_id.clone(), 1, "eval rollback".to_owned());
    actor
        .append(vec![incoming(
            EventKind::ConsolidationRetracted,
            &run_two.run_id,
            EventPayload::ConsolidationRetracted(Box::new(event)),
            None,
        )])
        .await?;
    actor.shutdown().await?;
    let store = ProjectionStore::open(&actor_directory, 64 * 1024 * 1024)?;
    let snapshot = store.begin_snapshot()?;
    let rollback_immediate = RunsProjection::active_generation(&snapshot)? == 1
        && MemoryProjection::get_visible(&snapshot, 1, b"memory-one")?.is_some();
    let leased_view_overridden = lease_generation == 2
        && MemoryProjection::get_visible(&snapshot, lease_generation, b"memory-two")?.is_none();
    Ok(GenerationResult {
        staged_hidden_after_kill,
        publish_after_restart,
        lost_ack_deduplicated,
        published_generation_count,
        rollback_immediate,
        leased_view_overridden,
    })
}

pub async fn child(
    actor_directory: &Path,
    marker: &Path,
    mode: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let actor = ActorEngine::open(config(actor_directory)).await?;
    match mode {
        "stage" => {
            actor
                .append(generation_events(1, 0, b"memory-one")[..2].to_vec())
                .await?;
        }
        "lost_ack" => {
            actor
                .append_idempotent(CONNECTION, 1, generation_events(2, 1, b"memory-two"))
                .await?;
        }
        _ => return Err("unknown generation child mode".into()),
    }
    std::fs::write(marker, b"committed")?;
    thread::sleep(Duration::from_secs(60));
    actor.shutdown().await?;
    Ok(())
}

fn spawn_child(actor_directory: &Path, marker: &Path, mode: &str) -> Result<Child, std::io::Error> {
    Command::new(std::env::current_exe()?)
        .arg("__generation_child")
        .arg(actor_directory)
        .arg(marker)
        .arg(mode)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
}

fn kill_after_marker(mut child: Child, marker: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let deadline = Instant::now() + Duration::from_secs(20);
    while !marker.exists() {
        if let Some(status) = child.try_wait()? {
            let stderr = child
                .stderr
                .take()
                .map(|mut stderr| {
                    let mut output = String::new();
                    let _ = std::io::Read::read_to_string(&mut stderr, &mut output);
                    output
                })
                .unwrap_or_default();
            return Err(format!("generation child exited {status}: {stderr}").into());
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            return Err("generation child did not reach commit marker".into());
        }
        thread::sleep(Duration::from_millis(5));
    }
    child.kill()?;
    if child.wait()?.success() {
        return Err("generation child was not killed".into());
    }
    Ok(())
}

fn config(actor_directory: &Path) -> ActorConfig {
    ActorConfig {
        actor_directory: actor_directory.to_path_buf(),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 64 * 1024 * 1024,
    }
}

fn generation_events(generation: u64, parent: u64, memory_id: &[u8]) -> Vec<IncomingEvent> {
    let run_id = run_id(generation);
    vec![
        incoming(
            EventKind::ConsolidationOpened,
            &run_id,
            EventPayload::ConsolidationOpened(Box::new(ConsolidationOpened {
                scope_digest: vec![u8::try_from(generation).unwrap_or(u8::MAX); 32],
                cadence_key: format!("eval-generation-{generation}"),
                generation,
                expected_active_generation: parent,
                phases: vec![ConsolidationPhaseName::Nrem],
                prompts: vec![PromptVersion {
                    prompt_id: "merge-cluster".to_owned(),
                    version: 1,
                    model_id: "fixture-model".to_owned(),
                }],
                budget: Box::new(ConsolidationBudget {
                    max_llm_calls: 4,
                    max_tokens: 4_000,
                    max_microusd: 4_000,
                    max_wall_ms: 30_000,
                }),
            })),
            None,
        ),
        incoming(
            EventKind::MemoryMinted,
            &run_id,
            EventPayload::MemoryMinted(Box::new(MemoryMinted {
                memory_id: memory_id.to_vec(),
                name: format!("Generation {generation}"),
                definition: format!("persistent dream generation {generation}").into_bytes(),
                tags: vec!["eval".to_owned()],
                salience_micros: 800_000,
                citations: vec![ProvenanceRange {
                    first_lsn: 1,
                    last_lsn: 1,
                    byte_start: 0,
                    byte_end: 1,
                }],
            })),
            Some(model(generation)),
        ),
        incoming(
            EventKind::ConsolidationClosed,
            &run_id,
            EventPayload::ConsolidationClosed(Box::new(ConsolidationClosed {
                generation,
                expected_active_generation: parent,
                derived_records: 1,
                dropped_candidates: 0,
                llm_calls: 1,
                input_tokens: 10,
                output_tokens: 5,
                cost_microusd: 2,
            })),
            None,
        ),
    ]
}

fn incoming(
    kind: EventKind,
    run_id: &[u8],
    payload: EventPayload,
    model_provenance: Option<ModelProvenance>,
) -> IncomingEvent {
    let derived = model_provenance.is_some();
    IncomingEvent {
        kind,
        conversation: ConversationId::derive("generation-eval"),
        payload: encode_event_envelope(&EventEnvelope {
            schema_version: CURRENT_SCHEMA_VERSION,
            payload,
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 0,
            origin_actor: 7,
            run_id: Some(run_id.to_vec()),
            model_provenance: model_provenance.map(Box::new),
            authority: if derived {
                Authority::DerivedInference
            } else {
                Authority::RuntimeFact
            },
            retention: Retention::Durable,
            sensitivity: Sensitivity::Personal,
            event_time_ns: 0,
        }),
    }
}

fn model(generation: u64) -> ModelProvenance {
    ModelProvenance {
        model_id: "fixture-model".to_owned(),
        prompt_id: "merge-cluster".to_owned(),
        prompt_version: 1,
        temperature: 0.0,
        call_id: Some(generation.to_le_bytes().to_vec()),
        input_tokens: 10,
        output_tokens: 5,
        cache_read_tokens: 0,
        cache_write_tokens: 0,
        cost_microusd: 2,
    }
}

fn run_id(generation: u64) -> [u8; 32] {
    *blake3::hash(&generation.to_le_bytes()).as_bytes()
}
