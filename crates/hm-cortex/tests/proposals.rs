#![forbid(unsafe_code)]

use hm_core::{ErrorCode, LSN};
use hm_cortex::procedures::{ImprovementDraft, ProcedureHead, propose_improvement};
use hm_schema::event::{
    Boundary, EventHistory, EventKind, HistorySource, encode_event_envelope,
    verify_event_with_history,
};
use hm_schema::events::{Authority, EventEnvelope, EventPayload, Retention, Sensitivity};

const PROPOSAL_ID: &[u8] = b"proposal-restart-ingest-2";
const PROCEDURE_ID: &[u8] = b"procedure-restart-ingest";
const FAILURES: [u64; 3] = [11, 19, 23];

struct ObservedFailures;

impl EventHistory for ObservedFailures {
    fn kind_at(&self, lsn: LSN) -> Option<EventKind> {
        FAILURES
            .contains(&lsn.get())
            .then_some(EventKind::ToolResult)
    }

    fn authority_at(&self, lsn: LSN) -> Option<Authority> {
        FAILURES
            .contains(&lsn.get())
            .then_some(Authority::ToolObserved)
    }

    fn source_at(&self, _: LSN) -> HistorySource {
        HistorySource::LedgerEvent
    }
}

fn head() -> ProcedureHead {
    ProcedureHead {
        procedure_id: PROCEDURE_ID.to_vec(),
        version_lsn: 41,
        strategy: "restart the worker".to_owned(),
        expected_outcomes: vec!["queue depth returns to zero".to_owned()],
        preconditions: vec!["the worker is unresponsive".to_owned()],
    }
}

fn draft() -> ImprovementDraft {
    ImprovementDraft {
        strategy: "drain the queue, then restart the worker".to_owned(),
        expected_outcomes: vec![
            "queue depth returns to zero".to_owned(),
            "no message is replayed twice".to_owned(),
        ],
        preconditions: vec![
            "the worker is unresponsive".to_owned(),
            "the lease is free".to_owned(),
        ],
        rationale: "three restarts replayed in-flight messages because the queue was never drained"
            .to_owned(),
    }
}

fn rejected_failure_lists() {
    for failures in [Vec::new(), vec![0], vec![0, 19], vec![11, 0, 23]] {
        assert_eq!(
            propose_improvement(PROPOSAL_ID, &head(), draft(), &failures)
                .unwrap_err()
                .code,
            ErrorCode::InvalidArgument
        );
    }
    for failures in [vec![23, 19, 11], vec![11, 23, 19], vec![11, 11, 23]] {
        assert_eq!(
            propose_improvement(PROPOSAL_ID, &head(), draft(), &failures)
                .unwrap_err()
                .code,
            ErrorCode::OrderingViolation
        );
    }
}

fn rejected_heads() {
    let mut unversioned = head();
    unversioned.version_lsn = 0;
    let mut anonymous = head();
    anonymous.procedure_id = Vec::new();
    for broken in [unversioned, anonymous] {
        assert_eq!(
            propose_improvement(PROPOSAL_ID, &broken, draft(), &FAILURES)
                .unwrap_err()
                .code,
            ErrorCode::InvalidArgument
        );
    }
    assert_eq!(
        propose_improvement(b"", &head(), draft(), &FAILURES)
            .unwrap_err()
            .code,
        ErrorCode::InvalidArgument
    );
}

fn rejected_drafts() {
    let mut strategyless = draft();
    strategyless.strategy = String::new();
    let mut unexplained = draft();
    unexplained.rationale = String::new();
    let mut outcomeless = draft();
    outcomeless.expected_outcomes = Vec::new();
    let mut blank_outcome = draft();
    blank_outcome.expected_outcomes = vec![String::new()];
    let mut blank_precondition = draft();
    blank_precondition.preconditions = vec![String::new()];
    let no_op = ImprovementDraft {
        strategy: head().strategy,
        expected_outcomes: head().expected_outcomes,
        preconditions: head().preconditions,
        rationale: draft().rationale,
    };
    for broken in [
        strategyless,
        unexplained,
        outcomeless,
        blank_outcome,
        blank_precondition,
        no_op,
    ] {
        assert_eq!(
            propose_improvement(PROPOSAL_ID, &head(), broken, &FAILURES)
                .unwrap_err()
                .code,
            ErrorCode::InvalidArgument
        );
    }
}

#[test]
fn improvement_proposals_link_failures_to_a_fresh_base_version() {
    let head = head();
    let proposed = propose_improvement(PROPOSAL_ID, &head, draft(), &FAILURES).unwrap();

    assert_eq!(proposed.proposal_id, PROPOSAL_ID.to_vec());
    assert_eq!(proposed.procedure_id, head.procedure_id);
    assert_eq!(proposed.base_lsn, head.version_lsn);
    assert_eq!(proposed.strategy, draft().strategy);
    assert_eq!(proposed.expected_outcomes, draft().expected_outcomes);
    assert_eq!(proposed.preconditions, draft().preconditions);
    assert_eq!(proposed.rationale, draft().rationale);
    assert_eq!(proposed.failure_lsns, FAILURES.to_vec());

    rejected_drafts();
    rejected_failure_lists();
    rejected_heads();

    let verified = verify_event_with_history(
        &encode_event_envelope(&EventEnvelope {
            schema_version: 2,
            payload: EventPayload::ProcedureImprovementProposed(Box::new(proposed)),
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 0,
            origin_actor: 7,
            run_id: None,
            model_provenance: None,
            authority: Authority::DerivedInference,
            retention: Retention::Durable,
            sensitivity: Sensitivity::Personal,
            event_time_ns: 0,
        }),
        EventKind::ProcedureImprovementProposed,
        Boundary::Socket,
        &ObservedFailures,
    )
    .unwrap();
    assert_eq!(verified.kind, EventKind::ProcedureImprovementProposed);
}
