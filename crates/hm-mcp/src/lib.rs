#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc)]

use base64::Engine as _;
use hm_compose::bundle::{ActivationBundle, HealthStatus};
use hm_compose::tokens::FallbackWeights;
use hm_core::{ConversationId, Error, ErrorCode, LSN};
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope, verify_event};
use hm_schema::events::{
    Authority, DeliveredMsg, EventEnvelope, EventPayload, Retention, Sensitivity, UserMsg,
};
use hm_serve::actor::{
    ActivateRequest as ActorActivateRequest, ActorEngine, IncomingEvent, RecallRequest,
};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{Json, ServiceExt, schemars, tool, tool_router, transport::stdio};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub mod tools;
pub use tools::bind::BindInput;
pub use tools::forget::{ForgetAction, ForgetInput};
pub use tools::inspect::InspectInput;
pub use tools::intend::{IntendAction, IntendCloseReason, IntendInput};
pub use tools::recall::{RecallFilters, RecallInput, RecallMode};
pub use tools::remember::{
    AnchorFacet, RememberAnchor, RememberInput, RememberKind, RetentionInput, SensitivityInput,
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_CHUNK_BYTES: usize = 32 * 1024;

#[derive(Clone, Debug, Serialize, schemars::JsonSchema)]
pub struct Envelope {
    pub ok: bool,
    pub items: Vec<Value>,
    pub provenance: Vec<String>,
    pub budget: Option<Value>,
    pub gaps: Vec<Value>,
    pub health: Value,
    pub warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect_state: Option<String>,
}

impl Envelope {
    pub(crate) fn empty() -> Self {
        Self {
            ok: true,
            items: Vec::new(),
            provenance: Vec::new(),
            budget: None,
            gaps: Vec::new(),
            health: json!({"projection": "ready"}),
            warnings: Vec::new(),
            effect_state: None,
        }
    }

    pub(crate) fn error(error: Error, mutation: bool) -> Self {
        let mut envelope = Self::empty();
        envelope.ok = false;
        envelope.items.push(json!({
            "error": error.code.as_str(),
            "system_error": error.system_error,
            "lsn": error.lsn.get(),
            "offset": error.offset,
        }));
        envelope.health = json!({"projection": "unavailable"});
        if mutation {
            envelope.effect_state = Some(
                hm_serve::errors::mutation_effect_state_name(
                    hm_serve::errors::mutation_effect_state(error),
                )
                .to_owned(),
            );
        }
        envelope
    }

    pub(crate) fn security_error(error: Error) -> Self {
        let mut envelope = Self::error(error, true);
        envelope
            .warnings
            .push(format!("security_event:tripwire:lsn={}", error.lsn.get()));
        envelope
    }
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct ActivateInput {
    pub conversation: String,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub turn_text: String,
    pub budget_tokens: usize,
}

#[derive(Clone)]
pub struct McpServer {
    actor: ActorEngine,
    admin_token: Option<hm_serve::config::CapabilityToken>,
}

impl McpServer {
    #[must_use]
    pub const fn new(actor: ActorEngine) -> Self {
        Self {
            actor,
            admin_token: None,
        }
    }

    #[must_use]
    pub const fn new_with_admin(
        actor: ActorEngine,
        admin_token: hm_serve::config::CapabilityToken,
    ) -> Self {
        Self {
            actor,
            admin_token: Some(admin_token),
        }
    }

    pub async fn remember_envelope(&self, input: RememberInput) -> Envelope {
        match self.remember_inner(input).await {
            Ok(value) => value,
            Err(error) => Envelope::error(error, true),
        }
    }

    #[allow(clippy::too_many_lines)]
    async fn remember_inner(&self, input: RememberInput) -> Result<Envelope, Error> {
        if input.conversation.is_empty() || input.content.is_empty() {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        if input
            .anchor
            .as_ref()
            .is_some_and(|anchor| anchor.value.is_empty() || anchor.value.len() > 4096)
        {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let retention: Retention = input.retention.unwrap_or(RetentionInput::Durable).into();
        let sensitivity: Sensitivity = input
            .sensitivity
            .unwrap_or(SensitivityInput::Personal)
            .into();
        if retention == Retention::DoNotStore {
            let mut envelope = Envelope::empty();
            envelope.items.push(json!({
                "stored": false,
                "retention": "do_not_store",
                "receipt": hex(blake3::hash(input.content.as_bytes()).as_bytes()),
            }));
            return Ok(envelope);
        }
        let conversation = ConversationId::derive(&input.conversation);
        let (kind, authority) = match input.kind {
            RememberKind::User => (
                hm_ledger::frame::EventKind::UserMsg,
                Authority::UserAsserted,
            ),
            RememberKind::Assistant => (
                hm_ledger::frame::EventKind::DeliveredMsg,
                Authority::AssistantGenerated,
            ),
            RememberKind::Document => (
                hm_ledger::frame::EventKind::UserMsg,
                Authority::ExternalObserved,
            ),
        };
        let chunks = if matches!(input.kind, RememberKind::Document) {
            chunk_text(
                &input.content,
                input.chunk_bytes.unwrap_or(DEFAULT_CHUNK_BYTES),
            )?
        } else {
            vec![input.content.as_str()]
        };
        let event_count =
            u32::try_from(chunks.len()).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
        let mut events = Vec::with_capacity(chunks.len());
        for (index, chunk) in chunks.into_iter().enumerate() {
            let payload = match kind {
                hm_ledger::frame::EventKind::UserMsg => EventPayload::UserMsg(Box::new(UserMsg {
                    content: chunk.as_bytes().to_vec(),
                })),
                hm_ledger::frame::EventKind::DeliveredMsg => {
                    EventPayload::DeliveredMsg(Box::new(DeliveredMsg {
                        content: chunk.as_bytes().to_vec(),
                    }))
                }
                _ => return Err(Error::new(ErrorCode::InvariantViolation)),
            };
            events.push(IncomingEvent {
                kind,
                conversation,
                payload: encode_event_envelope(&EventEnvelope {
                    schema_version: CURRENT_SCHEMA_VERSION,
                    payload,
                    connection_id: None,
                    client_seq: 0,
                    client_event_index: u32::try_from(index)
                        .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?,
                    client_event_count: event_count,
                    origin_actor: 0,
                    run_id: None,
                    model_provenance: None,
                    authority,
                    retention,
                    sensitivity,
                    event_time_ns: 0,
                }),
            });
        }
        let outcome = self.actor.append(events).await?;
        let mut envelope = Envelope::empty();
        envelope.items.push(json!({
            "first_lsn": outcome.first_lsn.get(),
            "last_lsn": outcome.last_lsn.get(),
            "count": outcome.last_lsn.get() - outcome.first_lsn.get() + 1,
            "anchor": input.anchor.as_ref().map(|anchor| json!({
                "facet": format!("{:?}", anchor.facet).to_lowercase(),
                "value": anchor.value,
            })),
        }));
        for lsn in outcome.first_lsn.get()..=outcome.last_lsn.get() {
            envelope
                .provenance
                .push(format!("hm://{}/lsn/{lsn}", self.actor.actor()));
        }
        Ok(envelope)
    }

    pub async fn recall_envelope(&self, input: RecallInput) -> Envelope {
        match self.recall_inner(input).await {
            Ok(value) => value,
            Err(error) => Envelope::error(error, false),
        }
    }

    async fn recall_inner(&self, input: RecallInput) -> Result<Envelope, Error> {
        if input.limit == 0 || input.limit > 4096 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let request = match input.mode {
            RecallMode::Semantic if !input.query.is_empty() => RecallRequest::Semantic {
                query: input.query.clone(),
                limit: input.limit,
            },
            RecallMode::Lexical if !input.query.is_empty() => RecallRequest::Lexical {
                query: input.query.clone(),
                limit: input.limit,
            },
            RecallMode::Entity
                if !input.query.is_empty() || !input.filters.turn_text.is_empty() =>
            {
                RecallRequest::Entity {
                    query: input.query.clone(),
                    turn_text: input.filters.turn_text.clone(),
                    limit: input.limit,
                }
            }
            RecallMode::Near
                if input
                    .filters
                    .anchor
                    .as_ref()
                    .is_some_and(|value| !value.is_empty()) =>
            {
                RecallRequest::Near {
                    anchor: input.filters.anchor.clone().expect("checked anchor"),
                    query: input.query.clone(),
                    turn_text: input.filters.turn_text.clone(),
                    limit: input.limit,
                }
            }
            RecallMode::Temporal => RecallRequest::Temporal {
                start_ns: input.filters.temporal_from_ns.unwrap_or(i64::MIN),
                end_ns: input.filters.temporal_to_ns.unwrap_or(i64::MAX),
                limit: input.limit,
            },
            RecallMode::Timeline if !input.conversation.is_empty() => RecallRequest::Timeline {
                conversation: ConversationId::derive(&input.conversation),
                since_lsn: LSN::new(input.since_lsn),
                limit: input.limit,
            },
            _ => return Err(Error::new(ErrorCode::InvalidArgument)),
        };
        let conversation_filter = input
            .filters
            .conversation
            .as_deref()
            .filter(|value| !value.is_empty())
            .map(ConversationId::derive);
        let since_lsn = input.filters.since_lsn.unwrap_or(input.since_lsn);
        let until_lsn = input.filters.until_lsn.unwrap_or(u64::MAX);
        if since_lsn > until_lsn {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let records = self
            .actor
            .recall(request)
            .await?
            .into_iter()
            .filter(|record| {
                record.lsn.get() > since_lsn
                    && record.lsn.get() <= until_lsn
                    && conversation_filter.is_none_or(|value| value == record.conversation)
            });
        let mut envelope = Envelope::empty();
        for record in records {
            let (content, authority) = event_content(record.kind, &record.payload)?;
            let uri = format!(
                "hm://{}/{}/{}?at={}&src={}&score={}",
                self.actor.actor(),
                record.conversation,
                record.lsn,
                record.wall_timestamp_ns,
                record.kind as u8,
                record.score_q32,
            );
            envelope.items.push(json!({
                "lsn": record.lsn.get(),
                "kind": record.kind as u8,
                "content": content,
                "authority": authority_name(authority),
                "score_q32": record.score_q32,
                "uri": uri,
            }));
            envelope.provenance.push(uri);
        }
        Ok(envelope)
    }

    pub async fn activate_envelope(&self, input: ActivateInput) -> Envelope {
        match self.activate_inner(input).await {
            Ok(value) => value,
            Err(error) if error.code == ErrorCode::Tripwire => Envelope::security_error(error),
            Err(error) => Envelope::error(error, false),
        }
    }

    async fn activate_inner(&self, input: ActivateInput) -> Result<Envelope, Error> {
        if input.conversation.is_empty() || input.budget_tokens == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let bundle = self
            .actor
            .activate(ActorActivateRequest {
                conversation: ConversationId::derive(&input.conversation),
                query: input.query,
                turn_text: input.turn_text,
                budget_tokens: input.budget_tokens,
                token_weights: FallbackWeights::default(),
            })
            .await?;
        Ok(bundle_envelope(&bundle))
    }

    pub async fn inspect_envelope(&self, input: InspectInput) -> Envelope {
        match tools::inspect::run(&self.actor, input).await {
            Ok(value) => value,
            Err(error) => Envelope::error(error, false),
        }
    }

    pub async fn forget_envelope(&self, input: ForgetInput) -> Envelope {
        match tools::forget::run(&self.actor, self.admin_token.as_ref(), input).await {
            Ok(value) => value,
            Err(error) if error.code == ErrorCode::Tripwire => Envelope::security_error(error),
            Err(error) => Envelope::error(error, true),
        }
    }

    pub async fn intend_envelope(&self, input: IntendInput) -> Envelope {
        match tools::intend::run(&self.actor, input).await {
            Ok(value) => value,
            Err(error) => Envelope::error(error, true),
        }
    }

    pub async fn bind_envelope(&self, input: BindInput) -> Envelope {
        match tools::bind::run(&self.actor, input).await {
            Ok(value) => value,
            Err(error) => Envelope::error(error, true),
        }
    }
}

#[tool_router(server_handler)]
impl McpServer {
    #[tool(
        description = "Persist a user message, delivered assistant message, or chunked document with optional anchor, retention, and sensitivity"
    )]
    async fn remember(&self, Parameters(input): Parameters<RememberInput>) -> Json<Envelope> {
        Json(self.remember_envelope(input).await)
    }

    #[tool(
        description = "Recall by semantic, lexical, entity, temporal, near-anchor, or timeline mode with filters"
    )]
    async fn recall(&self, Parameters(input): Parameters<RecallInput>) -> Json<Envelope> {
        Json(self.recall_envelope(input).await)
    }

    #[tool(description = "Compose a deterministic, budgeted activation bundle")]
    async fn activate(&self, Parameters(input): Parameters<ActivateInput>) -> Json<Envelope> {
        Json(self.activate_envelope(input).await)
    }

    #[tool(description = "Inspect actor ledger, projection checkpoints, and applied-state digest")]
    async fn inspect(&self, Parameters(input): Parameters<InspectInput>) -> Json<Envelope> {
        Json(self.inspect_envelope(input).await)
    }

    #[tool(description = "Fade a memory, retract a run, or crypto-shred the actor")]
    async fn forget(&self, Parameters(input): Parameters<ForgetInput>) -> Json<Envelope> {
        Json(self.forget_envelope(input).await)
    }

    #[tool(description = "Set an objective, open a work loop, or close a work loop")]
    async fn intend(&self, Parameters(input): Parameters<IntendInput>) -> Json<Envelope> {
        Json(self.intend_envelope(input).await)
    }

    #[tool(description = "Bind a task or scope to a canonical entity revision")]
    async fn bind(&self, Parameters(input): Parameters<BindInput>) -> Json<Envelope> {
        Json(self.bind_envelope(input).await)
    }
}

pub async fn serve_stdio(server: McpServer) -> Result<(), Box<dyn std::error::Error>> {
    server.serve(stdio()).await?.waiting().await?;
    Ok(())
}

fn bundle_envelope(bundle: &ActivationBundle) -> Envelope {
    let mut envelope = Envelope::empty();
    for section in &bundle.sections {
        for item in &section.items {
            envelope.items.push(json!({
                "tier": format!("{:?}", item.tier).to_lowercase(),
                "uri": item.uri,
                "content_base64": base64::engine::general_purpose::STANDARD.encode(&item.content),
                "authority": authority_name(item.authority),
                "tokens": item.tokens,
                "coarsened": item.coarsened,
            }));
            envelope.provenance.push(item.uri.clone());
        }
    }
    envelope.budget = Some(json!({
        "limit_tokens": bundle.budget_tokens,
        "spent_tokens": bundle.spent_tokens,
    }));
    envelope.gaps = bundle
        .gaps
        .iter()
        .map(|gap| json!({"kind": format!("{:?}", gap.kind).to_lowercase(), "detail": gap.detail}))
        .collect();
    envelope.health = json!({
        "encoder": health(bundle.health.encoder),
        "backlog": health(bundle.health.backlog),
        "projection": health(bundle.health.projection),
        "inclusion": health(bundle.health.inclusion),
        "bundle_hash": hex(&bundle.bundle_hash),
    });
    envelope
}

fn event_content(
    kind: hm_ledger::frame::EventKind,
    payload: &[u8],
) -> Result<(String, Authority), Error> {
    let schema_kind = hm_schema::event::EventKind::try_from(kind as u8)
        .map_err(|()| Error::new(ErrorCode::InvalidKind))?;
    let verified = verify_event(payload, schema_kind, hm_schema::event::Boundary::Disk)?;
    let authority = verified.envelope.authority;
    let bytes = match verified.envelope.payload {
        EventPayload::UserMsg(message) => message.content,
        EventPayload::DeliveredMsg(message) => message.content,
        _ => return Ok((String::new(), authority)),
    };
    String::from_utf8(bytes)
        .map(|content| (content, authority))
        .map_err(|_| Error::new(ErrorCode::SchemaInvalid))
}

const fn authority_name(authority: Authority) -> &'static str {
    match authority {
        Authority::UserAsserted => "user_asserted",
        Authority::ExternalObserved => "external_observed",
        Authority::ToolObserved => "tool_observed",
        Authority::RuntimeFact => "runtime_fact",
        Authority::AssistantGenerated => "assistant_generated",
        Authority::DerivedInference => "derived_inference",
    }
}

fn chunk_text(text: &str, maximum_bytes: usize) -> Result<Vec<&str>, Error> {
    if maximum_bytes == 0 || maximum_bytes > hm_schema::event::MAXIMUM_EVENT_BYTES {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < text.len() {
        let mut end = (start + maximum_bytes).min(text.len());
        while end > start && !text.is_char_boundary(end) {
            end -= 1;
        }
        if end == start {
            end = text[start..]
                .char_indices()
                .nth(1)
                .map_or(text.len(), |(offset, _)| start + offset);
        }
        chunks.push(&text[start..end]);
        start = end;
    }
    Ok(chunks)
}

const fn health(status: HealthStatus) -> &'static str {
    match status {
        HealthStatus::SemanticReady => "semantic_ready",
        HealthStatus::SemanticLagging => "semantic_lagging",
        HealthStatus::LexicalOnly => "lexical_only",
        HealthStatus::Unavailable => "unavailable",
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rmcp_router_exposes_wave_one_tools() {
        let names: std::collections::BTreeSet<_> = McpServer::tool_router()
            .list_all()
            .into_iter()
            .map(|tool| tool.name.into_owned())
            .collect();
        assert_eq!(
            names,
            [
                "activate", "bind", "forget", "inspect", "intend", "recall", "remember"
            ]
            .map(str::to_owned)
            .into()
        );
    }
}
