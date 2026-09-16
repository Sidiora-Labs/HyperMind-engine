#![allow(clippy::cast_precision_loss, clippy::missing_errors_doc)]

//! A controlled comparison that answers whether one retrieval change helps.
//!
//! Exactly one of the four knobs reaches the kernel. `lexical_limit` is the
//! limit of the real `RecallRequest::Lexical` call against a real
//! `ActorEngine`. `minimum_term_overlap`, `session_cap` and `recency_weight`
//! are post-filters this module applies to the returned `RecallItem`s: they are
//! properties of the sweep, not configurable engine settings, and nothing here
//! changes how the engine indexes or scores.
//!
//! The metric is judge-free on purpose. Evidence recall and the reciprocal rank
//! of the first evidence hit are retrieval arithmetic over annotated message
//! ids, so a retrieval regression can never be confused with a reader or a
//! judge problem. `wall_timestamp_ns` and every other clock value the engine
//! returns are discarded, so two runs over the same probe set agree byte for
//! byte.
//!
//! A variant that differs from the baseline in more than one field is refused
//! before any work happens, so a reported delta always belongs to a single
//! isolated change.

use super::beam::{self, Conversation, Message, Probe, ProbeKind, ProbeSet};
use super::beam_run::ENCODER;
use super::gateway::{DynError, write_json};
use hm_core::{ActorId, ConversationId};
use hm_ledger::frame::EventKind;
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{Authority, EventEnvelope, EventPayload, Retention, Sensitivity, UserMsg};
use hm_serve::actor::{ActorConfig, ActorEngine, IncomingEvent, RecallRequest};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub const SWEEP_FORMAT: &str = "hypermind.retrieval-sweep.v1";
pub const BASELINE_NAME: &str = "baseline";
pub const BASELINE_LEXICAL_LIMIT: usize = 16;
pub const BASELINE_SESSION_CAP: usize = 16;

const DELTA_TOLERANCE: f64 = 1e-12;
const CACHE_FILE: &str = "outcomes.json";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RetrievalVariant {
    pub name: String,
    pub lexical_limit: usize,
    pub minimum_term_overlap: f64,
    pub session_cap: usize,
    pub recency_weight: f64,
}

impl RetrievalVariant {
    #[must_use]
    pub fn baseline() -> Self {
        Self {
            name: BASELINE_NAME.to_owned(),
            lexical_limit: BASELINE_LEXICAL_LIMIT,
            minimum_term_overlap: 0.0,
            session_cap: BASELINE_SESSION_CAP,
            recency_weight: 0.0,
        }
    }

    #[must_use]
    pub fn changed_fields(&self, baseline: &Self) -> Vec<&'static str> {
        let mut fields = Vec::new();
        if self.lexical_limit != baseline.lexical_limit {
            fields.push("lexical_limit");
        }
        if self.minimum_term_overlap.to_bits() != baseline.minimum_term_overlap.to_bits() {
            fields.push("minimum_term_overlap");
        }
        if self.session_cap != baseline.session_cap {
            fields.push("session_cap");
        }
        if self.recency_weight.to_bits() != baseline.recency_weight.to_bits() {
            fields.push("recency_weight");
        }
        fields
    }

    fn field_value(&self, field: &str) -> String {
        match field {
            "lexical_limit" => self.lexical_limit.to_string(),
            "minimum_term_overlap" => self.minimum_term_overlap.to_string(),
            "session_cap" => self.session_cap.to_string(),
            "recency_weight" => self.recency_weight.to_string(),
            _ => String::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProbeOutcome {
    pub probe_id: String,
    pub kind: ProbeKind,
    pub evidence_expected: usize,
    pub evidence_retrieved: usize,
    pub first_evidence_rank: Option<usize>,
    pub hits: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VariantSummary {
    pub name: String,
    pub identity: String,
    pub changed_field: Option<String>,
    pub baseline_value: String,
    pub variant_value: String,
    pub probes: usize,
    pub evidence_recall: f64,
    pub mean_reciprocal_rank: f64,
    pub delta_evidence_recall: f64,
    pub delta_mean_reciprocal_rank: f64,
    pub outcomes: Vec<ProbeOutcome>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SweepReport {
    pub format: String,
    pub probe_set_digest: String,
    pub judge_free: bool,
    pub encoder: String,
    pub baseline: VariantSummary,
    pub variants: Vec<VariantSummary>,
    pub improved: Vec<String>,
    pub regressed: Vec<String>,
}

pub fn validate_single_variable(
    baseline: &RetrievalVariant,
    variants: &[RetrievalVariant],
) -> Result<(), DynError> {
    if baseline.name.trim().is_empty() {
        return Err("the sweep baseline carries an empty name".into());
    }
    if variants.is_empty() {
        return Err("a sweep compares the baseline against at least one variant".into());
    }
    let mut seen = BTreeSet::new();
    for (position, variant) in variants.iter().enumerate() {
        if variant.name.trim().is_empty() {
            return Err(format!("the variant at position {position} carries an empty name").into());
        }
        if variant.name == baseline.name {
            return Err(format!(
                "variant {} carries the baseline name {}, so its delta would be measured against itself",
                variant.name, baseline.name
            )
            .into());
        }
        if !seen.insert(variant.name.as_str()) {
            return Err(format!(
                "variant {} appears more than once in the sweep",
                variant.name
            )
            .into());
        }
        let changed = variant.changed_fields(baseline);
        if changed.is_empty() {
            return Err(format!(
                "variant {} differs from the baseline in no field, so it isolates no change",
                variant.name
            )
            .into());
        }
        if changed.len() > 1 {
            return Err(format!(
                "variant {} differs from the baseline in {} fields ({}), so a delta cannot be attributed to one change",
                variant.name,
                changed.len(),
                changed.join(", ")
            )
            .into());
        }
    }
    Ok(())
}

pub fn variant_identity(
    probe_set_digest: &str,
    variant: &RetrievalVariant,
) -> Result<String, DynError> {
    let record = json!({
        "format": SWEEP_FORMAT,
        "probe_set": probe_set_digest,
        "variant": variant,
        "implementation": implementation_digest(),
    });
    Ok(blake3::hash(&serde_json::to_vec(&record)?)
        .to_hex()
        .to_string())
}

#[must_use]
pub fn variant_cache_path(root: &Path, identity: &str) -> PathBuf {
    root.join(identity).join(CACHE_FILE)
}

#[allow(clippy::too_many_lines)]
pub async fn run(
    set: &ProbeSet,
    baseline: &RetrievalVariant,
    variants: &[RetrievalVariant],
    root: &Path,
) -> Result<SweepReport, DynError> {
    validate_single_variable(baseline, variants)?;
    let probe_set_digest = beam::probe_set_digest(set)?;
    let mut records = Vec::with_capacity(variants.len() + 1);
    records.push(baseline.clone());
    records.extend(variants.iter().cloned());
    let mut identities = Vec::with_capacity(records.len());
    for record in &records {
        identities.push(variant_identity(&probe_set_digest, record)?);
    }
    let mut cached = Vec::with_capacity(records.len());
    for identity in &identities {
        cached.push(read_cache(root, identity)?);
    }
    let pending: Vec<usize> = (0..records.len())
        .filter(|index| cached[*index].is_none())
        .collect();
    if !pending.is_empty() {
        let mut collected: Vec<Vec<ProbeOutcome>> = vec![Vec::new(); records.len()];
        for conversation in &set.conversations {
            let prepared = prepare(root, conversation).await?;
            let mut failure = None;
            'variants: for index in &pending {
                for probe in &conversation.probes {
                    match measure(&prepared, conversation, probe, &records[*index]).await {
                        Ok(outcome) => collected[*index].push(outcome),
                        Err(error) => {
                            failure = Some(error);
                            break 'variants;
                        }
                    }
                }
            }
            prepared.close().await?;
            if let Some(error) = failure {
                return Err(error);
            }
        }
        for index in &pending {
            let outcomes = std::mem::take(&mut collected[*index]);
            write_json(
                &variant_cache_path(root, &identities[*index]),
                &VariantCache {
                    format: SWEEP_FORMAT.to_owned(),
                    identity: identities[*index].clone(),
                    probe_set_digest: probe_set_digest.clone(),
                    variant: records[*index].clone(),
                    outcomes: outcomes.clone(),
                },
            )?;
            cached[*index] = Some(outcomes);
        }
    }
    let mut resolved = Vec::with_capacity(records.len());
    for (index, entry) in cached.into_iter().enumerate() {
        resolved.push(entry.ok_or_else(|| {
            format!("variant {} produced no probe outcomes", records[index].name)
        })?);
    }
    let baseline_summary = summarize(
        &records[0],
        &identities[0],
        baseline,
        std::mem::take(&mut resolved[0]),
        None,
    );
    let mut summaries = Vec::with_capacity(variants.len());
    let mut improved = Vec::new();
    let mut regressed = Vec::new();
    for index in 1..records.len() {
        let summary = summarize(
            &records[index],
            &identities[index],
            baseline,
            std::mem::take(&mut resolved[index]),
            Some(&baseline_summary),
        );
        let recall = sign(summary.delta_evidence_recall);
        let rank = sign(summary.delta_mean_reciprocal_rank);
        if recall > 0 || (recall == 0 && rank > 0) {
            improved.push(summary.name.clone());
        } else if recall < 0 || (recall == 0 && rank < 0) {
            regressed.push(summary.name.clone());
        }
        summaries.push(summary);
    }
    Ok(SweepReport {
        format: SWEEP_FORMAT.to_owned(),
        probe_set_digest,
        judge_free: true,
        encoder: ENCODER.to_owned(),
        baseline: baseline_summary,
        variants: summaries,
        improved,
        regressed,
    })
}

#[derive(Deserialize, Serialize)]
struct VariantCache {
    format: String,
    identity: String,
    probe_set_digest: String,
    variant: RetrievalVariant,
    outcomes: Vec<ProbeOutcome>,
}

struct Candidate {
    message: u64,
    session: usize,
    lsn: u64,
    score: u64,
}

struct Prepared {
    actor: ActorEngine,
    message_of_lsn: BTreeMap<u64, u64>,
    session_of_message: BTreeMap<u64, usize>,
    text_of_message: BTreeMap<u64, String>,
    highest_lsn: u64,
}

impl Prepared {
    async fn close(self) -> Result<(), DynError> {
        self.actor.shutdown().await?;
        Ok(())
    }
}

fn read_cache(root: &Path, identity: &str) -> Result<Option<Vec<ProbeOutcome>>, DynError> {
    let path = variant_cache_path(root, identity);
    if !path.exists() {
        return Ok(None);
    }
    let cache: VariantCache = serde_json::from_slice(&std::fs::read(&path)?)
        .map_err(|error| format!("variant cache {}: {error}", path.display()))?;
    if cache.format != SWEEP_FORMAT {
        return Err(format!(
            "variant cache {} carries format tag {} and not {SWEEP_FORMAT}",
            path.display(),
            cache.format
        )
        .into());
    }
    let rederived = variant_identity(&cache.probe_set_digest, &cache.variant)?;
    if cache.identity != rederived || rederived != identity {
        return Err(format!(
            "variant cache {} identity mismatch: the file records identity {}, the variant record re-derives {rederived} and this sweep expects {identity}; the cached result is foreign and is not reused",
            path.display(),
            cache.identity
        )
        .into());
    }
    Ok(Some(cache.outcomes))
}

async fn prepare(root: &Path, conversation: &Conversation) -> Result<Prepared, DynError> {
    let digest = blake3::hash(&serde_json::to_vec(conversation)?)
        .to_hex()
        .to_string();
    let directory = root.join("engine").join(&digest);
    std::fs::create_dir_all(&directory)?;
    let messages: Vec<&Message> = conversation
        .sessions
        .iter()
        .flat_map(|session| session.messages.iter())
        .collect();
    let kek =
        *blake3::hash(format!("{SWEEP_FORMAT}:{digest}:retrieval-sweep-key").as_bytes()).as_bytes();
    let actor = ActorEngine::open(ActorConfig {
        actor_directory: directory.join("actor"),
        actor: ActorId::new(1),
        user: [1; 16],
        kek,
        projection_map_bytes: 512 * 1024 * 1024,
    })
    .await
    .map_err(|error| {
        format!(
            "conversation {}: actor open/replay: {error}",
            conversation.id
        )
    })?;
    match ingest(&actor, conversation, &messages, &directory).await {
        Ok(pairs) => {
            let mut message_of_lsn = BTreeMap::new();
            let mut highest_lsn = 0;
            for (lsn, message) in pairs {
                message_of_lsn.insert(lsn, message);
                highest_lsn = highest_lsn.max(lsn);
            }
            let mut session_of_message = BTreeMap::new();
            let mut text_of_message = BTreeMap::new();
            for session in &conversation.sessions {
                for message in &session.messages {
                    session_of_message.insert(message.id, session.index);
                    text_of_message.insert(message.id, message.text.clone());
                }
            }
            Ok(Prepared {
                actor,
                message_of_lsn,
                session_of_message,
                text_of_message,
                highest_lsn,
            })
        }
        Err(error) => {
            let _ = actor.shutdown().await;
            Err(error)
        }
    }
}

async fn ingest(
    actor: &ActorEngine,
    conversation: &Conversation,
    messages: &[&Message],
    directory: &Path,
) -> Result<Vec<(u64, u64)>, DynError> {
    let manifest = directory.join("messages.json");
    let stats = actor
        .stats()
        .await
        .map_err(|error| format!("conversation {}: ingestion stats: {error}", conversation.id))?;
    if stats.log_events == 0 {
        let mut pairs = Vec::with_capacity(messages.len());
        for message in messages {
            let outcome = actor
                .append(vec![IncomingEvent {
                    kind: EventKind::UserMsg,
                    conversation: ConversationId::derive(&conversation.id),
                    payload: encode_event_envelope(&EventEnvelope {
                        schema_version: CURRENT_SCHEMA_VERSION,
                        payload: EventPayload::UserMsg(Box::new(UserMsg {
                            content: message.text.as_bytes().to_vec(),
                        })),
                        connection_id: None,
                        client_seq: 0,
                        client_event_index: 0,
                        client_event_count: 0,
                        origin_actor: 0,
                        run_id: None,
                        model_provenance: None,
                        authority: Authority::ExternalObserved,
                        retention: Retention::Durable,
                        sensitivity: Sensitivity::Public,
                        event_time_ns: 0,
                    }),
                }])
                .await
                .map_err(|error| {
                    format!(
                        "conversation {}: ingest message {}: {error}",
                        conversation.id, message.id
                    )
                })?;
            pairs.push((outcome.last_lsn.get(), message.id));
        }
        write_json(&manifest, &pairs)?;
        return Ok(pairs);
    }
    if stats.log_events != messages.len() as u64 || !manifest.exists() {
        return Err(format!(
            "conversation {} ledger holds {} events for {} messages and no matching ingestion manifest",
            conversation.id,
            stats.log_events,
            messages.len()
        )
        .into());
    }
    let pairs: Vec<(u64, u64)> = serde_json::from_slice(&std::fs::read(&manifest)?)
        .map_err(|error| format!("ingestion manifest {}: {error}", manifest.display()))?;
    if pairs.len() != messages.len() {
        return Err(format!(
            "ingestion manifest {} maps {} events onto {} messages",
            manifest.display(),
            pairs.len(),
            messages.len()
        )
        .into());
    }
    Ok(pairs)
}

async fn measure(
    prepared: &Prepared,
    conversation: &Conversation,
    probe: &Probe,
    variant: &RetrievalVariant,
) -> Result<ProbeOutcome, DynError> {
    let hits = prepared
        .actor
        .recall(RecallRequest::Lexical {
            query: probe.question.clone(),
            limit: variant.lexical_limit,
        })
        .await
        .map_err(|error| {
            format!(
                "conversation {}, probe {}, variant {}: lexical recall: {error}",
                conversation.id, probe.id, variant.name
            )
        })?;
    let question = terms(&probe.question);
    let mut ranked = Vec::new();
    for hit in &hits {
        let lsn = hit.lsn.get();
        let Some(message) = prepared.message_of_lsn.get(&lsn) else {
            continue;
        };
        let Some(text) = prepared.text_of_message.get(message) else {
            continue;
        };
        if overlap(&question, text) < variant.minimum_term_overlap {
            continue;
        }
        ranked.push(Candidate {
            message: *message,
            session: prepared
                .session_of_message
                .get(message)
                .copied()
                .unwrap_or_default(),
            lsn,
            score: hit.score_q32,
        });
    }
    let mut per_session: BTreeMap<usize, usize> = BTreeMap::new();
    ranked.retain(|candidate| {
        let taken = per_session.entry(candidate.session).or_default();
        if *taken >= variant.session_cap {
            return false;
        }
        *taken += 1;
        true
    });
    if variant.recency_weight.to_bits() != 0_f64.to_bits() {
        ranked = reorder_by_recency(ranked, variant.recency_weight, prepared.highest_lsn);
    }
    let expected: BTreeSet<u64> = probe.evidence_message_ids.iter().copied().collect();
    let mut retrieved = BTreeSet::new();
    let mut first_evidence_rank = None;
    for (position, candidate) in ranked.iter().enumerate() {
        if !expected.contains(&candidate.message) {
            continue;
        }
        retrieved.insert(candidate.message);
        if first_evidence_rank.is_none() {
            first_evidence_rank = Some(position + 1);
        }
    }
    Ok(ProbeOutcome {
        probe_id: probe.id.clone(),
        kind: probe.kind,
        evidence_expected: expected.len(),
        evidence_retrieved: retrieved.len(),
        first_evidence_rank,
        hits: ranked.len(),
    })
}

fn reorder_by_recency(ranked: Vec<Candidate>, weight: f64, highest_lsn: u64) -> Vec<Candidate> {
    let highest = highest_lsn.max(1) as f64;
    let maximum = ranked
        .iter()
        .map(|candidate| candidate.score)
        .max()
        .unwrap_or(0);
    let mut keyed: Vec<(f64, usize, Candidate)> = ranked
        .into_iter()
        .enumerate()
        .map(|(position, candidate)| {
            let relevance = if maximum == 0 {
                0.0
            } else {
                candidate.score as f64 / maximum as f64
            };
            let recency = candidate.lsn as f64 / highest;
            (
                (1.0 - weight).mul_add(relevance, weight * recency),
                position,
                candidate,
            )
        })
        .collect();
    keyed.sort_by(|left, right| right.0.total_cmp(&left.0).then(left.1.cmp(&right.1)));
    keyed
        .into_iter()
        .map(|(_, _, candidate)| candidate)
        .collect()
}

fn summarize(
    record: &RetrievalVariant,
    identity: &str,
    baseline: &RetrievalVariant,
    outcomes: Vec<ProbeOutcome>,
    reference: Option<&VariantSummary>,
) -> VariantSummary {
    let mut recall_total = 0.0;
    let mut reciprocal_total = 0.0;
    let mut with_evidence = 0;
    for outcome in &outcomes {
        if outcome.evidence_expected == 0 {
            continue;
        }
        with_evidence += 1;
        recall_total += outcome.evidence_retrieved as f64 / outcome.evidence_expected as f64;
        if let Some(rank) = outcome.first_evidence_rank
            && rank > 0
        {
            reciprocal_total += 1.0 / rank as f64;
        }
    }
    let evidence_recall = mean(recall_total, with_evidence);
    let mean_reciprocal_rank = mean(reciprocal_total, with_evidence);
    let changed = record.changed_fields(baseline);
    let changed_field = changed.first().map(|field| (*field).to_owned());
    let (baseline_value, variant_value) = changed_field.as_ref().map_or_else(
        || (String::new(), String::new()),
        |field| (baseline.field_value(field), record.field_value(field)),
    );
    VariantSummary {
        name: record.name.clone(),
        identity: identity.to_owned(),
        changed_field,
        baseline_value,
        variant_value,
        probes: outcomes.len(),
        evidence_recall,
        mean_reciprocal_rank,
        delta_evidence_recall: reference
            .map_or(0.0, |summary| evidence_recall - summary.evidence_recall),
        delta_mean_reciprocal_rank: reference.map_or(0.0, |summary| {
            mean_reciprocal_rank - summary.mean_reciprocal_rank
        }),
        outcomes,
    }
}

fn terms(text: &str) -> BTreeSet<String> {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|term| !term.is_empty())
        .map(str::to_lowercase)
        .collect()
}

fn overlap(question: &BTreeSet<String>, text: &str) -> f64 {
    if question.is_empty() {
        return 0.0;
    }
    let observed = terms(text);
    let shared = question
        .iter()
        .filter(|term| observed.contains(*term))
        .count();
    shared as f64 / question.len() as f64
}

fn mean(total: f64, count: usize) -> f64 {
    if count == 0 {
        0.0
    } else {
        total / count as f64
    }
}

fn sign(delta: f64) -> i8 {
    if delta > DELTA_TOLERANCE {
        1
    } else if delta < -DELTA_TOLERANCE {
        -1
    } else {
        0
    }
}

fn implementation_digest() -> String {
    let source = include_str!("sweep.rs");
    let mut digest = blake3::Hasher::new();
    digest.update(&(source.len() as u64).to_le_bytes());
    digest.update(source.as_bytes());
    digest.finalize().to_hex().to_string()
}
