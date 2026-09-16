#![allow(clippy::missing_errors_doc)]

use crate::runs::RunsProjection;
use crate::store::ReadSnapshot;
use hm_core::{Error, ErrorCode};
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

const NANOS_PER_SECOND: i64 = 1_000_000_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GenerationLease {
    pub lease_id: u64,
    pub generation: u64,
    pub expires_at_ns: i64,
}

pub struct LeaseManager {
    next_id: AtomicU64,
    leases: Mutex<BTreeMap<u64, GenerationLease>>,
}

impl Default for LeaseManager {
    fn default() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            leases: Mutex::new(BTreeMap::new()),
        }
    }
}

impl LeaseManager {
    pub fn lease(
        &self,
        snapshot: &ReadSnapshot<'_>,
        generation: u64,
        now_ns: i64,
        seconds: u32,
    ) -> Result<GenerationLease, Error> {
        if seconds == 0 || !RunsProjection::is_published(snapshot, generation)? {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let duration = i64::from(seconds)
            .checked_mul(NANOS_PER_SECOND)
            .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?;
        let expires_at_ns = now_ns
            .checked_add(duration)
            .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?;
        let lease_id = self.next_id.fetch_add(1, Ordering::Relaxed);
        if lease_id == 0 {
            return Err(Error::new(ErrorCode::CapacityExceeded));
        }
        let lease = GenerationLease {
            lease_id,
            generation,
            expires_at_ns,
        };
        self.leases
            .lock()
            .map_err(|_| Error::new(ErrorCode::InvariantViolation))?
            .insert(lease_id, lease);
        Ok(lease)
    }

    pub fn resolve(&self, lease_id: u64, now_ns: i64) -> Result<GenerationLease, Error> {
        let mut leases = self
            .leases
            .lock()
            .map_err(|_| Error::new(ErrorCode::InvariantViolation))?;
        let lease = leases
            .get(&lease_id)
            .copied()
            .ok_or_else(|| Error::new(ErrorCode::OperationUnavailable))?;
        if lease.expires_at_ns <= now_ns {
            leases.remove(&lease_id);
            Err(Error::new(ErrorCode::OperationUnavailable))
        } else {
            Ok(lease)
        }
    }

    pub fn revoke(&self, lease_id: u64) -> Result<bool, Error> {
        self.leases
            .lock()
            .map_err(|_| Error::new(ErrorCode::InvariantViolation))
            .map(|mut leases| leases.remove(&lease_id).is_some())
    }
}
