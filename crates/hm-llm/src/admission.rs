use crate::{LlmError, LlmProvider, ModelTier, StructuredRequest, StructuredResponse};
use std::collections::BTreeMap;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

const SATURATED: &str = "provider admission is saturated";
const ACTOR_SHARE_EXHAUSTED: &str = "actor exceeds its provider admission share";
const WAIT_EXPIRED: &str = "provider admission wait expired";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdmissionLimits {
    pub maximum_in_flight: usize,
    pub maximum_in_flight_per_actor: usize,
    pub maximum_wait_ms: u64,
}

impl Default for AdmissionLimits {
    fn default() -> Self {
        Self {
            maximum_in_flight: 8,
            maximum_in_flight_per_actor: 2,
            maximum_wait_ms: 2_000,
        }
    }
}

impl AdmissionLimits {
    #[must_use]
    pub const fn permits(self) -> usize {
        if self.maximum_in_flight == 0 {
            1
        } else {
            self.maximum_in_flight
        }
    }

    #[must_use]
    pub const fn actor_permits(self) -> usize {
        if self.maximum_in_flight_per_actor == 0 {
            1
        } else {
            self.maximum_in_flight_per_actor
        }
    }
}

#[derive(Debug, Default)]
struct AdmissionState {
    in_flight: usize,
    per_actor: BTreeMap<u16, usize>,
    high_water_mark: usize,
    refusals: u64,
}

#[derive(Debug)]
pub struct CallAdmission {
    limits: AdmissionLimits,
    state: Mutex<AdmissionState>,
    released: Condvar,
}

impl CallAdmission {
    #[must_use]
    pub fn new(limits: AdmissionLimits) -> Self {
        Self {
            limits,
            state: Mutex::new(AdmissionState::default()),
            released: Condvar::new(),
        }
    }

    #[must_use]
    pub const fn limits(&self) -> AdmissionLimits {
        self.limits
    }

    pub fn admit(self: &Arc<Self>, actor: u16) -> Result<AdmissionPermit, LlmError> {
        let started = Instant::now();
        let wait = Duration::from_millis(self.limits.maximum_wait_ms);
        let mut state = self.locked();
        loop {
            let held = state.per_actor.get(&actor).copied().unwrap_or(0);
            let actor_saturated = held >= self.limits.actor_permits();
            let globally_saturated = state.in_flight >= self.limits.permits();
            if !actor_saturated && !globally_saturated {
                state.in_flight += 1;
                state.per_actor.insert(actor, held + 1);
                state.high_water_mark = state.high_water_mark.max(state.in_flight);
                return Ok(AdmissionPermit {
                    admission: Arc::clone(self),
                    actor,
                });
            }
            if wait.is_zero() {
                state.refusals += 1;
                return Err(LlmError::Admission(if actor_saturated {
                    ACTOR_SHARE_EXHAUSTED
                } else {
                    SATURATED
                }));
            }
            let elapsed = started.elapsed();
            if elapsed >= wait {
                state.refusals += 1;
                return Err(LlmError::Admission(WAIT_EXPIRED));
            }
            state = self
                .released
                .wait_timeout(state, wait.saturating_sub(elapsed))
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
    }

    #[must_use]
    pub fn in_flight(&self) -> usize {
        self.locked().in_flight
    }

    #[must_use]
    pub fn high_water_mark(&self) -> usize {
        self.locked().high_water_mark
    }

    #[must_use]
    pub fn refusals(&self) -> u64 {
        self.locked().refusals
    }

    fn locked(&self) -> MutexGuard<'_, AdmissionState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn release(&self, actor: u16) {
        let mut state = self.locked();
        state.in_flight = state.in_flight.saturating_sub(1);
        if let Some(held) = state.per_actor.get_mut(&actor) {
            *held = held.saturating_sub(1);
            if *held == 0 {
                state.per_actor.remove(&actor);
            }
        }
        drop(state);
        self.released.notify_all();
    }
}

#[derive(Debug)]
pub struct AdmissionPermit {
    admission: Arc<CallAdmission>,
    actor: u16,
}

impl AdmissionPermit {
    #[must_use]
    pub const fn actor(&self) -> u16 {
        self.actor
    }
}

impl Drop for AdmissionPermit {
    fn drop(&mut self) {
        self.admission.release(self.actor);
    }
}

pub struct AdmittedProvider<P> {
    admission: Arc<CallAdmission>,
    actor: u16,
    inner: P,
}

impl<P> AdmittedProvider<P> {
    #[must_use]
    pub const fn new(admission: Arc<CallAdmission>, actor: u16, inner: P) -> Self {
        Self {
            admission,
            actor,
            inner,
        }
    }

    #[must_use]
    pub const fn inner(&self) -> &P {
        &self.inner
    }
}

impl<P: LlmProvider> LlmProvider for AdmittedProvider<P> {
    fn model_id(&self) -> &str {
        self.inner.model_id()
    }

    fn tier(&self) -> ModelTier {
        self.inner.tier()
    }

    fn generate_structured(
        &self,
        request: &StructuredRequest,
    ) -> Result<StructuredResponse, LlmError> {
        let permit = self.admission.admit(self.actor)?;
        let response = self.inner.generate_structured(request);
        drop(permit);
        response
    }
}
