#![allow(clippy::missing_errors_doc)]

use crate::actor::{
    ActivateRequest, ActorConfig, ActorEngine, IncomingEvent, RecallItem, RecallRequest,
};
use hm_compose::bundle::{ActivationBundle, Tier};
use hm_compose::tokens::FallbackWeights;
use hm_core::{ActorId, ConversationId, Error, ErrorCode, LSN};
use hm_ledger::frame::EventKind;
use hm_ledger::keyring::{KeyEncryptionKey, UserId};
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    Authority, DeliveredMsg, EventEnvelope, EventPayload, Retention, Sensitivity, UserMsg,
};
use std::path::Path;

#[derive(Clone, Debug)]
pub struct EmbeddedConfig {
    pub actor: ActorId,
    pub user: UserId,
    pub kek: KeyEncryptionKey,
    pub projection_map_bytes: usize,
}

#[derive(Clone)]
pub struct HyperMind {
    actor: Actor,
}

#[derive(Clone)]
pub struct Actor {
    engine: ActorEngine,
}

#[derive(Clone)]
pub struct Session {
    actor: Actor,
    conversation: ConversationId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryKind {
    User,
    Assistant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderModel {
    OpenAi,
    Anthropic,
    PlainText,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderAuthority {
    UntrustedMemory,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderedItem {
    pub tier: Tier,
    pub role: &'static str,
    pub authority: RenderAuthority,
    pub source_authority: Authority,
    pub provenance_uri: String,
    pub provenance: Vec<LSN>,
    pub content: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderedSection {
    pub tier: Tier,
    pub items: Vec<RenderedItem>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderedBundle {
    pub model: RenderModel,
    pub sections: Vec<RenderedSection>,
    pub bundle_hash: [u8; 32],
}

impl HyperMind {
    pub async fn open(path: impl AsRef<Path>, config: EmbeddedConfig) -> Result<Self, Error> {
        let engine = ActorEngine::open(ActorConfig {
            actor_directory: path.as_ref().join(config.actor.get().to_string()),
            actor: config.actor,
            user: config.user,
            kek: config.kek,
            projection_map_bytes: config.projection_map_bytes,
        })
        .await?;
        Ok(Self {
            actor: Actor { engine },
        })
    }

    #[must_use]
    pub fn actor(&self) -> Actor {
        self.actor.clone()
    }

    #[must_use]
    pub fn session(&self, conversation: impl AsRef<str>) -> Session {
        self.actor.session(conversation)
    }
}

impl Actor {
    #[must_use]
    pub fn id(&self) -> ActorId {
        self.engine.actor()
    }

    #[must_use]
    pub fn session(&self, conversation: impl AsRef<str>) -> Session {
        Session {
            actor: self.clone(),
            conversation: ConversationId::derive(conversation.as_ref()),
        }
    }

    pub async fn shutdown(self) -> Result<(), Error> {
        self.engine.shutdown().await
    }
}

impl Session {
    #[must_use]
    pub const fn conversation(&self) -> ConversationId {
        self.conversation
    }

    pub async fn remember(&self, kind: MemoryKind, content: impl AsRef<str>) -> Result<LSN, Error> {
        let content = content.as_ref();
        if content.is_empty() {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let (kind, payload, authority) = match kind {
            MemoryKind::User => (
                EventKind::UserMsg,
                EventPayload::UserMsg(Box::new(UserMsg {
                    content: content.as_bytes().to_vec(),
                })),
                Authority::UserAsserted,
            ),
            MemoryKind::Assistant => (
                EventKind::DeliveredMsg,
                EventPayload::DeliveredMsg(Box::new(DeliveredMsg {
                    content: content.as_bytes().to_vec(),
                })),
                Authority::AssistantGenerated,
            ),
        };
        let payload = encode_event_envelope(&EventEnvelope {
            schema_version: CURRENT_SCHEMA_VERSION,
            payload,
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 0,
            origin_actor: 0,
            run_id: None,
            model_provenance: None,
            authority,
            retention: Retention::Durable,
            sensitivity: Sensitivity::Personal,
            event_time_ns: 0,
        });
        self.actor
            .engine
            .append(vec![IncomingEvent {
                kind,
                conversation: self.conversation,
                payload,
            }])
            .await
            .map(|outcome| outcome.first_lsn)
    }

    pub async fn recall(
        &self,
        query: impl Into<String>,
        limit: usize,
    ) -> Result<Vec<RecallItem>, Error> {
        self.actor
            .engine
            .recall(RecallRequest::Lexical {
                query: query.into(),
                limit,
            })
            .await
    }

    pub async fn timeline(&self, since_lsn: LSN, limit: usize) -> Result<Vec<RecallItem>, Error> {
        self.actor
            .engine
            .recall(RecallRequest::Timeline {
                conversation: self.conversation,
                since_lsn,
                limit,
            })
            .await
    }

    pub async fn activate(
        &self,
        query: impl Into<String>,
        budget_tokens: usize,
    ) -> Result<ActivationBundle, Error> {
        self.actor
            .engine
            .activate(ActivateRequest {
                conversation: self.conversation,
                query: query.into(),
                turn_text: String::new(),
                budget_tokens,
                token_weights: FallbackWeights::default(),
            })
            .await
    }
}

pub fn render(bundle: &ActivationBundle, model: RenderModel) -> Result<RenderedBundle, Error> {
    let mut sections = Vec::new();
    for section in &bundle.sections {
        let mut rendered = RenderedSection {
            tier: section.tier,
            items: Vec::with_capacity(section.items.len()),
        };
        for item in &section.items {
            if item.provenance.is_empty() || !item.uri.starts_with("hm://") {
                return Err(Error::new(ErrorCode::CitationInvalid));
            }
            rendered.items.push(RenderedItem {
                tier: item.tier,
                role: "user",
                authority: RenderAuthority::UntrustedMemory,
                source_authority: item.authority,
                provenance_uri: item.uri.clone(),
                provenance: item.provenance.clone(),
                content: String::from_utf8(item.content.clone())
                    .map_err(|_| Error::new(ErrorCode::SchemaInvalid))?,
            });
        }
        sections.push(rendered);
    }
    Ok(RenderedBundle {
        model,
        sections,
        bundle_hash: bundle.bundle_hash,
    })
}
