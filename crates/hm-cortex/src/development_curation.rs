use hm_context::{Authority, development::*, digest_bytes, maintenance::Usage};
use hm_llm::{LlmProvider, StructuredRequest};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeSet;

#[derive(Debug)]
pub enum CurationError {
    Invalid(String),
    Provider(hm_llm::LlmError),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurationRequest {
    pub plan_id: String,
    pub actions: Vec<CurationAction>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum CurationAction {
    Reword {
        id: String,
    },
    Merge {
        id: String,
        parents: Vec<String>,
        archive_parents: bool,
    },
    Archive {
        id: String,
        duplicate_of: String,
    },
    Classify {
        id: String,
    },
}
pub type ClassificationPlan = DevelopmentPlan;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Answer {
    content: String,
    importance: u32,
    category: String,
    shareability: String,
}
fn invalid(s: &str) -> CurationError {
    CurationError::Invalid(s.into())
}
fn protected(r: &DevelopmentRecord) -> bool {
    r.pinned || r.kind == DevelopmentRecordKind::Anchor || !r.contradictions.is_empty()
}

pub fn curate(
    snapshot: EvidenceSnapshot,
    provider: &dyn LlmProvider,
    request: CurationRequest,
) -> Result<DevelopmentPlan, CurationError> {
    if request.plan_id.is_empty()
        || snapshot
            .computed_digest()
            .map_err(|_| invalid("snapshot"))?
            != snapshot.digest
        || request.actions.is_empty()
        || request.actions.len() > snapshot.budget.max_mutations as usize
    {
        return Err(invalid("invalid request or snapshot"));
    }
    let mut mutations = Vec::new();
    let mut touched = BTreeSet::new();
    let mut tokens = 0u64;
    for action in request.actions {
        let (id, parents, mode, archive) = match action {
            CurationAction::Reword { id } => (id.clone(), vec![id], "reword", false),
            CurationAction::Classify { id } => (id.clone(), vec![id], "classify", false),
            CurationAction::Merge {
                id,
                parents,
                archive_parents,
            } => (id, parents, "merge", archive_parents),
            CurationAction::Archive { id, duplicate_of } => {
                (id.clone(), vec![id, duplicate_of], "archive", false)
            }
        };
        if parents.is_empty()
            || parents.len() > 32
            || parents.iter().collect::<BTreeSet<_>>().len() != parents.len()
        {
            return Err(invalid("invalid parent set"));
        }
        let records = parents
            .iter()
            .map(|p| {
                snapshot
                    .records
                    .iter()
                    .find(|r| &r.id == p)
                    .ok_or_else(|| invalid("record outside snapshot"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if mode != "merge" && protected(records[0]) {
            return Err(invalid("protected record"));
        }
        if mode == "archive" {
            if records[0].content != records[1].content
                || records[0].provenance != records[1].provenance
            {
                return Err(invalid("archive requires exact grounded duplicate"));
            }
            let r = records[0];
            mutations.push(PlannedKnowledgeMutation::SetStatus {
                id: r.id.clone(),
                status: DevelopmentRecordStatus::Archived,
                expected_revision: r.revision,
                expected_digest: r.revision_digest.clone(),
            });
            continue;
        }
        if !touched.insert(id.clone()) {
            return Err(invalid("duplicate target"));
        }
        let sources = snapshot
            .sources
            .iter()
            .map(|s| String::from_utf8(s.content.clone()).map_err(|_| invalid("non-text evidence")))
            .collect::<Result<Vec<_>, _>>()?;
        let prompt = serde_json::to_string(
            &json!({"operation":mode,"records":records,"original_sources":sources}),
        )
        .map_err(|_| invalid("serialization"))?;
        if prompt.len() as u64 > snapshot.budget.max_input_bytes {
            return Err(invalid("input budget"));
        }
        let response=provider.generate_structured(&StructuredRequest {
            prompt_id:"development-curation@1".into(), system:"Return only the schema JSON. Ground every claim in the supplied original sources. Preserve numbers, quotations, uncertainty and contradictory alternatives. For classify copy the content exactly. For merge consolidate duplicate wording without losing evidence. Shareability is only a recommendation: private, owner_review or restricted; it grants no access.".into(), prompt,
            json_schema:json!({"type":"object","additionalProperties":false,"required":["content","importance","category","shareability"],"properties":{"content":{"type":"string"},"importance":{"type":"integer","minimum":0,"maximum":1000000},"category":{"type":"string"},"shareability":{"type":"string","enum":["private","owner_review","restricted"]}}}), maximum_output_tokens:512,
        }).map_err(CurationError::Provider)?;
        tokens = tokens
            .saturating_add(response.usage.input_tokens)
            .saturating_add(response.usage.output_tokens);
        if tokens > snapshot.budget.reserved_tokens
            || serde_json::to_vec(&response.value)
                .map_err(|_| invalid("output"))?
                .len() as u64
                > snapshot.budget.max_output_bytes
        {
            return Err(invalid("output budget"));
        }
        let answer: Answer = serde_json::from_value(response.value)
            .map_err(|_| invalid("malformed classification"))?;
        if answer.importance > 1_000_000
            || answer.category.is_empty()
            || answer.category.len() > 128
            || !["private", "owner_review", "restricted"].contains(&answer.shareability.as_str())
        {
            return Err(invalid("malformed policy"));
        }
        let evidence = sources.iter().map(String::as_str).collect::<Vec<_>>();
        if mode == "classify" && answer.content != records[0].content {
            return Err(invalid("classification changed content"));
        }
        if mode != "classify"
            && (!crate::quality::assess_thought(&answer.content, &evidence, Default::default())
                .accepted
                || records.iter().any(|r| {
                    !crate::quality::check_rewrite("", &r.content, &answer.content, &evidence)
                        .accepted
                }))
        {
            return Err(invalid("rewrite grounding guard"));
        }
        let mut record = records[0].clone();
        record.id = id.clone();
        record.revision = if mode == "merge" {
            1
        } else {
            record.revision + 1
        };
        record.revision_digest.clear();
        record.authority = Authority::DerivedInference;
        record.content = answer.content;
        record.importance = answer.importance;
        let classification = json!({"classification":{"content_digest":digest_bytes(record.content.as_bytes()),"importance":answer.importance,"topical_scope":answer.category,"shareability":if mode=="classify" {answer.shareability} else {"private".into()},"requires_owner_grant":true}});
        let mut metadata = record.metadata.as_object().cloned().unwrap_or_default();
        metadata.remove("shareability");
        metadata.remove("sharing");
        metadata.remove("classification");
        metadata.insert(
            "classification".into(),
            classification["classification"].clone(),
        );
        record.metadata = serde_json::Value::Object(metadata);
        if mode == "merge" {
            if snapshot.records.iter().any(|r| r.id == id) {
                return Err(invalid("merge identity exists"));
            }
            record.kind = DevelopmentRecordKind::Note;
            record.pinned = false;
            record.status = DevelopmentRecordStatus::Active;
            record.provenance = records.iter().flat_map(|r| r.provenance.clone()).collect();
            record
                .provenance
                .sort_by_key(|p| (p.source_id.clone(), p.span_start, p.span_end));
            record.provenance.dedup();
            record.contradictions = records
                .iter()
                .flat_map(|r| r.contradictions.clone())
                .collect();
            record.contradictions.sort();
            record.contradictions.dedup();
            record.lineage = records.iter().flat_map(|r| r.lineage.clone()).collect();
            for r in &records {
                record.lineage.push(DevelopmentLineage {
                    child_record_id: id.clone(),
                    child_revision: 1,
                    parent_record_id: r.id.clone(),
                    parent_revision_digest: r.revision_digest.clone(),
                    relation: "merged_from".into(),
                    created_at_ns: r.recorded_at_ns,
                });
            }
            mutations.push(PlannedKnowledgeMutation::Create { record });
            if archive {
                for r in records {
                    if !protected(r) {
                        mutations.push(PlannedKnowledgeMutation::SetStatus {
                            id: r.id.clone(),
                            status: DevelopmentRecordStatus::Archived,
                            expected_revision: r.revision,
                            expected_digest: r.revision_digest.clone(),
                        });
                    }
                }
            }
        } else {
            mutations.push(PlannedKnowledgeMutation::Revise {
                record,
                expected_revision: records[0].revision,
                expected_digest: records[0].revision_digest.clone(),
            });
        }
    }
    if mutations.len() > snapshot.budget.max_mutations as usize {
        return Err(invalid("mutation budget"));
    }
    Ok(DevelopmentPlan {
        version: 1,
        id: request.plan_id,
        kind: DevelopmentKind::Curation,
        evidence: snapshot,
        mutations,
        proposal: None,
        usage: Usage::Known(tokens),
    })
}
pub fn classify(
    snapshot: EvidenceSnapshot,
    provider: &dyn LlmProvider,
    request: CurationRequest,
) -> Result<ClassificationPlan, CurationError> {
    if request
        .actions
        .iter()
        .any(|a| !matches!(a, CurationAction::Classify { .. }))
    {
        return Err(invalid("classification actions required"));
    }
    curate(snapshot, provider, request)
}
