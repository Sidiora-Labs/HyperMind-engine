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
    context_scope: Option<hm_context::Scope>,
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
            actor: Actor { engine, context_scope: None },
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
        hm_compose::reconstruct::guard_remember(content)?;
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
                source_authority: if hm_compose::reconstruct::is_reconstruction(
                    &String::from_utf8_lossy(&item.content),
                ) {
                    Authority::AssistantGenerated
                } else {
                    item.authority
                },
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


pub fn validate_context_config(config: &crate::context_config::TrustedContextConfig, actor: ActorId) -> Result<(), Error> {
    if config.version != crate::context_config::CONTEXT_CONFIG_VERSION {
        return Err(Error::new(ErrorCode::SchemaVersion));
    }
    if actor.get() == 0 || config.actor != actor.get() {
        return Err(Error::new(ErrorCode::CapabilityDenied));
    }
    config.scope.validate().map_err(|_| Error::new(ErrorCode::InvalidArgument))
}

#[derive(Clone)]
pub struct ContextSession {
    actor: Actor,
    session_id: String,
    conversation: String,
    scope: hm_context::Scope,
}

impl HyperMind {
    pub async fn open_with_context(path: impl AsRef<Path>, config: EmbeddedConfig, context: crate::context_config::TrustedContextConfig) -> Result<Self, Error> {
        validate_context_config(&context, config.actor)?;
        let mut engine = Self::open(path, config).await?;
        engine.actor.context_scope = Some(context.scope);
        Ok(engine)
    }
}
impl Actor {
    pub fn context_session(&self, session_id: impl Into<String>, conversation: impl Into<String>) -> Result<ContextSession, Error> {
        let scope = self.context_scope.clone().ok_or_else(|| Error::new(ErrorCode::CapabilityDenied))?;
        let session_id = session_id.into();
        let conversation = conversation.into();
        hm_context::validate_id(&session_id).and_then(|()| hm_context::validate_id(&conversation)).map_err(|_| Error::new(ErrorCode::InvalidArgument))?;
        Ok(ContextSession { actor: self.clone(), session_id, conversation, scope })
    }
}
impl ContextSession {
    pub fn scope(&self) -> &hm_context::Scope { &self.scope }
    pub fn session_id(&self) -> &str { &self.session_id }
    pub async fn source(&self, message: hm_context::SourceMessage, original_bytes: Vec<u8>) -> Result<crate::context_history::HistoryReceipt, crate::context_history::HistoryError> {
        crate::context_history::ingest(&self.actor.engine, &self.scope, &crate::context_history::SourceIngestion { version: hm_context::CONTRACT_VERSION, scope: self.scope.clone(), session_id: self.session_id.clone(), conversation: self.conversation.clone(), message, original_bytes }).await
    }
    pub async fn relate(&self, relation: hm_context::history::SourceRelation) -> Result<crate::context_history::HistoryReceipt, crate::context_history::HistoryError> {
        crate::context_history::relate(&self.actor.engine, &self.scope, &crate::context_history::RelationIngestion { version: hm_context::CONTRACT_VERSION, scope: self.scope.clone(), session_id: self.session_id.clone(), conversation: self.conversation.clone(), relation }).await
    }
    pub async fn fork(&self, child_session_id: impl Into<String>, child_conversation: impl Into<String>) -> Result<ContextSession, crate::context_history::HistoryError> {
        let child_session_id = child_session_id.into();
        let child_conversation = child_conversation.into();
        crate::context_history::fork(&self.actor.engine, &self.scope, &crate::context_history::ForkRequest { version: hm_context::CONTRACT_VERSION, scope: self.scope.clone(), parent_session_id: self.session_id.clone(), parent_conversation: self.conversation.clone(), child_session_id: child_session_id.clone(), child_conversation: child_conversation.clone() }).await?;
        Ok(ContextSession { actor: self.actor.clone(), session_id: child_session_id, conversation: child_conversation, scope: self.scope.clone() })
    }
    pub async fn activate(&self, request: crate::session_context::SessionContextRequest) -> Result<serde_json::Value, Error> {
        if request.session_id != self.session_id { return Err(Error::new(ErrorCode::InvalidArgument)); }
        crate::session_context::activate(&self.actor.engine, &self.conversation, &self.scope, request).await
    }
    pub async fn inspect(&self) -> Result<serde_json::Value, Error> {
        crate::session_context::inspect(&self.actor.engine, &self.scope, Some(&self.session_id)).await
    }
    pub async fn history(&self) -> Result<crate::context_history::LedgerHistory, crate::context_history::HistoryError> {
        crate::context_history::replay(&self.actor.engine, &self.scope, &self.session_id, &self.conversation).await
    }
    pub async fn recover(&self, span: &hm_context::SourceSpan) -> Result<Vec<u8>, crate::context_history::HistoryError> {
        crate::context_history::recover(&self.actor.engine, &self.scope, &self.session_id, &self.conversation, span).await
    }
    pub async fn job(&self, request_id: impl Into<String>, action: crate::context_jobs::ContextJobAction) -> Result<serde_json::Value, Error> {
        crate::context_jobs::execute(&self.actor.engine, &self.scope, &self.scope.owner_id, crate::context_jobs::ContextJobRequest { version: hm_context::CONTRACT_VERSION, scope: self.scope.clone(), request_id: request_id.into(), action }).await
    }
    pub async fn import(&self, bundle: &crate::hypermid_import::ImportBundle, max_entries: usize) -> Result<crate::hypermid_import::ImportReceipt, crate::hypermid_import::ImportError> {
        if bundle.scope != self.scope || max_entries == 0 || max_entries > 256 { return Err(hm_context::ContextError::ScopeMismatch.into()); }
        let mut registry = hm_context::temporal::IdentityRegistry::new();
        registry.bind(self.scope.clone(), self.actor.id().get(), Vec::new())?;
        crate::hypermid_import::import_batch(&self.actor.engine, &registry, bundle, max_entries).await
    }
}
