#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc)]

use base64::Engine as _;
use hm_compose::bundle::{ActivationBundle, HealthStatus, RetrievalLane};
use hm_compose::tokens::FallbackWeights;
use hm_core::{ConversationId, Error, ErrorCode, LSN};
use hm_cortex::ingest::IngestResult;
use hm_cortex::vocabulary::{VocabularySource, import_envelope};
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

pub mod admission;
pub mod dispatcher;
pub mod extraction;
pub mod telemetry;
pub mod tools;
pub use telemetry::ObservedProvider;
pub use tools::attest::{AttestDisposition, AttestInput};
pub use tools::believe::{
    BeliefClaimInput, BeliefTypeInput, BelieveInput, ClaimInput, ProvenanceInput,
};
pub use tools::bind::BindInput;
pub use tools::consolidate::{
    ConsolidateAction, ConsolidateBudget, ConsolidateInput, ConsolidateMode, ConsolidationRuntime,
};
pub use tools::dispute::{DisputeInput, DisputeRuntime};
pub use tools::forget::{ForgetAction, ForgetInput};
pub use tools::inspect::{InspectInput, InspectMode};
pub use tools::intend::{
    AttentionFactorsInput, IntendAction, IntendCloseReason, IntendInput, WakeTriggerInput,
};
pub use tools::outcome::OutcomeInput;
pub use tools::predict::{ExpectedPredicateInput, PredicateKindInput, PredictInput};
pub use tools::recall::{RecallFilters, RecallInput, RecallMode};
pub use tools::reconstruct::ReconstructionRuntime;
pub use tools::relation::RelationBuildReport;
pub use tools::remember::{
    AnchorFacet, EmbeddingRuntime, RememberAnchor, RememberInput, RememberKind, RememberSource,
    RetentionInput, SensitivityInput, VocabularyInput,
};
pub use tools::retract::RetractInput;
pub use tools::websource::WebSourceRuntime;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_CHUNK_BYTES: usize = 32 * 1024;
const MAXIMUM_VOCABULARY_ID_BYTES: usize = 4096;

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest: Option<Value>,
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
            manifest: None,
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
    dispute_runtime: Option<DisputeRuntime>,
    consolidation_runtime: Option<ConsolidationRuntime>,
    embedding_runtime: Option<EmbeddingRuntime>,
    reconstruction_runtime: Option<ReconstructionRuntime>,
    web_source_runtime: Option<WebSourceRuntime>,
}

impl McpServer {
    #[must_use]
    pub const fn new(actor: ActorEngine) -> Self {
        Self {
            actor,
            admin_token: None,
            dispute_runtime: None,
            consolidation_runtime: None,
            embedding_runtime: None,
            reconstruction_runtime: None,
            web_source_runtime: None,
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
            dispute_runtime: None,
            consolidation_runtime: None,
            embedding_runtime: None,
            reconstruction_runtime: None,
            web_source_runtime: None,
        }
    }

    #[must_use]
    pub fn with_dispute_runtime(mut self, runtime: DisputeRuntime) -> Self {
        self.dispute_runtime = Some(runtime);
        self
    }

    #[must_use]
    pub fn with_consolidation_runtime(mut self, runtime: ConsolidationRuntime) -> Self {
        self.consolidation_runtime = Some(runtime);
        self
    }

    #[must_use]
    pub fn with_embedding_runtime(mut self, runtime: EmbeddingRuntime) -> Self {
        self.embedding_runtime = Some(runtime);
        self
    }

    #[must_use]
    pub fn with_reconstruction_runtime(mut self, runtime: ReconstructionRuntime) -> Self {
        self.reconstruction_runtime = Some(runtime);
        self
    }

    #[must_use]
    pub fn with_web_source_runtime(mut self, runtime: tools::websource::WebSourceRuntime) -> Self {
        self.web_source_runtime = Some(runtime);
        self
    }

    pub async fn configured(
        actor: ActorEngine,
        admin_token: Option<hm_serve::config::CapabilityToken>,
    ) -> Result<Self, Error> {
        let dispatcher = tokio::task::spawn_blocking(dispatcher::McpToolDispatcher::from_env)
            .await
            .map_err(|_| Error::new(ErrorCode::OperationUnavailable))??;
        let mut server = dispatcher.server(actor);
        server.admin_token = admin_token;
        Ok(server)
    }

    fn availability(&self) -> tools::surfaces::Availability {
        tools::surfaces::Availability {
            admin_token: self.admin_token.is_some(),
            embedding: self.embedding_runtime.is_some(),
            consolidation: self.consolidation_runtime.is_some(),
            reconstruction: self.reconstruction_runtime.is_some(),
            dispute: self.dispute_runtime.is_some(),
        }
    }

    pub async fn remember_envelope(&self, input: RememberInput) -> Envelope {
        match self.remember_inner(input).await {
            Ok(value) => value,
            Err(error) => Envelope::error(error, true),
        }
    }

    #[allow(clippy::too_many_lines, clippy::single_match_else)]
    async fn remember_inner(&self, input: RememberInput) -> Result<Envelope, Error> {
        if input.conversation.is_empty() {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        if input.source.is_some() {
            if !input.content.is_empty() {
                return Err(Error::new(ErrorCode::InvalidArgument));
            }
            return tools::websource::run(
                &self.actor,
                self.web_source_runtime.as_ref(),
                self.embedding_runtime.as_ref(),
                input,
            )
            .await;
        }
        if input.content.is_empty() {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        hm_compose::reconstruct::guard_remember(&input.content)?;
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
        if matches!(input.kind, RememberKind::Vocabulary) {
            return self.import_vocabulary(&input, retention, sensitivity).await;
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
            RememberKind::Vocabulary => return Err(Error::new(ErrorCode::InvariantViolation)),
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
        let embedding_documents = self.embedding_runtime.as_ref().map(|_| {
            chunks
                .iter()
                .map(|chunk| (*chunk).to_owned())
                .collect::<Vec<_>>()
        });
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
        if let (Some(runtime), Some(documents)) = (&self.embedding_runtime, embedding_documents) {
            match append_embeddings(
                &self.actor,
                runtime,
                documents,
                outcome.first_lsn,
                conversation,
                retention,
                sensitivity,
            )
            .await
            {
                Ok((first_lsn, last_lsn)) => {
                    envelope.items[0]["embedding_first_lsn"] = json!(first_lsn.get());
                    envelope.items[0]["embedding_last_lsn"] = json!(last_lsn.get());
                    envelope.health["encoder"] = json!(runtime.health());
                    envelope.health["backlog"] = json!("semantic_ready");
                }
                Err(_) => {
                    envelope.health["encoder"] = json!("semantic_lagging");
                    envelope.health["backlog"] = json!("semantic_lagging");
                    envelope.gaps.push(json!({"kind": "embedding_pending", "first_lsn": outcome.first_lsn.get(), "last_lsn": outcome.last_lsn.get()}));
                    envelope.warnings.push(
                        "Observation stored; its embedding was not confirmed committed.".to_owned(),
                    );
                }
            }
        } else {
            envelope.health["encoder"] = json!("lexical_only");
        }
        Ok(envelope)
    }

    async fn import_vocabulary(
        &self,
        input: &RememberInput,
        retention: Retention,
        sensitivity: Sensitivity,
    ) -> Result<Envelope, Error> {
        let vocabulary = input
            .vocabulary
            .as_ref()
            .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
        if vocabulary.vocabulary_id.is_empty()
            || vocabulary.vocabulary_id.len() > MAXIMUM_VOCABULARY_ID_BYTES
            || vocabulary.version == 0
        {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let imported = import_envelope(&VocabularySource {
            vocabulary_id: vocabulary.vocabulary_id.as_bytes(),
            version: vocabulary.version,
            source_uri: vocabulary.source_uri.as_str(),
            document: input.content.as_str(),
            retention,
            sensitivity,
        })?;
        let IngestResult::Append(envelope) = imported else {
            return Err(Error::new(ErrorCode::InvariantViolation));
        };
        let payload = encode_event_envelope(&envelope);
        let EventPayload::VocabularyImported(value) = envelope.payload else {
            return Err(Error::new(ErrorCode::InvariantViolation));
        };
        let outcome = self
            .actor
            .append(vec![IncomingEvent {
                kind: hm_ledger::frame::EventKind::VocabularyImported,
                conversation: ConversationId::derive(&input.conversation),
                payload,
            }])
            .await?;
        let mut result = Envelope::empty();
        result.items.push(json!({
            "first_lsn": outcome.first_lsn.get(),
            "last_lsn": outcome.last_lsn.get(),
            "count": outcome.last_lsn.get() - outcome.first_lsn.get() + 1,
            "vocabulary_id": vocabulary.vocabulary_id,
            "version": value.version,
            "source_uri": value.source_uri,
            "source_media_type": value.source_media_type,
            "source_digest": hex(&value.source_digest),
            "term_count": value.terms.len(),
            "ignored_triples": value.ignored_triples,
        }));
        result.provenance.push(format!(
            "hm://{}/lsn/{}",
            self.actor.actor(),
            outcome.first_lsn.get()
        ));
        result.warnings.push(
            "A vocabulary import declares terms for this actor; it merges no existing identity."
                .to_owned(),
        );
        Ok(result)
    }

    pub async fn recall_envelope(&self, input: RecallInput) -> Envelope {
        match self.recall_inner(input).await {
            Ok(value) => value,
            Err(error) => Envelope::error(error, false),
        }
    }

    #[allow(clippy::too_many_lines)]
    async fn recall_inner(&self, input: RecallInput) -> Result<Envelope, Error> {
        if matches!(input.mode, RecallMode::Reconstruct) {
            return tools::reconstruct::run(
                &self.actor,
                self.reconstruction_runtime.as_ref(),
                input,
            )
            .await;
        }
        if matches!(input.mode, RecallMode::Relation) {
            return self.relation_recall_inner(input).await;
        }
        if input.limit == 0 || input.limit > 4096 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let request = match input.mode {
            RecallMode::Semantic if !input.query.is_empty() => {
                let runtime = self
                    .embedding_runtime
                    .as_ref()
                    .ok_or_else(|| Error::new(ErrorCode::OperationUnavailable))?;
                let embedding = runtime.query(input.query.clone()).await?;
                RecallRequest::Vector {
                    space_id: tools::remember::space_id(&embedding.space),
                    query: embedding.values,
                    binary_prefilter: embedding.binary_prefilter,
                    limit: input.limit,
                }
            }
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
        if matches!(input.mode, RecallMode::Semantic) {
            envelope.health["encoder"] = json!(
                self.embedding_runtime
                    .as_ref()
                    .map(EmbeddingRuntime::health)
            );
        }
        for record in records {
            let (content, authority) = event_content(record.kind, &record.payload)?;
            let preference = if record.preference_q16 == hm_compose::fusion::Q16_ONE {
                String::new()
            } else {
                format!("&pref={}", record.preference_q16)
            };
            let uri = format!(
                "hm://{}/{}/{}?at={}&src={}&score={}{preference}",
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
                "preference_q16": record.preference_q16,
                "uri": uri,
            }));
            envelope.provenance.push(uri);
        }
        Ok(envelope)
    }

    async fn relation_recall_inner(&self, input: RecallInput) -> Result<Envelope, Error> {
        if input.limit == 0
            || input.limit > hm_compose::bundle::MAXIMUM_CANDIDATES
            || input.query.is_empty()
        {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let runtime = self
            .embedding_runtime
            .as_ref()
            .ok_or_else(|| Error::new(ErrorCode::OperationUnavailable))?;
        let embedding = runtime.query(input.query.clone()).await?;
        let relations = self
            .actor
            .relation_recall(RecallRequest::Relation {
                space_id: tools::remember::relation_space_id(&embedding.space),
                query: embedding.values,
                binary_prefilter: embedding.binary_prefilter,
                limit: input.limit,
            })
            .await?;
        let mut envelope = Envelope::empty();
        envelope.health["encoder"] = json!(runtime.health());
        for relation in relations {
            let uri = format!(
                "hm://{}/lsn/{}",
                self.actor.actor(),
                relation.event_lsn.get()
            );
            envelope.items.push(json!({
                "edge_id": hex(&relation.edge_id),
                "relation": relation.relation,
                "source": String::from_utf8_lossy(&relation.source_id),
                "target": String::from_utf8_lossy(&relation.target_id),
                "event_lsn": relation.event_lsn.get(),
                "weight_micros": relation.weight_micros,
                "support_lsns": relation
                    .support_lsns
                    .iter()
                    .map(|lsn| lsn.get())
                    .collect::<Vec<_>>(),
                "score_q32": relation.score_q32,
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
        match tools::inspect::run(&self.actor, self.availability(), input).await {
            Ok(value) => value,
            Err(error) => Envelope::error(error, false),
        }
    }

    pub async fn predict_envelope(&self, input: PredictInput) -> Envelope {
        match tools::predict::run(&self.actor, input).await {
            Ok(value) => value,
            Err(error) => Envelope::error(error, true),
        }
    }

    pub async fn outcome_envelope(&self, input: OutcomeInput) -> Envelope {
        match tools::outcome::run(&self.actor, input).await {
            Ok(value) => value,
            Err(error) => Envelope::error(error, true),
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

    pub async fn attest_envelope(&self, input: AttestInput) -> Envelope {
        match tools::attest::run(&self.actor, input).await {
            Ok(value) => value,
            Err(error) => Envelope::error(error, true),
        }
    }

    pub async fn consolidate_envelope(&self, input: ConsolidateInput) -> Envelope {
        let mutation = !matches!(input.action, ConsolidateAction::List);
        match tools::consolidate::run(
            &self.actor,
            self.consolidation_runtime.as_ref(),
            self.embedding_runtime.as_ref(),
            input,
        )
        .await
        {
            Ok(value) => value,
            Err(error) => Envelope::error(error, mutation),
        }
    }

    pub async fn believe_envelope(&self, input: BelieveInput) -> Envelope {
        match tools::believe::run(&self.actor, input).await {
            Ok(value) => value,
            Err(error) => Envelope::error(error, true),
        }
    }

    pub async fn retract_envelope(&self, input: RetractInput) -> Envelope {
        match tools::retract::run(&self.actor, input).await {
            Ok(value) => value,
            Err(error) => Envelope::error(error, true),
        }
    }

    pub async fn dispute_envelope(&self, input: DisputeInput) -> Envelope {
        match tools::dispute::run(&self.actor, self.dispute_runtime.as_ref(), input).await {
            Ok(value) => value,
            Err(error) => Envelope::error(error, true),
        }
    }
}

#[tool_router(server_handler)]
impl McpServer {
    #[tool(description = "Register immutable bounded expectations before observing results")]
    async fn predict(&self, Parameters(input): Parameters<PredictInput>) -> Json<Envelope> {
        Json(self.predict_envelope(input).await)
    }

    #[tool(
        description = "Assess a prediction from cited observed ledger events, never remembered narrative"
    )]
    async fn outcome(&self, Parameters(input): Parameters<OutcomeInput>) -> Json<Envelope> {
        Json(self.outcome_envelope(input).await)
    }
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

    #[tool(description = "Record used, ignored, helpful, or harmful provenance feedback")]
    async fn attest(&self, Parameters(input): Parameters<AttestInput>) -> Json<Envelope> {
        Json(self.attest_envelope(input).await)
    }

    #[tool(description = "Run, list, or retract budgeted consolidation generations")]
    async fn consolidate(&self, Parameters(input): Parameters<ConsolidateInput>) -> Json<Envelope> {
        Json(self.consolidate_envelope(input).await)
    }

    #[tool(description = "Write a typed bitemporal belief assertion")]
    async fn believe(&self, Parameters(input): Parameters<BelieveInput>) -> Json<Envelope> {
        Json(self.believe_envelope(input).await)
    }

    #[tool(description = "Tombstone a belief while preserving its history")]
    async fn retract(&self, Parameters(input): Parameters<RetractInput>) -> Json<Envelope> {
        Json(self.retract_envelope(input).await)
    }

    #[tool(description = "Adjudicate two belief claims with local bidirectional NLI")]
    async fn dispute(&self, Parameters(input): Parameters<DisputeInput>) -> Json<Envelope> {
        Json(self.dispute_envelope(input).await)
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
    envelope.manifest = Some(json!({
        "manifest_id": hex(&bundle.manifest.manifest_id),
        "query_digest": hex(&bundle.manifest.query_digest),
        "snapshot_epoch": bundle.manifest.snapshot_epoch,
        "encoder": bundle.manifest.encoder,
        "index_generation": bundle.manifest.index_generation,
        "candidate_lanes": bundle
            .manifest
            .candidate_lanes
            .iter()
            .copied()
            .map(lane_name)
            .collect::<Vec<_>>(),
        "retrieved": lsn_numbers(&bundle.manifest.candidates),
        "selected": lsn_numbers(&bundle.manifest.selected),
        "included": lsn_numbers(&bundle.manifest.included),
        "used": lsn_numbers(&bundle.manifest.used),
    }));
    envelope
}

fn lsn_numbers(values: &[LSN]) -> Vec<u64> {
    values.iter().map(|lsn| lsn.get()).collect()
}

const fn lane_name(lane: RetrievalLane) -> &'static str {
    match lane {
        RetrievalLane::Lexical => "lexical",
        RetrievalLane::Vector => "vector",
        RetrievalLane::Entity => "entity",
        RetrievalLane::Temporal => "temporal",
        RetrievalLane::Graph => "graph",
        RetrievalLane::Belief => "belief",
        RetrievalLane::Timeline => "timeline",
        RetrievalLane::Reconstruct => "reconstruct",
        RetrievalLane::Relation => "relation",
    }
}

pub(crate) async fn append_embeddings(
    actor: &ActorEngine,
    runtime: &EmbeddingRuntime,
    documents: Vec<String>,
    first_target_lsn: LSN,
    conversation: ConversationId,
    retention: Retention,
    sensitivity: Sensitivity,
) -> Result<(LSN, LSN), Error> {
    let embeddings = runtime.documents(documents).await?;
    let count =
        u32::try_from(embeddings.len()).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
    let mut events = Vec::with_capacity(embeddings.len());
    for (index, embedding) in embeddings.into_iter().enumerate() {
        let index = u32::try_from(index).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
        let dimension = u32::try_from(embedding.space.dimensions)
            .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
        let space_id = tools::remember::space_id(&embedding.space);
        let model_id = format!(
            "{}@{}",
            embedding.space.encoder_id, embedding.space.revision
        );
        events.push(IncomingEvent {
            kind: hm_ledger::frame::EventKind::Embedding,
            conversation,
            payload: encode_event_envelope(&EventEnvelope {
                schema_version: CURRENT_SCHEMA_VERSION,
                payload: EventPayload::Embedding(Box::new(hm_schema::events::Embedding {
                    target_lsn: first_target_lsn.get() + u64::from(index),
                    dimension,
                    quantized: embedding.values,
                    binary_prefilter: embedding.binary_prefilter,
                    space_id,
                })),
                connection_id: None,
                client_seq: 0,
                client_event_index: index,
                client_event_count: count,
                origin_actor: 0,
                run_id: None,
                model_provenance: Some(Box::new(hm_schema::events::ModelProvenance {
                    model_id,
                    prompt_id: "embedding/document".to_owned(),
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
                retention,
                sensitivity,
                event_time_ns: 0,
            }),
        });
    }
    let outcome = actor.append(events).await?;
    Ok((outcome.first_lsn, outcome.last_lsn))
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
    fn rmcp_router_exposes_current_tools() {
        let names: std::collections::BTreeSet<_> = McpServer::tool_router()
            .list_all()
            .into_iter()
            .map(|tool| tool.name.into_owned())
            .collect();
        assert_eq!(
            names,
            [
                "activate",
                "attest",
                "believe",
                "bind",
                "consolidate",
                "dispute",
                "forget",
                "inspect",
                "intend",
                "outcome",
                "predict",
                "recall",
                "remember",
                "retract"
            ]
            .map(str::to_owned)
            .into()
        );
    }
}
