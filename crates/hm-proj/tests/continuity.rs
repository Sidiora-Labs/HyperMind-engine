use hm_core::{ActorId, ConversationId, Error, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_proj::bindings::{
    BindingRequirement, BindingStatus, BindingsProjection, resolve_required_bindings,
};
use hm_proj::checkpoint::{encode_checkpoint_cursor, latest_checkpoint, turn_conversation};
use hm_proj::intent::{IntentFrameProjection, LoopClosure};
use hm_proj::ledger::{WorkKind, WorkLedgerProjection, WorkState};
use hm_proj::store::{ProjectionId, ProjectionStore};
use hm_proj::timeline::apply_timeline;
use hm_schema::event::encode_event_envelope;
use hm_schema::events::{
    Authority, Binding, Checkpoint, Effect, EffectState, EventEnvelope, EventPayload, IntentSet,
    LoopCloseReason, LoopClosed, LoopOpened, Outcome, ResultStatus, Retention, Sensitivity,
    ToolCall, ToolResult, UserMsg,
};
use std::path::Path;
use std::process::Command;

const MAP_BYTES: usize = 32 * 1024 * 1024;

fn envelope(payload: EventPayload, authority: Authority, event_time_ns: i64) -> Vec<u8> {
    encode_event_envelope(&EventEnvelope {
        schema_version: 2,
        payload,
        connection_id: None,
        client_seq: 0,
        client_event_index: 0,
        client_event_count: 0,
        origin_actor: 17,
        run_id: None,
        model_provenance: None,
        authority,
        retention: Retention::CurrentState,
        sensitivity: Sensitivity::Public,
        event_time_ns,
    })
}

fn frame(
    lsn: u64,
    kind: EventKind,
    conversation: ConversationId,
    payload: EventPayload,
    authority: Authority,
    event_time_ns: i64,
) -> Frame {
    Frame {
        header: FrameHeader {
            lsn: LSN::new(lsn),
            kind,
            wall_timestamp_ns: UtcNanos::new(event_time_ns),
            actor: ActorId::new(17),
            conversation,
        },
        sealed_payload: envelope(payload, authority, event_time_ns),
    }
}

#[allow(clippy::too_many_lines)]
fn workload(conversation: ConversationId) -> Vec<Frame> {
    vec![
        frame(
            1,
            EventKind::IntentSet,
            conversation,
            EventPayload::IntentSet(Box::new(IntentSet {
                objective: b"objective-one".to_vec(),
            })),
            Authority::UserAsserted,
            1_000,
        ),
        frame(
            2,
            EventKind::LoopOpened,
            conversation,
            EventPayload::LoopOpened(Box::new(LoopOpened {
                loop_id: b"loop-a".to_vec(),
                objective: b"deliver answer".to_vec(),
            })),
            Authority::UserAsserted,
            2_000,
        ),
        frame(
            3,
            EventKind::UserMsg,
            conversation,
            EventPayload::UserMsg(Box::new(UserMsg {
                content: b"continue".to_vec(),
            })),
            Authority::UserAsserted,
            3_000,
        ),
        frame(
            4,
            EventKind::ToolCall,
            conversation,
            EventPayload::ToolCall(Box::new(ToolCall {
                call_id: b"call-a".to_vec(),
                tool_name: "lookup".to_owned(),
                arguments: b"alpha".to_vec(),
            })),
            Authority::AssistantGenerated,
            4_000,
        ),
        frame(
            5,
            EventKind::Effect,
            conversation,
            EventPayload::Effect(Box::new(Effect {
                effect_id: b"effect-a".to_vec(),
                tool_call_lsn: 4,
                state: EffectState::Dispatched,
            })),
            Authority::RuntimeFact,
            5_000,
        ),
        frame(
            6,
            EventKind::ToolResult,
            conversation,
            EventPayload::ToolResult(Box::new(ToolResult {
                call_id: b"call-a".to_vec(),
                tool_call_lsn: 4,
                status: ResultStatus::Ok,
                result: b"lookup complete".to_vec(),
            })),
            Authority::ToolObserved,
            6_000,
        ),
        frame(
            7,
            EventKind::Outcome,
            conversation,
            EventPayload::Outcome(Box::new(Outcome {
                effect_id: b"effect-a".to_vec(),
                status: ResultStatus::Ok,
                detail: b"effect committed".to_vec(),
                evidence_lsns: Some(vec![6]),
            })),
            Authority::ExternalObserved,
            7_000,
        ),
        frame(
            8,
            EventKind::LoopOpened,
            conversation,
            EventPayload::LoopOpened(Box::new(LoopOpened {
                loop_id: b"loop-b".to_vec(),
                objective: b"reconcile write".to_vec(),
            })),
            Authority::UserAsserted,
            8_000,
        ),
        frame(
            9,
            EventKind::ToolCall,
            conversation,
            EventPayload::ToolCall(Box::new(ToolCall {
                call_id: b"call-b".to_vec(),
                tool_name: "write".to_owned(),
                arguments: b"beta".to_vec(),
            })),
            Authority::AssistantGenerated,
            9_000,
        ),
        frame(
            10,
            EventKind::Effect,
            conversation,
            EventPayload::Effect(Box::new(Effect {
                effect_id: b"effect-b".to_vec(),
                tool_call_lsn: 9,
                state: EffectState::Committed,
            })),
            Authority::RuntimeFact,
            10_000,
        ),
        frame(
            11,
            EventKind::IntentSet,
            conversation,
            EventPayload::IntentSet(Box::new(IntentSet {
                objective: b"objective-two".to_vec(),
            })),
            Authority::UserAsserted,
            11_000,
        ),
        frame(
            12,
            EventKind::LoopClosed,
            conversation,
            EventPayload::LoopClosed(Box::new(LoopClosed {
                loop_id: b"loop-a".to_vec(),
                reason: LoopCloseReason::Abandoned,
                cause: b"owner changed direction".to_vec(),
                evidence_lsns: None,
            })),
            Authority::UserAsserted,
            12_000,
        ),
        frame(
            13,
            EventKind::Outcome,
            conversation,
            EventPayload::Outcome(Box::new(Outcome {
                effect_id: b"effect-b".to_vec(),
                status: ResultStatus::OutcomeUnknown,
                detail: b"crashed after dispatch".to_vec(),
                evidence_lsns: Some(vec![10]),
            })),
            Authority::RuntimeFact,
            13_000,
        ),
        binding_frame(
            14,
            conversation,
            b"task-a",
            "repo",
            "revision",
            b"r1",
            12_000,
        ),
        binding_frame(
            15,
            conversation,
            b"task-a",
            "repo",
            "revision",
            b"old",
            11_000,
        ),
        binding_frame(
            16,
            conversation,
            b"task-a",
            "repo",
            "revision",
            b"r2",
            12_000,
        ),
    ]
}

fn binding_frame(
    lsn: u64,
    conversation: ConversationId,
    task: &[u8],
    entity: &str,
    property: &str,
    revision: &[u8],
    effective_time_ns: i64,
) -> Frame {
    frame(
        lsn,
        EventKind::Binding,
        conversation,
        EventPayload::Binding(Box::new(Binding {
            task: Some(task.to_vec()),
            scope: None,
            canonical_entity: entity.to_owned(),
            property: property.to_owned(),
            evidence_lsn: 10,
            revision: revision.to_vec(),
            freshness_requirement_ns: 60_000,
        })),
        Authority::RuntimeFact,
        effective_time_ns,
    )
}

fn rebuild_all(
    store: &ProjectionStore,
    frames: &[Frame],
    reset: bool,
    maximum_frames: usize,
) -> Result<(), Error> {
    if reset {
        store.reset(ProjectionId::ConversationHeads)?;
    }
    let snapshot = store.begin_snapshot()?;
    let checkpoint = snapshot.checkpoint(ProjectionId::ConversationHeads)?.get();
    drop(snapshot);
    let start = usize::try_from(checkpoint)
        .map_err(|_| Error::new(hm_core::ErrorCode::CapacityExceeded))?;
    for item in frames.iter().skip(start) {
        apply_timeline(store, item)?;
    }
    IntentFrameProjection::rebuild(store, frames, reset, maximum_frames)?;
    WorkLedgerProjection::rebuild(store, frames, reset, maximum_frames)?;
    BindingsProjection::rebuild(store, frames, reset, maximum_frames)?;
    Ok(())
}

fn verify_prefix(path: &Path, frames: &[Frame], conversation: ConversationId) {
    let store = ProjectionStore::open(path, MAP_BYTES).expect("open prefix store");
    rebuild_all(&store, frames, false, usize::MAX).expect("rebuild prefix");
    let snapshot = store.begin_snapshot().expect("prefix snapshot");
    let intent = IntentFrameProjection::read(&snapshot, conversation).expect("intent frame");
    let objective = intent.objective.expect("objective");
    assert_eq!(
        objective.content,
        if frames.len() >= 11 {
            b"objective-two".as_slice()
        } else {
            b"objective-one".as_slice()
        }
    );
    let expected_open =
        usize::from(frames.len() >= 2 && frames.len() < 12) + usize::from(frames.len() >= 8);
    assert_eq!(intent.open_loops.len(), expected_open);
    let work = WorkLedgerProjection::read_conversation(&snapshot, conversation, 4096)
        .expect("work ledger");
    let expected_work = usize::from(frames.len() >= 4)
        + usize::from(frames.len() >= 5)
        + usize::from(frames.len() >= 9)
        + usize::from(frames.len() >= 10);
    assert_eq!(work.len(), expected_work);
    for item in work {
        let expected = (item.tool_call_lsn == LSN::new(4)
            && ((item.kind == WorkKind::ToolCall && frames.len() < 6)
                || (item.kind == WorkKind::Effect && frames.len() < 7)))
            || item.tool_call_lsn == LSN::new(9);
        assert_eq!(item.requires_reconciliation, expected);
    }
}

#[test]
fn every_prefix_reset_and_bounded_resume_are_byte_identical() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let conversation = ConversationId::new([0x71; 16]);
    let frames = workload(conversation);
    for prefix in 1..=frames.len() {
        verify_prefix(
            &temporary.path().join(format!("prefix-{prefix}")),
            &frames[..prefix],
            conversation,
        );
    }

    let path = temporary.path().join("replay");
    let store = ProjectionStore::open(&path, MAP_BYTES).expect("open replay store");
    rebuild_all(&store, &frames, false, usize::MAX).expect("full rebuild");
    let original = canonical_dumps(&store);
    rebuild_all(&store, &frames, true, usize::MAX).expect("reset replay");
    assert_eq!(canonical_dumps(&store), original);

    let snapshot = store.begin_snapshot().expect("typed snapshot");
    let closed = IntentFrameProjection::read_closed(&snapshot, conversation).expect("closed loops");
    assert_eq!(closed.len(), 1);
    assert_eq!(closed[0].reason, LoopClosure::Abandoned);
    assert_eq!(closed[0].cause, b"owner changed direction");
    let tail =
        WorkLedgerProjection::read_conversation(&snapshot, conversation, 2).expect("work tail");
    assert_eq!(tail.len(), 2);
    assert!(tail.iter().all(|item| item.tool_call_lsn == LSN::new(9)));
    assert!(tail.iter().all(|item| item.requires_reconciliation));

    let partial_path = temporary.path().join("partial");
    let partial = ProjectionStore::open(&partial_path, MAP_BYTES).expect("partial store");
    rebuild_all(&partial, &frames, false, 10).expect("bounded prefix");
    rebuild_all(&partial, &frames, false, usize::MAX).expect("bounded resume");
    assert_eq!(canonical_dumps(&partial), original);
}

#[test]
fn sigkill_after_partial_projection_commit_resumes_identically() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let expected_path = temporary.path().join("expected");
    let killed_path = temporary.path().join("killed");
    let conversation = ConversationId::new([0x71; 16]);
    let frames = workload(conversation);
    let expected = ProjectionStore::open(&expected_path, MAP_BYTES).expect("expected store");
    rebuild_all(&expected, &frames, false, usize::MAX).expect("expected rebuild");
    let expected_dumps = canonical_dumps(&expected);

    let status = Command::new(std::env::current_exe().expect("test executable"))
        .arg("--exact")
        .arg("continuity_sigkill_child")
        .arg("--nocapture")
        .env("HM_CONTINUITY_KILLED", &killed_path)
        .status()
        .expect("run child");
    assert!(!status.success());
    let resumed = ProjectionStore::open(&killed_path, MAP_BYTES).expect("reopen killed store");
    rebuild_all(&resumed, &frames, false, usize::MAX).expect("resume killed rebuild");
    assert_eq!(canonical_dumps(&resumed), expected_dumps);
}

#[test]
fn continuity_sigkill_child() {
    let Ok(path) = std::env::var("HM_CONTINUITY_KILLED") else {
        return;
    };
    let conversation = ConversationId::new([0x71; 16]);
    let frames = workload(conversation);
    let store = ProjectionStore::open(path, MAP_BYTES).expect("child store");
    rebuild_all(&store, &frames, false, 10).expect("child partial rebuild");
    let status = Command::new("kill")
        .arg("-9")
        .arg(std::process::id().to_string())
        .status();
    panic!("SIGKILL failed: {status:?}");
}

#[test]
fn global_loop_ids_prevent_cross_conversation_and_double_close() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), MAP_BYTES).expect("store");
    let first = ConversationId::new([1; 16]);
    let second = ConversationId::new([2; 16]);
    let frames = [
        frame(
            1,
            EventKind::LoopOpened,
            first,
            EventPayload::LoopOpened(Box::new(LoopOpened {
                loop_id: b"global".to_vec(),
                objective: b"first".to_vec(),
            })),
            Authority::UserAsserted,
            1,
        ),
        frame(
            2,
            EventKind::LoopClosed,
            second,
            EventPayload::LoopClosed(Box::new(LoopClosed {
                loop_id: b"global".to_vec(),
                reason: LoopCloseReason::Abandoned,
                cause: Vec::new(),
                evidence_lsns: None,
            })),
            Authority::UserAsserted,
            2,
        ),
    ];
    apply_timeline(&store, &frames[0]).expect("timeline open");
    IntentFrameProjection::apply_event(&store, &frames[0]).expect("open loop");
    apply_timeline(&store, &frames[1]).expect("timeline close");
    let error = IntentFrameProjection::apply_event(&store, &frames[1]).expect_err("cross close");
    assert_eq!(error.code, hm_core::ErrorCode::OrderingViolation);
}

#[test]
fn bindings_resolve_missing_stale_conflicting_and_corrections() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), MAP_BYTES).expect("store");
    let conversation = ConversationId::new([4; 16]);
    let frames = vec![
        binding_frame(1, conversation, b"task", "repo", "revision", b"r1", 1_900),
        binding_frame(2, conversation, b"task", "cache", "revision", b"c1", 1_000),
        binding_frame(
            3,
            conversation,
            b"task",
            "branch",
            "revision",
            b"actual",
            1_900,
        ),
        binding_frame(
            4,
            conversation,
            b"task",
            "repo",
            "revision",
            b"older",
            1_800,
        ),
        binding_frame(5, conversation, b"task", "repo", "revision", b"r2", 1_900),
    ];
    BindingsProjection::rebuild(&store, &frames, false, usize::MAX).expect("bindings rebuild");
    let snapshot = store.begin_snapshot().expect("snapshot");
    let resolved = resolve_required_bindings(
        &snapshot,
        b"task",
        &[
            requirement("repo", "revision", Some(b"r2"), Some(500)),
            requirement("cache", "revision", Some(b"c1"), Some(500)),
            requirement("branch", "revision", Some(b"expected"), Some(500)),
            requirement("missing", "revision", None, Some(500)),
        ],
        UtcNanos::new(2_000),
    )
    .expect("resolve bindings");
    assert_eq!(
        resolved.iter().map(|item| item.status).collect::<Vec<_>>(),
        vec![
            BindingStatus::Resolved,
            BindingStatus::Stale,
            BindingStatus::Conflicting,
            BindingStatus::Missing,
        ]
    );
    assert_eq!(
        resolved[0]
            .binding
            .as_ref()
            .expect("corrected binding")
            .revision,
        b"r2"
    );
}

#[test]
fn latest_checkpoint_scans_past_deep_filler() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), MAP_BYTES).expect("store");
    let turn = b"turn-deep";
    let conversation = turn_conversation(turn);
    let mut frames = Vec::new();
    frames.push(frame(
        1,
        EventKind::Checkpoint,
        conversation,
        EventPayload::Checkpoint(Box::new(Checkpoint {
            cursor: encode_checkpoint_cursor(turn, b"deep-blob-one").expect("cursor"),
        })),
        Authority::RuntimeFact,
        1,
    ));
    for lsn in 2..=1_281 {
        frames.push(frame(
            lsn,
            EventKind::UserMsg,
            conversation,
            EventPayload::UserMsg(Box::new(UserMsg {
                content: format!("filler-{lsn}").into_bytes(),
            })),
            Authority::UserAsserted,
            i64::try_from(lsn).expect("timestamp fits"),
        ));
    }
    frames.push(frame(
        1_282,
        EventKind::Checkpoint,
        conversation,
        EventPayload::Checkpoint(Box::new(Checkpoint {
            cursor: encode_checkpoint_cursor(turn, b"deep-blob-two").expect("cursor"),
        })),
        Authority::RuntimeFact,
        1_282,
    ));
    for item in &frames {
        apply_timeline(&store, item).expect("timeline frame");
    }
    let snapshot = store.begin_snapshot().expect("snapshot");
    let latest = latest_checkpoint(&snapshot, turn)
        .expect("latest scan")
        .expect("checkpoint");
    assert_eq!(latest.lsn, LSN::new(1_282));
    assert_eq!(latest.blob, b"deep-blob-two");
}

fn requirement(
    entity: &str,
    property: &str,
    revision: Option<&[u8]>,
    freshness_requirement_ns: Option<u64>,
) -> BindingRequirement {
    BindingRequirement {
        canonical_entity: entity.to_owned(),
        property: property.to_owned(),
        revision: revision.map(<[u8]>::to_vec),
        freshness_requirement_ns,
    }
}

fn canonical_dumps(store: &ProjectionStore) -> [Vec<u8>; 3] {
    let snapshot = store.begin_snapshot().expect("dump snapshot");
    [
        snapshot
            .canonical_dump(ProjectionId::IntentFrame)
            .expect("intent dump"),
        snapshot
            .canonical_dump(ProjectionId::WorkLedger)
            .expect("ledger dump"),
        snapshot
            .canonical_dump(ProjectionId::Bindings)
            .expect("bindings dump"),
    ]
}

#[test]
fn transition_states_match_donor_vectors() {
    assert_eq!(WorkState::Dispatched as u8, 0);
    assert_eq!(WorkState::Committed as u8, 1);
    assert_eq!(WorkState::Returned as u8, 2);
    assert_eq!(WorkState::OutcomeUnknown as u8, 3);
}
