use hm_core::{ErrorCode, LSN};
use hm_ledger::gate::{BeliefEvidence, LiveBelief, admit_batch, apply_batch};
use hm_schema::events::{
    Assertion, AssertionClaim, Authority, BeliefType, Consolidation, EventEnvelope, EventPayload,
    ProvenanceRange, ResultStatus, Retention, Retract, Sensitivity, ToolResult,
};

#[derive(Default)]
struct Evidence {
    successful_tool_observed_results: Vec<u64>,
}

impl BeliefEvidence for Evidence {
    fn has_successful_tool_observed_result(&self, first_lsn: LSN, last_lsn: LSN) -> bool {
        self.successful_tool_observed_results
            .iter()
            .any(|lsn| first_lsn.get() <= *lsn && *lsn <= last_lsn.get())
    }
}

fn provenance(first_lsn: u64, last_lsn: u64) -> Vec<ProvenanceRange> {
    vec![ProvenanceRange {
        first_lsn,
        last_lsn,
        byte_start: 4,
        byte_end: 12,
    }]
}

fn assertion(
    id: u8,
    identity: &str,
    domain: &str,
    claim: AssertionClaim,
    provenance_lsn: u64,
) -> Assertion {
    Assertion {
        belief_id: vec![id],
        belief_type: BeliefType::Fact,
        canonical_identity: identity.to_owned(),
        value: identity.as_bytes().to_vec(),
        valid_from_ns: 0,
        valid_to_ns: 0,
        provenance: provenance(provenance_lsn, provenance_lsn),
        conflict_domain: Some(domain.to_owned()),
        claim,
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
        origin_actor: 0,
        run_id: None,
        model_provenance: None,
        authority,
        retention: Retention::CurrentState,
        sensitivity: Sensitivity::Public,
        event_time_ns: 10,
    }
}

fn assertion_event(assertion: Assertion) -> EventEnvelope {
    envelope(
        EventPayload::Assertion(Box::new(assertion)),
        Authority::UserAsserted,
    )
}

fn tool_result(status: ResultStatus, authority: Authority) -> EventEnvelope {
    envelope(
        EventPayload::ToolResult(Box::new(ToolResult {
            call_id: b"call-1".to_vec(),
            tool_call_lsn: 1,
            status,
            result: b"observed".to_vec(),
        })),
        authority,
    )
}

#[test]
fn negative_existence_requires_successful_tool_observed_evidence() {
    let negative = assertion_event(assertion(
        1,
        "service:moltbook:status:absent",
        "service:moltbook:status",
        AssertionClaim::NegativeExistence,
        2,
    ));

    let rejection = admit_batch(
        LSN::new(3),
        std::slice::from_ref(&negative),
        &Evidence::default(),
        &[],
    )
    .expect_err("uncorroborated negative existence must fail before commit");
    assert_eq!(rejection.event_index, 0);
    assert_eq!(
        rejection.error.code,
        ErrorCode::NegativeExistenceUncorroborated
    );
    assert_eq!(rejection.error.lsn, LSN::new(3));

    let failed = [
        tool_result(ResultStatus::Error, Authority::ToolObserved),
        negative.clone(),
    ];
    let rejection = admit_batch(LSN::new(2), &failed, &Evidence::default(), &[])
        .expect_err("a failed tool result must not corroborate absence");
    assert_eq!(
        rejection.error.code,
        ErrorCode::NegativeExistenceUncorroborated
    );

    let assistant = [
        tool_result(ResultStatus::Ok, Authority::AssistantGenerated),
        negative.clone(),
    ];
    let rejection = admit_batch(LSN::new(2), &assistant, &Evidence::default(), &[])
        .expect_err("assistant output must not corroborate absence");
    assert_eq!(
        rejection.error.code,
        ErrorCode::NegativeExistenceUncorroborated
    );

    let observed = [
        tool_result(ResultStatus::Ok, Authority::ToolObserved),
        negative,
    ];
    admit_batch(LSN::new(2), &observed, &Evidence::default(), &[])
        .expect("prior successful tool result in the socket batch corroborates the claim");

    let historical = Evidence {
        successful_tool_observed_results: vec![2],
    };
    let historical_negative = assertion_event(assertion(
        1,
        "service:moltbook:status:absent",
        "service:moltbook:status",
        AssertionClaim::NegativeExistence,
        2,
    ));
    admit_batch(LSN::new(3), &[historical_negative], &historical, &[])
        .expect("successful historical tool evidence corroborates the claim");
}

#[test]
fn same_type_domain_and_different_identity_is_listed_before_commit() {
    let live = LiveBelief {
        belief_id: vec![6],
        belief_type: BeliefType::Fact,
        canonical_identity: "deployment:region:europe".to_owned(),
        conflict_domain: "deployment:region".to_owned(),
    };
    let incoming = assertion_event(assertion(
        7,
        "deployment:region:america",
        "deployment:region",
        AssertionClaim::Affirmative,
        1,
    ));
    let admitted = admit_batch(LSN::new(2), &[incoming], &Evidence::default(), &[live])
        .expect("conflicts are surfaced rather than silently suppressing the assertion");
    assert_eq!(admitted.conflicts.len(), 1);
    assert_eq!(admitted.conflicts[0].event_index, 0);
    assert_eq!(
        admitted.conflicts[0].existing_identity,
        "deployment:region:europe"
    );
    assert_eq!(
        admitted.conflicts[0].incoming_identity,
        "deployment:region:america"
    );
}

#[test]
fn mixed_consolidation_is_an_atomic_deterministic_skip() {
    let valid = assertion(
        8,
        "consolidation:valid-sibling",
        "consolidation",
        AssertionClaim::Affirmative,
        1,
    );
    let invalid = assertion(
        9,
        "service:moltbook:status:absent",
        "service:moltbook:status",
        AssertionClaim::NegativeExistence,
        1,
    );
    let consolidation = envelope(
        EventPayload::Consolidation(Box::new(Consolidation {
            assertions: vec![valid, invalid],
        })),
        Authority::AssistantGenerated,
    );
    let retract_valid_sibling = envelope(
        EventPayload::Retract(Box::new(Retract {
            belief_id: vec![8],
            provenance: provenance(2, 2),
        })),
        Authority::UserAsserted,
    );
    let events = [consolidation, retract_valid_sibling];

    let first = apply_batch(LSN::new(1), &events, &Evidence::default(), &[]);
    let second = apply_batch(LSN::new(1), &events, &Evidence::default(), &[]);
    assert_eq!(first, second);
    assert!(first.admitted_indices.is_empty());
    assert_eq!(first.skipped.len(), 2);
    assert_eq!(
        first.skipped[0].error.code,
        ErrorCode::NegativeExistenceUncorroborated
    );
    assert_eq!(first.skipped[1].error.code, ErrorCode::BeliefNotFound);
}

#[test]
fn staged_retraction_removes_a_conflict_and_unknown_retraction_is_typed() {
    let live = LiveBelief {
        belief_id: vec![6],
        belief_type: BeliefType::Fact,
        canonical_identity: "deployment:region:europe".to_owned(),
        conflict_domain: "deployment:region".to_owned(),
    };
    let retract = envelope(
        EventPayload::Retract(Box::new(Retract {
            belief_id: vec![6],
            provenance: provenance(1, 1),
        })),
        Authority::UserAsserted,
    );
    let replacement = assertion_event(assertion(
        7,
        "deployment:region:america",
        "deployment:region",
        AssertionClaim::Affirmative,
        1,
    ));
    let admitted = admit_batch(
        LSN::new(2),
        &[retract, replacement],
        &Evidence::default(),
        &[live],
    )
    .expect("the socket batch is evaluated against its staged effects");
    assert!(admitted.conflicts.is_empty());

    let unknown = envelope(
        EventPayload::Retract(Box::new(Retract {
            belief_id: vec![99],
            provenance: provenance(1, 1),
        })),
        Authority::UserAsserted,
    );
    let rejection = admit_batch(LSN::new(2), &[unknown], &Evidence::default(), &[])
        .expect_err("unknown belief retraction must fail at admission");
    assert_eq!(rejection.error.code, ErrorCode::BeliefNotFound);
}
