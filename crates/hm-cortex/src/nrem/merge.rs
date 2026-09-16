#![allow(clippy::missing_errors_doc)]

use crate::budget::{CallReservation, RunReservation};
use crate::citations::{CitationClaim, CitationError, FrozenCandidateSet, SourceKind};
use crate::nrem::cluster::ObservationCluster;
use crate::quality::{
    RewriteGuardResult, ThoughtQualityOptions, ThoughtQualityResult, assess_thought, check_rewrite,
};
use hm_llm::contract::{
    ContractViolation, EXTRACTION_CONTRACT_VERSION, ExtractionContract, FieldRule, FieldShape,
};
use hm_llm::cost::RunCost;
use hm_llm::outcome::ResponseOutcome;
use hm_llm::{LlmError, LlmProvider, StructuredRequest, StructuredResponse};
use hm_schema::events::{Authority, ModelProvenance, ProvenanceRange};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fmt::Write;

pub const MERGE_CLUSTER_PROMPT: &str = include_str!("../../../../prompts/merge-cluster@1.md");

pub const MERGE_OUTPUT_TOKENS: u32 = 1_024;

static MERGE_FIELDS: [FieldRule; 7] = [
    FieldRule {
        name: "action",
        shape: FieldShape::Identifier,
    },
    FieldRule {
        name: "target",
        shape: FieldShape::NullableIdentifier,
    },
    FieldRule {
        name: "name",
        shape: FieldShape::Identifier,
    },
    FieldRule {
        name: "definition",
        shape: FieldShape::Prose,
    },
    FieldRule {
        name: "tags",
        shape: FieldShape::IdentifierList,
    },
    FieldRule {
        name: "salience_micros",
        shape: FieldShape::Count,
    },
    FieldRule {
        name: "citations",
        shape: FieldShape::Opaque,
    },
];

#[must_use]
pub const fn merge_contract() -> ExtractionContract {
    ExtractionContract {
        contract_id: "merge-cluster@1",
        version: EXTRACTION_CONTRACT_VERSION,
        fields: &MERGE_FIELDS,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExistingMemory {
    pub memory_id: Vec<u8>,
    pub name: String,
    pub definition: Vec<u8>,
    pub faded: bool,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MergeOptions {
    pub maximum_attempts_per_cluster: u32,
    pub retry_output_tokens: u32,
}

impl Default for MergeOptions {
    fn default() -> Self {
        Self {
            maximum_attempts_per_cluster: 2,
            retry_output_tokens: MERGE_OUTPUT_TOKENS.saturating_mul(2),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DropReason {
    InvalidStructuredOutput,
    ExtractionContract(ContractViolation),
    Citation(CitationError),
    InsufficientIndependentRoots,
    InsufficientConversations,
    SpeculationPoisoned,
    ThoughtQuality(Vec<String>),
    RewriteGuard(Vec<String>),
    MissingTarget,
    InvalidCandidateEncoding,
    ProviderOutcome(ResponseOutcome),
    RunReservationExhausted,
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
    pub retries: u64,
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

#[derive(Clone, Debug, PartialEq)]
pub enum ClusterOutcome {
    Decided(Box<NremDecision>),
    Dropped(DroppedCandidate),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClusterExtraction {
    pub outcome: ClusterOutcome,
    pub usage: hm_llm::Usage,
    pub llm_calls: u64,
    pub citation_invalid: bool,
}

pub fn extract_cluster(
    provider: &dyn LlmProvider,
    run_id: &[u8],
    cluster: &ObservationCluster,
    existing: &[ExistingMemory],
) -> Result<ClusterExtraction, NremError> {
    let request = match merge_request(cluster, existing) {
        Ok(request) => request,
        Err(reason) => {
            return Ok(ClusterExtraction {
                outcome: ClusterOutcome::Dropped(DroppedCandidate {
                    cluster_id: cluster.cluster_id,
                    reason,
                }),
                usage: hm_llm::Usage::default(),
                llm_calls: 0,
                citation_invalid: false,
            });
        }
    };
    let response = provider.generate_structured(&request)?;
    let (outcome, citation_invalid) = cluster_outcome(run_id, cluster, existing, &response);
    Ok(ClusterExtraction {
        outcome,
        usage: response.usage,
        llm_calls: 1,
        citation_invalid,
    })
}

pub fn consolidate_clusters(
    provider: &dyn LlmProvider,
    run_id: &[u8],
    clusters: &[ObservationCluster],
    existing: &[ExistingMemory],
) -> Result<NremReport, NremError> {
    let mut reservation = RunReservation::unbounded();
    consolidate_clusters_reserved(
        provider,
        run_id,
        clusters,
        existing,
        &mut reservation,
        MergeOptions {
            maximum_attempts_per_cluster: 1,
            retry_output_tokens: MERGE_OUTPUT_TOKENS,
        },
    )
}

pub fn consolidate_clusters_reserved(
    provider: &dyn LlmProvider,
    run_id: &[u8],
    clusters: &[ObservationCluster],
    existing: &[ExistingMemory],
    reservation: &mut RunReservation,
    options: MergeOptions,
) -> Result<NremReport, NremError> {
    let mut report = NremReport::default();
    for cluster in clusters {
        consolidate_cluster(
            provider,
            run_id,
            cluster,
            existing,
            reservation,
            options,
            &mut report,
        )?;
    }
    Ok(report)
}

fn consolidate_cluster(
    provider: &dyn LlmProvider,
    run_id: &[u8],
    cluster: &ObservationCluster,
    existing: &[ExistingMemory],
    reservation: &mut RunReservation,
    options: MergeOptions,
    report: &mut NremReport,
) -> Result<(), NremError> {
    let mut ceiling = MERGE_OUTPUT_TOKENS;
    let mut request = match merge_request_at(cluster, existing, ceiling) {
        Ok(request) => request,
        Err(reason) => {
            report.dropped.push(DroppedCandidate {
                cluster_id: cluster.cluster_id,
                reason,
            });
            return Ok(());
        }
    };
    let attempts = options.maximum_attempts_per_cluster.max(1);
    let mut affordable = true;
    for attempt in 0..attempts {
        if !affordable {
            report.dropped.push(DroppedCandidate {
                cluster_id: cluster.cluster_id,
                reason: DropReason::RunReservationExhausted,
            });
            break;
        }
        let Ok(ticket) = reservation.reserve(CallReservation {
            llm_calls: 1,
            output_tokens: u64::from(ceiling),
        }) else {
            report.dropped.push(DroppedCandidate {
                cluster_id: cluster.cluster_id,
                reason: DropReason::RunReservationExhausted,
            });
            break;
        };
        let fault = match provider.generate_structured(&request) {
            Ok(response) => {
                let _ = reservation.settle(ticket, response.usage);
                record_attempt(report, attempt, response.usage)?;
                let (outcome, citation_invalid) =
                    cluster_outcome(run_id, cluster, existing, &response);
                if citation_invalid {
                    report.citation_invalid = report.citation_invalid.saturating_add(1);
                }
                match outcome {
                    ClusterOutcome::Decided(decision) => report.decisions.push(*decision),
                    ClusterOutcome::Dropped(dropped) => report.dropped.push(dropped),
                }
                break;
            }
            Err(LlmError::Response(fault)) => fault,
            Err(error) => {
                reservation.release(ticket);
                return Err(NremError::Llm(error));
            }
        };
        affordable = reservation.settle(ticket, fault.usage).is_ok();
        record_attempt(report, attempt, fault.usage)?;
        let reask = match fault.outcome {
            ResponseOutcome::Refused => None,
            ResponseOutcome::Truncated => {
                (options.retry_output_tokens > ceiling).then_some(options.retry_output_tokens)
            }
            ResponseOutcome::Malformed | ResponseOutcome::Incomplete => Some(ceiling),
        };
        let Some(reask) = reask.filter(|_| attempt.saturating_add(1) < attempts) else {
            report.dropped.push(DroppedCandidate {
                cluster_id: cluster.cluster_id,
                reason: DropReason::ProviderOutcome(fault.outcome),
            });
            break;
        };
        if reask != ceiling {
            ceiling = reask;
            request = match merge_request_at(cluster, existing, ceiling) {
                Ok(request) => request,
                Err(reason) => {
                    report.dropped.push(DroppedCandidate {
                        cluster_id: cluster.cluster_id,
                        reason,
                    });
                    break;
                }
            };
        }
    }
    Ok(())
}

fn record_attempt(
    report: &mut NremReport,
    attempt: u32,
    usage: hm_llm::Usage,
) -> Result<(), NremError> {
    report.llm_calls = report.llm_calls.saturating_add(1);
    if attempt > 0 {
        report.retries = report.retries.saturating_add(1);
    }
    report.cost.record(usage).map_err(NremError::Cost)
}

fn cluster_outcome(
    run_id: &[u8],
    cluster: &ObservationCluster,
    existing: &[ExistingMemory],
    response: &StructuredResponse,
) -> (ClusterOutcome, bool) {
    match validate_response(
        run_id,
        cluster,
        existing,
        &response.model_id,
        response.usage,
        &response.value,
    ) {
        Ok(decision) => (ClusterOutcome::Decided(Box::new(decision)), false),
        Err(reason) => {
            let citation_invalid = matches!(reason, DropReason::Citation(_));
            (
                ClusterOutcome::Dropped(DroppedCandidate {
                    cluster_id: cluster.cluster_id,
                    reason,
                }),
                citation_invalid,
            )
        }
    }
}

pub fn merge_request(
    cluster: &ObservationCluster,
    existing: &[ExistingMemory],
) -> Result<StructuredRequest, DropReason> {
    merge_request_at(cluster, existing, MERGE_OUTPUT_TOKENS)
}

fn merge_request_at(
    cluster: &ObservationCluster,
    existing: &[ExistingMemory],
    maximum_output_tokens: u32,
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
        for memory in existing.iter().filter(|memory| !memory.faded) {
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
        maximum_output_tokens,
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
    merge_contract()
        .validate(value)
        .map_err(DropReason::ExtractionContract)?;
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
            .find(|memory| !memory.faded && hex(&memory.memory_id) == target)
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
