#![allow(clippy::missing_errors_doc)]

use hm_core::{Error, ErrorCode};
use hm_cortex::procedures::{self, Episode};
use hm_cortex::prospective::WakeSignal;
use hm_ledger::frame::FrameHeader;
use hm_proj::intentions::TriggerKind;
use hm_schema::events::{
    AttentionDecision, Authority, EventEnvelope, EventPayload, LoopCloseReason, OutcomeAssessment,
    ProcedureMined, ResultStatus,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct WakeEvaluation {
    pub observation_lsn: u64,
    pub fired: Vec<WakeDecision>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct WakeDecision {
    pub intention_id: Vec<u8>,
    pub wake_id: Vec<u8>,
    pub decision: AttentionDecision,
    pub reason: String,
    pub fired_lsn: u64,
    pub decision_lsn: u64,
    pub rearmed_intention_id: Option<Vec<u8>>,
}

pub(crate) fn signal(header: &FrameHeader, event: &EventEnvelope) -> Result<WakeSignal, Error> {
    let now_ns = if event.event_time_ns == 0 {
        header.wall_timestamp_ns.get()
    } else {
        event.event_time_ns
    };
    let typed = match &event.payload {
        EventPayload::LoopClosed(value) => Some(("loop_closed", value.loop_id.clone())),
        EventPayload::OutcomeObserved(value)
            if matches!(
                value.assessment,
                OutcomeAssessment::Supported | OutcomeAssessment::Contradicted
            ) =>
        {
            Some(("prediction_resolved", value.prediction_id.clone()))
        }
        EventPayload::Assertion(value) => Some((
            "belief_changed",
            value.canonical_identity.as_bytes().to_vec(),
        )),
        _ => None,
    };
    let (kind, key) = if let Some(typed) = typed {
        typed
    } else {
        if !observed(event.authority) {
            return Err(Error::new(ErrorCode::ForbiddenKind));
        }
        let bytes = match &event.payload {
            EventPayload::ToolResult(value) => &value.result,
            EventPayload::ProviderFrame(value) => &value.api_content,
            EventPayload::Supervisor(value) => &value.evidence,
            _ => return Err(Error::new(ErrorCode::InvalidArgument)),
        };
        let json: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|_| Error::new(ErrorCode::InvalidArgument))?;
        let wake = json
            .get("wake")
            .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
        let kind = match wake.get("kind").and_then(serde_json::Value::as_str) {
            Some("at" | "time") => "time",
            Some("schedule") => "schedule",
            Some("child_terminal") => "child_terminal",
            Some("process_exit") => "process_exit",
            Some("file_changed") => "file_changed",
            Some("repository_changed") => "repository_changed",
            Some("channel_message") => "channel_message",
            Some("external_condition") => "external_condition",
            Some("user_response") => "user_response",
            Some("entity_mentioned") => "entity_mentioned",
            Some("loop_closed") => "loop_closed",
            Some("prediction_resolved") => "prediction_resolved",
            Some("belief_changed") => "belief_changed",
            _ => return Err(Error::new(ErrorCode::InvalidArgument)),
        };
        let key = wake
            .get("key")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .as_bytes()
            .to_vec();
        if kind != "time" && key.is_empty() {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        (kind, key)
    };
    Ok(WakeSignal {
        trigger_lsn: header.lsn.get(),
        now_ns,
        kind,
        key,
    })
}

pub(crate) fn trigger_kind(signal: &WakeSignal) -> TriggerKind {
    match signal.kind {
        "time" => TriggerKind::At,
        "schedule" => TriggerKind::Schedule,
        "child_terminal" => TriggerKind::ChildTerminal,
        "process_exit" => TriggerKind::ProcessExit,
        "file_changed" => TriggerKind::FileChanged,
        "repository_changed" => TriggerKind::RepositoryChanged,
        "channel_message" => TriggerKind::ChannelMessage,
        "external_condition" => TriggerKind::ExternalCondition,
        "user_response" => TriggerKind::UserResponse,
        "entity_mentioned" => TriggerKind::EntityMentioned,
        "loop_closed" => TriggerKind::LoopClosed,
        "prediction_resolved" => TriggerKind::PredictionResolved,
        _ => TriggerKind::BeliefChanged,
    }
}

pub(crate) fn mined_procedures(
    frames: &[(FrameHeader, EventEnvelope)],
) -> Result<Vec<ProcedureMined>, Error> {
    let by_lsn: BTreeMap<_, _> = frames
        .iter()
        .map(|frame| (frame.0.lsn.get(), frame))
        .collect();
    let mut groups: BTreeMap<Vec<u8>, (String, Vec<Episode>)> = BTreeMap::new();
    let mut roots = BTreeSet::new();
    for (closed_header, envelope) in frames {
        let EventPayload::LoopClosed(closed) = &envelope.payload else {
            continue;
        };
        let Some((opened_header, _)) = frames.iter().rev().find(|(header, envelope)| {
            header.lsn < closed_header.lsn && header.conversation == closed_header.conversation
                && matches!(&envelope.payload, EventPayload::LoopOpened(opened) if opened.loop_id == closed.loop_id)
        }) else { continue };
        for evidence_lsn in closed.evidence_lsns.as_deref().unwrap_or_default() {
            let Some((outcome_header, outcome_event)) = by_lsn.get(evidence_lsn).copied() else {
                continue;
            };
            let EventPayload::Outcome(outcome) = &outcome_event.payload else {
                continue;
            };
            if outcome_header.conversation != closed_header.conversation
                || !observed(outcome_event.authority)
            {
                continue;
            }
            let Some((effect_header, effect_event)) = frames.iter().rev().find(|(header, event)| {
                header.lsn < outcome_header.lsn && header.conversation == closed_header.conversation
                    && matches!(&event.payload, EventPayload::Effect(effect) if effect.effect_id == outcome.effect_id)
            }) else { continue };
            let EventPayload::Effect(effect) = &effect_event.payload else {
                continue;
            };
            let Some((call_header, call_event)) = by_lsn.get(&effect.tool_call_lsn).copied() else {
                continue;
            };
            let EventPayload::ToolCall(call) = &call_event.payload else {
                continue;
            };
            if call_header.lsn <= opened_header.lsn
                || call_header.conversation != closed_header.conversation
            {
                continue;
            }
            let Some((result_header, result_event)) = frames.iter().rev().find(|(header, event)| {
                header.lsn > call_header.lsn && header.lsn < effect_header.lsn
                    && header.conversation == closed_header.conversation
                    && matches!(&event.payload, EventPayload::ToolResult(result)
                        if result.tool_call_lsn == effect.tool_call_lsn && result.call_id == call.call_id)
            }) else { continue };
            let EventPayload::ToolResult(result) = &result_event.payload else {
                continue;
            };
            if !observed(result_event.authority)
                || !observed(effect_event.authority)
                || !outcome
                    .evidence_lsns
                    .as_deref()
                    .unwrap_or_default()
                    .contains(&result_header.lsn.get())
            {
                continue;
            }
            let mut root = blake3::Hasher::new();
            root.update(b"hypermind.procedure.episode.v1\0");
            root.update(call_header.conversation.as_bytes());
            root.update(&call_header.lsn.get().to_le_bytes());
            root.update(&call.call_id);
            let source_root = root.finalize().as_bytes().to_vec();
            if !roots.insert(source_root.clone()) {
                continue;
            }
            let mut identity = blake3::Hasher::new();
            identity.update(b"hypermind.procedure.strategy.v1\0");
            identity.update(&(call.tool_name.len() as u64).to_le_bytes());
            identity.update(call.tool_name.as_bytes());
            identity.update(&call.arguments);
            let id = identity.finalize().as_bytes().to_vec();
            let strategy = format!(
                "Run {} with {}; verify the tool result and observed effect outcome",
                call.tool_name,
                String::from_utf8_lossy(&call.arguments)
            );
            let episode = Episode {
                tool_call_lsn: call_header.lsn.get(),
                tool_call_id: call.call_id.clone(),
                tool_result_lsn: result_header.lsn.get(),
                tool_result_call_id: result.call_id.clone(),
                effect_lsn: effect_header.lsn.get(),
                effect_tool_call_lsn: effect.tool_call_lsn,
                effect_id: effect.effect_id.clone(),
                outcome_lsn: outcome_header.lsn.get(),
                outcome_effect_id: outcome.effect_id.clone(),
                loop_closed_lsn: closed_header.lsn.get(),
                loop_evidence_lsns: closed.evidence_lsns.clone().unwrap_or_default(),
                source_root,
                conversation: call_header.conversation.as_bytes().to_vec(),
                successful: closed.reason == LoopCloseReason::Done
                    && result.status == ResultStatus::Ok
                    && outcome.status == ResultStatus::Ok,
            };
            groups
                .entry(id)
                .or_insert_with(|| (strategy, Vec::new()))
                .1
                .push(episode);
        }
    }
    groups.into_iter().map(|(id, (strategy, episodes))| procedures::mine(
        &id, &strategy, vec!["Observed successful tool result and effect outcome".into()],
        vec!["Use the recorded tool arguments and verify external evidence before closing the loop".into()], &episodes,
    )).collect()
}

fn observed(authority: Authority) -> bool {
    matches!(
        authority,
        Authority::ToolObserved | Authority::ExternalObserved | Authority::RuntimeFact
    )
}
