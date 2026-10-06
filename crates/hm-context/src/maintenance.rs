use crate::types::{digest_bytes, Authority, ContextError, Cursor, Scope, SourceMessage};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs::{self, File}, io::Write, path::{Path, PathBuf}};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobKind { Historian, Verification, Curation, Extraction, Indexing, Consolidation, Retrospective, Primer, ConditionalNote, ProfileProposal, DocumentationProposal }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SchedulerConfig {
    pub max_concurrency: usize,
    pub budget: u64,
    pub lease_ms: u64,
    pub backoff_ms: u64,
    pub max_attempts: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobRequest {
    pub kind: JobKind,
    pub sources: Vec<SourceMessage>,
    pub cursor: Cursor,
    pub source_revision: u64,
    pub policy_revision: u64,
    pub reservation: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PublicationFence {
    pub input_digest: String,
    pub source_revision: u64,
    pub policy_revision: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct JobLease {
    pub job_id: String,
    pub attempt: u64,
    pub expires_ms: u64,
    pub fence: PublicationFence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Usage { Known(u64), Unknown }
impl Default for Usage { fn default() -> Self { Self::Unknown } }

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum JobStatus { Pending, Running(JobLease), Complete, NoWork, Cancelled, Exhausted }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JobRecord {
    pub id: String,
    pub request: JobRequest,
    pub fence: PublicationFence,
    pub status: JobStatus,
    pub attempts: u64,
    pub available_ms: u64,
    pub output_digest: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PublicationReceipt {
    pub job_id: String,
    pub kind: JobKind,
    pub cursor: Cursor,
    pub output_digest: String,
    pub authority: Authority,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SchedulerSnapshot {
    pub version: u32,
    pub scope: Scope,
    pub config: SchedulerConfig,
    pub jobs: BTreeMap<String, JobRecord>,
    pub watermarks: BTreeMap<JobKind, Cursor>,
    #[serde(default)]
    pub watermark_gaps: BTreeMap<JobKind, Vec<Cursor>>,
    pub spent: u64,
    pub unknown_usage: BTreeMap<String, u64>,
}

pub struct MaintenanceScheduler { path: Option<PathBuf>, state: SchedulerSnapshot }

impl MaintenanceScheduler {
    pub fn enqueue_evidence(&mut self, kind: JobKind, input_digest: String, cursor: Cursor, policy_revision: u64, reservation: u64) -> Result<String, ContextError> {
        cursor.validate()?;
        if reservation == 0 || reservation > self.state.config.budget || input_digest.len() != 64 || !input_digest.bytes().all(|b| b.is_ascii_hexdigit()) { return Err(ContextError::Invalid("invalid evidence reservation".into())); }
        let fence = PublicationFence { input_digest, source_revision: cursor.sequence, policy_revision };
        let id = digest_bytes(&serde_json::to_vec(&(&self.state.scope, kind, &fence))?);
        if let Some(existing) = self.state.jobs.get(&id) { if existing.request.cursor != cursor || existing.request.reservation != reservation { return Err(ContextError::Conflict); } return Ok(id); }
        self.transaction(|state| { state.jobs.insert(id.clone(), JobRecord { id: id.clone(), request: JobRequest { kind, sources: vec![], cursor, source_revision: fence.source_revision, policy_revision, reservation }, fence, status: JobStatus::Pending, attempts: 0, available_ms: 0, output_digest: None }); update_watermark(state, kind); Ok(id) })
    }
    pub fn open(path: impl AsRef<Path>, scope: Scope, config: SchedulerConfig) -> Result<Self, ContextError> {
        scope.validate()?;
        if config.max_concurrency == 0 || config.lease_ms == 0 || config.max_attempts == 0 || config.budget == 0 {
            return Err(ContextError::Invalid("invalid scheduler bounds".into()));
        }
        let path = path.as_ref().to_path_buf();
        let state = if path.exists() {
            let state: SchedulerSnapshot = serde_json::from_slice(&fs::read(&path)?)?;
            if state.scope != scope { return Err(ContextError::ScopeMismatch); }
            if state.version != 1 { return Err(ContextError::Invalid("unsupported scheduler version".into())); }
            if serde_json::to_vec(&state.config)? != serde_json::to_vec(&config)? { return Err(ContextError::Conflict); }
            state
        } else {
            SchedulerSnapshot { version: 1, scope, config, jobs: BTreeMap::new(), watermarks: BTreeMap::new(), watermark_gaps: BTreeMap::new(), spent: 0, unknown_usage: BTreeMap::new() }
        };
        let mut scheduler = Self::from_snapshot(state)?;
        scheduler.path = Some(path);
        scheduler.persist()?;
        Ok(scheduler)
    }

    pub fn new(scope: Scope, config: SchedulerConfig) -> Result<Self, ContextError> {
        Self::from_snapshot(SchedulerSnapshot { version: 1, scope, config, jobs: BTreeMap::new(), watermarks: BTreeMap::new(), watermark_gaps: BTreeMap::new(), spent: 0, unknown_usage: BTreeMap::new() })
    }

    pub fn from_snapshot(mut state: SchedulerSnapshot) -> Result<Self, ContextError> {
        state.scope.validate()?;
        if state.version != 1 || state.config.max_concurrency == 0 || state.config.budget == 0 || state.config.lease_ms == 0 || state.config.max_attempts == 0 { return Err(ContextError::Invalid("invalid scheduler snapshot".into())); }
        for (id, job) in &state.jobs {
            job.request.cursor.validate()?;
            for source in &job.request.sources { source.validate()?; }
            if id != &job.id || job.attempts > state.config.max_attempts { return Err(ContextError::Invalid("invalid job snapshot".into())); }
        }
        committed_budget(&state)?;
        for kind in [JobKind::Historian,JobKind::Verification,JobKind::Curation,JobKind::Extraction,JobKind::Indexing,JobKind::Consolidation,JobKind::Retrospective,JobKind::Primer,JobKind::ConditionalNote,JobKind::ProfileProposal,JobKind::DocumentationProposal] { update_watermark(&mut state,kind); }
        Ok(Self { path: None, state })
    }

    pub fn snapshot(&self) -> SchedulerSnapshot { self.state.clone() }

    pub fn enqueue(&mut self, request: JobRequest) -> Result<String, ContextError> {
        request.cursor.validate()?;
        if request.sources.is_empty() || request.reservation == 0 || request.reservation > self.state.config.budget {
            return Err(ContextError::Invalid("invalid job input or reservation".into()));
        }
        let mut identities = BTreeMap::new();
        for source in &request.sources {
            source.validate()?;
            if source.authority == Authority::DerivedInference || (source.authority == Authority::AssistantGenerated && request.kind != JobKind::Historian) {
                return Err(ContextError::Invalid("derived output is not a fresh observation".into()));
            }
            if identities.insert(source.id.clone(), source.source_digest.clone()).is_some() {
                return Err(ContextError::Conflict);
            }
        }
        let input_digest = digest_bytes(&serde_json::to_vec(&identities)?);
        let fence = PublicationFence { input_digest, source_revision: request.source_revision, policy_revision: request.policy_revision };
        let id = digest_bytes(&serde_json::to_vec(&(&self.state.scope, request.kind, &fence))?);
        if let Some(existing) = self.state.jobs.get(&id) {
            if existing.request.cursor != request.cursor || existing.request.reservation != request.reservation { return Err(ContextError::Conflict); }
            return Ok(id);
        }
        if self.state.watermarks.get(&request.kind).is_some_and(|cursor| *cursor > request.cursor) { return Err(ContextError::Stale); }
        self.transaction(|state| {
            let kind = request.kind;
            state.jobs.insert(id.clone(), JobRecord { id: id.clone(), request, fence, status: JobStatus::Pending, attempts: 0, available_ms: 0, output_digest: None });
            update_watermark(state, kind);
            Ok(id)
        })
    }

    pub fn claim(&mut self, now_ms: u64) -> Result<Option<JobLease>, ContextError> {
        self.claim_selected(now_ms, None, None, self.state.config.lease_ms)
    }

    pub fn claim_job(&mut self, id: &str, now_ms: u64, lease_ms: u64) -> Result<Option<JobLease>, ContextError> {
        if !self.state.jobs.contains_key(id) { return Err(ContextError::Stale); }
        self.claim_selected(now_ms, Some(id), None, lease_ms)
    }

    pub fn claim_non_historian(&mut self, now_ms: u64) -> Result<Option<JobLease>, ContextError> {
        self.claim_selected(now_ms, None, Some(JobKind::Historian), self.state.config.lease_ms)
    }

    fn claim_selected(&mut self, now_ms: u64, id: Option<&str>, excluded_kind: Option<JobKind>, lease_ms: u64) -> Result<Option<JobLease>, ContextError> {
        if lease_ms == 0 { return Err(ContextError::Invalid("zero lease".into())); }
        self.transaction(|state| {
            expire_attempts(state, now_ms)?;
            let running = state.jobs.values().filter(|job| matches!(job.status, JobStatus::Running(_))).count();
            if running >= state.config.max_concurrency { return Ok(None); }
            let committed = committed_budget(state)?;
            let candidate = state.jobs.values().filter(|job| job.status == JobStatus::Pending && job.available_ms <= now_ms)
                .filter(|job| id.is_none_or(|id|job.id == id) && excluded_kind != Some(job.request.kind))
                .filter(|job| job.request.reservation <= state.config.budget.saturating_sub(committed))
                .min_by_key(|job| (job.available_ms, job.attempts, job.id.clone())).map(|job| job.id.clone());
            let Some(id) = candidate else { return Ok(None); };
            let job = state.jobs.get_mut(&id).ok_or(ContextError::Stale)?;
            job.attempts = job.attempts.checked_add(1).ok_or(ContextError::Capacity)?;
            let lease = JobLease { job_id: id, attempt: job.attempts, expires_ms: now_ms.checked_add(lease_ms).ok_or(ContextError::Capacity)?, fence: job.fence.clone() };
            job.status = JobStatus::Running(lease.clone());
            Ok(Some(lease))
        })
    }

    pub fn heartbeat(&mut self, lease: &JobLease, now_ms: u64, lease_ms: u64) -> Result<JobLease, ContextError> {
        if lease_ms == 0 { return Err(ContextError::Invalid("zero lease".into())); }
        self.transaction(|state| {
            validate_lease(state, lease, now_ms)?;
            let mut renewed = lease.clone();
            renewed.expires_ms = now_ms.checked_add(lease_ms).ok_or(ContextError::Capacity)?;
            state.jobs.get_mut(&lease.job_id).ok_or(ContextError::Stale)?.status = JobStatus::Running(renewed.clone());
            Ok(renewed)
        })
    }

    pub fn expire(&mut self, now_ms: u64) -> Result<usize, ContextError> {
        self.transaction(|state| expire_attempts(state, now_ms))
    }

    pub fn complete(&mut self, lease: &JobLease, current: &PublicationFence, usage: Usage, output_digest: &str, now_ms: u64) -> Result<PublicationReceipt, ContextError> {
        if output_digest.len() != 64 || !output_digest.bytes().all(|byte| byte.is_ascii_hexdigit()) { return Err(ContextError::Invalid("invalid output digest".into())); }
        self.transaction(|state| {
            validate_lease(state, lease, now_ms)?;
            if current != &lease.fence { return Err(ContextError::Stale); }
            finish_attempt(state, lease, usage, now_ms, true)?;
            let job = state.jobs.get_mut(&lease.job_id).ok_or(ContextError::Stale)?;
            job.output_digest = Some(output_digest.into());
            let receipt = PublicationReceipt { job_id: job.id.clone(), kind: job.request.kind, cursor: job.request.cursor, output_digest: output_digest.into(), authority: Authority::DerivedInference };
            update_watermark(state, receipt.kind);
            Ok(receipt)
        })
    }

    pub fn settle_no_work(&mut self, lease: &JobLease, now_ms: u64) -> Result<(), ContextError> {
        self.transaction(|state| {
            validate_lease(state, lease, now_ms)?;
            let job = state.jobs.get_mut(&lease.job_id).ok_or(ContextError::Stale)?;
            job.status = JobStatus::NoWork;
            Ok(())
        })
    }

    pub fn fail(&mut self, lease: &JobLease, usage: Usage, now_ms: u64) -> Result<(), ContextError> {
        self.transaction(|state| { validate_lease(state, lease, now_ms)?; finish_attempt(state, lease, usage, now_ms, false) })
    }

    pub fn cancel(&mut self, id: &str, now_ms: u64) -> Result<(), ContextError> {
        self.transaction(|state| {
            let status = state.jobs.get(id).ok_or(ContextError::Stale)?.status.clone();
            if let JobStatus::Running(lease) = status { finish_attempt(state, &lease, Usage::Unknown, now_ms, false)?; }
            let job = state.jobs.get_mut(id).ok_or(ContextError::Stale)?;
            if job.status == JobStatus::Complete { return Err(ContextError::Conflict); }
            job.status = JobStatus::Cancelled;
            Ok(())
        })
    }

    pub fn settle_unknown(&mut self, id: &str, attempt: u64, actual: u64) -> Result<(), ContextError> {
        self.transaction(|state| {
            let key = usage_key(id, attempt);
            let reserved = *state.unknown_usage.get(&key).ok_or(ContextError::Stale)?;
            if actual > reserved { return Err(ContextError::Capacity); }
            state.unknown_usage.remove(&key);
            state.spent = state.spent.checked_add(actual).ok_or(ContextError::Capacity)?;
            Ok(())
        })
    }

    fn transaction<T>(&mut self, operation: impl FnOnce(&mut SchedulerSnapshot) -> Result<T, ContextError>) -> Result<T, ContextError> {
        let before = self.state.clone();
        match operation(&mut self.state).and_then(|result| { self.persist()?; Ok(result) }) {
            Ok(result) => Ok(result),
            Err(error) => { self.state = before; Err(error) }
        }
    }

    fn persist(&self) -> Result<(), ContextError> {
        let Some(path) = &self.path else { return Ok(()); };
        let parent = path.parent().filter(|parent| !parent.as_os_str().is_empty()).unwrap_or(Path::new("."));
        fs::create_dir_all(parent)?;
        let temporary = path.with_extension("maintenance.tmp");
        let mut file = File::create(&temporary)?;
        file.write_all(&serde_json::to_vec(&self.state)?)?;
        file.sync_all()?;
        fs::rename(temporary, path)?;
        File::open(parent)?.sync_all()?;
        Ok(())
    }
}

fn usage_key(id: &str, attempt: u64) -> String { format!("{id}:{attempt}") }

fn committed_budget(state: &SchedulerSnapshot) -> Result<u64, ContextError> {
    state.jobs.values().filter(|job| matches!(job.status, JobStatus::Running(_))).map(|job| job.request.reservation)
        .chain(state.unknown_usage.values().copied()).try_fold(state.spent, |sum, cost| sum.checked_add(cost).ok_or(ContextError::Capacity))
}

fn validate_lease(state: &SchedulerSnapshot, lease: &JobLease, now_ms: u64) -> Result<(), ContextError> {
    let job = state.jobs.get(&lease.job_id).ok_or(ContextError::Stale)?;
    if job.status != JobStatus::Running(lease.clone()) || lease.expires_ms <= now_ms { return Err(ContextError::Stale); }
    Ok(())
}

fn finish_attempt(state: &mut SchedulerSnapshot, lease: &JobLease, usage: Usage, now_ms: u64, complete: bool) -> Result<(), ContextError> {
    let job = state.jobs.get_mut(&lease.job_id).ok_or(ContextError::Stale)?;
    if job.status != JobStatus::Running(lease.clone()) { return Err(ContextError::Stale); }
    match usage {
        Usage::Known(actual) => state.spent = state.spent.checked_add(actual).ok_or(ContextError::Capacity)?,
        Usage::Unknown => { state.unknown_usage.insert(usage_key(&job.id, lease.attempt), job.request.reservation); }
    }
    job.status = if complete { JobStatus::Complete } else if job.attempts >= state.config.max_attempts { JobStatus::Exhausted } else { JobStatus::Pending };
    let multiplier = 1_u64.checked_shl(job.attempts.saturating_sub(1).min(63) as u32).unwrap_or(u64::MAX);
    job.available_ms = now_ms.saturating_add(state.config.backoff_ms.saturating_mul(multiplier));
    Ok(())
}

fn expire_attempts(state: &mut SchedulerSnapshot, now_ms: u64) -> Result<usize, ContextError> {
    let expired: Vec<_> = state.jobs.values().filter_map(|job| match &job.status {
        JobStatus::Running(lease) if lease.expires_ms <= now_ms => Some(lease.clone()), _ => None,
    }).collect();
    for lease in &expired { finish_attempt(state, lease, Usage::Unknown, now_ms, false)?; }
    Ok(expired.len())
}

fn update_watermark(state: &mut SchedulerSnapshot, kind: JobKind) {
    let mut groups: BTreeMap<Cursor, bool> = BTreeMap::new();
    for job in state.jobs.values().filter(|job|job.request.kind == kind) {
        groups.entry(job.request.cursor).and_modify(|complete| *complete &= job.status == JobStatus::Complete).or_insert(job.status == JobStatus::Complete);
    }
    let gaps = groups.iter().filter_map(|(cursor,complete)| (!complete).then_some(*cursor)).collect();
    state.watermark_gaps.insert(kind,gaps);
    state.watermarks.remove(&kind);
    for (cursor,complete) in groups {
        if !complete { break; }
        state.watermarks.insert(kind,cursor);
    }
}
