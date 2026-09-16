#![forbid(unsafe_code)]

use hm_core::{ActorId, ConversationId, ErrorCode, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_proj::attestations::{
    AttestationsProjection, MAXIMUM_PREFERENCE_TARGETS, PREFERENCE_MAXIMUM_Q16,
    PREFERENCE_MINIMUM_Q16, PREFERENCE_NEUTRAL_Q16,
};
use hm_proj::rebuild::rebuild_projection_stream;
use hm_proj::store::{ProjectionId, ProjectionStore};
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    Attestation, AttestationDisposition, Authority, EventEnvelope, EventPayload, Retention,
    Sensitivity,
};

const HELPFUL_AFTER_ONE_Q16: u32 = 68_812;
const HARMFUL_AFTER_ONE_Q16: u32 = 62_259;

#[test]
fn helpful_and_harmful_move_the_weight_in_opposite_bounded_directions() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).expect("store");
    rebuild_projection_stream(&store, &mixed_feedback(), false, usize::MAX).expect("apply");
    let snapshot = store.begin_snapshot().expect("snapshot");

    let helpful = AttestationsProjection::preference(&snapshot, LSN::new(3))
        .expect("helpful preference")
        .expect("helpful record");
    assert_eq!(helpful.target_lsn, 3);
    assert_eq!(helpful.weight_q16, HELPFUL_AFTER_ONE_Q16);
    assert!(helpful.weight_q16 > PREFERENCE_NEUTRAL_Q16);
    assert!(helpful.weight_q16 <= PREFERENCE_MAXIMUM_Q16);
    assert_eq!(helpful.observations, 1);
    assert_eq!(helpful.last_attestation_lsn, 1);

    let harmful = AttestationsProjection::preference(&snapshot, LSN::new(4))
        .expect("harmful preference")
        .expect("harmful record");
    assert_eq!(harmful.weight_q16, HARMFUL_AFTER_ONE_Q16);
    assert!(harmful.weight_q16 < PREFERENCE_NEUTRAL_Q16);
    assert!(harmful.weight_q16 >= PREFERENCE_MINIMUM_Q16);
    assert_eq!(harmful.last_attestation_lsn, 2);

    assert_eq!(
        PREFERENCE_NEUTRAL_Q16 - HARMFUL_AFTER_ONE_Q16,
        HELPFUL_AFTER_ONE_Q16 - PREFERENCE_NEUTRAL_Q16 + 1
    );

    let mixed = AttestationsProjection::preference(&snapshot, LSN::new(5))
        .expect("mixed preference")
        .expect("mixed record");
    assert_eq!(mixed.observations, 2);
    assert!(mixed.weight_q16 < PREFERENCE_NEUTRAL_Q16);

    assert_eq!(
        AttestationsProjection::preferences(
            &snapshot,
            &[LSN::new(3), LSN::new(4), LSN::new(5), LSN::new(9)]
        )
        .expect("bounded preferences"),
        [
            (3_u64, HELPFUL_AFTER_ONE_Q16),
            (4, HARMFUL_AFTER_ONE_Q16),
            (5, mixed.weight_q16),
        ]
        .into_iter()
        .collect()
    );
}

#[test]
fn repeated_feedback_saturates_inside_the_band() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).expect("store");
    let mut frames = Vec::new();
    for _ in 0..64 {
        push(&mut frames, 100, AttestationDisposition::Helpful);
    }
    rebuild_projection_stream(&store, &frames, false, usize::MAX).expect("apply");
    let snapshot = store.begin_snapshot().expect("snapshot");
    let saturated = AttestationsProjection::preference(&snapshot, LSN::new(100))
        .expect("saturated preference")
        .expect("saturated record");
    assert!(saturated.weight_q16 <= PREFERENCE_MAXIMUM_Q16);
    assert!(saturated.weight_q16 > HELPFUL_AFTER_ONE_Q16);
    assert_eq!(saturated.observations, 64);
    drop(snapshot);

    let other = tempfile::tempdir().expect("temporary directory");
    let punished = ProjectionStore::open(other.path(), 16 * 1024 * 1024).expect("store");
    let mut frames = Vec::new();
    for _ in 0..64 {
        push(&mut frames, 100, AttestationDisposition::Harmful);
    }
    rebuild_projection_stream(&punished, &frames, false, usize::MAX).expect("apply");
    let snapshot = punished.begin_snapshot().expect("snapshot");
    let floored = AttestationsProjection::preference(&snapshot, LSN::new(100))
        .expect("floored preference")
        .expect("floored record");
    assert!(floored.weight_q16 >= PREFERENCE_MINIMUM_Q16);
    assert!(floored.weight_q16 < HARMFUL_AFTER_ONE_Q16);
    assert_eq!(floored.observations, 64);
}

#[test]
fn unattested_targets_have_no_preference_record() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).expect("store");
    rebuild_projection_stream(&store, &mixed_feedback(), false, usize::MAX).expect("apply");
    let snapshot = store.begin_snapshot().expect("snapshot");
    assert!(
        AttestationsProjection::preference(&snapshot, LSN::new(9))
            .expect("unattested preference")
            .is_none()
    );
    assert!(
        AttestationsProjection::preferences(&snapshot, &[LSN::new(9), LSN::new(11)])
            .expect("unattested preferences")
            .is_empty()
    );
}

#[test]
fn counters_are_unchanged_by_the_preference_write() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).expect("store");
    rebuild_projection_stream(&store, &mixed_feedback(), false, usize::MAX).expect("apply");
    let snapshot = store.begin_snapshot().expect("snapshot");

    let helpful = AttestationsProjection::get(&snapshot, LSN::new(3))
        .expect("helpful counters")
        .expect("helpful record");
    assert_eq!(helpful.target_lsn, 3);
    assert_eq!(
        (
            helpful.used,
            helpful.ignored,
            helpful.helpful,
            helpful.harmful
        ),
        (0, 0, 1, 0)
    );
    assert_eq!(helpful.last_attestation_lsn, 1);

    let harmful = AttestationsProjection::get(&snapshot, LSN::new(4))
        .expect("harmful counters")
        .expect("harmful record");
    assert_eq!(
        (
            harmful.used,
            harmful.ignored,
            harmful.helpful,
            harmful.harmful
        ),
        (0, 0, 0, 1)
    );
    assert_eq!(harmful.last_attestation_lsn, 2);

    let mixed = AttestationsProjection::get(&snapshot, LSN::new(5))
        .expect("mixed counters")
        .expect("mixed record");
    assert_eq!(
        (mixed.used, mixed.ignored, mixed.helpful, mixed.harmful),
        (1, 1, 0, 0)
    );
    assert_eq!(mixed.last_attestation_lsn, 4);
    assert!(
        AttestationsProjection::get(&snapshot, LSN::new(9))
            .expect("unattested counters")
            .is_none()
    );
}

#[test]
fn preference_weights_are_byte_identical_after_a_reset_rebuild() {
    let frames = mixed_feedback();
    let forward_root = tempfile::tempdir().expect("temporary directory");
    let forward = ProjectionStore::open(forward_root.path(), 16 * 1024 * 1024).expect("store");
    rebuild_projection_stream(&forward, &frames, false, usize::MAX).expect("forward apply");
    let snapshot = forward.begin_snapshot().expect("snapshot");
    let expected = snapshot
        .canonical_dump(ProjectionId::Attestations)
        .expect("forward dump");
    drop(snapshot);

    let rebuilt_root = tempfile::tempdir().expect("temporary directory");
    let rebuilt = ProjectionStore::open(rebuilt_root.path(), 16 * 1024 * 1024).expect("store");
    rebuild_projection_stream(&rebuilt, &frames, false, 2).expect("partial apply");
    rebuild_projection_stream(&rebuilt, &frames, true, usize::MAX).expect("reset rebuild");
    let snapshot = rebuilt.begin_snapshot().expect("snapshot");
    assert_eq!(
        snapshot
            .canonical_dump(ProjectionId::Attestations)
            .expect("rebuilt dump"),
        expected
    );
}

#[test]
fn feedback_writes_no_belief_memory_or_graph_entry() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).expect("store");
    rebuild_projection_stream(&store, &mixed_feedback(), false, usize::MAX).expect("apply");
    let snapshot = store.begin_snapshot().expect("snapshot");
    for projection in [
        ProjectionId::BeliefStore,
        ProjectionId::Memories,
        ProjectionId::Graph,
    ] {
        assert_eq!(
            snapshot
                .canonical_dump(projection)
                .expect("fact dump")
                .len(),
            8,
            "{} must stay empty",
            projection.name()
        );
    }
}

#[test]
fn recent_preferences_is_bounded_and_ordered() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).expect("store");
    rebuild_projection_stream(&store, &mixed_feedback(), false, usize::MAX).expect("apply");
    let snapshot = store.begin_snapshot().expect("snapshot");
    assert_eq!(
        AttestationsProjection::recent_preferences(&snapshot, 0)
            .expect_err("zero limit")
            .code,
        ErrorCode::InvalidArgument
    );
    assert_eq!(
        AttestationsProjection::recent_preferences(&snapshot, MAXIMUM_PREFERENCE_TARGETS + 1)
            .expect_err("oversized limit")
            .code,
        ErrorCode::InvalidArgument
    );
    let recent =
        AttestationsProjection::recent_preferences(&snapshot, 2).expect("recent preferences");
    assert_eq!(
        recent
            .iter()
            .map(|weight| weight.target_lsn)
            .collect::<Vec<_>>(),
        vec![5, 4]
    );
    assert_eq!(
        AttestationsProjection::recent_preferences(&snapshot, MAXIMUM_PREFERENCE_TARGETS)
            .expect("all preferences")
            .len(),
        3
    );
}

#[test]
fn preferences_rejects_zero_lsn_and_oversized_target_sets() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).expect("store");
    rebuild_projection_stream(&store, &mixed_feedback(), false, usize::MAX).expect("apply");
    let snapshot = store.begin_snapshot().expect("snapshot");
    assert_eq!(
        AttestationsProjection::preferences(&snapshot, &[LSN::new(3), LSN::new(0)])
            .expect_err("zero target")
            .code,
        ErrorCode::InvalidArgument
    );
    let oversized = (1..=u64::try_from(MAXIMUM_PREFERENCE_TARGETS).expect("bound") + 1)
        .map(LSN::new)
        .collect::<Vec<_>>();
    assert_eq!(
        AttestationsProjection::preferences(&snapshot, &oversized)
            .expect_err("oversized target set")
            .code,
        ErrorCode::CapacityExceeded
    );
    assert_eq!(
        AttestationsProjection::preferences(
            &snapshot,
            &oversized[..MAXIMUM_PREFERENCE_TARGETS.min(oversized.len() - 1)]
        )
        .expect("bounded target set")
        .len(),
        3
    );
}

fn mixed_feedback() -> Vec<Frame> {
    let mut frames = Vec::new();
    push(&mut frames, 3, AttestationDisposition::Helpful);
    push(&mut frames, 4, AttestationDisposition::Harmful);
    push(&mut frames, 5, AttestationDisposition::Used);
    push(&mut frames, 5, AttestationDisposition::Ignored);
    frames
}

fn push(frames: &mut Vec<Frame>, target_lsn: u64, disposition: AttestationDisposition) {
    let lsn = u64::try_from(frames.len()).expect("frame index") + 1;
    frames.push(Frame {
        header: FrameHeader {
            lsn: LSN::new(lsn),
            kind: EventKind::Attestation,
            wall_timestamp_ns: UtcNanos::new(
                5_000_000 + i64::try_from(lsn).expect("wall timestamp"),
            ),
            actor: ActorId::new(41),
            conversation: ConversationId::new([0x4d; 16]),
        },
        sealed_payload: encode_event_envelope(&EventEnvelope {
            schema_version: CURRENT_SCHEMA_VERSION,
            payload: EventPayload::Attestation(Box::new(Attestation {
                target_lsn,
                disposition,
            })),
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 0,
            origin_actor: 41,
            run_id: None,
            model_provenance: None,
            authority: Authority::RuntimeFact,
            retention: Retention::Durable,
            sensitivity: Sensitivity::Personal,
            event_time_ns: i64::try_from(lsn).expect("event time"),
        }),
    });
}
