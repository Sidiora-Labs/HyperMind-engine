use hm_eval::bench::execution::repository_root;
use hm_eval::contract::report::{self, ContractReport};
use hm_eval::contract::{
    self, CONTRACT_CONVERSATION, ContractObservation, StepObservation, Unsupported,
};
use std::collections::BTreeMap;

fn observed(sdk: &str, steps: &[&str]) -> ContractObservation {
    ContractObservation {
        format: contract::CONTRACT_FORMAT.to_owned(),
        sdk: sdk.to_owned(),
        transport: "in-process".to_owned(),
        steps: steps
            .iter()
            .map(|step| StepObservation {
                step: (*step).to_owned(),
                verb: "remember".to_owned(),
                ok: true,
                error_code: None,
                effect_state: Some("applied".to_owned()),
                item_identifiers: vec!["lsn=1".to_owned()],
                provenance: vec!["hm://7/lsn/1".to_owned()],
                authority: vec!["observed".to_owned()],
                health: BTreeMap::new(),
                gap_kinds: Vec::new(),
                warning_kinds: Vec::new(),
            })
            .collect(),
    }
}

async fn contract_report() -> ContractReport {
    match report::run().await {
        Ok(report) => report,
        Err(error) => {
            let path = repository_root().join(report::RESULT_PATH);
            let bytes = std::fs::read(&path).unwrap_or_else(|read| {
                panic!(
                    "the contract run failed and {} could not be read ({read}): {error}",
                    path.display()
                )
            });
            serde_json::from_slice(&bytes)
                .unwrap_or_else(|parse| panic!("the written report did not parse ({parse})"))
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_names_every_leg_it_could_not_run() {
    let report = contract_report().await;
    assert_eq!(report.format, report::REPORT_FORMAT);
    assert_eq!(
        report.scenario_steps,
        contract::scenario(CONTRACT_CONVERSATION).len()
    );
    assert!(
        report
            .comparison
            .participants
            .iter()
            .any(|sdk| sdk == contract::REFERENCE_SDK),
        "{:?}",
        report.comparison.participants
    );
    for entry in &report.comparison.unavailable {
        assert!(
            !entry.reason.trim().is_empty(),
            "{} carries no reason",
            entry.sdk
        );
        assert!(!entry.sdk.trim().is_empty(), "{entry:?}");
    }
    let go = report
        .comparison
        .unavailable
        .iter()
        .find(|entry| entry.sdk == report::GO_SDK)
        .expect("the go sdk is not listed as unavailable");
    assert!(go.reason.contains("go toolchain"), "{}", go.reason);
    assert!(
        report.comparison.participants.len() + report.comparison.unavailable.len() >= 3,
        "{:?} / {:?}",
        report.comparison.participants,
        report.comparison.unavailable
    );
    assert!(
        !report
            .comparison
            .participants
            .iter()
            .any(|sdk| sdk == report::GO_SDK),
        "the go sdk was reported as a participant"
    );
    assert!(
        repository_root().join(report::RESULT_PATH).is_file(),
        "the contract report was not written"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn participating_legs_agree() {
    let report = report::run().await.unwrap_or_else(|error| {
        panic!("the participating legs did not certify: {error}");
    });
    assert!(report.restarted, "no leg crossed a daemon restart");
    assert!(
        report.comparison.agreed,
        "{:?}",
        report.comparison.divergences
    );
    assert!(
        report.comparison.divergences.is_empty(),
        "{:?}",
        report.comparison.divergences
    );
    assert_eq!(
        report.comparison.steps_compared,
        contract::scenario(CONTRACT_CONVERSATION).len()
    );
    assert!(report.comparison.participants.len() >= report::MINIMUM_PARTICIPANTS);
}

#[test]
fn unsupported_steps_are_not_divergences() {
    let reference = observed(contract::REFERENCE_SDK, &["remember-user"]);
    let leg = observed("typescript", &["remember-user", report::MIGRATION_STEP]);
    let declared = vec![Unsupported {
        sdk: contract::REFERENCE_SDK.to_owned(),
        step: report::MIGRATION_STEP.to_owned(),
    }];
    let comparison = contract::compare(&[reference, leg], Vec::new(), declared);
    assert!(comparison.agreed, "{:?}", comparison.divergences);
    assert!(
        comparison.divergences.is_empty(),
        "{:?}",
        comparison.divergences
    );
    assert_eq!(comparison.steps_compared, 2);
    assert!(
        comparison
            .unsupported
            .iter()
            .any(|entry| entry.sdk == contract::REFERENCE_SDK
                && entry.step == report::MIGRATION_STEP)
    );
    let declared = report::migration_unsupported();
    assert!(!declared.is_empty());
    for entry in &declared {
        assert_eq!(entry.step, report::MIGRATION_STEP);
        assert!(!report::exposes_migration_surface(&entry.sdk));
    }
    assert!(declared.iter().any(|entry| entry.sdk == report::GO_SDK));
    assert!(
        declared
            .iter()
            .any(|entry| entry.sdk == contract::REFERENCE_SDK)
    );
    assert!(report::exposes_migration_surface("typescript"));
    assert!(!declared.iter().any(|entry| entry.sdk == "typescript"));
}

#[test]
fn a_single_participant_is_an_error() {
    let reference = observed(contract::REFERENCE_SDK, &["remember-user"]);
    let alone = contract::compare(
        std::slice::from_ref(&reference),
        vec![report::go_availability()],
        Vec::new(),
    );
    assert_eq!(alone.participants.len(), 1);
    let lonely = report::certify(&alone).expect_err("a single participant certified");
    let lonely = lonely.to_string();
    assert!(lonely.contains(contract::REFERENCE_SDK), "{lonely}");
    assert!(lonely.contains(report::GO_SDK), "{lonely}");

    let mut diverging = observed("typescript", &["remember-user"]);
    diverging.steps[0].ok = false;
    let comparison = contract::compare(&[reference, diverging], Vec::new(), Vec::new());
    assert!(!comparison.agreed);
    let divergence = report::certify(&comparison).expect_err("a divergence certified");
    let divergence = divergence.to_string();
    assert!(divergence.contains("remember-user"), "{divergence}");
    assert!(divergence.contains("ok"), "{divergence}");
}
