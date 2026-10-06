use crate::{cache::CacheFence, provider::{render_context, CapabilityProfile, RenderError, RenderRequest, RenderedContext, TokenCounter}, reduction::{self, ReductionError, ReductionItem, ReductionPlan, ReductionPolicy}, types::*};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HookFence {
    pub cache: CacheFence,
    pub generation: u64,
    pub grant_revision: String,
    pub profile_revision: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PressureSnapshot {
    pub fence: HookFence,
    pub cursor: Cursor,
    pub messages: Vec<SourceMessage>,
    pub source_spans: BTreeMap<String, SourceSpan>,
    pub items: Vec<ReductionItem>,
    pub policy: ReductionPolicy,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PressurePlan {
    pub fence: HookFence,
    pub reduction: ReductionPlan,
    pub rendered: RenderedContext,
    pub profile_digest: String,
    pub budget: TokenBudget,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HookReceipt {
    pub fence: HookFence,
    pub rendered_digest: String,
    pub profile_digest: String,
    pub budget: TokenBudget,
}
#[derive(Debug, thiserror::Error)]
pub enum PressureError {
    #[error(transparent)] Context(#[from] ContextError),
    #[error(transparent)] Reduction(#[from] ReductionError),
    #[error(transparent)] Render(#[from] RenderError),
    #[error("hook generation, source, grant or profile changed")] Stale,
}
pub fn pre_hook_fence(snapshot: &PressureSnapshot) -> Result<HookFence, PressureError> {
    snapshot.fence.cache.validate()?;
    snapshot.cursor.validate()?;
    validate_id(&snapshot.fence.grant_revision)?;
    validate_id(&snapshot.fence.profile_revision)?;
    if snapshot.fence.generation == 0 { return Err(PressureError::Stale); }
    Ok(snapshot.fence.clone())
}
pub fn post_hook_receipt(before: &HookFence, current: &HookFence, plan: &PressurePlan, profile: CapabilityProfile, budget: TokenBudget) -> Result<HookReceipt, PressureError> {
    let profile_digest = digest_bytes(&serde_json::to_vec(&profile).map_err(ContextError::from)?);
    if before != current || before != &plan.fence || profile_digest != plan.profile_digest || budget != plan.budget { return Err(PressureError::Stale); }
    Ok(HookReceipt { fence: current.clone(), rendered_digest: plan.rendered.digest.clone(), profile_digest, budget })
}
pub fn plan_pressure(snapshot: &PressureSnapshot, required: &[String], budget: TokenBudget, profile: CapabilityProfile, counter: &dyn TokenCounter) -> Result<PressurePlan, PressureError> {
    let fence = pre_hook_fence(snapshot)?;
    let mut items = snapshot.items.clone();
    let known: BTreeSet<_> = items.iter().map(|i| i.original.id.clone()).collect();
    if required.iter().any(|id| !known.contains(id)) { return Err(ContextError::Invalid("unknown required evidence".into()).into()); }
    let message_ids: BTreeSet<_> = snapshot.messages.iter().map(|m| m.id.clone()).collect();
    for message in &snapshot.messages {
        let item = items.iter().find(|i| i.original.id == message.id).ok_or_else(|| ContextError::Invalid("missing source item".into()))?;
        let span = snapshot.source_spans.get(&message.id).ok_or_else(|| ContextError::Invalid("missing source span".into()))?;
        if item.original.provenance != vec![span.clone()] || span.source_digest != message.source_digest { return Err(PressureError::Stale); }
    }
    for item in &mut items {
        if required.contains(&item.original.id) { item.original.required = true; for summary in item.summaries.iter_mut().flatten() { summary.required = true; } }
        if snapshot.messages.iter().any(|m| m.id == item.original.id && m.parts.iter().any(|p| !matches!(p, MessagePart::Text { .. }))) { item.summaries = [None,None,None,None]; }
    }
    reduction::protect_tool_dependencies(&mut items, &snapshot.messages)?;
    let available = budget.available()?;
    let mut estimated = available;
    let mut attempts = 0usize;
    loop {
        attempts += 1;
        let reduction = reduction::select(&items, TokenBudget { context_tokens: estimated.checked_add(1).ok_or(ContextError::Capacity)?, reserved_output_tokens: 1, required_tokens: 0 }, snapshot.policy)?;
        let original_ids: BTreeSet<_> = reduction.selections.iter().filter(|s| s.tier.is_none() && message_ids.contains(&s.id)).map(|s| s.id.clone()).collect();
        let messages: Vec<_> = snapshot.messages.iter().filter(|m| original_ids.contains(&m.id)).cloned().collect();
        let mut blocks: Vec<_> = reduction.blocks.iter().filter(|b| !original_ids.contains(&b.id)).cloned().collect();
        for block in &mut blocks { block.required = true; }
        let result = render_context(RenderRequest { scope: fence.cache.scope.clone(), session_id: fence.cache.session_id.clone(), cursor: snapshot.cursor, generation: fence.generation, messages: &messages, source_spans: &snapshot.source_spans, blocks: &blocks, required_message_ids: &original_ids.into_iter().collect::<Vec<_>>(), budget, profile }, counter);
        match result {
            Ok(rendered) => return Ok(PressurePlan { fence, reduction, rendered, profile_digest: digest_bytes(&serde_json::to_vec(&profile).map_err(ContextError::from)?), budget }),
            Err(RenderError::RequiredOverflow { required, available: exact_available }) => {
                let next = estimated.saturating_sub(required.saturating_sub(exact_available).max(1));
                if next == estimated || attempts > items.len().saturating_mul(5).saturating_add(2) { return Err(RenderError::RequiredOverflow {required, available: exact_available}.into()); }
                estimated = next;
            }
            Err(error) => return Err(error.into()),
        }
    }
}
