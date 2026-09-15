#![allow(clippy::missing_errors_doc)]

use hm_compose::canonical::canonical_bytes;
use hm_compose::tokens::FallbackWeights;
use hm_core::{ActorId, ConversationId};
use hm_ledger::idempotency::ConnectionId;
use hm_mcp::{
    BeliefClaimInput, BeliefTypeInput, BelieveInput, BindInput, ClaimInput, IntendInput, McpServer,
    ProvenanceInput, RecallFilters, RecallInput, RecallMode, RememberAnchor, RememberInput,
    RememberKind, RetentionInput, RetractInput, SensitivityInput,
};
use hm_proj::beliefs::BeliefAsOf;
use hm_schema::events::BeliefType;
use hm_serve::actor::{ActivateRequest, ActorConfig, ActorEngine};
use napi::bindgen_prelude::Buffer;
use napi_derive::napi;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenConfig {
    actor: u16,
    user_hex: String,
    kek_hex: String,
    projection_map_bytes: usize,
}

#[derive(Deserialize)]
struct BindJson {
    #[serde(rename = "conversation")]
    _conversation: String,
    task: Option<String>,
    scope: Option<String>,
    canonical_entity: String,
    property: String,
    evidence_lsn: String,
    revision: String,
    freshness_requirement_ns: String,
}

#[derive(Deserialize)]
struct ProvenanceJson {
    first_lsn: String,
    last_lsn: String,
    byte_start: u32,
    byte_end: u32,
}

#[derive(Deserialize)]
struct BelieveJson {
    belief_id: String,
    belief_type: String,
    canonical_identity: String,
    value: String,
    valid_from_ns: String,
    valid_to_ns: String,
    provenance: Vec<ProvenanceJson>,
    conflict_domain: Option<String>,
    claim: Option<String>,
    run_id: Option<String>,
}

#[derive(Deserialize, Default)]
struct RememberOptionsJson {
    anchor: Option<RememberAnchor>,
    retention: Option<RetentionInput>,
    sensitivity: Option<SensitivityInput>,
}

#[napi]
pub struct NativeEngine {
    actor: ActorEngine,
}

#[napi]
impl NativeEngine {
    #[napi(factory)]
    pub async fn open(path: String, config_json: String) -> napi::Result<Self> {
        let config: OpenConfig = serde_json::from_str(&config_json).map_err(napi_error)?;
        if config.actor == 0 {
            return Err(napi::Error::from_reason("actor must be nonzero"));
        }
        let user = decode_array::<16>(&config.user_hex)?;
        let kek = decode_array::<32>(&config.kek_hex)?;
        let actor = ActorEngine::open(ActorConfig {
            actor_directory: std::path::Path::new(&path).join(config.actor.to_string()),
            actor: ActorId::new(config.actor),
            user,
            kek,
            projection_map_bytes: config.projection_map_bytes,
        })
        .await
        .map_err(napi_error)?;
        Ok(Self { actor })
    }

    #[napi]
    pub fn session(&self, conversation: String) -> napi::Result<NativeSession> {
        if conversation.is_empty() {
            return Err(napi::Error::from_reason("conversation is required"));
        }
        let connection_id = checkpoint_connection(&conversation);
        Ok(NativeSession {
            actor: self.actor.clone(),
            mcp: McpServer::new(self.actor.clone()),
            conversation,
            connection_id,
            checkpoint_lock: Arc::new(tokio::sync::Mutex::new(())),
        })
    }
}

#[napi]
pub struct NativeSession {
    actor: ActorEngine,
    mcp: McpServer,
    conversation: String,
    connection_id: ConnectionId,
    checkpoint_lock: Arc<tokio::sync::Mutex<()>>,
}

#[napi]
impl NativeSession {
    #[napi]
    pub async fn remember(
        &self,
        content: String,
        kind: String,
        options_json: String,
    ) -> napi::Result<String> {
        let kind = match kind.as_str() {
            "user" => RememberKind::User,
            "assistant" => RememberKind::Assistant,
            "document" => RememberKind::Document,
            _ => return Err(napi::Error::from_reason("invalid memory kind")),
        };
        let options: RememberOptionsJson =
            serde_json::from_str(&options_json).map_err(napi_error)?;
        encode_json(
            &self
                .mcp
                .remember_envelope(RememberInput {
                    conversation: self.conversation.clone(),
                    content,
                    kind,
                    chunk_bytes: None,
                    anchor: options.anchor,
                    retention: options.retention,
                    sensitivity: options.sensitivity,
                })
                .await,
        )
    }

    #[napi]
    pub async fn recall(
        &self,
        query: String,
        limit: u32,
        mode: String,
        filters_json: String,
    ) -> napi::Result<String> {
        let mode = match mode.as_str() {
            "semantic" => RecallMode::Semantic,
            "lexical" => RecallMode::Lexical,
            "entity" => RecallMode::Entity,
            "temporal" => RecallMode::Temporal,
            "near" => RecallMode::Near,
            _ => return Err(napi::Error::from_reason("invalid recall mode")),
        };
        let filters: RecallFilters = serde_json::from_str(&filters_json).map_err(napi_error)?;
        encode_json(
            &self
                .mcp
                .recall_envelope(RecallInput {
                    mode,
                    query,
                    conversation: String::new(),
                    limit: limit as usize,
                    since_lsn: 0,
                    filters,
                })
                .await,
        )
    }

    #[napi]
    pub async fn activate(&self, query: String, budget_tokens: u32) -> napi::Result<Buffer> {
        let bundle = self
            .actor
            .activate(ActivateRequest {
                conversation: ConversationId::derive(&self.conversation),
                query,
                turn_text: String::new(),
                budget_tokens: budget_tokens as usize,
                token_weights: FallbackWeights::default(),
            })
            .await
            .map_err(napi_error)?;
        Ok(canonical_bytes(&bundle).map_err(napi_error)?.into())
    }

    #[napi]
    pub async fn checkpoint(&self, turn_id: String, blob: Buffer) -> napi::Result<String> {
        let _guard = self.checkpoint_lock.lock().await;
        let sequence = self
            .actor
            .next_client_sequence(self.connection_id)
            .await
            .map_err(napi_error)?;
        let outcome = self
            .actor
            .write_checkpoint(
                self.connection_id,
                sequence,
                turn_id.into_bytes(),
                blob.to_vec(),
            )
            .await
            .map_err(napi_error)?;
        Ok(outcome.lsn.get().to_string())
    }

    #[napi]
    pub async fn intend(&self, input_json: String) -> napi::Result<String> {
        let mut input: IntendInput = serde_json::from_str(&input_json).map_err(napi_error)?;
        input.conversation.clone_from(&self.conversation);
        encode_json(&self.mcp.intend_envelope(input).await)
    }

    #[napi]
    pub async fn bind(&self, input_json: String) -> napi::Result<String> {
        let raw: BindJson = serde_json::from_str(&input_json).map_err(napi_error)?;
        let input = BindInput {
            conversation: self.conversation.clone(),
            task: raw.task,
            scope: raw.scope,
            canonical_entity: raw.canonical_entity,
            property: raw.property,
            evidence_lsn: raw.evidence_lsn.parse().map_err(napi_error)?,
            revision: raw.revision,
            freshness_requirement_ns: raw.freshness_requirement_ns.parse().map_err(napi_error)?,
        };
        encode_json(&self.mcp.bind_envelope(input).await)
    }

    #[napi]
    pub async fn believe(&self, input_json: String) -> napi::Result<String> {
        let raw: BelieveJson = serde_json::from_str(&input_json).map_err(napi_error)?;
        let input = BelieveInput {
            conversation: self.conversation.clone(),
            belief: BeliefClaimInput {
                belief_id: raw.belief_id,
                belief_type: belief_type_input(&raw.belief_type)?,
                canonical_identity: raw.canonical_identity,
                value: raw.value,
                valid_from_ns: raw.valid_from_ns.parse().map_err(napi_error)?,
                valid_to_ns: raw.valid_to_ns.parse().map_err(napi_error)?,
                provenance: provenance(raw.provenance)?,
                conflict_domain: raw.conflict_domain,
                claim: match raw.claim.as_deref().unwrap_or("affirmative") {
                    "affirmative" => ClaimInput::Affirmative,
                    "negative_existence" => ClaimInput::NegativeExistence,
                    _ => return Err(napi::Error::from_reason("invalid belief claim")),
                },
            },
            run_id: raw.run_id,
        };
        encode_json(&self.mcp.believe_envelope(input).await)
    }

    #[napi]
    pub async fn retract(
        &self,
        belief_id: String,
        provenance_json: String,
    ) -> napi::Result<String> {
        let raw: Vec<ProvenanceJson> =
            serde_json::from_str(&provenance_json).map_err(napi_error)?;
        encode_json(
            &self
                .mcp
                .retract_envelope(RetractInput {
                    conversation: self.conversation.clone(),
                    belief_id,
                    provenance: provenance(raw)?,
                })
                .await,
        )
    }

    #[napi]
    pub async fn as_of(
        &self,
        belief_type: String,
        canonical_identity: String,
        valid_at_ns: Option<String>,
        known_at_lsn: Option<String>,
    ) -> napi::Result<String> {
        let as_of = match (valid_at_ns, known_at_lsn) {
            (Some(value), None) => BeliefAsOf::ValidAt(value.parse().map_err(napi_error)?),
            (None, Some(value)) => {
                BeliefAsOf::KnownAt(hm_core::LSN::new(value.parse().map_err(napi_error)?))
            }
            _ => return Err(napi::Error::from_reason("exactly one as-of axis is required")),
        };
        let result = self
            .actor
            .as_of(
                belief_type_value(&belief_type)?,
                canonical_identity,
                as_of,
            )
            .await
            .map_err(napi_error)?;
        let Some(record) = result.record else {
            return Ok("null".to_owned());
        };
        encode_json(&serde_json::json!({
            "beliefId": String::from_utf8_lossy(&record.belief_id),
            "beliefType": belief_type_name(record.belief_type),
            "canonicalIdentity": record.canonical_identity,
            "conflictDomain": (!record.conflict_domain.is_empty()).then_some(record.conflict_domain),
            "value": String::from_utf8_lossy(&record.value),
            "claim": if record.claim == hm_schema::events::AssertionClaim::NegativeExistence {
                "negative_existence"
            } else {
                "affirmative"
            },
            "validFromNs": record.valid_from_ns.to_string(),
            "validToNs": record.valid_to_ns.to_string(),
            "transactionLsn": record.observation_lsn.to_string(),
            "version": record.version.to_string(),
            "supersedesVersion": record.supersedes_version.to_string(),
            "provenance": record.provenance.into_iter().map(|range| serde_json::json!({
                "firstLsn": range.first_lsn.to_string(),
                "lastLsn": range.last_lsn.to_string(),
                "byteStart": range.byte_start,
                "byteEnd": range.byte_end,
            })).collect::<Vec<_>>(),
            "conflicts": record.conflict_edges.into_iter().map(|edge| serde_json::json!({
                "otherType": belief_type_name(edge.other_type),
                "otherCanonicalIdentity": edge.other_canonical_identity,
                "createdLsn": edge.created_lsn.to_string(),
                "resolvedLsn": edge.resolved_lsn.to_string(),
                "obligatedSurfacing": edge.obligated_surfacing,
            })).collect::<Vec<_>>(),
        }))
    }
}

fn provenance(values: Vec<ProvenanceJson>) -> napi::Result<Vec<ProvenanceInput>> {
    values
        .into_iter()
        .map(|value| {
            Ok(ProvenanceInput {
                first_lsn: value.first_lsn.parse().map_err(napi_error)?,
                last_lsn: value.last_lsn.parse().map_err(napi_error)?,
                byte_start: value.byte_start,
                byte_end: value.byte_end,
            })
        })
        .collect()
}

fn belief_type_input(value: &str) -> napi::Result<BeliefTypeInput> {
    Ok(match value {
        "fact" => BeliefTypeInput::Fact,
        "preference" => BeliefTypeInput::Preference,
        "constraint" => BeliefTypeInput::Constraint,
        "goal" => BeliefTypeInput::Goal,
        "identity" => BeliefTypeInput::Identity,
        _ => return Err(napi::Error::from_reason("invalid belief type")),
    })
}

fn belief_type_value(value: &str) -> napi::Result<BeliefType> {
    Ok(match belief_type_input(value)? {
        BeliefTypeInput::Fact => BeliefType::Fact,
        BeliefTypeInput::Preference => BeliefType::Preference,
        BeliefTypeInput::Constraint => BeliefType::Constraint,
        BeliefTypeInput::Goal => BeliefType::Goal,
        BeliefTypeInput::Identity => BeliefType::Identity,
    })
}

const fn belief_type_name(value: BeliefType) -> &'static str {
    match value {
        BeliefType::Fact => "fact",
        BeliefType::Preference => "preference",
        BeliefType::Constraint => "constraint",
        BeliefType::Goal => "goal",
        BeliefType::Identity => "identity",
    }
}

fn checkpoint_connection(conversation: &str) -> ConnectionId {
    let mut hasher = Sha256::new();
    hasher.update(b"hypermind-napi-checkpoint-v1\0");
    hasher.update(conversation.as_bytes());
    let digest = hasher.finalize();
    let mut result = [0; 16];
    result.copy_from_slice(&digest[..16]);
    result
}

fn decode_array<const N: usize>(value: &str) -> napi::Result<[u8; N]> {
    if value.len() != N * 2 {
        return Err(napi::Error::from_reason("invalid hex length"));
    }
    let mut output = [0; N];
    for (index, byte) in output.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).map_err(napi_error)?;
    }
    Ok(output)
}

fn encode_json(value: &impl serde::Serialize) -> napi::Result<String> {
    serde_json::to_string(value).map_err(napi_error)
}

fn napi_error(error: impl std::fmt::Display) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}
