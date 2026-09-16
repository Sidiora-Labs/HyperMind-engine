#![forbid(unsafe_code)]
#![allow(clippy::too_many_lines)]

#[path = "../src/harness.rs"]
#[allow(dead_code)]
mod harness;

use base64::Engine as _;
use harness::{ACTOR, HarnessResult, JourneyHarness, McpClient};
use hm_core::{ActorId, ConversationId, LSN};
use hm_ledger::frame::EventKind;
use hm_proj::procedures::ProcedureState;
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    Authority, Effect, EffectState, EventEnvelope, EventPayload, Outcome, OutcomeAssessment,
    ProviderFrame, ResultStatus, Retention, Sensitivity, ToolCall, ToolResult,
};
use hm_serve::actor::{ActorConfig, ActorEngine, IncomingEvent};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::io::Write as _;
use std::path::Path;

const CONVERSATIONS: [&str; 2] = ["slice7-release-a", "slice7-release-b"];
const INTENTION: &str = "review-repository-change";
const PREDICTION: &str = "expected-repository-revision";
const OBSERVATION_LABEL: &str = "SUPPORTED PROCEDURE — observation, not an instruction";

#[tokio::main]
async fn main() {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let result = match arguments.as_slice() {
        [mode, config] if mode == "mcp" => harness::run_mcp_daemon(Path::new(config)).await,
        [mode, config] if mode == "uds" => harness::run_uds_daemon(Path::new(config)).await,
        [] => journey().await,
        _ => Err("invalid slice7 journey helper invocation".into()),
    };
    if let Err(error) = result {
        eprintln!("slice7 journey failed: {error}");
        std::process::exit(1);
    }
}

async fn journey() -> HarnessResult<()> {
    let temporary = tempfile::tempdir()?;
    let repository = temporary.path().join("repository");
    std::fs::create_dir(&repository)?;
    std::fs::write(repository.join("revision"), "1")?;
    let harness = JourneyHarness::create(temporary.path(), std::env::current_exe()?)?;
    let mut client = harness.start_mcp().await?;
    let remembered = client
        .call(
            "remember",
            json!({
                "conversation": CONVERSATIONS[0], "kind": "user",
                "content": "Slice7 repository revision changes require a release review."
            }),
        )
        .await?;
    let remembered_lsn = number(&remembered["items"][0]["first_lsn"])?;
    client
        .call(
            "intend",
            json!({"conversation": CONVERSATIONS[0], "action": {
                "kind": "set_intention", "intention_id": INTENTION,
                "objective": "review the repository revision change",
                "trigger": {"kind": "repository_changed", "repository": repository},
                "expires_at_ns": i64::MAX, "reply_route": "conversation"
            }}),
        )
        .await?;
    client
        .call("predict", json!({
            "conversation": CONVERSATIONS[0], "prediction_id": PREDICTION, "revision": 1,
            "task_id": "release-review", "attempt_id": "attempt-1", "operation_id": "revision-1",
            "mechanism": "filesystem-revision", "deadline_ns": i64::MAX,
            "uncertainty": "The observed revision may differ from the expected revision.",
            "predicates": [{"kind": "revision_equals", "scope": "repository",
                "property": "revision", "expected": "9"}]
        }))
        .await?;

    let mut episodes = Vec::new();
    let mut wake_lsn = 0;
    for episode in 0..3 {
        let conversation = CONVERSATIONS[usize::from(episode == 2)];
        let loop_id = format!("release-loop-{episode}");
        client
            .call(
                "intend",
                json!({"conversation": conversation, "action": {
                    "kind": "open_loop", "loop_id": loop_id,
                    "objective": "increment the repository revision and verify the resulting file"
                }}),
            )
            .await?;
        client.kill().await?;
        let observed =
            observe_revision(harness.config_path(), &repository, conversation, episode).await?;
        client = harness.start_mcp().await?;
        if episode == 0 {
            wake_lsn = observed
                .wake_lsn
                .ok_or("first observation omitted repository wake")?;
            let wake = client.call("intend", wake_input(wake_lsn)).await?;
            if wake["items"][0]["fired"]
                .as_array()
                .is_none_or(|items| items.len() != 1)
            {
                return Err(format!("repository change did not fire exactly once: {wake}").into());
            }
            assert_attention(&client).await?;
            assert_digest(&activate(&client).await?)?;
            let outcome = client
                .call("outcome", outcome_input(observed.result_lsn))
                .await?;
            if outcome["items"][0]["assessment"] != "contradicted"
                || outcome["items"][0]["observation_lsns"] != json!([observed.result_lsn])
            {
                return Err(format!(
                    "prediction was not contradicted by its observation: {outcome}"
                )
                .into());
            }
            let inspected = client
                .call("inspect", json!({"uri": format!("hm://{ACTOR}/lsn/{}", number(&outcome["items"][0]["lsn"])?)}))
                .await?;
            if !inspected["items"].as_array().is_some_and(|items| {
                items.iter().any(|item| {
                    item["lsn"] == observed.result_lsn && item["authority"] == "tool_observed"
                })
            }) {
                return Err("contradicted assessment omitted its observed citation chain".into());
            }
        }
        let closed = client
            .call(
                "intend",
                json!({"conversation": conversation, "action": {
                    "kind": "close_loop", "loop_id": loop_id, "reason": "done",
                    "cause": "the written revision was read back from the filesystem",
                    "evidence_lsns": [observed.outcome_lsn]
                }}),
            )
            .await?;
        let closed_lsn = number(&closed["items"][0]["lsn"])?;
        episodes.push((observed, closed_lsn));
        assert_procedure(&activate(&client).await?, episode + 1)?;
    }

    let attention_before = assert_attention(&client).await?;
    let calibration_before = assert_calibration(&client).await?;
    let before = client.call("inspect", json!({})).await?;
    client.kill().await?;
    verify_ledger(harness.config_path(), &episodes, &before).await?;

    let client = harness.start_mcp().await?;
    let after = client.call("inspect", json!({})).await?;
    for field in [
        "log_events",
        "applied_lsn",
        "applied_digest",
        "verification",
    ] {
        if before["items"][0][field] != after["items"][0][field] {
            return Err(format!("restart changed durable {field}").into());
        }
    }
    if assert_attention(&client).await? != attention_before
        || assert_calibration(&client).await? != calibration_before
    {
        return Err("restart changed attention history or prediction calibration".into());
    }
    let duplicate_wake = client.call("intend", wake_input(wake_lsn)).await?;
    let duplicate_outcome = client
        .call("outcome", outcome_input(episodes[0].0.result_lsn))
        .await?;
    if duplicate_wake["items"][0]["fired"] != json!([])
        || duplicate_outcome["items"][0]["duplicate"] != true
        || client.call("inspect", json!({})).await?["items"][0]["log_events"]
            != after["items"][0]["log_events"]
    {
        return Err("replayed wake or assessment appended duplicate events".into());
    }
    let activated = activate(&client).await?;
    assert_digest(&activated)?;
    assert_procedure(&activated, 3)?;
    let recalled = client
        .call(
            "recall",
            json!({"mode": "lexical", "query": "Slice7 repository revision", "limit": 10}),
        )
        .await?;
    if !recalled["items"].as_array().is_some_and(|items| {
        items
            .iter()
            .any(|item| item["lsn"] == remembered_lsn && item["authority"] == "user_asserted")
    }) {
        return Err("post-restart recall lost the original release review instruction".into());
    }
    client.kill().await?;
    let daemon = harness.start_uds().await?;
    let canonical = harness
        .uds_activate(CONVERSATIONS[0], "how increment repository revision", 8192)
        .await?;
    if !canonical.starts_with(b"HMA1")
        || !canonical
            .windows(OBSERVATION_LABEL.len())
            .any(|bytes| bytes == OBSERVATION_LABEL.as_bytes())
        || !canonical
            .windows(b"BATCH DIGEST".len())
            .any(|bytes| bytes == b"BATCH DIGEST")
    {
        return Err(
            "Unix-socket activation omitted durable anticipation or procedure evidence".into(),
        );
    }
    daemon.kill()?;
    println!(
        "slice7 journey passed: quiet-hours batch, observed contradiction, three independent episodes across two conversations, supported observation, MCP/UDS restart and recall; public slice7 benchmark gate is a separate prerequisite"
    );
    Ok(())
}

struct ObservedEpisode {
    call_lsn: u64,
    result_lsn: u64,
    effect_lsn: u64,
    outcome_lsn: u64,
    wake_lsn: Option<u64>,
    revision: String,
}

async fn observe_revision(
    configuration: &Path,
    repository: &Path,
    conversation: &str,
    episode: usize,
) -> HarnessResult<ObservedEpisode> {
    let actor = open_actor(configuration).await?;
    let path = repository.join("revision");
    let call_id = format!("revision-call-{episode}").into_bytes();
    let effect_id = format!("revision-effect-{episode}").into_bytes();
    let call_lsn = append(
        &actor,
        conversation,
        EventKind::ToolCall,
        EventPayload::ToolCall(Box::new(ToolCall {
            call_id: call_id.clone(),
            tool_name: "filesystem.increment_revision".into(),
            arguments: serde_json::to_vec(
                &json!({"path": path, "operation": "increment_and_read_back"}),
            )?,
        })),
        Authority::RuntimeFact,
    )
    .await?;
    let previous: u64 = std::fs::read_to_string(&path)?.parse()?;
    let next = previous
        .checked_add(1)
        .ok_or("revision overflow")?
        .to_string();
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&path)?;
    file.write_all(next.as_bytes())?;
    file.sync_all()?;
    drop(file);
    let revision = std::fs::read_to_string(&path)?;
    if revision != next {
        return Err("filesystem read-back disagreed with the completed write".into());
    }
    let evidence = serde_json::to_vec(&json!({"schema_version": 1, "observations": [{
        "kind": "revision_equals", "scope": "repository", "property": "revision",
        "value": revision, "executed": true, "resolvable": true
    }]}))?;
    let result_lsn = append(
        &actor,
        conversation,
        EventKind::ToolResult,
        EventPayload::ToolResult(Box::new(ToolResult {
            call_id,
            tool_call_lsn: call_lsn,
            status: ResultStatus::Ok,
            result: evidence.clone(),
        })),
        Authority::ToolObserved,
    )
    .await?;
    let effect_lsn = append(
        &actor,
        conversation,
        EventKind::Effect,
        EventPayload::Effect(Box::new(Effect {
            effect_id: effect_id.clone(),
            tool_call_lsn: call_lsn,
            state: EffectState::Returned,
        })),
        Authority::RuntimeFact,
    )
    .await?;
    let outcome_lsn = append(
        &actor,
        conversation,
        EventKind::Outcome,
        EventPayload::Outcome(Box::new(Outcome {
            effect_id,
            status: ResultStatus::Ok,
            detail: evidence,
            evidence_lsns: Some(vec![result_lsn]),
        })),
        Authority::RuntimeFact,
    )
    .await?;
    let wake_lsn = if episode == 0 {
        Some(
            append(
                &actor,
                conversation,
                EventKind::ProviderFrame,
                EventPayload::ProviderFrame(Box::new(ProviderFrame {
                    provider: "filesystem".into(),
                    api_content: serde_json::to_vec(&json!({
                        "wake": {"kind": "repository_changed", "key": repository},
                        "revision": revision
                    }))?,
                })),
                Authority::ExternalObserved,
            )
            .await?,
        )
    } else {
        None
    };
    actor.shutdown().await?;
    Ok(ObservedEpisode {
        call_lsn,
        result_lsn,
        effect_lsn,
        outcome_lsn,
        wake_lsn,
        revision,
    })
}

async fn verify_ledger(
    configuration: &Path,
    episodes: &[(ObservedEpisode, u64)],
    before: &Value,
) -> HarnessResult<()> {
    let actor = open_actor(configuration).await?;
    if actor.stats().await?.log_events != number(&before["items"][0]["log_events"])? {
        return Err("ledger recovery appended unexpected events".into());
    }
    let procedures = actor.procedures(10).await?;
    let procedure = procedures.first().ok_or("ledger has no mined procedure")?;
    if procedures.len() != 1
        || procedure.state != ProcedureState::Supported
        || procedure.adopted_lsn != 0
        || procedure.supports.len() != 3
        || procedure
            .supports
            .iter()
            .map(|support| &support.source_root)
            .collect::<BTreeSet<_>>()
            .len()
            != 3
        || procedure
            .supports
            .iter()
            .map(|support| &support.conversation)
            .collect::<BTreeSet<_>>()
            .len()
            != 2
    {
        return Err("procedure support did not come from three roots in two conversations".into());
    }
    let prediction = actor
        .prediction(PREDICTION.as_bytes().to_vec())
        .await?
        .ok_or("prediction was absent after restart")?;
    if prediction.assessment != Some(OutcomeAssessment::Contradicted)
        || prediction.observation_lsns != vec![episodes[0].0.result_lsn]
    {
        return Err(
            "persisted prediction lost the contradicted assessment or observed evidence".into(),
        );
    }
    for (index, (episode, closed_lsn)) in episodes.iter().enumerate() {
        if !procedure
            .supports
            .iter()
            .any(|support| support.episode_lsn == *closed_lsn)
        {
            return Err("procedure omitted a real closed loop".into());
        }
        let call = actor.verified_event(LSN::new(episode.call_lsn)).await?;
        let EventPayload::ToolCall(call) = call.envelope.payload else {
            return Err("episode root did not resolve to its tool call".into());
        };
        let result = actor.verified_event(LSN::new(episode.result_lsn)).await?;
        if result.envelope.authority != Authority::ToolObserved {
            return Err("tool observation lost observed authority".into());
        }
        let EventPayload::ToolResult(result) = result.envelope.payload else {
            return Err("episode omitted its tool result".into());
        };
        let actual: Value = serde_json::from_slice(&result.result)?;
        if result.tool_call_lsn != episode.call_lsn
            || result.call_id != call.call_id
            || actual["observations"][0]["value"] != episode.revision
            || episode.revision != (index + 2).to_string()
        {
            return Err(
                "tool result no longer reproduces its independent filesystem observation".into(),
            );
        }
        let effect = actor.verified_event(LSN::new(episode.effect_lsn)).await?;
        let EventPayload::Effect(effect) = effect.envelope.payload else {
            return Err("episode omitted its effect".into());
        };
        let outcome = actor.verified_event(LSN::new(episode.outcome_lsn)).await?;
        let EventPayload::Outcome(outcome) = outcome.envelope.payload else {
            return Err("episode omitted its outcome".into());
        };
        let closed = actor.verified_event(LSN::new(*closed_lsn)).await?;
        let EventPayload::LoopClosed(closed) = closed.envelope.payload else {
            return Err("procedure support did not resolve to a closed loop".into());
        };
        if effect.tool_call_lsn != episode.call_lsn
            || outcome.effect_id != effect.effect_id
            || outcome.evidence_lsns != Some(vec![episode.result_lsn])
            || closed.evidence_lsns != Some(vec![episode.outcome_lsn])
        {
            return Err("closed procedure episode lost its observed evidence chain".into());
        }
    }
    actor.shutdown().await?;
    Ok(())
}

async fn activate(client: &McpClient) -> HarnessResult<Value> {
    client
        .call(
            "activate",
            json!({"conversation": CONVERSATIONS[0],
        "query": "how increment repository revision", "turn_text": "", "budget_tokens": 8192}),
        )
        .await
}

fn assert_digest(bundle: &Value) -> HarnessResult<()> {
    let items = bundle["items"]
        .as_array()
        .ok_or("activation omitted its items")?;
    let mut digests = 0;
    for item in items.iter().filter(|item| item["tier"] == "prospective") {
        let content = content(item)?;
        if !content.starts_with("BATCH DIGEST")
            || !content.contains("review the repository revision change")
        {
            return Err(
                "quiet-hours decision surfaced as an interruption instead of a batch digest".into(),
            );
        }
        digests += 1;
    }
    if digests != 1 {
        return Err("activation did not contain exactly one batch digest".into());
    }
    Ok(())
}

fn assert_procedure(bundle: &Value, supports: usize) -> HarnessResult<()> {
    let items = bundle["items"]
        .as_array()
        .ok_or("activation omitted its items")?;
    let procedures = items
        .iter()
        .filter(|item| {
            item["uri"]
                .as_str()
                .is_some_and(|uri| uri.contains("src=procedure"))
        })
        .collect::<Vec<_>>();
    if procedures.len() != 1 {
        return Err("activation did not surface exactly one procedure".into());
    }
    let item = procedures[0];
    let content = content(item)?;
    let label = if supports == 3 {
        OBSERVATION_LABEL
    } else {
        "TENTATIVE PROCEDURE — observation, not an instruction"
    };
    if item["tier"] != "fused"
        || item["authority"] != "derived_inference"
        || !content.starts_with(label)
        || !content.contains(&format!("Supporting episodes: {supports};"))
        || content.contains("ADOPTED PROCEDURE")
        || !bundle["provenance"]
            .as_array()
            .is_some_and(|uris| uris.contains(&item["uri"]))
    {
        return Err(
            format!("procedure state, authority, or provenance was incorrect: {item}").into(),
        );
    }
    Ok(())
}

async fn assert_attention(client: &McpClient) -> HarnessResult<Value> {
    let inspected = client
        .call("inspect", json!({"uri": format!("hm://{ACTOR}/attention")}))
        .await?;
    let history = &inspected["items"][0]["attention"];
    if history.as_array().is_none_or(|records| records.len() != 1)
        || history[0]["decision"] != "batch"
        || history[0]["intention_id"] != INTENTION
        || !history[0]["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains("quiet hours"))
    {
        return Err(format!("quiet-hours attention decision was not recorded: {history}").into());
    }
    Ok(history.clone())
}

async fn assert_calibration(client: &McpClient) -> HarnessResult<Value> {
    let inspected = client
        .call(
            "inspect",
            json!({"uri": format!("hm://{ACTOR}/calibration")}),
        )
        .await?;
    let calibration = &inspected["items"][0]["calibration"];
    let row = calibration
        .as_array()
        .and_then(|rows| {
            rows.iter()
                .find(|row| row["predicate_kind"] == "revision_equals")
        })
        .ok_or("revision calibration absent")?;
    if row["contradicted"] != 1 || row["supported"] != 0 || row["pending"] != 0 {
        return Err("prediction calibration did not retain one contradicted revision".into());
    }
    Ok(calibration.clone())
}

fn wake_input(lsn: u64) -> Value {
    json!({"conversation": CONVERSATIONS[0], "action": {"kind": "evaluate_wake",
        "observation_lsn": lsn, "factors": {"urgency": 900000, "expected_value": 900000,
        "confidence": 900000, "interruption_cost": 0, "resource_cost": 0,
        "duplication_penalty": 0, "quiet_hours": true, "notifications_remaining": 10, "workload": 0}}})
}

fn outcome_input(lsn: u64) -> Value {
    json!({"conversation": CONVERSATIONS[0], "prediction_id": PREDICTION,
        "revision": 1, "observation_lsns": [lsn]})
}

fn number(value: &Value) -> HarnessResult<u64> {
    value
        .as_u64()
        .ok_or_else(|| format!("expected a ledger sequence number, got {value}").into())
}

fn content(item: &Value) -> HarnessResult<String> {
    Ok(String::from_utf8(
        base64::engine::general_purpose::STANDARD.decode(
            item["content_base64"]
                .as_str()
                .ok_or("activation item omitted content")?,
        )?,
    )?)
}

async fn append(
    actor: &ActorEngine,
    conversation: &str,
    kind: EventKind,
    payload: EventPayload,
    authority: Authority,
) -> HarnessResult<u64> {
    Ok(actor
        .append(vec![IncomingEvent {
            kind,
            conversation: ConversationId::derive(conversation),
            payload: encode_event_envelope(&EventEnvelope {
                schema_version: CURRENT_SCHEMA_VERSION,
                payload,
                connection_id: None,
                client_seq: 0,
                client_event_index: 0,
                client_event_count: 1,
                origin_actor: 0,
                run_id: None,
                model_provenance: None,
                authority,
                retention: Retention::Durable,
                sensitivity: Sensitivity::Personal,
                event_time_ns: 0,
            }),
        }])
        .await?
        .first_lsn
        .get())
}

async fn open_actor(path: &Path) -> HarnessResult<ActorEngine> {
    let config = hm_serve::config::load(path)?;
    let capability = config.actors.first().ok_or("configuration has no actor")?;
    Ok(ActorEngine::open(ActorConfig {
        actor_directory: config.actor_directory(capability.actor),
        actor: ActorId::new(capability.actor),
        user: config.user,
        kek: config.kek,
        projection_map_bytes: config.projection_map_bytes,
    })
    .await?)
}
