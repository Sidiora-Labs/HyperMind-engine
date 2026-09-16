#![forbid(unsafe_code)]

use hm_core::{ActorId, ConversationId, ErrorCode, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_proj::procedures::{ProcedureState, ProceduresProjection};
use hm_proj::rebuild::rebuild_projection_stream;
use hm_proj::store::{ProjectionId, ProjectionStore};
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    Authority, EventEnvelope, EventPayload, ProcedureAdopted, ProcedureImported, ProcedureMined,
    ProcedureSupport, Retention, Sensitivity,
};

const PLAYBOOK_ID: &[u8] = b"playbook-release-checks";
const MINED_ID: &[u8] = b"procedure-mined";
const INSTRUCTIONS: &[u8] =
    b"1. read the release notes\n2. run the checks\n3. publish only when green\n";
const DIGEST: [u8; 32] = [0x5a; 32];

#[test]
fn imported_playbooks_are_metadata_first_and_adoptable_by_user_authority() {
    let frames = stream();

    let split_root = tempfile::tempdir().unwrap();
    let split = ProjectionStore::open(split_root.path(), 64 * 1024 * 1024).unwrap();
    rebuild_projection_stream(&split, &frames, false, 2).unwrap();

    let snapshot = split.begin_snapshot().unwrap();
    let record = ProceduresProjection::get(&snapshot, PLAYBOOK_ID)
        .unwrap()
        .expect("the imported playbook is a head record");
    assert_eq!(record.state, ProcedureState::Imported);
    assert!(record.supports.is_empty());
    assert!(record.failures.is_empty());
    assert!(record.counterexamples.is_empty());
    assert_eq!(record.strategy, "follow the release playbook");
    assert_eq!(record.version_lsn, 1);
    assert_eq!(record.previous_lsn, 0);
    assert_eq!(record.adopted_lsn, 0);

    let metadata = ProceduresProjection::playbook_metadata(&snapshot, PLAYBOOK_ID)
        .unwrap()
        .expect("the import stores playbook metadata");
    assert_eq!(metadata.procedure_id, PLAYBOOK_ID);
    assert_eq!(metadata.name, "release checks");
    assert_eq!(
        metadata.declared_tools,
        vec!["repository".to_owned(), "deploy".to_owned()]
    );
    assert_eq!(metadata.source_uri, "file:///playbooks/release-checks.md");
    assert_eq!(metadata.source_digest, DIGEST.to_vec());
    assert_eq!(metadata.playbook_version, 3);
    assert_eq!(
        metadata.instruction_bytes,
        u64::try_from(INSTRUCTIONS.len()).unwrap()
    );
    assert_eq!(metadata.imported_lsn, 1);

    assert_eq!(
        ProceduresProjection::playbook_instructions(&snapshot, PLAYBOOK_ID)
            .unwrap()
            .expect("the instruction body is stored under its own key"),
        INSTRUCTIONS.to_vec()
    );
    let listed = ProceduresProjection::list(&snapshot, 4096).unwrap();
    assert_eq!(listed.len(), 2);
    assert!(
        ProceduresProjection::playbook_instructions(&snapshot, MINED_ID)
            .unwrap()
            .is_none()
    );
    drop(snapshot);

    assert_eq!(
        ProceduresProjection::apply_event(&split, &duplicate_import())
            .expect_err("the same procedure id cannot be imported twice")
            .code,
        ErrorCode::AlreadyExists
    );
    assert_eq!(
        ProceduresProjection::apply_event(&split, &tentative_adoption())
            .expect_err("a tentative head is not adoptable")
            .code,
        ErrorCode::OrderingViolation
    );

    rebuild_projection_stream(&split, &frames, false, usize::MAX).unwrap();
    let snapshot = split.begin_snapshot().unwrap();
    let adopted = ProceduresProjection::get(&snapshot, PLAYBOOK_ID)
        .unwrap()
        .expect("the adopted playbook is still a head record");
    assert_eq!(adopted.state, ProcedureState::Adopted);
    assert_eq!(adopted.version_lsn, 1);
    assert_eq!(adopted.adopted_lsn, 3);
    assert!(adopted.supports.is_empty());
    let split_dump = snapshot.canonical_dump(ProjectionId::Procedures).unwrap();
    drop(snapshot);

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
    let imported_lsn = push(
        &mut frames,
        EventKind::ProcedureImported,
        EventPayload::ProcedureImported(Box::new(playbook())),
        Authority::ExternalObserved,
    );
    push(
        &mut frames,
        EventKind::ProcedureMined,
        EventPayload::ProcedureMined(Box::new(mined())),
        Authority::DerivedInference,
    );
    push(
        &mut frames,
        EventKind::ProcedureAdopted,
        EventPayload::ProcedureAdopted(Box::new(ProcedureAdopted {
            procedure_id: PLAYBOOK_ID.to_vec(),
            procedure_lsn: imported_lsn,
        })),
        Authority::UserAsserted,
    );
    frames
}

fn duplicate_import() -> Frame {
    frame(
        3,
        EventKind::ProcedureImported,
        EventPayload::ProcedureImported(Box::new(playbook())),
        Authority::ExternalObserved,
    )
}

fn tentative_adoption() -> Frame {
    frame(
        3,
        EventKind::ProcedureAdopted,
        EventPayload::ProcedureAdopted(Box::new(ProcedureAdopted {
            procedure_id: MINED_ID.to_vec(),
            procedure_lsn: 2,
        })),
        Authority::UserAsserted,
    )
}

fn playbook() -> ProcedureImported {
    ProcedureImported {
        procedure_id: PLAYBOOK_ID.to_vec(),
        name: "release checks".to_owned(),
        strategy: "follow the release playbook".to_owned(),
        expected_outcomes: vec!["release published".to_owned()],
        preconditions: vec!["checks are green".to_owned()],
        instructions: INSTRUCTIONS.to_vec(),
        declared_tools: vec!["repository".to_owned(), "deploy".to_owned()],
        source_uri: "file:///playbooks/release-checks.md".to_owned(),
        source_digest: DIGEST.to_vec(),
        playbook_version: 3,
    }
}

fn mined() -> ProcedureMined {
    ProcedureMined {
        procedure_id: MINED_ID.to_vec(),
        strategy: "check then deploy".to_owned(),
        expected_outcomes: vec!["deployment committed".to_owned()],
        preconditions: vec!["clean repository".to_owned()],
        supports: vec![ProcedureSupport {
            source_root: vec![1; 32],
            conversation: vec![1; 16],
            episode_lsn: 1,
        }],
        failures: None,
        counterexamples: None,
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
