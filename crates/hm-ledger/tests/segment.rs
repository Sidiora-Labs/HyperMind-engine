use hm_core::{ActorId, ConversationId, ErrorCode, LSN, UtcNanos};
use hm_ledger::frame::{self, EventKind, Frame, FrameHeader};
use hm_ledger::segment::{AppendRequest, SegmentLog, SegmentLogOptions, WriteBackend};
use std::fs::{self, OpenOptions};
use std::process::Command;

fn options() -> SegmentLogOptions {
    SegmentLogOptions {
        segment_bytes: 4096,
        maximum_frame_bytes: 1024,
        direct_alignment: 512,
        maximum_io_bytes: 3,
        backend: WriteBackend::Pwrite,
    }
}

fn conversation(first: u8) -> ConversationId {
    let mut bytes = [0_u8; 16];
    bytes[0] = first;
    ConversationId::new(bytes)
}

fn request(kind: EventKind, timestamp: i64, first: u8, payload: &[u8]) -> AppendRequest {
    AppendRequest {
        kind,
        wall_timestamp_ns: UtcNanos::new(timestamp),
        conversation: conversation(first),
        sealed_payload: payload.to_vec(),
    }
}

fn first_segment(actor: &std::path::Path) -> std::path::PathBuf {
    actor.join("log/00000000000000000001.seg")
}

#[test]
fn donor_round_trip_group_commit_and_manifest_rebuild() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let actor = temporary.path().join("actor-7");
    let mut log = SegmentLog::open(&actor, ActorId::new(7), options()).expect("open");
    let requests = [
        request(EventKind::UserMsg, 10, 1, b"one"),
        request(EventKind::ToolCall, 11, 1, b"two"),
        request(EventKind::ToolResult, 12, 1, b"three"),
    ];
    let commit = log.append_batch(&requests).expect("append batch");
    assert_eq!(commit.first_lsn, LSN::new(1));
    assert_eq!(commit.last_lsn, LSN::new(3));
    assert_eq!(commit.backend, WriteBackend::Pwrite);
    let frames = log.read_all().expect("read all");
    assert_eq!(frames.len(), requests.len());
    for (index, frame) in frames.iter().enumerate() {
        assert_eq!(frame.header.lsn, LSN::new(index as u64 + 1));
        assert_eq!(frame.header.actor, ActorId::new(7));
        assert_eq!(frame.header.kind, requests[index].kind);
        assert_eq!(frame.sealed_payload, requests[index].sealed_payload);
    }
    assert_eq!(
        log.read_from(LSN::new(2), 1).expect("range")[0].header.lsn,
        LSN::new(2)
    );
    assert!(
        log.read_from(LSN::new(4), 1)
            .expect("empty tail")
            .is_empty()
    );
    assert_eq!(
        log.read_from(LSN::new(0), 1)
            .expect_err("zero first LSN")
            .code,
        ErrorCode::InvalidArgument
    );
    assert_eq!(
        log.read_from(LSN::new(1), 0).expect_err("zero limit").code,
        ErrorCode::InvalidArgument
    );
    assert_eq!(
        &fs::read(actor.join("log/MANIFEST")).expect("manifest")[..4],
        b"NCMF"
    );

    let reopened = SegmentLog::open(&actor, ActorId::new(7), options()).expect("clean reopen");
    assert_eq!(reopened.next_lsn(), LSN::new(4));
    assert!(!reopened.recovery_report().rebuilt_manifest);
    let manifest = actor.join("log/MANIFEST");
    let mut bytes = fs::read(&manifest).expect("read manifest");
    bytes[0] = 0xff;
    fs::write(&manifest, bytes).expect("corrupt manifest");
    OpenOptions::new()
        .write(true)
        .open(&manifest)
        .expect("open manifest")
        .sync_data()
        .expect("sync manifest");
    let rebuilt = SegmentLog::open(&actor, ActorId::new(7), options()).expect("rebuild manifest");
    assert!(rebuilt.recovery_report().rebuilt_manifest);
    assert_eq!(rebuilt.next_lsn(), LSN::new(4));
}

#[test]
fn donor_torn_tail_is_truncated_on_open() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let actor = temporary.path().join("actor-7");
    let mut log = SegmentLog::open(&actor, ActorId::new(7), options()).expect("open");
    let requests = [
        request(EventKind::UserMsg, 10, 2, b"durable"),
        request(EventKind::DeliveredMsg, 11, 2, b"torn-tail"),
    ];
    log.append_batch(&requests).expect("append");
    let first = Frame {
        header: FrameHeader {
            lsn: LSN::new(1),
            kind: requests[0].kind,
            wall_timestamp_ns: requests[0].wall_timestamp_ns,
            actor: ActorId::new(7),
            conversation: requests[0].conversation,
        },
        sealed_payload: requests[0].sealed_payload.clone(),
    };
    let first_length = frame::encode(&first).expect("encode first").len();
    let segment = OpenOptions::new()
        .write(true)
        .open(first_segment(&actor))
        .expect("open segment");
    segment
        .set_len((first_length + 7) as u64)
        .expect("tear tail");
    segment.sync_data().expect("sync tear");

    let recovered = SegmentLog::open(&actor, ActorId::new(7), options()).expect("recover");
    assert!(recovered.recovery_report().truncated_torn_tail);
    assert_eq!(recovered.recovery_report().recovered_tail_lsn, LSN::new(2));
    assert_eq!(recovered.next_lsn(), LSN::new(2));
    let frames = recovered.read_all().expect("read recovered");
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].header.lsn, LSN::new(1));
}

#[test]
fn donor_interior_corruption_is_refused() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let actor = temporary.path().join("actor-7");
    let mut log = SegmentLog::open(&actor, ActorId::new(7), options()).expect("open");
    log.append_batch(&[
        request(EventKind::UserMsg, 20, 3, b"first"),
        request(EventKind::DeliveredMsg, 21, 3, b"second"),
    ])
    .expect("append");
    let path = first_segment(&actor);
    let mut bytes = fs::read(&path).expect("read segment");
    bytes[43] ^= 0x80;
    fs::write(&path, bytes).expect("corrupt segment");
    assert_eq!(
        SegmentLog::open(&actor, ActorId::new(7), options())
            .err()
            .expect("interior corruption")
            .code,
        ErrorCode::InteriorCorruption
    );
}

#[test]
fn property_rotation_reopens_to_identical_frames() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let actor = temporary.path().join("actor-19");
    let mut log = SegmentLog::open(&actor, ActorId::new(19), options()).expect("open");
    let mut expected = Vec::new();
    let mut state = 0x9e37_79b9_7f4a_7c15_u64;
    for batch in 0..96_u64 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let count = usize::try_from(state % 5 + 1).expect("batch count");
        let mut requests = Vec::with_capacity(count);
        for item in 0..count {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let payload = vec![
                state.to_le_bytes()[0];
                usize::try_from(state % 400 + 1).expect("payload length")
            ];
            let kind_number =
                u8::try_from((batch + u64::try_from(item).expect("item index")) % 21 + 1)
                    .expect("kind byte");
            let kind = EventKind::try_from(kind_number).expect("kind");
            let value = request(
                kind,
                i64::try_from(batch).expect("batch timestamp"),
                batch.to_le_bytes()[0],
                &payload,
            );
            expected.push(value.clone());
            requests.push(value);
        }
        log.append_batch(&requests).expect("property append");
    }
    drop(log);
    let reopened = SegmentLog::open(&actor, ActorId::new(19), options()).expect("property reopen");
    let actual = reopened.read_all().expect("property read");
    assert_eq!(actual.len(), expected.len());
    for (frame, request) in actual.iter().zip(expected) {
        assert_eq!(frame.header.kind, request.kind);
        assert_eq!(frame.header.wall_timestamp_ns, request.wall_timestamp_ns);
        assert_eq!(frame.header.conversation, request.conversation);
        assert_eq!(frame.sealed_payload, request.sealed_payload);
    }
}

#[test]
fn committed_batch_survives_real_sigkill() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let actor = temporary.path().join("actor-31");
    let status = Command::new(std::env::current_exe().expect("test executable"))
        .arg("--exact")
        .arg("sigkill_commit_child")
        .arg("--nocapture")
        .env("HM_SIGKILL_ACTOR", &actor)
        .status()
        .expect("run child");
    assert!(!status.success());
    let reopened =
        SegmentLog::open(&actor, ActorId::new(31), options()).expect("reopen after kill");
    assert_eq!(reopened.next_lsn(), LSN::new(3));
    assert_eq!(reopened.read_all().expect("read after kill").len(), 2);
}

#[test]
fn sigkill_commit_child() {
    let Ok(actor) = std::env::var("HM_SIGKILL_ACTOR") else {
        return;
    };
    let mut log = SegmentLog::open(actor, ActorId::new(31), options()).expect("child open");
    log.append_batch(&[
        request(EventKind::UserMsg, 1, 1, b"before-kill"),
        request(EventKind::ToolCall, 2, 1, b"durable-effect"),
    ])
    .expect("child commit");
    let status = Command::new("kill")
        .arg("-9")
        .arg(std::process::id().to_string())
        .status();
    panic!("SIGKILL failed: {status:?}");
}
