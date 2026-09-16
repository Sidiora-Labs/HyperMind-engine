#![forbid(unsafe_code)]

use hm_compose::bundle::{RetrievalLane, WhyCode};
use hm_compose::fusion::{LaneRanking, Q16_ONE, RankedCandidate, fuse};
use hm_compose::lanes::relation::{MAXIMUM_RELATION_CANDIDATES, RelationHit, rank};
use hm_compose::planner::{RecallMode, plan};
use hm_core::{ActorId, ConversationId, ErrorCode, LSN};

fn hit(edge_id: &[u8], event_lsn: u64, weight_micros: u32, score: i64) -> RelationHit {
    RelationHit {
        edge_id: edge_id.to_vec(),
        event_lsn: LSN::new(event_lsn),
        weight_micros,
        score,
        support_lsns: vec![LSN::new(event_lsn)],
    }
}

fn workload() -> Vec<RelationHit> {
    vec![
        hit(b"edge-alpha", 7, 1_000_000, 90),
        hit(b"edge-beta", 3, 250_000, 90),
        hit(b"edge-gamma", 9, 1, 40),
    ]
}

#[test]
fn relationship_hits_rank_by_score_then_event_lsn_and_carry_edge_salience() {
    let ranked = rank(&workload(), 8).expect("relation ranking");
    assert_eq!(ranked.dropped, 0);
    assert_eq!(ranked.ranking.lane, RetrievalLane::Relation);
    assert_eq!(ranked.ranking.weight_q16, Q16_ONE);

    let order = ranked
        .ranking
        .candidates
        .iter()
        .map(|candidate| candidate.lsn.get())
        .collect::<Vec<_>>();
    assert_eq!(order, vec![3, 7, 9]);

    let ids = ranked
        .ranking
        .candidates
        .iter()
        .map(|candidate| candidate.canonical_id.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        vec![
            b"edge-beta".to_vec(),
            b"edge-alpha".to_vec(),
            b"edge-gamma".to_vec()
        ]
    );

    let salience = ranked
        .ranking
        .candidates
        .iter()
        .map(|candidate| candidate.salience_q16)
        .collect::<Vec<_>>();
    assert_eq!(
        salience,
        vec![
            250_000 * u64::from(Q16_ONE) / 1_000_000,
            1_000_000 * u64::from(Q16_ONE) / 1_000_000,
            1
        ]
        .into_iter()
        .map(|value| u32::try_from(value).expect("salience fits"))
        .collect::<Vec<_>>()
    );

    let truncated = rank(&workload(), 2).expect("truncated ranking");
    assert_eq!(truncated.ranking.candidates.len(), 2);
    assert_eq!(truncated.ranking.candidates[1].lsn.get(), 7);
}

#[test]
fn unusable_relationship_hits_are_dropped_and_counted() {
    let mut hits = workload();
    hits.push(hit(b"", 11, 500_000, 95));
    hits.push(hit(b"edge-delta", 0, 500_000, 95));

    let ranked = rank(&hits, 8).expect("relation ranking");
    assert_eq!(ranked.dropped, 2);
    assert_eq!(ranked.ranking.candidates.len(), 3);
    assert!(
        ranked
            .ranking
            .candidates
            .iter()
            .all(|candidate| !candidate.canonical_id.is_empty() && candidate.lsn.get() != 0)
    );
    assert!(
        !ranked
            .ranking
            .candidates
            .iter()
            .any(|candidate| candidate.canonical_id == b"edge-delta".to_vec())
    );

    let empty = rank(&[], 8).expect("empty ranking");
    assert!(empty.ranking.candidates.is_empty());
    assert_eq!(empty.dropped, 0);
}

#[test]
fn the_relation_lane_fuses_and_explains_itself() {
    let ranked = rank(&workload(), 8).expect("relation ranking");
    let lexical = LaneRanking {
        lane: RetrievalLane::Lexical,
        weight_q16: Q16_ONE,
        candidates: vec![RankedCandidate::neutral(
            b"edge-alpha".to_vec(),
            LSN::new(7),
        )],
    };
    let hits = fuse(
        ActorId::new(7),
        ConversationId::new([9; 16]),
        &[ranked.ranking, lexical],
        8,
    )
    .expect("fused hits");
    assert_eq!(hits.len(), 3);
    assert!(
        hits.iter()
            .all(|hit| hit.lane_ranks.contains_key(&RetrievalLane::Relation))
    );

    let relation_only = hits
        .iter()
        .find(|hit| hit.canonical_id == b"edge-beta".to_vec())
        .expect("relation-only hit");
    assert_eq!(relation_only.why, WhyCode::Relation);
    assert!(relation_only.uri.contains("relation:1"));
    assert!(relation_only.uri.ends_with("&why=relation"));

    let shared = hits
        .iter()
        .find(|hit| hit.canonical_id == b"edge-alpha".to_vec())
        .expect("shared hit");
    assert_eq!(shared.why, WhyCode::Fused);
}

#[test]
fn relation_mode_plans_the_relation_lane_and_bounds_are_enforced() {
    let planned = plan("who owns the deploy key", "", RecallMode::Relation, None).expect("plan");
    assert_eq!(planned.lanes[0].lane, RetrievalLane::Relation);
    assert_eq!(planned.lanes.len(), 1);
    assert_eq!(
        planned,
        plan("who owns the deploy key", "", RecallMode::Relation, None).expect("same plan")
    );

    let hits = workload();
    assert_eq!(
        rank(&hits, 0).expect_err("zero limit").code,
        ErrorCode::InvalidArgument
    );
    assert_eq!(
        rank(&hits, MAXIMUM_RELATION_CANDIDATES + 1)
            .expect_err("limit above the bound")
            .code,
        ErrorCode::InvalidArgument
    );
    assert!(rank(&hits, MAXIMUM_RELATION_CANDIDATES).is_ok());
}
