#![allow(clippy::missing_errors_doc)]

use crate::citations::FrozenCandidate;
use hm_llm::cost::RunCost;
use hm_llm::{LlmError, LlmProvider, StructuredRequest};
use hm_schema::events::{EdgeAsserted, ModelProvenance, ProvenanceRange};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

pub const CONNECT_PROMPT: &str = include_str!("../../../../prompts/connect-long-context@1.md");

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConnectMemory {
    pub memory_id: Vec<u8>,
    pub cluster_ordinal: u64,
    pub name: String,
    pub definition: Vec<u8>,
    pub entities: BTreeSet<String>,
    pub evidence: Vec<FrozenCandidate>,
    pub faded: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConnectedEdge {
    pub run_id: Vec<u8>,
    pub event: EdgeAsserted,
    pub model_provenance: ModelProvenance,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConnectReport {
    pub edges: Vec<ConnectedEdge>,
    pub dropped: u64,
    pub llm_calls: u64,
    pub cost: RunCost,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConnectError {
    InvalidEncoding,
    InvalidStructuredOutput,
    Llm(LlmError),
    Cost(LlmError),
}

pub fn connect(
    provider: &dyn LlmProvider,
    run_id: &[u8],
    memories: &[ConnectMemory],
) -> Result<ConnectReport, ConnectError> {
    let candidates = candidate_pairs(memories);
    if candidates.is_empty() {
        return Ok(ConnectReport::default());
    }
    let request = connect_request(memories, &candidates)?;
    let response = provider
        .generate_structured(&request)
        .map_err(ConnectError::Llm)?;
    let mut report = ConnectReport {
        llm_calls: 1,
        ..ConnectReport::default()
    };
    report
        .cost
        .record(response.usage)
        .map_err(ConnectError::Cost)?;
    let edges = response
        .value
        .get("edges")
        .and_then(Value::as_array)
        .ok_or(ConnectError::InvalidStructuredOutput)?;
    for edge in edges {
        if let Some(edge) = validate_edge(
            run_id,
            memories,
            &candidates,
            &response.model_id,
            response.usage,
            edge,
        ) {
            report.edges.push(edge);
        } else {
            report.dropped = report.dropped.saturating_add(1);
        }
    }
    Ok(report)
}

pub fn connect_request(
    memories: &[ConnectMemory],
    candidates: &[(usize, usize)],
) -> Result<StructuredRequest, ConnectError> {
    let mut prompt = String::from("MEMORIES\n");
    for memory in memories.iter().filter(|memory| !memory.faded) {
        let definition =
            std::str::from_utf8(&memory.definition).map_err(|_| ConnectError::InvalidEncoding)?;
        let _ = writeln!(
            prompt,
            "id={} cluster={} name={} entities={} definition={}",
            hex(&memory.memory_id),
            memory.cluster_ordinal,
            memory.name,
            memory
                .entities
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(","),
            definition
        );
    }
    prompt.push_str("CANDIDATE PAIRS\n");
    for (left, right) in candidates {
        let _ = writeln!(
            prompt,
            "{} {}",
            hex(&memories[*left].memory_id),
            hex(&memories[*right].memory_id)
        );
    }
    Ok(StructuredRequest {
        prompt_id: "connect-long-context@1".to_owned(),
        system: CONNECT_PROMPT.to_owned(),
        prompt,
        json_schema: response_schema(),
        maximum_output_tokens: 2_048,
    })
}

fn candidate_pairs(memories: &[ConnectMemory]) -> Vec<(usize, usize)> {
    let mut pairs = Vec::new();
    for left in 0..memories.len() {
        if memories[left].faded {
            continue;
        }
        for right in left + 1..memories.len() {
            if memories[right].faded {
                continue;
            }
            let adjacent = memories[left]
                .cluster_ordinal
                .abs_diff(memories[right].cluster_ordinal)
                <= 1;
            let entity_overlap = !memories[left]
                .entities
                .is_disjoint(&memories[right].entities);
            if adjacent || entity_overlap {
                pairs.push((left, right));
            }
        }
    }
    pairs
}

fn validate_edge(
    run_id: &[u8],
    memories: &[ConnectMemory],
    candidates: &[(usize, usize)],
    model_id: &str,
    usage: hm_llm::Usage,
    value: &Value,
) -> Option<ConnectedEdge> {
    let source_hex = value.get("source")?.as_str()?;
    let target_hex = value.get("target")?.as_str()?;
    let relation = value.get("relation")?.as_str()?.trim();
    let (source_index, target_index) = candidates.iter().find_map(|(left, right)| {
        let left_id = hex(&memories[*left].memory_id);
        let right_id = hex(&memories[*right].memory_id);
        ((left_id == source_hex && right_id == target_hex)
            || (left_id == target_hex && right_id == source_hex))
            .then_some((*left, *right))
    })?;
    if relation.is_empty() {
        return None;
    }
    let allowed = memories[source_index]
        .evidence
        .iter()
        .chain(&memories[target_index].evidence)
        .map(|candidate| (candidate.lsn, candidate))
        .collect::<BTreeMap<_, _>>();
    let evidence_lsns = value
        .get("evidence_lsns")?
        .as_array()?
        .iter()
        .map(Value::as_u64)
        .collect::<Option<BTreeSet<_>>>()?;
    if evidence_lsns.is_empty() || evidence_lsns.iter().any(|lsn| !allowed.contains_key(lsn)) {
        return None;
    }
    let citations = evidence_lsns
        .iter()
        .map(|lsn| {
            let candidate = allowed.get(lsn)?;
            Some(ProvenanceRange {
                first_lsn: *lsn,
                last_lsn: *lsn,
                byte_start: 0,
                byte_end: u32::try_from(candidate.content.len()).ok()?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let valid_from_ns = value.get("valid_from_ns")?.as_i64()?;
    let valid_to_ns = value.get("valid_to_ns")?.as_i64()?;
    if valid_to_ns != 0 && valid_to_ns <= valid_from_ns {
        return None;
    }
    let mut edge_hash = blake3::Hasher::new();
    edge_hash.update(b"hypermind edge v1\0");
    edge_hash.update(run_id);
    edge_hash.update(&memories[source_index].memory_id);
    edge_hash.update(&memories[target_index].memory_id);
    edge_hash.update(relation.as_bytes());
    let edge_id = edge_hash.finalize().as_bytes().to_vec();
    Some(ConnectedEdge {
        run_id: run_id.to_vec(),
        event: EdgeAsserted {
            edge_id: edge_id.clone(),
            source_id: memories[source_index].memory_id.clone(),
            target_id: memories[target_index].memory_id.clone(),
            relation: relation.to_owned(),
            weight_micros: u32::try_from(evidence_lsns.len())
                .unwrap_or(u32::MAX)
                .saturating_mul(250_000)
                .min(1_000_000),
            valid_from_ns,
            valid_to_ns,
            citations,
        },
        model_provenance: provenance(run_id, &edge_id, model_id, usage),
    })
}

fn provenance(
    run_id: &[u8],
    item_id: &[u8],
    model_id: &str,
    usage: hm_llm::Usage,
) -> ModelProvenance {
    let mut hash = blake3::Hasher::new();
    hash.update(b"hypermind connect call v1\0");
    hash.update(run_id);
    hash.update(item_id);
    hash.update(model_id.as_bytes());
    ModelProvenance {
        model_id: model_id.to_owned(),
        prompt_id: "connect-long-context".to_owned(),
        prompt_version: 1,
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
            "edges": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "source": {"type": "string"},
                        "target": {"type": "string"},
                        "relation": {"type": "string"},
                        "evidence_lsns": {"type": "array", "items": {"type": "integer"}, "minItems": 1},
                        "valid_from_ns": {"type": "integer"},
                        "valid_to_ns": {"type": "integer"}
                    },
                    "required": ["source", "target", "relation", "evidence_lsns", "valid_from_ns", "valid_to_ns"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["edges"],
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
