#![forbid(unsafe_code)]
#![allow(clippy::too_many_lines)]

use hm_core::{ActorId, ConversationId, ErrorCode, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_proj::procedures::{ProcedureState, ProceduresProjection};
use hm_proj::rebuild::rebuild_projection_stream;
use hm_proj::store::{ProjectionId, ProjectionStore};
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    Authority, EventEnvelope, EventPayload, ProcedureAdopted, ProcedureImprovementProposed,
    ProcedureMined, ProcedureRevised, ProcedureSupport, Retention, Sensitivity, UserMsg,
};

const PROCEDURE_ID: &[u8] = b"procedure-release";
const MINED_LSN: u64 = 3;
const REVISED_LSN: u64 = 4;
const HEAD_ADOPTION_LSN: u64 = 5;
const FIRST_PROPOSAL_LSN: u64 = 6;
const SECOND_PROPOSAL_LSN: u64 = 7;
const PROPOSAL_ADOPTION_LSN: u64 = 8;

#[test]
fn adopted_procedures_improve_only_through_reviewable_proposals() {
    let frames = stream();

    let split_root = tempfile::tempdir().unwrap();
    let split = ProjectionStore::open(split_root.path(), 64 * 1024 * 1024).unwrap();
    rebuild_projection_stream(
        &split,
        &frames,
        false,
        usize::try_from(HEAD_ADOPTION_LSN).unwrap(),
    )
    .unwrap();

    let snapshot = split.begin_snapshot().unwrap();
    let before = ProceduresProjection::get(&snapshot, PROCEDURE_ID)
        .unwrap()
        .expect("the mined procedure was adopted at its revised version");
    assert_eq!(before.state, ProcedureState::Adopted);
    assert_eq!(before.strategy, "check, deploy, then verify");
    assert_eq!(before.version_lsn, REVISED_LSN);
    assert_eq!(before.previous_lsn, MINED_LSN);
    assert_eq!(before.adopted_lsn, HEAD_ADOPTION_LSN);
    assert_eq!(before.supports.len(), 3);
    drop(snapshot);

    rebuild_projection_stream(&split, &frames, false, 2).unwrap();

    let snapshot = split.begin_snapshot().unwrap();
    assert_eq!(
        ProceduresProjection::get(&snapshot, PROCEDURE_ID)
            .unwrap()
            .expect("the head survives a proposal"),
        before
    );
    let proposal = ProceduresProjection::proposal(&snapshot, PROCEDURE_ID, FIRST_PROPOSAL_LSN)
        .unwrap()
        .expect("the proposal is stored under its own key");
    assert_eq!(proposal.proposal_id, b"proposal-first".to_vec());
    assert_eq!(proposal.procedure_id, PROCEDURE_ID.to_vec());
    assert_eq!(proposal.base_lsn, REVISED_LSN);
    assert_eq!(proposal.strategy, "check, deploy, verify, then announce");
    assert_eq!(
        proposal.expected_outcomes,
        vec!["deployment committed".to_owned(), "team told".to_owned()]
    );
    assert_eq!(
        proposal.preconditions,
        vec![
            "clean repository".to_owned(),
            "announcement drafted".to_owned()
        ]
    );
    assert_eq!(
        proposal.rationale,
        "two deployments landed without a notice"
    );
    assert_eq!(proposal.failure_lsns, vec![1, 2]);
    assert_eq!(proposal.proposed_lsn, FIRST_PROPOSAL_LSN);
    assert_eq!(proposal.adopted_lsn, 0);
    let listed = ProceduresProjection::proposals(&snapshot, PROCEDURE_ID, 4096).unwrap();
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].proposed_lsn, FIRST_PROPOSAL_LSN);
    assert_eq!(listed[1].proposed_lsn, SECOND_PROPOSAL_LSN);
    assert!(
        ProceduresProjection::version(&snapshot, PROCEDURE_ID, REVISED_LSN)
            .unwrap()
            .is_none()
    );
    drop(snapshot);

    assert_eq!(
        ProceduresProjection::apply_event(&split, &stale_proposal())
            .expect_err("a proposal against a superseded version is stale")
            .code,
        ErrorCode::IdempotencyConflict
    );
    assert_eq!(
        ProceduresProjection::apply_event(&split, &orphan_proposal())
            .expect_err("a proposal needs a procedure to improve")
            .code,
        ErrorCode::OrderingViolation
    );

    rebuild_projection_stream(&split, &frames, false, usize::MAX).unwrap();

    let snapshot = split.begin_snapshot().unwrap();
    let head = ProceduresProjection::get(&snapshot, PROCEDURE_ID)
        .unwrap()
        .expect("adopting a proposal installs a new head");
    assert_eq!(head.state, ProcedureState::Adopted);
    assert_eq!(head.strategy, "check, deploy, verify, then announce");
    assert_eq!(
        head.expected_outcomes,
        vec!["deployment committed".to_owned(), "team told".to_owned()]
    );
    assert_eq!(
        head.preconditions,
        vec![
            "clean repository".to_owned(),
            "announcement drafted".to_owned()
        ]
    );
    assert_eq!(head.version_lsn, PROPOSAL_ADOPTION_LSN);
    assert_eq!(head.previous_lsn, REVISED_LSN);
    assert_eq!(head.adopted_lsn, PROPOSAL_ADOPTION_LSN);
    assert_eq!(head.supports, before.supports);
    assert_eq!(head.failures, before.failures);
    assert_eq!(head.counterexamples, before.counterexamples);

    assert_eq!(
        ProceduresProjection::version(&snapshot, PROCEDURE_ID, REVISED_LSN)
            .unwrap()
            .expect("the outgoing adopted version is preserved"),
        before
    );
    assert_eq!(
        ProceduresProjection::proposal(&snapshot, PROCEDURE_ID, FIRST_PROPOSAL_LSN)
            .unwrap()
            .expect("the adopted proposal is still readable")
            .adopted_lsn,
        PROPOSAL_ADOPTION_LSN
    );
    assert_eq!(
        ProceduresProjection::list(&snapshot, 4096).unwrap().len(),
        1
    );
    let split_dump = snapshot.canonical_dump(ProjectionId::Procedures).unwrap();
    drop(snapshot);

    assert_eq!(
        ProceduresProjection::apply_event(&split, &adoption(9, FIRST_PROPOSAL_LSN))
            .expect_err("a proposal cannot be adopted twice")
            .code,
        ErrorCode::OrderingViolation
    );
    assert_eq!(
        ProceduresProjection::apply_event(&split, &adoption(9, SECOND_PROPOSAL_LSN))
            .expect_err("a proposal drafted against a superseded head is not adoptable")
            .code,
        ErrorCode::OrderingViolation
    );

    let full_root = tempfile::tempdir().unwrap();
    let full = ProjectionStore::open(full_root.path(), 64 * 1024 * 1024).unwrap();
    let progress = rebuild_projection_stream(&full, &frames, false, usize::MAX).unwrap();
    assert!(progress.complete);
    assert_eq!(
        full.begin_snapshot()
            .unwrap()
            .canonical_dump(ProjectionId::Procedures)
            .unwrap(),
        split_dump
    );
}

fn stream() -> Vec<Frame> {
    let mut frames = Vec::new();
    push(
        &mut frames,
        EventKind::UserMsg,
        EventPayload::UserMsg(Box::new(UserMsg {
            content: b"the deploy shipped with no announcement".to_vec(),
        })),
        Authority::RuntimeFact,
    );
    push(
        &mut frames,
        EventKind::UserMsg,
        EventPayload::UserMsg(Box::new(UserMsg {
            content: b"nobody heard about the second deploy either".to_vec(),
        })),
        Authority::RuntimeFact,
    );
    push(
        &mut frames,
        EventKind::ProcedureMined,
        EventPayload::ProcedureMined(Box::new(ProcedureMined {
            procedure_id: PROCEDURE_ID.to_vec(),
            strategy: "check then deploy".to_owned(),
            expected_outcomes: vec!["deployment committed".to_owned()],
            preconditions: vec!["clean repository".to_owned()],
            supports: vec![support(1, 1, 1)],
            failures: None,
            counterexamples: None,
        })),
        Authority::DerivedInference,
    );
    push(
        &mut frames,
        EventKind::ProcedureRevised,
        EventPayload::ProcedureRevised(Box::new(ProcedureRevised {
            procedure_id: PROCEDURE_ID.to_vec(),
            previous_lsn: MINED_LSN,
            strategy: "check, deploy, then verify".to_owned(),
            expected_outcomes: vec!["deployment committed".to_owned()],
            preconditions: vec!["clean repository".to_owned()],
            supports: vec![support(1, 1, 1), support(2, 2, 2), support(3, 1, 3)],
            failures: Some(vec![1]),
            counterexamples: Some(vec![2]),
        })),
        Authority::DerivedInference,
    );
    push(
        &mut frames,
        EventKind::ProcedureAdopted,
        EventPayload::ProcedureAdopted(Box::new(ProcedureAdopted {
            procedure_id: PROCEDURE_ID.to_vec(),
            procedure_lsn: REVISED_LSN,
        })),
        Authority::UserAsserted,
    );
    push(
        &mut frames,
        EventKind::ProcedureImprovementProposed,
        EventPayload::ProcedureImprovementProposed(Box::new(proposal(
            b"proposal-first",
            REVISED_LSN,
        ))),
        Authority::DerivedInference,
    );
    push(
        &mut frames,
        EventKind::ProcedureImprovementProposed,
        EventPayload::ProcedureImprovementProposed(Box::new(proposal(
            b"proposal-second",
            REVISED_LSN,
        ))),
        Authority::DerivedInference,
    );
    push(
        &mut frames,
        EventKind::ProcedureAdopted,
        EventPayload::ProcedureAdopted(Box::new(ProcedureAdopted {
            procedure_id: PROCEDURE_ID.to_vec(),
            procedure_lsn: FIRST_PROPOSAL_LSN,
        })),
        Authority::UserAsserted,
    );
    frames
}

fn stale_proposal() -> Frame {
    frame(
        PROPOSAL_ADOPTION_LSN,
        EventKind::ProcedureImprovementProposed,
        EventPayload::ProcedureImprovementProposed(Box::new(proposal(
            b"proposal-stale",
            MINED_LSN,
        ))),
        Authority::DerivedInference,
    )
}

fn orphan_proposal() -> Frame {
    let mut payload = proposal(b"proposal-orphan", REVISED_LSN);
    payload.procedure_id = b"procedure-unknown".to_vec();
    frame(
        PROPOSAL_ADOPTION_LSN,
        EventKind::ProcedureImprovementProposed,
        EventPayload::ProcedureImprovementProposed(Box::new(payload)),
        Authority::DerivedInference,
    )
}

fn adoption(lsn: u64, procedure_lsn: u64) -> Frame {
    frame(
        lsn,
        EventKind::ProcedureAdopted,
        EventPayload::ProcedureAdopted(Box::new(ProcedureAdopted {
            procedure_id: PROCEDURE_ID.to_vec(),
            procedure_lsn,
        })),
        Authority::UserAsserted,
    )
}

fn proposal(proposal_id: &[u8], base_lsn: u64) -> ProcedureImprovementProposed {
    ProcedureImprovementProposed {
        proposal_id: proposal_id.to_vec(),
        procedure_id: PROCEDURE_ID.to_vec(),
        base_lsn,
        strategy: "check, deploy, verify, then announce".to_owned(),
        expected_outcomes: vec!["deployment committed".to_owned(), "team told".to_owned()],
        preconditions: vec![
            "clean repository".to_owned(),
            "announcement drafted".to_owned(),
        ],
        rationale: "two deployments landed without a notice".to_owned(),
        failure_lsns: vec![1, 2],
    }
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
    frames.push(frame(lsn, kind, payload, authority));
    lsn
}

fn frame(lsn: u64, kind: EventKind, payload: EventPayload, authority: Authority) -> Frame {
    Frame {
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
    }
}
