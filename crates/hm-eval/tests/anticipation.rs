#[tokio::test]
async fn labelled_attention_and_observed_calibration_are_measured() {
    let attention = hm_eval::suites::attention::run().unwrap();
    assert!(attention.cases >= 20);
    assert!(attention.precision() >= 0.9);
    assert!(attention.recall() >= 0.9);
    assert_eq!(attention.reasons_present, attention.cases);
    let calibration = hm_eval::suites::calibration::run().await.unwrap();
    assert_eq!(calibration.cases, 37);
    assert_eq!(calibration.assessment_mismatches, 0);
    assert_eq!(calibration.duplicate_writes, 0);
    assert!(calibration.restart_identical);
    assert!(calibration.revision_required);
    assert_eq!(calibration.per_kind.len(), 7);
    for row in calibration.per_kind {
        assert_eq!(row.counts.supported, 1);
        assert_eq!(row.counts.pending, 1);
        assert_eq!(row.counts.unresolvable, 1);
        assert_eq!(row.counts.not_executed, 1);
        assert_eq!(
            row.counts.contradicted,
            if row.predicate_kind == "RevisionEquals" {
                3
            } else {
                1
            }
        );
    }
}
