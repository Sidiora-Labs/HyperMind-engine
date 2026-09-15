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
    fn empty() -> Self {
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

    fn error(error: Error, mutation: bool) -> Self {
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
                if matches!(
                    error.code,
                    ErrorCode::InvalidArgument
                        | ErrorCode::InvalidLength
                        | ErrorCode::SchemaInvalid
                        | ErrorCode::SchemaVersion
                        | ErrorCode::ForbiddenKind
                        | ErrorCode::OrderingViolation
                        | ErrorCode::ProtectedTypeWrite
                ) {
                    "rejected"
                } else {
                    "unknown"
                }
                .to_owned(),
            );
        }
        envelope
    }
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RememberKind {
    User,
    Assistant,
    Document,
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct RememberInput {
    pub conversation: String,
    pub content: String,
    pub kind: RememberKind,
    #[serde(default)]
    pub chunk_bytes: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecallMode {
    Lexical,
    Timeline,
}

#[derive(Clone, Debug, Deserialize, schemars::JsonSchema)]
pub struct RecallInput {
    pub mode: RecallMode,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub conversation: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub since_lsn: u64,
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

#[derive(Clone, Debug, Default, Deserialize, schemars::JsonSchema)]
pub struct InspectInput {}

const fn default_limit() -> usize {
    32
}

#[derive(Clone)]
pub struct McpServer {
    actor: ActorEngine,
}

impl McpServer {
    #[must_use]
    pub const fn new(actor: ActorEngine) -> Self {
        Self { actor }
    }

    pub async fn remember_envelope(&self, input: RememberInput) -> Envelope {
        match self.remember_inner(input).await {
            Ok(value) => value,
            Err(error) => Envelope::error(error, true),
        }
    }

    async fn remember_inner(&self, input: RememberInput) -> Result<Envelope, Error> {
        if input.conversation.is_empty() || input.content.is_empty() {
            return Err(Error::new(ErrorCode::InvalidArgument));
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
                    retention: Retention::Durable,
                    sensitivity: Sensitivity::Personal,
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
            RecallMode::Lexical if !input.query.is_empty() => RecallRequest::Lexical {
                query: input.query,
                limit: input.limit,
            },
            RecallMode::Timeline if !input.conversation.is_empty() => RecallRequest::Timeline {
                conversation: ConversationId::derive(&input.conversation),
                since_lsn: LSN::new(input.since_lsn),
                limit: input.limit,
            },
            _ => return Err(Error::new(ErrorCode::InvalidArgument)),
        };
        let records = self.actor.recall(request).await?;
        let mut envelope = Envelope::empty();
        for record in records {
            let content = event_content(record.kind, &record.payload)?;
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

    pub async fn inspect_envelope(&self, _input: InspectInput) -> Envelope {
        match self.actor.stats().await {
            Ok(stats) => {
                let mut envelope = Envelope::empty();
                envelope.items.push(json!({
                    "actor": stats.actor.get(),
                    "log_events": stats.log_events,
                    "log_bytes": stats.log_bytes,
                    "applied_lsn": stats.applied.last_lsn.get(),
                    "applied_digest": hex(&stats.applied.rolling_digest),
                    "projections": stats.projections.iter().map(|projection| json!({
                        "name": projection.name,
                        "applied_lsn": projection.applied_lsn.get(),
                    })).collect::<Vec<_>>(),
                }));
                envelope
            }
            Err(error) => Envelope::error(error, false),
        }
    }
}

#[tool_router(server_handler)]
impl McpServer {
    #[tool(
        description = "Persist a user message, delivered assistant message, or chunked document"
    )]
    async fn remember(&self, Parameters(input): Parameters<RememberInput>) -> Json<Envelope> {
        Json(self.remember_envelope(input).await)
    }

    #[tool(description = "Recall memories by lexical match or conversation timeline")]
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

fn event_content(kind: hm_ledger::frame::EventKind, payload: &[u8]) -> Result<String, Error> {
    let schema_kind = hm_schema::event::EventKind::try_from(kind as u8)
        .map_err(|()| Error::new(ErrorCode::InvalidKind))?;
    let verified = verify_event(payload, schema_kind, hm_schema::event::Boundary::Disk)?;
    let bytes = match verified.envelope.payload {
        EventPayload::UserMsg(message) => message.content,
        EventPayload::DeliveredMsg(message) => message.content,
        _ => return Ok(String::new()),
    };
    String::from_utf8(bytes).map_err(|_| Error::new(ErrorCode::SchemaInvalid))
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
            ["activate", "inspect", "recall", "remember"]
                .map(str::to_owned)
                .into()
        );
    }
}
