use crate::{
    notes::Predicate,
    types::{Authority, ContextError, Cursor, Scope, SourceSpan, digest_bytes, validate_id},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DevelopmentKind {
    Historian,
    Extraction,
    Indexing,
    Verification,
    Curation,
    Retrospective,
    Primer,
    ConditionalNote,
    ProfileProposal,
    DocumentationProposal,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DevelopmentRecordKind {
    Fact,
    Episode,
    Note,
    ConditionalNote,
    Anchor,
    Summary,
    Primer,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DevelopmentRecordStatus {
    Active,
    Archived,
    Stale,
    Tombstoned,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DevelopmentProvenance {
    pub source_id: String,
    pub source_digest: String,
    pub span_start: u64,
    pub span_end: u64,
    pub quoted_digest: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DevelopmentLineage {
    pub child_record_id: String,
    pub child_revision: u64,
    pub parent_record_id: String,
    pub parent_revision_digest: String,
    pub relation: String,
    #[serde(with = "crate::types::timestamp_wire")]
    pub created_at_ns: i64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DevelopmentCondition {
    pub operator: String,
    pub clauses: Vec<DevelopmentClause>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DevelopmentClause {
    pub field: String,
    pub comparison: String,
    pub value: Value,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DevelopmentRecord {
    pub id: String,
    pub kind: DevelopmentRecordKind,
    pub category: String,
    pub status: DevelopmentRecordStatus,
    pub revision: u64,
    pub revision_digest: String,
    pub content: String,
    pub authority: Authority,
    pub confidence: u32,
    pub importance: u32,
    #[serde(with = "crate::types::optional_timestamp_wire")]
    pub occurred_at_ns: Option<i64>,
    #[serde(with = "crate::types::timestamp_wire")]
    pub recorded_at_ns: i64,
    #[serde(with = "crate::types::optional_timestamp_wire")]
    pub expires_at_ns: Option<i64>,
    pub pinned: bool,
    pub provenance: Vec<DevelopmentProvenance>,
    pub lineage: Vec<DevelopmentLineage>,
    pub contradictions: Vec<String>,
    pub last_lsn: u64,
    pub predicate: Option<Predicate>,
    pub smart_condition: Option<DevelopmentCondition>,
    #[serde(with = "crate::types::optional_timestamp_wire")]
    pub retention_until_ns: Option<i64>,
    pub metadata: Value,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceOrigin {
    Memory,
    Conversation,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceSource {
    pub id: String,
    pub origin: EvidenceOrigin,
    pub revision: u64,
    pub digest: String,
    pub content_digest: String,
    pub authority: Authority,
    pub content: Vec<u8>,
    pub spans: Vec<SourceSpan>,
    #[serde(with = "crate::types::optional_timestamp_wire")]
    pub occurred_at_ns: Option<i64>,
    #[serde(with = "crate::types::timestamp_wire")]
    pub recorded_at_ns: i64,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DevelopmentLease {
    pub id: String,
    pub attempt: u64,
    #[serde(with = "crate::types::timestamp_wire")]
    pub expires_at_ns: i64,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DevelopmentBudget {
    pub reserved_tokens: u64,
    pub max_input_bytes: u64,
    pub max_output_bytes: u64,
    pub max_mutations: usize,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerCapability {
    pub id: String,
    pub scope: Scope,
    pub principal: Scope,
    pub worker_id: String,
    pub revision: u64,
    pub revoked: bool,
    pub allowed_kinds: BTreeSet<DevelopmentKind>,
    pub source_ids: BTreeSet<String>,
    pub record_ids: BTreeSet<String>,
    pub new_record_ids: BTreeSet<String>,
    pub session_id: String,
    pub conversation: String,
    pub lease: DevelopmentLease,
    pub budget: DevelopmentBudget,
}
impl WorkerCapability {
    pub fn validate(&self) -> Result<(), ContextError> {
        self.scope.validate()?;
        self.principal.validate()?;
        for id in [
            &self.id,
            &self.worker_id,
            &self.session_id,
            &self.conversation,
            &self.lease.id,
        ] {
            validate_id(id)?;
        }
        for id in self
            .source_ids
            .iter()
            .chain(&self.record_ids)
            .chain(&self.new_record_ids)
        {
            validate_id(id)?;
        }
        if self.revision == 0
            || self.lease.attempt == 0
            || self.allowed_kinds.is_empty()
            || self.source_ids.len() > 256
            || self.record_ids.len() > 256
            || self.new_record_ids.len() > 64
            || !self.record_ids.is_disjoint(&self.new_record_ids)
            || self.budget.reserved_tokens == 0
            || self.budget.max_mutations == 0
            || self.budget.max_mutations > 64
            || self.budget.max_input_bytes == 0
            || self.budget.max_input_bytes > 4 * 1024 * 1024
            || self.budget.max_output_bytes == 0
            || self.budget.max_output_bytes > 1024 * 1024
        {
            return Err(ContextError::Invalid(
                "invalid worker capability bounds".into(),
            ));
        }
        Ok(())
    }
    pub fn digest(&self) -> Result<String, ContextError> {
        self.validate()?;
        Ok(digest_bytes(&serde_json::to_vec(self)?))
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotRequest {
    pub capability_id: String,
    pub source_ids: BTreeSet<String>,
    pub record_ids: BTreeSet<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceSnapshot {
    pub version: u32,
    pub scope: Scope,
    pub principal: Scope,
    pub worker_id: String,
    pub capability_id: String,
    pub capability_revision: u64,
    pub capability_digest: String,
    pub session_id: String,
    pub conversation: String,
    pub ledger_tail: u64,
    pub memory_cursor: u64,
    pub source_cursor: Cursor,
    pub policy_revision: u64,
    pub grant_digest: String,
    pub sources: Vec<EvidenceSource>,
    pub records: Vec<DevelopmentRecord>,
    pub lease: DevelopmentLease,
    pub budget: DevelopmentBudget,
    pub digest: String,
}
impl EvidenceSnapshot {
    pub fn computed_digest(&self) -> Result<String, ContextError> {
        let mut value = self.clone();
        value.digest.clear();
        Ok(digest_bytes(&serde_json::to_vec(&value)?))
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DevelopmentVerificationState {
    Supported,
    Contradicted,
    Unresolved,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DevelopmentVerification {
    pub id: String,
    pub record_id: String,
    pub revision_digest: String,
    pub state: DevelopmentVerificationState,
    pub evidence_source_id: Option<String>,
    pub confidence: u32,
    #[serde(with = "crate::types::timestamp_wire")]
    pub created_at_ns: i64,
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub metadata: Value,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlannedKnowledgeMutation {
    Create {
        record: DevelopmentRecord,
    },
    Revise {
        record: DevelopmentRecord,
        expected_revision: u64,
        expected_digest: String,
    },
    SetStatus {
        id: String,
        status: DevelopmentRecordStatus,
        expected_revision: u64,
        expected_digest: String,
    },
    Verify {
        verification: DevelopmentVerification,
    },
    AddLineage {
        lineage: DevelopmentLineage,
    },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalStatus {
    Pending,
    Accepted,
    Rejected,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApprovalProposal {
    pub id: String,
    pub revision: u64,
    pub kind: DevelopmentKind,
    pub scope: Scope,
    pub worker_id: String,
    pub evidence: EvidenceSnapshot,
    pub mutations: Vec<PlannedKnowledgeMutation>,
    pub status: ProposalStatus,
    pub digest: String,
}
impl ApprovalProposal {
    pub fn computed_digest(&self) -> Result<String, ContextError> {
        let mut value = self.clone();
        value.digest.clear();
        Ok(digest_bytes(&serde_json::to_vec(&value)?))
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DevelopmentPlan {
    pub version: u32,
    pub id: String,
    pub kind: DevelopmentKind,
    pub evidence: EvidenceSnapshot,
    pub mutations: Vec<PlannedKnowledgeMutation>,
    pub proposal: Option<ApprovalProposal>,
    pub usage: crate::maintenance::Usage,
}
impl DevelopmentPlan {
    pub fn digest(&self) -> Result<String, ContextError> {
        Ok(digest_bytes(&serde_json::to_vec(self)?))
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProposalDecisionKind {
    Accept,
    Reject,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProposalDecision {
    pub request_id: String,
    pub proposal_id: String,
    pub expected_revision: u64,
    pub expected_digest: String,
    pub decision: ProposalDecisionKind,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerReceipt {
    pub version: u32,
    pub plan_id: String,
    pub scope: Scope,
    pub ledger_lsn: u64,
    pub cursor: u64,
    pub snapshot_digest: String,
    pub plan_digest: String,
    pub mutation_ids: Vec<String>,
    pub proposal_id: Option<String>,
    pub replayed: bool,
}
