#![allow(clippy::missing_errors_doc)]

use hm_core::{Error, ErrorCode};
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio::time::{Instant, timeout_at};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LeaseLimits {
    pub maximum_active_actors: usize,
    pub maximum_heavy_jobs: usize,
    pub wait_ms: u64,
}

impl Default for LeaseLimits {
    fn default() -> Self {
        Self {
            maximum_active_actors: 64,
            maximum_heavy_jobs: 2,
            wait_ms: 250,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LeaseClass {
    Actor,
    HeavyJob,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LeaseSnapshot {
    pub active_actors: usize,
    pub active_leases: usize,
    pub heavy_jobs: usize,
    pub refusals: u64,
}

struct Pinned {
    depth: usize,
    _permit: OwnedSemaphorePermit,
}

struct Shared {
    pinned: Mutex<BTreeMap<u16, Pinned>>,
    actor_slots: Arc<Semaphore>,
    heavy_slots: Arc<Semaphore>,
    refusals: AtomicU64,
}

#[derive(Clone)]
pub struct LeaseRegistry {
    limits: LeaseLimits,
    shared: Arc<Shared>,
}

impl LeaseRegistry {
    #[must_use]
    pub fn new(limits: LeaseLimits) -> Self {
        let limits = LeaseLimits {
            maximum_active_actors: limits.maximum_active_actors.max(1),
            maximum_heavy_jobs: limits.maximum_heavy_jobs.max(1),
            wait_ms: limits.wait_ms,
        };
        Self {
            limits,
            shared: Arc::new(Shared {
                pinned: Mutex::new(BTreeMap::new()),
                actor_slots: Arc::new(Semaphore::new(limits.maximum_active_actors)),
                heavy_slots: Arc::new(Semaphore::new(limits.maximum_heavy_jobs)),
                refusals: AtomicU64::new(0),
            }),
        }
    }

    pub async fn acquire(&self, actor: u16, class: LeaseClass) -> Result<ActorLease, Error> {
        let deadline = Instant::now() + Duration::from_millis(self.limits.wait_ms);
        let heavy = match class {
            LeaseClass::Actor => None,
            LeaseClass::HeavyJob => Some(self.slot(&self.shared.heavy_slots, deadline).await?),
        };
        self.pin(actor, deadline).await?;
        Ok(ActorLease {
            shared: Arc::clone(&self.shared),
            actor,
            class,
            _heavy: heavy,
        })
    }

    #[must_use]
    pub fn is_pinned(&self, actor: u16) -> bool {
        self.shared
            .pinned
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .contains_key(&actor)
    }

    #[must_use]
    pub fn active_actors(&self) -> BTreeSet<u16> {
        self.shared
            .pinned
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .keys()
            .copied()
            .collect()
    }

    #[must_use]
    pub fn snapshot(&self) -> LeaseSnapshot {
        let pinned = self
            .shared
            .pinned
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let active_leases = pinned.values().map(|entry| entry.depth).sum();
        let active_actors = pinned.len();
        drop(pinned);
        LeaseSnapshot {
            active_actors,
            active_leases,
            heavy_jobs: self
                .limits
                .maximum_heavy_jobs
                .saturating_sub(self.shared.heavy_slots.available_permits()),
            refusals: self.shared.refusals.load(Ordering::Relaxed),
        }
    }

    #[must_use]
    pub const fn limits(&self) -> LeaseLimits {
        self.limits
    }

    async fn pin(&self, actor: u16, deadline: Instant) -> Result<(), Error> {
        {
            let mut pinned = self
                .shared
                .pinned
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(entry) = pinned.get_mut(&actor) {
                entry.depth = entry.depth.saturating_add(1);
                return Ok(());
            }
        }
        let permit = self.slot(&self.shared.actor_slots, deadline).await?;
        let mut pinned = self
            .shared
            .pinned
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match pinned.entry(actor) {
            Entry::Occupied(mut occupied) => {
                let entry = occupied.get_mut();
                entry.depth = entry.depth.saturating_add(1);
            }
            Entry::Vacant(vacant) => {
                vacant.insert(Pinned {
                    depth: 1,
                    _permit: permit,
                });
            }
        }
        Ok(())
    }

    async fn slot(
        &self,
        semaphore: &Arc<Semaphore>,
        deadline: Instant,
    ) -> Result<OwnedSemaphorePermit, Error> {
        let Ok(Ok(permit)) = timeout_at(deadline, Arc::clone(semaphore).acquire_owned()).await
        else {
            self.shared.refusals.fetch_add(1, Ordering::Relaxed);
            return Err(Error::new(ErrorCode::CapacityExceeded));
        };
        Ok(permit)
    }
}

pub struct ActorLease {
    shared: Arc<Shared>,
    actor: u16,
    class: LeaseClass,
    _heavy: Option<OwnedSemaphorePermit>,
}

impl ActorLease {
    #[must_use]
    pub const fn actor(&self) -> u16 {
        self.actor
    }

    #[must_use]
    pub const fn class(&self) -> LeaseClass {
        self.class
    }
}

impl Drop for ActorLease {
    fn drop(&mut self) {
        let mut pinned = self
            .shared
            .pinned
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Entry::Occupied(mut occupied) = pinned.entry(self.actor) {
            let entry = occupied.get_mut();
            entry.depth = entry.depth.saturating_sub(1);
            if entry.depth == 0 {
                occupied.remove();
            }
        }
    }
}
