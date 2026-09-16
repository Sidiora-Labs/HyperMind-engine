use crate::tools::remember::{EmbeddingRuntime, RememberInput, RetentionInput, SensitivityInput};
use crate::tools::websource::media_conversation;
use crate::{DEFAULT_CHUNK_BYTES, Envelope};
use hm_core::{ConversationId, Error, ErrorCode, LSN};
use hm_cortex::media::{MediaDerivation, MediaDerivationKind, MediaDropReason, MediaSubject};
use hm_ledger::frame::EventKind;
use hm_llm::media::MediaProvider;
use hm_llm::openai_compat::OpenAiCompatible;
use hm_llm::outcome::ResponseOutcome;
use hm_llm::{HttpTransport, LlmError, ModelTier, Pricing, ProviderConfig};
use hm_proj::media::MediaRecord;
use hm_schema::event::{self, Boundary, CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{Authority, EventEnvelope, EventPayload, Retention, Sensitivity, UserMsg};
use hm_serve::actor::{ActorEngine, IncomingEvent};
use serde_json::{Value, json};
use std::sync::Arc;

pub const MEDIA_ATTACHMENT_PROVIDER_LABEL: &str = "hm-media-attachment@1";

const MAXIMUM_RETAINED_FRAMES: usize = 64;
const MEDIA_CATALOG_LIMIT: usize = 256;

#[derive(Clone)]
pub struct MediaRuntime {
    pub provider: Arc<dyn MediaProvider>,
}

impl MediaRuntime {
    pub fn from_env() -> Result<Option<Self>, Error> {
        match std::env::var("HM_MEDIA_PROVIDER").as_deref() {
            Err(_) | Ok("") => return Ok(None),
            Ok("centra") => {}
            _ => return Err(Error::new(ErrorCode::InvalidArgument)),
        }
        let api_key = std::env::var("CENTRA_GATEWAY_API_KEY")
            .ok()
            .filter(|key| !key.is_empty())
            .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
        let gateway_url = std::env::var("CENTRA_GATEWAY_URL")
            .unwrap_or_else(|_| "https://gateway.centra.ag/v1".to_owned());
        let provider = OpenAiCompatible::new(
            ProviderConfig {
                endpoint: format!("{}/chat/completions", gateway_url.trim_end_matches('/')),
                api_key: Some(api_key),
                model: "openrouter/openai/gpt-4o-mini".to_owned(),
                tier: ModelTier::Economy,
                pricing: Pricing {
                    input_microusd_per_million_tokens: 150_000,
                    output_microusd_per_million_tokens: 600_000,
                },
            },
            HttpTransport::default(),
        )
        .map_err(|_| Error::new(ErrorCode::InvalidArgument))?;
        Ok(Some(Self {
            provider: Arc::new(provider),
        }))
    }
}

#[allow(clippy::too_many_lines, clippy::single_match_else)]
pub(crate) async fn run(
    actor: &ActorEngine,
    runtime: Option<&MediaRuntime>,
    embedding: Option<&EmbeddingRuntime>,
    input: RememberInput,
) -> Result<Envelope, Error> {
    let runtime = runtime.ok_or_else(|| Error::new(ErrorCode::OperationUnavailable))?;
    let request = input
        .derive
        .as_ref()
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
    let media_lsn = request.media_lsn;
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
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let subject = media_subject(actor, media_lsn).await?;
    if hm_cortex::media::modality_for(&subject.media_type).is_none() {
        return Err(Error::new(ErrorCode::OperationUnavailable));
    }
    if let Some(record) = derived_record(actor, &subject.digest).await? {
        return Ok(recorded_envelope(actor, &subject, &record));
    }
    let bytes = retained_bytes(actor, &subject.digest).await?;
    let provider = Arc::clone(&runtime.provider);
    let called = subject.clone();
    let derivation = tokio::task::spawn_blocking(move || {
        hm_cortex::media::derive(provider.as_ref(), &called, bytes)
    })
    .await
    .map_err(|_| Error::new(ErrorCode::OperationUnavailable))?
    .map_err(|reason| Error::new(drop_code(&reason)))?;
    hm_compose::reconstruct::guard_remember(&derivation.text)?;
    let conversation = ConversationId::derive(&input.conversation);
    let chunks = crate::chunk_text(
        &derivation.text,
        input.chunk_bytes.unwrap_or(DEFAULT_CHUNK_BYTES),
    )?;
    let count = u32::try_from(chunks.len()).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
    let mut events = Vec::with_capacity(chunks.len());
    for (offset, chunk) in chunks.iter().enumerate() {
        let index = u32::try_from(offset).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
        events.push(IncomingEvent {
            kind: EventKind::UserMsg,
            conversation,
            payload: encode_event_envelope(&EventEnvelope {
                schema_version: CURRENT_SCHEMA_VERSION,
                payload: EventPayload::UserMsg(Box::new(UserMsg {
                    content: chunk.as_bytes().to_vec(),
                })),
                connection_id: None,
                client_seq: 0,
                client_event_index: index,
                client_event_count: count,
                origin_actor: 0,
                run_id: None,
                model_provenance: Some(Box::new(derivation.model_provenance.clone())),
                authority: Authority::DerivedInference,
                retention,
                sensitivity,
                event_time_ns: 0,
            }),
        });
    }
    let documents = embedding.map(|_| {
        chunks
            .iter()
            .map(|chunk| (*chunk).to_owned())
            .collect::<Vec<_>>()
    });
    let outcome = actor.append(events).await?;
    let mut envelope = derived_envelope(
        actor,
        &subject,
        &derivation,
        outcome.first_lsn,
        outcome.last_lsn,
    );
    if let (Some(runtime), Some(documents)) = (embedding, documents) {
        match crate::append_embeddings(
            actor,
            runtime,
            documents,
            outcome.first_lsn,
            conversation,
            retention,
            sensitivity,
        )
        .await
        {
            Ok((first, last)) => {
                envelope.items[0]["embedding_first_lsn"] = json!(first.get());
                envelope.items[0]["embedding_last_lsn"] = json!(last.get());
                envelope.health["encoder"] = json!(runtime.health());
                envelope.health["backlog"] = json!("semantic_ready");
            }
            Err(_) => {
                envelope.health["encoder"] = json!("semantic_lagging");
                envelope.health["backlog"] = json!("semantic_lagging");
                envelope.gaps.push(json!({
                    "kind": "embedding_pending",
                    "first_lsn": outcome.first_lsn.get(),
                    "last_lsn": outcome.last_lsn.get(),
                }));
                envelope.warnings.push(
                    "Derivation stored; its embedding was not confirmed committed.".to_owned(),
                );
            }
        }
    } else {
        envelope.health["encoder"] = json!("lexical_only");
    }
    Ok(envelope)
}

async fn media_subject(actor: &ActorEngine, media_lsn: u64) -> Result<MediaSubject, Error> {
    if media_lsn == 0 {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let frame = actor
        .frames_since(LSN::new(media_lsn - 1), None, 1)
        .await?
        .into_iter()
        .next()
        .filter(|frame| {
            frame.header.lsn.get() == media_lsn && frame.header.kind == EventKind::MediaRef
        })
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
    let verified = event::verify_event(
        &frame.sealed_payload,
        event::EventKind::MediaRef,
        Boundary::Disk,
    )?;
    if verified.envelope.authority != Authority::ExternalObserved {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let EventPayload::MediaRef(media) = verified.envelope.payload else {
        return Err(Error::new(ErrorCode::InvalidArgument));
    };
    let digest = <[u8; 32]>::try_from(media.digest.as_slice())
        .map_err(|_| Error::new(ErrorCode::SchemaInvalid))?;
    Ok(MediaSubject {
        media_lsn,
        media_type: media.media_type,
        source_uri: media.uri,
        digest,
    })
}

async fn retained_bytes(actor: &ActorEngine, digest: &[u8; 32]) -> Result<Vec<u8>, Error> {
    let conversation = media_conversation(digest);
    let frames = actor
        .frames_since(LSN::new(0), Some(conversation), MAXIMUM_RETAINED_FRAMES)
        .await?;
    for frame in frames {
        if frame.header.kind != EventKind::ProviderFrame {
            continue;
        }
        let verified = event::verify_event(
            &frame.sealed_payload,
            event::EventKind::ProviderFrame,
            Boundary::Disk,
        )?;
        let EventPayload::ProviderFrame(retained) = verified.envelope.payload else {
            continue;
        };
        if blake3::hash(&retained.api_content).as_bytes() == digest {
            return Ok(retained.api_content);
        }
    }
    Err(Error::new(ErrorCode::OperationUnavailable))
}

async fn derived_record(
    actor: &ActorEngine,
    digest: &[u8; 32],
) -> Result<Option<MediaRecord>, Error> {
    Ok(actor
        .media_catalog(false, MEDIA_CATALOG_LIMIT)
        .await?
        .into_iter()
        .find(|record| record.digest.as_slice() == digest.as_slice() && record.derived_lsn != 0))
}

fn derived_envelope(
    actor: &ActorEngine,
    subject: &MediaSubject,
    derivation: &MediaDerivation,
    first_lsn: LSN,
    last_lsn: LSN,
) -> Envelope {
    let provenance = &derivation.model_provenance;
    let mut envelope = Envelope::empty();
    envelope.items.push(json!({
        "media_lsn": subject.media_lsn,
        "derived_first_lsn": first_lsn.get(),
        "derived_last_lsn": last_lsn.get(),
        "kind": kind_name(derivation.kind),
        "model": provenance.model_id,
        "prompt_id": provenance.prompt_id,
        "prompt_version": provenance.prompt_version,
        "language": derivation.language,
        "confidence_micros": derivation.confidence_micros,
        "already_derived": false,
    }));
    envelope.budget = Some(json!({
        "input_tokens": derivation.usage.input_tokens,
        "output_tokens": derivation.usage.output_tokens,
        "cost_microusd": derivation.usage.cost_microusd,
    }));
    envelope
        .provenance
        .push(format!("hm://{}/lsn/{}", actor.actor(), subject.media_lsn));
    for lsn in first_lsn.get()..=last_lsn.get() {
        envelope
            .provenance
            .push(format!("hm://{}/lsn/{lsn}", actor.actor()));
    }
    envelope.warnings.push(
        "Derived media text is derived_inference with model provenance and never observed evidence."
            .to_owned(),
    );
    envelope
}

fn recorded_envelope(
    actor: &ActorEngine,
    subject: &MediaSubject,
    record: &MediaRecord,
) -> Envelope {
    let mut envelope = Envelope::empty();
    envelope.items.push(json!({
        "media_lsn": subject.media_lsn,
        "derived_first_lsn": record.derived_lsn,
        "derived_last_lsn": record.derived_lsn,
        "kind": kind_name_of(&record.derived_prompt_id),
        "model": Value::Null,
        "prompt_id": record.derived_prompt_id,
        "prompt_version": 1,
        "language": Value::Null,
        "confidence_micros": Value::Null,
        "already_derived": true,
    }));
    envelope.budget = Some(json!({
        "input_tokens": 0,
        "output_tokens": 0,
        "cost_microusd": 0,
    }));
    envelope
        .provenance
        .push(format!("hm://{}/lsn/{}", actor.actor(), subject.media_lsn));
    envelope
        .provenance
        .push(format!("hm://{}/lsn/{}", actor.actor(), record.derived_lsn));
    envelope.health["encoder"] = json!("lexical_only");
    envelope.warnings.push(
        "This media is already derived; the catalogue records its final derived LSN and no provider call was made."
            .to_owned(),
    );
    envelope
}

const fn kind_name(kind: MediaDerivationKind) -> &'static str {
    match kind {
        MediaDerivationKind::Transcript => "transcript",
        MediaDerivationKind::Description => "description",
    }
}

fn kind_name_of(prompt_id: &str) -> &str {
    match prompt_id {
        "media-transcript" => "transcript",
        "media-description" => "description",
        other => other,
    }
}

const fn drop_code(reason: &MediaDropReason) -> ErrorCode {
    match reason {
        MediaDropReason::UnsupportedMediaType
        | MediaDropReason::Refused(_)
        | MediaDropReason::EmptyText => ErrorCode::OperationUnavailable,
        MediaDropReason::Schema(_) => ErrorCode::SchemaInvalid,
        MediaDropReason::Provider(error) => provider_code(error),
    }
}

const fn provider_code(error: &LlmError) -> ErrorCode {
    match error {
        LlmError::Response(fault) => match fault.outcome {
            ResponseOutcome::Refused => ErrorCode::OperationUnavailable,
            ResponseOutcome::Truncated => ErrorCode::CapacityExceeded,
            ResponseOutcome::Malformed | ResponseOutcome::Incomplete => ErrorCode::SchemaInvalid,
        },
        LlmError::InvalidArgument(_) => ErrorCode::InvalidArgument,
        LlmError::Schema(_) | LlmError::Wire(_) => ErrorCode::SchemaInvalid,
        LlmError::Admission(_) | LlmError::Capacity => ErrorCode::CapacityExceeded,
        LlmError::Network(_) => ErrorCode::OperationUnavailable,
    }
}
