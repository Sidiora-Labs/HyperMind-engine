use anyhow::{Context, Result, ensure};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use hm_core::{ActorId, ConversationId, LSN};
use hm_ledger::frame::EventKind;
use hm_ledger::idempotency::ConnectionId;
use hm_schema::event::{self, Boundary};
use hm_schema::events::*;
use hm_serve::actor::{ActorConfig, ActorEngine, IncomingEvent};
use hm_serve::config::ServerConfig;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::Path;

const FORMAT: &str = "hypermind.cortex-import.v1";
const KINDS: [&str; 7] = [
    "observations",
    "memories",
    "edges",
    "beliefs",
    "ops",
    "signals",
    "generic",
];

#[derive(Deserialize)]
struct Manifest {
    #[serde(rename = "type")]
    record_type: String,
    format: String,
    counts: BTreeMap<String, usize>,
    embedding_samples: Vec<EmbeddingSample>,
}

#[derive(Deserialize)]
struct EmbeddingSample {
    kind: String,
    id: String,
    f32_le: String,
}

#[derive(Deserialize)]
struct SourceRecord {
    #[serde(rename = "type")]
    record_type: String,
    kind: String,
    id: String,
    record: Value,
    times_ns: BTreeMap<String, String>,
    embedding_f32_le: Option<String>,
}

impl SourceRecord {
    fn text(&self, key: &str) -> Result<&str> {
        self.record[key]
            .as_str()
            .filter(|v| !v.is_empty())
            .with_context(|| format!("{}/{} requires {key}", self.kind, self.id))
    }

    fn time(&self, key: &str) -> Result<i64> {
        self.times_ns
            .get(key)
            .map_or(Ok(0), |value| value.parse().context("invalid timestamp"))
    }

    fn time_required(&self) -> Result<i64> {
        let time = self.time(if self.kind == "beliefs" {
            "changed_at"
        } else {
            "created_at"
        })?;
        ensure!(
            time > 0,
            "{}/{} requires a positive source timestamp",
            self.kind,
            self.id
        );
        Ok(time)
    }

    fn embedding(&self) -> Result<Option<Vec<u8>>> {
        let Some(encoded) = &self.embedding_f32_le else {
            ensure!(
                self.record["embedding"]
                    .as_array()
                    .is_none_or(Vec::is_empty),
                "embedding bytes missing"
            );
            return Ok(None);
        };
        let bytes = STANDARD.decode(encoded)?;
        let values = self.record["embedding"]
            .as_array()
            .context("embedding values missing")?;
        ensure!(
            bytes.len() == values.len() * 4 && !bytes.is_empty(),
            "embedding dimension mismatch"
        );
        for (chunk, value) in bytes.chunks_exact(4).zip(values) {
            let actual = f32::from_le_bytes(chunk.try_into()?);
            let expected = value.as_f64().context("embedding is not numeric")? as f32;
            ensure!(
                actual.is_finite() && actual.to_bits() == expected.to_bits(),
                "float32 embedding mismatch"
            );
        }
        Ok(Some(bytes))
    }
}

struct Plan {
    events: Vec<IncomingEvent>,
    envelopes: Vec<EventEnvelope>,
    archives: BTreeMap<(String, String), (u64, Vec<u8>)>,
    mappings: Vec<Value>,
    gaps: Vec<Value>,
    run_id: Vec<u8>,
    digest: [u8; 32],
    conversation: ConversationId,
}

impl Plan {
    fn push(
        &mut self,
        payload: EventPayload,
        authority: Authority,
        at: i64,
        derived: bool,
    ) -> Result<u64> {
        let kind = match &payload {
            EventPayload::ProviderFrame(_) => event::EventKind::ProviderFrame,
            EventPayload::UserMsg(_) => event::EventKind::UserMsg,
            EventPayload::DeliveredMsg(_) => event::EventKind::DeliveredMsg,
            EventPayload::MemoryMinted(_) => event::EventKind::MemoryMinted,
            EventPayload::EdgeAsserted(_) => event::EventKind::EdgeAsserted,
            EventPayload::Reviewed(_) => event::EventKind::Reviewed,
            EventPayload::Assertion(_) => event::EventKind::Assertion,
            EventPayload::ConsolidationOpened(_) => event::EventKind::ConsolidationOpened,
            EventPayload::ConsolidationClosed(_) => event::EventKind::ConsolidationClosed,
            _ => anyhow::bail!("unsupported import event"),
        };
        let envelope = EventEnvelope {
            schema_version: event::CURRENT_SCHEMA_VERSION,
            run_id: kind.requires_run_id().then(|| self.run_id.clone()),
            model_provenance: derived.then(|| {
                Box::new(ModelProvenance {
                    model_id: "cortex-store-import".into(),
                    prompt_id: "cortex-store-import/v1".into(),
                    prompt_version: 1,
                    call_id: Some(self.digest.to_vec()),
                    ..ModelProvenance::default()
                })
            }),
            payload,
            authority,
            retention: Retention::Durable,
            event_time_ns: at,
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 0,
            origin_actor: 0,
            sensitivity: Sensitivity::Public,
        };
        let bytes = event::encode_event_envelope(&envelope);
        event::verify_event(&bytes, kind, Boundary::Import).with_context(|| {
            format!("invalid import event {kind:?}; no source data was appended")
        })?;
        self.events.push(IncomingEvent {
            kind: EventKind::try_from(kind as u8)?,
            conversation: self.conversation,
            payload: bytes,
        });
        self.envelopes.push(envelope);
        Ok(self.events.len() as u64)
    }

    fn archive_citation(&self, kind: &str, id: &str) -> Result<ProvenanceRange> {
        let (lsn, bytes) = self
            .archives
            .get(&(kind.into(), id.into()))
            .context("missing source record")?;
        Ok(citation(*lsn, bytes.len())?)
    }
}

fn citation(lsn: u64, length: usize) -> Result<ProvenanceRange> {
    ensure!(length > 0, "cannot cite empty source");
    Ok(ProvenanceRange {
        first_lsn: lsn,
        last_lsn: lsn,
        byte_start: 0,
        byte_end: u32::try_from(length)?,
    })
}

fn micros(value: &Value, divisor: f64) -> Result<u32> {
    let number = value.as_f64().context("missing numeric source field")? / divisor;
    ensure!(
        number.is_finite() && (0.0..=1.0).contains(&number),
        "source value is outside supported range"
    );
    Ok((number * 1_000_000.0).round() as u32)
}

fn plan(bytes: &[u8]) -> Result<(Plan, Manifest, String)> {
    let input = std::str::from_utf8(bytes)?;
    let mut lines = input.lines();
    let first = lines.next().context("empty migration stream")?;
    let manifest: Manifest = serde_json::from_str(first)?;
    ensure!(
        manifest.record_type == "manifest" && manifest.format == FORMAT,
        "unsupported migration format"
    );
    ensure!(
        manifest.counts.len() == KINDS.len()
            && KINDS.iter().all(|kind| manifest.counts.contains_key(*kind)),
        "invalid source counts"
    );
    let mut rows = Vec::new();
    let mut actual = KINDS
        .map(|kind| (kind.to_string(), 0usize))
        .into_iter()
        .collect::<BTreeMap<_, _>>();
    let digest = blake3::hash(bytes);
    let mut plan = Plan {
        events: Vec::new(),
        envelopes: Vec::new(),
        archives: BTreeMap::new(),
        mappings: Vec::new(),
        gaps: Vec::new(),
        run_id: format!("cortex-store-import/{}", digest.to_hex()).into_bytes(),
        digest: *digest.as_bytes(),
        conversation: ConversationId::new(digest.as_bytes()[..16].try_into()?),
    };
    plan.push(EventPayload::ProviderFrame(Box::new(ProviderFrame {provider: FORMAT.into(), api_content: serde_json::to_vec(&json!({"input_blake3": digest.to_hex().to_string(), "manifest": serde_json::from_str::<Value>(first)?}))?})), Authority::ExternalObserved, 0, false)?;
    for (index, line) in lines.enumerate() {
        ensure!(!line.trim().is_empty(), "blank JSON line {}", index + 2);
        let row: SourceRecord =
            serde_json::from_str(line).with_context(|| format!("JSON line {}", index + 2))?;
        ensure!(
            row.record_type == "record" && KINDS.contains(&row.kind.as_str()),
            "unknown source record kind"
        );
        ensure!(
            !row.id.is_empty() && row.record["id"].as_str() == Some(&row.id),
            "source id mismatch"
        );
        ensure!(
            !plan
                .archives
                .contains_key(&(row.kind.clone(), row.id.clone())),
            "duplicate source id"
        );
        row.embedding()?;
        let archive = line.as_bytes().to_vec();
        let lsn = plan.push(
            EventPayload::ProviderFrame(Box::new(ProviderFrame {
                provider: FORMAT.into(),
                api_content: archive.clone(),
            })),
            Authority::ExternalObserved,
            row.time("created_at")?,
            false,
        )?;
        plan.archives
            .insert((row.kind.clone(), row.id.clone()), (lsn, archive));
        *actual.get_mut(&row.kind).context("unknown record kind")? += 1;
        rows.push(row);
    }
    ensure!(
        actual == manifest.counts,
        "source counts mismatch or truncated stream"
    );
    for sample in &manifest.embedding_samples {
        let row = rows
            .iter()
            .find(|row| row.kind == sample.kind && row.id == sample.id)
            .context("embedding sample record missing")?;
        ensure!(
            row.embedding_f32_le.as_ref() == Some(&sample.f32_le),
            "embedding sample mismatch"
        );
    }
    let has_derived = rows
        .iter()
        .any(|row| row.kind == "memories" || row.kind == "edges");
    if has_derived {
        plan.push(
            EventPayload::ConsolidationOpened(Box::new(ConsolidationOpened {
                scope_digest: digest.as_bytes().to_vec(),
                cadence_key: format!("cortex-import:{}", digest.to_hex()),
                generation: 1,
                expected_active_generation: 0,
                phases: vec![ConsolidationPhaseName::Publish],
                prompts: vec![PromptVersion {
                    prompt_id: "cortex-store-import/v1".into(),
                    version: 1,
                    model_id: "cortex-store-import".into(),
                }],
                budget: Box::new(ConsolidationBudget {
                    max_llm_calls: 1,
                    max_tokens: 1,
                    max_microusd: 1,
                    max_wall_ms: 1,
                }),
            })),
            Authority::RuntimeFact,
            0,
            false,
        )?;
    }
    let mut observation_sources = BTreeMap::<String, Vec<ProvenanceRange>>::new();
    let mut memories = BTreeMap::new();
    let mut derived_count = 0;
    for row in rows.iter().filter(|row| row.kind == "observations") {
        let content = row.record["content"]
            .as_str()
            .context("observation content missing")?
            .as_bytes()
            .to_vec();
        let source_type = row.record["source_type"].as_str().unwrap_or("");
        let assistant = matches!(
            source_type,
            "assistant" | "assistant_generated" | "model" | "dream"
        ) || row.record["provenance"]["model_id"].is_string();
        let (payload, authority) = if assistant {
            (
                EventPayload::DeliveredMsg(Box::new(DeliveredMsg {
                    content: content.clone(),
                })),
                Authority::AssistantGenerated,
            )
        } else {
            (
                EventPayload::UserMsg(Box::new(UserMsg {
                    content: content.clone(),
                })),
                Authority::UserAsserted,
            )
        };
        let lsn = plan.push(payload, authority, row.time_required()?, false)?;
        if !content.is_empty() {
            observation_sources
                .entry(row.record["source_file"].as_str().unwrap_or("").into())
                .or_default()
                .push(citation(lsn, content.len())?);
        }
        plan.mappings.push(json!({"kind": row.kind, "id": row.id, "event_lsn": lsn, "source_lsn": plan.archives[&(row.kind.clone(),row.id.clone())].0}));
    }
    for row in rows.iter().filter(|row| row.kind == "memories") {
        let mut citations = Vec::new();
        for source in row.record["source_files"]
            .as_array()
            .context("memory source_files missing")?
        {
            let source = source.as_str().context("invalid source file")?;
            if !source.is_empty() {
                citations.extend(
                    observation_sources
                        .get(source)
                        .into_iter()
                        .flatten()
                        .copied(),
                );
            }
        }
        citations.sort_by_key(|value| value.first_lsn);
        citations.dedup_by_key(|value| value.first_lsn);
        if citations.is_empty() {
            plan.gaps
                .push(json!({"kind":"source_observations_unavailable", "memory_id":row.id}));
            citations.push(plan.archive_citation("memories", &row.id)?);
        }
        let mut tags: Vec<String> =
            serde_json::from_value(row.record["tags"].clone()).context("invalid memory tags")?;
        if tags.is_empty() {
            tags.push("cortex-import".into());
        }
        let lsn = plan.push(
            EventPayload::MemoryMinted(Box::new(MemoryMinted {
                memory_id: row.id.as_bytes().to_vec(),
                name: row.text("name")?.into(),
                definition: row.text("definition")?.as_bytes().to_vec(),
                tags,
                salience_micros: micros(&row.record["salience"], 1.0)?,
                citations,
            })),
            Authority::DerivedInference,
            row.time_required()?,
            true,
        )?;
        derived_count += 1;
        memories.insert(row.id.clone(), lsn);
        let fsrs = &row.record["fsrs"];
        let stability = fsrs["stability"]
            .as_f64()
            .context("missing FSRS stability")?
            * 86_400_000.0;
        ensure!(
            stability.is_finite() && stability >= 1.0 && stability < u64::MAX as f64,
            "invalid FSRS stability"
        );
        let reviewed_at = match row.time("fsrs_last_review")? {
            0 => row.time_required()?,
            value => value,
        };
        let due = reviewed_at
            .checked_add((stability * 1_000_000.0).round() as i64)
            .context("FSRS due time overflow")?;
        let review_lsn = plan.push(
            EventPayload::Reviewed(Box::new(Reviewed {
                memory_id: row.id.as_bytes().to_vec(),
                rating: ReviewRating::Good,
                source_lsn: plan.archives[&(row.kind.clone(), row.id.clone())].0,
                reviewed_at_ns: reviewed_at,
                stability_millis: stability.round() as u64,
                difficulty_micros: micros(&fsrs["difficulty"], 10.0)?,
                due_at_ns: due,
            })),
            Authority::DerivedInference,
            reviewed_at,
            false,
        )?;
        derived_count += 1;
        plan.gaps.push(json!({"kind":"fsrs_import_snapshot", "memory_id":row.id, "review_lsn":review_lsn, "rating":"import_default_good", "due":"last_review_or_created_at_plus_stability_days", "original_state":"source_record"}));
        plan.mappings.push(json!({"kind":row.kind,"id":row.id,"event_lsn":lsn,"source_lsn":plan.archives[&(row.kind.clone(),row.id.clone())].0}));
    }
    for row in rows.iter().filter(|row| row.kind == "edges") {
        ensure!(
            memories.contains_key(row.text("source_id")?)
                && memories.contains_key(row.text("target_id")?),
            "edge references an absent memory"
        );
        let lsn = plan.push(
            EventPayload::EdgeAsserted(Box::new(EdgeAsserted {
                edge_id: row.id.as_bytes().to_vec(),
                source_id: row.text("source_id")?.as_bytes().to_vec(),
                target_id: row.text("target_id")?.as_bytes().to_vec(),
                relation: row.text("relation")?.into(),
                weight_micros: micros(&row.record["weight"], 1.0)?,
                valid_from_ns: row.time_required()?,
                valid_to_ns: 0,
                citations: vec![plan.archive_citation("edges", &row.id)?],
            })),
            Authority::DerivedInference,
            row.time_required()?,
            true,
        )?;
        derived_count += 1;
        plan.mappings.push(json!({"kind":row.kind,"id":row.id,"event_lsn":lsn,"source_lsn":plan.archives[&(row.kind.clone(),row.id.clone())].0}));
    }
    for row in rows.iter().filter(|row| row.kind == "beliefs") {
        ensure!(
            memories.contains_key(row.text("concept_id")?),
            "belief references an absent memory"
        );
        let lsn = plan.push(
            EventPayload::Assertion(Box::new(Assertion {
                belief_id: row.id.as_bytes().to_vec(),
                belief_type: BeliefType::Fact,
                canonical_identity: row.text("concept_id")?.into(),
                value: row.text("new_definition")?.as_bytes().to_vec(),
                valid_from_ns: row.time("valid_from")?,
                valid_to_ns: row.time("valid_to")?,
                provenance: vec![plan.archive_citation("beliefs", &row.id)?],
                conflict_domain: None,
                claim: AssertionClaim::Affirmative,
            })),
            Authority::DerivedInference,
            row.time_required()?,
            false,
        )?;
        plan.mappings.push(json!({"kind":row.kind,"id":row.id,"event_lsn":lsn,"source_lsn":plan.archives[&(row.kind.clone(),row.id.clone())].0}));
    }
    plan.push(
        EventPayload::ProviderFrame(Box::new(ProviderFrame {
            provider: FORMAT.into(),
            api_content: serde_json::to_vec(&json!({
                "type": "import_index", "input_blake3": digest.to_hex().to_string(),
                "mappings": plan.mappings, "gaps": plan.gaps,
                "embedding_storage": "source_record_float32_unindexed_encoder_unknown",
            }))?,
        })),
        Authority::RuntimeFact,
        0,
        false,
    )?;
    if has_derived {
        plan.push(
            EventPayload::ConsolidationClosed(Box::new(ConsolidationClosed {
                generation: 1,
                expected_active_generation: 0,
                derived_records: derived_count,
                ..ConsolidationClosed::default()
            })),
            Authority::RuntimeFact,
            0,
            false,
        )?;
    }
    Ok((plan, manifest, digest.to_hex().to_string()))
}

pub async fn run(config: &ServerConfig, input: &Path) -> Result<Value> {
    let bytes = std::fs::read(input)?;
    let (plan, manifest, digest) = plan(&bytes)?;
    let capability = config.actors.first().context("no actor configured")?;
    let actor = ActorEngine::open(ActorConfig {
        actor_directory: config.actor_directory(capability.actor),
        actor: ActorId::new(capability.actor),
        user: config.user,
        kek: config.kek,
        projection_map_bytes: config.projection_map_bytes,
    })
    .await?;
    let result = apply(&actor, plan, manifest, digest).await;
    let shutdown = actor.shutdown().await;
    let result = result?;
    shutdown?;
    Ok(result)
}

async fn apply(
    actor: &ActorEngine,
    plan: Plan,
    manifest: Manifest,
    digest: String,
) -> Result<Value> {
    let existing = actor.stats().await?.log_events;
    if existing > 0 {
        let first = actor.verified_event(LSN::new(1)).await?;
        ensure!(
            first.envelope.payload == plan.envelopes[0].payload,
            "destination is not empty or belongs to another import"
        );
        ensure!(
            existing <= plan.events.len() as u64,
            "destination contains events after this import"
        );
        for index in 0..existing as usize {
            let actual = actor
                .verified_event(LSN::new(index as u64 + 1))
                .await?
                .envelope;
            let expected = &plan.envelopes[index];
            ensure!(
                actual.payload == expected.payload
                    && actual.authority == expected.authority
                    && actual.event_time_ns == expected.event_time_ns,
                "destination contains a different event; import was not resumed"
            );
        }
    }
    let connection: ConnectionId = plan.digest[..16].try_into()?;
    for (index, events) in plan.events.chunks(64).enumerate() {
        actor
            .append_idempotent(connection, index as u64 + 1, events.to_vec())
            .await?;
    }
    let mut observed_counts = KINDS
        .map(|kind| (kind.to_string(), 0usize))
        .into_iter()
        .collect::<BTreeMap<_, _>>();
    for (index, expected) in plan.envelopes.iter().enumerate() {
        let actual = actor
            .verified_event(LSN::new(index as u64 + 1))
            .await?
            .envelope;
        ensure!(
            actual.payload == expected.payload
                && actual.authority == expected.authority
                && actual.event_time_ns == expected.event_time_ns,
            "persisted event mismatch"
        );
        if let EventPayload::ProviderFrame(source) = actual.payload {
            let value: Value = serde_json::from_slice(&source.api_content)?;
            if value["type"] == "record" {
                let row: SourceRecord = serde_json::from_value(value)?;
                row.embedding()?;
                *observed_counts
                    .get_mut(&row.kind)
                    .context("unexpected persisted source kind")? += 1;
            }
        }
    }
    ensure!(
        observed_counts == manifest.counts,
        "persisted source counts mismatch"
    );
    let stats = actor.stats().await?;
    ensure!(
        stats.log_events == plan.events.len() as u64
            && stats
                .projections
                .iter()
                .all(|projection| projection.applied_lsn.get() == stats.log_events),
        "projection checkpoint mismatch"
    );
    Ok(json!({
        "format":FORMAT, "verified":true, "input_blake3":digest, "counts":observed_counts,
        "events":stats.log_events, "resumed":existing > 0, "embedding_samples_verified":manifest.embedding_samples.len(),
        "embedding_storage":"source_record_float32_unindexed_encoder_unknown", "mappings":plan.mappings, "gaps":plan.gaps,
    }))
}
