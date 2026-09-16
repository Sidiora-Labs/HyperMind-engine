use crate::{Envelope, RecallInput};
use hm_compose::reconstruct::{ReconstructionAnchor, reconstruct};
use hm_core::{Error, ErrorCode, LSN};
use hm_llm::openai_compat::OpenAiCompatible;
use hm_llm::{HttpTransport, LlmProvider, ModelTier, Pricing, ProviderConfig};
use hm_serve::actor::ActorEngine;
use serde_json::json;
use std::sync::Arc;

#[derive(Clone)]
pub struct ReconstructionRuntime {
    pub provider: Arc<dyn LlmProvider>,
}

impl ReconstructionRuntime {
    pub fn from_env() -> Result<Option<Self>, Error> {
        match std::env::var("HM_RECONSTRUCTION_PROVIDER").as_deref() {
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
                model: "openrouter/openai/gpt-5.6-luna".to_owned(),
                tier: ModelTier::Economy,
                pricing: Pricing {
                    input_microusd_per_million_tokens: 200_000,
                    output_microusd_per_million_tokens: 1_200_000,
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

pub async fn run(
    actor: &ActorEngine,
    runtime: Option<&ReconstructionRuntime>,
    input: RecallInput,
) -> Result<Envelope, Error> {
    let runtime = runtime.ok_or_else(|| Error::new(ErrorCode::OperationUnavailable))?;
    let lsns = input.filters.anchor_lsns;
    if !(2..=hm_compose::reconstruct::MAXIMUM_ANCHORS).contains(&lsns.len())
        || lsns.contains(&0)
        || lsns.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let mut anchors = Vec::with_capacity(lsns.len());
    actor
        .guard_tripwires(lsns.iter().copied().map(LSN::new).collect())
        .await?;
    for lsn in lsns {
        let frame = actor
            .frames_since(LSN::new(lsn - 1), None, 1)
            .await?
            .into_iter()
            .next()
            .filter(|frame| frame.header.lsn.get() == lsn)
            .ok_or_else(|| Error::new(ErrorCode::CitationInvalid).at_lsn(LSN::new(lsn)))?;
        let verified = actor.verified_event(frame.header.lsn).await?;
        let authority = verified.envelope.authority;
        let bytes = match verified.envelope.payload {
            hm_schema::events::EventPayload::UserMsg(value) => value.content,
            hm_schema::events::EventPayload::DeliveredMsg(value) => value.content,
            hm_schema::events::EventPayload::ToolResult(value) => value.result,
            hm_schema::events::EventPayload::ProviderFrame(value) => value.api_content,
            _ => return Err(Error::new(ErrorCode::ForbiddenKind)),
        };
        let content = String::from_utf8(bytes).map_err(|_| Error::new(ErrorCode::SchemaInvalid))?;
        anchors.push(ReconstructionAnchor {
            lsn: frame.header.lsn,
            uri: format!("hm://{}/{}/{lsn}", actor.actor(), frame.header.conversation),
            content,
            authority,
        });
    }
    let provider = runtime.provider.clone();
    let maximum = input.filters.maximum_output_tokens.unwrap_or(256);
    let provenance = anchors.iter().map(|anchor| anchor.uri.clone()).collect();
    let result =
        tokio::task::spawn_blocking(move || reconstruct(provider.as_ref(), &anchors, maximum))
            .await
            .map_err(|_| Error::new(ErrorCode::OperationUnavailable))?
            .map_err(|error| {
                Error::new(match error {
                    hm_llm::LlmError::InvalidArgument(_) => ErrorCode::InvalidArgument,
                    hm_llm::LlmError::Schema(_) | hm_llm::LlmError::Wire(_) => {
                        ErrorCode::SchemaInvalid
                    }
                    hm_llm::LlmError::Response(fault) => match fault.outcome {
                        hm_llm::outcome::ResponseOutcome::Refused => {
                            ErrorCode::OperationUnavailable
                        }
                        hm_llm::outcome::ResponseOutcome::Truncated => ErrorCode::CapacityExceeded,
                        hm_llm::outcome::ResponseOutcome::Malformed
                        | hm_llm::outcome::ResponseOutcome::Incomplete => ErrorCode::SchemaInvalid,
                    },
                    hm_llm::LlmError::Admission(_) | hm_llm::LlmError::Capacity => {
                        ErrorCode::CapacityExceeded
                    }
                    hm_llm::LlmError::Network(_) => ErrorCode::OperationUnavailable,
                })
            })?;
    let mut envelope = Envelope::empty();
    envelope.provenance = provenance;
    envelope.items.push(json!({"content":result.content,"authority":"assistant_generated",
        "label":"RECONSTRUCTION","anchor_lsns":result.anchor_lsns.iter().map(|lsn|lsn.get()).collect::<Vec<_>>(),
        "model_id":result.model_id,"prompt_id":result.prompt_id}));
    envelope.budget = Some(
        json!({"input_tokens":result.usage.input_tokens,"output_tokens":result.usage.output_tokens,
        "cost_microusd":result.usage.cost_microusd,"maximum_output_tokens":maximum}),
    );
    envelope.warnings.push(
        "Assistant-generated reconstruction is not evidence and cannot be remembered verbatim."
            .to_owned(),
    );
    Ok(envelope)
}
