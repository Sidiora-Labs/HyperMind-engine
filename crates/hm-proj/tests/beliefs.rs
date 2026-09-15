use hm_core::{ActorId, ConversationId, ErrorCode, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_proj::beliefs::{BeliefAsOf, BeliefAsOfAxis, BeliefProjection};
use hm_proj::protected::{enforce_direct_write, rejection_error};
use hm_proj::store::{ProjectionId, ProjectionStore};
use hm_schema::event::encode_event_envelope;
use hm_schema::events::{
    Assertion, AssertionClaim, Authority, BeliefType, Consolidation, EventEnvelope, EventPayload,
    ProposedAssertion, ProvenanceRange, ResultStatus, Retention, Retract, Sensitivity, ToolCall,
    ToolResult,
};

fn provenance(lsn: u64) -> Vec<ProvenanceRange> {
    vec![ProvenanceRange {
        first_lsn: lsn,
        last_lsn: lsn,
        byte_start: 3,
        byte_end: 11,
    }]
}

#[allow(clippy::too_many_arguments)]
fn assertion(
    id: &[u8],
    belief_type: BeliefType,
    identity: &str,
    value: &str,
    domain: &str,
    valid_from_ns: i64,
    valid_to_ns: i64,
    claim: AssertionClaim,
    evidence_lsn: u64,
) -> Assertion {
    Assertion {
        belief_id: id.to_vec(),
        belief_type,
        canonical_identity: identity.to_owned(),
        value: value.as_bytes().to_vec(),
        valid_from_ns,
        valid_to_ns,
        provenance: provenance(evidence_lsn),
        conflict_domain: Some(domain.to_owned()),
        claim,
    }
}

fn proposal(assertion: &Assertion) -> ProposedAssertion {
    ProposedAssertion {
        belief_id: assertion.belief_id.clone(),
        belief_type: assertion.belief_type,
        canonical_identity: assertion.canonical_identity.clone(),
        value: assertion.value.clone(),
        valid_from_ns: assertion.valid_from_ns,
        valid_to_ns: assertion.valid_to_ns,
        provenance: assertion.provenance.clone(),
        conflict_domain: assertion.conflict_domain.clone(),
        claim: assertion.claim,
    }
}

fn frame(
    lsn: u64,
    kind: EventKind,
    payload: EventPayload,
    authority: Authority,
    run_id: Option<Vec<u8>>,
    event_time_ns: i64,
) -> Frame {
    let envelope = EventEnvelope {
        schema_version: 2,
        payload,
        connection_id: None,
        client_seq: 0,
        client_event_index: 0,
        client_event_count: 0,
        origin_actor: 0,
        run_id,
        model_provenance: None,
        authority,
        retention: Retention::CurrentState,
        sensitivity: Sensitivity::Public,
        event_time_ns,
    };
    Frame {
        header: FrameHeader {
            lsn: LSN::new(lsn),
            kind,
            wall_timestamp_ns: UtcNanos::new(event_time_ns),
            actor: ActorId::new(7),
            conversation: ConversationId::new([0x45; 16]),
        },
        sealed_payload: encode_event_envelope(&envelope),
    }
}

fn assertion_frame(
    lsn: u64,
    assertion: Assertion,
    authority: Authority,
    run_id: Option<Vec<u8>>,
    event_time_ns: i64,
) -> Frame {
    frame(
        lsn,
        EventKind::Assertion,
        EventPayload::Assertion(Box::new(assertion)),
        authority,
        run_id,
        event_time_ns,
    )
}

#[test]
fn versions_are_tri_temporal_and_asof_reports_the_selected_axis() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 32 * 1024 * 1024).expect("store");
    let frames = vec![
        assertion_frame(
            1,
            assertion(
                b"region-v1",
                BeliefType::Fact,
                "deployment:region",
                "europe",
                "deployment:region",
                0,
                100,
                AssertionClaim::Affirmative,
                1,
            ),
            Authority::UserAsserted,
            None,
            10,
        ),
        assertion_frame(
            2,
            assertion(
                b"region-v2",
                BeliefType::Fact,
                "deployment:region",
                "america",
                "deployment:region",
                101,
                0,
                AssertionClaim::Affirmative,
                1,
            ),
            Authority::UserAsserted,
            None,
            200,
        ),
    ];
    let partial = BeliefProjection::rebuild(&store, &frames, false, 1).expect("partial");
    assert!(!partial.complete);
    let resumed = BeliefProjection::rebuild(&store, &frames, false, usize::MAX).expect("resume");
    assert!(resumed.complete);

    let snapshot = store.begin_snapshot().expect("snapshot");
    let valid_old = BeliefProjection::read_as_of(
        &snapshot,
        BeliefType::Fact,
        "deployment:region",
        BeliefAsOf::ValidAt(50),
    )
    .expect("valid as-of");
    assert_eq!(valid_old.axis, BeliefAsOfAxis::ValidTime);
    let valid_old = valid_old.record.expect("old valid record");
    assert_eq!(valid_old.value, b"europe");
    assert_eq!(valid_old.event_time_ns, 10);
    assert_eq!(valid_old.observation_lsn, 1);
    assert_eq!(valid_old.version, 1);

    let known_latest = BeliefProjection::read_as_of(
        &snapshot,
        BeliefType::Fact,
        "deployment:region",
        BeliefAsOf::KnownAt(LSN::new(2)),
    )
    .expect("known as-of");
    assert_eq!(known_latest.axis, BeliefAsOfAxis::KnownLsn);
    let known_latest = known_latest.record.expect("latest known record");
    assert_eq!(known_latest.value, b"america");
    assert_eq!(known_latest.version, 2);
    assert_eq!(known_latest.supersedes_version, 1);
    assert_ne!(valid_old.value, known_latest.value);
    drop(snapshot);

    let original = store
        .begin_snapshot()
        .expect("snapshot")
        .canonical_dump(ProjectionId::BeliefStore)
        .expect("dump");
    BeliefProjection::rebuild(&store, &frames, true, usize::MAX).expect("rebuild");
    let rebuilt = store
        .begin_snapshot()
        .expect("snapshot")
        .canonical_dump(ProjectionId::BeliefStore)
        .expect("rebuilt dump");
    assert_eq!(original, rebuilt);
}

#[test]
fn conflicts_are_obligated_until_retraction_resolves_both_edges() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 32 * 1024 * 1024).expect("store");
    let europe = assertion(
        b"europe",
        BeliefType::Fact,
        "deployment:region:europe",
        "europe",
        "deployment:region",
        0,
        0,
        AssertionClaim::Affirmative,
        1,
    );
    let america = assertion(
        b"america",
        BeliefType::Fact,
        "deployment:region:america",
        "america",
        "deployment:region",
        0,
        0,
        AssertionClaim::Affirmative,
        1,
    );
    let frames = vec![
        assertion_frame(1, europe, Authority::UserAsserted, None, 10),
        assertion_frame(2, america, Authority::UserAsserted, None, 20),
        frame(
            3,
            EventKind::Retract,
            EventPayload::Retract(Box::new(Retract {
                belief_id: b"america".to_vec(),
                provenance: provenance(2),
            })),
            Authority::UserAsserted,
            None,
            30,
        ),
    ];
    BeliefProjection::rebuild(&store, &frames, false, usize::MAX).expect("rebuild");
    let snapshot = store.begin_snapshot().expect("snapshot");
    let during = BeliefProjection::read_as_of(
        &snapshot,
        BeliefType::Fact,
        "deployment:region:europe",
        BeliefAsOf::KnownAt(LSN::new(2)),
    )
    .expect("during")
    .record
    .expect("europe during");
    assert_eq!(during.conflict_edges.len(), 1);
    assert!(during.conflict_edges[0].obligated_surfacing);
    assert_eq!(
        during.conflict_edges[0].other_canonical_identity,
        "deployment:region:america"
    );
    let heads = BeliefProjection::read_heads(&snapshot, 10).expect("heads");
    assert_eq!(heads.len(), 1);
    assert!(heads[0].conflict_edges.is_empty());
}

#[test]
fn only_successful_tool_observed_results_corroborate_negative_existence() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 32 * 1024 * 1024).expect("store");
    let frames = vec![
        frame(
            1,
            EventKind::ToolCall,
            EventPayload::ToolCall(Box::new(ToolCall {
                call_id: b"probe".to_vec(),
                tool_name: "lookup".to_owned(),
                arguments: b"moltbook".to_vec(),
            })),
            Authority::AssistantGenerated,
            None,
            10,
        ),
        frame(
            2,
            EventKind::ToolResult,
            EventPayload::ToolResult(Box::new(ToolResult {
                call_id: b"probe".to_vec(),
                tool_call_lsn: 1,
                status: ResultStatus::Ok,
                result: b"not found".to_vec(),
            })),
            Authority::ToolObserved,
            None,
            20,
        ),
        assertion_frame(
            3,
            assertion(
                b"missing",
                BeliefType::Fact,
                "service:moltbook:absent",
                "absent",
                "service:moltbook",
                0,
                0,
                AssertionClaim::NegativeExistence,
                2,
            ),
            Authority::UserAsserted,
            None,
            30,
        ),
        assertion_frame(
            4,
            assertion(
                b"poison",
                BeliefType::Fact,
                "service:unknown:absent",
                "absent",
                "service:unknown",
                0,
                0,
                AssertionClaim::NegativeExistence,
                1,
            ),
            Authority::UserAsserted,
            None,
            40,
        ),
    ];
    BeliefProjection::rebuild(&store, &frames, false, usize::MAX).expect("rebuild");
    let snapshot = store.begin_snapshot().expect("snapshot");
    assert!(
        BeliefProjection::read_as_of(
            &snapshot,
            BeliefType::Fact,
            "service:moltbook:absent",
            BeliefAsOf::KnownAt(LSN::new(4)),
        )
        .expect("accepted negative")
        .record
        .is_some()
    );
    assert!(
        BeliefProjection::read_as_of(
            &snapshot,
            BeliefType::Fact,
            "service:unknown:absent",
            BeliefAsOf::KnownAt(LSN::new(4)),
        )
        .expect("skipped poisoning")
        .record
        .is_none()
    );
    assert_eq!(
        snapshot
            .checkpoint(ProjectionId::BeliefStore)
            .expect("checkpoint"),
        LSN::new(4)
    );
}

#[test]
fn mixed_consolidation_is_skipped_atomically_during_replay() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).expect("store");
    let valid = assertion(
        b"valid-sibling",
        BeliefType::Fact,
        "consolidation:valid",
        "present",
        "consolidation",
        0,
        0,
        AssertionClaim::Affirmative,
        1,
    );
    let poisoned = assertion(
        b"invalid-sibling",
        BeliefType::Fact,
        "consolidation:absent",
        "absent",
        "consolidation",
        0,
        0,
        AssertionClaim::NegativeExistence,
        1,
    );
    let consolidation = frame(
        1,
        EventKind::Consolidation,
        EventPayload::Consolidation(Box::new(Consolidation {
            assertions: vec![valid, poisoned],
        })),
        Authority::DerivedInference,
        Some(b"run-mixed".to_vec()),
        10,
    );
    BeliefProjection::apply_event(&store, &consolidation).expect("deterministic policy skip");
    let snapshot = store.begin_snapshot().expect("snapshot");
    assert!(
        BeliefProjection::read_heads(&snapshot, 10)
            .expect("heads")
            .is_empty()
    );
    assert_eq!(
        snapshot
            .checkpoint(ProjectionId::BeliefStore)
            .expect("checkpoint"),
        LSN::new(1)
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn protected_run_writes_emit_security_events_and_proposals_await_user_choice() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 32 * 1024 * 1024).expect("store");
    let identity = assertion(
        b"identity-1",
        BeliefType::Identity,
        "self:name",
        "HyperMind",
        "self",
        0,
        0,
        AssertionClaim::Affirmative,
        1,
    );
    let preference = assertion(
        b"preference-1",
        BeliefType::Preference,
        "self:color",
        "blue",
        "self:preference",
        0,
        0,
        AssertionClaim::Affirmative,
        1,
    );
    let constraint = assertion(
        b"constraint-1",
        BeliefType::Constraint,
        "self:boundary",
        "never disclose secrets",
        "self:constraint",
        0,
        0,
        AssertionClaim::Affirmative,
        1,
    );
    let frames = vec![
        frame(
            1,
            EventKind::ProposedAssertion,
            EventPayload::ProposedAssertion(Box::new(proposal(&identity))),
            Authority::DerivedInference,
            Some(b"run-1".to_vec()),
            10,
        ),
        assertion_frame(
            2,
            identity.clone(),
            Authority::DerivedInference,
            Some(b"run-1".to_vec()),
            20,
        ),
        assertion_frame(3, identity, Authority::UserAsserted, None, 30),
        frame(
            4,
            EventKind::ProposedAssertion,
            EventPayload::ProposedAssertion(Box::new(proposal(&preference))),
            Authority::DerivedInference,
            Some(b"run-2".to_vec()),
            40,
        ),
        frame(
            5,
            EventKind::Retract,
            EventPayload::Retract(Box::new(Retract {
                belief_id: b"preference-1".to_vec(),
                provenance: provenance(4),
            })),
            Authority::UserAsserted,
            None,
            50,
        ),
        frame(
            6,
            EventKind::Retract,
            EventPayload::Retract(Box::new(Retract {
                belief_id: b"identity-1".to_vec(),
                provenance: provenance(3),
            })),
            Authority::DerivedInference,
            Some(b"run-3".to_vec()),
            60,
        ),
        frame(
            7,
            EventKind::ProposedAssertion,
            EventPayload::ProposedAssertion(Box::new(proposal(&constraint))),
            Authority::DerivedInference,
            Some(b"run-4".to_vec()),
            70,
        ),
        frame(
            8,
            EventKind::Retract,
            EventPayload::Retract(Box::new(Retract {
                belief_id: b"constraint-1".to_vec(),
                provenance: provenance(7),
            })),
            Authority::DerivedInference,
            Some(b"run-5".to_vec()),
            80,
        ),
        frame(
            9,
            EventKind::Retract,
            EventPayload::Retract(Box::new(Retract {
                belief_id: b"constraint-1".to_vec(),
                provenance: provenance(7),
            })),
            Authority::UserAsserted,
            None,
            90,
        ),
    ];
    BeliefProjection::rebuild(&store, &frames, false, usize::MAX).expect("rebuild");
    let snapshot = store.begin_snapshot().expect("snapshot");
    assert!(
        BeliefProjection::read_pending(&snapshot, 10)
            .expect("pending")
            .is_empty()
    );
    let identity = BeliefProjection::read_as_of(
        &snapshot,
        BeliefType::Identity,
        "self:name",
        BeliefAsOf::KnownAt(LSN::new(9)),
    )
    .expect("identity")
    .record
    .expect("protected identity remains live");
    assert_eq!(identity.authority, Authority::UserAsserted);
    let security = BeliefProjection::read_security_events(&snapshot, 10).expect("security");
    assert_eq!(security.len(), 3);
    assert!(
        security
            .iter()
            .all(|event| event.code == ErrorCode::ProtectedTypeWrite.as_str())
    );

    let event = enforce_direct_write(
        BeliefType::Constraint,
        b"constraint-1",
        Authority::AssistantGenerated,
        Some(b"run-4"),
        LSN::new(7),
    )
    .expect_err("run authority cannot write a protected type");
    assert_eq!(rejection_error(&event).code, ErrorCode::ProtectedTypeWrite);
}
