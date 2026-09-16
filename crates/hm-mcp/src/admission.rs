use hm_llm::admission::{AdmissionLimits, CallAdmission};
use std::sync::Arc;

#[must_use]
pub fn admission_from_env() -> Arc<CallAdmission> {
    let fallback = AdmissionLimits::default();
    Arc::new(CallAdmission::new(AdmissionLimits {
        maximum_in_flight: limit_from_env("HM_PROVIDER_MAX_IN_FLIGHT", fallback.maximum_in_flight),
        maximum_in_flight_per_actor: limit_from_env(
            "HM_PROVIDER_MAX_IN_FLIGHT_PER_ACTOR",
            fallback.maximum_in_flight_per_actor,
        ),
        maximum_wait_ms: wait_from_env("HM_PROVIDER_ADMISSION_WAIT_MS", fallback.maximum_wait_ms),
    }))
}

fn limit_from_env(name: &str, fallback: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(fallback)
}

fn wait_from_env(name: &str, fallback: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(fallback)
}
