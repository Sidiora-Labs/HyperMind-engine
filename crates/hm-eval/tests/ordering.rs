#![allow(clippy::float_cmp)]

use hm_eval::bench::beam::{self, Probe, ProbeKind, ProbeSet};
use hm_eval::bench::ordering::{self, DEFAULT_MINIMUM_OVERLAP};
use hm_eval::bench::rubric;

const JUDGE: &str = "hypermind-test-judge";
const ANSWER: &str = "The Lanternfish crew moved the beacon rehearsal to the second Tuesday.";

fn events(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|line| (*line).to_owned()).collect()
}

fn reference() -> Vec<String> {
    events(&[
        "Kestrel Sound gateway cutover completed",
        "Drift corrected calibration enabled in production",
        "Old telemetry bridge retired",
        "Harbour replay harness shipped",
    ])
}

fn scored(reference: &[String], observed: &[String]) -> ordering::OrderingScore {
    let alignment = ordering::align_by_terms(reference, observed, DEFAULT_MINIMUM_OVERLAP);
    ordering::score_ordering(reference, observed, &alignment).unwrap()
}

fn probe_of_kind(set: &ProbeSet, kind: ProbeKind) -> &Probe {
    set.conversations
        .iter()
        .flat_map(|conversation| conversation.probes.iter())
        .find(|probe| probe.kind == kind)
        .unwrap_or_else(|| panic!("the fixture carries no {kind:?} probe"))
}

#[test]
fn rank_correlation_matches_known_vectors() {
    assert_eq!(
        ordering::rank_correlation(&[0, 1, 2, 3], &[0, 1, 2, 3]),
        1.0
    );
    assert_eq!(
        ordering::rank_correlation(&[0, 1, 2, 3], &[3, 2, 1, 0]),
        -1.0
    );
    assert_eq!(ordering::rank_correlation(&[0], &[0]), 0.0);
    assert_eq!(ordering::rank_correlation(&[], &[]), 0.0);
    assert_eq!(ordering::rank_correlation(&[0, 1, 2], &[0, 1]), 0.0);
    let tied = ordering::rank_correlation(&[0, 1, 2], &[0, 0, 2]);
    assert!(tied > 0.0 && tied < 1.0, "one-sided tie scored {tied}");
}

#[test]
fn alignment_is_deterministic_and_uses_each_reference_once() {
    let reference = reference();
    let observed = events(&[
        "old telemetry bridge retired",
        "the harbour replay harness shipped",
        "Kestrel Sound gateway cutover completed",
        "old telemetry bridge retired",
        "seed packets weighed on the kitchen scale",
    ]);
    let alignment = ordering::align_by_terms(&reference, &observed, DEFAULT_MINIMUM_OVERLAP);
    assert_eq!(
        alignment.observed_to_reference,
        vec![Some(2), Some(3), Some(0), None, None]
    );
    let mut claimed = Vec::new();
    for index in alignment.observed_to_reference.iter().flatten() {
        assert!(
            !claimed.contains(index),
            "reference {index} was claimed twice"
        );
        claimed.push(*index);
    }
    assert_eq!(alignment.observed_to_reference[4], None);
    let again = ordering::align_by_terms(&reference, &observed, DEFAULT_MINIMUM_OVERLAP);
    assert_eq!(alignment, again);
}

#[test]
fn ordering_score_decomposes_into_order_and_coverage() {
    let reference = reference();

    let perfect = scored(&reference, &reference);
    assert_eq!(perfect.reference_events, 4);
    assert_eq!(perfect.observed_events, 4);
    assert_eq!(perfect.matched, 4);
    assert_eq!(perfect.precision, 1.0);
    assert_eq!(perfect.recall, 1.0);
    assert_eq!(perfect.coverage_f1, 1.0);
    assert_eq!(perfect.normalized_correlation, 1.0);
    assert_eq!(perfect.score, 1.0);
    assert_eq!(
        perfect.score,
        perfect.normalized_correlation * perfect.coverage_f1
    );

    let mut backwards = reference.clone();
    backwards.reverse();
    let reversed = scored(&reference, &backwards);
    assert_eq!(reversed.matched, 4);
    assert_eq!(reversed.coverage_f1, 1.0);
    assert_eq!(reversed.rank_correlation, -1.0);
    assert_eq!(reversed.normalized_correlation, 0.0);
    assert_eq!(reversed.score, 0.0);
    assert_eq!(
        reversed.score,
        reversed.normalized_correlation * reversed.coverage_f1
    );

    let partial = scored(
        &reference,
        &events(&[
            "Kestrel Sound gateway cutover completed",
            "Old telemetry bridge retired",
        ]),
    );
    assert_eq!(partial.reference_events, 4);
    assert_eq!(partial.observed_events, 2);
    assert_eq!(partial.matched, 2);
    assert_eq!(partial.precision, 1.0);
    assert_eq!(partial.recall, 0.5);
    assert_eq!(partial.normalized_correlation, 1.0);
    assert_eq!(partial.score, partial.coverage_f1);
    assert_eq!(
        partial.score,
        partial.normalized_correlation * partial.coverage_f1
    );
    assert!(partial.score < perfect.score);
}

#[test]
fn empty_reference_is_an_error_not_a_zero() {
    let observed = events(&["Kestrel Sound gateway cutover completed"]);
    let alignment = ordering::align_by_terms(&[], &observed, DEFAULT_MINIMUM_OVERLAP);
    assert!(ordering::score_ordering(&[], &observed, &alignment).is_err());
}

#[test]
fn event_ordering_probes_are_graded_without_a_judge() {
    let set = beam::fixture_probe_set().unwrap();
    let probe = probe_of_kind(&set, ProbeKind::EventOrdering);
    let grade = rubric::grade_from_responses(probe, &probe.reference_answer, JUDGE, &[]).unwrap();
    assert!(grade.grades.is_empty());
    assert!(grade.ordering.is_some());
    assert_eq!(grade.judge_failures, 0);
    assert_eq!(grade.graded_criteria, 0);
    let score = grade.score.unwrap();
    assert!((0.0..=1.0).contains(&score), "ordering score was {score}");
    assert_eq!(grade.score, Some(1.0));
    let ordering = grade.ordering.unwrap();
    assert_eq!(ordering.reference_events, probe.criteria.len());
    assert_eq!(ordering.matched, probe.criteria.len());
    assert_eq!(ordering.coverage_f1, 1.0);
    assert_eq!(ordering.normalized_correlation, 1.0);
}

#[test]
fn summary_separates_judge_scored_from_ordering_probes() {
    let set = beam::fixture_probe_set().unwrap();
    let ordering_probe = probe_of_kind(&set, ProbeKind::EventOrdering);
    let ordered =
        rubric::grade_from_responses(ordering_probe, &ordering_probe.reference_answer, JUDGE, &[])
            .unwrap();
    let ordering_score = ordered.score.unwrap();

    let judged_probe = probe_of_kind(&set, ProbeKind::InformationExtraction);
    let responses: Vec<String> = judged_probe
        .criteria
        .iter()
        .map(|_| r#"{"compliance":"partial","evidence":"the second Tuesday"}"#.to_owned())
        .collect();
    let judged = rubric::grade_from_responses(judged_probe, ANSWER, JUDGE, &responses).unwrap();
    assert_eq!(judged.score, Some(0.5));
    assert!(judged.ordering.is_none());

    let summary = rubric::summarize(&[ordered, judged], JUDGE);
    assert_eq!(summary.probes, 2);
    assert_eq!(summary.scored_probes, 2);
    assert_eq!(summary.ordering_probes, 1);
    assert_eq!(summary.judge_scored_probes, 1);
    assert_eq!(
        summary.ordering_probes + summary.judge_scored_probes,
        summary.scored_probes
    );
    assert_eq!(summary.mean_ordering_score, ordering_score);
    assert_eq!(summary.mean_score, f64::midpoint(ordering_score, 0.5));
    assert_eq!(summary.judge_failures, 0);
}
