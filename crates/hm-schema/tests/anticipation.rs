#![forbid(unsafe_code)]

use hm_core::{ErrorCode, LSN};
use hm_schema::event::{
    Boundary, EventHistory, EventKind, HistorySource, MAXIMUM_PLAYBOOK_BYTES,
    encode_event_envelope, verify_event, verify_event_with_history,
};
use hm_schema::events::{
    Authority, EventEnvelope, EventPayload, ExpectedPredicate, IntentionSet, OutcomeAssessment,
    OutcomeObserved, PredicateKind, Predicted, ProcedureAdopted, ProcedureImported,
    ProcedureImprovementProposed, Retention, Sensitivity, WakeAt, WakeBeliefChanged,
    WakeChannelMessage, WakeChildTerminal, WakeEntityMentioned, WakeExternalCondition,
    WakeFileChanged, WakeLoopClosed, WakePredictionResolved, WakeProcessExit,
    WakeRepositoryChanged, WakeSchedule, WakeTrigger, WakeUserResponse,
};

#[test]
fn accepts_the_complete_wake_trigger_vocabulary() {
    for (index, trigger) in wake_triggers().into_iter().enumerate() {
        let payload = EventPayload::IntentionSet(Box::new(IntentionSet {
            intention_id: format!("intention-{index}").into_bytes(),
            objective: b"wait for condition".to_vec(),
            trigger: Some(trigger),
            expires_at_ns: 1_000,
            reply_route: "channel:test".to_owned(),
        }));
        verify_event(
            &encode_event_envelope(&envelope(payload, Authority::UserAsserted)),
            EventKind::IntentionSet,
            Boundary::Socket,
        )
        .unwrap();
    }
}

#[test]
fn bounds_predictions_requires_observations_and_protects_adoption() {
    let predicates = (0..17)
        .map(|_| ExpectedPredicate {
            kind: PredicateKind::ObjectExists,
            scope: "repository".to_owned(),
            property: None,
            expected: None,
        })
        .collect();
    let invalid = EventPayload::Predicted(Box::new(Predicted {
        prediction_id: b"prediction".to_vec(),
        revision: 1,
        task_id: None,
        attempt_id: None,
        operation_id: None,
        mechanism: "deploy".to_owned(),
        predicates,
        deadline_ns: 1_000,
        uncertainty: "remote state".to_owned(),
    }));
    let error = verify_event(
        &encode_event_envelope(&envelope(invalid, Authority::AssistantGenerated)),
        EventKind::Predicted,
        Boundary::Socket,
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::SchemaInvalid);

    let outcome = EventPayload::OutcomeObserved(Box::new(OutcomeObserved {
        prediction_id: b"prediction".to_vec(),
        revision: 1,
        assessment: OutcomeAssessment::Supported,
        observation_lsns: vec![7],
        evaluator_version: "deterministic-v1".to_owned(),
    }));
    verify_event_with_history(
        &encode_event_envelope(&envelope(outcome, Authority::RuntimeFact)),
        EventKind::OutcomeObserved,
        Boundary::Disk,
        &ObservedHistory,
    )
    .unwrap();

    let adopted = EventPayload::ProcedureAdopted(Box::new(ProcedureAdopted {
        procedure_id: b"procedure".to_vec(),
        procedure_lsn: 9,
    }));
    let error = verify_event(
        &encode_event_envelope(&envelope(adopted, Authority::DerivedInference)),
        EventKind::ProcedureAdopted,
        Boundary::Socket,
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::ProtectedTypeWrite);
}

#[test]
fn playbook_imports_and_improvement_proposals_are_bounded_and_authority_gated() {
    verify_event(
        &encode_event_envelope(&envelope(
            EventPayload::ProcedureImported(Box::new(imported_playbook())),
            Authority::ExternalObserved,
        )),
        EventKind::ProcedureImported,
        Boundary::Socket,
    )
    .unwrap();

    assert_eq!(
        import_error(imported_playbook(), Authority::UserAsserted),
        ErrorCode::ProtectedTypeWrite
    );

    let mut oversize = imported_playbook();
    oversize.instructions = vec![b'x'; MAXIMUM_PLAYBOOK_BYTES + 1];
    let mut empty_body = imported_playbook();
    empty_body.instructions = Vec::new();
    let mut short_digest = imported_playbook();
    short_digest.source_digest = vec![9; 31];
    for invalid in [oversize, empty_body, short_digest] {
        assert_eq!(
            import_error(invalid, Authority::ExternalObserved),
            ErrorCode::SchemaInvalid
        );
    }

    verify_event_with_history(
        &encode_event_envelope(&envelope(
            EventPayload::ProcedureImprovementProposed(Box::new(improvement_proposal(vec![7], 5))),
            Authority::DerivedInference,
        )),
        EventKind::ProcedureImprovementProposed,
        Boundary::Socket,
        &ObservedHistory,
    )
    .unwrap();

    assert_eq!(
        proposal_error(
            improvement_proposal(vec![8], 5),
            Authority::DerivedInference
        ),
        ErrorCode::CitationInvalid
    );
    for invalid in [
        improvement_proposal(vec![7, 7], 5),
        improvement_proposal(vec![0], 5),
        improvement_proposal(Vec::new(), 5),
        improvement_proposal((1..=257).collect(), 5),
        improvement_proposal(vec![7], 0),
    ] {
        assert_eq!(
            proposal_error(invalid, Authority::DerivedInference),
            ErrorCode::SchemaInvalid
        );
    }
    assert_eq!(
        proposal_error(
            improvement_proposal(vec![7], 5),
            Authority::ExternalObserved
        ),
        ErrorCode::ProtectedTypeWrite
    );
}

fn imported_playbook() -> ProcedureImported {
    ProcedureImported {
        procedure_id: b"procedure".to_vec(),
        name: "restart the ingest worker".to_owned(),
        strategy: "drain the queue, then restart".to_owned(),
        expected_outcomes: vec!["queue depth returns to zero".to_owned()],
        preconditions: vec!["the worker is unresponsive".to_owned()],
        instructions: b"1. drain\n2. restart\n".to_vec(),
        declared_tools: vec!["shell".to_owned()],
        source_uri: "file:///playbooks/restart-ingest".to_owned(),
        source_digest: vec![9; 32],
        playbook_version: 1,
    }
}

fn improvement_proposal(failure_lsns: Vec<u64>, base_lsn: u64) -> ProcedureImprovementProposed {
    ProcedureImprovementProposed {
        proposal_id: b"proposal".to_vec(),
        procedure_id: b"procedure".to_vec(),
        base_lsn,
        strategy: "drain the queue, wait for the lease, then restart".to_owned(),
        expected_outcomes: vec!["queue depth returns to zero".to_owned()],
        preconditions: vec!["the worker is unresponsive".to_owned()],
        rationale: "the restart raced the lease and the queue refilled".to_owned(),
        failure_lsns,
    }
}

fn import_error(value: ProcedureImported, authority: Authority) -> ErrorCode {
    verify_event(
        &encode_event_envelope(&envelope(
            EventPayload::ProcedureImported(Box::new(value)),
            authority,
        )),
        EventKind::ProcedureImported,
        Boundary::Socket,
    )
    .expect_err("the import must fail closed")
    .code
}

fn proposal_error(value: ProcedureImprovementProposed, authority: Authority) -> ErrorCode {
    verify_event_with_history(
        &encode_event_envelope(&envelope(
            EventPayload::ProcedureImprovementProposed(Box::new(value)),
            authority,
        )),
        EventKind::ProcedureImprovementProposed,
        Boundary::Socket,
        &ObservedHistory,
    )
    .expect_err("the proposal must fail closed")
    .code
}

struct ObservedHistory;

impl EventHistory for ObservedHistory {
    fn kind_at(&self, lsn: LSN) -> Option<EventKind> {
        (lsn == LSN::new(7)).then_some(EventKind::ToolResult)
    }

    fn authority_at(&self, lsn: LSN) -> Option<Authority> {
        (lsn == LSN::new(7)).then_some(Authority::ToolObserved)
    }

    fn source_at(&self, _: LSN) -> HistorySource {
        HistorySource::LedgerEvent
    }
}

fn envelope(payload: EventPayload, authority: Authority) -> EventEnvelope {
    EventEnvelope {
        schema_version: 2,
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
        event_time_ns: 0,
    }
}

fn wake_triggers() -> Vec<WakeTrigger> {
    vec![
        WakeTrigger::WakeAt(Box::new(WakeAt { at_ns: 1 })),
        WakeTrigger::WakeSchedule(Box::new(WakeSchedule {
            schedule: "daily".to_owned(),
        })),
        WakeTrigger::WakeChildTerminal(Box::new(WakeChildTerminal {
            child_id: b"child".to_vec(),
        })),
        WakeTrigger::WakeProcessExit(Box::new(WakeProcessExit {
            process_id: b"process".to_vec(),
        })),
        WakeTrigger::WakeFileChanged(Box::new(WakeFileChanged {
            path: "file".to_owned(),
        })),
        WakeTrigger::WakeRepositoryChanged(Box::new(WakeRepositoryChanged {
            repository: "repo".to_owned(),
        })),
        WakeTrigger::WakeChannelMessage(Box::new(WakeChannelMessage {
            channel: "channel".to_owned(),
        })),
        WakeTrigger::WakeExternalCondition(Box::new(WakeExternalCondition {
            condition: "healthy".to_owned(),
        })),
        WakeTrigger::WakeUserResponse(Box::new(WakeUserResponse {
            reply_to: b"reply".to_vec(),
        })),
        WakeTrigger::WakeEntityMentioned(Box::new(WakeEntityMentioned {
            entity_id: b"entity".to_vec(),
        })),
        WakeTrigger::WakeLoopClosed(Box::new(WakeLoopClosed {
            loop_id: b"loop".to_vec(),
        })),
        WakeTrigger::WakePredictionResolved(Box::new(WakePredictionResolved {
            prediction_id: b"prediction".to_vec(),
        })),
        WakeTrigger::WakeBeliefChanged(Box::new(WakeBeliefChanged {
            canonical_identity: "belief".to_owned(),
        })),
    ]
}
