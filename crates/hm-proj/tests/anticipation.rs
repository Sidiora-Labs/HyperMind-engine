#![forbid(unsafe_code)]
#![allow(clippy::too_many_lines)]

use hm_core::{ActorId, ConversationId, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_proj::attention::{AttentionProjection, MAXIMUM_ATTENTION_HISTORY};
use hm_proj::attestations::AttestationsProjection;
use hm_proj::intentions::{IntentionStatus, IntentionsProjection, TriggerKind};
use hm_proj::predictions::PredictionsProjection;
use hm_proj::procedures::{ProcedureState, ProceduresProjection};
use hm_proj::rebuild::rebuild_projection_stream;
use hm_proj::store::{ProjectionId, ProjectionStore};
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    AttentionDecided, AttentionDecision, Attestation, AttestationDisposition, Authority,
    EventEnvelope, EventPayload, ExpectedPredicate, IntentionCancelled, IntentionFired,
    IntentionSet, OutcomeAssessment, OutcomeObserved, PredicateKind, Predicted, ProcedureAdopted,
    ProcedureMined, ProcedureRevised, ProcedureSupport, Retention, Sensitivity, UserMsg, WakeAt,
    WakeBeliefChanged, WakeChannelMessage, WakeChildTerminal, WakeEntityMentioned,
    WakeExternalCondition, WakeFileChanged, WakeLoopClosed, WakePredictionResolved,
    WakeProcessExit, WakeRepositoryChanged, WakeSchedule, WakeTrigger, WakeUserResponse,
};

#[test]
fn anticipation_events_rebuild_into_bounded_indexed_state() {
    let mut frames = Vec::new();
    for (index, trigger) in wake_triggers().into_iter().enumerate() {
        push(
            &mut frames,
            EventKind::IntentionSet,
            EventPayload::IntentionSet(Box::new(IntentionSet {
                intention_id: format!("intention-{index}").into_bytes(),
                objective: format!("objective-{index}").into_bytes(),
                trigger: Some(trigger),
                expires_at_ns: 9_000,
                reply_route: "channel:test".to_owned(),
            })),
            Authority::UserAsserted,
        );
    }
    let trigger_lsn = push(
        &mut frames,
        EventKind::UserMsg,
        EventPayload::UserMsg(Box::new(UserMsg {
            content: b"repository changed".to_vec(),
        })),
        Authority::RuntimeFact,
    );
    push(
        &mut frames,
        EventKind::Attestation,
        EventPayload::Attestation(Box::new(Attestation {
            target_lsn: trigger_lsn,
            disposition: AttestationDisposition::Helpful,
        })),
        Authority::RuntimeFact,
    );
    let fired_lsn = push(
        &mut frames,
        EventKind::IntentionFired,
        EventPayload::IntentionFired(Box::new(IntentionFired {
            intention_id: b"intention-0".to_vec(),
            wake_id: b"stable-wake".to_vec(),
            trigger_lsn,
        })),
        Authority::RuntimeFact,
    );
    push(
        &mut frames,
        EventKind::IntentionFired,
        EventPayload::IntentionFired(Box::new(IntentionFired {
            intention_id: b"intention-0".to_vec(),
            wake_id: b"stable-wake".to_vec(),
            trigger_lsn,
        })),
        Authority::RuntimeFact,
    );
    push(
        &mut frames,
        EventKind::IntentionCancelled,
        EventPayload::IntentionCancelled(Box::new(IntentionCancelled {
            intention_id: b"intention-1".to_vec(),
            reason: "superseded".to_owned(),
        })),
        Authority::UserAsserted,
    );

    for index in 0..3 {
        let prediction_id = format!("prediction-{index}").into_bytes();
        push(
            &mut frames,
            EventKind::Predicted,
            EventPayload::Predicted(Box::new(Predicted {
                prediction_id: prediction_id.clone(),
                revision: 1,
                task_id: Some(b"task-7".to_vec()),
                attempt_id: Some(format!("attempt-{index}").into_bytes()),
                operation_id: Some(b"deploy".to_vec()),
                mechanism: "repository-deploy".to_owned(),
                predicates: vec![ExpectedPredicate {
                    kind: PredicateKind::DigestEquals,
                    scope: "repository".to_owned(),
                    property: Some("HEAD".to_owned()),
                    expected: Some(b"expected-digest".to_vec()),
                }],
                deadline_ns: 10_000,
                uncertainty: "remote update may race".to_owned(),
            })),
            Authority::AssistantGenerated,
        );
        let observation_lsn = push(
            &mut frames,
            EventKind::UserMsg,
            EventPayload::UserMsg(Box::new(UserMsg {
                content: format!("observed digest mismatch {index}").into_bytes(),
            })),
            Authority::RuntimeFact,
        );
        push(
            &mut frames,
            EventKind::OutcomeObserved,
            EventPayload::OutcomeObserved(Box::new(OutcomeObserved {
                prediction_id,
                revision: 1,
                assessment: OutcomeAssessment::Contradicted,
                observation_lsns: vec![observation_lsn],
                evaluator_version: "deterministic-v1".to_owned(),
            })),
            Authority::RuntimeFact,
        );
    }

    let mined_lsn = push(
        &mut frames,
        EventKind::ProcedureMined,
        EventPayload::ProcedureMined(Box::new(ProcedureMined {
            procedure_id: b"procedure-1".to_vec(),
            strategy: "check then deploy".to_owned(),
            expected_outcomes: vec!["deployment committed".to_owned()],
            preconditions: vec!["clean repository".to_owned()],
            supports: vec![support(1, 1, 1)],
            failures: None,
            counterexamples: None,
        })),
        Authority::DerivedInference,
    );
    let revised_lsn = push(
        &mut frames,
        EventKind::ProcedureRevised,
        EventPayload::ProcedureRevised(Box::new(ProcedureRevised {
            procedure_id: b"procedure-1".to_vec(),
            previous_lsn: mined_lsn,
            strategy: "check, deploy, then verify".to_owned(),
            expected_outcomes: vec!["deployment committed".to_owned()],
            preconditions: vec!["clean repository".to_owned()],
            supports: vec![support(1, 1, 1), support(2, 2, 2), support(3, 1, 3)],
            failures: Some(vec![4]),
            counterexamples: Some(vec![5]),
        })),
        Authority::DerivedInference,
    );
    push(
        &mut frames,
        EventKind::ProcedureAdopted,
        EventPayload::ProcedureAdopted(Box::new(ProcedureAdopted {
            procedure_id: b"procedure-1".to_vec(),
            procedure_lsn: revised_lsn,
        })),
        Authority::UserAsserted,
    );

    for index in 0..=MAXIMUM_ATTENTION_HISTORY {
        push(
            &mut frames,
            EventKind::AttentionDecided,
            EventPayload::AttentionDecided(Box::new(AttentionDecided {
                intention_id: b"intention-0".to_vec(),
                wake_id: b"stable-wake".to_vec(),
                decision: AttentionDecision::Batch,
                reason: format!("quiet-hours-{index}"),
            })),
            Authority::RuntimeFact,
        );
    }

    let first_root = tempfile::tempdir().unwrap();
    let first = ProjectionStore::open(first_root.path(), 64 * 1024 * 1024).unwrap();
    rebuild_projection_stream(&first, &frames, false, usize::MAX).unwrap();
    assert_state(&first, fired_lsn);

    let second_root = tempfile::tempdir().unwrap();
    let second = ProjectionStore::open(second_root.path(), 64 * 1024 * 1024).unwrap();
    let split = frames.len() / 2;
    rebuild_projection_stream(&second, &frames, false, split).unwrap();
    rebuild_projection_stream(&second, &frames, false, usize::MAX).unwrap();
    for projection in [
        ProjectionId::Intentions,
        ProjectionId::AttentionHistory,
        ProjectionId::Predictions,
        ProjectionId::Procedures,
        ProjectionId::Attestations,
    ] {
        assert_eq!(
            first
                .begin_snapshot()
                .unwrap()
                .canonical_dump(projection)
                .unwrap(),
            second
                .begin_snapshot()
                .unwrap()
                .canonical_dump(projection)
                .unwrap(),
        );
    }
}

fn assert_state(store: &ProjectionStore, fired_lsn: u64) {
    let snapshot = store.begin_snapshot().unwrap();
    let intention = IntentionsProjection::get(&snapshot, b"intention-0")
        .unwrap()
        .unwrap();
    assert_eq!(intention.status, IntentionStatus::Fired);
    assert_eq!(intention.status_lsn, fired_lsn);
    assert_eq!(
        intention.wake_id.as_deref(),
        Some(b"stable-wake".as_slice())
    );
    let file_intentions =
        IntentionsProjection::pending_by_trigger(&snapshot, TriggerKind::FileChanged, 10).unwrap();
    assert_eq!(file_intentions.len(), 1);

    let attention = AttentionProjection::recent(&snapshot, MAXIMUM_ATTENTION_HISTORY).unwrap();
    assert_eq!(attention.len(), MAXIMUM_ATTENTION_HISTORY);
    assert_eq!(attention[0].reason, "quiet-hours-256");

    let calibration =
        PredictionsProjection::calibration(&snapshot, PredicateKind::DigestEquals).unwrap();
    assert_eq!(calibration.contradicted, 3);
    let failures =
        PredictionsProjection::mechanism_failures(&snapshot, "repository-deploy").unwrap();
    assert_eq!(failures.consecutive, 3);
    assert!(failures.revision_required);

    let procedure = ProceduresProjection::get(&snapshot, b"procedure-1")
        .unwrap()
        .unwrap();
    assert_eq!(procedure.state, ProcedureState::Adopted);
    assert_eq!(procedure.supports.len(), 3);
    assert_eq!(procedure.counterexamples, vec![5]);
    let attestation = AttestationsProjection::get(&snapshot, LSN::new(14))
        .unwrap()
        .unwrap();
    assert_eq!(attestation.helpful, 1);
}

fn wake_triggers() -> Vec<WakeTrigger> {
    vec![
        WakeTrigger::WakeAt(Box::new(WakeAt { at_ns: 100 })),
        WakeTrigger::WakeSchedule(Box::new(WakeSchedule {
            schedule: "0 9 * * *".to_owned(),
        })),
        WakeTrigger::WakeChildTerminal(Box::new(WakeChildTerminal {
            child_id: b"child".to_vec(),
        })),
        WakeTrigger::WakeProcessExit(Box::new(WakeProcessExit {
            process_id: b"process".to_vec(),
        })),
        WakeTrigger::WakeFileChanged(Box::new(WakeFileChanged {
            path: "src/lib.rs".to_owned(),
        })),
        WakeTrigger::WakeRepositoryChanged(Box::new(WakeRepositoryChanged {
            repository: "hypermind".to_owned(),
        })),
        WakeTrigger::WakeChannelMessage(Box::new(WakeChannelMessage {
            channel: "deployments".to_owned(),
        })),
        WakeTrigger::WakeExternalCondition(Box::new(WakeExternalCondition {
            condition: "service healthy".to_owned(),
        })),
        WakeTrigger::WakeUserResponse(Box::new(WakeUserResponse {
            reply_to: b"question".to_vec(),
        })),
        WakeTrigger::WakeEntityMentioned(Box::new(WakeEntityMentioned {
            entity_id: b"orchid".to_vec(),
        })),
        WakeTrigger::WakeLoopClosed(Box::new(WakeLoopClosed {
            loop_id: b"loop".to_vec(),
        })),
        WakeTrigger::WakePredictionResolved(Box::new(WakePredictionResolved {
            prediction_id: b"prediction".to_vec(),
        })),
        WakeTrigger::WakeBeliefChanged(Box::new(WakeBeliefChanged {
            canonical_identity: "deployment:region".to_owned(),
        })),
    ]
}

fn support(root: u8, conversation: u8, episode_lsn: u64) -> ProcedureSupport {
    ProcedureSupport {
        source_root: vec![root; 32],
        conversation: vec![conversation; 16],
        episode_lsn,
    }
}

fn push(
    frames: &mut Vec<Frame>,
    kind: EventKind,
    payload: EventPayload,
    authority: Authority,
) -> u64 {
    let lsn = u64::try_from(frames.len()).unwrap() + 1;
    frames.push(Frame {
        header: FrameHeader {
            lsn: LSN::new(lsn),
            kind,
            wall_timestamp_ns: UtcNanos::new(i64::try_from(lsn).unwrap()),
            actor: ActorId::new(7),
            conversation: ConversationId::new([u8::try_from(lsn % 2 + 1).unwrap(); 16]),
        },
        sealed_payload: encode_event_envelope(&EventEnvelope {
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
            retention: Retention::Durable,
            sensitivity: Sensitivity::Personal,
            event_time_ns: i64::try_from(lsn).unwrap(),
        }),
    });
    lsn
}
