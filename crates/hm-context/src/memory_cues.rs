use crate::{
    Authority, ContextError, Scope, TokenBudget, digest_bytes, provider::TokenCounter, validate_id,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CueSource {
    pub id: String,
    pub digest: String,
    pub start: u64,
    pub end: u64,
    pub quoted_digest: String,
    pub bytes: Vec<u8>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CueRecord {
    pub id: String,
    pub revision: u64,
    pub revision_digest: String,
    pub content: String,
    pub authority: Authority,
    pub sources: Vec<CueSource>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CueSnapshot {
    pub scope: Scope,
    pub principal: Scope,
    pub permission_digest: String,
    pub records: Vec<CueRecord>,
}
impl CueSnapshot {
    pub fn digest(&self) -> Result<String, ContextError> {
        self.scope.validate()?;
        self.principal.validate()?;
        if self.records.len() > 256 {
            return Err(ContextError::Capacity);
        }
        let mut ids = BTreeSet::new();
        let mut input = 0usize;
        for record in &self.records {
            validate_id(&record.id)?;
            if !ids.insert(&record.id) || record.revision == 0 || record.revision_digest.is_empty()
            {
                return Err(ContextError::Conflict);
            }
            input = input
                .checked_add(record.content.len())
                .ok_or(ContextError::Capacity)?;
            for source in &record.sources {
                validate_id(&source.id)?;
                if source.start >= source.end
                    || source.end - source.start != source.bytes.len() as u64
                    || source.quoted_digest != digest_bytes(&source.bytes)
                {
                    return Err(ContextError::Stale);
                }
                input = input
                    .checked_add(source.bytes.len())
                    .ok_or(ContextError::Capacity)?;
            }
        }
        if input > 4 * 1024 * 1024 {
            return Err(ContextError::Capacity);
        }
        Ok(digest_bytes(&serde_json::to_vec(self)?))
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CueProfile {
    pub model_id: String,
    pub max_cue_bytes: usize,
    pub request_visual: bool,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualAvailability {
    NotRequested,
    UnavailableTextFallback,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompactCue {
    pub id: String,
    pub record_id: String,
    pub revision_digest: String,
    pub content_digest: String,
    pub authority: Authority,
    pub text: String,
    pub truncated: bool,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CueAccounting {
    pub input_record_tokens: u64,
    pub text_tokens: u64,
    pub image_tokens: u64,
    pub stored_bytes: u64,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CuePlan {
    pub version: u32,
    pub generation: u64,
    pub snapshot_digest: String,
    pub profile: CueProfile,
    pub budget: TokenBudget,
    pub cues: Vec<CompactCue>,
    pub omitted: Vec<String>,
    pub visual: VisualAvailability,
    pub accounting: CueAccounting,
    pub rendered: Vec<u8>,
    pub digest: String,
}
impl CuePlan {
    pub fn computed_digest(&self) -> Result<String, ContextError> {
        let mut copy = self.clone();
        copy.digest.clear();
        Ok(digest_bytes(&serde_json::to_vec(&copy)?))
    }
    pub fn validate(&self) -> Result<(), ContextError> {
        let mut ids = BTreeSet::new();
        if self.version != 1
            || self.generation == 0
            || self.digest != self.computed_digest()?
            || self.accounting.stored_bytes != self.rendered.len() as u64
            || self.accounting.image_tokens != 0
            || self.accounting.text_tokens > self.budget.available()?
            || self.cues.len() + self.omitted.len() > 256
            || self.rendered != render(&self.cues)?
            || self.visual
                != if self.profile.request_visual {
                    VisualAvailability::UnavailableTextFallback
                } else {
                    VisualAvailability::NotRequested
                }
        {
            return Err(ContextError::Stale);
        }
        for cue in &self.cues {
            if !ids.insert(&cue.id)
                || cue.text.len() > self.profile.max_cue_bytes
                || cue.content_digest.is_empty()
                || cue.revision_digest.is_empty()
            {
                return Err(ContextError::Stale);
            }
        }
        Ok(())
    }
}
fn render(cues: &[CompactCue]) -> Result<Vec<u8>, ContextError> {
    Ok(serde_json::to_vec(
        &cues
            .iter()
            .map(|c| (&c.id, c.authority, &c.text, c.truncated))
            .collect::<Vec<_>>(),
    )?)
}
pub fn build_cues(
    snapshot: &CueSnapshot,
    profile: CueProfile,
    budget: TokenBudget,
    generation: u64,
    counter: &dyn TokenCounter,
) -> Result<CuePlan, ContextError> {
    validate_id(&profile.model_id)?;
    if profile.max_cue_bytes == 0 || profile.max_cue_bytes > 4096 || generation == 0 {
        return Err(ContextError::Capacity);
    }
    let snapshot_digest = snapshot.digest()?;
    let available = budget.available()?;
    let mut cues = Vec::new();
    let mut omitted = Vec::new();
    let mut input_record_tokens = 0u64;
    for record in &snapshot.records {
        input_record_tokens = input_record_tokens
            .checked_add(counter.count(record.content.as_bytes())?)
            .ok_or(ContextError::Capacity)?;
        let mut end = record.content.len().min(profile.max_cue_bytes);
        while !record.content.is_char_boundary(end) {
            end -= 1;
        }
        let text = record.content[..end].to_owned();
        if text.is_empty() {
            omitted.push(record.id.clone());
            continue;
        }
        let id = digest_bytes(&serde_json::to_vec(&(
            &snapshot_digest,
            &profile,
            budget,
            generation,
            &record.id,
            &record.revision_digest,
        ))?);
        let cue = CompactCue {
            id,
            record_id: record.id.clone(),
            revision_digest: record.revision_digest.clone(),
            content_digest: digest_bytes(record.content.as_bytes()),
            authority: record.authority,
            text,
            truncated: end < record.content.len(),
        };
        cues.push(cue);
        if counter.count(&render(&cues)?)? > available {
            let cue = cues.pop().ok_or(ContextError::Conflict)?;
            omitted.push(cue.record_id);
        }
    }
    let rendered = render(&cues)?;
    let text_tokens = counter.count(&rendered)?;
    if text_tokens > available {
        return Err(ContextError::Capacity);
    }
    let visual = if profile.request_visual {
        VisualAvailability::UnavailableTextFallback
    } else {
        VisualAvailability::NotRequested
    };
    let mut plan = CuePlan {
        version: 1,
        generation,
        snapshot_digest,
        profile,
        budget,
        cues,
        omitted,
        visual,
        accounting: CueAccounting {
            input_record_tokens,
            text_tokens,
            image_tokens: 0,
            stored_bytes: rendered.len() as u64,
        },
        rendered,
        digest: String::new(),
    };
    plan.digest = plan.computed_digest()?;
    plan.validate()?;
    Ok(plan)
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CueExpansion {
    pub cue_id: String,
    pub generation: u64,
    pub record: CueRecord,
    pub text_tokens: u64,
}
pub fn resolve_cue(
    snapshot: &CueSnapshot,
    plan: &CuePlan,
    id: &str,
    counter: &dyn TokenCounter,
    budget: TokenBudget,
) -> Result<CueExpansion, ContextError> {
    plan.validate()?;
    if snapshot.digest()? != plan.snapshot_digest {
        return Err(ContextError::Stale);
    }
    let cue = plan
        .cues
        .iter()
        .find(|c| c.id == id)
        .ok_or(ContextError::Stale)?;
    let record = snapshot
        .records
        .iter()
        .find(|r| r.id == cue.record_id)
        .ok_or(ContextError::Stale)?;
    if record.revision_digest != cue.revision_digest
        || digest_bytes(record.content.as_bytes()) != cue.content_digest
        || record.authority != cue.authority
    {
        return Err(ContextError::Stale);
    }
    let text_tokens = counter.count(&serde_json::to_vec(record)?)?;
    if text_tokens > budget.available()? {
        return Err(ContextError::Capacity);
    }
    Ok(CueExpansion {
        cue_id: id.into(),
        generation: plan.generation,
        record: record.clone(),
        text_tokens,
    })
}
