use crate::{
    development::{DevelopmentKind, SnapshotRequest},
    maintenance::{
        JobKind, JobLease, JobStatus, MaintenanceScheduler, SchedulerConfig, SchedulerSnapshot,
        Usage,
    },
    types::{ContextError, Cursor, Scope, digest_bytes, validate_id},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum ScheduleMode {
    Disabled,
    Manual,
    Timed { interval_ms: u64 },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DevelopmentSchedule {
    pub id: String,
    pub kind: DevelopmentKind,
    pub mode: ScheduleMode,
    pub worker_id: String,
    pub principal: Scope,
    pub snapshot: SnapshotRequest,
    pub reservation: u64,
    pub timeout_ms: u64,
    pub backoff_ms: u64,
    pub max_attempts: u64,
    pub identical_failure_limit: u64,
    pub provider_policy: Option<String>,
}
impl DevelopmentSchedule {
    pub fn validate(&self) -> Result<(), ContextError> {
        validate_id(&self.id)?;
        validate_id(&self.worker_id)?;
        validate_id(&self.snapshot.capability_id)?;
        self.principal.validate()?;
        for id in self
            .snapshot
            .source_ids
            .iter()
            .chain(&self.snapshot.record_ids)
        {
            validate_id(id)?;
        }
        if self.reservation == 0
            || self.timeout_ms == 0
            || self.timeout_ms > 60_000
            || self.max_attempts == 0
            || self.max_attempts > 16
            || self.identical_failure_limit == 0
            || matches!(self.mode, ScheduleMode::Timed { interval_ms: 0 })
        {
            return Err(ContextError::Invalid("invalid schedule bounds".into()));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DispatchLease {
    pub job_id: String,
    pub attempt: u64,
    pub expires_ms: u64,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum DispatchStatus {
    Pending,
    Running { lease: DispatchLease },
    Complete { receipt_id: String },
    Cancelled,
    Exhausted,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DispatchJob {
    pub id: String,
    pub schedule_id: String,
    pub input_digest: String,
    pub accounting_id: String,
    pub ordinal: u64,
    pub attempt: u64,
    pub ready_ms: u64,
    pub status: DispatchStatus,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ScheduleProgress {
    pub next_due_ms: u64,
    pub registered: u64,
    pub completed_frontier: u64,
    pub last_error: Option<String>,
    pub last_error_digest: Option<String>,
    pub identical_failures: u64,
    pub circuit_open: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DevelopmentSchedules {
    pub version: u32,
    pub scope: Scope,
    pub sequence: u64,
    pub budget: u64,
    pub max_concurrency: usize,
    pub schedules: BTreeMap<String, DevelopmentSchedule>,
    pub progress: BTreeMap<String, ScheduleProgress>,
    pub jobs: BTreeMap<String, DispatchJob>,
    pub accounting: SchedulerSnapshot,
}
impl DevelopmentSchedules {
    pub fn new(scope: Scope, budget: u64, max_concurrency: usize) -> Result<Self, ContextError> {
        scope.validate()?;
        if budget == 0 || max_concurrency == 0 || max_concurrency > 16 {
            return Err(ContextError::Capacity);
        }
        Ok(Self {
            version: 1,
            accounting: MaintenanceScheduler::new(
                scope.clone(),
                SchedulerConfig {
                    max_concurrency,
                    budget,
                    lease_ms: 60_000,
                    backoff_ms: 0,
                    max_attempts: 16,
                },
            )?
            .snapshot(),
            scope,
            sequence: 0,
            budget,
            max_concurrency,
            schedules: BTreeMap::new(),
            progress: BTreeMap::new(),
            jobs: BTreeMap::new(),
        })
    }
    pub fn configure(
        &mut self,
        schedule: DevelopmentSchedule,
        now: u64,
    ) -> Result<(), ContextError> {
        schedule.validate()?;
        if self.jobs.values().any(|job| {
            job.schedule_id == schedule.id && matches!(job.status, DispatchStatus::Running { .. })
        }) {
            return Err(ContextError::Conflict);
        }
        if self.jobs.values().any(|job| {
            job.schedule_id == schedule.id && matches!(job.status, DispatchStatus::Pending)
        }) {
            let mut previous = self
                .schedules
                .get(&schedule.id)
                .ok_or(ContextError::Stale)?
                .clone();
            previous.mode = schedule.mode.clone();
            if serde_json::to_vec(&previous)? != serde_json::to_vec(&schedule)? {
                return Err(ContextError::Conflict);
            }
        }
        let progress = self.progress.entry(schedule.id.clone()).or_default();
        progress.next_due_ms = now;
        progress.circuit_open = false;
        progress.identical_failures = 0;
        progress.last_error = None;
        progress.last_error_digest = None;
        self.schedules.insert(schedule.id.clone(), schedule);
        Ok(())
    }
    pub fn enqueue(
        &mut self,
        schedule_id: &str,
        input_digest: String,
        now: u64,
    ) -> Result<String, ContextError> {
        let schedule = self
            .schedules
            .get(schedule_id)
            .ok_or_else(|| ContextError::Unavailable("schedule".into()))?;
        if matches!(schedule.mode, ScheduleMode::Disabled) {
            return Err(ContextError::Unavailable("schedule disabled".into()));
        }
        if input_digest.len() != 64 || !input_digest.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(ContextError::Invalid("input digest".into()));
        }
        let id = digest_bytes(&serde_json::to_vec(&(schedule_id, &input_digest))?);
        if self.jobs.contains_key(&id) {
            return Ok(id);
        }
        let progress = self
            .progress
            .get_mut(schedule_id)
            .ok_or(ContextError::Stale)?;
        progress.registered = progress
            .registered
            .checked_add(1)
            .ok_or(ContextError::Capacity)?;
        let mut accounting = MaintenanceScheduler::from_snapshot(self.accounting.clone())?;
        let accounting_id = accounting.enqueue_evidence(
            job_kind(schedule.kind),
            digest_bytes(format!("{schedule_id}:{input_digest}").as_bytes()),
            Cursor {
                epoch: 1,
                sequence: progress.registered,
            },
            1,
            schedule.reservation,
        )?;
        self.accounting = accounting.snapshot();
        self.jobs.insert(
            id.clone(),
            DispatchJob {
                id: id.clone(),
                schedule_id: schedule_id.into(),
                input_digest,
                accounting_id,
                ordinal: progress.registered,
                attempt: 0,
                ready_ms: now,
                status: DispatchStatus::Pending,
            },
        );
        Ok(id)
    }
    pub fn claim(&mut self, id: &str, now: u64) -> Result<DispatchLease, ContextError> {
        if self
            .jobs
            .values()
            .filter(|j| matches!(j.status, DispatchStatus::Running { .. }))
            .count()
            >= self.max_concurrency
        {
            return Err(ContextError::Capacity);
        }
        // A project has one mutation lease even when unrelated providers can operate elsewhere.
        if self
            .jobs
            .values()
            .any(|j| matches!(j.status, DispatchStatus::Running { .. }))
        {
            return Err(ContextError::Conflict);
        }
        let job = self.jobs.get(id).ok_or(ContextError::Stale)?;
        let schedule = &self.schedules[&job.schedule_id];
        if self.progress[&job.schedule_id].circuit_open
            || matches!(schedule.mode, ScheduleMode::Disabled)
            || job.status != DispatchStatus::Pending
            || job.ready_ms > now
            || job.attempt >= schedule.max_attempts
        {
            return Err(ContextError::Stale);
        }
        let mut accounting = MaintenanceScheduler::from_snapshot(self.accounting.clone())?;
        let accounting_lease = accounting
            .claim_job(&job.accounting_id, now, schedule.timeout_ms)?
            .ok_or(ContextError::Capacity)?;
        self.accounting = accounting.snapshot();
        let job = self.jobs.get_mut(id).unwrap();
        job.attempt += 1;
        let lease = DispatchLease {
            job_id: id.into(),
            attempt: job.attempt,
            expires_ms: now
                .checked_add(schedule.timeout_ms)
                .ok_or(ContextError::Capacity)?,
        };
        if accounting_lease.attempt != lease.attempt {
            return Err(ContextError::Stale);
        }
        job.status = DispatchStatus::Running {
            lease: lease.clone(),
        };
        Ok(lease)
    }
    pub fn due(&self, now: u64) -> Vec<String> {
        self.schedules
            .values()
            .filter(|s| {
                matches!(s.mode, ScheduleMode::Timed { .. })
                    && !self.progress[&s.id].circuit_open
                    && self.progress[&s.id].next_due_ms <= now
            })
            .map(|s| s.id.clone())
            .collect()
    }
    pub fn mark_due(&mut self, id: &str, now: u64) {
        if let ScheduleMode::Timed { interval_ms } = self.schedules[id].mode {
            self.progress.get_mut(id).unwrap().next_due_ms = now.saturating_add(interval_ms);
        }
    }
    pub fn renew(
        &mut self,
        lease: &DispatchLease,
        now: u64,
    ) -> Result<DispatchLease, ContextError> {
        self.validate_lease(lease, now)?;
        let schedule_id = self.jobs[&lease.job_id].schedule_id.clone();
        let timeout = self.schedules[&schedule_id].timeout_ms;
        let active = self.accounting_lease(lease)?;
        let mut accounting = MaintenanceScheduler::from_snapshot(self.accounting.clone())?;
        let active = accounting.heartbeat(&active, now, timeout)?;
        self.accounting = accounting.snapshot();
        let lease = DispatchLease {
            expires_ms: active.expires_ms,
            ..lease.clone()
        };
        self.jobs.get_mut(&lease.job_id).unwrap().status = DispatchStatus::Running {
            lease: lease.clone(),
        };
        Ok(lease)
    }
    pub fn charge_key(lease: &DispatchLease) -> String {
        format!("{}:{}", lease.job_id, lease.attempt)
    }
    pub fn validate_lease(&self, lease: &DispatchLease, now: u64) -> Result<(), ContextError> {
        if now >= lease.expires_ms
            || self.jobs.get(&lease.job_id).is_none_or(|j| {
                j.status
                    != DispatchStatus::Running {
                        lease: lease.clone(),
                    }
            })
        {
            return Err(ContextError::Stale);
        }
        Ok(())
    }
    fn accounting_lease(&self, lease: &DispatchLease) -> Result<JobLease, ContextError> {
        let job = self.jobs.get(&lease.job_id).ok_or(ContextError::Stale)?;
        let record = self
            .accounting
            .jobs
            .get(&job.accounting_id)
            .ok_or(ContextError::Stale)?;
        match &record.status {
            JobStatus::Running(active) if active.attempt == lease.attempt => Ok(active.clone()),
            _ => Err(ContextError::Stale),
        }
    }
    fn settle(
        &mut self,
        lease: &DispatchLease,
        usage: Usage,
        complete: bool,
        now: u64,
        receipt: &str,
    ) -> Result<(), ContextError> {
        let active = self.accounting_lease(lease)?;
        let mut accounting = MaintenanceScheduler::from_snapshot(self.accounting.clone())?;
        if complete {
            accounting.complete(
                &active,
                &active.fence,
                usage,
                &digest_bytes(receipt.as_bytes()),
                now,
            )?;
        } else {
            accounting.fail(&active, usage, now)?;
        }
        self.accounting = accounting.snapshot();
        Ok(())
    }
    pub fn finish(
        &mut self,
        lease: &DispatchLease,
        usage: Usage,
        receipt_id: String,
        now: u64,
    ) -> Result<(), ContextError> {
        self.validate_lease(lease, now)?;
        self.settle(lease, usage, true, now, &receipt_id)?;
        let job = self.jobs.get_mut(&lease.job_id).unwrap();
        job.status = DispatchStatus::Complete { receipt_id };
        let schedule = job.schedule_id.clone();
        self.frontier(&schedule);
        let progress = self.progress.get_mut(&schedule).unwrap();
        progress.last_error = None;
        progress.last_error_digest = None;
        progress.identical_failures = 0;
        Ok(())
    }
    pub fn fail(
        &mut self,
        lease: &DispatchLease,
        usage: Usage,
        error: String,
        now: u64,
    ) -> Result<(), ContextError> {
        self.validate_lease(lease, now)?;
        self.fail_unchecked(lease, usage, error, now)
    }
    fn fail_unchecked(
        &mut self,
        lease: &DispatchLease,
        usage: Usage,
        error: String,
        now: u64,
    ) -> Result<(), ContextError> {
        self.settle(lease, usage, false, now, "")?;
        let job = self
            .jobs
            .get_mut(&lease.job_id)
            .ok_or(ContextError::Stale)?;
        let schedule = &self.schedules[&job.schedule_id];
        job.status = if job.attempt >= schedule.max_attempts {
            DispatchStatus::Exhausted
        } else {
            DispatchStatus::Pending
        };
        job.ready_ms = now.saturating_add(schedule.backoff_ms);
        let progress = self.progress.get_mut(&job.schedule_id).unwrap();
        let digest = digest_bytes(error.as_bytes());
        progress.identical_failures = if progress.last_error_digest.as_ref() == Some(&digest) {
            progress.identical_failures + 1
        } else {
            1
        };
        progress.last_error_digest = Some(digest);
        progress.last_error = Some(error.chars().take(2048).collect());
        progress.circuit_open = progress.identical_failures >= schedule.identical_failure_limit;
        Ok(())
    }
    pub fn expire(&mut self, now: u64) -> Result<(), ContextError> {
        let leases: Vec<_> = self
            .jobs
            .values()
            .filter_map(|j| match &j.status {
                DispatchStatus::Running { lease } if lease.expires_ms <= now => Some(lease.clone()),
                _ => None,
            })
            .collect();
        for lease in leases {
            self.fail_unchecked(
                &lease,
                Usage::Unknown,
                "lease expired".into(),
                lease.expires_ms.saturating_sub(1),
            )?;
        }
        Ok(())
    }
    pub fn cancel(&mut self, id: &str) -> Result<(), ContextError> {
        let job = self.jobs.get(id).ok_or(ContextError::Stale)?;
        if let DispatchStatus::Running { lease } = &job.status {
            let lease = lease.clone();
            self.settle(
                &lease,
                Usage::Unknown,
                false,
                lease.expires_ms.saturating_sub(1),
                "",
            )?;
        }
        let job = self.jobs.get_mut(id).unwrap();
        if matches!(job.status, DispatchStatus::Complete { .. }) {
            return Err(ContextError::Stale);
        }
        job.status = DispatchStatus::Cancelled;
        Ok(())
    }
    pub fn settle_unknown(&mut self, key: &str, actual: u64) -> Result<(), ContextError> {
        let (id, attempt) = key.rsplit_once(':').ok_or(ContextError::Stale)?;
        let job = self.jobs.get(id).ok_or(ContextError::Stale)?;
        let mut accounting = MaintenanceScheduler::from_snapshot(self.accounting.clone())?;
        accounting.settle_unknown(
            &job.accounting_id,
            attempt.parse().map_err(|_| ContextError::Stale)?,
            actual,
        )?;
        self.accounting = accounting.snapshot();
        Ok(())
    }
    pub fn validate(&self) -> Result<(), ContextError> {
        self.scope.validate()?;
        if self.version != 1 || self.accounting.scope != self.scope {
            return Err(ContextError::ScopeMismatch);
        }
        MaintenanceScheduler::from_snapshot(self.accounting.clone())?;
        for (id, schedule) in &self.schedules {
            schedule.validate()?;
            if id != &schedule.id || !self.progress.contains_key(id) {
                return Err(ContextError::Stale);
            }
        }
        for (id, job) in &self.jobs {
            if id != &job.id
                || !self.schedules.contains_key(&job.schedule_id)
                || !self.accounting.jobs.contains_key(&job.accounting_id)
            {
                return Err(ContextError::Stale);
            }
        }
        Ok(())
    }
    fn frontier(&mut self, schedule_id: &str) {
        let mut jobs: Vec<_> = self
            .jobs
            .values()
            .filter(|j| j.schedule_id == schedule_id)
            .collect();
        jobs.sort_by_key(|j| j.ordinal);
        let mut frontier = 0;
        for job in jobs {
            if !matches!(job.status, DispatchStatus::Complete { .. }) {
                break;
            }
            frontier = job.ordinal;
        }
        self.progress
            .get_mut(schedule_id)
            .unwrap()
            .completed_frontier = frontier;
    }
}

fn job_kind(kind: DevelopmentKind) -> JobKind {
    match kind {
        DevelopmentKind::Historian => JobKind::Historian,
        DevelopmentKind::Extraction => JobKind::Extraction,
        DevelopmentKind::Indexing => JobKind::Indexing,
        DevelopmentKind::Verification => JobKind::Verification,
        DevelopmentKind::Curation => JobKind::Curation,
        DevelopmentKind::Retrospective => JobKind::Retrospective,
        DevelopmentKind::Primer => JobKind::Primer,
        DevelopmentKind::ConditionalNote => JobKind::ConditionalNote,
        DevelopmentKind::ProfileProposal => JobKind::ProfileProposal,
        DevelopmentKind::DocumentationProposal => JobKind::DocumentationProposal,
    }
}
