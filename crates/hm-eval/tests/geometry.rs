use hm_compose::geometry::{GEOMETRY_VERSION, GeometryEvidence};
use hm_eval::suites::geometry::{self, DEPLOYMENT_ID, GeometryResult};

const FIXTURE_HASH: &str = "02a0979c8bb5ec9b0e930a6cdb2cee95a24416dfe0992709baa0457349966057";
const FIXTURE_QUERIES: u64 = 14;

fn evidence(result: &GeometryResult) -> GeometryEvidence {
    GeometryEvidence {
        producer: "hm-eval".to_owned(),
        deployment_id: DEPLOYMENT_ID.to_owned(),
        version: GEOMETRY_VERSION,
        evaluation_id: result.fixture_hash.clone(),
        evaluated_queries: result.queries,
        baseline_mrr_at_10: result.baseline_mrr_at_10,
        boosted_mrr_at_10: result.boosted_mrr_at_10,
    }
}

#[test]
fn the_geometry_ablation_reports_both_arms_and_stays_disabled() {
    let result = geometry::run().unwrap();
    assert_eq!(result.fixture_hash, FIXTURE_HASH);
    assert_eq!(result.queries, FIXTURE_QUERIES);
    assert!((0.0..=1.0).contains(&result.baseline_mrr_at_10));
    assert!((0.0..=1.0).contains(&result.boosted_mrr_at_10));
    assert!(!result.enabled_by_default);
}

#[test]
fn the_geometry_ablation_is_reproducible() {
    let first = geometry::run().unwrap();
    let second = geometry::run().unwrap();
    assert_eq!(first.fixture_hash, second.fixture_hash);
    assert_eq!(first.queries, second.queries);
    assert_eq!(
        first.baseline_mrr_at_10.to_bits(),
        second.baseline_mrr_at_10.to_bits()
    );
    assert_eq!(
        first.boosted_mrr_at_10.to_bits(),
        second.boosted_mrr_at_10.to_bits()
    );
    assert_eq!(first.enabled_by_default, second.enabled_by_default);
}

#[test]
fn the_ablation_never_qualifies_the_boost_on_its_own() {
    let result = geometry::run().unwrap();
    println!(
        "geometry_baseline_mrr_at_10={} geometry_boosted_mrr_at_10={}",
        result.baseline_mrr_at_10, result.boosted_mrr_at_10
    );
    let qualified = evidence(&result).justifies(DEPLOYMENT_ID, GEOMETRY_VERSION);
    assert_eq!(
        qualified,
        result.boosted_mrr_at_10 > result.baseline_mrr_at_10
    );
    assert!(!evidence(&result).justifies(DEPLOYMENT_ID, GEOMETRY_VERSION + 1));
    assert!(!evidence(&result).justifies("", GEOMETRY_VERSION));
    assert!(!geometry::run().unwrap().enabled_by_default);
}
