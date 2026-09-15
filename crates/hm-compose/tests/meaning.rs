#![forbid(unsafe_code)]

mod common;

use hm_compose::bundle::{ActivationRequest, HealthStatus, RetrievalLane, WhyCode, activate};
use hm_compose::deadline::DeadlineActivator;
use hm_compose::fusion::{LaneRanking, Q16_ONE, RankedCandidate, fuse};
use hm_compose::health::{EncoderState, HealthInput, evaluate};
use hm_compose::lanes::{entity, temporal, vector};
use hm_compose::planner::{Anchor, AnchorFacet, QueryShape, RecallMode, plan};
use hm_compose::tokens::{FallbackWeights, TokenCounter};
use hm_core::{ActorId, ConversationId, LSN, UtcNanos};
use hm_ledger::frame::EventKind;
use hm_proj::entities::EntityProjection;
use hm_proj::rebuild::rebuild_projection_stream;
use hm_proj::store::ProjectionStore;
use hm_proj::vectors::VectorLane;
use std::time::Duration;

#[test]
fn planner_is_deterministic_and_near_walks_the_anchor_entity_first() {
    let identifier = plan("find src/main.rs", "", RecallMode::Semantic, None).expect("plan");
    assert_eq!(identifier.shape, QueryShape::Identifier);
    assert_eq!(identifier.lanes[0].lane, RetrievalLane::Entity);
    assert_eq!(
        identifier,
        plan("find src/main.rs", "", RecallMode::Semantic, None).expect("same plan")
    );

    let temporal = plan("what changed last week", "", RecallMode::Semantic, None).expect("plan");
    assert_eq!(temporal.shape, QueryShape::Temporal);
    assert_eq!(temporal.lanes[0].lane, RetrievalLane::Temporal);
    let explanatory =
        plan("how does retrieval work", "", RecallMode::Semantic, None).expect("plan");
    assert_eq!(explanatory.shape, QueryShape::Explanatory);

    let anchor = Anchor {
        facet: AnchorFacet::Path,
        value: "src/main.rs".to_owned(),
    };
    let near = plan("related work", "", RecallMode::Near, Some(&anchor)).expect("near plan");
    assert_eq!(near.shape, QueryShape::Anchored);
    assert_eq!(near.lanes[0].lane, RetrievalLane::Entity);

    for (mode, lane) in [
        (RecallMode::Lexical, RetrievalLane::Lexical),
        (RecallMode::Entity, RetrievalLane::Entity),
        (RecallMode::Temporal, RetrievalLane::Temporal),
        (RecallMode::Graph, RetrievalLane::Graph),
        (RecallMode::AsOf, RetrievalLane::Belief),
        (RecallMode::Timeline, RetrievalLane::Timeline),
        (RecallMode::Reconstruct, RetrievalLane::Reconstruct),
    ] {
        assert_eq!(
            plan("query", "", mode, None).expect("mode").lanes[0].lane,
            lane
        );
    }
}

#[test]
fn vector_entity_and_temporal_lanes_feed_ranked_candidates() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let vector_lane =
        VectorLane::open(temporary.path().join("vectors"), "g1", "s1", 4).expect("vector lane");
    vector_lane
        .append(LSN::new(1), &[5, 5, 0, 0], &[0b0000_0011])
        .expect("append vector");
    assert_eq!(
        vector::search(&vector_lane, &[5, 5, 0, 0], &[0b0000_0011], 5).expect("vector search")[0]
            .lsn,
        LSN::new(1)
    );

    let store = ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).expect("store");
    let frame = common::message_frame(
        1,
        common::conversation(1),
        EventKind::UserMsg,
        "Alice Smith edited src/main.rs for GH-123",
    );
    EntityProjection::apply_event(&store, &frame).expect("entity projection");
    let snapshot = store.begin_snapshot().expect("snapshot");
    assert_eq!(
        entity::search(&snapshot, "GH-123", "", 5).expect("entity search")[0].lsn,
        LSN::new(1)
    );

    let temporal = temporal::search(
        &[
            temporal::TemporalCandidate {
                canonical_id: b"old".to_vec(),
                lsn: LSN::new(1),
                event_time_ns: UtcNanos::new(1_000),
            },
            temporal::TemporalCandidate {
                canonical_id: b"new".to_vec(),
                lsn: LSN::new(2),
                event_time_ns: UtcNanos::new(9_000),
            },
        ],
        UtcNanos::new(10_000),
        1_000,
        2,
    )
    .expect("temporal search");
    assert_eq!(temporal[0].lsn, LSN::new(2));
    assert!(temporal[0].recency_q16 > temporal[1].recency_q16);
}

#[test]
fn reciprocal_rank_fusion_uses_signals_and_explains_every_lane_rank() {
    let mut older = RankedCandidate::neutral(b"a".to_vec(), LSN::new(1));
    older.recency_q16 = Q16_ONE / 4;
    let newer = RankedCandidate::neutral(b"b".to_vec(), LSN::new(2));
    let hits = fuse(
        ActorId::new(7),
        ConversationId::new([9; 16]),
        &[
            LaneRanking {
                lane: RetrievalLane::Vector,
                weight_q16: 3 * Q16_ONE,
                candidates: vec![older.clone(), newer.clone()],
            },
            LaneRanking {
                lane: RetrievalLane::Lexical,
                weight_q16: 2 * Q16_ONE,
                candidates: vec![newer, older],
            },
        ],
        10,
    )
    .expect("fusion");
    assert_eq!(hits[0].lsn, LSN::new(2));
    assert_eq!(hits[0].why, WhyCode::Fused);
    assert_eq!(hits[0].lane_ranks[&RetrievalLane::Vector], 2);
    assert_eq!(hits[0].lane_ranks[&RetrievalLane::Lexical], 1);
    assert!(hits[0].uri.contains("ranks=lexical:1,vector:2"));
    assert!(hits[0].uri.contains("why=fused"));
}

#[test]
fn health_reports_encoder_backlog_projection_and_inclusion_separately() {
    let health = evaluate(HealthInput {
        encoder: EncoderState::Ready,
        embedding_backlog: 3,
        projection_lsn: LSN::new(8),
        ledger_lsn: LSN::new(10),
        semantic_included: false,
        lexical_included: true,
    });
    assert_eq!(health.encoder, HealthStatus::SemanticReady);
    assert_eq!(health.backlog, HealthStatus::SemanticLagging);
    assert_eq!(health.projection, HealthStatus::SemanticLagging);
    assert_eq!(health.inclusion, HealthStatus::LexicalOnly);
}

#[test]
fn deadline_returns_visible_stale_fallback_then_pushes_fresh_bundle() {
    let base = base_bundle();
    let activator = DeadlineActivator::new();
    let primed = base.clone();
    let first = activator
        .activate_default(common::conversation(0x33), LSN::new(4), move || Ok(primed))
        .expect("prime cache");
    assert!(!first.degraded);
    assert_eq!(first.stale_by_lsn, 0);

    let subscriber = activator.subscribe().expect("subscribe");
    let mut fresh = base;
    fresh.manifest.query_digest = *blake3::hash(b"fresh query").as_bytes();
    let stale = activator
        .activate(
            common::conversation(0x33),
            LSN::new(9),
            Duration::from_millis(1),
            move || {
                std::thread::sleep(Duration::from_millis(20));
                Ok(fresh)
            },
        )
        .expect("cached fallback");
    assert!(stale.degraded);
    assert_eq!(stale.stale_by_lsn, 5);
    assert!(
        stale
            .bundle
            .gaps
            .iter()
            .any(|gap| gap.detail.contains("deadline missed"))
    );

    let pushed = subscriber
        .recv_timeout(Duration::from_secs(1))
        .expect("fresh subscription push");
    assert_eq!(
        pushed.manifest.query_digest,
        *blake3::hash(b"fresh query").as_bytes()
    );
}

fn base_bundle() -> hm_compose::bundle::ActivationBundle {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).expect("store");
    let (conversation, frames) = common::workload();
    rebuild_projection_stream(&store, &frames, true, usize::MAX).expect("projections");
    let snapshot = store.begin_snapshot().expect("snapshot");
    let counter =
        TokenCounter::for_model("fallback", None, FallbackWeights::default()).expect("counter");
    activate(
        &snapshot,
        &ActivationRequest {
            actor: ActorId::new(19),
            conversation,
            query: "alpha".to_owned(),
            turn_text: String::new(),
            budget_tokens: 1_000,
            token_counter: &counter,
            maximum_candidates: 64,
            maximum_conversation_records: 64,
        },
    )
    .expect("activation bundle")
}
