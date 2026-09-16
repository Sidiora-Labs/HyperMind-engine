use hm_eval::bench::sweep::RetrievalVariant;
use hm_eval::bench::{beam, sweep};

fn named(name: &str) -> RetrievalVariant {
    RetrievalVariant {
        name: name.to_owned(),
        ..RetrievalVariant::baseline()
    }
}

fn wide_lexical() -> RetrievalVariant {
    RetrievalVariant {
        lexical_limit: 64,
        ..named("wide-lexical")
    }
}

fn single_per_session() -> RetrievalVariant {
    RetrievalVariant {
        session_cap: 1,
        ..named("single-per-session")
    }
}

#[test]
fn single_variable_validation_rejects_uncontrolled_variants() {
    let baseline = RetrievalVariant::baseline();
    sweep::validate_single_variable(&baseline, &[wide_lexical(), single_per_session()])
        .expect("two variants that each change one field are controlled");

    let two_fields = RetrievalVariant {
        lexical_limit: 64,
        session_cap: 1,
        ..named("two-at-once")
    };
    let error = sweep::validate_single_variable(&baseline, &[two_fields])
        .expect_err("a variant changing two fields isolates nothing");
    assert!(error.to_string().contains("two-at-once"), "{error}");

    let identical = named("no-change");
    let error = sweep::validate_single_variable(&baseline, &[identical])
        .expect_err("a variant identical to the baseline isolates nothing");
    assert!(error.to_string().contains("no-change"), "{error}");

    let error = sweep::validate_single_variable(
        &baseline,
        &[
            wide_lexical(),
            RetrievalVariant {
                session_cap: 1,
                ..named("wide-lexical")
            },
        ],
    )
    .expect_err("two variants may not share a name");
    assert!(error.to_string().contains("wide-lexical"), "{error}");

    let error = sweep::validate_single_variable(
        &baseline,
        &[RetrievalVariant {
            lexical_limit: 64,
            ..named("")
        }],
    )
    .expect_err("an unnamed variant cannot be reported");
    assert!(error.to_string().contains("empty name"), "{error}");

    let error = sweep::validate_single_variable(
        &baseline,
        &[RetrievalVariant {
            lexical_limit: 64,
            ..named(&baseline.name)
        }],
    )
    .expect_err("a variant may not carry the baseline name");
    assert!(error.to_string().contains(&baseline.name), "{error}");
}

#[test]
fn variant_identity_is_bound_to_the_variant_and_the_probe_set() {
    let digest = "a".repeat(64);
    let baseline = RetrievalVariant::baseline();
    let identity = sweep::variant_identity(&digest, &baseline).unwrap();
    assert_eq!(identity.len(), 64);
    assert!(
        identity
            .chars()
            .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase()),
        "{identity}"
    );
    assert_eq!(
        identity,
        sweep::variant_identity(&digest, &RetrievalVariant::baseline()).unwrap()
    );
    for mutated in [
        RetrievalVariant {
            lexical_limit: baseline.lexical_limit + 1,
            ..RetrievalVariant::baseline()
        },
        RetrievalVariant {
            minimum_term_overlap: 0.25,
            ..RetrievalVariant::baseline()
        },
        RetrievalVariant {
            session_cap: 1,
            ..RetrievalVariant::baseline()
        },
        RetrievalVariant {
            recency_weight: 0.25,
            ..RetrievalVariant::baseline()
        },
    ] {
        assert_ne!(
            identity,
            sweep::variant_identity(&digest, &mutated).unwrap(),
            "changing {} left the identity unchanged",
            mutated.changed_fields(&baseline).join(",")
        );
    }
    assert_ne!(
        identity,
        sweep::variant_identity(&"b".repeat(64), &baseline).unwrap()
    );
}

#[tokio::test]
async fn sweep_measures_each_variant_against_the_baseline() {
    let set = beam::fixture_probe_set().unwrap();
    let baseline = RetrievalVariant::baseline();
    assert_eq!(baseline.lexical_limit, 16);
    let temporary = tempfile::tempdir().unwrap();
    let report = sweep::run(
        &set,
        &baseline,
        &[wide_lexical(), single_per_session()],
        temporary.path(),
    )
    .await
    .unwrap();
    assert_eq!(report.format, sweep::SWEEP_FORMAT);
    assert!(report.judge_free);
    assert_eq!(report.encoder, "lexical_only");
    assert_eq!(report.baseline.name, "baseline");
    assert_eq!(report.baseline.probes, 12);
    assert!(report.baseline.changed_field.is_none());
    assert_eq!(report.variants.len(), 2);
    for variant in &report.variants {
        assert_eq!(variant.probes, 12);
        let changed = variant
            .changed_field
            .as_deref()
            .expect("every variant isolates one field");
        assert_ne!(variant.baseline_value, variant.variant_value);
        assert!(variant.delta_evidence_recall.is_finite());
        assert!(variant.delta_mean_reciprocal_rank.is_finite());
        assert_eq!(variant.identity.len(), 64);
        match variant.name.as_str() {
            "wide-lexical" => {
                assert_eq!(changed, "lexical_limit");
                assert_eq!(variant.baseline_value, "16");
                assert_eq!(variant.variant_value, "64");
            }
            "single-per-session" => {
                assert_eq!(changed, "session_cap");
                assert_eq!(variant.variant_value, "1");
            }
            other => panic!("unexpected variant {other}"),
        }
        for outcome in &variant.outcomes {
            assert!(outcome.evidence_retrieved <= outcome.evidence_expected);
            if outcome.first_evidence_rank.is_some() {
                assert!(outcome.evidence_retrieved > 0);
            }
        }
    }
    let wide = report
        .variants
        .iter()
        .find(|variant| variant.name == "wide-lexical")
        .expect("the wide lexical variant is reported");
    assert!(
        wide.evidence_recall >= report.baseline.evidence_recall,
        "a wider lexical limit lost evidence: {} against {}",
        wide.evidence_recall,
        report.baseline.evidence_recall
    );
    if wide.delta_evidence_recall > 0.0 {
        assert!(report.improved.contains(&wide.name));
        assert!(!report.regressed.contains(&wide.name));
    } else if wide.delta_evidence_recall.abs() < f64::EPSILON
        && wide.delta_mean_reciprocal_rank.abs() < f64::EPSILON
    {
        assert!(!report.improved.contains(&wide.name));
        assert!(!report.regressed.contains(&wide.name));
    }
}

#[tokio::test]
async fn cached_variants_are_reused_and_identical() {
    let set = beam::fixture_probe_set().unwrap();
    let baseline = RetrievalVariant::baseline();
    let temporary = tempfile::tempdir().unwrap();
    let first = sweep::run(&set, &baseline, &[wide_lexical()], temporary.path())
        .await
        .unwrap();
    let identity =
        sweep::variant_identity(&beam::probe_set_digest(&set).unwrap(), &baseline).unwrap();
    let cache = sweep::variant_cache_path(temporary.path(), &identity);
    assert!(cache.exists(), "{} was not written", cache.display());
    let second = sweep::run(&set, &baseline, &[wide_lexical()], temporary.path())
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(&first).unwrap(),
        serde_json::to_value(&second).unwrap()
    );
}

#[tokio::test]
async fn a_foreign_cache_identity_is_refused() {
    let set = beam::fixture_probe_set().unwrap();
    let baseline = RetrievalVariant::baseline();
    let temporary = tempfile::tempdir().unwrap();
    sweep::run(&set, &baseline, &[wide_lexical()], temporary.path())
        .await
        .unwrap();
    let identity =
        sweep::variant_identity(&beam::probe_set_digest(&set).unwrap(), &baseline).unwrap();
    let cache = sweep::variant_cache_path(temporary.path(), &identity);
    let mut stored: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&cache).unwrap()).unwrap();
    stored["identity"] = serde_json::Value::String("c".repeat(64));
    std::fs::write(&cache, serde_json::to_vec(&stored).unwrap()).unwrap();
    let error = sweep::run(&set, &baseline, &[wide_lexical()], temporary.path())
        .await
        .expect_err("a foreign cache identity is never reused");
    assert!(error.to_string().contains("identity mismatch"), "{error}");
}
