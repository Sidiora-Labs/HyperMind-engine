#![allow(clippy::missing_errors_doc)]

use crate::bundle::{ActivationBundle, Gap, GapKind, HealthStatus};
use crate::canonical::bundle_hash;
use hm_core::{ConversationId, Error, ErrorCode, LSN};
use std::collections::BTreeMap;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const DEFAULT_ACTIVATION_DEADLINE: Duration = Duration::from_millis(25);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeadlineBundle {
    pub bundle: ActivationBundle,
    pub degraded: bool,
    pub stale_by_lsn: u64,
}

#[derive(Clone)]
pub struct DeadlineActivator {
    shared: Arc<Shared>,
}

struct Shared {
    cache: Mutex<BTreeMap<ConversationId, CachedBundle>>,
    subscribers: Mutex<Vec<Sender<ActivationBundle>>>,
}

#[derive(Clone)]
struct CachedBundle {
    built_at_lsn: LSN,
    bundle: ActivationBundle,
}

impl Default for DeadlineActivator {
    fn default() -> Self {
        Self::new()
    }
}

impl DeadlineActivator {
    #[must_use]
    pub fn new() -> Self {
        Self {
            shared: Arc::new(Shared {
                cache: Mutex::new(BTreeMap::new()),
                subscribers: Mutex::new(Vec::new()),
            }),
        }
    }

    pub fn subscribe(&self) -> Result<Receiver<ActivationBundle>, Error> {
        let (sender, receiver) = mpsc::channel();
        self.shared
            .subscribers
            .lock()
            .map_err(|_| Error::new(ErrorCode::InvariantViolation))?
            .push(sender);
        Ok(receiver)
    }

    pub fn activate<F>(
        &self,
        conversation: ConversationId,
        current_lsn: LSN,
        deadline: Duration,
        build: F,
    ) -> Result<DeadlineBundle, Error>
    where
        F: FnOnce() -> Result<ActivationBundle, Error> + Send + 'static,
    {
        if current_lsn.get() == 0 || deadline.is_zero() {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let (sender, receiver) = mpsc::sync_channel(1);
        let shared = Arc::clone(&self.shared);
        std::thread::spawn(move || {
            let result = build().and_then(|mut bundle| {
                bundle.bundle_hash = bundle_hash(&bundle)?;
                let cached = CachedBundle {
                    built_at_lsn: current_lsn,
                    bundle: bundle.clone(),
                };
                shared
                    .cache
                    .lock()
                    .map_err(|_| Error::new(ErrorCode::InvariantViolation))?
                    .insert(conversation, cached);
                let mut subscribers = shared
                    .subscribers
                    .lock()
                    .map_err(|_| Error::new(ErrorCode::InvariantViolation))?;
                subscribers.retain(|subscriber| subscriber.send(bundle.clone()).is_ok());
                Ok(bundle)
            });
            let _ = sender.send(result);
        });
        match receiver.recv_timeout(deadline) {
            Ok(result) => result.map(|bundle| DeadlineBundle {
                bundle,
                degraded: false,
                stale_by_lsn: 0,
            }),
            Err(RecvTimeoutError::Timeout) => self.cached_fallback(conversation, current_lsn),
            Err(RecvTimeoutError::Disconnected) => Err(Error::new(ErrorCode::InvariantViolation)),
        }
    }

    pub fn activate_default<F>(
        &self,
        conversation: ConversationId,
        current_lsn: LSN,
        build: F,
    ) -> Result<DeadlineBundle, Error>
    where
        F: FnOnce() -> Result<ActivationBundle, Error> + Send + 'static,
    {
        self.activate(
            conversation,
            current_lsn,
            DEFAULT_ACTIVATION_DEADLINE,
            build,
        )
    }

    fn cached_fallback(
        &self,
        conversation: ConversationId,
        current_lsn: LSN,
    ) -> Result<DeadlineBundle, Error> {
        let cached = self
            .shared
            .cache
            .lock()
            .map_err(|_| Error::new(ErrorCode::InvariantViolation))?
            .get(&conversation)
            .cloned()
            .ok_or_else(|| Error::new(ErrorCode::DeadlineMissed))?;
        let mut bundle = cached.bundle;
        let stale_by_lsn = current_lsn.get().saturating_sub(cached.built_at_lsn.get());
        bundle.health.backlog = HealthStatus::SemanticLagging;
        bundle.health.projection = HealthStatus::SemanticLagging;
        bundle.gaps.push(Gap {
            kind: GapKind::IndexLag,
            tier: None,
            lane: None,
            detail: format!(
                "activation deadline missed; cached query digest starts {:02x}{:02x}{:02x}{:02x}",
                bundle.manifest.query_digest[0],
                bundle.manifest.query_digest[1],
                bundle.manifest.query_digest[2],
                bundle.manifest.query_digest[3]
            ),
        });
        bundle.bundle_hash = bundle_hash(&bundle)?;
        Ok(DeadlineBundle {
            bundle,
            degraded: true,
            stale_by_lsn,
        })
    }
}
