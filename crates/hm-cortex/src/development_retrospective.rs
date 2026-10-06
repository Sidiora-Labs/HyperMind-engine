use hm_context::{
    development::*, digest_bytes, maintenance::Usage, validate_id, Authority, ContextError,
};
use hm_llm::{LlmProvider, StructuredRequest};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

pub const LESSON_CATEGORY: &str = "retrospective_lesson";
pub const CHECKPOINT_CATEGORY: &str = "retrospective_checkpoint";
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetrospectiveWatermark {
    pub version: u32,
    pub successful_corrections: BTreeMap<String, String>,
}
impl Default for RetrospectiveWatermark {
    fn default() -> Self {
        Self {
            version: 1,
            successful_corrections: BTreeMap::new(),
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectionSignal {
    pub relation_id: String,
    pub source_id: String,
    pub source_digest: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignalBatch {
    pub evidence: EvidenceSnapshot,
    pub signals: Vec<CorrectionSignal>,
    pub watermark: RetrospectiveWatermark,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case", deny_unknown_fields)]
pub enum RetrospectiveProposal {
    NoSignal {
        watermark: RetrospectiveWatermark,
        provider_calls: u32,
    },
    Proposed {
        plan: DevelopmentPlan,
        watermark: RetrospectiveWatermark,
        provider_calls: u32,
    },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GeneratedLessons {
    lessons: Vec<GeneratedLesson>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GeneratedLesson {
    source_id: String,
    quote: String,
    lesson: String,
}
fn invalid(message: &str) -> ContextError {
    ContextError::Invalid(message.into())
}

pub fn detect_new_corrections(
    evidence: EvidenceSnapshot,
    signals: Vec<CorrectionSignal>,
    watermark: RetrospectiveWatermark,
) -> Result<SignalBatch, ContextError> {
    if watermark.version != 1 || evidence.digest != evidence.computed_digest()? {
        return Err(ContextError::Stale);
    }
    let mut seen = BTreeSet::new();
    let mut selected = Vec::new();
    for signal in signals {
        validate_id(&signal.relation_id)?;
        validate_id(&signal.source_id)?;
        let source = evidence
            .sources
            .iter()
            .find(|s| s.id == signal.source_id)
            .ok_or(ContextError::ScopeMismatch)?;
        if source.origin != EvidenceOrigin::Conversation
            || source.authority != Authority::UserAsserted
            || source.digest != signal.source_digest
            || source.content.is_empty()
            || !seen.insert(signal.source_id.clone())
        {
            return Err(ContextError::ScopeMismatch);
        }
        if let Some(digest) = watermark.successful_corrections.get(&signal.source_id) {
            if digest != &signal.source_digest {
                return Err(ContextError::Conflict);
            }
        } else {
            selected.push(signal);
        }
    }
    selected.sort_by(|a, b| a.source_id.cmp(&b.source_id));
    Ok(SignalBatch {
        evidence,
        signals: selected,
        watermark,
    })
}
fn lesson_key(text: &str) -> String {
    digest_bytes(
        text.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase()
            .as_bytes(),
    )
}
fn record(
    id: String,
    category: &str,
    content: String,
    recorded_at_ns: i64,
    provenance: Vec<DevelopmentProvenance>,
) -> DevelopmentRecord {
    DevelopmentRecord {
        id,
        kind: DevelopmentRecordKind::Note,
        category: category.into(),
        status: DevelopmentRecordStatus::Active,
        revision: 1,
        revision_digest: String::new(),
        content,
        authority: Authority::DerivedInference,
        confidence: 500_000,
        importance: 500_000,
        occurred_at_ns: None,
        recorded_at_ns,
        expires_at_ns: None,
        pinned: false,
        provenance,
        lineage: vec![],
        contradictions: vec![],
        last_lsn: 0,
        predicate: None,
        smart_condition: None,
        retention_until_ns: None,
        metadata: json!({}),
    }
}
pub fn propose_lessons(
    batch: SignalBatch,
    provider: &dyn LlmProvider,
    plan_id: &str,
    checkpoint_id: &str,
    lesson_ids: &[String],
) -> Result<RetrospectiveProposal, ContextError> {
    if batch.signals.is_empty() {
        return Ok(RetrospectiveProposal::NoSignal {
            watermark: batch.watermark,
            provider_calls: 0,
        });
    }
    validate_id(plan_id)?;
    validate_id(checkpoint_id)?;
    let mut unique_ids = BTreeSet::new();
    for id in lesson_ids {
        validate_id(id)?;
        if id == checkpoint_id
            || !unique_ids.insert(id)
            || batch.evidence.records.iter().any(|r| &r.id == id)
        {
            return Err(ContextError::Conflict);
        }
    }
    let input = batch
        .signals
        .iter()
        .map(|signal| {
            let source = batch
                .evidence
                .sources
                .iter()
                .find(|s| s.id == signal.source_id)
                .ok_or(ContextError::ScopeMismatch)?;
            let text = std::str::from_utf8(&source.content)
                .map_err(|_| invalid("correction source is not UTF-8 text"))?;
            Ok(json!({"source_id":source.id,"correction":text}))
        })
        .collect::<Result<Vec<_>, ContextError>>()?;
    let maximum = lesson_ids
        .len()
        .min(batch.evidence.budget.max_mutations.saturating_sub(1));
    if maximum == 0 || batch.signals.len() > maximum {
        return Err(ContextError::Capacity);
    }
    let schema = json!({"type":"object","additionalProperties":false,"required":["lessons"],"properties":{"lessons":{"type":"array","minItems":1,"maxItems":maximum,"items":{"type":"object","additionalProperties":false,"required":["source_id","quote","lesson"],"properties":{"source_id":{"type":"string","enum":batch.signals.iter().map(|s|s.source_id.clone()).collect::<Vec<_>>()},"quote":{"type":"string","enum":input.iter().map(|value|value["correction"].clone()).collect::<Vec<_>>()},"lesson":{"type":"string","minLength":1}}}}}});
    let request = StructuredRequest {
        prompt_id:"hypermind-retrospective-v1".into(),
        system:"Produce procedural lessons from explicit user corrections. Each lesson must be supported by an exact contiguous quote from its correction. Describe how to perform the corrected work in future. Do not infer user preferences, personality, profile attributes or unrelated facts. Return only the requested JSON.".into(),
        prompt:serde_json::to_string(&input)?,json_schema:schema,
        maximum_output_tokens:u32::try_from(batch.evidence.budget.reserved_tokens.min(2048)).map_err(|_|ContextError::Capacity)?,
    };
    if request.prompt.len() as u64 > batch.evidence.budget.max_input_bytes {
        return Err(ContextError::Capacity);
    }
    let response = provider
        .generate_structured(&request)
        .map_err(|e| ContextError::Unavailable(format!("retrospective inference: {e:?}")))?;
    if serde_json::to_vec(&response.value)?.len() as u64 > batch.evidence.budget.max_output_bytes {
        return Err(ContextError::Capacity);
    }
    let generated: GeneratedLessons = serde_json::from_value(response.value)?;
    if generated.lessons.is_empty() || generated.lessons.len() > maximum {
        return Err(invalid("invalid lesson count"));
    }
    let used = response
        .usage
        .input_tokens
        .checked_add(response.usage.output_tokens)
        .ok_or(ContextError::Capacity)?;
    if used > batch.evidence.budget.reserved_tokens {
        return Err(ContextError::Capacity);
    }
    let recorded_at_ns = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| ContextError::Unavailable("recording clock".into()))?
            .as_nanos(),
    )
    .map_err(|_| ContextError::Capacity)?;
    let mut keys: BTreeSet<String> = batch
        .evidence
        .records
        .iter()
        .filter(|r| r.category == LESSON_CATEGORY)
        .map(|r| lesson_key(&r.content))
        .collect();
    let mut covered = BTreeSet::new();
    let mut mutations = Vec::new();
    let mut ids = lesson_ids.iter();
    for lesson in generated.lessons {
        if lesson.lesson.trim().is_empty() || lesson.lesson.len() > 16384 || lesson.quote.is_empty()
        {
            return Err(invalid("invalid lesson text"));
        }
        if !batch
            .signals
            .iter()
            .any(|s| s.source_id == lesson.source_id)
        {
            return Err(ContextError::ScopeMismatch);
        }
        let source = batch
            .evidence
            .sources
            .iter()
            .find(|s| s.id == lesson.source_id)
            .ok_or(ContextError::ScopeMismatch)?;
        let text =
            std::str::from_utf8(&source.content).map_err(|_| invalid("invalid correction text"))?;
        let start = text
            .find(&lesson.quote)
            .ok_or_else(|| invalid("lesson quote does not match original source"))?;
        covered.insert(lesson.source_id.clone());
        let key = lesson_key(&lesson.lesson);
        if !keys.insert(key.clone()) {
            continue;
        }
        let provenance = DevelopmentProvenance {
            source_id: source.id.clone(),
            source_digest: source.content_digest.clone(),
            span_start: start as u64,
            span_end: (start + lesson.quote.len()) as u64,
            quoted_digest: digest_bytes(lesson.quote.as_bytes()),
        };
        let mut lesson_record = record(
            ids.next().ok_or(ContextError::Capacity)?.clone(),
            LESSON_CATEGORY,
            lesson.lesson.trim().into(),
            recorded_at_ns,
            vec![provenance],
        );
        lesson_record.metadata = json!({"version":1,"lesson_key":key,"review_status":"proposed","correction_source_id":source.id});
        mutations.push(PlannedKnowledgeMutation::Create {
            record: lesson_record,
        });
    }
    if batch
        .signals
        .iter()
        .any(|signal| !covered.contains(&signal.source_id))
    {
        return Err(invalid("lesson output does not cover every new correction"));
    }
    let mut watermark = batch.watermark;
    let mut provenance = Vec::new();
    for signal in &batch.signals {
        watermark
            .successful_corrections
            .insert(signal.source_id.clone(), signal.source_digest.clone());
        let source = batch
            .evidence
            .sources
            .iter()
            .find(|s| s.id == signal.source_id)
            .ok_or(ContextError::ScopeMismatch)?;
        provenance.push(DevelopmentProvenance {
            source_id: source.id.clone(),
            source_digest: source.content_digest.clone(),
            span_start: 0,
            span_end: source.content.len() as u64,
            quoted_digest: source.content_digest.clone(),
        });
    }
    if watermark.successful_corrections.len() > 4096 {
        return Err(ContextError::Capacity);
    }
    let mut checkpoint = record(
        checkpoint_id.into(),
        CHECKPOINT_CATEGORY,
        "Successful retrospective correction coverage".into(),
        recorded_at_ns,
        provenance,
    );
    checkpoint.status = DevelopmentRecordStatus::Archived;
    checkpoint.metadata = serde_json::to_value(&watermark)?;
    if let Some(old) = batch
        .evidence
        .records
        .iter()
        .find(|r| r.id == checkpoint_id)
    {
        if old.category != CHECKPOINT_CATEGORY || old.kind != DevelopmentRecordKind::Note {
            return Err(ContextError::Conflict);
        }
        checkpoint.revision = old.revision.checked_add(1).ok_or(ContextError::Capacity)?;
        mutations.push(PlannedKnowledgeMutation::Revise {
            record: checkpoint,
            expected_revision: old.revision,
            expected_digest: old.revision_digest.clone(),
        });
    } else {
        mutations.push(PlannedKnowledgeMutation::Create { record: checkpoint });
    }
    Ok(RetrospectiveProposal::Proposed {
        plan: DevelopmentPlan {
            version: 1,
            id: plan_id.into(),
            kind: DevelopmentKind::Retrospective,
            evidence: batch.evidence,
            mutations,
            proposal: None,
            usage: Usage::Known(used),
        },
        watermark,
        provider_calls: 1,
    })
}
