use crate::types::{digest_bytes, validate_id, ContextError, Scope, CONTRACT_VERSION};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs::{self, OpenOptions}, io::Write, path::Path};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CacheFence {
    pub scope: Scope,
    pub session_id: String,
    pub model_id: String,
    pub policy_revision: String,
    pub source_revision: String,
}
impl CacheFence {
    pub fn validate(&self) -> Result<(), ContextError> {
        self.scope.validate()?;
        for value in [&self.session_id, &self.model_id, &self.policy_revision, &self.source_revision] { validate_id(value)?; }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CacheSource {
    pub id: String,
    pub digest: String,
    pub ordinal: u64,
    pub required: bool,
}
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CacheRegion {
    pub bytes: Vec<u8>,
    pub sources: Vec<CacheSource>,
}
impl CacheRegion {
    pub fn source_identity(&self) -> Result<String, ContextError> { Ok(digest_bytes(&serde_json::to_vec(&self.sources)?)) }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionKind { Baseline, Delta, LiveTail }
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PendingChange {
    pub id: String,
    pub region: RegionKind,
    pub source_identity: String,
    pub replacement: CacheRegion,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BoundaryChange {
    Defer,
    SoftFold,
    HardFold,
    Replace { baseline: CacheRegion, delta: CacheRegion, live_tail: CacheRegion },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SafetyInvalidation {
    AccessRevoked,
    RequiredEvidenceCorrected { source_id: String },
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CacheCheckpoint {
    pub version: u32,
    pub fence: CacheFence,
    pub generation: u64,
    pub baseline: CacheRegion,
    pub delta: CacheRegion,
    pub live_tail: CacheRegion,
    pub pending: Vec<PendingChange>,
    pub invalidation: Option<SafetyInvalidation>,
    pub digest: String,
}
#[derive(Clone, Debug)]
pub struct ContextCache { state: CacheCheckpoint }
impl ContextCache {
    pub fn new(fence: CacheFence, baseline: CacheRegion, delta: CacheRegion, live_tail: CacheRegion) -> Result<Self, ContextError> {
        let cache = Self { state: CacheCheckpoint { version: CONTRACT_VERSION, fence, generation: 1, baseline, delta, live_tail, pending: Vec::new(), invalidation: None, digest: String::new() } };
        cache.validate()?;
        Ok(cache)
    }
    pub fn generation(&self) -> u64 { self.state.generation }
    pub fn fence(&self) -> &CacheFence { &self.state.fence }
    pub fn pending(&self) -> &[PendingChange] { &self.state.pending }
    fn check_fence(&self, fence: &CacheFence) -> Result<(), ContextError> {
        fence.validate()?;
        if fence.scope != self.state.fence.scope || fence.session_id != self.state.fence.session_id { return Err(ContextError::ScopeMismatch); }
        if fence != &self.state.fence { return Err(ContextError::Stale); }
        Ok(())
    }
    pub fn replay(&self, fence: &CacheFence) -> Result<Vec<u8>, ContextError> {
        self.check_fence(fence)?;
        if self.state.invalidation.is_some() { return Err(ContextError::Stale); }
        let mut bytes = self.state.baseline.bytes.clone();
        bytes.extend_from_slice(&self.state.delta.bytes);
        bytes.extend_from_slice(&self.state.live_tail.bytes);
        Ok(bytes)
    }
    fn cas(&self, generation: u64) -> Result<(), ContextError> {
        if generation != self.generation() { return Err(ContextError::Conflict); }
        Ok(())
    }
    fn region(&self, kind: RegionKind) -> &CacheRegion { match kind { RegionKind::Baseline => &self.state.baseline, RegionKind::Delta => &self.state.delta, RegionKind::LiveTail => &self.state.live_tail } }
    fn region_mut(&mut self, kind: RegionKind) -> &mut CacheRegion { match kind { RegionKind::Baseline => &mut self.state.baseline, RegionKind::Delta => &mut self.state.delta, RegionKind::LiveTail => &mut self.state.live_tail } }
    fn validate_change(&self, change: &PendingChange) -> Result<(), ContextError> {
        validate_id(&change.id)?;
        let region = self.region(change.region);
        if change.source_identity != region.source_identity()? || change.replacement.sources != region.sources { return Err(ContextError::Stale); }
        Ok(())
    }
    pub fn queue_change(&mut self, expected_generation: u64, change: PendingChange) -> Result<(), ContextError> {
        self.cas(expected_generation)?;
        if self.state.invalidation.is_some() { return Err(ContextError::Stale); }
        self.validate_change(&change)?;
        if let Some(existing) = self.state.pending.iter().find(|item| item.id == change.id) {
            return if existing == &change { Ok(()) } else { Err(ContextError::Conflict) };
        }
        if self.state.pending.iter().any(|item| item.region == change.region) { return Err(ContextError::Conflict); }
        self.state.pending.push(change);
        Ok(())
    }
    pub fn reconcile(&mut self, expected_generation: u64, fence: &CacheFence, change: BoundaryChange) -> Result<u64, ContextError> {
        self.cas(expected_generation)?;
        fence.validate()?;
        if fence.scope != self.state.fence.scope || fence.session_id != self.state.fence.session_id { return Err(ContextError::ScopeMismatch); }
        if change == BoundaryChange::Defer { self.replay(fence)?; return Ok(self.generation()); }
        let replacing = matches!(change, BoundaryChange::Replace { .. });
        if !replacing { self.check_fence(fence)?; if self.state.invalidation.is_some() { return Err(ContextError::Stale); } }
        let mut next = self.clone();
        if !replacing {
            for pending in &self.state.pending {
                self.validate_change(pending)?;
                *next.region_mut(pending.region) = pending.replacement.clone();
            }
        }
        match change {
            BoundaryChange::SoftFold => { let tail = std::mem::take(&mut next.state.live_tail); append(&mut next.state.delta, tail); }
            BoundaryChange::HardFold => { let delta = std::mem::take(&mut next.state.delta); let tail = std::mem::take(&mut next.state.live_tail); append(&mut next.state.baseline, delta); append(&mut next.state.baseline, tail); }
            BoundaryChange::Replace { baseline, delta, live_tail } => { next.state.baseline = baseline; next.state.delta = delta; next.state.live_tail = live_tail; next.state.fence = fence.clone(); next.state.invalidation = None; }
            BoundaryChange::Defer => unreachable!(),
        }
        next.state.pending.clear();
        next.state.generation = next.state.generation.checked_add(1).ok_or(ContextError::Capacity)?;
        next.validate()?;
        *self = next;
        Ok(self.generation())
    }
    pub fn invalidate(&mut self, reason: SafetyInvalidation) -> Result<u64, ContextError> {
        if let SafetyInvalidation::RequiredEvidenceCorrected { source_id } = &reason { validate_id(source_id)?; }
        self.state.invalidation = Some(reason);
        self.state.pending.clear();
        let generation = self.generation().checked_add(1).ok_or(ContextError::Capacity)?;
        self.state.generation = generation;
        Ok(generation)
    }
    fn validate(&self) -> Result<(), ContextError> {
        self.state.fence.validate()?;
        if self.state.version != CONTRACT_VERSION || self.generation() == 0 { return Err(ContextError::Invalid("unsupported cache checkpoint".into())); }
        let mut ids = BTreeSet::new();
        let mut last = None;
        for region in [&self.state.baseline, &self.state.delta, &self.state.live_tail] {
            for source in &region.sources {
                validate_id(&source.id)?;
                if source.digest.len() != 64 || !source.digest.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()) || !ids.insert(&source.id) || last.is_some_and(|ordinal| ordinal >= source.ordinal) { return Err(ContextError::Invalid("invalid cache source boundary".into())); }
                last = Some(source.ordinal);
            }
        }
        if self.state.invalidation.is_some() && !self.state.pending.is_empty() { return Err(ContextError::Invalid("invalidated cache has pending changes".into())); }
        if let Some(SafetyInvalidation::RequiredEvidenceCorrected { source_id }) = &self.state.invalidation { validate_id(source_id)?; }
        let mut changes = BTreeSet::new();
        let mut regions = BTreeSet::new();
        for change in &self.state.pending {
            self.validate_change(change)?;
            if !changes.insert(&change.id) || !regions.insert(change.region as u8) { return Err(ContextError::Conflict); }
        }
        Ok(())
    }
    pub fn checkpoint(&self) -> Result<CacheCheckpoint, ContextError> {
        self.validate()?;
        let mut state = self.state.clone();
        state.digest.clear();
        state.digest = digest_bytes(&serde_json::to_vec(&state)?);
        Ok(state)
    }
    pub fn restore(mut checkpoint: CacheCheckpoint, expected_fence: &CacheFence) -> Result<Self, ContextError> {
        let digest = std::mem::take(&mut checkpoint.digest);
        if digest != digest_bytes(&serde_json::to_vec(&checkpoint)?) { return Err(ContextError::Invalid("cache checkpoint digest mismatch".into())); }
        let cache = Self { state: checkpoint };
        cache.validate()?;
        cache.check_fence(expected_fence)?;
        Ok(cache)
    }
    pub fn save(&self, path: &Path) -> Result<(), ContextError> {
        let bytes = serde_json::to_vec(&self.checkpoint()?)?;
        let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
        let name = path.file_name().ok_or_else(|| ContextError::Invalid("checkpoint path".into()))?.to_string_lossy();
        let temp = parent.join(format!(".{name}.{}.{}.tmp", std::process::id(), self.generation()));
        let mut file = OpenOptions::new().write(true).create_new(true).open(&temp)?;
        let result = (|| -> Result<(), ContextError> { file.write_all(&bytes)?; file.sync_all()?; fs::rename(&temp, path)?; fs::File::open(parent)?.sync_all()?; Ok(()) })();
        if result.is_err() { let _ = fs::remove_file(&temp); }
        result
    }
    pub fn load(path: &Path, expected_fence: &CacheFence) -> Result<Self, ContextError> { Self::restore(serde_json::from_slice(&fs::read(path)?)?, expected_fence) }
}
fn append(region: &mut CacheRegion, other: CacheRegion) { region.bytes.extend(other.bytes); region.sources.extend(other.sources); }
