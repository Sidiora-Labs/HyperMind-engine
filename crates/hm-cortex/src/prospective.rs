#![allow(clippy::missing_errors_doc)]

use hm_schema::events::{AttentionDecision, IntentionFired, IntentionSet, WakeAt, WakeTrigger};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WakeSignal {
    pub trigger_lsn: u64,
    pub now_ns: i64,
    pub kind: &'static str,
    pub key: Vec<u8>,
}

#[must_use]
pub fn evaluate(intention: &IntentionSet, signal: &WakeSignal) -> Option<IntentionFired> {
    if signal.trigger_lsn == 0
        || signal.now_ns > intention.expires_at_ns
        || !matches_signal(intention.trigger.as_ref()?, signal)
    {
        return None;
    }
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"hypermind.wake.v1\0");
    hasher.update(&intention.intention_id);
    hasher.update(signal.kind.as_bytes());
    hasher.update(&signal.key);
    Some(IntentionFired {
        intention_id: intention.intention_id.clone(),
        wake_id: hasher.finalize().as_bytes().to_vec(),
        trigger_lsn: signal.trigger_lsn,
    })
}

#[must_use]
pub fn rearm(
    intention: &IntentionSet,
    decision: AttentionDecision,
    next_at_ns: i64,
) -> Option<IntentionSet> {
    (decision == AttentionDecision::Schedule
        && next_at_ns > 0
        && next_at_ns < intention.expires_at_ns)
        .then(|| IntentionSet {
            trigger: Some(WakeTrigger::WakeAt(Box::new(WakeAt { at_ns: next_at_ns }))),
            ..intention.clone()
        })
}

fn matches_signal(trigger: &WakeTrigger, signal: &WakeSignal) -> bool {
    match trigger {
        WakeTrigger::WakeAt(value) => signal.kind == "time" && signal.now_ns >= value.at_ns,
        WakeTrigger::WakeSchedule(value) => {
            signal.kind == "schedule" && signal.key == value.schedule.as_bytes()
        }
        WakeTrigger::WakeChildTerminal(value) => {
            match_key(signal, "child_terminal", &value.child_id)
        }
        WakeTrigger::WakeProcessExit(value) => match_key(signal, "process_exit", &value.process_id),
        WakeTrigger::WakeFileChanged(value) => {
            match_key(signal, "file_changed", value.path.as_bytes())
        }
        WakeTrigger::WakeRepositoryChanged(value) => {
            match_key(signal, "repository_changed", value.repository.as_bytes())
        }
        WakeTrigger::WakeChannelMessage(value) => {
            match_key(signal, "channel_message", value.channel.as_bytes())
        }
        WakeTrigger::WakeExternalCondition(value) => {
            match_key(signal, "external_condition", value.condition.as_bytes())
        }
        WakeTrigger::WakeUserResponse(value) => match_key(signal, "user_response", &value.reply_to),
        WakeTrigger::WakeEntityMentioned(value) => {
            match_key(signal, "entity_mentioned", &value.entity_id)
        }
        WakeTrigger::WakeLoopClosed(value) => match_key(signal, "loop_closed", &value.loop_id),
        WakeTrigger::WakePredictionResolved(value) => {
            match_key(signal, "prediction_resolved", &value.prediction_id)
        }
        WakeTrigger::WakeBeliefChanged(value) => match_key(
            signal,
            "belief_changed",
            value.canonical_identity.as_bytes(),
        ),
    }
}

fn match_key(signal: &WakeSignal, kind: &str, key: &[u8]) -> bool {
    signal.kind == kind && signal.key == key
}
