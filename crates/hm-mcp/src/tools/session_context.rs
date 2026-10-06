use crate::Envelope;
use hm_context::{Authority, MessageRole, Scope};
use hm_core::{Error, ErrorCode};
use hm_serve::{
    actor::ActorEngine,
    context_history::{ForkRequest, RelationIngestion, SourceIngestion},
    context_jobs::ContextJobRequest,
};
use serde::Deserialize;
use serde_json::{Value, json};
fn default_import_limit() -> usize {
    128
}
fn invalid() -> Error {
    Error::new(ErrorCode::InvalidArgument)
}
#[derive(Debug, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum RememberContext {
    Source {
        request: SourceIngestion,
    },
    Relation {
        request: RelationIngestion,
    },
    Fork {
        request: ForkRequest,
    },
    Job {
        request: ContextJobRequest,
    },
    Fabric {
        request: hm_serve::fabric_service::FabricRequest,
    },
    Continuity {
        request: hm_serve::continuity_service::ContinuityRequest,
    },
    Development {
        request: hm_serve::development_service::DevelopmentRequest,
    },
    Memory {
        request: hm_serve::context_memory::MemoryRequest,
    },
    RetrievalSource {
        expected_revision: u64,
        source: hm_serve::context_retrieval::SourceRecord,
    },
    RetrievalTombstone {
        kind: hm_context::retrieval::SourceKind,
        id: String,
        expected_revision: u64,
    },
    RetrievalGrant {
        grant: hm_context::retrieval::EvidenceGrant,
    },
    RetrievalRevoke {
        recipient_scope: Scope,
        kind: hm_context::retrieval::SourceKind,
        id: String,
    },
    Embedding {
        expected_revision: u64,
        enabled: bool,
    },
    Backfill {
        maximum_items: usize,
        maximum_bytes: usize,
    },
    Import {
        request: hm_serve::hypermid_import::ImportBundle,
        #[serde(default = "default_import_limit")]
        max_entries: usize,
    },
}
pub async fn remember(
    actor: &ActorEngine,
    scope: &Scope,
    conversation: &str,
    input: Value,
    runtime: Option<&super::remember::EmbeddingRuntime>,
) -> Result<Envelope, Error> {
    remember_with_development(actor, scope, conversation, input, runtime, None).await
}
pub async fn remember_with_development(
    actor: &ActorEngine,
    scope: &Scope,
    conversation: &str,
    input: Value,
    runtime: Option<&super::remember::EmbeddingRuntime>,
    development: Option<&super::remember::DevelopmentRuntime>,
) -> Result<Envelope, Error> {
    remember_with_operational(
        actor,
        scope,
        conversation,
        input,
        runtime,
        development,
        None,
    )
    .await
}
pub async fn remember_with_operational(
    actor: &ActorEngine,
    scope: &Scope,
    conversation: &str,
    input: Value,
    runtime: Option<&super::remember::EmbeddingRuntime>,
    development: Option<&super::remember::DevelopmentRuntime>,
    fabric: Option<&hm_serve::fabric_service::FabricService>,
) -> Result<Envelope, Error> {
    if input
        .get("request")
        .and_then(|r| r.get("version"))
        .and_then(Value::as_u64)
        .is_some_and(|version| version != 1)
    {
        return Err(Error::new(ErrorCode::ProtocolVersion));
    }
    let operation: RememberContext = serde_json::from_value(input).map_err(|_| invalid())?;
    let reply = match operation {
        RememberContext::Fabric { request } => fabric
            .ok_or_else(|| Error::new(ErrorCode::OperationUnavailable))?
            .execute(actor, scope, request)
            .await
            .map_err(fabric_error)?,
        RememberContext::Continuity { request } => {
            hm_serve::continuity_service::execute(actor, scope, scope, request).await?
        }
        RememberContext::Development { request } => {
            let service = development_service(actor, scope, development)?;
            service
                .execute(actor, scope, request)
                .await
                .map_err(hm_serve::session_context::memory_error)?
        }
        RememberContext::Memory { request } => {
            if matches!(&request.command,hm_serve::context_memory::MemoryCommand::Create{record}|hm_serve::context_memory::MemoryCommand::Revise{record,..} if record.authority==Authority::RuntimeFact)
            {
                return Err(Error::new(ErrorCode::ProtectedTypeWrite));
            }
            serde_json::to_value(
                hm_serve::context_memory::execute(actor, scope, scope, request)
                    .await
                    .map_err(hm_serve::session_context::memory_error)?,
            )
            .map_err(|_| Error::new(ErrorCode::SchemaInvalid))?
        }
        RememberContext::RetrievalSource {
            expected_revision,
            source,
        } => {
            let tail = hm_serve::context_retrieval::register_source(
                actor,
                scope,
                expected_revision,
                source,
            )
            .await
            .map_err(hm_serve::session_context::retrieval_error)?;
            json!({"version":1,"scope":scope,"last_lsn":tail})
        }
        RememberContext::RetrievalTombstone {
            kind,
            id,
            expected_revision,
        } => {
            let tail =
                hm_serve::context_retrieval::tombstone(actor, scope, kind, &id, expected_revision)
                    .await
                    .map_err(hm_serve::session_context::retrieval_error)?;
            json!({"version":1,"scope":scope,"last_lsn":tail})
        }
        RememberContext::RetrievalGrant { grant } => {
            let tail = hm_serve::context_retrieval::set_grant(
                actor,
                scope,
                grant,
                hm_serve::session_context::runtime_now_ns()?,
            )
            .await
            .map_err(hm_serve::session_context::retrieval_error)?;
            json!({"version":1,"last_lsn":tail})
        }
        RememberContext::RetrievalRevoke {
            recipient_scope,
            kind,
            id,
        } => {
            let tail = hm_serve::context_retrieval::revoke_grant(
                actor,
                scope,
                &recipient_scope,
                kind,
                &id,
            )
            .await
            .map_err(hm_serve::session_context::retrieval_error)?;
            json!({"version":1,"last_lsn":tail})
        }
        RememberContext::Embedding {
            expected_revision,
            enabled,
        } => {
            let revision = expected_revision
                .checked_add(1)
                .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?;
            let registration = if enabled {
                hm_serve::context_retrieval::EmbeddingProvider::new(
                    "context-embedding",
                    revision,
                    runtime
                        .ok_or_else(|| Error::new(ErrorCode::OperationUnavailable))?
                        .context_mode(),
                    runtime
                        .ok_or_else(|| Error::new(ErrorCode::OperationUnavailable))?
                        .embedder(),
                )
                .map_err(hm_serve::session_context::retrieval_error)?
                .registration
            } else {
                hm_serve::context_retrieval::EmbeddingRegistration {
                    id: "context-embedding".into(),
                    revision,
                    mode: hm_serve::context_retrieval::EmbeddingMode::Off,
                    fingerprint: None,
                }
            };
            let tail = hm_serve::context_retrieval::register_embedding(
                actor,
                scope,
                expected_revision,
                registration.clone(),
            )
            .await
            .map_err(hm_serve::session_context::retrieval_error)?;
            json!({"version":1,"scope":scope,"registration":registration,"last_lsn":tail})
        }
        RememberContext::Backfill {
            maximum_items,
            maximum_bytes,
        } => {
            let provider = configured_provider(actor, scope, runtime).await?;
            serde_json::to_value(
                hm_serve::context_retrieval::backfill(
                    actor,
                    scope,
                    provider.as_ref(),
                    maximum_items,
                    maximum_bytes,
                    hm_serve::session_context::runtime_now_ns()?,
                )
                .await
                .map_err(hm_serve::session_context::retrieval_error)?,
            )
            .map_err(|_| Error::new(ErrorCode::SchemaInvalid))?
        }
        RememberContext::Source { request } => {
            if request.conversation != conversation {
                return Err(invalid());
            }
            let allowed = match request.message.role {
                MessageRole::User => matches!(
                    request.message.authority,
                    Authority::UserAsserted
                        | Authority::ExternalObserved
                        | Authority::DerivedInference
                ),
                MessageRole::Assistant => matches!(
                    request.message.authority,
                    Authority::AssistantGenerated | Authority::DerivedInference
                ),
                MessageRole::Tool => matches!(
                    request.message.authority,
                    Authority::ToolObserved
                        | Authority::ExternalObserved
                        | Authority::DerivedInference
                ),
            };
            if !allowed {
                return Err(invalid());
            }
            serde_json::to_value(
                hm_serve::context_history::ingest(actor, scope, &request)
                    .await
                    .map_err(hm_serve::session_context::history_error)?,
            )
            .map_err(|_| invalid())?
        }
        RememberContext::Relation { request } => {
            if request.conversation != conversation {
                return Err(invalid());
            }
            serde_json::to_value(
                hm_serve::context_history::relate(actor, scope, &request)
                    .await
                    .map_err(hm_serve::session_context::history_error)?,
            )
            .map_err(|_| invalid())?
        }
        RememberContext::Fork { request } => {
            if request.child_conversation != conversation {
                return Err(invalid());
            }
            serde_json::to_value(
                hm_serve::context_history::fork(actor, scope, &request)
                    .await
                    .map_err(hm_serve::session_context::history_error)?,
            )
            .map_err(|_| invalid())?
        }
        RememberContext::Import {
            request,
            max_entries,
        } => {
            if request.scope != *scope || max_entries == 0 || max_entries > 256 {
                return Err(invalid());
            }
            let mut registry = hm_context::temporal::IdentityRegistry::new();
            registry
                .bind(scope.clone(), actor.actor().get(), Vec::new())
                .map_err(|_| invalid())?;
            serde_json::to_value(
                hm_serve::hypermid_import::import_batch(actor, &registry, &request, max_entries)
                    .await
                    .map_err(hm_serve::session_context::import_error)?,
            )
            .map_err(|_| invalid())?
        }
        RememberContext::Job { request } => {
            hm_serve::context_jobs::execute(actor, scope, &scope.owner_id, request).await?
        }
    };
    Ok(envelope(reply))
}
pub async fn activate(
    actor: &ActorEngine,
    conversation: &str,
    scope: &Scope,
    input: Value,
    runtime: Option<&super::remember::EmbeddingRuntime>,
) -> Result<Envelope, Error> {
    let provider = configured_provider(actor, scope, runtime).await?;
    let request = serde_json::from_value(input).map_err(|_| invalid())?;
    Ok(envelope(
        hm_serve::session_context::activate_with_provider(
            actor,
            conversation,
            scope,
            request,
            provider.as_ref(),
        )
        .await?,
    ))
}
pub async fn inspect(
    actor: &ActorEngine,
    scope: Option<&Scope>,
    uri: &str,
    runtime: Option<&super::remember::EmbeddingRuntime>,
) -> Result<Envelope, Error> {
    inspect_with_development(actor, scope, uri, runtime, None).await
}
pub async fn inspect_with_development(
    actor: &ActorEngine,
    scope: Option<&Scope>,
    uri: &str,
    runtime: Option<&super::remember::EmbeddingRuntime>,
    development: Option<&super::remember::DevelopmentRuntime>,
) -> Result<Envelope, Error> {
    inspect_with_operational(actor, scope, uri, runtime, development, None).await
}
pub async fn inspect_with_operational(
    actor: &ActorEngine,
    scope: Option<&Scope>,
    uri: &str,
    runtime: Option<&super::remember::EmbeddingRuntime>,
    development: Option<&super::remember::DevelopmentRuntime>,
    fabric: Option<&hm_serve::fabric_service::FabricService>,
) -> Result<Envelope, Error> {
    let scope = scope.ok_or_else(invalid)?;
    if uri == format!("hm://{}/context-fabric", actor.actor()) {
        return Ok(envelope(
            fabric
                .ok_or_else(|| Error::new(ErrorCode::OperationUnavailable))?
                .inspect(scope, hm_context::Cursor::default(), 128)
                .await
                .map_err(fabric_error)?,
        ));
    }

    if uri == format!("hm://{}/context-development", actor.actor()) {
        let service = development_service(actor, scope, development)?;
        let mut state = service
            .inspect(actor, scope)
            .await
            .map_err(hm_serve::session_context::memory_error)?;
        state["runtime"] = super::remember::DevelopmentRuntime::metadata(development);
        return Ok(envelope(state));
    }
    if uri == format!("hm://{}/context-memory-export", actor.actor()) {
        return Ok(envelope(
            hm_serve::session_context::export_memory(actor, scope).await?,
        ));
    }
    if uri.starts_with(&format!("hm://{}/context-memory", actor.actor())) {
        return memory_inspect(actor, scope, uri).await;
    }
    if uri == format!("hm://{}/context-retrieval", actor.actor()) {
        return Ok(envelope(
            hm_serve::context_retrieval::inspect(actor, scope)
                .await
                .map_err(hm_serve::session_context::retrieval_error)?,
        ));
    }
    let provider = configured_provider(actor, scope, runtime).await?;
    let base = format!("hm://{}/context", actor.actor());
    if uri == format!("{base}-jobs") {
        let _guard = hm_serve::context_jobs::CONTEXT_MUTATIONS.lock().await;
        return Ok(envelope(
            hm_serve::context_jobs::inspect_state(actor, scope, &scope.owner_id).await?,
        ));
    }
    if uri == base {
        return Ok(envelope(
            hm_serve::session_context::inspect_with_provider(actor, scope, None, provider.as_ref())
                .await?,
        ));
    }
    let tail = uri.strip_prefix(&(base + "/")).ok_or_else(invalid)?;
    let (session, suffix) = tail
        .split_once('/')
        .map_or((tail, None), |(s, x)| (s, Some(x)));
    let session = decode(session)?;
    let state = match suffix {
        None => {
            hm_serve::session_context::inspect_with_provider(
                actor,
                scope,
                Some(&session),
                provider.as_ref(),
            )
            .await?
        }
        Some("history") => {
            hm_serve::session_context::inspect_history(actor, scope, &session, None).await?
        }
        Some(path) if path.starts_with("source/") => {
            let source = decode(path.strip_prefix("source/").unwrap())?;
            hm_serve::session_context::inspect_history(actor, scope, &session, Some(&source))
                .await?
        }
        _ => return Err(invalid()),
    };
    Ok(envelope(state))
}
fn envelope(state: Value) -> Envelope {
    let mut envelope = Envelope::empty();
    if let Some(values) = state["provenance"].as_array() {
        envelope.provenance = values
            .iter()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect();
    }
    envelope.health = json!({"context":if state.get("report").is_some(){"assembled"}else{"operation"},"tokenizer":if state.get("report").is_some(){"supported"}else{"not_required"},"background":"ledger_backed"});
    envelope.budget = state
        .get("report")
        .map(|r| json!({"spent_tokens":r["token_count"]}));
    envelope.gaps = state["gaps"].as_array().cloned().unwrap_or_default();
    envelope.items.push(state);
    envelope
}
fn decode(value: &str) -> Result<String, Error> {
    let mut out = Vec::new();
    let bytes = value.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err(invalid());
            }
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).map_err(|_| invalid())?;
            out.push(u8::from_str_radix(hex, 16).map_err(|_| invalid())?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| invalid())
}

async fn configured_provider(
    actor: &ActorEngine,
    scope: &Scope,
    runtime: Option<&super::remember::EmbeddingRuntime>,
) -> Result<Option<hm_serve::context_retrieval::EmbeddingProvider>, Error> {
    let state = hm_serve::context_retrieval::inspect(actor, scope)
        .await
        .map_err(hm_serve::session_context::retrieval_error)?;
    if state["registration"].is_null() {
        return Ok(None);
    }
    let registration: hm_serve::context_retrieval::EmbeddingRegistration =
        serde_json::from_value(state["registration"].clone())
            .map_err(|_| Error::new(ErrorCode::SchemaInvalid))?;
    if registration.mode == hm_serve::context_retrieval::EmbeddingMode::Off {
        return Ok(None);
    }
    let Some(runtime) = runtime else {
        return Ok(None);
    };
    if registration.mode != runtime.context_mode() {
        return Err(Error::new(ErrorCode::SequenceViolation));
    }
    let provider = hm_serve::context_retrieval::EmbeddingProvider::new(
        &registration.id,
        registration.revision,
        registration.mode,
        runtime.embedder(),
    )
    .map_err(hm_serve::session_context::retrieval_error)?;
    if provider.registration != registration {
        return Err(Error::new(ErrorCode::SequenceViolation));
    }
    Ok(Some(provider))
}
async fn memory_inspect(
    actor: &ActorEngine,
    principal: &Scope,
    uri: &str,
) -> Result<Envelope, Error> {
    let (path, query) = uri
        .split_once('?')
        .map_or((uri, None), |(p, q)| (p, Some(q)));
    let owner = if let Some(query) = query {
        let value = decode(query.strip_prefix("scope=").ok_or_else(invalid)?)?;
        serde_json::from_str(&value).map_err(|_| invalid())?
    } else {
        principal.clone()
    };
    let base = format!("hm://{}/context-memory", actor.actor());
    if path == base {
        return Ok(envelope(
            hm_serve::session_context::inspect_memory(actor, principal, &owner, None, None).await?,
        ));
    }
    let tail = path.strip_prefix(&(base + "/")).ok_or_else(invalid)?;
    let (id, source) = tail
        .split_once("/source/")
        .map_or((tail, None), |(id, source)| (id, Some(source)));
    let id = decode(id)?;
    let source = source.map(decode).transpose()?;
    Ok(envelope(
        hm_serve::session_context::inspect_memory(
            actor,
            principal,
            &owner,
            Some(&id),
            source.as_deref(),
        )
        .await?,
    ))
}

fn development_service(
    actor: &ActorEngine,
    scope: &Scope,
    runtime: Option<&super::remember::DevelopmentRuntime>,
) -> Result<hm_serve::development_service::DevelopmentService, Error> {
    match runtime {
        Some(runtime) => runtime.service(actor.clone(), scope.clone()),
        None => hm_serve::development_service::DevelopmentService::new(scope.clone(), vec![])
            .map_err(hm_serve::session_context::memory_error),
    }
}

pub(crate) fn fabric_error(error: hm_serve::fabric_service::FabricError) -> Error {
    use hm_serve::fabric_service::FabricError;
    match error {
        FabricError::Context(error) => hm_serve::session_context::context_error(error),
        FabricError::Runtime(hm_fabric::runtime::RuntimeError::Context(error))
        | FabricError::Backend(hm_fabric::backend_runtime::BackendRuntimeError::Context(error)) => {
            hm_serve::session_context::context_error(error)
        }
        FabricError::Backend(hm_fabric::backend_runtime::BackendRuntimeError::Conflict) => {
            Error::new(ErrorCode::IdempotencyConflict)
        }
        FabricError::Ledger(error) => error,
        FabricError::Json(_) => Error::new(ErrorCode::SchemaInvalid),
        FabricError::Cancelled { .. } | FabricError::EvidenceUnavailable { .. } => {
            Error::new(ErrorCode::OperationUnavailable)
        }
        _ => Error::new(ErrorCode::OperationUnavailable),
    }
}
