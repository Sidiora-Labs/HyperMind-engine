use hm_eval::bench::{beam, beam_run};

#[tokio::test]
async fn judge_free_run_over_the_fixture_reports_retrieval_only() {
    let set = beam::fixture_probe_set().unwrap();
    let temporary = tempfile::tempdir().unwrap();
    let result = beam_run::run_judge_free(&set, temporary.path())
        .await
        .unwrap();
    assert_eq!(result.format, beam_run::RESULT_FORMAT);
    assert_eq!(result.conversations, 2);
    assert_eq!(result.total, 12);
    assert_eq!(result.answered, 0);
    assert!(result.judged.is_none());
    assert_eq!(result.encoder, "lexical_only");
    assert!(!result.complete);
    let failure = result.failure.as_deref().expect("a fixture run is flagged");
    assert!(failure.contains("fixture"), "{failure}");
    assert_eq!(result.judge_free.rows.len(), 12);
    assert_eq!(result.judge_free.probes, 12);
    assert!(
        result.judge_free.evidence_recall > 0.0,
        "evidence recall was {}",
        result.judge_free.evidence_recall
    );
    assert_eq!(
        result.probe_set_digest,
        beam::probe_set_digest(&set).unwrap()
    );
    assert_eq!(result.source_digest, set.source_digest);
    let kinds: usize = result
        .judge_free
        .per_kind_probes
        .iter()
        .map(|(_, probes)| probes)
        .sum();
    assert_eq!(kinds, 12);
    for row in &result.judge_free.rows {
        assert!(row.evidence_retrieved <= row.evidence_expected);
        if row.first_evidence_rank.is_some() {
            assert!(row.evidence_retrieved > 0);
        }
    }
}

#[tokio::test]
async fn report_is_identical_across_roots() {
    let set = beam::fixture_probe_set().unwrap();
    let first_root = tempfile::tempdir().unwrap();
    let second_root = tempfile::tempdir().unwrap();
    let first = beam_run::run_judge_free(&set, first_root.path())
        .await
        .unwrap();
    let second = beam_run::run_judge_free(&set, second_root.path())
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(&first).unwrap(),
        serde_json::to_value(&second).unwrap()
    );
}

#[tokio::test]
async fn a_foreign_format_tag_is_refused() {
    let mut set = beam::fixture_probe_set().unwrap();
    set.format = "hypermind.beam-probe-set.v0".to_owned();
    let temporary = tempfile::tempdir().unwrap();
    let error = beam_run::run_judge_free(&set, temporary.path())
        .await
        .expect_err("a foreign format tag produces no report");
    assert!(
        error.to_string().contains("hypermind.beam-probe-set.v0"),
        "{error}"
    );
}
