use crate::types::{
    digest_bytes, validate_id, ContextError, Cursor, Scope, SourceMessage, SourceSpan,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ChunkLimits {
    pub max_messages: usize,
    pub max_bytes: usize,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceChunk {
    pub sources: Vec<SourceMessage>,
    pub spans: Vec<SourceSpan>,
    pub digest: String,
}
impl SourceChunk {
    pub fn validate(&self) -> Result<(), ContextError> {
        if self.sources.is_empty() {
            return Err(ContextError::Invalid("empty chunk".into()));
        }
        let mut ids = BTreeSet::new();
        let mut previous = None;
        for source in &self.sources {
            source.validate()?;
            if !ids.insert(&source.id) || previous.is_some_and(|ordinal| ordinal >= source.ordinal)
            {
                return Err(ContextError::Invalid(
                    "unordered or duplicate sources".into(),
                ));
            }
            previous = Some(source.ordinal);
        }
        if self.spans.len() != self.sources.len()
            || self.spans.iter().zip(&self.sources).any(|(span, source)| {
                span.source_id != source.id
                    || span.source_digest != source.source_digest
                    || span.byte_start != 0
                    || span.byte_end == 0
            })
        {
            return Err(ContextError::Invalid("invalid source spans".into()));
        }
        if self.digest != digest_bytes(&serde_json::to_vec(&(&self.sources, &self.spans))?) {
            return Err(ContextError::Invalid("chunk digest mismatch".into()));
        }
        Ok(())
    }
    pub fn coverage(&self) -> Result<Vec<SourceSpan>, ContextError> {
        self.validate()?;
        Ok(self.spans.clone())
    }
}
pub fn select_chunks(
    sources: &[SourceMessage],
    limits: ChunkLimits,
) -> Result<Vec<SourceChunk>, ContextError> {
    if limits.max_messages == 0 || limits.max_bytes == 0 {
        return Err(ContextError::Capacity);
    }
    let mut chunks = Vec::new();
    let mut selected = Vec::new();
    let mut bytes = 0usize;
    let mut previous = None;
    let mut ids = BTreeSet::new();
    for source in sources {
        source.validate()?;
        if !ids.insert(&source.id) || previous.is_some_and(|p| p >= source.ordinal) {
            return Err(ContextError::Invalid(
                "unordered or duplicate sources".into(),
            ));
        }
        previous = Some(source.ordinal);
        let size = serde_json::to_vec(source)?.len();
        if size > limits.max_bytes {
            return Err(ContextError::Capacity);
        }
        if !selected.is_empty()
            && (selected.len() == limits.max_messages
                || bytes.checked_add(size).ok_or(ContextError::Capacity)? > limits.max_bytes)
        {
            chunks.push(make_chunk(std::mem::take(&mut selected))?);
            bytes = 0;
        }
        selected.push(source.clone());
        bytes += size;
    }
    if !selected.is_empty() {
        chunks.push(make_chunk(selected)?);
    }
    Ok(chunks)
}
fn make_chunk(sources: Vec<SourceMessage>) -> Result<SourceChunk, ContextError> {
    let spans: Vec<SourceSpan> = sources
        .iter()
        .map(|s| {
            Ok(SourceSpan {
                source_id: s.id.clone(),
                source_digest: s.source_digest.clone(),
                byte_start: 0,
                byte_end: serde_json::to_vec(s)?.len() as u64,
            })
        })
        .collect::<Result<_, ContextError>>()?;
    Ok(SourceChunk {
        digest: digest_bytes(&serde_json::to_vec(&(&sources, &spans))?),
        sources,
        spans,
    })
}
pub fn select_chunks_with_spans(
    sources: &[SourceMessage],
    spans: &[SourceSpan],
    limits: ChunkLimits,
) -> Result<Vec<SourceChunk>, ContextError> {
    if sources.len() != spans.len() {
        return Err(ContextError::Invalid("coverage count mismatch".into()));
    }
    select_chunks(sources, limits)?;
    let mut chunks = Vec::new();
    let mut chunk_sources = Vec::new();
    let mut chunk_spans = Vec::new();
    let mut bytes = 0usize;
    for (source, span) in sources.iter().zip(spans) {
        if span.source_id != source.id
            || span.source_digest != source.source_digest
            || span.byte_start != 0
            || span.byte_end == 0
        {
            return Err(ContextError::Invalid("invalid source spans".into()));
        }
        let size = usize::try_from(span.byte_end)
            .map_err(|_| ContextError::Capacity)?
            .max(serde_json::to_vec(source)?.len());
        if size > limits.max_bytes {
            return Err(ContextError::Capacity);
        }
        if !chunk_sources.is_empty()
            && (chunk_sources.len() == limits.max_messages
                || bytes.checked_add(size).ok_or(ContextError::Capacity)? > limits.max_bytes)
        {
            chunks.push(SourceChunk {
                digest: digest_bytes(&serde_json::to_vec(&(&chunk_sources, &chunk_spans))?),
                sources: std::mem::take(&mut chunk_sources),
                spans: std::mem::take(&mut chunk_spans),
            });
            bytes = 0;
        }
        chunk_sources.push(source.clone());
        chunk_spans.push(span.clone());
        bytes += size;
    }
    if !chunk_sources.is_empty() {
        chunks.push(SourceChunk {
            digest: digest_bytes(&serde_json::to_vec(&(&chunk_sources, &chunk_spans))?),
            sources: chunk_sources,
            spans: chunk_spans,
        });
    }
    Ok(chunks)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SummaryTier {
    pub text: String,
    pub coverage: Vec<SourceSpan>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistorianResult {
    pub source_digest: String,
    pub tiers: [SummaryTier; 4],
}
impl HistorianResult {
    pub fn validate(&self, chunk: &SourceChunk) -> Result<(), ContextError> {
        chunk.validate()?;
        if self.source_digest != chunk.digest {
            return Err(ContextError::Stale);
        }
        let expected = chunk.coverage()?;
        let mut previous = usize::MAX;
        for tier in &self.tiers {
            if tier.text.trim().is_empty()
                || tier.text.len() > 1_048_576
                || tier.text.len() > previous
                || tier.coverage != expected
            {
                return Err(ContextError::Invalid(
                    "invalid summary tier or coverage".into(),
                ));
            }
            previous = tier.text.len();
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum JobState {
    Pending { ready_at_ms: u64 },
    Claimed { worker: String, expires_at_ms: u64 },
    Complete,
    Cancelled,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistorianJob {
    pub id: String,
    pub session_id: String,
    pub cursor: Cursor,
    pub policy_revision: u64,
    pub chunk: SourceChunk,
    pub attempt: u64,
    pub state: JobState,
    pub result: Option<HistorianResult>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistorianClaim {
    pub job: HistorianJob,
    pub worker: String,
    pub attempt: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Historian {
    scope: Scope,
    jobs: BTreeMap<String, HistorianJob>,
}
impl Historian {
    pub fn new(scope: Scope) -> Result<Self, ContextError> {
        scope.validate()?;
        Ok(Self {
            scope,
            jobs: BTreeMap::new(),
        })
    }
    pub fn jobs(&self) -> impl Iterator<Item = &HistorianJob> {
        self.jobs.values()
    }
    pub fn result(&self, id: &str) -> Option<&HistorianResult> {
        self.jobs.get(id).and_then(|job| job.result.as_ref())
    }
    pub fn enqueue(
        &mut self,
        session_id: &str,
        cursor: Cursor,
        policy_revision: u64,
        chunk: SourceChunk,
        now_ms: u64,
    ) -> Result<String, ContextError> {
        validate_id(session_id)?;
        cursor.validate()?;
        chunk.validate()?;
        let id = digest_bytes(&serde_json::to_vec(&(
            &self.scope,
            session_id,
            cursor,
            policy_revision,
            &chunk.digest,
        ))?);
        self.jobs.entry(id.clone()).or_insert(HistorianJob {
            id: id.clone(),
            session_id: session_id.into(),
            cursor,
            policy_revision,
            chunk,
            attempt: 0,
            state: JobState::Pending {
                ready_at_ms: now_ms,
            },
            result: None,
        });
        Ok(id)
    }
    pub fn expire(&mut self, now_ms: u64, cooldown_ms: u64) -> Result<usize, ContextError> {
        let ready = now_ms
            .checked_add(cooldown_ms)
            .ok_or(ContextError::Capacity)?;
        let mut count = 0;
        for job in self.jobs.values_mut() {
            if matches!(job.state, JobState::Claimed { expires_at_ms, .. } if expires_at_ms <= now_ms)
            {
                job.state = JobState::Pending { ready_at_ms: ready };
                count += 1;
            }
        }
        Ok(count)
    }
    pub fn claim(
        &mut self,
        worker: &str,
        now_ms: u64,
        lease_ms: u64,
    ) -> Result<Option<HistorianClaim>, ContextError> {
        validate_id(worker)?;
        if lease_ms == 0 {
            return Err(ContextError::Invalid("zero lease".into()));
        }
        let expiry = now_ms.checked_add(lease_ms).ok_or(ContextError::Capacity)?;
        let Some(job) = self.jobs.values_mut().find(
            |j| matches!(j.state, JobState::Pending { ready_at_ms } if ready_at_ms <= now_ms),
        ) else {
            return Ok(None);
        };
        job.attempt = job.attempt.checked_add(1).ok_or(ContextError::Capacity)?;
        job.state = JobState::Claimed {
            worker: worker.into(),
            expires_at_ms: expiry,
        };
        Ok(Some(HistorianClaim {
            job: job.clone(),
            worker: worker.into(),
            attempt: job.attempt,
        }))
    }
    pub fn claim_job(
        &mut self,
        id: &str,
        worker: &str,
        now_ms: u64,
        lease_ms: u64,
    ) -> Result<Option<HistorianClaim>, ContextError> {
        validate_id(worker)?;
        if lease_ms == 0 { return Err(ContextError::Invalid("zero lease".into())); }
        let expiry = now_ms.checked_add(lease_ms).ok_or(ContextError::Capacity)?;
        let job = self.jobs.get_mut(id).ok_or(ContextError::Stale)?;
        if !matches!(job.state, JobState::Pending { ready_at_ms } if ready_at_ms <= now_ms) { return Ok(None); }
        job.attempt = job.attempt.checked_add(1).ok_or(ContextError::Capacity)?;
        job.state = JobState::Claimed { worker: worker.into(), expires_at_ms: expiry };
        Ok(Some(HistorianClaim { job: job.clone(), worker: worker.into(), attempt: job.attempt }))
    }

    fn active(
        &mut self,
        claim: &HistorianClaim,
        now_ms: u64,
    ) -> Result<&mut HistorianJob, ContextError> {
        let job = self
            .jobs
            .get_mut(&claim.job.id)
            .ok_or(ContextError::Stale)?;
        if job.attempt != claim.attempt
            || !matches!(&job.state, JobState::Claimed { worker, expires_at_ms } if worker == &claim.worker && *expires_at_ms > now_ms)
        {
            return Err(ContextError::Stale);
        }
        Ok(job)
    }
    pub fn heartbeat(
        &mut self,
        claim: &HistorianClaim,
        now_ms: u64,
        lease_ms: u64,
    ) -> Result<(), ContextError> {
        if lease_ms == 0 {
            return Err(ContextError::Invalid("zero lease".into()));
        }
        let expiry = now_ms.checked_add(lease_ms).ok_or(ContextError::Capacity)?;
        let job = self.active(claim, now_ms)?;
        job.state = JobState::Claimed {
            worker: claim.worker.clone(),
            expires_at_ms: expiry,
        };
        Ok(())
    }
    pub fn complete(
        &mut self,
        claim: &HistorianClaim,
        now_ms: u64,
        current: &SourceChunk,
        policy_revision: u64,
        result: HistorianResult,
    ) -> Result<(), ContextError> {
        current.validate()?;
        let job = self.active(claim, now_ms)?;
        if current != &job.chunk || policy_revision != job.policy_revision {
            return Err(ContextError::Stale);
        }
        result.validate(&job.chunk)?;
        job.result = Some(result);
        job.state = JobState::Complete;
        Ok(())
    }
    pub fn fail(
        &mut self,
        claim: &HistorianClaim,
        now_ms: u64,
        cooldown_ms: u64,
    ) -> Result<(), ContextError> {
        let ready_at_ms = now_ms
            .checked_add(cooldown_ms)
            .ok_or(ContextError::Capacity)?;
        self.active(claim, now_ms)?.state = JobState::Pending { ready_at_ms };
        Ok(())
    }
    pub fn cancel(&mut self, id: &str) -> Result<(), ContextError> {
        let job = self.jobs.get_mut(id).ok_or(ContextError::Stale)?;
        job.state = JobState::Cancelled;
        job.result = None;
        Ok(())
    }
    pub fn checkpoint(&self, path: &Path) -> Result<(), ContextError> {
        let bytes = serde_json::to_vec(self)?;
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| ContextError::Unavailable("clock before epoch".into()))?
            .as_nanos();
        let temporary =
            path.with_extension(format!("historian.{}.{}.tmp", std::process::id(), nonce));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        let outcome = (|| {
            file.write_all(&bytes)?;
            file.sync_all()?;
            fs::rename(&temporary, path)?;
            if let Some(parent) = path.parent() {
                fs::File::open(parent)?.sync_all()?;
            }
            Ok::<_, std::io::Error>(())
        })();
        if outcome.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        outcome?;
        Ok(())
    }
    pub fn restore(path: &Path, scope: &Scope) -> Result<Self, ContextError> {
        scope.validate()?;
        let state: Self = serde_json::from_slice(&fs::read(path)?)?;
        if &state.scope != scope {
            return Err(ContextError::ScopeMismatch);
        }
        for (id, job) in &state.jobs {
            job.chunk.validate()?;
            validate_id(&job.session_id)?;
            job.cursor.validate()?;
            let expected = digest_bytes(&serde_json::to_vec(&(
                &state.scope,
                &job.session_id,
                job.cursor,
                job.policy_revision,
                &job.chunk.digest,
            ))?);
            if id != &job.id || id != &expected {
                return Err(ContextError::Invalid("job identity mismatch".into()));
            }
            if matches!(job.state, JobState::Claimed { .. }) && job.attempt == 0 {
                return Err(ContextError::Invalid("invalid attempt".into()));
            }
            if let Some(result) = &job.result {
                result.validate(&job.chunk)?;
            }
            if matches!(job.state, JobState::Complete) != job.result.is_some() {
                return Err(ContextError::Invalid("invalid completed state".into()));
            }
        }
        Ok(state)
    }
}
