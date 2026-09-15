use hm_core::{ActorId, ConversationId, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_proj::ladder::{TemporalLadder, TemporalLevel};
use hm_proj::store::{Mutation, ProjectionId, ProjectionStore};

const MINUTE: i64 = 60_000_000_000;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;
const WEEK: i64 = 7 * DAY;

fn frames() -> Vec<Frame> {
    (0_u64..256)
        .map(|index| Frame {
            header: FrameHeader {
                lsn: LSN::new(index + 1),
                kind: EventKind::UserMsg,
                wall_timestamp_ns: UtcNanos::new(
                    -2 * DAY + i64::try_from(index).expect("index") * 37 * MINUTE,
                ),
                actor: ActorId::new(91),
                conversation: ConversationId::new([0; 16]),
            },
            sealed_payload: Vec::new(),
        })
        .collect()
}

#[test]
fn ladder_resumes_descends_and_rebuilds_byte_identically() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 32 * 1024 * 1024).expect("store");
    let frames = frames();

    let partial = TemporalLadder::rebuild(&store, &frames, false, 73).expect("partial rebuild");
    assert!(!partial.complete);
    assert_eq!(partial.applied_lsn, LSN::new(73));
    let resumed = TemporalLadder::rebuild(&store, &frames, false, usize::MAX).expect("resume");
    assert!(resumed.complete);
    assert_eq!(resumed.applied_lsn, LSN::new(256));

    let snapshot = store.begin_snapshot().expect("snapshot");
    let weeks = TemporalLadder::list_windows(&snapshot, TemporalLevel::Week, -WEEK, 2 * WEEK, 1024)
        .expect("weeks");
    let days = TemporalLadder::list_windows(&snapshot, TemporalLevel::Day, -3 * DAY, 6 * DAY, 1024)
        .expect("days");
    let hours = TemporalLadder::list_windows(&snapshot, TemporalLevel::Hour, -2 * DAY, -DAY, 1024)
        .expect("hours");
    let minutes = TemporalLadder::list_windows(
        &snapshot,
        TemporalLevel::Minute,
        -2 * DAY,
        -2 * DAY + 4 * HOUR,
        1024,
    )
    .expect("minutes");
    assert!(!weeks.is_empty());
    assert!(!days.is_empty());
    assert!(!hours.is_empty());
    assert!(!minutes.is_empty());
    assert!(
        days.windows(2)
            .all(|pair| pair[0].start_ns < pair[1].start_ns)
    );

    let child_days = TemporalLadder::open_window(&snapshot, weeks[0], 1024).expect("week to day");
    assert!(!child_days.is_empty());
    assert!(child_days.iter().all(|child| {
        child.level == TemporalLevel::Day
            && child.start_ns >= weeks[0].start_ns
            && child.end_ns <= weeks[0].end_ns
    }));
    let child_hours =
        TemporalLadder::open_window(&snapshot, child_days[0], 1024).expect("day to hour");
    let child_minutes =
        TemporalLadder::open_window(&snapshot, child_hours[0], 1024).expect("hour to minute");
    assert!(
        TemporalLadder::open_window(&snapshot, child_minutes[0], 1024)
            .expect("minute is terminal")
            .is_empty()
    );
    let members =
        TemporalLadder::resolve_members(&snapshot, child_minutes[0], 4096).expect("members");
    assert!(!members.is_empty());
    assert!(members.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(members.len() as u64, child_minutes[0].member_count);
    assert_eq!(
        TemporalLadder::resolve_members(&snapshot, child_minutes[0], 1).expect("limited members"),
        vec![members[0]]
    );
    drop(snapshot);

    let original = store
        .begin_snapshot()
        .expect("snapshot")
        .canonical_dump(ProjectionId::TemporalLadder)
        .expect("dump");
    TemporalLadder::rebuild(&store, &frames, true, usize::MAX).expect("clean rebuild");
    let rebuilt = store
        .begin_snapshot()
        .expect("snapshot")
        .canonical_dump(ProjectionId::TemporalLadder)
        .expect("rebuilt dump");
    assert_eq!(original, rebuilt);

    store
        .apply(
            ProjectionId::TemporalLadder,
            LSN::new(257),
            &[Mutation::put([0x7f, 0x01], [0x55])],
        )
        .expect("pollute");
    TemporalLadder::rebuild(&store, &frames, true, usize::MAX).expect("repair");
    let repaired = store
        .begin_snapshot()
        .expect("snapshot")
        .canonical_dump(ProjectionId::TemporalLadder)
        .expect("repaired dump");
    assert_eq!(original, repaired);
}

#[test]
fn ladder_range_boundaries_are_explicit() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).expect("store");
    TemporalLadder::rebuild(&store, &frames()[..4], false, usize::MAX).expect("rebuild");
    let snapshot = store.begin_snapshot().expect("snapshot");
    assert!(TemporalLadder::list_windows(&snapshot, TemporalLevel::Minute, 7, 7, 10).is_err());
    assert!(
        TemporalLadder::list_windows(&snapshot, TemporalLevel::Minute, 100 * WEEK, 101 * WEEK, 10,)
            .expect("absent range")
            .is_empty()
    );
}
