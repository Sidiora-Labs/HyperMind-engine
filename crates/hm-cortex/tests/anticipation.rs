#![forbid(unsafe_code)]

use hm_cortex::attention::{AttentionFactors, decide};
use hm_cortex::predict::{Observation, assess, revision_gap};
use hm_cortex::procedures::{Episode, can_adopt, mine, supported};
use hm_cortex::prospective::{WakeSignal, evaluate, rearm};
use hm_schema::events::{
    AttentionDecision, Authority, ExpectedPredicate, IntentionSet, OutcomeAssessment,
    PredicateKind, Predicted, WakeRepositoryChanged, WakeTrigger,
};

#[test]
fn wake_ids_and_attention_decisions_are_deterministic_and_policy_bounded() {
    let intention = IntentionSet {
        intention_id: b"deploy-watch".to_vec(),
        objective: b"inspect repository update".to_vec(),
        trigger: Some(WakeTrigger::WakeRepositoryChanged(Box::new(
            WakeRepositoryChanged {
                repository: "hypermind".to_owned(),
            },
        ))),
        expires_at_ns: 10_000,
        reply_route: "channel:ops".to_owned(),
    };
    let signal = WakeSignal {
        trigger_lsn: 17,
        now_ns: 1_000,
        kind: "repository_changed",
        key: b"hypermind".to_vec(),
    };
    let first = evaluate(&intention, &signal).unwrap();
    let replay = evaluate(&intention, &signal).unwrap();
    assert_eq!(first.wake_id, replay.wake_id);
    assert_eq!(first.trigger_lsn, 17);

    let quiet = decide(
        &first.intention_id,
        &first.wake_id,
        factors(true, 3, 100_000),
    )
    .unwrap();
    assert_eq!(quiet.decision, AttentionDecision::Batch);
    assert!(quiet.reason.contains("quiet hours"));
    let notify = decide(
        &first.intention_id,
        &first.wake_id,
        factors(false, 3, 100_000),
    )
    .unwrap();
    assert_eq!(notify.decision, AttentionDecision::Notify);
    let exhausted = decide(
        &first.intention_id,
        &first.wake_id,
        factors(false, 0, 100_000),
    )
    .unwrap();
    assert_eq!(exhausted.decision, AttentionDecision::Remember);
    assert!(rearm(&intention, AttentionDecision::Schedule, 2_000).is_some());
}

#[test]
fn assessments_use_frozen_predicates_and_emit_bounded_revision_gaps() {
    let prediction = prediction();
    let supported = assess(
        &prediction,
        &[observation(Some(b"abc"), true, true)],
        900,
        "rules-v1",
    )
    .unwrap();
    assert_eq!(supported.assessment, OutcomeAssessment::Supported);
    assert_eq!(supported.observation_lsns, vec![42]);
    let contradicted = assess(
        &prediction,
        &[observation(Some(b"different"), true, true)],
        900,
        "rules-v1",
    )
    .unwrap();
    assert_eq!(contradicted.assessment, OutcomeAssessment::Contradicted);
    assert_eq!(
        prediction.predicates[0].expected.as_deref(),
        Some(b"abc".as_slice())
    );
    assert_eq!(
        assess(
            &prediction,
            &[observation(Some(b"abc"), false, true)],
            900,
            "rules-v1"
        )
        .unwrap()
        .assessment,
        OutcomeAssessment::NotExecuted
    );
    assert_eq!(
        assess(
            &prediction,
            &[observation(Some(b"abc"), true, false)],
            900,
            "rules-v1"
        )
        .unwrap()
        .assessment,
        OutcomeAssessment::Unresolvable
    );
    let mut missing = observation(Some(b"abc"), true, true);
    missing.kind = PredicateKind::ObjectExists;
    assert_eq!(
        assess(&prediction, &[missing], 900, "rules-v1")
            .unwrap()
            .assessment,
        OutcomeAssessment::Pending
    );
    let gap = revision_gap(3, 1, 4);
    assert!(gap.revision_required);
    assert_eq!(gap.revision_rounds_remaining, 1);
    assert_eq!(gap.probes_remaining, 1);
}

#[test]
fn procedure_ladder_requires_independent_episodes_and_user_adoption() {
    let episodes = vec![
        episode(1, 1, true),
        episode(2, 2, true),
        episode(3, 1, true),
    ];
    let procedure = mine(
        b"deploy-procedure",
        "check, deploy, verify",
        vec!["receipt committed".to_owned()],
        vec!["clean repository".to_owned()],
        &episodes,
    )
    .unwrap();
    assert!(supported(&procedure));
    assert!(!can_adopt(&procedure, Authority::DerivedInference));
    assert!(can_adopt(&procedure, Authority::UserAsserted));

    let tentative = mine(
        b"tentative",
        "check once",
        vec!["checked".to_owned()],
        Vec::new(),
        &episodes[..2],
    )
    .unwrap();
    assert!(!supported(&tentative));
}

fn factors(quiet_hours: bool, notifications_remaining: u32, workload: u32) -> AttentionFactors {
    AttentionFactors {
        urgency: 900_000,
        expected_value: 900_000,
        confidence: 900_000,
        interruption_cost: 100_000,
        resource_cost: 100_000,
        duplication_penalty: 0,
        quiet_hours,
        notifications_remaining,
        workload,
    }
}

fn prediction() -> Predicted {
    Predicted {
        prediction_id: b"prediction".to_vec(),
        revision: 1,
        task_id: Some(b"task".to_vec()),
        attempt_id: Some(b"attempt".to_vec()),
        operation_id: Some(b"operation".to_vec()),
        mechanism: "deploy".to_owned(),
        predicates: vec![ExpectedPredicate {
            kind: PredicateKind::DigestEquals,
            scope: "repository".to_owned(),
            property: Some("HEAD".to_owned()),
            expected: Some(b"abc".to_vec()),
        }],
        deadline_ns: 1_000,
        uncertainty: "remote state".to_owned(),
    }
}

fn observation(value: Option<&[u8]>, executed: bool, resolvable: bool) -> Observation {
    Observation {
        lsn: 42,
        kind: PredicateKind::DigestEquals,
        scope: "repository".to_owned(),
        property: Some("HEAD".to_owned()),
        value: value.map(<[u8]>::to_vec),
        executed,
        resolvable,
    }
}

fn episode(root: u8, conversation: u8, successful: bool) -> Episode {
    let base = u64::from(root) * 10;
    Episode {
        tool_call_lsn: base + 1,
        tool_call_id: format!("call-{root}").into_bytes(),
        tool_result_lsn: base + 2,
        tool_result_call_id: format!("call-{root}").into_bytes(),
        effect_lsn: base + 3,
        effect_tool_call_lsn: base + 1,
        effect_id: format!("effect-{root}").into_bytes(),
        outcome_lsn: base + 4,
        outcome_effect_id: format!("effect-{root}").into_bytes(),
        loop_closed_lsn: base + 5,
        loop_evidence_lsns: vec![base + 4],
        source_root: vec![root; 32],
        conversation: vec![conversation; 16],
        successful,
    }
}
