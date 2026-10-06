use crate::types::{
    Authority, ContextBlock, ContextError, Omission, Scope, SourceSpan, validate_id,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    Memory,
    Conversation,
    File,
    Commit,
    Document,
    Entity,
    Relationship,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VectorFingerprint {
    pub model: String,
    pub revision: String,
    pub dimensions: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbeddedVector {
    pub fingerprint: VectorFingerprint,
    pub content_digest: String,
    pub values: Vec<f32>,
}

impl EmbeddedVector {
    pub fn validate(&self) -> Result<(), ContextError> {
        validate_id(&self.fingerprint.model)?;
        validate_id(&self.fingerprint.revision)?;
        validate_id(&self.content_digest)?;
        if self.values.is_empty()
            || self.values.len() > 65_536
            || self.values.len() != self.fingerprint.dimensions
            || self.values.iter().any(|v| !v.is_finite())
            || self.values.iter().all(|v| *v == 0.0)
        {
            return Err(ContextError::Invalid("invalid embedding vector".into()));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceCandidate {
    pub scope: Scope,
    pub kind: SourceKind,
    pub id: String,
    pub text: String,
    pub authority: Authority,
    pub provenance: Vec<SourceSpan>,
    pub tokens: u64,
    pub content_digest: String,
    pub source_revision: u64,
    pub current_revision: u64,
    #[serde(with = "crate::types::optional_timestamp_wire")]
    pub occurred_at_ns: Option<i64>,
    #[serde(with = "crate::types::timestamp_wire")]
    pub recorded_at_ns: i64,
    #[serde(with = "crate::types::optional_timestamp_wire")]
    pub valid_from_ns: Option<i64>,
    #[serde(with = "crate::types::optional_timestamp_wire")]
    pub expires_at_ns: Option<i64>,
    pub tombstoned: bool,
    pub vector: Option<EmbeddedVector>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexBatch {
    pub kind: SourceKind,
    pub index_revision: u64,
    pub candidates: Vec<EvidenceCandidate>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceGrant {
    pub source_scope: Scope,
    pub recipient_scope: Scope,
    pub kind: SourceKind,
    pub source_id: String,
    pub source_digest: String,
    pub source_revision: u64,
    #[serde(with = "crate::types::timestamp_wire")]
    pub expires_at_ns: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeField {
    Occurred,
    Recorded,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceTimeFilter {
    pub field: TimeField,
    #[serde(with = "crate::types::optional_timestamp_wire")]
    pub from_ns: Option<i64>,
    #[serde(with = "crate::types::optional_timestamp_wire")]
    pub through_ns: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetrievalRequest {
    pub scope: Scope,
    #[serde(with = "crate::types::timestamp_wire")]
    pub now_ns: i64,
    pub grants: Vec<EvidenceGrant>,
    pub visible: Vec<SourceSpan>,
    pub time_filter: Option<SourceTimeFilter>,
    pub query_vector: Option<EmbeddedVector>,
    pub max_candidates: usize,
    pub max_results: usize,
    pub max_tokens: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum SemanticStatus {
    Unavailable { reason: String },
    Available { scored: usize, unavailable: usize },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RankedEvidence {
    pub candidate: EvidenceCandidate,
    pub fusion_score: f64,
    pub semantic_score: Option<f64>,
}

impl RankedEvidence {
    pub fn context_block(&self) -> ContextBlock {
        ContextBlock {
            id: self.candidate.id.clone(),
            text: self.candidate.text.clone(),
            authority: self.candidate.authority,
            provenance: self.candidate.provenance.clone(),
            tokens: self.candidate.tokens,
            required: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetrievalReport {
    pub results: Vec<RankedEvidence>,
    pub omitted: Vec<Omission>,
    pub semantic: SemanticStatus,
    pub unauthorized_count: usize,
    pub token_count: u64,
}

type Key = (Scope, SourceKind, String);

pub fn fuse(
    request: &RetrievalRequest,
    batches: &[IndexBatch],
) -> Result<RetrievalReport, ContextError> {
    fuse_inner(request, batches, None)
}

pub fn fuse_with_lexical_members(
    request: &RetrievalRequest,
    batches: &[IndexBatch],
    lexical_members: &BTreeSet<(Scope, SourceKind, String)>,
) -> Result<RetrievalReport, ContextError> {
    if lexical_members.len() > request.max_candidates {
        return Err(ContextError::Capacity);
    }
    fuse_inner(request, batches, Some(lexical_members))
}

fn fuse_inner(
    request: &RetrievalRequest,
    batches: &[IndexBatch],
    lexical_members: Option<&BTreeSet<Key>>,
) -> Result<RetrievalReport, ContextError> {
    request.scope.validate()?;
    if request.max_candidates == 0
        || request.max_candidates > 100_000
        || request.max_results > request.max_candidates
        || batches.len() > 64
        || request.grants.len() > 100_000
        || request.visible.len() > 100_000
    {
        return Err(ContextError::Capacity);
    }
    let count = batches.iter().try_fold(0usize, |n, b| {
        n.checked_add(b.candidates.len())
            .ok_or(ContextError::Capacity)
    })?;
    if count > request.max_candidates {
        return Err(ContextError::Capacity);
    }
    if let Some(query) = &request.query_vector {
        query.validate()?;
    }
    if let Some(filter) = &request.time_filter {
        if matches!((filter.from_ns, filter.through_ns), (Some(a), Some(b)) if a > b) {
            return Err(ContextError::Invalid("reversed source-time range".into()));
        }
    }
    for span in &request.visible {
        validate_span(span)?;
    }
    for grant in &request.grants {
        grant.source_scope.validate()?;
        grant.recipient_scope.validate()?;
        validate_id(&grant.source_id)?;
        validate_id(&grant.source_digest)?;
    }
    let mut report = RetrievalReport {
        results: vec![],
        omitted: vec![],
        semantic: SemanticStatus::Unavailable {
            reason: "query vector absent".into(),
        },
        unauthorized_count: 0,
        token_count: 0,
    };
    let mut pool: BTreeMap<Key, RankedEvidence> = BTreeMap::new();
    let mut ranks: BTreeMap<(SourceKind, Key), usize> = BTreeMap::new();
    for batch in batches {
        for (offset, candidate) in batch.candidates.iter().enumerate() {
            candidate.scope.validate()?;
            validate_id(&candidate.id)?;
            if candidate.kind != batch.kind {
                return Err(ContextError::Invalid("index source kind mismatch".into()));
            }
            let authorized = candidate.scope == request.scope
                || request.grants.iter().any(|g| {
                    g.source_scope == candidate.scope
                        && g.recipient_scope == request.scope
                        && g.kind == candidate.kind
                        && g.source_id == candidate.id
                        && g.source_digest == candidate.content_digest
                        && g.source_revision == candidate.source_revision
                        && g.expires_at_ns > request.now_ns
                });
            if !authorized {
                report.unauthorized_count += 1;
                continue;
            }
            validate_id(&candidate.content_digest)?;
            if candidate.text.len() > 4 * 1024 * 1024
                || candidate.tokens == 0
                || candidate.provenance.is_empty()
                || candidate.provenance.len() > 4096
            {
                return Err(ContextError::Invalid("invalid evidence content".into()));
            }
            for span in &candidate.provenance {
                validate_span(span)?;
            }
            if let Some(vector) = &candidate.vector {
                vector.validate()?;
            }
            let reason = if candidate.tombstoned {
                Some("tombstoned")
            } else if candidate.source_revision != candidate.current_revision
                || candidate.source_revision > batch.index_revision
            {
                Some("stale")
            } else if candidate.expires_at_ns.is_some_and(|t| t <= request.now_ns) {
                Some("expired")
            } else if candidate.valid_from_ns.is_some_and(|t| t > request.now_ns) {
                Some("not_yet_valid")
            } else if request.time_filter.as_ref().is_some_and(|f| {
                let time = match f.field {
                    TimeField::Occurred => candidate.occurred_at_ns,
                    TimeField::Recorded => Some(candidate.recorded_at_ns),
                };
                time.is_none_or(|t| {
                    f.from_ns.is_some_and(|a| t < a) || f.through_ns.is_some_and(|b| t > b)
                })
            }) {
                Some("source_time_filtered")
            } else if candidate
                .provenance
                .iter()
                .all(|span| covered(span, &request.visible))
            {
                Some("already_visible")
            } else {
                None
            };
            if let Some(reason) = reason {
                report.omitted.push(Omission {
                    id: candidate.id.clone(),
                    reason: reason.into(),
                });
                continue;
            }
            let key = (
                candidate.scope.clone(),
                candidate.kind,
                candidate.id.clone(),
            );
            if let Some(existing) = pool.get(&key) {
                if existing.candidate != *candidate {
                    return Err(ContextError::Conflict);
                }
            } else {
                pool.insert(
                    key.clone(),
                    RankedEvidence {
                        candidate: candidate.clone(),
                        fusion_score: 0.0,
                        semantic_score: None,
                    },
                );
            }
            if lexical_members.is_none_or(|members| members.contains(&key)) {
                ranks
                    .entry((batch.kind, key))
                    .and_modify(|rank| *rank = (*rank).min(offset + 1))
                    .or_insert(offset + 1);
            }
        }
    }
    for ((_, key), rank) in ranks {
        pool.get_mut(&key)
            .expect("ranked candidate exists")
            .fusion_score += 1.0 / (60.0 + rank as f64);
    }
    if let Some(query) = &request.query_vector {
        let mut semantic_ranks: BTreeMap<SourceKind, Vec<(Key, f64)>> = BTreeMap::new();
        let mut scored = 0;
        for (key, evidence) in &mut pool {
            if let Some(vector) = &evidence.candidate.vector {
                if vector.fingerprint == query.fingerprint
                    && vector.content_digest == evidence.candidate.content_digest
                {
                    let score = cosine(&query.values, &vector.values);
                    evidence.semantic_score = Some(score);
                    semantic_ranks
                        .entry(evidence.candidate.kind)
                        .or_default()
                        .push((key.clone(), score));
                    scored += 1;
                }
            }
        }
        for lane in semantic_ranks.values_mut() {
            lane.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            for (rank, (key, _)) in lane.iter().enumerate() {
                pool.get_mut(key)
                    .expect("semantic candidate exists")
                    .fusion_score += 1.0 / (61.0 + rank as f64);
            }
        }
        report.semantic = if scored == 0 {
            SemanticStatus::Unavailable {
                reason: "no compatible current candidate vectors".into(),
            }
        } else {
            SemanticStatus::Available {
                scored,
                unavailable: pool.len() - scored,
            }
        };
    }
    let mut ordered: Vec<_> = pool.into_iter().collect();
    ordered.sort_by(|a, b| {
        b.1.fusion_score
            .total_cmp(&a.1.fusion_score)
            .then_with(|| a.0.cmp(&b.0))
    });
    for (_, evidence) in ordered {
        let tokens = report
            .token_count
            .checked_add(evidence.candidate.tokens)
            .ok_or(ContextError::Capacity)?;
        if report.results.len() >= request.max_results || tokens > request.max_tokens {
            report.omitted.push(Omission {
                id: evidence.candidate.id,
                reason: "retrieval_budget".into(),
            });
        } else {
            report.token_count = tokens;
            report.results.push(evidence);
        }
    }
    report
        .omitted
        .sort_by(|a, b| (&a.id, &a.reason).cmp(&(&b.id, &b.reason)));
    report.omitted.dedup();
    Ok(report)
}

fn validate_span(span: &SourceSpan) -> Result<(), ContextError> {
    validate_id(&span.source_id)?;
    validate_id(&span.source_digest)?;
    if span.byte_start >= span.byte_end {
        return Err(ContextError::Invalid("invalid source span".into()));
    }
    Ok(())
}

fn covered(span: &SourceSpan, visible: &[SourceSpan]) -> bool {
    let mut intervals: Vec<_> = visible
        .iter()
        .filter(|v| v.source_id == span.source_id && v.source_digest == span.source_digest)
        .map(|v| (v.byte_start, v.byte_end))
        .collect();
    intervals.sort_unstable();
    let mut end = span.byte_start;
    for (start, next) in intervals {
        if start > end {
            break;
        }
        end = end.max(next);
        if end >= span.byte_end {
            return true;
        }
    }
    false
}

fn cosine(a: &[f32], b: &[f32]) -> f64 {
    let dot: f64 = a
        .iter()
        .zip(b)
        .map(|(x, y)| f64::from(*x) * f64::from(*y))
        .sum();
    let norm_a: f64 = a.iter().map(|x| f64::from(*x).powi(2)).sum();
    let norm_b: f64 = b.iter().map(|x| f64::from(*x).powi(2)).sum();
    (dot / (norm_a.sqrt() * norm_b.sqrt())).clamp(-1.0, 1.0)
}
