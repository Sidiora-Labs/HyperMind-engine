use crate::{
    Authority, ContextBlock, ContextError, Scope, SourceSpan, digest_bytes, provider::TokenCounter,
    validate_id,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeRecord {
    pub id: String,
    pub revision_digest: String,
    pub category: String,
    pub text: String,
    pub authority: Authority,
    pub pinned: bool,
    pub importance: u32,
    #[serde(with = "crate::types::timestamp_wire")]
    pub recorded_at_ns: i64,
    pub provenance: Vec<SourceSpan>,
    pub verbatim_provenance: bool,
    pub observed_uses: u64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeSnapshot {
    pub scope: Scope,
    pub principal: Scope,
    pub session_id: String,
    pub generation: u64,
    pub records: Vec<KnowledgeRecord>,
    pub authority_digest: String,
    pub digest: String,
}
impl KnowledgeSnapshot {
    pub fn seal(&mut self) -> Result<(), ContextError> {
        self.scope.validate()?;
        self.principal.validate()?;
        validate_id(&self.session_id)?;
        self.records.sort_by(|a, b| a.id.cmp(&b.id));
        if self.records.len() > 100_000 || self.records.windows(2).any(|w| w[0].id == w[1].id) {
            return Err(ContextError::Capacity);
        }
        for record in &self.records {
            validate_id(&record.id)?;
            validate_id(&record.category)?;
            if record.importance > 1_000_000 {
                return Err(ContextError::Invalid("knowledge importance".into()));
            }
        }
        self.digest.clear();
        self.digest = digest_bytes(&serde_json::to_vec(self)?);
        Ok(())
    }
    pub fn validate(&self) -> Result<(), ContextError> {
        let mut copy = self.clone();
        copy.seal()?;
        if copy.digest != self.digest || copy.records != self.records {
            return Err(ContextError::Stale);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VisibleRecord {
    pub id: String,
    pub revision_digest: String,
}
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct VisibleEvidence {
    pub records: Vec<VisibleRecord>,
    pub source_spans: Vec<SourceSpan>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct KnowledgeOmission {
    pub id: String,
    pub reason: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct KnowledgePlan {
    pub scope: Scope,
    pub principal: Scope,
    pub session_id: String,
    pub generation: u64,
    pub snapshot_digest: String,
    pub authority_digest: String,
    pub blocks: Vec<ContextBlock>,
    pub selected: Vec<VisibleRecord>,
    pub omissions: Vec<KnowledgeOmission>,
    pub token_count: u64,
    pub digest: String,
}
impl KnowledgePlan {
    pub fn validate(
        &self,
        snapshot: &KnowledgeSnapshot,
        generation: u64,
    ) -> Result<(), ContextError> {
        snapshot.validate()?;
        if self.scope != snapshot.scope
            || self.principal != snapshot.principal
            || self.session_id != snapshot.session_id
            || self.generation != generation
            || snapshot.generation != generation
            || self.authority_digest != snapshot.authority_digest
        {
            return Err(ContextError::Stale);
        }
        let mut copy = self.clone();
        copy.digest.clear();
        if digest_bytes(&serde_json::to_vec(&copy)?) != self.digest {
            return Err(ContextError::Stale);
        }
        if self.blocks.len() != self.selected.len()
            || self
                .blocks
                .iter()
                .map(|b| b.tokens)
                .try_fold(0u64, |a, b| a.checked_add(b))
                .ok_or(ContextError::Capacity)?
                != self.token_count
        {
            return Err(ContextError::Stale);
        }
        let mut ids = BTreeSet::new();
        for (block, selected) in self.blocks.iter().zip(&self.selected) {
            let record = snapshot
                .records
                .iter()
                .find(|r| r.id == selected.id && r.revision_digest == selected.revision_digest)
                .ok_or(ContextError::Stale)?;
            if !ids.insert(&selected.id)
                || block.id != record.id
                || block.text != record.text
                || block.authority != record.authority
                || block.provenance != record.provenance
                || block.required
            {
                return Err(ContextError::Stale);
            }
        }
        Ok(())
    }
}
pub fn select_knowledge(
    snapshot: &KnowledgeSnapshot,
    budget: u64,
    visible: &VisibleEvidence,
    counter: &dyn TokenCounter,
) -> Result<KnowledgePlan, ContextError> {
    snapshot.validate()?;
    let mut candidates: Vec<_> = snapshot.records.iter().collect();
    candidates.sort_by(|a, b| {
        b.pinned
            .cmp(&a.pinned)
            .then(a.category.cmp(&b.category))
            .then(b.importance.cmp(&a.importance))
            .then(b.recorded_at_ns.cmp(&a.recorded_at_ns))
            .then(a.observed_uses.cmp(&b.observed_uses))
            .then(a.id.cmp(&b.id))
    });
    let mut plan = KnowledgePlan {
        scope: snapshot.scope.clone(),
        principal: snapshot.principal.clone(),
        session_id: snapshot.session_id.clone(),
        generation: snapshot.generation,
        snapshot_digest: snapshot.digest.clone(),
        authority_digest: snapshot.authority_digest.clone(),
        blocks: vec![],
        selected: vec![],
        omissions: vec![],
        token_count: 0,
        digest: String::new(),
    };
    for record in candidates {
        let exact_record = visible
            .records
            .iter()
            .any(|v| v.id == record.id && v.revision_digest == record.revision_digest);
        let exact_sources = record.verbatim_provenance
            && !record.provenance.is_empty()
            && record
                .provenance
                .iter()
                .all(|span| visible.source_spans.contains(span));
        if exact_record || exact_sources {
            plan.omissions.push(KnowledgeOmission {
                id: record.id.clone(),
                reason: "visible_evidence".into(),
            });
            continue;
        }
        let mut block = ContextBlock {
            id: record.id.clone(),
            text: record.text.clone(),
            authority: record.authority,
            provenance: record.provenance.clone(),
            tokens: 0,
            required: false,
        };
        block.tokens = counter.count(&serde_json::to_vec(&block)?)?;
        let total = plan
            .token_count
            .checked_add(block.tokens)
            .ok_or(ContextError::Capacity)?;
        if total > budget {
            plan.omissions.push(KnowledgeOmission {
                id: record.id.clone(),
                reason: "budget_exact_recall_available".into(),
            });
            continue;
        }
        plan.token_count = total;
        plan.selected.push(VisibleRecord {
            id: record.id.clone(),
            revision_digest: record.revision_digest.clone(),
        });
        plan.blocks.push(block);
    }
    plan.digest = digest_bytes(&serde_json::to_vec(&plan)?);
    Ok(plan)
}
