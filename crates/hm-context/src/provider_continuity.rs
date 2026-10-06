use crate::{history::SourceHistory, provider::{render_context, CapabilityProfile, RenderError, RenderRequest, RenderedContext, TokenCounter}, types::*};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderProfile {
    pub model_id: String,
    pub model_revision: String,
    pub tokenizer_id: String,
    pub tokenizer_revision: String,
    pub serializer_id: String,
    pub serializer_revision: String,
    pub capabilities: CapabilityProfile,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProfileFence { pub digest: String }

pub fn fence_profile(profile: &ProviderProfile, budget: TokenBudget, source_revision: &str) -> Result<ProfileFence, ContextError> {
    for identity in [&profile.model_id, &profile.model_revision, &profile.tokenizer_id, &profile.tokenizer_revision, &profile.serializer_id, &profile.serializer_revision, source_revision] {
        if identity.is_empty() { return Err(ContextError::Invalid("missing provider profile identity".into())); }
    }
    budget.available()?;
    Ok(ProfileFence { digest: digest_bytes(&serde_json::to_vec(&(profile, budget, source_revision))?) })
}

#[derive(Debug, thiserror::Error)]
pub enum ContinuityError {
    #[error(transparent)] Context(#[from] ContextError),
    #[error(transparent)] Render(#[from] RenderError),
    #[error("unsupported serializer: {0}")] UnsupportedSerializer(String),
    #[error("provider profile changed")] ProfileChanged,
    #[error("provider dispatch cancelled")] Cancelled,
    #[error("output token ceiling exceeded")] OutputOverflow,
}

/// Only the canonical context envelope is supported here. Provider-specific chat
/// framing requires its own serializer and a counter for that exact wire format.
pub fn normalize_tool_pairs(request: RenderRequest<'_>, profile: &ProviderProfile, expected: &ProfileFence, source_revision: &str, counter: &dyn TokenCounter, cancelled: bool) -> Result<RenderedContext, ContinuityError> {
    if cancelled { return Err(ContinuityError::Cancelled); }
    if profile.serializer_id != "context-json-v1" || profile.serializer_revision != "1" {
        return Err(ContinuityError::UnsupportedSerializer(profile.serializer_id.clone()));
    }
    if &fence_profile(profile, request.budget, source_revision)? != expected { return Err(ContinuityError::ProfileChanged); }
    if serde_json::to_vec(&request.profile).map_err(ContextError::from)? != serde_json::to_vec(&profile.capabilities).map_err(ContextError::from)? { return Err(ContinuityError::ProfileChanged); }
    Ok(render_context(request, counter)?)
}

pub fn validate_opaque_parts(history: &SourceHistory, scope: &Scope, messages: &[SourceMessage]) -> Result<(), ContextError> {
    if scope != history.scope() { return Err(ContextError::ScopeMismatch); }
    for message in messages {
        if history.message(&message.id)? != message { return Err(ContextError::Stale); }
        for part in &message.parts {
            if let MessagePart::Opaque { media_type, reference, digest } = part {
                if media_type.is_empty() || reference.is_empty() || digest.is_empty() { return Err(ContextError::Invalid("incomplete opaque reference".into())); }
            }
        }
        history.recover(scope, &history.source_span(&message.id)?)?;
    }
    Ok(())
}

/// Original spans remain recoverable independently of dispatch capabilities.
pub fn recovery_plan(history: &SourceHistory, scope: &Scope, messages: &[SourceMessage]) -> Result<BTreeMap<String, SourceSpan>, ContextError> {
    validate_opaque_parts(history, scope, messages)?;
    messages.iter().map(|message| Ok((message.id.clone(), history.source_span(&message.id)?))).collect()
}

pub fn validate_output_budget(observed_output_tokens: u64, budget: TokenBudget) -> Result<(), ContinuityError> {
    budget.available()?;
    if observed_output_tokens > budget.reserved_output_tokens { return Err(ContinuityError::OutputOverflow); }
    Ok(())
}
