#![allow(clippy::missing_errors_doc)]

use hm_compose::canonical::canonical_bytes;
use hm_compose::tokens::FallbackWeights;
use hm_core::{ActorId, ConversationId};
use hm_ledger::idempotency::ConnectionId;
use hm_mcp::{
    BindInput, IntendInput, McpServer, RecallInput, RecallMode, RememberInput, RememberKind,
};
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
    pub async fn remember(&self, content: String, kind: String) -> napi::Result<String> {
        let kind = match kind.as_str() {
            "user" => RememberKind::User,
            "assistant" => RememberKind::Assistant,
            _ => return Err(napi::Error::from_reason("invalid memory kind")),
        };
        encode_json(
            &self
                .mcp
                .remember_envelope(RememberInput {
                    conversation: self.conversation.clone(),
                    content,
                    kind,
                    chunk_bytes: None,
                })
                .await,
        )
    }

    #[napi]
    pub async fn recall(&self, query: String, limit: u32) -> napi::Result<String> {
        encode_json(
            &self
                .mcp
                .recall_envelope(RecallInput {
                    mode: RecallMode::Lexical,
                    query,
                    conversation: String::new(),
                    limit: limit as usize,
                    since_lsn: 0,
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
