#![forbid(unsafe_code)]

use hm_compose::bundle::{
    ActivationContext, ActivationRequest, GapKind, Tier, activate_with_context,
};
use hm_compose::lanes::belief::{recall_asof, recall_timeline};
use hm_compose::tokens::{FallbackWeights, TokenCounter};
use hm_core::{ActorId, ConversationId, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_proj::beliefs::{BeliefAsOf, BeliefAsOfAxis, BeliefProjection};
use hm_proj::ladder::TemporalLadder;
use hm_proj::store::ProjectionStore;
use hm_schema::event::encode_event_envelope;
use hm_schema::events::{
    Assertion, AssertionClaim, Authority, BeliefType, EventEnvelope, EventPayload,
    ProposedAssertion, ProvenanceRange, Retention, Sensitivity,
};

const DAY: i64 = 86_400_000_000_000;

#[allow(clippy::too_many_arguments)]
fn assertion(
    id: &str,
    belief_type: BeliefType,
    identity: &str,
    value: &str,
    domain: &str,
    valid_from_ns: i64,
    valid_to_ns: i64,
) -> Assertion {
    Assertion {
        belief_id: id.as_bytes().to_vec(),
        belief_type,
        canonical_identity: identity.to_owned(),
        value: value.as_bytes().to_vec(),
        valid_from_ns,
        valid_to_ns,
        provenance: vec![ProvenanceRange {
            first_lsn: 1,
            last_lsn: 1,
            byte_start: 0,
            byte_end: u32::try_from(value.len()).expect("value length"),
        }],
        conflict_domain: Some(domain.to_owned()),
        claim: AssertionClaim::Affirmative,
    }
}

fn proposed(assertion: &Assertion) -> ProposedAssertion {
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
    payload: EventPayload,
    kind: EventKind,
    authority: Authority,
    run_id: Option<Vec<u8>>,
) -> Frame {
    let event_time_ns = i64::try_from(lsn).expect("lsn") * DAY;
    Frame {
        header: FrameHeader {
            lsn: LSN::new(lsn),
            kind,
            wall_timestamp_ns: UtcNanos::new(event_time_ns),
            actor: ActorId::new(17),
            conversation: ConversationId::new([0x51; 16]),
        },
        sealed_payload: encode_event_envelope(&EventEnvelope {
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
        }),
    }
}

fn assertion_frame(lsn: u64, assertion: Assertion) -> Frame {
    frame(
        lsn,
        EventPayload::Assertion(Box::new(assertion)),
        EventKind::Assertion,
        Authority::UserAsserted,
        None,
    )
}

#[allow(clippy::too_many_lines)]
fn workload() -> Vec<Frame> {
    let proposal = assertion(
        "pending-name",
        BeliefType::Identity,
        "self:display-name",
        "HyperMind",
        "self",
        0,
        0,
    );
    vec![
        assertion_frame(
            1,
            assertion(
                "identity",
                BeliefType::Identity,
                "self:name",
                "Cortex",
                "self",
                0,
                0,
            ),
        ),
        assertion_frame(
            2,
            assertion(
                "goal",
                BeliefType::Goal,
                "goal:release",
                "ship wave five",
                "goals",
                0,
                0,
            ),
        ),
        assertion_frame(
            3,
            assertion(
                "structural",
                BeliefType::Constraint,
                "self:implementation",
                "internal detail",
                "structural-self",
                0,
                0,
            ),
        ),
        assertion_frame(
            4,
            assertion(
                "failure",
                BeliefType::Preference,
                "failure:last",
                "do not surface",
                "failure:tool",
                0,
                0,
            ),
        ),
        assertion_frame(
            5,
            assertion(
                "europe",
                BeliefType::Fact,
                "deployment:region:europe",
                "Europe",
                "deployment:region",
                0,
                0,
            ),
        ),
        assertion_frame(
            6,
            assertion(
                "america",
                BeliefType::Fact,
                "deployment:region:america",
                "America",
                "deployment:region",
                0,
                0,
            ),
        ),
        frame(
            7,
            EventPayload::ProposedAssertion(Box::new(proposed(&proposal))),
            EventKind::ProposedAssertion,
            Authority::DerivedInference,
            Some(b"run-7".to_vec()),
        ),
        assertion_frame(
            8,
            assertion(
                "active-v1",
                BeliefType::Fact,
                "deployment:active",
                "Europe",
                "deployment:active",
                0,
                100,
            ),
        ),
        assertion_frame(
            9,
            assertion(
                "active-v2",
                BeliefType::Fact,
                "deployment:active",
                "America",
                "deployment:active",
                101,
                0,
            ),
        ),
    ]
}

fn build_store() -> (tempfile::TempDir, ProjectionStore) {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 32 * 1024 * 1024).expect("store");
    for frame in workload() {
        BeliefProjection::apply_event(&store, &frame).expect("belief projection");
        TemporalLadder::apply_event(&store, &frame).expect("temporal projection");
    }
    (temporary, store)
}

#[test]
fn resident_conflicts_pending_proposals_and_week_windows_fill_their_tiers() {
    let (_temporary, store) = build_store();
    let snapshot = store.begin_snapshot().expect("snapshot");
    let counter =
        TokenCounter::for_model("fallback", None, FallbackWeights::default()).expect("counter");
    let request = ActivationRequest {
        actor: ActorId::new(17),
        conversation: ConversationId::new([0x51; 16]),
        query: "what is current and conflicting".to_owned(),
        turn_text: "new turn".to_owned(),
        budget_tokens: 100_000,
        token_counter: &counter,
        maximum_candidates: 64,
        maximum_conversation_records: 64,
    };
    let context = ActivationContext {
        now_ns: Some(UtcNanos::new(9 * DAY)),
        ..ActivationContext::default()
    };
    let bundle = activate_with_context(&snapshot, &request, &context).expect("activation");
    let resident = &bundle.sections[Tier::Resident as usize];
    assert!(resident.required);
    assert_eq!(resident.items.len(), 2);
    assert!(
        resident
            .items
            .iter()
            .all(|item| item.provenance == [LSN::new(1)])
    );
    let conflicts = &bundle.sections[Tier::Conflicts as usize];
    assert_eq!(conflicts.items.len(), 3);
    assert!(bundle.gaps.iter().any(|gap| {
        gap.kind == GapKind::PendingProtectedProposal && gap.tier == Some(Tier::Conflicts)
    }));
    assert!(!bundle.sections[Tier::Temporal as usize].items.is_empty());
}

#[test]
fn asof_reports_valid_or_known_axis_and_timeline_descends_to_members() {
    let (_temporary, store) = build_store();
    let snapshot = store.begin_snapshot().expect("snapshot");
    let valid = recall_asof(
        &snapshot,
        BeliefType::Fact,
        "deployment:active",
        BeliefAsOf::ValidAt(50),
    )
    .expect("valid-time recall");
    assert_eq!(valid.axis, BeliefAsOfAxis::ValidTime);
    assert_eq!(valid.record.expect("valid record").value, b"Europe");
    let known = recall_asof(
        &snapshot,
        BeliefType::Fact,
        "deployment:active",
        BeliefAsOf::KnownAt(LSN::new(9)),
    )
    .expect("known-time recall");
    assert_eq!(known.axis, BeliefAsOfAxis::KnownLsn);
    assert_eq!(known.record.expect("known record").value, b"America");

    let timeline = recall_timeline(&snapshot, 5 * DAY, 64).expect("timeline recall");
    assert_eq!(timeline.windows.len(), 4);
    assert!(
        timeline
            .windows
            .windows(2)
            .all(|pair| pair[0].start_ns <= pair[1].start_ns && pair[1].end_ns <= pair[0].end_ns)
    );
    assert!(timeline.members.contains(&LSN::new(5)));
}
