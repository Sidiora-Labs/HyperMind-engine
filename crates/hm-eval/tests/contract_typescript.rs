use hm_eval::contract::daemon::{self, ContractDaemon};
use hm_eval::contract::{
    self, CONTRACT_CONVERSATION, ContractObservation, StepObservation, typescript,
};
use std::process::Command;

const PROBE: &str = "HM_CONTRACT_BINARY_PROBE";
const MISSING_BINARY: &str = "/nonexistent/hypermind/hm";

fn step<'a>(observation: &'a ContractObservation, name: &str) -> &'a StepObservation {
    observation
        .steps
        .iter()
        .find(|step| step.step == name)
        .unwrap_or_else(|| panic!("the typescript leg is missing step {name}"))
}

fn cross_the_restart(root: &std::path::Path) -> ContractObservation {
    let mut daemon = ContractDaemon::start(root).expect("the contract daemon did not start");
    let socket = daemon.socket().to_path_buf();
    let observed = typescript::observation(&socket, CONTRACT_CONVERSATION, root, &mut daemon);
    let stopped = daemon.stop();
    let observed = observed.expect("the typescript leg did not complete both phases");
    stopped.expect("the contract daemon did not stop");
    observed
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn typescript_and_rust_agree_on_the_behavioural_contract() {
    let daemon_root = tempfile::tempdir().unwrap();
    let reference_root = tempfile::tempdir().unwrap();
    let observed = cross_the_restart(daemon_root.path());
    assert_eq!(observed.sdk, "typescript");
    assert_eq!(observed.transport, "unix");
    assert_eq!(
        observed.steps.len(),
        contract::scenario(CONTRACT_CONVERSATION).len()
    );

    let reference = contract::reference_leg(reference_root.path(), CONTRACT_CONVERSATION)
        .await
        .expect("the reference leg did not complete");
    let comparison = contract::compare(&[reference, observed], Vec::new(), Vec::new());
    assert!(comparison.agreed, "{:?}", comparison.divergences);
    assert!(
        comparison.divergences.is_empty(),
        "{:?}",
        comparison.divergences
    );
    assert_eq!(comparison.participants, ["rust-embedded", "typescript"]);
    assert_eq!(
        comparison.steps_compared,
        contract::scenario(CONTRACT_CONVERSATION).len()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn restart_replays_the_same_identifiers() {
    let root = tempfile::tempdir().unwrap();
    let observed = cross_the_restart(root.path());
    let before = step(&observed, "recall-lexical");
    let after = step(&observed, "recall-after-restart");
    assert!(before.ok && after.ok, "{before:?} {after:?}");
    assert!(!before.item_identifiers.is_empty());
    assert!(!before.provenance.is_empty());
    assert_eq!(before.item_identifiers, after.item_identifiers);
    assert_eq!(before.provenance, after.provenance);
}

#[test]
fn availability_failures_are_named_not_swallowed() {
    if std::env::var_os(PROBE).is_some() {
        let missing = daemon::binary().expect_err("a nonexistent HM_BINARY resolved");
        let missing = missing.to_string();
        assert!(missing.contains(MISSING_BINARY), "{missing}");
        assert!(!missing.contains("ran"), "{missing}");
        let absent = tempfile::tempdir().unwrap();
        let unavailable =
            typescript::availability_in(absent.path()).expect_err("an empty root was available");
        assert_eq!(unavailable.sdk, "typescript");
        assert!(
            unavailable.reason.contains(typescript::CLIENT_BUNDLE),
            "{}",
            unavailable.reason
        );
        assert!(
            !unavailable.reason.contains("ran"),
            "{}",
            unavailable.reason
        );
        return;
    }
    typescript::ensure_built().expect("the typescript client workspace did not build");
    typescript::availability().expect("the typescript leg is unavailable");
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "availability_failures_are_named_not_swallowed",
            "--nocapture",
        ])
        .env(PROBE, "1")
        .env("HM_BINARY", MISSING_BINARY)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
