use hm_eval::contract::{
    self, ContractObservation, Phase, RAW_FORMAT, StepObservation, Unavailable, Unsupported,
};

const VERB_INVENTORY: [&str; 14] = [
    "remember",
    "recall",
    "activate",
    "inspect",
    "forget",
    "believe",
    "retract",
    "dispute",
    "bind",
    "attest",
    "consolidate",
    "intend",
    "predict",
    "outcome",
];

fn step<'a>(observation: &'a ContractObservation, name: &str) -> &'a StepObservation {
    observation
        .steps
        .iter()
        .find(|step| step.step == name)
        .unwrap_or_else(|| panic!("observation is missing step {name}"))
}

#[test]
fn scenario_uses_only_the_frozen_verb_inventory() {
    let calls = contract::scenario("contract-a");
    assert!(calls.len() >= 8, "scenario has {} steps", calls.len());
    let mut names = std::collections::BTreeSet::new();
    for call in &calls {
        assert!(
            VERB_INVENTORY.contains(&call.verb),
            "{} uses verb {} outside the inventory",
            call.step,
            call.verb
        );
        assert!(names.insert(call.step), "duplicate step {}", call.step);
    }
    assert_eq!(
        calls
            .iter()
            .filter(|call| call.verb == "remember" && call.phase == Phase::BeforeRestart)
            .count(),
        3
    );
    assert!(
        calls
            .iter()
            .any(|call| call.phase == Phase::AfterRestart && call.verb == "recall")
    );
    assert!(calls.iter().any(|call| call.verb == "forget"));
    assert!(calls.iter().any(|call| call.verb == "inspect"));
}

#[test]
fn provenance_normalization_drops_the_clock_field() {
    let normalized =
        contract::normalize_provenance("hm://7/abc/3?at=1700000000000000000&src=1&score=42");
    assert!(normalized.contains("hm://7/abc/3"), "{normalized}");
    assert!(normalized.contains("src=1"), "{normalized}");
    assert!(normalized.contains("score=42"), "{normalized}");
    assert!(!normalized.contains("at="), "{normalized}");
    let later =
        contract::normalize_provenance("hm://7/abc/3?at=1900000000000000009&src=1&score=42");
    assert_eq!(normalized, later);
    assert_eq!(
        contract::normalize_provenance("hm://7/lsn/4"),
        "hm://7/lsn/4"
    );
}

#[tokio::test]
async fn reference_leg_is_deterministic_and_survives_restart() {
    let first_root = tempfile::tempdir().unwrap();
    let second_root = tempfile::tempdir().unwrap();
    let first = contract::reference_leg(first_root.path(), "contract-a")
        .await
        .unwrap();
    let second = contract::reference_leg(second_root.path(), "contract-a")
        .await
        .unwrap();
    assert_eq!(first.sdk, "rust-embedded");
    assert_eq!(first.steps.len(), contract::scenario("contract-a").len());
    assert_eq!(
        serde_json::to_value(&first).unwrap(),
        serde_json::to_value(&second).unwrap()
    );

    let rejected_read = step(&first, "recall-rejected-argument");
    assert!(!rejected_read.ok);
    assert!(rejected_read.error_code.is_some());
    assert!(rejected_read.effect_state.is_none());

    let rejected_mutation = step(&first, "remember-rejected-mutation");
    assert!(!rejected_mutation.ok);
    assert!(rejected_mutation.error_code.is_some());
    assert!(rejected_mutation.effect_state.is_some());

    for name in [
        "remember-user",
        "remember-assistant",
        "recall-lexical",
        "inspect-first-event",
        "recall-after-restart",
        "forget-fade",
    ] {
        assert!(step(&first, name).ok, "{name} did not succeed");
    }

    let before = step(&first, "recall-lexical");
    let after = step(&first, "recall-after-restart");
    assert!(!before.item_identifiers.is_empty());
    assert!(!before.provenance.is_empty());
    assert_eq!(before.item_identifiers, after.item_identifiers);
    assert_eq!(before.provenance, after.provenance);
}

#[test]
fn comparer_reports_agreement_and_one_field_divergence() {
    let reference = ContractObservation {
        format: contract::CONTRACT_FORMAT.to_owned(),
        sdk: "rust-embedded".to_owned(),
        transport: "in-process".to_owned(),
        steps: vec![contract::observe(
            "remember-rejected-mutation",
            "remember",
            &serde_json::json!({
                "ok": false,
                "items": [{"error": "invalid_argument", "lsn": 0}],
                "provenance": [],
                "gaps": [],
                "health": {"projection": "unavailable"},
                "warnings": [],
                "effect_state": "not_applied",
            }),
        )],
    };
    let mut other = reference.clone();
    other.sdk = "typescript".to_owned();
    other.transport = "unix".to_owned();

    let agreement = contract::compare(&[reference.clone(), other.clone()], Vec::new(), Vec::new());
    assert!(agreement.agreed);
    assert!(agreement.divergences.is_empty());
    assert_eq!(agreement.steps_compared, 1);
    assert_eq!(agreement.participants, ["rust-embedded", "typescript"]);

    other.steps[0].effect_state = Some("applied".to_owned());
    let divergent = contract::compare(
        &[reference, other],
        vec![Unavailable {
            sdk: "go".to_owned(),
            reason: "toolchain absent".to_owned(),
        }],
        vec![Unsupported {
            sdk: "python".to_owned(),
            step: "forget-fade".to_owned(),
        }],
    );
    assert!(!divergent.agreed);
    assert_eq!(divergent.divergences.len(), 1);
    assert_eq!(divergent.divergences[0].step, "remember-rejected-mutation");
    assert_eq!(divergent.divergences[0].field, "effect_state");
    assert!(
        divergent.divergences[0]
            .values
            .contains_key("rust-embedded")
    );
    assert!(divergent.divergences[0].values.contains_key("typescript"));
    assert_eq!(divergent.unavailable.len(), 1);
    assert_eq!(divergent.unsupported.len(), 1);
}

#[tokio::test]
async fn raw_leg_parses_to_the_same_observation() {
    let raw_root = tempfile::tempdir().unwrap();
    let reference_root = tempfile::tempdir().unwrap();
    let document = contract::reference_raw(raw_root.path(), "contract-a")
        .await
        .unwrap();
    assert_eq!(document.format, RAW_FORMAT);
    assert_eq!(document.calls.len(), contract::scenario("contract-a").len());
    let bytes = serde_json::to_vec(&document).unwrap();
    let parsed = contract::observe_raw(&bytes).unwrap();
    let reference = contract::reference_leg(reference_root.path(), "contract-a")
        .await
        .unwrap();
    assert_eq!(parsed.steps, reference.steps);
    assert_eq!(parsed.sdk, reference.sdk);
    assert_eq!(parsed.format, contract::CONTRACT_FORMAT);
}
