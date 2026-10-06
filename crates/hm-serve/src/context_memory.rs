use crate::actor::{ActorEngine, IncomingEvent};
use hm_context::{Authority, ContextError, Scope, digest_bytes, validate_id};
use hm_core::{ConversationId, LSN};
use hm_ledger::frame::EventKind;
use hm_schema::{
    event::{CURRENT_SCHEMA_VERSION, encode_event_envelope},
    events::{EventEnvelope, EventPayload, ProviderFrame, Retention, Sensitivity},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub const MEMORY_PROVIDER: &str = "hypermind/context-memory/v1";
#[derive(Debug)]
pub enum MemoryError {
    Context(ContextError),
    Ledger(hm_core::Error),
}
impl std::fmt::Display for MemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Context(e) => e.fmt(f),
            Self::Ledger(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for MemoryError {}
impl From<ContextError> for MemoryError {
    fn from(e: ContextError) -> Self {
        Self::Context(e)
    }
}
impl From<hm_core::Error> for MemoryError {
    fn from(e: hm_core::Error) -> Self {
        Self::Ledger(e)
    }
}
impl From<serde_json::Error> for MemoryError {
    fn from(e: serde_json::Error) -> Self {
        Self::Context(e.into())
    }
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordKind {
    Fact,
    Episode,
    #[default]
    Note,
    ConditionalNote,
    Anchor,
    Summary,
    Primer,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordStatus {
    #[default]
    Active,
    Archived,
    Stale,
    Tombstoned,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub source_id: String,
    pub source_digest: String,
    pub span_start: u64,
    pub span_end: u64,
    pub quoted_digest: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lineage {
    pub child_record_id: String,
    pub child_revision: u64,
    pub parent_record_id: String,
    pub parent_revision_digest: String,
    pub relation: String,
    #[serde(with = "nanos")]
    pub created_at_ns: i64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryRecord {
    pub id: String,
    pub kind: RecordKind,
    pub category: String,
    pub status: RecordStatus,
    pub revision: u64,
    pub revision_digest: String,
    pub content: String,
    pub authority: Authority,
    pub confidence: u32,
    pub importance: u32,
    #[serde(with = "optional_nanos")]
    pub occurred_at_ns: Option<i64>,
    #[serde(with = "nanos")]
    pub recorded_at_ns: i64,
    #[serde(with = "optional_nanos")]
    pub expires_at_ns: Option<i64>,
    pub pinned: bool,
    pub provenance: Vec<Provenance>,
    pub lineage: Vec<Lineage>,
    pub contradictions: Vec<String>,
    pub last_lsn: u64,
    #[serde(default)]
    pub predicate: Option<hm_context::notes::Predicate>,
    #[serde(default)]
    pub smart_condition: Option<SmartCondition>,
    #[serde(default)]
    #[serde(with = "optional_nanos")]
    pub retention_until_ns: Option<i64>,
    #[serde(default)]
    pub metadata: Value,
}
impl MemoryRecord {
    pub fn new(
        id: impl Into<String>,
        kind: RecordKind,
        content: impl Into<String>,
        recorded_at_ns: i64,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            category: "general".into(),
            status: RecordStatus::Active,
            revision: 1,
            revision_digest: String::new(),
            content: content.into(),
            authority: Authority::UserAsserted,
            confidence: 1_000_000,
            importance: 500_000,
            occurred_at_ns: None,
            recorded_at_ns,
            expires_at_ns: None,
            pinned: false,
            provenance: vec![],
            lineage: vec![],
            contradictions: vec![],
            last_lsn: 0,
            predicate: None,
            smart_condition: None,
            retention_until_ns: None,
            metadata: Value::Null,
        }
    }
    pub fn computed_revision_digest(&self) -> Result<String, ContextError> {
        let mut r = self.clone();
        r.revision_digest.clear();
        r.last_lsn = 0;
        Ok(digest_bytes(&serde_json::to_vec(&serde_json::to_value(
            &r,
        )?)?))
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemorySource {
    pub id: String,
    pub digest: String,
    pub content: Vec<u8>,
    pub locator: String,
    #[serde(with = "optional_nanos")]
    pub occurred_at_ns: Option<i64>,
    #[serde(with = "nanos")]
    pub recorded_at_ns: i64,
    pub tombstoned: bool,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationState {
    Supported,
    Contradicted,
    Unresolved,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Verification {
    pub id: String,
    pub record_id: String,
    pub revision_digest: String,
    pub state: VerificationState,
    pub evidence_source_id: Option<String>,
    pub confidence: u32,
    #[serde(with = "nanos")]
    pub created_at_ns: i64,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryGrant {
    #[serde(default)]
    pub principal_digest: Option<String>,
    pub id: String,
    pub principal: Scope,
    pub record_ids: BTreeSet<String>,
    pub categories: BTreeSet<String>,
    pub read: bool,
    #[serde(with = "optional_nanos")]
    pub expires_at_ns: Option<i64>,
    pub revoked: bool,
    pub revision: u64,
    #[serde(default)]
    pub record_revisions: BTreeMap<String, String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MemoryCommand {
    Create {
        record: MemoryRecord,
    },
    Revise {
        record: MemoryRecord,
        expected_revision: u64,
    },
    Tombstone {
        id: String,
        expected_revision: u64,
    },
    SetStatus {
        id: String,
        status: RecordStatus,
        expected_revision: u64,
    },
    Source {
        source: MemorySource,
    },
    TombstoneSource {
        id: String,
    },
    Verify {
        verification: Verification,
    },
    SetGrant {
        grant: MemoryGrant,
    },
    RevokeGrant {
        id: String,
    },
    Lineage {
        lineage: Lineage,
    },
    ImportRows {
        import_id: String,
        bundle_digest: String,
        start: usize,
        total: usize,
        entries: Vec<crate::hypermid_import::ImportEntry>,
    },
    RestoreExport {
        export: MemoryExport,
    },
    CommitImport {
        import_id: String,
        bundle_digest: String,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryRequest {
    pub version: u32,
    pub scope: Scope,
    pub request_id: String,
    pub command: MemoryCommand,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryReceipt {
    pub version: u32,
    pub scope: Scope,
    pub cursor: u64,
    pub last_lsn: u64,
    pub replayed: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MemoryEvent {
    version: u32,
    scope: Scope,
    principal: Scope,
    request_id: String,
    digest: String,
    cursor: u64,
    command: MemoryCommand,
}
#[derive(Clone, Debug)]
struct PendingImport {
    digest: String,
    total: usize,
    entries: Vec<crate::hypermid_import::ImportEntry>,
}
#[derive(Clone, Debug)]
pub struct MemoryProjection {
    pub scope: Scope,
    pub cursor: u64,
    pub records: BTreeMap<String, MemoryRecord>,
    pub revisions: BTreeMap<(String, u64), MemoryRecord>,
    pub sources: BTreeMap<String, MemorySource>,
    pub verifications: BTreeMap<String, Verification>,
    pub grants: BTreeMap<String, MemoryGrant>,
    pub origins: BTreeMap<String, crate::hypermid_import::ImportEntry>,
    pub audits: BTreeMap<String, MutationAudit>,
    receipts: BTreeMap<String, (String, MemoryReceipt)>,
    pending: BTreeMap<String, PendingImport>,
    pub completed_imports: BTreeMap<String, String>,
    events: Vec<MemoryEvent>,
}
impl MemoryProjection {
    fn new(scope: Scope) -> Self {
        Self {
            scope,
            cursor: 0,
            records: BTreeMap::new(),
            revisions: BTreeMap::new(),
            sources: BTreeMap::new(),
            verifications: BTreeMap::new(),
            grants: BTreeMap::new(),
            origins: BTreeMap::new(),
            audits: BTreeMap::new(),
            receipts: BTreeMap::new(),
            pending: BTreeMap::new(),
            completed_imports: BTreeMap::new(),
            events: vec![],
        }
    }
    fn authorize_read(&self, principal: &Scope, r: &MemoryRecord, now_ns: i64) -> bool {
        principal == &self.scope
            || self.grants.values().any(|g| {
                g.read
                    && !g.revoked
                    && (g
                        .principal_digest
                        .as_ref()
                        .is_some_and(|d| d == &principal_digest(principal))
                        || g.principal_digest.is_none() && &g.principal == principal)
                    && g.expires_at_ns.is_none_or(|t| now_ns < t)
                    && (g.record_ids.contains(&r.id) || g.categories.contains(&r.category))
                    && g.record_revisions
                        .get(&r.id)
                        .is_some_and(|d| d == &r.revision_digest)
            })
    }
    fn visible(&self, r: &MemoryRecord, now_ns: i64) -> bool {
        r.status != RecordStatus::Tombstoned
            && r.expires_at_ns.is_none_or(|t| now_ns < t)
            && r.provenance.iter().all(|p| {
                self.sources
                    .get(&p.source_id)
                    .is_some_and(|s| !s.tombstoned && s.digest == p.source_digest)
            })
    }
    pub fn read(
        &self,
        principal: &Scope,
        id: &str,
        now_ns: i64,
    ) -> Result<Option<&MemoryRecord>, MemoryError> {
        principal.validate()?;
        let Some(r) = self.records.get(id) else {
            return Ok(None);
        };
        if !self.authorize_read(principal, r, now_ns) {
            return Err(ContextError::ScopeMismatch.into());
        }
        Ok(self.visible(r, now_ns).then_some(r))
    }
    pub fn visible_records(
        &self,
        principal: &Scope,
        now_ns: i64,
    ) -> Result<Vec<&MemoryRecord>, MemoryError> {
        principal.validate()?;
        Ok(self
            .records
            .values()
            .filter(|r| {
                r.status == RecordStatus::Active
                    && r.kind != RecordKind::ConditionalNote
                    && self.visible(r, now_ns)
                    && self.authorize_read(principal, r, now_ns)
            })
            .collect())
    }
    pub fn read_with_facts(
        &self,
        principal: &Scope,
        id: &str,
        now_ns: i64,
        facts: &BTreeMap<String, String>,
    ) -> Result<Option<&MemoryRecord>, MemoryError> {
        let Some(r) = self.read(principal, id, now_ns)? else {
            return Ok(None);
        };
        if let Some(p) = &r.predicate {
            if !p.evaluate(facts)? {
                return Ok(None);
            }
        }
        if let Some(p) = &r.smart_condition {
            if !p.evaluate(facts)? {
                return Ok(None);
            }
        }
        Ok(Some(r))
    }
    pub fn validate_import(
        &self,
        entries: &[crate::hypermid_import::ImportEntry],
    ) -> Result<(), MemoryError> {
        let mut candidate = self.clone();
        candidate.apply_import(entries, 0)
    }
    pub fn import_progress(&self, id: &str, digest: &str) -> Result<usize, MemoryError> {
        if let Some(done) = self.completed_imports.get(id) {
            if done != digest {
                return Err(ContextError::Conflict.into());
            }
            return Ok(usize::MAX);
        }
        match self.pending.get(id) {
            Some(p) if p.digest == digest => Ok(p.entries.len()),
            Some(_) => Err(ContextError::Conflict.into()),
            None => Ok(0),
        }
    }
    fn validate_record(&self, r: &MemoryRecord) -> Result<(), MemoryError> {
        validate_id(&r.id)?;
        crate::hypermid_import::reject_excluded(&r.metadata)?;
        validate_id(&r.category)?;
        if r.content.is_empty()
            || r.content.len() > 262144
            || r.confidence > 1_000_000
            || r.importance > 1_000_000
            || r.provenance.len() > 128
            || r.contradictions.len() > 128
        {
            return Err(ContextError::Capacity.into());
        }
        if let Some(p) = &r.predicate {
            p.evaluate(&BTreeMap::new())?;
        }
        if let Some(p) = &r.smart_condition {
            p.validate()?;
        }
        for p in &r.provenance {
            let source = self
                .sources
                .get(&p.source_id)
                .ok_or_else(|| ContextError::Unavailable("source".into()))?;
            let start = usize::try_from(p.span_start).map_err(|_| ContextError::Capacity)?;
            let end = usize::try_from(p.span_end).map_err(|_| ContextError::Capacity)?;
            if source.tombstoned
                || source.digest != p.source_digest
                || start > end
                || end > source.content.len()
                || digest_bytes(&source.content[start..end]) != p.quoted_digest
            {
                return Err(ContextError::Conflict.into());
            }
        }
        let mut candidate = self.clone();
        let mut record = r.clone();
        record.lineage.clear();
        candidate.records.insert(r.id.clone(), record);
        for lineage in &r.lineage {
            if lineage.child_record_id != r.id || lineage.child_revision != r.revision {
                return Err(ContextError::Conflict.into());
            }
            candidate.lineage(lineage)?;
        }
        for id in &r.contradictions {
            if id == &r.id || !self.records.contains_key(id) {
                return Err(ContextError::Conflict.into());
            }
        }
        Ok(())
    }
    fn save_record(&mut self, mut r: MemoryRecord, lsn: u64) {
        r.last_lsn = lsn;
        self.revisions.insert((r.id.clone(), r.revision), r.clone());
        self.records.insert(r.id.clone(), r);
    }
    fn lineage(&mut self, l: &Lineage) -> Result<(), MemoryError> {
        validate_id(&l.relation)?;
        if ![
            "derived_from",
            "cites",
            "supersedes",
            "contradicts",
            "merged_from",
            "split_from",
            "imported_from",
            "verifies",
        ]
        .contains(&l.relation.as_str())
        {
            return Err(ContextError::Invalid("invalid lineage relation".into()).into());
        }
        if l.child_record_id == l.parent_record_id {
            return Err(ContextError::Conflict.into());
        }
        let child = self
            .records
            .get(&l.child_record_id)
            .ok_or_else(|| ContextError::Unavailable("child record".into()))?;
        if child.revision != l.child_revision
            || !self.revisions.values().any(|r| {
                r.id == l.parent_record_id && r.revision_digest == l.parent_revision_digest
            })
        {
            return Err(ContextError::Stale.into());
        }
        let mut stack = vec![l.parent_record_id.clone()];
        let mut seen = BTreeSet::new();
        while let Some(id) = stack.pop() {
            if acyclic_relation(&l.relation) && id == l.child_record_id {
                return Err(ContextError::Conflict.into());
            }
            if seen.insert(id.clone()) {
                if let Some(r) = self.records.get(&id) {
                    stack.extend(
                        r.lineage
                            .iter()
                            .filter(|x| acyclic_relation(&x.relation))
                            .map(|x| x.parent_record_id.clone()),
                    );
                }
            }
        }
        if l.relation == "contradicts" {
            self.records
                .get_mut(&l.child_record_id)
                .unwrap()
                .contradictions
                .push(l.parent_record_id.clone());
        }
        self.records
            .get_mut(&l.child_record_id)
            .unwrap()
            .lineage
            .push(l.clone());
        Ok(())
    }
    fn apply(&mut self, c: &MemoryCommand, lsn: u64) -> Result<(), MemoryError> {
        match c {
            MemoryCommand::RestoreExport { export } => {
                let mut check = export.clone();
                let digest = check.digest.clone();
                check.digest.clear();
                if export.version != 1
                    || export.scope != self.scope
                    || digest_bytes(&serde_json::to_vec(&check)?) != digest
                    || export.cursor != export.events.len() as u64
                {
                    return Err(ContextError::Conflict.into());
                }
                let mut restored = MemoryProjection::new(self.scope.clone());
                for value in &export.events {
                    let event: MemoryEvent = serde_json::from_value(value.clone())?;
                    if event.version != 1
                        || event.scope != self.scope
                        || event.principal != self.scope
                        || event.cursor != restored.cursor + 1
                    {
                        return Err(ContextError::Conflict.into());
                    }
                    let request = MemoryRequest {
                        version: 1,
                        scope: event.scope.clone(),
                        request_id: event.request_id.clone(),
                        command: event.command.clone(),
                    };
                    if event.digest != digest_bytes(&serde_json::to_vec(&request)?) {
                        return Err(ContextError::Conflict.into());
                    }
                    restored.apply(&event.command, lsn)?;
                    restored.cursor = event.cursor;
                }
                if restored
                    .records
                    .keys()
                    .any(|id| self.records.contains_key(id))
                    || restored
                        .sources
                        .keys()
                        .any(|id| self.sources.contains_key(id))
                {
                    return Err(ContextError::Conflict.into());
                }
                self.records.extend(restored.records);
                self.revisions.extend(restored.revisions);
                self.sources.extend(restored.sources);
                self.verifications.extend(restored.verifications);
                self.grants.extend(restored.grants);
                self.origins.extend(restored.origins);
                self.audits.extend(restored.audits);
                self.completed_imports.extend(restored.completed_imports);
                self.pending.extend(restored.pending);
            }
            MemoryCommand::Create { record } => {
                if self.records.contains_key(&record.id)
                    || record.revision != 1
                    || record.status != RecordStatus::Active
                {
                    return Err(ContextError::Conflict.into());
                }
                self.validate_record(record)?;
                if record.revision_digest != record.computed_revision_digest()? {
                    return Err(ContextError::Conflict.into());
                }
                self.save_record(record.clone(), lsn);
            }
            MemoryCommand::Revise {
                record,
                expected_revision,
            } => {
                let old = self
                    .records
                    .get(&record.id)
                    .ok_or_else(|| ContextError::Unavailable("record".into()))?;
                if old.revision != *expected_revision
                    || record.revision
                        != expected_revision
                            .checked_add(1)
                            .ok_or(ContextError::Capacity)?
                {
                    return Err(ContextError::Stale.into());
                }
                if old.kind == RecordKind::Anchor
                    || old.status == RecordStatus::Tombstoned
                    || old.kind != record.kind
                    || record.status == RecordStatus::Tombstoned
                {
                    return Err(ContextError::Conflict.into());
                }
                self.validate_record(record)?;
                if record.revision_digest != record.computed_revision_digest()? {
                    return Err(ContextError::Conflict.into());
                }
                self.save_record(record.clone(), lsn);
            }
            MemoryCommand::Tombstone {
                id,
                expected_revision,
            } => self.change_status(id, RecordStatus::Tombstoned, *expected_revision, lsn)?,
            MemoryCommand::SetStatus {
                id,
                status,
                expected_revision,
            } => self.change_status(id, *status, *expected_revision, lsn)?,
            MemoryCommand::Source { source } => {
                validate_id(&source.id)?;
                if source.content.len() > 512 * 1024
                    || source.tombstoned
                    || source.digest != digest_bytes(&source.content)
                {
                    return Err(ContextError::Conflict.into());
                }
                if let Some(old) = self.sources.get(&source.id) {
                    if old != source {
                        return Err(ContextError::Conflict.into());
                    }
                } else {
                    self.sources.insert(source.id.clone(), source.clone());
                }
            }
            MemoryCommand::TombstoneSource { id } => {
                self.sources
                    .get_mut(id)
                    .ok_or_else(|| ContextError::Unavailable("source".into()))?
                    .tombstoned = true;
            }
            MemoryCommand::Verify { verification: v } => {
                validate_id(&v.id)?;
                let r = self
                    .records
                    .get(&v.record_id)
                    .ok_or_else(|| ContextError::Unavailable("record".into()))?;
                if r.status == RecordStatus::Tombstoned
                    || r.revision_digest != v.revision_digest
                    || v.confidence > 1_000_000
                    || v.evidence_source_id
                        .as_ref()
                        .is_some_and(|id| self.sources.get(id).is_none_or(|s| s.tombstoned))
                {
                    return Err(ContextError::Stale.into());
                }
                if self.verifications.contains_key(&v.id) {
                    return Err(ContextError::Conflict.into());
                }
                self.verifications.insert(v.id.clone(), v.clone());
            }
            MemoryCommand::SetGrant { grant: g } => {
                validate_id(&g.id)?;
                g.principal.validate()?;
                let expected = self.grants.get(&g.id).map_or(1, |old| old.revision + 1);
                if g.revision != expected || g.record_ids.is_empty() && g.categories.is_empty() {
                    return Err(ContextError::Stale.into());
                }
                for (id, digest) in &g.record_revisions {
                    if self.records.get(id).is_none_or(|r| {
                        &r.revision_digest != digest || r.status == RecordStatus::Tombstoned
                    }) {
                        return Err(ContextError::Stale.into());
                    }
                }
                self.grants.insert(g.id.clone(), g.clone());
            }
            MemoryCommand::RevokeGrant { id } => {
                let g = self
                    .grants
                    .get_mut(id)
                    .ok_or_else(|| ContextError::Unavailable("grant".into()))?;
                g.revoked = true;
                g.revision += 1;
            }
            MemoryCommand::Lineage { lineage } => self.lineage(lineage)?,
            MemoryCommand::ImportRows {
                import_id,
                bundle_digest,
                start,
                total,
                entries,
            } => {
                validate_id(import_id)?;
                if let Some(done) = self.completed_imports.get(import_id) {
                    return if done == bundle_digest {
                        Ok(())
                    } else {
                        Err(ContextError::Conflict.into())
                    };
                }
                let p = self
                    .pending
                    .entry(import_id.clone())
                    .or_insert(PendingImport {
                        digest: bundle_digest.clone(),
                        total: *total,
                        entries: vec![],
                    });
                if p.digest != *bundle_digest
                    || p.total != *total
                    || *start != p.entries.len()
                    || start.saturating_add(entries.len()) > *total
                {
                    return Err(ContextError::Conflict.into());
                }
                for entry in entries {
                    if entry.digest != digest_bytes(&serde_json::to_vec(&entry.payload)?)
                        || p.entries.iter().any(|e| e.source_id == entry.source_id)
                    {
                        return Err(ContextError::Conflict.into());
                    }
                    p.entries.push(entry.clone());
                }
            }
            MemoryCommand::CommitImport {
                import_id,
                bundle_digest,
            } => {
                if let Some(done) = self.completed_imports.get(import_id) {
                    return if done == bundle_digest {
                        Ok(())
                    } else {
                        Err(ContextError::Conflict.into())
                    };
                }
                let p = self
                    .pending
                    .get(import_id)
                    .ok_or_else(|| ContextError::Unavailable("staged import".into()))?
                    .clone();
                if p.digest != *bundle_digest || p.entries.len() != p.total {
                    return Err(ContextError::Stale.into());
                }
                self.apply_import(&p.entries, lsn)?;
                self.completed_imports
                    .insert(import_id.clone(), bundle_digest.clone());
                self.pending.remove(import_id);
            }
        }
        Ok(())
    }
    fn change_status(
        &mut self,
        id: &str,
        status: RecordStatus,
        revision: u64,
        lsn: u64,
    ) -> Result<(), MemoryError> {
        let old = self
            .records
            .get(id)
            .ok_or_else(|| ContextError::Unavailable("record".into()))?;
        if old.revision != revision {
            return Err(ContextError::Stale.into());
        }
        if old.status == RecordStatus::Tombstoned || old.pinned && status == RecordStatus::Archived
        {
            return Err(ContextError::Conflict.into());
        }
        let mut r = old.clone();
        r.status = status;
        r.revision = revision.checked_add(1).ok_or(ContextError::Capacity)?;
        r.revision_digest = r.computed_revision_digest()?;
        self.save_record(r, lsn);
        Ok(())
    }
}

pub async fn rebuild(actor: &ActorEngine, scope: &Scope) -> Result<MemoryProjection, MemoryError> {
    scope.validate()?;
    let mut state = MemoryProjection::new(scope.clone());
    let frozen_tail = actor.stats().await?.applied.last_lsn;
    let conversation = ConversationId::derive(&format!("context-memory:{}", scope.digest()?));
    let mut since = LSN::new(0);
    loop {
        let frames = actor.frames_since(since, Some(conversation), 256).await?;
        if frames.is_empty() {
            break;
        }
        for frame in &frames {
            if frame.header.lsn.get() > frozen_tail.get() {
                break;
            }
            since = frame.header.lsn;
            let verified = actor.verified_event(frame.header.lsn).await?;
            let EventPayload::ProviderFrame(p) = verified.envelope.payload else {
                continue;
            };
            if p.provider != MEMORY_PROVIDER {
                continue;
            }
            let e: MemoryEvent = serde_json::from_slice(&p.api_content)?;
            if e.version != 1
                || e.scope != *scope
                || e.principal != *scope
                || e.cursor != state.cursor + 1
                || state.receipts.contains_key(&e.request_id)
            {
                return Err(ContextError::Conflict.into());
            }
            let computed = digest_bytes(&serde_json::to_vec(&MemoryRequest {
                version: e.version,
                scope: e.scope.clone(),
                request_id: e.request_id.clone(),
                command: e.command.clone(),
            })?);
            if computed != e.digest {
                return Err(ContextError::Conflict.into());
            }
            state.apply(&e.command, frame.header.lsn.get())?;
            state.cursor = e.cursor;
            state.receipts.insert(
                e.request_id.clone(),
                (
                    e.digest.clone(),
                    MemoryReceipt {
                        version: 1,
                        scope: scope.clone(),
                        cursor: e.cursor,
                        last_lsn: frame.header.lsn.get(),
                        replayed: false,
                    },
                ),
            );
            state.events.push(e);
        }
        if frames.len() < 256
            || frames
                .last()
                .is_some_and(|frame| frame.header.lsn.get() >= frozen_tail.get())
        {
            break;
        }
    }
    Ok(state)
}

pub async fn execute(
    actor: &ActorEngine,
    trusted_scope: &Scope,
    principal: &Scope,
    request: MemoryRequest,
) -> Result<MemoryReceipt, MemoryError> {
    if matches!(
        &request.command,
        MemoryCommand::ImportRows { .. } | MemoryCommand::CommitImport { .. }
    ) {
        return Err(ContextError::Invalid(
            "import commands require verified bundle admission".into(),
        )
        .into());
    }
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    execute_locked(actor, trusted_scope, principal, request).await
}
pub(crate) async fn execute_locked(
    actor: &ActorEngine,
    trusted_scope: &Scope,
    principal: &Scope,
    mut request: MemoryRequest,
) -> Result<MemoryReceipt, MemoryError> {
    trusted_scope.validate()?;
    principal.validate()?;
    validate_id(&request.request_id)?;
    if request.version != 1 || &request.scope != trusted_scope || principal != trusted_scope {
        return Err(ContextError::ScopeMismatch.into());
    }
    match &mut request.command {
        MemoryCommand::Create { record } | MemoryCommand::Revise { record, .. } => {
            if record.revision_digest.is_empty() {
                record.revision_digest = record.computed_revision_digest()?
            }
        }
        MemoryCommand::SetGrant { grant } => {
            if grant.record_revisions.is_empty() {
                let state = rebuild(actor, trusted_scope).await?;
                for r in state.records.values().filter(|r| {
                    grant.record_ids.contains(&r.id) || grant.categories.contains(&r.category)
                }) {
                    grant
                        .record_revisions
                        .insert(r.id.clone(), r.revision_digest.clone());
                }
            }
        }
        _ => {}
    }
    let tail = actor.stats().await?.applied.last_lsn;
    let mut state = rebuild(actor, trusted_scope).await?;
    let digest = digest_bytes(&serde_json::to_vec(&request)?);
    if let Some((previous, receipt)) = state.receipts.get(&request.request_id) {
        if previous != &digest {
            return Err(ContextError::Conflict.into());
        }
        let mut receipt = receipt.clone();
        receipt.replayed = true;
        return Ok(receipt);
    }
    state.apply(&request.command, tail.get() + 1)?;
    let cursor = state.cursor.checked_add(1).ok_or(ContextError::Capacity)?;
    let event = MemoryEvent {
        version: 1,
        scope: trusted_scope.clone(),
        principal: principal.clone(),
        request_id: request.request_id,
        digest,
        cursor,
        command: request.command,
    };
    let api_content = serde_json::to_vec(&event)?;
    if api_content.len() > 2 * 1024 * 1024 {
        return Err(ContextError::Capacity.into());
    }
    let payload = encode_event_envelope(&EventEnvelope {
        schema_version: CURRENT_SCHEMA_VERSION,
        payload: EventPayload::ProviderFrame(Box::new(ProviderFrame {
            provider: MEMORY_PROVIDER.into(),
            api_content,
        })),
        connection_id: None,
        client_seq: 0,
        client_event_index: 0,
        client_event_count: 0,
        origin_actor: 0,
        run_id: None,
        model_provenance: None,
        authority: hm_schema::events::Authority::ExternalObserved,
        retention: Retention::Durable,
        sensitivity: Sensitivity::Personal,
        event_time_ns: 0,
    });
    let outcome = actor
        .append_if_tail(
            tail,
            vec![IncomingEvent {
                kind: EventKind::ProviderFrame,
                conversation: ConversationId::derive(&format!(
                    "context-memory:{}",
                    trusted_scope.digest()?
                )),
                payload,
            }],
        )
        .await?;
    Ok(MemoryReceipt {
        version: 1,
        scope: trusted_scope.clone(),
        cursor,
        last_lsn: outcome.last_lsn.get(),
        replayed: false,
    })
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryExport {
    pub version: u32,
    pub scope: Scope,
    pub cursor: u64,
    pub events: Vec<Value>,
    pub digest: String,
}
pub async fn export(
    actor: &ActorEngine,
    scope: &Scope,
    principal: &Scope,
) -> Result<MemoryExport, MemoryError> {
    if principal != scope {
        return Err(ContextError::ScopeMismatch.into());
    }
    let state = rebuild(actor, scope).await?;
    let events = state
        .events
        .into_iter()
        .map(|e| serde_json::to_value(e))
        .collect::<Result<Vec<_>, _>>()?;
    let mut output = MemoryExport {
        version: 1,
        scope: scope.clone(),
        cursor: state.cursor,
        events,
        digest: String::new(),
    };
    output.digest = digest_bytes(&serde_json::to_vec(&output)?);
    Ok(output)
}

mod imports;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MutationAudit {
    pub id: String,
    pub epoch: u64,
    pub sequence: u64,
    pub operation: String,
    pub record_id: Option<String>,
    pub previous_revision_digest: Option<String>,
    pub result_revision_digest: Option<String>,
    pub actor_scope_digest: String,
    #[serde(with = "nanos")]
    pub created_at_ns: i64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SmartCondition {
    pub operator: String,
    pub clauses: Vec<ConditionClause>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConditionClause {
    pub field: String,
    pub comparison: String,
    pub value: Value,
}
impl SmartCondition {
    pub fn validate(&self) -> Result<(), ContextError> {
        if !["all", "any", "none"].contains(&self.operator.as_str())
            || self.clauses.is_empty()
            || self.clauses.len() > 32
        {
            return Err(ContextError::Invalid("invalid condition".into()));
        }
        for c in &self.clauses {
            if ![
                "event.kind",
                "event.label",
                "record.kind",
                "record.category",
                "record.status",
                "time.hour",
                "time.weekday",
            ]
            .contains(&c.field.as_str())
                || !["eq", "neq", "in", "contains", "gte", "lte"].contains(&c.comparison.as_str())
                || serde_json::to_vec(&c.value)?.len() > 4096
            {
                return Err(ContextError::Invalid("invalid condition clause".into()));
            }
            if c.comparison == "in"
                && c.value
                    .as_array()
                    .is_none_or(|v| v.is_empty() || v.len() > 64)
            {
                return Err(ContextError::Invalid("invalid condition values".into()));
            }
        }
        Ok(())
    }
    pub fn evaluate(&self, facts: &BTreeMap<String, String>) -> Result<bool, ContextError> {
        self.validate()?;
        let mut values = Vec::new();
        for c in &self.clauses {
            let Some(value) = facts.get(&c.field) else {
                return Err(ContextError::Unavailable("condition fact".into()));
            };
            let actual: Value =
                serde_json::from_str(value).unwrap_or_else(|_| Value::String(value.clone()));
            values.push(match c.comparison.as_str() {
                "eq" => actual == c.value,
                "neq" => actual != c.value,
                "in" => c.value.as_array().is_some_and(|v| v.contains(&actual)),
                "contains" => actual
                    .as_str()
                    .zip(c.value.as_str())
                    .is_some_and(|(a, b)| a.contains(b)),
                "gte" => actual
                    .as_f64()
                    .zip(c.value.as_f64())
                    .is_some_and(|(a, b)| a >= b),
                "lte" => actual
                    .as_f64()
                    .zip(c.value.as_f64())
                    .is_some_and(|(a, b)| a <= b),
                _ => false,
            });
        }
        Ok(match self.operator.as_str() {
            "all" => values.into_iter().all(|v| v),
            "any" => values.into_iter().any(|v| v),
            "none" => values.into_iter().all(|v| !v),
            _ => false,
        })
    }
}
fn principal_digest(scope: &Scope) -> String {
    let mut bytes = b"hypermid.memory.scope.v1\0".to_vec();
    for s in [&scope.owner_id, &scope.project_id] {
        bytes.extend_from_slice(&(s.len() as u64).to_be_bytes());
        bytes.extend_from_slice(s.as_bytes());
    }
    match &scope.workspace_id {
        Some(s) => {
            bytes.push(1);
            bytes.extend_from_slice(&(s.len() as u64).to_be_bytes());
            bytes.extend_from_slice(s.as_bytes());
        }
        None => bytes.push(0),
    }
    digest_bytes(&bytes)
}

mod nanos {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    pub fn serialize<S: Serializer>(v: &i64, s: S) -> Result<S::Ok, S::Error> {
        v.to_string().serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<i64, D::Error> {
        let value = serde_json::Value::deserialize(d)?;
        match value {
            serde_json::Value::String(s) => s.parse().map_err(serde::de::Error::custom),
            serde_json::Value::Number(n) => n
                .as_i64()
                .ok_or_else(|| serde::de::Error::custom("invalid nanoseconds")),
            _ => Err(serde::de::Error::custom("invalid nanoseconds")),
        }
    }
}
mod optional_nanos {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    pub fn serialize<S: Serializer>(v: &Option<i64>, s: S) -> Result<S::Ok, S::Error> {
        v.map(|v| v.to_string()).serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<i64>, D::Error> {
        let value = Option::<serde_json::Value>::deserialize(d)?;
        match value {
            None => Ok(None),
            Some(serde_json::Value::String(s)) => {
                s.parse().map(Some).map_err(serde::de::Error::custom)
            }
            Some(serde_json::Value::Number(n)) => n
                .as_i64()
                .map(Some)
                .ok_or_else(|| serde::de::Error::custom("invalid nanoseconds")),
            _ => Err(serde::de::Error::custom("invalid nanoseconds")),
        }
    }
}

fn acyclic_relation(relation: &str) -> bool {
    matches!(
        relation,
        "derived_from" | "supersedes" | "merged_from" | "split_from"
    )
}
