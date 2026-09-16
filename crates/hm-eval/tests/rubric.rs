#![allow(clippy::float_cmp)]

use hm_eval::bench::beam::{self, Probe, ProbeKind, ProbeSet};
use hm_eval::bench::rubric::{
    self, CRITERION_PROMPT, CRITERION_PROMPT_ID, Compliance, ProbeGrade, VerdictFailure,
};

const JUDGE: &str = "hypermind-test-judge";
const ANSWER: &str = "The lanternfish crew moved the beacon rehearsal to the second Tuesday.";

fn probe_set() -> ProbeSet {
    beam::fixture_probe_set().unwrap()
}

fn probe_with_id<'a>(set: &'a ProbeSet, id: &str) -> &'a Probe {
    set.conversations
        .iter()
        .flat_map(|conversation| conversation.probes.iter())
        .find(|probe| probe.id == id)
        .unwrap()
}

fn verdicts(compliances: &[&str]) -> Vec<String> {
    compliances
        .iter()
        .enumerate()
        .map(|(index, compliance)| {
            format!(r#"{{"compliance":"{compliance}","evidence":"span {index}"}}"#)
        })
        .collect()
}

fn graded(probe: &Probe, compliances: &[&str]) -> ProbeGrade {
    rubric::grade_from_responses(probe, ANSWER, JUDGE, &verdicts(compliances)).unwrap()
}

#[test]
fn verdict_parsing_separates_judge_failures_from_noncompliance() {
    type Expected = Result<(Compliance, &'static str), VerdictFailure>;
    let table: [(&str, Expected); 8] = [
        (
            r#"{"compliance":"full","evidence":"moved the beacon rehearsal"}"#,
            Ok((Compliance::Full, "moved the beacon rehearsal")),
        ),
        (
            "```json\n{\"compliance\":\"partial\",\"evidence\":\"the second Tuesday\"}\n```",
            Ok((Compliance::Partial, "the second Tuesday")),
        ),
        (
            r#"{"compliance":"none","evidence":"the lanternfish crew"}"#,
            Ok((Compliance::None, "the lanternfish crew")),
        ),
        (
            "\n\n   {\"compliance\":\"full\",\"evidence\":\"second Tuesday\"}   \n\n",
            Ok((Compliance::Full, "second Tuesday")),
        ),
        ("", Err(VerdictFailure::EmptyResponse)),
        (
            "I am not able to grade this criterion with confidence.",
            Err(VerdictFailure::Unparsable),
        ),
        (
            r#"{"compliance":0.5,"evidence":"second Tuesday"}"#,
            Err(VerdictFailure::UnknownCompliance),
        ),
        (
            r#"{"compliance":"full","evidence":""}"#,
            Err(VerdictFailure::MissingEvidence),
        ),
    ];
    for (raw, expected) in table {
        match (rubric::parse_verdict(raw), expected) {
            (Ok((compliance, evidence)), Ok((wanted, wanted_evidence))) => {
                assert_eq!(compliance, wanted, "{raw:?}");
                assert_eq!(evidence, wanted_evidence, "{raw:?}");
            }
            (Err(failure), Err(wanted)) => assert_eq!(failure, wanted, "{raw:?}"),
            (actual, wanted) => panic!("{raw:?}: expected {wanted:?}, got {actual:?}"),
        }
    }
    assert!(
        rubric::parse_verdict(r#"{"compliance":"none","evidence":"nothing about the beacon"}"#)
            .is_ok()
    );
    assert_eq!(
        rubric::parse_verdict(r#"{"compliance":0.5,"evidence":"second Tuesday"}"#).unwrap_err(),
        VerdictFailure::UnknownCompliance
    );
    assert_eq!(
        rubric::parse_verdict(r#"{"compliance":"mostly","evidence":"second Tuesday"}"#)
            .unwrap_err(),
        VerdictFailure::UnknownCompliance
    );
}

#[test]
fn probe_grade_preserves_every_criterion_and_its_evidence() {
    let set = probe_set();
    let probe = set
        .conversations
        .iter()
        .flat_map(|conversation| conversation.probes.iter())
        .find(|probe| probe.kind == ProbeKind::InformationExtraction)
        .unwrap();
    let responses = verdicts(&vec!["full"; probe.criteria.len()]);
    let grade = rubric::grade_from_responses(probe, ANSWER, JUDGE, &responses).unwrap();
    assert_eq!(grade.probe_id, probe.id);
    assert_eq!(grade.kind, ProbeKind::InformationExtraction);
    assert_eq!(grade.question, probe.question);
    assert_eq!(grade.answer, ANSWER);
    assert_eq!(
        grade.answer_digest,
        blake3::hash(ANSWER.as_bytes()).to_hex().to_string()
    );
    assert_eq!(grade.grades.len(), probe.criteria.len());
    assert_eq!(grade.graded_criteria, probe.criteria.len());
    assert_eq!(grade.judge_failures, 0);
    for (index, criterion_grade) in grade.grades.iter().enumerate() {
        assert_eq!(criterion_grade.criterion_index, index);
        assert_eq!(criterion_grade.criterion, probe.criteria[index]);
        assert_eq!(criterion_grade.evidence, format!("span {index}"));
        assert_eq!(
            criterion_grade.response_digest,
            blake3::hash(responses[index].as_bytes())
                .to_hex()
                .to_string()
        );
        assert_eq!(criterion_grade.prompt_id, CRITERION_PROMPT_ID);
        assert_eq!(criterion_grade.judge_model, JUDGE);
        assert_eq!(criterion_grade.compliance, Some(Compliance::Full));
        assert!(criterion_grade.failure.is_none());
    }
}

#[test]
fn score_is_the_mean_of_criterion_weights() {
    let set = probe_set();
    let probe = probe_with_id(&set, "lanternfish-01");
    assert_eq!(probe.criteria.len(), 2);
    for (compliances, expected) in [
        (["full", "full"], Some(1.0)),
        (["partial", "partial"], Some(0.5)),
        (["none", "none"], Some(0.0)),
        (["full", "partial"], Some(0.75)),
    ] {
        let grade = graded(probe, &compliances);
        assert_eq!(grade.score, expected, "{compliances:?}");
        assert_eq!(grade.judge_failures, 0, "{compliances:?}");
        assert_eq!(grade.graded_criteria, 2, "{compliances:?}");
    }
}

#[test]
fn a_judge_failure_leaves_the_probe_unscored() {
    let set = probe_set();
    let probe = probe_with_id(&set, "lanternfish-03");
    let mut responses = verdicts(&vec!["full"; probe.criteria.len()]);
    responses[1] = "I cannot decide this one.".to_owned();
    let failed = rubric::grade_from_responses(probe, ANSWER, JUDGE, &responses).unwrap();
    assert!(failed.score.is_none());
    assert_eq!(failed.judge_failures, 1);
    assert_eq!(failed.graded_criteria, probe.criteria.len() - 1);
    assert!(failed.grades[1].failure.is_some());
    assert!(failed.grades[1].compliance.is_none());
    assert_eq!(failed.grades[1].evidence, "");
    assert_eq!(failed.grades[1].criterion, probe.criteria[1]);
    assert_eq!(failed.grades[0].compliance, Some(Compliance::Full));

    let complete = graded(probe_with_id(&set, "lanternfish-01"), &["full", "partial"]);
    assert_eq!(complete.score, Some(0.75));
    let summary = rubric::summarize(&[failed, complete], JUDGE);
    assert_eq!(summary.probes, 2);
    assert_eq!(summary.scored_probes, 1);
    assert_eq!(summary.unscored_probes, 1);
    assert_eq!(summary.judge_failures, 1);
    assert_eq!(summary.mean_score, 0.75);
    assert_eq!(summary.judge_model, JUDGE);
    assert_eq!(summary.prompt_id, CRITERION_PROMPT_ID);
}

#[test]
fn per_kind_rows_are_not_blended() {
    let set = probe_set();
    let grades = [
        graded(probe_with_id(&set, "lanternfish-01"), &["full", "full"]),
        graded(probe_with_id(&set, "lanternfish-02"), &["full", "none"]),
        graded(
            probe_with_id(&set, "lanternfish-03"),
            &["partial", "partial", "partial"],
        ),
    ];
    let summary = rubric::summarize(&grades, JUDGE);
    assert_eq!(summary.per_kind.len(), 3);
    let expected = [
        (ProbeKind::InformationExtraction, 1.0),
        (ProbeKind::TemporalReasoning, 0.5),
        (ProbeKind::MultiSessionReasoning, 0.5),
    ];
    for (kind, mean_score) in expected {
        let row = summary
            .per_kind
            .iter()
            .find(|row| row.kind == kind)
            .unwrap_or_else(|| panic!("missing per-kind row for {kind:?}"));
        assert_eq!(row.probes, 1, "{kind:?}");
        assert_eq!(row.scored, 1, "{kind:?}");
        assert_eq!(row.judge_failures, 0, "{kind:?}");
        assert_eq!(row.mean_score, mean_score, "{kind:?}");
        assert!(
            (row.mean_score - summary.mean_score).abs() > 1e-9,
            "{kind:?} was blended into the overall mean"
        );
    }
    assert_eq!(summary.scored_probes, 3);
    assert!((summary.mean_score - 2.0 / 3.0).abs() < 1e-12);
}

#[test]
fn prompt_is_versioned_and_states_the_grading_contract() {
    assert!(CRITERION_PROMPT_ID.ends_with("@1"));
    assert_eq!(CRITERION_PROMPT_ID, "criterion-grade@1");
    for token in ["compliance", "evidence", "full", "partial", "none"] {
        assert!(CRITERION_PROMPT.contains(token), "prompt omits {token}");
    }
    let prompt = rubric::criterion_prompt("who moved it?", "names the crew", ANSWER);
    let parsed: serde_json::Value = serde_json::from_str(&prompt).unwrap();
    assert_eq!(parsed["question"], "who moved it?");
    assert_eq!(parsed["criterion"], "names the crew");
    assert_eq!(parsed["answer"], ANSWER);
}
