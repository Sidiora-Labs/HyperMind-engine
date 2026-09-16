#![allow(clippy::missing_errors_doc)]

use crate::citations::{CitationClaim, FrozenCandidate, FrozenCandidateSet};
use crate::quality::{RewriteGuardResult, check_rewrite};
use hm_llm::{LlmError, LlmProvider, StructuredRequest};
use hm_schema::events::{Authority, ModelProvenance, ProvenanceRange};
use serde_json::{Value, json};
use std::fmt::Write;

pub const HINDSIGHT_PROMPT: &str = include_str!("../../../../prompts/hindsight-review@3.md");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConcernSourceKind {
    Neighbour,
    BeliefHistory,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConcernSource {
    pub kind: ConcernSourceKind,
    pub candidate: FrozenCandidate,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HindsightMemory {
    pub memory_id: Vec<u8>,
    pub name: String,
    pub candidate: FrozenCandidate,
    pub faded: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HindsightRevision {
    pub run_id: Vec<u8>,
    pub definition: Vec<u8>,
    pub concern: String,
    pub citations: Vec<ProvenanceRange>,
    pub authority: Authority,
    pub model_provenance: ModelProvenance,
    pub rewrite_guard: RewriteGuardResult,
}

#[derive(Clone, Debug, PartialEq)]
pub enum HindsightDecision {
    NoChange,
    Revised(Box<HindsightRevision>),
    Declined(Vec<String>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HindsightError {
    InvalidEncoding,
    Llm(LlmError),
}

pub fn audit(
    provider: &dyn LlmProvider,
    run_id: &[u8],
    memory: &HindsightMemory,
    sources: &[ConcernSource],
) -> Result<HindsightDecision, HindsightError> {
    if memory.faded || sources.is_empty() {
        return Ok(HindsightDecision::NoChange);
    }
    let request = hindsight_request(memory, sources)?;
    let response = provider
        .generate_structured(&request)
        .map_err(HindsightError::Llm)?;
    Ok(validate_response(run_id, memory, sources, &response))
}

pub fn hindsight_request(
    memory: &HindsightMemory,
    sources: &[ConcernSource],
) -> Result<StructuredRequest, HindsightError> {
    let definition = std::str::from_utf8(&memory.candidate.content)
        .map_err(|_| HindsightError::InvalidEncoding)?;
    let mut prompt = format!(
        "MEMORY\nid={} lsn={} name={} definition={}\nNEIGHBOURS AND BELIEF HISTORY\n",
        hex(&memory.memory_id),
        memory.candidate.lsn,
        memory.name,
        definition
    );
    for source in sources {
        let content = std::str::from_utf8(&source.candidate.content)
            .map_err(|_| HindsightError::InvalidEncoding)?;
        let _ = writeln!(
            prompt,
            "lsn={} kind={:?} bytes={}\n{}\n---",
            source.candidate.lsn,
            source.kind,
            source.candidate.content.len(),
            content
        );
    }
    Ok(StructuredRequest {
        prompt_id: "hindsight-review@3".to_owned(),
        system: HINDSIGHT_PROMPT.to_owned(),
        prompt,
        json_schema: response_schema(),
        maximum_output_tokens: 1_024,
    })
}

fn validate_response(
    run_id: &[u8],
    memory: &HindsightMemory,
    sources: &[ConcernSource],
    response: &hm_llm::StructuredResponse,
) -> HindsightDecision {
    let Some(action) = response.value.get("action").and_then(Value::as_str) else {
        return HindsightDecision::Declined(vec!["invalid structured output".to_owned()]);
    };
    if action == "no_change" {
        return HindsightDecision::NoChange;
    }
    if action != "revise" {
        return HindsightDecision::Declined(vec!["invalid structured output".to_owned()]);
    }
    let Some(concern) = response
        .value
        .get("concern")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return HindsightDecision::Declined(vec!["revision has no concern".to_owned()]);
    };
    let Some(definition) = response
        .value
        .get("definition")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return HindsightDecision::Declined(vec!["revision has no definition".to_owned()]);
    };
    let Some(claims) = parse_citations(&response.value) else {
        return HindsightDecision::Declined(vec!["revision has no valid citations".to_owned()]);
    };
    let Ok(concern_sources) = FrozenCandidateSet::new(
        sources
            .iter()
            .map(|source| source.candidate.clone())
            .collect(),
    ) else {
        return HindsightDecision::Declined(vec!["invalid frozen candidate set".to_owned()]);
    };
    let concern_claims = claims
        .iter()
        .filter(|claim| {
            sources.iter().any(|source| {
                source.candidate.lsn == claim.lsn
                    && matches!(
                        source.kind,
                        ConcernSourceKind::Neighbour | ConcernSourceKind::BeliefHistory
                    )
            })
        })
        .cloned()
        .collect::<Vec<_>>();
    if concern_claims.is_empty() {
        return HindsightDecision::Declined(vec![
            "concern does not cite a neighbour or belief history".to_owned(),
        ]);
    }
    if let Err(error) = concern_sources.validate(concern.as_bytes(), &concern_claims) {
        return HindsightDecision::Declined(vec![format!("invalid concern citation: {error:?}")]);
    }
    let mut definition_sources = sources
        .iter()
        .map(|source| source.candidate.clone())
        .collect::<Vec<_>>();
    definition_sources.push(memory.candidate.clone());
    let validated = match FrozenCandidateSet::new(definition_sources)
        .and_then(|frozen| frozen.validate(definition.as_bytes(), &claims))
    {
        Ok(value) => value,
        Err(error) => {
            return HindsightDecision::Declined(vec![format!(
                "invalid revision citation: {error:?}"
            )]);
        }
    };
    let Ok(old) = std::str::from_utf8(&memory.candidate.content) else {
        return HindsightDecision::Declined(vec!["invalid memory encoding".to_owned()]);
    };
    let evidence = sources
        .iter()
        .filter_map(|source| std::str::from_utf8(&source.candidate.content).ok())
        .collect::<Vec<_>>();
    let guard = check_rewrite(&memory.name, old, definition, &evidence);
    if !guard.accepted {
        return HindsightDecision::Declined(guard.reasons);
    }
    HindsightDecision::Revised(Box::new(HindsightRevision {
        run_id: run_id.to_vec(),
        definition: definition.as_bytes().to_vec(),
        concern: concern.to_owned(),
        citations: validated.ranges,
        authority: validated.authority,
        model_provenance: provenance(
            run_id,
            &memory.memory_id,
            &response.model_id,
            response.usage,
        ),
        rewrite_guard: guard,
    }))
}

fn parse_citations(value: &Value) -> Option<Vec<CitationClaim>> {
    let claims = value
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
    (!claims.is_empty()).then_some(claims)
}

fn provenance(
    run_id: &[u8],
    memory_id: &[u8],
    model_id: &str,
    usage: hm_llm::Usage,
) -> ModelProvenance {
    let mut hash = blake3::Hasher::new();
    hash.update(b"hypermind hindsight call v3\0");
    hash.update(run_id);
    hash.update(memory_id);
    hash.update(model_id.as_bytes());
    ModelProvenance {
        model_id: model_id.to_owned(),
        prompt_id: "hindsight-review".to_owned(),
        prompt_version: 3,
        temperature: 0.0,
        call_id: Some(hash.finalize().as_bytes().to_vec()),
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
        cache_read_tokens: usage.cache_read_tokens,
        cache_write_tokens: usage.cache_write_tokens,
        cost_microusd: usage.cost_microusd,
    }
}

fn response_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {"type": "string", "enum": ["no_change", "revise"]},
            "concern": {"type": ["string", "null"]},
            "definition": {"type": ["string", "null"]},
            "citations": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "lsn": {"type": "integer"},
                        "byte_start": {"type": "integer"},
                        "byte_end": {"type": "integer"},
                        "quote": {"type": "string"}
                    },
                    "required": ["lsn", "byte_start", "byte_end", "quote"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["action", "concern", "definition", "citations"],
        "additionalProperties": false
    })
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    output
}
