use crate::{
    history::{SourceHistory, SourceRelation},
    maintenance::Usage,
    provider::TokenCounter,
    reduction::{ReductionItem, SummaryTier},
    types::*,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContinuityFence {
    pub scope: Scope,
    pub session_id: String,
    pub cursor: Cursor,
    pub history_digest: String,
}
impl ContinuityFence {
    pub fn capture(history: &SourceHistory) -> Result<Self, ContextError> {
        Ok(Self {
            scope: history.scope().clone(),
            session_id: history.session_id().into(),
            cursor: history.cursor(),
            history_digest: digest_bytes(&history.export_canonical()?),
        })
    }
    pub fn validate(&self, history: &SourceHistory) -> Result<(), ContextError> {
        if self.scope != *history.scope() || self.session_id != history.session_id() {
            return Err(ContextError::ScopeMismatch);
        }
        if self != &Self::capture(history)? {
            return Err(ContextError::Stale);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceIdentity {
    pub id: String,
    pub ordinal: u64,
    pub role: MessageRole,
    pub authority: Authority,
    pub span: SourceSpan,
    pub original_digest: String,
    #[serde(with = "crate::types::optional_timestamp_wire")]
    pub occurred_at_ns: Option<i64>,
    #[serde(with = "crate::types::timestamp_wire")]
    pub recorded_at_ns: i64,
}
fn identity(history: &SourceHistory, id: &str) -> Result<SourceIdentity, ContextError> {
    let message = history.message(id)?;
    let span = history.source_span(id)?;
    Ok(SourceIdentity {
        id: id.into(),
        ordinal: message.ordinal,
        role: message.role,
        authority: message.authority,
        original_digest: digest_bytes(&history.recover(history.scope(), &span)?),
        span,
        occurred_at_ns: message.occurred_at_ns,
        recorded_at_ns: message.recorded_at_ns,
    })
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributionLimits {
    pub max_blocks: usize,
    pub max_sources: usize,
    pub max_bytes: u64,
    pub max_output_tokens: u64,
    pub max_child_tokens: u64,
    pub max_child_calls: u64,
}
impl ContributionLimits {
    pub fn validate(&self) -> Result<(), ContextError> {
        if self.max_blocks == 0
            || self.max_blocks > 64
            || self.max_sources == 0
            || self.max_sources > 256
            || self.max_bytes == 0
            || self.max_bytes > 1024 * 1024
            || self.max_output_tokens == 0
            || self.max_child_tokens == 0
            || self.max_child_calls == 0
        {
            return Err(ContextError::Invalid("invalid contribution ceiling".into()));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForkPlan {
    pub parent: ContinuityFence,
    pub child_session_id: String,
    pub inherited: Vec<SourceIdentity>,
    pub limits: ContributionLimits,
    pub digest: String,
}
impl ForkPlan {
    pub fn computed_digest(&self) -> Result<String, ContextError> {
        let mut value = self.clone();
        value.digest.clear();
        Ok(digest_bytes(&serde_json::to_vec(&value)?))
    }
    pub fn validate_parent(&self, history: &SourceHistory) -> Result<(), ContextError> {
        if self.digest != self.computed_digest()? {
            return Err(ContextError::Stale);
        }
        self.limits.validate()?;
        self.parent.validate(history)?;
        if self.inherited
            != history
                .messages()
                .iter()
                .map(|m| identity(history, &m.id))
                .collect::<Result<Vec<_>, _>>()?
        {
            return Err(ContextError::Stale);
        }
        Ok(())
    }
    pub fn validate_child(&self, history: &SourceHistory) -> Result<(), ContextError> {
        if self.digest != self.computed_digest()? {
            return Err(ContextError::Stale);
        }
        self.limits.validate()?;
        if history.scope() != &self.parent.scope || history.session_id() != self.child_session_id {
            return Err(ContextError::ScopeMismatch);
        }
        let parent = history.parent().ok_or(ContextError::Conflict)?;
        if parent.session_id != self.parent.session_id
            || parent.cursor != self.parent.cursor
            || parent.digest != self.parent.history_digest
        {
            return Err(ContextError::Stale);
        }
        for inherited in &self.inherited {
            if &identity(history, &inherited.id)? != inherited {
                return Err(ContextError::Stale);
            }
        }
        Ok(())
    }
}
pub fn plan_fork(
    history: &SourceHistory,
    child_session_id: &str,
    limits: ContributionLimits,
) -> Result<ForkPlan, ContextError> {
    validate_id(child_session_id)?;
    limits.validate()?;
    if history.session_id() == child_session_id {
        return Err(ContextError::Conflict);
    }
    let mut plan = ForkPlan {
        parent: ContinuityFence::capture(history)?,
        child_session_id: child_session_id.into(),
        inherited: history
            .messages()
            .iter()
            .map(|m| identity(history, &m.id))
            .collect::<Result<Vec<_>, _>>()?,
        limits,
        digest: String::new(),
    };
    plan.digest = plan.computed_digest()?;
    Ok(plan)
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditPlan {
    pub fence: ContinuityFence,
    pub relation: SourceRelation,
    pub original: SourceIdentity,
    pub replacement: Option<SourceIdentity>,
}
impl EditPlan {
    pub fn validate(&self, history: &SourceHistory) -> Result<(), ContextError> {
        self.fence.validate(history)?;
        if self != &plan_edit(history, self.relation.clone())? {
            return Err(ContextError::Stale);
        }
        Ok(())
    }
}
pub fn plan_edit(
    history: &SourceHistory,
    relation: SourceRelation,
) -> Result<EditPlan, ContextError> {
    let (original, replacement) = match &relation {
        SourceRelation::Edit {
            original_id,
            replacement_id,
            ..
        }
        | SourceRelation::Regenerate {
            original_id,
            replacement_id,
            ..
        } => (original_id.as_str(), Some(replacement_id.as_str())),
        SourceRelation::Tombstone { source_id, .. } => (source_id.as_str(), None),
    };
    let mut candidate = history.clone();
    candidate.relate(relation.clone())?;
    Ok(EditPlan {
        fence: ContinuityFence::capture(history)?,
        original: identity(history, original)?,
        replacement: replacement.map(|id| identity(history, id)).transpose()?,
        relation,
    })
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReductionRelation {
    pub fence: ContinuityFence,
    pub artifact_id: String,
    pub artifact_digest: String,
    pub tier: SummaryTier,
    pub coverage: Vec<SourceSpan>,
    pub sources: Vec<SourceIdentity>,
}
fn coverage(
    history: &SourceHistory,
    spans: &[SourceSpan],
) -> Result<Vec<SourceIdentity>, ContextError> {
    if spans.is_empty() || spans.len() > 256 {
        return Err(ContextError::Invalid(
            "bounded original source coverage required".into(),
        ));
    }
    let visible: BTreeSet<_> = history
        .visible_messages()
        .iter()
        .map(|m| m.id.as_str())
        .collect();
    let mut seen = BTreeSet::new();
    let mut sources = Vec::new();
    for span in spans {
        if !visible.contains(span.source_id.as_str()) || span.byte_start >= span.byte_end {
            return Err(ContextError::Stale);
        }
        history.recover(history.scope(), span)?;
        if seen.insert(span.source_id.clone()) {
            sources.push(identity(history, &span.source_id)?);
        }
    }
    Ok(sources)
}
pub fn reduction_relation(
    history: &SourceHistory,
    item: &ReductionItem,
    summary: &ContextBlock,
    tier: SummaryTier,
) -> Result<ReductionRelation, ContextError> {
    validate_id(&item.original.id)?;
    if summary.id != item.original.id
        || summary.authority != Authority::DerivedInference
        || summary.provenance != item.original.provenance
        || summary.required != item.original.required
        || item.original.tokens == 0
        || summary.tokens == 0
        || summary.tokens > item.original.tokens
        || summary.text.is_empty()
    {
        return Err(ContextError::Invalid(
            "reduction must retain exact original source coverage".into(),
        ));
    }
    let sources = coverage(history, &summary.provenance)?;
    Ok(ReductionRelation {
        fence: ContinuityFence::capture(history)?,
        artifact_id: summary.id.clone(),
        artifact_digest: digest_bytes(&serde_json::to_vec(summary)?),
        tier,
        coverage: summary.provenance.clone(),
        sources,
    })
}
impl ReductionRelation {
    pub fn validate(
        &self,
        history: &SourceHistory,
        summary: &ContextBlock,
    ) -> Result<(), ContextError> {
        self.fence.validate(history)?;
        if summary.id != self.artifact_id
            || summary.authority != Authority::DerivedInference
            || summary.provenance != self.coverage
            || digest_bytes(&serde_json::to_vec(summary)?) != self.artifact_digest
            || coverage(history, &self.coverage)? != self.sources
        {
            return Err(ContextError::Stale);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContributionPlan {
    pub parent: ContinuityFence,
    pub child: ContinuityFence,
    pub fork_digest: String,
    pub blocks: Vec<ContextBlock>,
    pub sources: Vec<SourceIdentity>,
    pub output_tokens: u64,
    pub child_usage: Usage,
    pub child_calls: u64,
    pub digest: String,
}
impl ContributionPlan {
    pub fn computed_digest(&self) -> Result<String, ContextError> {
        let mut value = self.clone();
        value.digest.clear();
        Ok(digest_bytes(&serde_json::to_vec(&value)?))
    }
    pub fn validate(
        &self,
        parent: &SourceHistory,
        child: &SourceHistory,
        fork: &ForkPlan,
        counter: &dyn TokenCounter,
    ) -> Result<(), ContextError> {
        self.parent.validate(parent)?;
        self.child.validate(child)?;
        let expected = plan_contribution(
            parent,
            child,
            fork,
            &self.blocks,
            self.child_usage,
            self.child_calls,
            counter,
        )?;
        if self != &expected {
            return Err(ContextError::Stale);
        }
        Ok(())
    }
}
pub fn plan_contribution(
    parent: &SourceHistory,
    child: &SourceHistory,
    fork: &ForkPlan,
    blocks: &[ContextBlock],
    usage: Usage,
    calls: u64,
    counter: &dyn TokenCounter,
) -> Result<ContributionPlan, ContextError> {
    fork.validate_child(child)?;
    if parent.scope() != child.scope() || parent.session_id() != fork.parent.session_id {
        return Err(ContextError::ScopeMismatch);
    }
    let Usage::Known(tokens) = usage else {
        return Err(ContextError::Unavailable(
            "unknown child usage cannot satisfy a contribution ceiling".into(),
        ));
    };
    if tokens > fork.limits.max_child_tokens
        || calls > fork.limits.max_child_calls
        || blocks.len() > fork.limits.max_blocks
    {
        return Err(ContextError::Capacity);
    }
    let mut seen = BTreeSet::new();
    let mut all_spans = Vec::new();
    let mut bytes = 0u64;
    let mut normalized = Vec::new();
    for block in blocks {
        validate_id(&block.id)?;
        if !seen.insert(block.id.clone())
            || block.authority != Authority::DerivedInference
            || block.required
            || block.text.is_empty()
        {
            return Err(ContextError::Invalid(
                "child contributions must be attributed derived blocks".into(),
            ));
        }
        bytes = bytes
            .checked_add(block.text.len() as u64)
            .ok_or(ContextError::Capacity)?;
        coverage(child, &block.provenance)?;
        all_spans.extend(block.provenance.clone());
        let mut block = block.clone();
        block.tokens = counter.count(block.text.as_bytes())?;
        normalized.push(block);
    }
    let output_tokens = counter.count(&serde_json::to_vec(&normalized)?)?;
    if bytes > fork.limits.max_bytes || output_tokens > fork.limits.max_output_tokens {
        return Err(ContextError::Capacity);
    }
    let sources = if all_spans.is_empty() {
        vec![]
    } else {
        coverage(child, &all_spans)?
    };
    if sources.len() > fork.limits.max_sources {
        return Err(ContextError::Capacity);
    }
    let mut plan = ContributionPlan {
        parent: ContinuityFence::capture(parent)?,
        child: ContinuityFence::capture(child)?,
        fork_digest: fork.digest.clone(),
        blocks: normalized,
        sources,
        output_tokens,
        child_usage: usage,
        child_calls: calls,
        digest: String::new(),
    };
    plan.digest = plan.computed_digest()?;
    Ok(plan)
}
