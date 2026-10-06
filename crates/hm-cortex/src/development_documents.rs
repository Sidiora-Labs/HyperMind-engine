use hm_context::{
    development::*, digest_bytes, maintenance::Usage, validate_id, Authority, ContextError,
};
use hm_llm::{LlmProvider, StructuredRequest};
use serde::{Deserialize, Serialize};
use serde_json::json;

pub const DOCUMENT_PATCH_CATEGORY: &str = "documentation_patch";
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepositoryDelta {
    pub version: u32,
    pub repository_digest: String,
    pub source_path: String,
    pub before_source: Vec<u8>,
    pub before_source_digest: String,
    pub after_source: Vec<u8>,
    pub after_source_digest: String,
    pub target_path: String,
    pub target_bytes: Vec<u8>,
    pub target_digest: String,
}
impl RepositoryDelta {
    pub fn validate(&self) -> Result<(), ContextError> {
        if self.version != 1
            || self.before_source == self.after_source
            || self.before_source_digest != digest_bytes(&self.before_source)
            || self.after_source_digest != digest_bytes(&self.after_source)
            || self.target_digest != digest_bytes(&self.target_bytes)
            || self.repository_digest.len() != 64
        {
            return Err(ContextError::Invalid("invalid repository delta".into()));
        }
        for path in [&self.source_path, &self.target_path] {
            validate_relative_path(path)?;
        }
        if self.source_path == self.target_path
            || self.before_source.len() + self.after_source.len() + self.target_bytes.len()
                > 1024 * 1024
        {
            return Err(ContextError::Capacity);
        }
        Ok(())
    }
    pub fn evidence_bytes(&self) -> Result<Vec<u8>, ContextError> {
        self.validate()?;
        Ok(serde_json::to_vec(self)?)
    }
    pub fn digest(&self) -> Result<String, ContextError> {
        Ok(digest_bytes(&self.evidence_bytes()?))
    }
}
pub fn validate_relative_path(value: &str) -> Result<(), ContextError> {
    let path = std::path::Path::new(value);
    if value.is_empty()
        || value.len() > 4096
        || path.is_absolute()
        || path
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err(ContextError::Invalid(
            "invalid repository relative path".into(),
        ));
    }
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| ContextError::Invalid("non-UTF-8 filename".into()))?;
    if name == "AGENTS.md"
        || name == "GOTCHA.kvx"
        || path.components().any(|c| c.as_os_str() == ".git")
    {
        return Err(ContextError::Invalid("protected repository file".into()));
    }
    Ok(())
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentationPatch {
    pub version: u32,
    pub delta: RepositoryDelta,
    pub source_id: String,
    pub source_digest: String,
    pub quote: String,
    pub replacement: Vec<u8>,
    pub replacement_digest: String,
    pub digest: String,
}
impl DocumentationPatch {
    pub fn computed_digest(&self) -> Result<String, ContextError> {
        let mut copy = self.clone();
        copy.digest.clear();
        Ok(digest_bytes(&serde_json::to_vec(&copy)?))
    }
    pub fn validate(&self) -> Result<(), ContextError> {
        self.delta.validate()?;
        validate_id(&self.source_id)?;
        let text = std::str::from_utf8(&self.delta.after_source)
            .map_err(|_| ContextError::Invalid("source is not text".into()))?;
        if self.version != 1
            || self.quote.is_empty()
            || !text.contains(&self.quote)
            || self.source_digest != digest_bytes(&self.delta.evidence_bytes()?)
            || self.replacement.is_empty()
            || self.replacement == self.delta.target_bytes
            || self.replacement.len() > 512 * 1024
            || self.replacement_digest != digest_bytes(&self.replacement)
            || self.digest != self.computed_digest()?
        {
            return Err(ContextError::Invalid("invalid documentation patch".into()));
        }
        std::str::from_utf8(&self.replacement)
            .map_err(|_| ContextError::Invalid("documentation is not UTF-8".into()))?;
        Ok(())
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GeneratedDocumentation {
    quote: String,
    replacement: String,
}
pub fn propose_docs(
    evidence: EvidenceSnapshot,
    delta: RepositoryDelta,
    provider: &dyn LlmProvider,
    plan_id: &str,
    proposal_id: &str,
    patch_record_id: &str,
    source_id: &str,
) -> Result<DevelopmentPlan, ContextError> {
    delta.validate()?;
    for id in [plan_id, proposal_id, patch_record_id, source_id] {
        validate_id(id)?;
    }
    if evidence.digest != evidence.computed_digest()? {
        return Err(ContextError::Stale);
    }
    let bytes = delta.evidence_bytes()?;
    let source = evidence
        .sources
        .iter()
        .find(|s| {
            s.id == source_id && s.content == bytes && s.content_digest == digest_bytes(&bytes)
        })
        .ok_or(ContextError::ScopeMismatch)?;
    let old = std::str::from_utf8(&delta.before_source)
        .map_err(|_| ContextError::Invalid("source is not text".into()))?;
    let current = std::str::from_utf8(&delta.after_source)
        .map_err(|_| ContextError::Invalid("source is not text".into()))?;
    let document = std::str::from_utf8(&delta.target_bytes)
        .map_err(|_| ContextError::Invalid("target is not text".into()))?;
    let prompt=json!({"source_path":delta.source_path,"source_before":old,"source_after":current,"target_path":delta.target_path,"existing_documentation":document}).to_string();
    if prompt.len() as u64 > evidence.budget.max_input_bytes {
        return Err(ContextError::Capacity);
    }
    let response=provider.generate_structured(&StructuredRequest {prompt_id:"hypermind-documentation-v1".into(),system:"Update the existing product documentation to match the actual code change. Preserve unrelated material. Do not introduce assistant instructions, personal information, operational secrets or unsupported behavior. Return the complete replacement document and an exact supporting quote from source_after, only as the requested JSON.".into(),prompt,json_schema:json!({"type":"object","additionalProperties":false,"required":["quote","replacement"],"properties":{"quote":{"type":"string","enum":[current]},"replacement":{"type":"string","minLength":1}}}),maximum_output_tokens:u32::try_from(evidence.budget.reserved_tokens.min(4096)).map_err(|_|ContextError::Capacity)?}).map_err(|e|ContextError::Unavailable(format!("documentation inference: {e:?}")))?;
    let used = response
        .usage
        .input_tokens
        .checked_add(response.usage.output_tokens)
        .ok_or(ContextError::Capacity)?;
    if used > evidence.budget.reserved_tokens
        || serde_json::to_vec(&response.value)?.len() as u64 > evidence.budget.max_output_bytes
    {
        return Err(ContextError::Capacity);
    }
    let output: GeneratedDocumentation = serde_json::from_value(response.value)?;
    let mut patch = DocumentationPatch {
        version: 1,
        delta,
        source_id: source_id.into(),
        source_digest: source.content_digest.clone(),
        quote: output.quote,
        replacement: output.replacement.into_bytes(),
        replacement_digest: String::new(),
        digest: String::new(),
    };
    patch.replacement_digest = digest_bytes(&patch.replacement);
    patch.digest = patch.computed_digest()?;
    patch.validate()?;
    let recorded_at_ns = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| ContextError::Unavailable("recording clock".into()))?
            .as_nanos(),
    )
    .map_err(|_| ContextError::Capacity)?;
    let record = DevelopmentRecord {
        id: patch_record_id.into(),
        kind: DevelopmentRecordKind::Note,
        category: DOCUMENT_PATCH_CATEGORY.into(),
        status: DevelopmentRecordStatus::Archived,
        revision: 1,
        revision_digest: String::new(),
        content: format!("Documentation patch for {}", patch.delta.target_path),
        authority: Authority::DerivedInference,
        confidence: 500_000,
        importance: 500_000,
        occurred_at_ns: None,
        recorded_at_ns,
        expires_at_ns: None,
        pinned: false,
        provenance: vec![DevelopmentProvenance {
            source_id: source.id.clone(),
            source_digest: source.content_digest.clone(),
            span_start: 0,
            span_end: source.content.len() as u64,
            quoted_digest: source.content_digest.clone(),
        }],
        lineage: vec![],
        contradictions: vec![],
        last_lsn: 0,
        predicate: None,
        smart_condition: None,
        retention_until_ns: None,
        metadata: serde_json::to_value(&patch)?,
    };
    let mutations = vec![PlannedKnowledgeMutation::Create { record }];
    let mut proposal = ApprovalProposal {
        id: proposal_id.into(),
        revision: 1,
        kind: DevelopmentKind::DocumentationProposal,
        scope: evidence.scope.clone(),
        worker_id: evidence.worker_id.clone(),
        evidence: evidence.clone(),
        mutations,
        status: ProposalStatus::Pending,
        digest: String::new(),
    };
    proposal.digest = proposal.computed_digest()?;
    let plan = DevelopmentPlan {
        version: 1,
        id: plan_id.into(),
        kind: DevelopmentKind::DocumentationProposal,
        evidence,
        mutations: vec![],
        proposal: Some(proposal),
        usage: Usage::Known(used),
    };
    if serde_json::to_vec(&plan)?.len() as u64 > plan.evidence.budget.max_output_bytes {
        return Err(ContextError::Capacity);
    }
    Ok(plan)
}
