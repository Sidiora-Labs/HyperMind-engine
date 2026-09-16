#![allow(clippy::missing_errors_doc)]

use crate::citations::{CitationClaim, CitationError, FrozenCandidate, FrozenCandidateSet};
use crate::quality::{ThoughtQualityOptions, ThoughtQualityResult, assess_thought};
use hm_llm::{LlmError, LlmProvider, StructuredRequest};
use hm_schema::events::{Authority, ModelProvenance, ProvenanceRange};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fmt::Write;

pub const ABSTRACT_PROMPT: &str = include_str!("../../../../prompts/abstract-synthesis@2.md");

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExistingAbstract {
    pub definition: String,
    pub faded: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AbstractOptions {
    pub maximum_similarity_micros: u32,
    pub minimum_grounding_micros: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AbstractDecision {
    pub run_id: Vec<u8>,
    pub name: String,
    pub definition: Vec<u8>,
    pub tag: String,
    pub citations: Vec<ProvenanceRange>,
    pub authority: Authority,
    pub model_provenance: ModelProvenance,
    pub quality: ThoughtQualityResult,
    pub maximum_similarity_micros: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AbstractDrop {
    InvalidStructuredOutput,
    InvalidEncoding,
    Citation(CitationError),
    Quality(Vec<String>),
    Duplicate { similarity_micros: u32 },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AbstractError {
    Llm(LlmError),
    Drop(AbstractDrop),
}

pub fn synthesize(
    provider: &dyn LlmProvider,
    run_id: &[u8],
    tag: &str,
    sources: &[FrozenCandidate],
    store: &[ExistingAbstract],
    current_run: &[AbstractDecision],
    options: AbstractOptions,
) -> Result<AbstractDecision, AbstractError> {
    let request = abstract_request(tag, sources).map_err(AbstractError::Drop)?;
    let response = provider
        .generate_structured(&request)
        .map_err(AbstractError::Llm)?;
    validate_response(run_id, tag, sources, store, current_run, options, &response)
        .map_err(AbstractError::Drop)
}

pub fn abstract_request(
    tag: &str,
    sources: &[FrozenCandidate],
) -> Result<StructuredRequest, AbstractDrop> {
    if tag.trim().is_empty() || sources.is_empty() {
        return Err(AbstractDrop::InvalidStructuredOutput);
    }
    let mut prompt = format!("TAG {tag}\nSOURCES\n");
    for source in sources {
        let content =
            std::str::from_utf8(&source.content).map_err(|_| AbstractDrop::InvalidEncoding)?;
        let _ = writeln!(
            prompt,
            "lsn={} bytes={}\n{}\n---",
            source.lsn,
            source.content.len(),
            content
        );
    }
    Ok(StructuredRequest {
        prompt_id: "abstract-synthesis@2".to_owned(),
        system: ABSTRACT_PROMPT.to_owned(),
        prompt,
        json_schema: response_schema(),
        maximum_output_tokens: 1_024,
    })
}

fn validate_response(
    run_id: &[u8],
    tag: &str,
    sources: &[FrozenCandidate],
    store: &[ExistingAbstract],
    current_run: &[AbstractDecision],
    options: AbstractOptions,
    response: &hm_llm::StructuredResponse,
) -> Result<AbstractDecision, AbstractDrop> {
    if options.maximum_similarity_micros > 1_000_000 || options.minimum_grounding_micros > 1_000_000
    {
        return Err(AbstractDrop::InvalidStructuredOutput);
    }
    let name = response
        .value
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(AbstractDrop::InvalidStructuredOutput)?;
    let definition = response
        .value
        .get("definition")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(AbstractDrop::InvalidStructuredOutput)?;
    let claims = parse_citations(&response.value).ok_or(AbstractDrop::InvalidStructuredOutput)?;
    let frozen = FrozenCandidateSet::new(sources.to_vec()).map_err(AbstractDrop::Citation)?;
    let validated = frozen
        .validate(definition.as_bytes(), &claims)
        .map_err(AbstractDrop::Citation)?;
    let evidence = sources
        .iter()
        .map(|source| std::str::from_utf8(&source.content))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| AbstractDrop::InvalidEncoding)?;
    let quality = assess_thought(
        definition,
        &evidence,
        ThoughtQualityOptions {
            minimum_grounding_micros: options.minimum_grounding_micros,
            ..ThoughtQualityOptions::default()
        },
    );
    if !quality.accepted {
        return Err(AbstractDrop::Quality(quality.reasons));
    }
    let similarity = store
        .iter()
        .filter(|existing| !existing.faded)
        .map(|existing| similarity_micros(definition, &existing.definition))
        .chain(current_run.iter().map(|existing| {
            similarity_micros(definition, &String::from_utf8_lossy(&existing.definition))
        }))
        .max()
        .unwrap_or(0);
    if similarity >= options.maximum_similarity_micros {
        return Err(AbstractDrop::Duplicate {
            similarity_micros: similarity,
        });
    }
    let call_id = call_id(run_id, tag, definition, &response.model_id);
    Ok(AbstractDecision {
        run_id: run_id.to_vec(),
        name: name.to_owned(),
        definition: definition.as_bytes().to_vec(),
        tag: tag.to_owned(),
        citations: validated.ranges,
        authority: validated.authority,
        model_provenance: ModelProvenance {
            model_id: response.model_id.clone(),
            prompt_id: "abstract-synthesis".to_owned(),
            prompt_version: 2,
            temperature: 0.0,
            call_id: Some(call_id),
            input_tokens: response.usage.input_tokens,
            output_tokens: response.usage.output_tokens,
            cache_read_tokens: response.usage.cache_read_tokens,
            cache_write_tokens: response.usage.cache_write_tokens,
            cost_microusd: response.usage.cost_microusd,
        },
        quality,
        maximum_similarity_micros: similarity,
    })
}

fn parse_citations(value: &Value) -> Option<Vec<CitationClaim>> {
    value
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
        .collect()
}

fn similarity_micros(left: &str, right: &str) -> u32 {
    let left = words(left);
    let right = words(right);
    if left.is_empty() && right.is_empty() {
        return 1_000_000;
    }
    let intersection = left.intersection(&right).count();
    let union = left.union(&right).count();
    u32::try_from(intersection.saturating_mul(1_000_000) / union.max(1)).unwrap_or(1_000_000)
}

fn words(value: &str) -> BTreeSet<String> {
    value
        .to_lowercase()
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| word.len() >= 3)
        .map(str::to_owned)
        .collect()
}

fn call_id(run_id: &[u8], tag: &str, definition: &str, model_id: &str) -> Vec<u8> {
    let mut hash = blake3::Hasher::new();
    hash.update(b"hypermind abstract call v2\0");
    hash.update(run_id);
    hash.update(tag.as_bytes());
    hash.update(definition.as_bytes());
    hash.update(model_id.as_bytes());
    hash.finalize().as_bytes().to_vec()
}

fn response_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "definition": {"type": "string"},
            "citations": {
                "type": "array",
                "minItems": 1,
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
        "required": ["name", "definition", "citations"],
        "additionalProperties": false
    })
}
