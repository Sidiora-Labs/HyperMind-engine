use crate::development_mapping::{
    self, AuthorizedRepository, RecordVerification, VerificationBatch,
};
use hm_context::{ContextError, Scope, development::*, validate_id};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CycleRecord {
    pub id: String,
    pub revision: String,
    pub fingerprint: String,
    pub mapped: bool,
    pub result: Option<RecordVerification>,
    pub receipt: Option<WorkerReceipt>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerificationCycle {
    pub version: u32,
    pub id: String,
    pub scope: Scope,
    pub principal: Scope,
    pub worker_id: String,
    pub capability_id: String,
    pub generation: u64,
    pub cancelled: bool,
    pub contiguous: usize,
    pub records: Vec<CycleRecord>,
}
impl VerificationCycle {
    pub fn validate(&self) -> Result<(), ContextError> {
        validate_id(&self.id)?;
        self.scope.validate()?;
        self.principal.validate()?;
        if self.version != 1 || self.generation == 0 || self.records.len() > 256 {
            return Err(ContextError::Invalid("verification cycle state".into()));
        }
        let mut last: Option<&str> = None;
        for r in &self.records {
            validate_id(&r.id)?;
            if last.is_some_and(|p| p >= r.id.as_str())
                || r.result.is_some() != r.receipt.is_some()
                || r.result.as_ref().is_some_and(|v| {
                    v.record_id != r.id || v.fingerprint != r.fingerprint || v.findings.is_empty()
                })
                || r.receipt
                    .as_ref()
                    .is_some_and(|v| v.scope != self.scope || v.version != 1)
            {
                return Err(ContextError::Invalid("verification cycle record".into()));
            }
            last = Some(&r.id);
        }
        if self.contiguous
            != self
                .records
                .iter()
                .take_while(|r| r.receipt.is_some())
                .count()
        {
            return Err(ContextError::Invalid("verification coverage cursor".into()));
        }
        Ok(())
    }
    pub fn next_batch(&self, limit: usize) -> Result<BTreeSet<String>, ContextError> {
        self.validate()?;
        if self.cancelled {
            return Err(ContextError::Stale);
        }
        if limit == 0 || limit > 64 {
            return Err(ContextError::Capacity);
        }
        Ok(self
            .records
            .iter()
            .filter(|r| r.receipt.is_none() && r.mapped)
            .take(limit)
            .map(|r| r.id.clone())
            .collect())
    }
    pub fn refresh(
        &mut self,
        snapshot: &EvidenceSnapshot,
        repository: &AuthorizedRepository,
    ) -> Result<(), ContextError> {
        let next = open_cycle(snapshot, repository, &self.id)?;
        if next.scope != self.scope
            || next.principal != self.principal
            || next.worker_id != self.worker_id
            || next.capability_id != self.capability_id
            || next
                .records
                .iter()
                .map(|r| &r.id)
                .ne(self.records.iter().map(|r| &r.id))
        {
            return Err(ContextError::ScopeMismatch);
        }
        for (old, new) in self.records.iter_mut().zip(next.records) {
            if old.revision != new.revision || old.fingerprint != new.fingerprint {
                *old = new;
            }
        }
        self.contiguous = self
            .records
            .iter()
            .take_while(|r| r.receipt.is_some())
            .count();
        self.validate()
    }
    pub fn complete_batch(
        &mut self,
        batch: &VerificationBatch,
        receipt: &WorkerReceipt,
    ) -> Result<(), ContextError> {
        if receipt.plan_id != batch.plan.id
            || receipt.plan_digest != batch.plan.digest()?
            || receipt.snapshot_digest != batch.plan.evidence.digest
            || receipt.scope != self.scope
            || !batch.skipped.is_empty()
        {
            return Err(ContextError::Stale);
        }
        for result in &batch.records {
            let r = self
                .records
                .iter_mut()
                .find(|r| r.id == result.record_id)
                .ok_or(ContextError::ScopeMismatch)?;
            if r.fingerprint != result.fingerprint || result.findings.is_empty() {
                return Err(ContextError::Stale);
            }
            r.result = Some(result.clone());
            r.receipt = Some(receipt.clone());
        }
        self.contiguous = self
            .records
            .iter()
            .take_while(|r| r.receipt.is_some())
            .count();
        self.validate()
    }
}
pub fn open_cycle(
    snapshot: &EvidenceSnapshot,
    repository: &AuthorizedRepository,
    id: &str,
) -> Result<VerificationCycle, ContextError> {
    validate_id(id)?;
    if snapshot.records.len() > 256 {
        return Err(ContextError::Capacity);
    }
    let mut records = Vec::new();
    for record in &snapshot.records {
        let mut selected = snapshot.clone();
        selected.records = vec![record.clone()];
        selected.digest = selected.computed_digest()?;
        let mappings = development_mapping::map_evidence(&selected, repository)?;
        let mapping = mappings.records.first().ok_or(ContextError::Capacity)?;
        records.push(CycleRecord {
            id: record.id.clone(),
            revision: record.revision_digest.clone(),
            fingerprint: mapping.fingerprint.clone(),
            mapped: !mapping.spans.is_empty(),
            result: None,
            receipt: None,
        });
    }
    records.sort_by(|a, b| a.id.cmp(&b.id));
    let cycle = VerificationCycle {
        version: 1,
        id: id.into(),
        scope: snapshot.scope.clone(),
        principal: snapshot.principal.clone(),
        worker_id: snapshot.worker_id.clone(),
        capability_id: snapshot.capability_id.clone(),
        generation: 1,
        cancelled: false,
        contiguous: 0,
        records,
    };
    cycle.validate()?;
    Ok(cycle)
}
