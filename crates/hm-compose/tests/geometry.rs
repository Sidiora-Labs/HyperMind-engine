use hm_compose::bundle::RetrievalLane;
use hm_compose::fusion::{FusedHit, LaneRanking, Q16_ONE as FUSION_Q16_ONE, RankedCandidate, fuse};
use hm_compose::geometry::{
    AlignmentBoost, AlignmentReport, GEOMETRY_VERSION, GeometryConfig, GeometryError,
    GeometryEvidence,
};
use hm_core::{ActorId, ConversationId, LSN};
use hm_index::geometry::{MAXIMUM_FACTOR_Q16, MINIMUM_FACTOR_Q16, Q16_ONE};
use std::collections::BTreeMap;

const ACTOR: ActorId = ActorId::new(19);
const CONVERSATION: ConversationId = ConversationId::new([0x33; 16]);

fn fused(ids: &[&[u8]]) -> Vec<FusedHit> {
    let candidates = ids
        .iter()
        .enumerate()
        .map(|(index, id)| {
            RankedCandidate::neutral(
                (*id).to_vec(),
                LSN::new(u64::try_from(index + 1).expect("lsn")),
            )
        })
        .collect();
    fuse(
        ACTOR,
        CONVERSATION,
        &[LaneRanking {
            lane: RetrievalLane::Lexical,
            weight_q16: FUSION_Q16_ONE,
            candidates,
        }],
        ids.len(),
    )
    .expect("fused hits")
}

fn vectors(entries: &[(&[u8], &[i8])]) -> BTreeMap<Vec<u8>, Vec<i8>> {
    entries
        .iter()
        .map(|(id, vector)| ((*id).to_vec(), (*vector).to_vec()))
        .collect()
}

fn measured_evidence() -> GeometryEvidence {
    GeometryEvidence {
        producer: "hm-eval".to_owned(),
        deployment_id: "deployment-a".to_owned(),
        version: GEOMETRY_VERSION,
        evaluation_id: "geometry-ablation".to_owned(),
        evaluated_queries: 64,
        baseline_mrr_at_10: 0.61,
        boosted_mrr_at_10: 0.68,
    }
}

fn qualified_boost() -> AlignmentBoost {
    let boost = AlignmentBoost::open(&GeometryConfig {
        enabled: true,
        deployment_id: "deployment-a".to_owned(),
        version: GEOMETRY_VERSION,
        evidence: Some(measured_evidence()),
    })
    .expect("qualified boost");
    assert!(boost.enabled());
    boost
}

#[test]
fn the_boost_is_off_by_default_and_is_an_exact_identity() {
    let boost = AlignmentBoost::open(&GeometryConfig::default()).expect("default boost");
    assert!(!boost.enabled());
    let mut hits = fused(&[b"alpha", b"bravo", b"charlie", b"delta", b"echo"]);
    let before = hits.clone();
    let report = boost
        .apply(&[], &BTreeMap::new(), &mut hits)
        .expect("identity transform");
    assert_eq!(
        report,
        AlignmentReport {
            applied: 0,
            neutral: 5,
            minimum_factor_q16: Q16_ONE,
            maximum_factor_q16: Q16_ONE,
        }
    );
    assert_eq!(hits, before);
    assert!(
        hits.iter()
            .zip(&before)
            .all(|(left, right)| left.uri == right.uri && left.score_q32 == right.score_q32)
    );
}

#[test]
fn enabling_the_boost_requires_measured_hm_eval_evidence() {
    let refused = |config: &GeometryConfig| {
        assert_eq!(
            AlignmentBoost::open(config).err(),
            Some(GeometryError::UnqualifiedDeployment)
        );
    };
    let mut config = GeometryConfig {
        enabled: true,
        deployment_id: "deployment-a".to_owned(),
        version: GEOMETRY_VERSION,
        evidence: None,
    };
    refused(&config);
    let mut local = measured_evidence();
    local.producer = "local".to_owned();
    config.evidence = Some(local);
    refused(&config);
    let mut elsewhere = measured_evidence();
    elsewhere.deployment_id = "deployment-b".to_owned();
    config.evidence = Some(elsewhere);
    refused(&config);
    let mut ahead = measured_evidence();
    ahead.version = GEOMETRY_VERSION + 1;
    config.version = GEOMETRY_VERSION + 1;
    config.evidence = Some(ahead);
    refused(&config);
    let mut flat = measured_evidence();
    flat.boosted_mrr_at_10 = flat.baseline_mrr_at_10;
    config.version = GEOMETRY_VERSION;
    config.evidence = Some(flat);
    refused(&config);
    assert!(qualified_boost().enabled());
}

#[test]
fn a_qualified_boost_rescales_and_reorders_and_records_the_factor() {
    let boost = qualified_boost();
    let mut hits = fused(&[b"orthogonal", b"aligned", b"mixed"]);
    assert_eq!(hits[0].canonical_id.as_slice(), b"orthogonal".as_slice());
    let before = hits.clone();
    let query = [127_i8, 0, 0, 0];
    let stored = vectors(&[
        (b"orthogonal", &[0, 127, 0, 0]),
        (b"aligned", &[127, 0, 0, 0]),
        (b"mixed", &[90, 90, 0, 0]),
    ]);
    let report = boost
        .apply(&query, &stored, &mut hits)
        .expect("boosted hits");
    assert_eq!(report.applied, 3);
    assert_eq!(report.neutral, 0);
    assert!((MINIMUM_FACTOR_Q16..=MAXIMUM_FACTOR_Q16).contains(&report.minimum_factor_q16));
    assert!((MINIMUM_FACTOR_Q16..=MAXIMUM_FACTOR_Q16).contains(&report.maximum_factor_q16));
    assert!(report.minimum_factor_q16 <= report.maximum_factor_q16);
    assert_eq!(hits[0].canonical_id.as_slice(), b"aligned".as_slice());
    assert_eq!(hits[2].canonical_id.as_slice(), b"orthogonal".as_slice());
    for hit in &hits {
        let original = before
            .iter()
            .find(|candidate| candidate.canonical_id == hit.canonical_id)
            .expect("original hit");
        let (prefix, factor) = hit.uri.split_once("&boost=").expect("recorded factor");
        let factor: i64 = factor.parse().expect("integer factor");
        assert_eq!(prefix, original.uri);
        assert!(prefix.contains(&format!("?score={}&", original.score_q32)));
        assert!((MINIMUM_FACTOR_Q16..=MAXIMUM_FACTOR_Q16).contains(&factor));
        assert_eq!(
            u128::from(hit.score_q32),
            u128::from(original.score_q32) * u128::try_from(factor).expect("factor")
                / u128::try_from(Q16_ONE).expect("scale")
        );
    }
    assert!(hits[0].score_q32 > hits[1].score_q32);
    assert!(hits[1].score_q32 > hits[2].score_q32);
}

#[test]
fn missing_vectors_are_neutral_and_mismatched_dimensions_are_refused() {
    let boost = qualified_boost();
    let query = [127_i8, 0, 0, 0];
    let mut hits = fused(&[b"alpha", b"bravo", b"charlie"]);
    let before = hits.clone();
    let stored = vectors(&[(b"alpha", &[127, 0, 0, 0]), (b"bravo", &[0, 127, 0, 0])]);
    let report = boost
        .apply(&query, &stored, &mut hits)
        .expect("boosted hits");
    assert_eq!(report.applied, 2);
    assert_eq!(report.neutral, 1);
    let untouched = hits
        .iter()
        .find(|hit| hit.canonical_id.as_slice() == b"charlie".as_slice())
        .expect("neutral hit");
    let original = before
        .iter()
        .find(|hit| hit.canonical_id.as_slice() == b"charlie".as_slice())
        .expect("original hit");
    assert_eq!(untouched.score_q32, original.score_q32);
    assert_eq!(untouched.uri, original.uri);

    let mut hits = before.clone();
    let mismatched = vectors(&[
        (b"alpha", &[127, 0, 0, 0]),
        (b"bravo", &[0, 127, 0, 0]),
        (b"charlie", &[1, 2]),
    ]);
    assert_eq!(
        boost.apply(&query, &mismatched, &mut hits),
        Err(GeometryError::DimensionMismatch)
    );
    assert_eq!(hits, before);
    assert_eq!(
        boost.apply(&[], &BTreeMap::new(), &mut hits),
        Err(GeometryError::InvalidArgument)
    );
    assert_eq!(hits, before);
}

#[test]
fn an_overflowing_score_is_refused_rather_than_wrapped() {
    let boost = qualified_boost();
    let mut hits = fused(&[b"saturated"]);
    hits[0].score_q32 = u64::MAX;
    let before = hits.clone();
    let stored = vectors(&[(b"saturated", &[127, 0, 0, 0])]);
    assert_eq!(
        boost.apply(&[127, 0, 0, 0], &stored, &mut hits),
        Err(GeometryError::CapacityExceeded)
    );
    assert_eq!(hits, before);
}
