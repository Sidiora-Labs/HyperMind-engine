#![forbid(unsafe_code)]

use hm_llm::admission::{AdmissionLimits, AdmittedProvider, CallAdmission};
use hm_llm::openai_compat::OpenAiCompatible;
use hm_llm::{
    LlmError, LlmProvider, ModelTier, Pricing, ProviderConfig, RecordedTransport, StructuredRequest,
};
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

fn supersession_request() -> StructuredRequest {
    StructuredRequest {
        prompt_id: "supersession_1".to_owned(),
        system: "Decide temporal supersession.".to_owned(),
        prompt: "old: Europe\nnew: America".to_owned(),
        json_schema: json!({
            "type": "object",
            "properties": {
                "supersedes": {"type": "boolean"},
                "reason": {"type": "string"}
            },
            "required": ["supersedes", "reason"],
            "additionalProperties": false
        }),
        maximum_output_tokens: 64,
    }
}

fn recorded_config() -> ProviderConfig {
    ProviderConfig {
        endpoint: "https://fixture.invalid/v1/chat/completions".to_owned(),
        api_key: Some("fixture-key".to_owned()),
        model: "fixture-model".to_owned(),
        tier: ModelTier::Capable,
        pricing: Pricing {
            input_microusd_per_million_tokens: 1_000_000,
            output_microusd_per_million_tokens: 2_000_000,
        },
    }
}

#[test]
fn admission_bounds_in_flight_calls_and_each_actor_share() {
    let admission = Arc::new(CallAdmission::new(AdmissionLimits {
        maximum_in_flight: 2,
        maximum_in_flight_per_actor: 1,
        maximum_wait_ms: 0,
    }));
    let first = admission.admit(7).unwrap();
    assert_eq!(first.actor(), 7);
    assert_eq!(
        admission.admit(7).unwrap_err(),
        LlmError::Admission("actor exceeds its provider admission share")
    );
    let second = admission.admit(8).unwrap();
    assert_eq!(admission.in_flight(), 2);
    assert_eq!(
        admission.admit(9).unwrap_err(),
        LlmError::Admission("provider admission is saturated")
    );
    drop(first);
    let third = admission.admit(9).unwrap();
    assert_eq!(admission.in_flight(), 2);
    assert_eq!(admission.refusals(), 2);
    assert_eq!(admission.high_water_mark(), 2);
    drop(second);
    drop(third);
    assert_eq!(admission.in_flight(), 0);
}

#[test]
fn a_zero_ceiling_serialises_rather_than_deadlocking() {
    let limits = AdmissionLimits {
        maximum_in_flight: 0,
        maximum_in_flight_per_actor: 0,
        maximum_wait_ms: 0,
    };
    assert_eq!(limits.permits(), 1);
    assert_eq!(limits.actor_permits(), 1);
    assert_eq!(AdmissionLimits::default().permits(), 8);
    assert_eq!(AdmissionLimits::default().actor_permits(), 2);
    assert_eq!(AdmissionLimits::default().maximum_wait_ms, 2_000);

    let admission = Arc::new(CallAdmission::new(limits));
    assert_eq!(admission.limits(), limits);
    let held = admission.admit(3).unwrap();
    assert_eq!(admission.in_flight(), 1);
    assert!(admission.admit(4).is_err());
    assert!(admission.admit(3).is_err());
    drop(held);
    let next = admission.admit(4).unwrap();
    assert_eq!(admission.in_flight(), 1);
    drop(next);
    assert_eq!(admission.in_flight(), 0);
    assert_eq!(admission.high_water_mark(), 1);
    assert_eq!(admission.refusals(), 2);
}

#[test]
fn a_bounded_wait_expires_rather_than_queueing_forever() {
    let admission = Arc::new(CallAdmission::new(AdmissionLimits {
        maximum_in_flight: 1,
        maximum_in_flight_per_actor: 1,
        maximum_wait_ms: 60,
    }));
    let held = admission.admit(1).unwrap();
    let started = Instant::now();
    let refused = admission.admit(2).unwrap_err();
    let waited = started.elapsed();
    assert_eq!(
        refused,
        LlmError::Admission("provider admission wait expired")
    );
    assert!(waited >= Duration::from_millis(50), "waited {waited:?}");
    assert_eq!(admission.refusals(), 1);
    drop(held);
    let next = admission.admit(2).unwrap();
    assert_eq!(next.actor(), 2);
}

#[test]
fn concurrent_callers_never_exceed_the_ceiling() {
    let admission = Arc::new(CallAdmission::new(AdmissionLimits {
        maximum_in_flight: 2,
        maximum_in_flight_per_actor: 2,
        maximum_wait_ms: 2_000,
    }));
    let admitted = Arc::new(AtomicUsize::new(0));
    let workers: Vec<_> = (0..8_u16)
        .map(|index| {
            let admission = Arc::clone(&admission);
            let admitted = Arc::clone(&admitted);
            thread::spawn(move || {
                if let Ok(permit) = admission.admit(index % 2) {
                    let mut checksum = 0_u64;
                    for step in 0..10_000_u64 {
                        checksum = checksum.wrapping_add(step.wrapping_mul(u64::from(index) + 1));
                    }
                    assert!(checksum > 0);
                    admitted.fetch_add(1, Ordering::SeqCst);
                    drop(permit);
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    assert_eq!(admission.in_flight(), 0);
    assert!(
        admission.high_water_mark() <= 2,
        "high water mark {}",
        admission.high_water_mark()
    );
    let successes = u64::try_from(admitted.load(Ordering::SeqCst)).unwrap();
    assert_eq!(successes + admission.refusals(), 8);
}

#[test]
fn an_admitted_provider_passes_the_recorded_call_through_and_releases_its_permit() {
    let admission = Arc::new(CallAdmission::new(AdmissionLimits::default()));
    let transport = RecordedTransport::from_json(include_str!("fixtures/openai.json")).unwrap();
    let provider = AdmittedProvider::new(
        Arc::clone(&admission),
        7,
        OpenAiCompatible::new(recorded_config(), transport).unwrap(),
    );
    assert_eq!(provider.model_id(), "fixture-model");
    assert_eq!(provider.tier(), ModelTier::Capable);
    let response = provider
        .generate_structured(&supersession_request())
        .unwrap();
    assert_eq!(response.model_id, "fixture-model");
    assert_eq!(response.value["supersedes"], true);
    assert_eq!(response.value["reason"], "later interval");
    assert_eq!(response.usage.input_tokens, 10);
    assert_eq!(response.usage.output_tokens, 4);
    assert_eq!(response.usage.cache_read_tokens, 2);
    assert_eq!(response.usage.cost_microusd, 18);
    assert_eq!(admission.in_flight(), 0);
    assert_eq!(admission.high_water_mark(), 1);
    assert_eq!(provider.inner().transport().remaining(), 0);
}
