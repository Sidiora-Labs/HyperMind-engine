#![allow(clippy::missing_errors_doc)]

use hm_compose::bundle::Tier;
use hm_compose::tokens::FallbackWeights;
use hm_core::{ActorId, ConversationId, Error, ErrorCode};
use hm_ledger::frame::EventKind;
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    Authority, Binding, Effect, EffectState, EventEnvelope, EventPayload, IntentSet, LoopOpened,
    Retention, Sensitivity, ToolCall, UserMsg,
};
use hm_serve::actor::{ActivateRequest, ActorConfig, ActorEngine, IncomingEvent};
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub const TRIALS: usize = 8;
const TURN_STEPS: u8 = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContinuityResult {
    pub rust_trials: usize,
    pub typescript_passed: bool,
}

pub async fn run() -> Result<ContinuityResult, Box<dyn std::error::Error>> {
    for trial in 0..TRIALS {
        run_trial(u8::try_from(trial)?).await?;
    }
    let output = Command::new("npm")
        .args(["--prefix", "sdk/typescript", "test"])
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "TypeScript continuity suite failed:\n{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(ContinuityResult {
        rust_trials: TRIALS,
        typescript_passed: true,
    })
}

async fn run_trial(trial: u8) -> Result<(), Box<dyn std::error::Error>> {
    let temporary = tempfile::tempdir()?;
    let actor_directory = temporary.path().join("actor");
    let marker = temporary.path().join("progress");
    let kill_after = 1 + usize::from(mix(trial) % TURN_STEPS);
    let mut child = Command::new(std::env::current_exe()?)
        .arg("__continuity_child")
        .arg(&actor_directory)
        .arg(&marker)
        .arg(trial.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if child.try_wait()?.is_some() {
            return Err("continuity child exited before kill boundary".into());
        }
        let progress = fs::read_to_string(&marker)
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        if progress >= kill_after {
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            return Err("continuity child did not reach kill boundary".into());
        }
        thread::sleep(Duration::from_millis(5));
    }
    child.kill()?;
    let status = child.wait()?;
    if status.success() {
        return Err("continuity child was not killed".into());
    }

    let actor = ActorEngine::open(config(&actor_directory, trial)).await?;
    let next_sequence = actor
        .next_client_sequence([trial.wrapping_add(1); 16])
        .await?;
    let committed_steps = usize::try_from(next_sequence.saturating_sub(1))?;
    if !(kill_after..=usize::from(TURN_STEPS)).contains(&committed_steps) {
        return Err(format!(
            "restart found {committed_steps} committed steps after boundary {kill_after}"
        )
        .into());
    }
    for (index, events) in scripted_turn(trial)
        .into_iter()
        .enumerate()
        .skip(committed_steps - 1)
    {
        let client_seq = u64::try_from(index + 1)?;
        let outcome = actor
            .append_idempotent([trial.wrapping_add(1); 16], client_seq, events)
            .await?;
        if outcome.duplicate != (index + 1 == committed_steps) {
            return Err(format!(
                "step {client_seq} duplicate={}, expected {}",
                outcome.duplicate,
                index + 1 == committed_steps
            )
            .into());
        }
    }
    let actor_stats = actor.stats().await?;
    if actor_stats.log_events != 6 {
        return Err(format!(
            "idempotent replay applied {} events, expected 6",
            actor_stats.log_events
        )
        .into());
    }
    let bundle = actor
        .activate(ActivateRequest {
            conversation: conversation(trial),
            query: "continuity evidence".to_owned(),
            turn_text: "resume".to_owned(),
            budget_tokens: 16_384,
            token_weights: FallbackWeights::default(),
        })
        .await?;
    for tier in [Tier::Intent, Tier::Bindings, Tier::WorkLedger] {
        if bundle.sections[tier as usize].items.is_empty() {
            return Err(format!("continuity restart omitted {tier:?}").into());
        }
    }
    let work = &bundle.sections[Tier::WorkLedger as usize].items;
    if !work
        .iter()
        .any(|item| String::from_utf8_lossy(&item.content).contains("outcome_unknown"))
    {
        return Err("continuity restart lost unreconciled effect".into());
    }
    actor.shutdown().await?;
    Ok(())
}

pub async fn child(actor_directory: &Path, marker: &Path, trial: u8) -> Result<(), Error> {
    let actor = ActorEngine::open(config(actor_directory, trial)).await?;
    for (index, events) in scripted_turn(trial).into_iter().enumerate() {
        let client_seq =
            u64::try_from(index + 1).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
        let outcome = actor
            .append_idempotent([trial.wrapping_add(1); 16], client_seq, events)
            .await?;
        if outcome.duplicate {
            return Err(Error::new(ErrorCode::InvariantViolation));
        }
        fs::write(marker, (index + 1).to_string())
            .map_err(|_| Error::new(ErrorCode::WriteFailed))?;
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    tokio::time::sleep(Duration::from_secs(30)).await;
    Ok(())
}

fn config(actor_directory: &Path, trial: u8) -> ActorConfig {
    ActorConfig {
        actor_directory: actor_directory.to_owned(),
        actor: ActorId::new(7),
        user: [0x21; 16],
        kek: [trial.wrapping_add(1); 32],
        projection_map_bytes: 32 * 1024 * 1024,
    }
}

fn conversation(trial: u8) -> ConversationId {
    ConversationId::derive(&format!("continuity-trial-{trial}"))
}

fn scripted_turn(trial: u8) -> Vec<Vec<IncomingEvent>> {
    let conversation = conversation(trial);
    vec![
        vec![incoming(
            EventKind::IntentSet,
            conversation,
            EventPayload::IntentSet(Box::new(IntentSet {
                objective: b"resume the exact interrupted turn".to_vec(),
            })),
            Authority::UserAsserted,
        )],
        vec![incoming(
            EventKind::LoopOpened,
            conversation,
            EventPayload::LoopOpened(Box::new(LoopOpened {
                loop_id: format!("task-{trial}").into_bytes(),
                objective: b"complete the external write".to_vec(),
            })),
            Authority::UserAsserted,
        )],
        vec![
            incoming(
                EventKind::UserMsg,
                conversation,
                EventPayload::UserMsg(Box::new(UserMsg {
                    content: b"continuity evidence".to_vec(),
                })),
                Authority::ExternalObserved,
            ),
            incoming(
                EventKind::Binding,
                conversation,
                EventPayload::Binding(Box::new(Binding {
                    task: Some(format!("task-{trial}").into_bytes()),
                    scope: None,
                    canonical_entity: "repository".to_owned(),
                    property: "revision".to_owned(),
                    evidence_lsn: 3,
                    revision: format!("revision-{trial}").into_bytes(),
                    freshness_requirement_ns: u64::MAX,
                })),
                Authority::RuntimeFact,
            ),
        ],
        vec![
            incoming(
                EventKind::ToolCall,
                conversation,
                EventPayload::ToolCall(Box::new(ToolCall {
                    call_id: format!("call-{trial}").into_bytes(),
                    tool_name: "write_file".to_owned(),
                    arguments: b"path=continuity".to_vec(),
                })),
                Authority::AssistantGenerated,
            ),
            incoming(
                EventKind::Effect,
                conversation,
                EventPayload::Effect(Box::new(Effect {
                    effect_id: format!("effect-{trial}").into_bytes(),
                    tool_call_lsn: 5,
                    state: EffectState::OutcomeUnknown,
                })),
                Authority::RuntimeFact,
            ),
        ],
    ]
}

fn incoming(
    kind: EventKind,
    conversation: ConversationId,
    payload: EventPayload,
    authority: Authority,
) -> IncomingEvent {
    IncomingEvent {
        kind,
        conversation,
        payload: encode_event_envelope(&EventEnvelope {
            schema_version: CURRENT_SCHEMA_VERSION,
            payload,
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 0,
            origin_actor: 7,
            run_id: None,
            model_provenance: None,
            authority,
            retention: Retention::CurrentState,
            sensitivity: Sensitivity::Personal,
            event_time_ns: 1,
        }),
    }
}

const fn mix(value: u8) -> u8 {
    value.wrapping_mul(73).wrapping_add(41) ^ value.rotate_left(3)
}
