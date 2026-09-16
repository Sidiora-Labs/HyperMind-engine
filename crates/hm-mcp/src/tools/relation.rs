#![allow(clippy::missing_errors_doc)]

use crate::tools::remember::{EmbeddingRuntime, relation_space_id};
use hm_core::{ConversationId, Error, ErrorCode, LSN};
use hm_cortex::relations::{RelationOptions, RelationRepresentation, RelationSource, represent};
use hm_embed::{QuantizedEmbedding, SpaceIdentity};
use hm_ledger::frame::EventKind;
use hm_schema::event::{self, Boundary, CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    Authority, Embedding, EventEnvelope, EventPayload, ModelProvenance, Retention, Sensitivity,
};
use hm_serve::actor::{ActorEngine, IncomingEvent};
use std::collections::BTreeSet;

const MAXIMUM_REPRESENTATION_BYTES: usize = 4_096;
const RELATION_CONVERSATION: &str = "hypermind.relations";
const RELATION_PROMPT: &str = "embedding/relation";

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RelationBuildReport {
    pub scanned: u64,
    pub embedded: u64,
    pub skipped: u64,
    pub space_id: String,
    pub first_lsn: u64,
    pub last_lsn: u64,
}

pub async fn build(
    actor: &ActorEngine,
    runtime: &EmbeddingRuntime,
    maximum_relations: usize,
) -> Result<RelationBuildReport, Error> {
    if maximum_relations == 0 {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let space = runtime.document_space();
    let space_id = relation_space_id(&space);
    let relationships = actor.relations(maximum_relations).await?;
    let embedded = embedded_targets(actor, &space_id).await?;
    let mut report = RelationBuildReport {
        scanned: count(relationships.len())?,
        space_id: space_id.clone(),
        ..RelationBuildReport::default()
    };
    let mut sources = Vec::with_capacity(relationships.len());
    for relationship in &relationships {
        if embedded.contains(&relationship.event_lsn) {
            report.skipped = report.skipped.saturating_add(1);
            continue;
        }
        let mut support = BTreeSet::new();
        for citation in &relationship.citations {
            for lsn in [citation.first_lsn, citation.last_lsn] {
                if lsn != 0 {
                    support.insert(lsn);
                }
            }
        }
        sources.push(RelationSource {
            edge_id: relationship.edge_id.clone(),
            edge_lsn: relationship.event_lsn,
            source_id: relationship.source_id.clone(),
            target_id: relationship.target_id.clone(),
            relation: relationship.relation.clone(),
            support_lsns: support.into_iter().collect(),
        });
    }
    let plan = represent(
        &sources,
        RelationOptions {
            maximum_relations,
            maximum_text_bytes: MAXIMUM_REPRESENTATION_BYTES,
        },
    )?;
    report.skipped = report.skipped.saturating_add(count(plan.dropped.len())?);
    if plan.representations.is_empty() {
        return Ok(report);
    }
    let texts = plan
        .representations
        .iter()
        .map(|representation| representation.text.clone())
        .collect::<Vec<_>>();
    let embeddings = runtime.documents(texts).await?;
    if embeddings.len() != plan.representations.len() {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    let events = embedding_events(&plan.representations, embeddings, &space, &space_id)?;
    report.embedded = count(events.len())?;
    let outcome = actor.append(events).await?;
    report.first_lsn = outcome.first_lsn.get();
    report.last_lsn = outcome.last_lsn.get();
    Ok(report)
}

fn embedding_events(
    representations: &[RelationRepresentation],
    embeddings: Vec<QuantizedEmbedding>,
    space: &SpaceIdentity,
    space_id: &str,
) -> Result<Vec<IncomingEvent>, Error> {
    let model_id = format!("{}@{}", space.encoder_id, space.revision);
    let total =
        u32::try_from(embeddings.len()).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
    let mut events = Vec::with_capacity(embeddings.len());
    for (index, (representation, embedding)) in representations
        .iter()
        .zip(embeddings.into_iter())
        .enumerate()
    {
        let index = u32::try_from(index).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
        let dimension = u32::try_from(embedding.space.dimensions)
            .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
        events.push(IncomingEvent {
            kind: EventKind::Embedding,
            conversation: ConversationId::derive(RELATION_CONVERSATION),
            payload: encode_event_envelope(&EventEnvelope {
                schema_version: CURRENT_SCHEMA_VERSION,
                payload: EventPayload::Embedding(Box::new(Embedding {
                    target_lsn: representation.edge_lsn,
                    dimension,
                    quantized: embedding.values,
                    binary_prefilter: embedding.binary_prefilter,
                    space_id: space_id.to_owned(),
                })),
                connection_id: None,
                client_seq: 0,
                client_event_index: index,
                client_event_count: total,
                origin_actor: 0,
                run_id: None,
                model_provenance: Some(Box::new(ModelProvenance {
                    model_id: model_id.clone(),
                    prompt_id: RELATION_PROMPT.to_owned(),
                    prompt_version: 1,
                    temperature: 0.0,
                    call_id: None,
                    input_tokens: 0,
                    output_tokens: 0,
                    cache_read_tokens: 0,
                    cache_write_tokens: 0,
                    cost_microusd: 0,
                })),
                authority: Authority::DerivedInference,
                retention: Retention::Durable,
                sensitivity: Sensitivity::Public,
                event_time_ns: 0,
            }),
        });
    }
    Ok(events)
}

async fn embedded_targets(actor: &ActorEngine, space_id: &str) -> Result<BTreeSet<u64>, Error> {
    let mut targets = BTreeSet::new();
    for frame in actor.frames_since(LSN::new(0), None, usize::MAX).await? {
        if frame.header.kind != EventKind::Embedding {
            continue;
        }
        let verified = event::verify_event(
            &frame.sealed_payload,
            event::EventKind::Embedding,
            Boundary::Disk,
        )?;
        let EventPayload::Embedding(embedding) = verified.envelope.payload else {
            return Err(Error::new(ErrorCode::InvalidKind));
        };
        if embedding.space_id == space_id {
            targets.insert(embedding.target_lsn);
        }
    }
    Ok(targets)
}

fn count(value: usize) -> Result<u64, Error> {
    u64::try_from(value).map_err(|_| Error::new(ErrorCode::CapacityExceeded))
}
