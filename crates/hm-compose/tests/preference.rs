mod common;

use hm_compose::bundle::{
    ActivationRequest, AttestationRequest, RetrievalLane, activate, build_attestations,
};
use hm_compose::fusion::{
    FusedHit, LaneRanking, Q16_ONE, RankedCandidate, fuse, fuse_with_preferences,
};
use hm_compose::preference::{PreferenceProfile, adjust_q32};
use hm_compose::tokens::{FallbackWeights, TokenCounter};
use hm_core::{ActorId, ConversationId, ErrorCode, LSN, UtcNanos};
use hm_proj::attestations::{
    AttestationsProjection, MAXIMUM_PREFERENCE_TARGETS, PREFERENCE_MAXIMUM_Q16,
    PREFERENCE_MINIMUM_Q16, PREFERENCE_NEUTRAL_Q16,
};
use hm_proj::rebuild::rebuild_projection_stream;
use hm_proj::store::ProjectionStore;
use hm_schema::events::AttestationDisposition;
use tempfile::TempDir;

const ACTOR: ActorId = ActorId::new(19);
const ATTESTED_LSN: u64 = 3;
const UNATTESTED_LSN: u64 = 9;
const SAMPLE_SCORE_Q32: u64 = 1_000_000_000;

#[test]
fn neutral_profile_is_byte_identical_to_plain_fusion() {
    let feedback = Feedback::build(AttestationDisposition::Helpful);
    let plain = fuse(ACTOR, feedback.conversation, &rankings(), 8).expect("plain fusion");
    let neutral = fuse_with_preferences(
        ACTOR,
        feedback.conversation,
        &rankings(),
        8,
        &PreferenceProfile::neutral(),
    )
    .expect("neutral fusion");
    assert_eq!(plain, neutral);
    assert_eq!(plain.len(), 2);
    for hit in &plain {
        assert_eq!(hit.preference_q16, Q16_ONE);
        assert!(!hit.uri.contains("pref="), "{}", hit.uri);
    }

    let snapshot = feedback.store.begin_snapshot().expect("snapshot");
    let unattested = PreferenceProfile::load(&snapshot, &[LSN::new(UNATTESTED_LSN)])
        .expect("unattested profile");
    assert!(unattested.is_neutral());
    assert_eq!(
        fuse_with_preferences(ACTOR, feedback.conversation, &rankings(), 8, &unattested)
            .expect("unattested fusion"),
        plain
    );
}

#[test]
fn helpful_feedback_raises_only_its_own_evidence() {
    let feedback = Feedback::build(AttestationDisposition::Helpful);
    let snapshot = feedback.store.begin_snapshot().expect("snapshot");
    let weight = AttestationsProjection::preference(&snapshot, LSN::new(ATTESTED_LSN))
        .expect("stored preference")
        .expect("preference record")
        .weight_q16;
    assert!(weight > PREFERENCE_NEUTRAL_Q16);
    assert!(weight <= PREFERENCE_MAXIMUM_Q16);
    let profile = PreferenceProfile::load(
        &snapshot,
        &[LSN::new(ATTESTED_LSN), LSN::new(UNATTESTED_LSN)],
    )
    .expect("profile");
    assert!(!profile.is_neutral());
    assert_eq!(profile.weight_q16(LSN::new(ATTESTED_LSN)), weight);
    assert_eq!(
        profile.weight_q16(LSN::new(UNATTESTED_LSN)),
        PREFERENCE_NEUTRAL_Q16
    );

    let plain = fuse(ACTOR, feedback.conversation, &rankings(), 8).expect("plain fusion");
    let preferred = fuse_with_preferences(ACTOR, feedback.conversation, &rankings(), 8, &profile)
        .expect("preferred fusion");
    assert_ordered(&preferred);
    let attested = hit(&preferred, ATTESTED_LSN);
    assert!(attested.score_q32 > hit(&plain, ATTESTED_LSN).score_q32);
    assert_eq!(attested.preference_q16, weight);
    assert!(
        attested.uri.contains(&format!("&pref={weight}")),
        "{}",
        attested.uri
    );
    assert_eq!(hit(&preferred, UNATTESTED_LSN), hit(&plain, UNATTESTED_LSN));
}

#[test]
fn harmful_feedback_lowers_only_its_own_evidence() {
    let feedback = Feedback::build(AttestationDisposition::Harmful);
    let snapshot = feedback.store.begin_snapshot().expect("snapshot");
    let weight = AttestationsProjection::preference(&snapshot, LSN::new(ATTESTED_LSN))
        .expect("stored preference")
        .expect("preference record")
        .weight_q16;
    assert!(weight < PREFERENCE_NEUTRAL_Q16);
    assert!(weight >= PREFERENCE_MINIMUM_Q16);
    let profile = PreferenceProfile::load(
        &snapshot,
        &[LSN::new(ATTESTED_LSN), LSN::new(UNATTESTED_LSN)],
    )
    .expect("profile");

    let plain = fuse(ACTOR, feedback.conversation, &rankings(), 8).expect("plain fusion");
    let disfavoured = fuse_with_preferences(ACTOR, feedback.conversation, &rankings(), 8, &profile)
        .expect("disfavoured fusion");
    assert_ordered(&disfavoured);
    let attested = hit(&disfavoured, ATTESTED_LSN);
    assert!(attested.score_q32 < hit(&plain, ATTESTED_LSN).score_q32);
    assert_eq!(attested.preference_q16, weight);
    assert!(
        attested.uri.contains(&format!("&pref={weight}")),
        "{}",
        attested.uri
    );
    assert_eq!(
        hit(&disfavoured, UNATTESTED_LSN),
        hit(&plain, UNATTESTED_LSN)
    );
}

#[test]
fn preference_band_is_bounded_both_ways() {
    let raised = adjust_q32(SAMPLE_SCORE_Q32, PREFERENCE_MAXIMUM_Q16).expect("raised score");
    let lowered = adjust_q32(SAMPLE_SCORE_Q32, PREFERENCE_MINIMUM_Q16).expect("lowered score");
    assert!(raised <= SAMPLE_SCORE_Q32 / 4 * 5 + 4);
    assert!(lowered >= SAMPLE_SCORE_Q32 / 4 * 3 - 4);
    assert!(raised > SAMPLE_SCORE_Q32);
    assert!(lowered < SAMPLE_SCORE_Q32);
    assert_eq!(
        adjust_q32(SAMPLE_SCORE_Q32, PREFERENCE_NEUTRAL_Q16).expect("neutral score"),
        SAMPLE_SCORE_Q32
    );
    assert_eq!(
        adjust_q32(0, PREFERENCE_MAXIMUM_Q16).expect("zero score"),
        0
    );
    assert_eq!(
        adjust_q32(u64::MAX, PREFERENCE_MAXIMUM_Q16)
            .expect_err("overflowing score")
            .code,
        ErrorCode::CapacityExceeded
    );
}

#[test]
fn adjust_q32_rejects_out_of_band_weights() {
    assert_eq!(
        adjust_q32(SAMPLE_SCORE_Q32, PREFERENCE_MINIMUM_Q16 - 1)
            .expect_err("below the band")
            .code,
        ErrorCode::InvalidArgument
    );
    assert_eq!(
        adjust_q32(SAMPLE_SCORE_Q32, PREFERENCE_MAXIMUM_Q16 + 1)
            .expect_err("above the band")
            .code,
        ErrorCode::InvalidArgument
    );
    assert_eq!(
        adjust_q32(SAMPLE_SCORE_Q32, 0)
            .expect_err("zero weight")
            .code,
        ErrorCode::InvalidArgument
    );
}

#[test]
fn load_rejects_zero_lsn_and_oversized_target_sets() {
    let feedback = Feedback::build(AttestationDisposition::Helpful);
    let snapshot = feedback.store.begin_snapshot().expect("snapshot");
    assert_eq!(
        PreferenceProfile::load(&snapshot, &[LSN::new(ATTESTED_LSN), LSN::new(0)])
            .expect_err("zero target")
            .code,
        ErrorCode::InvalidArgument
    );
    let oversized = (1..=u64::try_from(MAXIMUM_PREFERENCE_TARGETS).expect("bound") + 1)
        .map(LSN::new)
        .collect::<Vec<_>>();
    assert_eq!(
        PreferenceProfile::load(&snapshot, &oversized)
            .expect_err("oversized target set")
            .code,
        ErrorCode::CapacityExceeded
    );
    assert!(
        PreferenceProfile::load(&snapshot, &[])
            .expect("empty target set")
            .is_neutral()
    );
    assert!(
        PreferenceProfile::load(&snapshot, &[LSN::new(UNATTESTED_LSN)])
            .expect("unattested target")
            .is_neutral()
    );
    assert!(
        !PreferenceProfile::load(&snapshot, &[LSN::new(ATTESTED_LSN)])
            .expect("attested target")
            .is_neutral()
    );
    assert_eq!(
        PreferenceProfile::neutral().weight_q16(LSN::new(ATTESTED_LSN)),
        PREFERENCE_NEUTRAL_Q16
    );
}

struct Feedback {
    _directory: TempDir,
    store: ProjectionStore,
    conversation: ConversationId,
}

impl Feedback {
    fn build(disposition: AttestationDisposition) -> Self {
        let directory = tempfile::tempdir().expect("temporary directory");
        let store = ProjectionStore::open(directory.path(), 16 * 1024 * 1024).expect("store");
        let (conversation, mut frames) = common::workload();
        rebuild_projection_stream(&store, &frames, true, usize::MAX).expect("projections");
        let counter =
            TokenCounter::for_model("fallback", None, FallbackWeights::default()).expect("counter");
        let attestations = {
            let snapshot = store.begin_snapshot().expect("snapshot");
            let mut bundle = activate(
                &snapshot,
                &ActivationRequest {
                    actor: ACTOR,
                    conversation,
                    query: "alpha".to_owned(),
                    turn_text: String::new(),
                    budget_tokens: 10_000,
                    token_counter: &counter,
                    maximum_candidates: 64,
                    maximum_conversation_records: 64,
                },
            )
            .expect("activate");
            build_attestations(
                &mut bundle,
                AttestationRequest {
                    first_lsn: LSN::new(5),
                    actor: ACTOR,
                    conversation,
                    wall_timestamp_ns: UtcNanos::new(999),
                    disposition,
                },
            )
            .expect("attestations")
        };
        assert_eq!(attestations.len(), 4);
        frames.extend(attestations);
        rebuild_projection_stream(&store, &frames, true, usize::MAX).expect("attested projections");
        Self {
            _directory: directory,
            store,
            conversation,
        }
    }
}

fn rankings() -> Vec<LaneRanking> {
    vec![LaneRanking {
        lane: RetrievalLane::Lexical,
        weight_q16: Q16_ONE,
        candidates: vec![
            RankedCandidate::neutral(ATTESTED_LSN.to_be_bytes().to_vec(), LSN::new(ATTESTED_LSN)),
            RankedCandidate::neutral(
                UNATTESTED_LSN.to_be_bytes().to_vec(),
                LSN::new(UNATTESTED_LSN),
            ),
        ],
    }]
}

fn hit(hits: &[FusedHit], lsn: u64) -> &FusedHit {
    hits.iter()
        .find(|hit| hit.lsn == LSN::new(lsn))
        .expect("fused hit")
}

fn assert_ordered(hits: &[FusedHit]) {
    for pair in hits.windows(2) {
        assert!(
            pair[0].score_q32 > pair[1].score_q32
                || (pair[0].score_q32 == pair[1].score_q32
                    && pair[0].canonical_id < pair[1].canonical_id)
        );
    }
}
