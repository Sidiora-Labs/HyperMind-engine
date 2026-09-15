use hm_core::{ActorId, ConversationId, Error, ErrorCode, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_ledger::idempotency::{
    Admission, BatchEvent, BatchIdentity, DedupTable, rollback_torn_batch,
};
use hm_ledger::keyring::{EntropySource, KeyHierarchy};
use hm_ledger::segment::{AppendRequest, SegmentLog, SegmentLogOptions, WriteBackend};
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{Authority, EventEnvelope, EventPayload, Retention, Sensitivity, UserMsg};

const ACTOR: ActorId = ActorId::new(17);
const CONNECTION: [u8; 16] = [0x63; 16];

struct DeterministicEntropy(u64);

impl EntropySource for DeterministicEntropy {
    fn fill(&mut self, destination: &mut [u8]) -> Result<(), Error> {
        for byte in destination {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            *byte = self.0.to_le_bytes()[0];
        }
        Ok(())
    }
}

fn options() -> SegmentLogOptions {
    SegmentLogOptions {
        segment_bytes: 64 * 1024,
        maximum_frame_bytes: 16 * 1024,
        direct_alignment: 512,
        maximum_io_bytes: 13,
        backend: WriteBackend::Pwrite,
    }
}

fn event(seq: u64, index: u32, count: u32, content: &str) -> Vec<u8> {
    encode_event_envelope(&EventEnvelope {
        schema_version: CURRENT_SCHEMA_VERSION,
        payload: EventPayload::UserMsg(Box::new(UserMsg {
            content: content.as_bytes().to_vec(),
        })),
        connection_id: Some(CONNECTION.to_vec()),
        client_seq: seq,
        client_event_index: index,
        client_event_count: count,
        origin_actor: 0,
        run_id: None,
        model_provenance: None,
        authority: Authority::UserAsserted,
        retention: Retention::Durable,
        sensitivity: Sensitivity::Personal,
        event_time_ns: 0,
    })
}

fn batch_payloads(seq: u64, contents: &[&str]) -> Vec<Vec<u8>> {
    contents
        .iter()
        .enumerate()
        .map(|(index, content)| {
            event(
                seq,
                u32::try_from(index).expect("event index"),
                u32::try_from(contents.len()).expect("event count"),
                content,
            )
        })
        .collect()
}

fn identity<'payload>(
    seq: u64,
    payloads: &'payload [Vec<u8>],
    events: &'payload mut Vec<BatchEvent<'payload>>,
) -> BatchIdentity<'payload> {
    events.extend(
        payloads
            .iter()
            .enumerate()
            .map(|(index, payload)| BatchEvent {
                kind: EventKind::UserMsg,
                conversation: ConversationId::derive(&format!("idempotency-{seq}-{index}")),
                plaintext_payload: payload,
            }),
    );
    BatchIdentity {
        connection_id: CONNECTION,
        client_seq: seq,
        events,
    }
}

fn append_plaintext_batch(
    log: &mut SegmentLog,
    keys: &KeyHierarchy,
    entropy: &mut DeterministicEntropy,
    events: &[BatchEvent<'_>],
) -> (LSN, LSN) {
    let first = log.next_lsn();
    let requests: Vec<_> = events
        .iter()
        .enumerate()
        .map(|(index, event)| {
            let header = FrameHeader {
                lsn: LSN::new(first.get() + u64::try_from(index).expect("LSN offset")),
                kind: event.kind,
                wall_timestamp_ns: UtcNanos::new(
                    i64::try_from(first.get()).expect("timestamp") * 1_000
                        + i64::try_from(index).expect("timestamp offset"),
                ),
                actor: ACTOR,
                conversation: event.conversation,
            };
            AppendRequest {
                kind: event.kind,
                wall_timestamp_ns: header.wall_timestamp_ns,
                conversation: event.conversation,
                sealed_payload: keys
                    .seal(&header, event.plaintext_payload, entropy)
                    .expect("seal event"),
            }
        })
        .collect();
    let committed = log.append_batch(&requests).expect("append batch");
    (committed.first_lsn, committed.last_lsn)
}

fn plaintext_frames(log: &SegmentLog, keys: &KeyHierarchy) -> Vec<Frame> {
    log.read_all()
        .expect("read log")
        .into_iter()
        .map(|mut frame| {
            frame.sealed_payload = keys
                .unseal(&frame.header, &frame.sealed_payload)
                .expect("unseal frame");
            frame
        })
        .collect()
}

#[test]
fn exact_replay_is_duplicate_and_sequence_is_strict_after_rebuild() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let mut entropy = DeterministicEntropy(0x91);
    let keys = KeyHierarchy::open_or_create(
        temporary.path(),
        ACTOR,
        [0x11; 16],
        &[0x22; 32],
        &mut entropy,
        true,
    )
    .expect("key hierarchy");
    let mut log = SegmentLog::open(temporary.path(), ACTOR, options()).expect("segment log");
    let first_payloads = batch_payloads(1, &["alpha", "beta"]);
    let mut first_events = Vec::new();
    let first = identity(1, &first_payloads, &mut first_events);
    assert!(matches!(
        DedupTable::default()
            .admit(&first)
            .expect("first admission"),
        Admission::Fresh { .. }
    ));
    let mut dedup = DedupTable::default();
    let (first_lsn, last_lsn) = append_plaintext_batch(&mut log, &keys, &mut entropy, first.events);
    let recorded = dedup
        .record(&first, first_lsn, last_lsn)
        .expect("record first request");
    assert_eq!(
        dedup.admit(&first).expect("exact replay"),
        Admission::Duplicate(recorded)
    );

    let changed_payloads = batch_payloads(1, &["alpha", "changed"]);
    let mut changed_events = Vec::new();
    let changed = identity(1, &changed_payloads, &mut changed_events);
    assert_eq!(
        dedup.admit(&changed).expect_err("digest conflict").code,
        ErrorCode::IdempotencyConflict
    );
    let skipped_payloads = batch_payloads(3, &["skipped"]);
    let mut skipped_events = Vec::new();
    let skipped = identity(3, &skipped_payloads, &mut skipped_events);
    assert_eq!(
        dedup.admit(&skipped).expect_err("sequence gap").code,
        ErrorCode::SequenceViolation
    );

    let second_payloads = batch_payloads(2, &["gamma"]);
    let mut second_events = Vec::new();
    let second = identity(2, &second_payloads, &mut second_events);
    let (first_lsn, last_lsn) =
        append_plaintext_batch(&mut log, &keys, &mut entropy, second.events);
    let second_state = dedup
        .record(&second, first_lsn, last_lsn)
        .expect("record second request");
    drop(dedup);
    drop(log);

    let reopened = SegmentLog::open(temporary.path(), ACTOR, options()).expect("reopen log");
    let rebuilt = DedupTable::rebuild(&plaintext_frames(&reopened, &keys)).expect("rebuild dedup");
    assert_eq!(rebuilt.len(), 1);
    assert_eq!(rebuilt.get(&CONNECTION), Some(&second_state));
    assert_eq!(
        rebuilt.admit(&second).expect("replay after rebuild"),
        Admission::Duplicate(second_state)
    );
}

#[test]
fn open_rolls_back_every_durable_member_of_a_torn_multi_event_batch() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let mut entropy = DeterministicEntropy(0x37);
    let keys = KeyHierarchy::open_or_create(
        temporary.path(),
        ACTOR,
        [0x11; 16],
        &[0x22; 32],
        &mut entropy,
        true,
    )
    .expect("key hierarchy");
    let mut log = SegmentLog::open(temporary.path(), ACTOR, options()).expect("segment log");

    let complete_payloads = batch_payloads(1, &["complete"]);
    let mut complete_events = Vec::new();
    let complete = identity(1, &complete_payloads, &mut complete_events);
    append_plaintext_batch(&mut log, &keys, &mut entropy, complete.events);

    let torn_payloads = batch_payloads(2, &["torn-0", "torn-1", "never-written"]);
    let mut torn_events = Vec::new();
    let torn = identity(2, &torn_payloads, &mut torn_events);
    append_plaintext_batch(&mut log, &keys, &mut entropy, &torn.events[..2]);
    drop(log);

    let mut reopened = SegmentLog::open(temporary.path(), ACTOR, options()).expect("reopen log");
    assert_eq!(
        rollback_torn_batch(&mut reopened, &keys).expect("rollback torn batch"),
        Some(LSN::new(1))
    );
    assert_eq!(reopened.next_lsn(), LSN::new(2));
    let frames = plaintext_frames(&reopened, &keys);
    assert_eq!(frames.len(), 1);
    let rebuilt = DedupTable::rebuild(&frames).expect("rebuild after rollback");
    assert_eq!(
        rebuilt.get(&CONNECTION).expect("first request").client_seq,
        1
    );
}
