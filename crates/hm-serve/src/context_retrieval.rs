use crate::actor::{ActorEngine, IncomingEvent};
use hm_context::retrieval::{
    self, EmbeddedVector, EvidenceCandidate, IndexBatch, RetrievalReport, RetrievalRequest,
    SourceKind, VectorFingerprint,
};
use hm_context::{Authority, ContextError, Scope, SourceSpan, digest_bytes, validate_id};
use hm_core::{ConversationId, Error, LSN};
use hm_embed::{Embedder, InputRole, SpaceIdentity};
use hm_ledger::frame::EventKind;
use hm_schema::{
    event::{self, CURRENT_SCHEMA_VERSION},
    events::{EventEnvelope, EventPayload, ProviderFrame, Retention, Sensitivity},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub mod sources;
pub use sources::{CommitIndexRequest, FileIndexRequest, index_commit, index_file};
pub const RETRIEVAL_PROVIDER: &str = "hypermind/context-retrieval/v1";
const MAX_FRAMES: usize = 100_000;
const MAX_TEXT: usize = 128 * 1024;

#[derive(Debug)]
pub enum RetrievalError {
    Context(ContextError),
    Ledger(Error),
}
impl std::fmt::Display for RetrievalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Context(e) => e.fmt(f),
            Self::Ledger(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for RetrievalError {}
impl From<ContextError> for RetrievalError {
    fn from(e: ContextError) -> Self {
        Self::Context(e)
    }
}
impl From<Error> for RetrievalError {
    fn from(e: Error) -> Self {
        Self::Ledger(e)
    }
}
impl From<serde_json::Error> for RetrievalError {
    fn from(e: serde_json::Error) -> Self {
        Self::Context(e.into())
    }
}
impl From<std::io::Error> for RetrievalError {
    fn from(e: std::io::Error) -> Self {
        Self::Context(e.into())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRecord {
    pub scope: Scope,
    pub kind: SourceKind,
    pub id: String,
    pub revision: u64,
    pub text: String,
    pub content_digest: String,
    pub authority: Authority,
    pub provenance: Vec<SourceSpan>,
    #[serde(with = "hm_context::types::optional_timestamp_wire")]
    pub occurred_at_ns: Option<i64>,
    #[serde(with = "hm_context::types::timestamp_wire")]
    pub recorded_at_ns: i64,
    #[serde(with = "hm_context::types::optional_timestamp_wire")]
    pub expires_at_ns: Option<i64>,
    pub tombstoned: bool,
}
impl SourceRecord {
    pub fn validate(&self) -> Result<(), ContextError> {
        self.scope.validate()?;
        validate_id(&self.id)?;
        if self.revision == 0
            || self.text.is_empty()
            || self.text.len() > MAX_TEXT
            || self.content_digest != digest_bytes(self.text.as_bytes())
            || self.provenance.is_empty()
            || self.provenance.len() > 1024
        {
            return Err(ContextError::Invalid("invalid retrieval source".into()));
        }
        for span in &self.provenance {
            validate_id(&span.source_id)?;
            validate_id(&span.source_digest)?;
            if span.byte_start >= span.byte_end {
                return Err(ContextError::Invalid("invalid retrieval provenance".into()));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingMode {
    Off,
    Local,
    RemoteCompatible,
    Managed,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmbeddingRegistration {
    pub id: String,
    pub revision: u64,
    pub mode: EmbeddingMode,
    pub fingerprint: Option<VectorFingerprint>,
}
impl EmbeddingRegistration {
    pub fn validate(&self) -> Result<(), ContextError> {
        validate_id(&self.id)?;
        if self.revision == 0 || (self.mode == EmbeddingMode::Off) != self.fingerprint.is_none() {
            return Err(ContextError::Invalid(
                "invalid embedding registration".into(),
            ));
        }
        if let Some(f) = &self.fingerprint {
            validate_id(&f.model)?;
            validate_id(&f.revision)?;
            if f.dimensions == 0 || f.dimensions > 65_536 {
                return Err(ContextError::Invalid("invalid embedding dimensions".into()));
            }
        }
        Ok(())
    }
}
#[derive(Clone)]
pub struct EmbeddingProvider {
    pub registration: EmbeddingRegistration,
    pub embedder: Arc<dyn Embedder>,
}
impl EmbeddingProvider {
    pub fn new(
        id: &str,
        revision: u64,
        mode: EmbeddingMode,
        embedder: Arc<dyn Embedder>,
    ) -> Result<Self, RetrievalError> {
        if mode == EmbeddingMode::Off || embedder.report_label() == "lexical_only" {
            return Err(ContextError::Unavailable("semantic encoder unavailable".into()).into());
        }
        let document = embedder.identity(InputRole::Document);
        let mut query = embedder.identity(InputRole::Query);
        if document.input_role != InputRole::Document || query.input_role != InputRole::Query {
            return Err(ContextError::Invalid("embedding role mismatch".into()).into());
        }
        query.input_role = InputRole::Document;
        if query != document {
            return Err(ContextError::Invalid("incompatible query encoder".into()).into());
        }
        let registration = EmbeddingRegistration {
            id: id.into(),
            revision,
            mode,
            fingerprint: Some(fingerprint(&document)),
        };
        registration.validate()?;
        Ok(Self {
            registration,
            embedder,
        })
    }
}
fn fingerprint(space: &SpaceIdentity) -> VectorFingerprint {
    VectorFingerprint {
        model: space.encoder_id.clone(),
        revision: digest_bytes(
            format!(
                "{}:{:?}:{:?}",
                space.revision, space.distance, space.normalization
            )
            .as_bytes(),
        ),
        dimensions: space.dimensions,
    }
}

type Key = (SourceKind, String);
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredVector {
    key: Key,
    source_revision: u64,
    registration: EmbeddingRegistration,
    vector: EmbeddedVector,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
enum Operation {
    Grant {
        grant: hm_context::retrieval::EvidenceGrant,
        revoked: bool,
    },
    Source {
        expected_revision: u64,
        source: SourceRecord,
    },
    Registration {
        expected_revision: u64,
        registration: EmbeddingRegistration,
    },
    Backfill {
        registration: EmbeddingRegistration,
        vectors: Vec<StoredVector>,
        checkpoint: Option<String>,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    version: u32,
    scope: Scope,
    operation: Operation,
}
#[derive(Default)]
struct State {
    grants: BTreeMap<(Scope, Key), hm_context::retrieval::EvidenceGrant>,
    sources: BTreeMap<Key, SourceRecord>,
    vectors: BTreeMap<Key, StoredVector>,
    registration: Option<EmbeddingRegistration>,
    checkpoint: Option<String>,
    tail: u64,
}

async fn state(actor: &ActorEngine, scope: &Scope) -> Result<State, RetrievalError> {
    scope.validate()?;
    let frames = actor
        .frames_since(LSN::new(0), None, MAX_FRAMES + 1)
        .await?;
    if frames.len() > MAX_FRAMES {
        return Err(ContextError::Capacity.into());
    }
    let mut out = State::default();
    for frame in frames {
        out.tail = frame.header.lsn.get();
        if frame.header.kind != EventKind::ProviderFrame {
            continue;
        }
        let event = actor.verified_event(frame.header.lsn).await?;
        let EventPayload::ProviderFrame(provider) = event.envelope.payload else {
            continue;
        };
        if provider.provider != RETRIEVAL_PROVIDER {
            continue;
        }
        let record: Record = serde_json::from_slice(&provider.api_content)?;
        if record.version != 1 {
            return Err(ContextError::Invalid("unsupported retrieval record".into()).into());
        }
        record.scope.validate()?;
        if frame.header.conversation != conversation(&record.scope)? {
            return Err(ContextError::Conflict.into());
        }
        if &record.scope != scope {
            continue;
        }
        match record.operation {
            Operation::Source {
                expected_revision,
                source,
            } => {
                source.validate()?;
                if source.scope != *scope {
                    return Err(ContextError::ScopeMismatch.into());
                }
                let key = (source.kind, source.id.clone());
                let current = out.sources.get(&key).map_or(0, |s| s.revision);
                if current != expected_revision || source.revision != current + 1 {
                    return Err(ContextError::Conflict.into());
                }
                out.vectors.remove(&key);
                out.sources.insert(key, source);
            }
            Operation::Grant { grant, revoked } => {
                if grant.source_scope != *scope {
                    return Err(ContextError::ScopeMismatch.into());
                }
                grant.recipient_scope.validate()?;
                validate_id(&grant.source_id)?;
                validate_id(&grant.source_digest)?;
                let key = (
                    grant.recipient_scope.clone(),
                    (grant.kind, grant.source_id.clone()),
                );
                if revoked {
                    out.grants.remove(&key);
                } else {
                    out.grants.insert(key, grant);
                }
            }
            Operation::Registration {
                expected_revision,
                registration,
            } => {
                registration.validate()?;
                if out.registration.as_ref().map_or(0, |r| r.revision) != expected_revision
                    || registration.revision != expected_revision + 1
                {
                    return Err(ContextError::Conflict.into());
                }
                out.registration = Some(registration);
                out.checkpoint = None;
            }
            Operation::Backfill {
                registration,
                vectors,
                checkpoint,
            } => {
                if out.registration.as_ref() != Some(&registration) {
                    return Err(ContextError::Stale.into());
                }
                for vector in vectors {
                    vector.vector.validate()?;
                    if vector.registration != registration {
                        return Err(ContextError::Conflict.into());
                    }
                    out.vectors.insert(vector.key.clone(), vector);
                }
                out.checkpoint = checkpoint;
            }
        }
    }
    Ok(out)
}
fn conversation(scope: &Scope) -> Result<ConversationId, RetrievalError> {
    Ok(ConversationId::derive(&format!(
        "context-retrieval:{}",
        scope.digest()?
    )))
}
async fn append(
    actor: &ActorEngine,
    scope: &Scope,
    tail: u64,
    operation: Operation,
) -> Result<u64, RetrievalError> {
    let content = serde_json::to_vec(&Record {
        version: 1,
        scope: scope.clone(),
        operation,
    })?;
    if content.len() > 512 * 1024 {
        return Err(ContextError::Capacity.into());
    }
    let payload = event::encode_event_envelope(&EventEnvelope {
        schema_version: CURRENT_SCHEMA_VERSION,
        payload: EventPayload::ProviderFrame(Box::new(ProviderFrame {
            provider: RETRIEVAL_PROVIDER.into(),
            api_content: content,
        })),
        connection_id: None,
        client_seq: 0,
        client_event_index: 0,
        client_event_count: 1,
        origin_actor: 0,
        run_id: None,
        model_provenance: None,
        authority: hm_schema::events::Authority::RuntimeFact,
        retention: Retention::Durable,
        sensitivity: Sensitivity::Personal,
        event_time_ns: 0,
    });
    Ok(actor
        .append_if_tail(
            LSN::new(tail),
            vec![IncomingEvent {
                kind: EventKind::ProviderFrame,
                conversation: conversation(scope)?,
                payload,
            }],
        )
        .await?
        .last_lsn
        .get())
}

pub async fn register_source(
    actor: &ActorEngine,
    trusted_scope: &Scope,
    expected_revision: u64,
    source: SourceRecord,
) -> Result<u64, RetrievalError> {
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    source.validate()?;
    if matches!(source.kind, SourceKind::Memory | SourceKind::Conversation)
        || source.id.starts_with("history:")
        || source.id.starts_with("memory:")
        || source.authority == Authority::RuntimeFact
    {
        return Err(
            ContextError::Invalid("source requires its canonical admission path".into()).into(),
        );
    }
    if &source.scope != trusted_scope {
        return Err(ContextError::ScopeMismatch.into());
    }
    let current = state(actor, trusted_scope).await?;
    let key = (source.kind, source.id.clone());
    if current.sources.get(&key) == Some(&source) {
        return Ok(current.tail);
    }
    if current.sources.get(&key).map_or(0, |s| s.revision) != expected_revision
        || source.revision
            != expected_revision
                .checked_add(1)
                .ok_or(ContextError::Capacity)?
    {
        return Err(ContextError::Conflict.into());
    }
    append(
        actor,
        trusted_scope,
        current.tail,
        Operation::Source {
            expected_revision,
            source,
        },
    )
    .await
}
pub async fn tombstone(
    actor: &ActorEngine,
    scope: &Scope,
    kind: SourceKind,
    id: &str,
    expected_revision: u64,
) -> Result<u64, RetrievalError> {
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let current = state(actor, scope).await?;
    let mut source = current
        .sources
        .get(&(kind, id.into()))
        .cloned()
        .ok_or(ContextError::Stale)?;
    if source.revision != expected_revision {
        return Err(ContextError::Conflict.into());
    }
    source.revision = source
        .revision
        .checked_add(1)
        .ok_or(ContextError::Capacity)?;
    source.tombstoned = true;
    append(
        actor,
        scope,
        current.tail,
        Operation::Source {
            expected_revision,
            source,
        },
    )
    .await
}
pub async fn register_embedding(
    actor: &ActorEngine,
    scope: &Scope,
    expected_revision: u64,
    registration: EmbeddingRegistration,
) -> Result<u64, RetrievalError> {
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    registration.validate()?;
    let current = state(actor, scope).await?;
    if current.registration.as_ref() == Some(&registration) {
        return Ok(current.tail);
    }
    if current.registration.as_ref().map_or(0, |r| r.revision) != expected_revision
        || registration.revision
            != expected_revision
                .checked_add(1)
                .ok_or(ContextError::Capacity)?
    {
        return Err(ContextError::Conflict.into());
    }
    append(
        actor,
        scope,
        current.tail,
        Operation::Registration {
            expected_revision,
            registration,
        },
    )
    .await
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BackfillReport {
    pub registration: Option<EmbeddingRegistration>,
    pub checkpoint: Option<String>,
    pub embedded: usize,
    pub remaining: usize,
    pub unavailable: Option<String>,
    pub ledger_tail: u64,
    pub usage: String,
}
fn cursor(key: &Key) -> String {
    format!("{:02}:{}", key.0 as u8, key.1)
}
pub async fn backfill(
    actor: &ActorEngine,
    scope: &Scope,
    provider: Option<&EmbeddingProvider>,
    maximum_items: usize,
    maximum_bytes: usize,
    now_ns: i64,
) -> Result<BackfillReport, RetrievalError> {
    if maximum_items == 0
        || maximum_items > 16
        || maximum_bytes == 0
        || maximum_bytes > MAX_TEXT * 16
    {
        return Err(ContextError::Capacity.into());
    }
    let frozen = state(actor, scope).await?;
    let sources = all_sources(actor, scope, now_ns, &frozen).await?;
    let registration = frozen.registration.clone();
    let mut report = BackfillReport {
        registration: registration.clone(),
        checkpoint: frozen.checkpoint.clone(),
        embedded: 0,
        remaining: 0,
        unavailable: None,
        ledger_tail: frozen.tail,
        usage: "unknown: embedding provider does not report accounting".into(),
    };
    report.remaining = sources
        .iter()
        .filter(|(_, s)| !s.tombstoned && s.expires_at_ns.is_none_or(|t| t > now_ns))
        .count();
    let Some(provider) = provider else {
        report.unavailable = Some("embedding provider unavailable".into());
        return Ok(report);
    };
    if registration.as_ref() != Some(&provider.registration) {
        return Err(ContextError::Stale.into());
    }
    let pending: Vec<_> = sources
        .iter()
        .filter(|(key, s)| {
            !s.tombstoned
                && s.expires_at_ns.is_none_or(|t| t > now_ns)
                && !valid_vector(&frozen, key, s, &provider.registration)
        })
        .collect();
    report.remaining = pending.len();
    let mut selected = Vec::new();
    let mut bytes = 0usize;
    for (key, source) in pending.iter().copied() {
        if selected.len() >= maximum_items {
            break;
        }
        let vector_bytes = provider
            .registration
            .fingerprint
            .as_ref()
            .map_or(0, |f| f.dimensions.saturating_mul(32));
        if vector_bytes > 384 * 1024 {
            return Err(ContextError::Capacity.into());
        }
        if vector_bytes.saturating_mul(selected.len() + 1) > 384 * 1024 {
            break;
        }
        if bytes
            .checked_add(source.text.len())
            .is_none_or(|n| n > maximum_bytes)
        {
            break;
        }
        bytes += source.text.len();
        selected.push((key.clone(), source.clone()));
    }
    if selected.is_empty() {
        return Ok(report);
    }
    let worker = provider.embedder.clone();
    let inputs: Vec<_> = selected.iter().map(|(_, s)| s.text.clone()).collect();
    let embeddings = tokio::task::spawn_blocking(move || {
        let refs: Vec<_> = inputs.iter().map(String::as_str).collect();
        worker.embed_documents(&refs)
    })
    .await
    .map_err(|_| ContextError::Unavailable("embedding worker failed".into()))?;
    let embeddings = match embeddings {
        Ok(v) => v,
        Err(_) => {
            report.unavailable = Some("embedding provider failed; checkpoint unchanged".into());
            return Ok(report);
        }
    };
    if embeddings.len() != selected.len() {
        return Err(ContextError::Invalid("embedding batch count mismatch".into()).into());
    }
    let identity = provider.embedder.identity(InputRole::Document);
    let mut vectors = Vec::new();
    for ((key, source), embedding) in selected.iter().zip(embeddings) {
        if embedding.space != identity
            || Some(fingerprint(&embedding.space)) != provider.registration.fingerprint
        {
            return Err(ContextError::Invalid("embedding model substitution".into()).into());
        }
        let vector = EmbeddedVector {
            fingerprint: fingerprint(&embedding.space),
            content_digest: source.content_digest.clone(),
            values: embedding.values,
        };
        vector.validate()?;
        vectors.push(StoredVector {
            key: key.clone(),
            source_revision: source.revision,
            registration: provider.registration.clone(),
            vector,
        });
    }
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let current = state(actor, scope).await?;
    let current_sources = all_sources(actor, scope, now_ns, &current).await?;
    if current.registration != registration
        || selected
            .iter()
            .any(|(key, source)| current_sources.get(key) != Some(source))
    {
        return Err(ContextError::Stale.into());
    }
    let checkpoint = selected.last().map(|(key, _)| cursor(key));
    report.ledger_tail = append(
        actor,
        scope,
        current.tail,
        Operation::Backfill {
            registration: provider.registration.clone(),
            vectors,
            checkpoint: checkpoint.clone(),
        },
    )
    .await?;
    report.embedded = selected.len();
    report.remaining -= selected.len();
    report.checkpoint = checkpoint;
    Ok(report)
}
fn valid_vector(
    state: &State,
    key: &Key,
    source: &SourceRecord,
    registration: &EmbeddingRegistration,
) -> bool {
    state.vectors.get(key).is_some_and(|v| {
        v.registration == *registration
            && v.source_revision == source.revision
            && v.vector.content_digest == source.content_digest
            && Some(&v.vector.fingerprint) == registration.fingerprint.as_ref()
    })
}

pub async fn retrieve(
    actor: &ActorEngine,
    scope: &Scope,
    query: &str,
    request: &RetrievalRequest,
    tokenizer: &hm_compose::tokens::TokenCounter,
    provider: Option<&EmbeddingProvider>,
) -> Result<RetrievalReport, RetrievalError> {
    if &request.scope != scope || query.is_empty() || query.len() > MAX_TEXT {
        return Err(ContextError::Invalid("invalid retrieval request".into()).into());
    }
    let current = state(actor, scope).await?;
    let own_sources = all_sources(actor, scope, request.now_ns, &current).await?;
    let mut vector_sources: BTreeMap<(Scope, Key), EmbeddedVector> = BTreeMap::new();
    let mut sources: Vec<_> = own_sources.into_iter().collect();
    for (key, source) in &sources {
        if let Some(registration) = &current.registration {
            if valid_vector(&current, key, source, registration) {
                vector_sources.insert(
                    (source.scope.clone(), key.clone()),
                    current.vectors[key].vector.clone(),
                );
            }
        }
    }
    let foreign_scopes: BTreeSet<_> = request
        .grants
        .iter()
        .filter(|g| {
            g.recipient_scope == *scope
                && g.expires_at_ns > request.now_ns
                && g.source_scope != *scope
        })
        .map(|g| g.source_scope.clone())
        .collect();
    if foreign_scopes.len() > 64 {
        return Err(ContextError::Capacity.into());
    }
    for foreign_scope in foreign_scopes {
        let foreign = state(actor, &foreign_scope).await?;
        for (key, source) in all_sources(actor, &foreign_scope, request.now_ns, &foreign).await? {
            let allowed = request.grants.iter().any(|g| {
                foreign.grants.get(&(scope.clone(), key.clone())) == Some(g)
                    && g.recipient_scope == *scope
                    && g.source_scope == foreign_scope
                    && g.kind == source.kind
                    && g.source_id == source.id
                    && g.source_digest == source.content_digest
                    && g.source_revision == source.revision
                    && g.expires_at_ns > request.now_ns
            });
            if !allowed {
                continue;
            }
            if let Some(registration) = &foreign.registration {
                if valid_vector(&foreign, &key, &source, registration) {
                    vector_sources.insert(
                        (source.scope.clone(), key.clone()),
                        foreign.vectors[&key].vector.clone(),
                    );
                }
            }
            sources.push((key, source));
        }
    }
    if sources.len() > request.max_candidates {
        return Err(ContextError::Capacity.into());
    }
    let mut req = request.clone();
    req.query_vector = None;
    let mut query_failed = false;
    if let Some(provider) = provider {
        if current.registration.as_ref() != Some(&provider.registration) {
            return Err(ContextError::Stale.into());
        }
        let worker = provider.embedder.clone();
        let query_owned = query.to_owned();
        if let Ok(Ok(embedding)) =
            tokio::task::spawn_blocking(move || worker.embed_query(&query_owned)).await
        {
            if embedding.space != provider.embedder.identity(InputRole::Query)
                || Some(fingerprint(&embedding.space)) != provider.registration.fingerprint
            {
                return Err(ContextError::Invalid("query model substitution".into()).into());
            }
            let vector = EmbeddedVector {
                fingerprint: fingerprint(&embedding.space),
                content_digest: digest_bytes(query.as_bytes()),
                values: embedding.values,
            };
            vector.validate()?;
            req.query_vector = Some(vector);
        } else {
            query_failed = true;
        }
    }
    let terms: BTreeSet<_> = hm_index::tokenize::tokenize(query).into_iter().collect();
    let documents: Vec<_> = sources
        .into_iter()
        .filter(|(_, s)| !s.tombstoned && s.expires_at_ns.is_none_or(|t| t > request.now_ns))
        .collect();
    let tokenized: Vec<_> = documents
        .iter()
        .map(|(_, s)| hm_index::tokenize::tokenize(&s.text))
        .collect();
    let total_terms = tokenized.iter().map(|t| t.len() as u64).sum::<u64>();
    let mut lanes: BTreeMap<SourceKind, Vec<(u64, EvidenceCandidate)>> = BTreeMap::new();
    let mut lexical_members = BTreeSet::new();
    for ((key, source), tokens) in documents.iter().zip(&tokenized) {
        let mut score = 0u64;
        for term in &terms {
            let frequency = tokens.iter().filter(|t| *t == term).count();
            if frequency == 0 {
                continue;
            }
            let df = tokenized.iter().filter(|t| t.contains(term)).count() as u64;
            score = score
                .checked_add(hm_index::bm25::score_q32(
                    documents.len() as u64,
                    df,
                    total_terms,
                    tokens.len() as u32,
                    frequency as u32,
                )?)
                .ok_or(ContextError::Capacity)?;
        }
        let vector = vector_sources
            .get(&(source.scope.clone(), key.clone()))
            .cloned();
        if score == 0 && (req.query_vector.is_none() || vector.is_none()) {
            continue;
        }
        let candidate = EvidenceCandidate {
            scope: source.scope.clone(),
            kind: source.kind,
            id: source.id.clone(),
            text: source.text.clone(),
            authority: source.authority,
            provenance: source.provenance.clone(),
            tokens: tokenizer.count(source.text.as_bytes())? as u64,
            content_digest: source.content_digest.clone(),
            source_revision: source.revision,
            current_revision: source.revision,
            occurred_at_ns: source.occurred_at_ns,
            recorded_at_ns: source.recorded_at_ns,
            valid_from_ns: None,
            expires_at_ns: source.expires_at_ns,
            tombstoned: source.tombstoned,
            vector,
        };
        if score > 0 {
            lexical_members.insert((source.scope.clone(), source.kind, source.id.clone()));
        }
        lanes
            .entry(source.kind)
            .or_default()
            .push((score, candidate));
    }
    let batches: Vec<_> = lanes
        .into_iter()
        .map(|(kind, mut candidates)| {
            candidates.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.id.cmp(&b.1.id)));
            IndexBatch {
                kind,
                index_revision: u64::MAX,
                candidates: candidates.into_iter().map(|(_, c)| c).collect(),
            }
        })
        .collect();
    let mut report = retrieval::fuse_with_lexical_members(&req, &batches, &lexical_members)?;
    if query_failed {
        report.semantic = hm_context::retrieval::SemanticStatus::Unavailable {
            reason: "embedding query provider failed".into(),
        };
    }
    if actor.stats().await?.applied.last_lsn.get() != current.tail {
        return Err(ContextError::Stale.into());
    }
    Ok(report)
}

async fn all_sources(
    actor: &ActorEngine,
    scope: &Scope,
    now_ns: i64,
    state: &State,
) -> Result<BTreeMap<Key, SourceRecord>, RetrievalError> {
    let mut sources = state.sources.clone();
    sources::add_history(actor, scope, &mut sources).await?;
    sources::add_memory(actor, scope, now_ns, &mut sources).await?;
    Ok(sources)
}

pub async fn inspect(
    actor: &ActorEngine,
    scope: &Scope,
) -> Result<serde_json::Value, RetrievalError> {
    let current = state(actor, scope).await?;
    let now_ns = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| ContextError::Unavailable("clock unavailable".into()))?
            .as_nanos(),
    )
    .map_err(|_| ContextError::Capacity)?;
    let sources = all_sources(actor, scope, now_ns, &current).await?;
    let vector_count = sources
        .iter()
        .filter(|(key, source)| {
            !source.tombstoned
                && source.expires_at_ns.is_none_or(|t| t > now_ns)
                && current
                    .registration
                    .as_ref()
                    .is_some_and(|r| valid_vector(&current, key, source, r))
        })
        .count();
    Ok(
        serde_json::json!({"registration":current.registration,"checkpoint":current.checkpoint,"source_count":sources.len(),"registered_source_count":current.sources.len(),"vector_count":vector_count,"stored_vector_count":current.vectors.len(),"ledger_tail":current.tail,"observed_at_ns":now_ns.to_string()}),
    )
}

pub async fn set_grant(
    actor: &ActorEngine,
    owner: &Scope,
    grant: hm_context::retrieval::EvidenceGrant,
    now_ns: i64,
) -> Result<u64, RetrievalError> {
    if grant.source_scope != *owner || grant.expires_at_ns <= now_ns {
        return Err(ContextError::ScopeMismatch.into());
    }
    grant.recipient_scope.validate()?;
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let current = state(actor, owner).await?;
    let sources = all_sources(actor, owner, now_ns, &current).await?;
    let source = sources
        .get(&(grant.kind, grant.source_id.clone()))
        .ok_or(ContextError::Stale)?;
    if source.tombstoned
        || source.content_digest != grant.source_digest
        || source.revision != grant.source_revision
    {
        return Err(ContextError::Stale.into());
    }
    append(
        actor,
        owner,
        current.tail,
        Operation::Grant {
            grant,
            revoked: false,
        },
    )
    .await
}

pub async fn revoke_grant(
    actor: &ActorEngine,
    owner: &Scope,
    recipient: &Scope,
    kind: SourceKind,
    id: &str,
) -> Result<u64, RetrievalError> {
    let _guard = crate::context_jobs::CONTEXT_MUTATIONS.lock().await;
    let current = state(actor, owner).await?;
    let grant = current
        .grants
        .get(&(recipient.clone(), (kind, id.into())))
        .cloned()
        .ok_or(ContextError::Stale)?;
    append(
        actor,
        owner,
        current.tail,
        Operation::Grant {
            grant,
            revoked: true,
        },
    )
    .await
}
