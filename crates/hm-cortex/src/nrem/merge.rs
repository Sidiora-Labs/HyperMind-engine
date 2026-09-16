#![allow(clippy::missing_errors_doc)]

use crate::citations::{CitationClaim, CitationError, FrozenCandidateSet, SourceKind};
use crate::nrem::cluster::ObservationCluster;
use crate::quality::{
    RewriteGuardResult, ThoughtQualityOptions, ThoughtQualityResult, assess_thought, check_rewrite,
};
use hm_llm::cost::RunCost;
use hm_llm::{LlmError, LlmProvider, StructuredRequest};
use hm_schema::events::{Authority, ModelProvenance, ProvenanceRange};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fmt::Write;

pub const MERGE_CLUSTER_PROMPT: &str = include_str!("../../../../prompts/merge-cluster@1.md");

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExistingMemory {
    pub memory_id: Vec<u8>,
    pub name: String,
    pub definition: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MergeAction {
    Attach,
    Revise,
    Mint,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NremDecision {
    pub cluster_id: [u8; 32],
    pub action: MergeAction,
    pub target_id: Option<Vec<u8>>,
    pub name: String,
    pub definition: Vec<u8>,
    pub tags: Vec<String>,
    pub salience_micros: u32,
    pub citations: Vec<ProvenanceRange>,
    pub authority: Authority,
    pub model_provenance: ModelProvenance,
    pub thought_quality: ThoughtQualityResult,
    pub rewrite_guard: Option<RewriteGuardResult>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DropReason {
    InvalidStructuredOutput,
    Citation(CitationError),
    InsufficientIndependentRoots,
    InsufficientConversations,
    SpeculationPoisoned,
    ThoughtQuality(Vec<String>),
    RewriteGuard(Vec<String>),
    MissingTarget,
    InvalidCandidateEncoding,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DroppedCandidate {
    pub cluster_id: [u8; 32],
    pub reason: DropReason,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NremReport {
    pub decisions: Vec<NremDecision>,
    pub dropped: Vec<DroppedCandidate>,
    pub citation_invalid: u64,
    pub llm_calls: u64,
    pub cost: RunCost,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NremError {
    Llm(LlmError),
    Cost(LlmError),
}

impl From<LlmError> for NremError {
    fn from(value: LlmError) -> Self {
        Self::Llm(value)
    }
}

pub fn consolidate_clusters(
    provider: &dyn LlmProvider,
    run_id: &[u8],
    clusters: &[ObservationCluster],
    existing: &[ExistingMemory],
) -> Result<NremReport, NremError> {
    let mut report = NremReport::default();
    for cluster in clusters {
        let request = match merge_request(cluster, existing) {
            Ok(request) => request,
            Err(reason) => {
                report.dropped.push(DroppedCandidate {
                    cluster_id: cluster.cluster_id,
                    reason,
                });
                continue;
            }
        };
        let response = provider.generate_structured(&request)?;
        report.llm_calls = report.llm_calls.saturating_add(1);
        report
            .cost
            .record(response.usage)
            .map_err(NremError::Cost)?;
        match validate_response(
            run_id,
            cluster,
            existing,
            &response.model_id,
            response.usage,
            &response.value,
        ) {
            Ok(decision) => report.decisions.push(decision),
            Err(reason) => {
                if matches!(reason, DropReason::Citation(_)) {
                    report.citation_invalid = report.citation_invalid.saturating_add(1);
                }
                report.dropped.push(DroppedCandidate {
                    cluster_id: cluster.cluster_id,
                    reason,
                });
            }
        }
    }
    Ok(report)
}

pub fn merge_request(
    cluster: &ObservationCluster,
    existing: &[ExistingMemory],
) -> Result<StructuredRequest, DropReason> {
    let mut prompt = String::from("FROZEN OBSERVATIONS\n");
    for observation in &cluster.observations {
        let content = std::str::from_utf8(&observation.source.content)
            .map_err(|_| DropReason::InvalidCandidateEncoding)?;
        let _ = write!(
            prompt,
            "lsn={} conversation={} root={} kind={:?} bytes={}\n{}\n---\n",
            observation.source.lsn,
            hex(&observation.source.conversation),
            hex(&observation.source.source_root),
            observation.source.kind,
            observation.source.content.len(),
            content
        );
    }
    prompt.push_str("EXISTING MEMORIES\n");
    if existing.is_empty() {
        prompt.push_str("none\n");
    } else {
        for memory in existing {
            let definition = std::str::from_utf8(&memory.definition)
                .map_err(|_| DropReason::InvalidCandidateEncoding)?;
            let _ = writeln!(
                prompt,
                "target={} name={} definition={}",
                hex(&memory.memory_id),
                memory.name,
                definition
            );
        }
    }
    Ok(StructuredRequest {
        prompt_id: "merge-cluster@1".to_owned(),
        system: MERGE_CLUSTER_PROMPT.to_owned(),
        prompt,
        json_schema: response_schema(),
        maximum_output_tokens: 1_024,
    })
}

fn validate_response(
    run_id: &[u8],
    cluster: &ObservationCluster,
    existing: &[ExistingMemory],
    model_id: &str,
    usage: hm_llm::Usage,
    value: &Value,
) -> Result<NremDecision, DropReason> {
    let parsed = parse_response(value).ok_or(DropReason::InvalidStructuredOutput)?;
    let frozen = FrozenCandidateSet::new(
        cluster
            .observations
            .iter()
            .map(|observation| observation.source.clone())
            .collect(),
    )
    .map_err(DropReason::Citation)?;
    let validated = frozen
        .validate(parsed.definition.as_bytes(), &parsed.citations)
        .map_err(DropReason::Citation)?;
    let evidence = cluster
        .observations
        .iter()
        .map(|observation| std::str::from_utf8(&observation.source.content))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| DropReason::InvalidCandidateEncoding)?;
    let quality = assess_thought(
        &parsed.definition,
        &evidence,
        ThoughtQualityOptions::default(),
    );
    if !quality.accepted {
        return Err(DropReason::ThoughtQuality(quality.reasons));
    }
    let target = parsed.target.as_deref().and_then(|target| {
        existing
            .iter()
            .find(|memory| hex(&memory.memory_id) == target)
    });
    let rewrite_guard = match parsed.action {
        MergeAction::Attach => {
            if target.is_none() {
                return Err(DropReason::MissingTarget);
            }
            None
        }
        MergeAction::Revise => {
            let target = target.ok_or(DropReason::MissingTarget)?;
            let old = std::str::from_utf8(&target.definition)
                .map_err(|_| DropReason::InvalidCandidateEncoding)?;
            let guard = check_rewrite(&target.name, old, &parsed.definition, &evidence);
            if !guard.accepted {
                return Err(DropReason::RewriteGuard(guard.reasons));
            }
            Some(guard)
        }
        MergeAction::Mint => {
            validate_mint_independence(cluster, &parsed.citations)?;
            None
        }
    };
    let call_id = call_id(run_id, cluster.cluster_id, model_id);
    Ok(NremDecision {
        cluster_id: cluster.cluster_id,
        action: parsed.action,
        target_id: target.map(|memory| memory.memory_id.clone()),
        name: parsed.name,
        definition: parsed.definition.into_bytes(),
        tags: parsed.tags,
        salience_micros: parsed.salience_micros,
        citations: validated.ranges,
        authority: validated.authority,
        model_provenance: ModelProvenance {
            model_id: model_id.to_owned(),
            prompt_id: "merge-cluster".to_owned(),
            prompt_version: 1,
            temperature: 0.0,
            call_id: Some(call_id),
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            cache_read_tokens: usage.cache_read_tokens,
            cache_write_tokens: usage.cache_write_tokens,
            cost_microusd: usage.cost_microusd,
        },
        thought_quality: quality,
        rewrite_guard,
    })
}

struct ParsedResponse {
    action: MergeAction,
    target: Option<String>,
    name: String,
    definition: String,
    tags: Vec<String>,
    salience_micros: u32,
    citations: Vec<CitationClaim>,
}

fn parse_response(value: &Value) -> Option<ParsedResponse> {
    let action = match value.get("action")?.as_str()? {
        "attach" => MergeAction::Attach,
        "revise" => MergeAction::Revise,
        "mint" => MergeAction::Mint,
        _ => return None,
    };
    let target = value
        .get("target")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let name = value.get("name")?.as_str()?.trim().to_owned();
    let definition = value.get("definition")?.as_str()?.trim().to_owned();
    let tags = value
        .get("tags")?
        .as_array()?
        .iter()
        .map(Value::as_str)
        .collect::<Option<Vec<_>>>()?
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let salience_micros = u32::try_from(value.get("salience_micros")?.as_u64()?).ok()?;
    if name.is_empty()
        || definition.is_empty()
        || tags.is_empty()
        || tags.iter().any(String::is_empty)
        || salience_micros > 1_000_000
    {
        return None;
    }
    let citations = value
        .get("citations")?
        .as_array()?
        .iter()
        .map(|citation| {
            Some(CitationClaim {
                lsn: citation.get("lsn")?.as_u64()?,
                byte_start: u32::try_from(citation.get("byte_start")?.as_u64()?).ok()?,
                byte_end: u32::try_from(citation.get("byte_end")?.as_u64()?).ok()?,
                quote: citation.get("quote")?.as_str()?.as_bytes().to_vec(),
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(ParsedResponse {
        action,
        target,
        name,
        definition,
        tags,
        salience_micros,
        citations,
    })
}

fn validate_mint_independence(
    cluster: &ObservationCluster,
    citations: &[CitationClaim],
) -> Result<(), DropReason> {
    if cluster
        .observations
        .iter()
        .any(|observation| observation.source.kind == SourceKind::Speculation)
    {
        return Err(DropReason::SpeculationPoisoned);
    }
    let cited = citations
        .iter()
        .filter_map(|citation| {
            cluster
                .observations
                .iter()
                .find(|observation| observation.source.lsn == citation.lsn)
        })
        .collect::<Vec<_>>();
    let roots = cited
        .iter()
        .map(|observation| observation.source.source_root)
        .collect::<BTreeSet<_>>();
    if roots.len() < 3 {
        return Err(DropReason::InsufficientIndependentRoots);
    }
    let conversations = cited
        .iter()
        .map(|observation| observation.source.conversation)
        .collect::<BTreeSet<_>>();
    if conversations.len() < 2 {
        return Err(DropReason::InsufficientConversations);
    }
    Ok(())
}

fn response_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {"type": "string", "enum": ["attach", "revise", "mint"]},
            "target": {"type": ["string", "null"]},
            "name": {"type": "string"},
            "definition": {"type": "string"},
            "tags": {"type": "array", "items": {"type": "string"}, "minItems": 1},
            "salience_micros": {"type": "integer", "minimum": 0, "maximum": 1_000_000},
            "citations": {
                "type": "array",
                "minItems": 1,
                "items": {
                    "type": "object",
                    "properties": {
                        "lsn": {"type": "integer", "minimum": 1},
                        "byte_start": {"type": "integer", "minimum": 0},
                        "byte_end": {"type": "integer", "minimum": 1},
                        "quote": {"type": "string", "minLength": 1}
                    },
                    "required": ["lsn", "byte_start", "byte_end", "quote"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["action", "target", "name", "definition", "tags", "salience_micros", "citations"],
        "additionalProperties": false
    })
}

fn call_id(run_id: &[u8], cluster_id: [u8; 32], model_id: &str) -> Vec<u8> {
    let mut hasher = blake3::Hasher::new();
    hasher.update(run_id);
    hasher.update(&cluster_id);
    hasher.update(model_id.as_bytes());
    hasher.update(b"merge-cluster@1");
    hasher.finalize().as_bytes().to_vec()
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(output, "{byte:02x}");
    }
    output
}
