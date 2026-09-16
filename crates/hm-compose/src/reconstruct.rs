use hm_core::{Error, ErrorCode, LSN};
use hm_llm::{LlmError, LlmProvider, StructuredRequest, Usage};
use hm_schema::events::Authority;
use serde_json::{Value, json};

pub const PROMPT_ID: &str = "reconstruct@1";
pub const LABEL: &str = "RECONSTRUCTION";
pub const PROMPT: &str = include_str!("../../../prompts/reconstruct@1.md");
pub const MAXIMUM_ANCHORS: usize = 32;
pub const MAXIMUM_ANCHOR_BYTES: usize = 65_536;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReconstructionAnchor {
    pub lsn: LSN,
    pub uri: String,
    pub content: String,
    pub authority: Authority,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Reconstruction {
    pub content: String,
    pub authority: Authority,
    pub anchor_lsns: Vec<LSN>,
    pub model_id: String,
    pub prompt_id: &'static str,
    pub usage: Usage,
}

pub fn reconstruct(
    provider: &dyn LlmProvider,
    anchors: &[ReconstructionAnchor],
    maximum_output_tokens: u32,
) -> Result<Reconstruction, LlmError> {
    let request = reconstruction_request(anchors, maximum_output_tokens)?;
    let response = provider.generate_structured(&request)?;
    let object = response
        .value
        .as_object()
        .filter(|object| object.len() == 2)
        .ok_or_else(|| LlmError::Schema("invalid reconstruction object".to_owned()))?;
    let narrative = object
        .get("narrative")
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty() && text.len() <= MAXIMUM_ANCHOR_BYTES)
        .ok_or_else(|| LlmError::Schema("invalid reconstruction narrative".to_owned()))?;
    let cited = object
        .get("anchor_lsns")
        .and_then(Value::as_array)
        .and_then(|lsns| lsns.iter().map(Value::as_u64).collect::<Option<Vec<_>>>())
        .ok_or_else(|| LlmError::Schema("invalid reconstruction anchors".to_owned()))?;
    let expected = anchors
        .iter()
        .map(|anchor| anchor.lsn.get())
        .collect::<Vec<_>>();
    if cited != expected {
        return Err(LlmError::Schema(
            "reconstruction changed its frozen anchor set".to_owned(),
        ));
    }
    Ok(Reconstruction {
        content: format!("{LABEL}\n{narrative}"),
        authority: Authority::AssistantGenerated,
        anchor_lsns: anchors.iter().map(|anchor| anchor.lsn).collect(),
        model_id: response.model_id,
        prompt_id: PROMPT_ID,
        usage: response.usage,
    })
}

pub fn reconstruction_request(
    anchors: &[ReconstructionAnchor],
    maximum_output_tokens: u32,
) -> Result<StructuredRequest, LlmError> {
    if !(2..=MAXIMUM_ANCHORS).contains(&anchors.len())
        || maximum_output_tokens == 0
        || maximum_output_tokens > 4_096
        || anchors.iter().any(|anchor| {
            anchor.lsn.get() == 0
                || !anchor.uri.starts_with("hm://")
                || anchor.content.trim().is_empty()
                || crate::safety::raw_wire_bytes(anchor.content.as_bytes())
                || is_reconstruction(&anchor.content)
        })
        || anchors.windows(2).any(|pair| pair[0].lsn >= pair[1].lsn)
        || anchors
            .iter()
            .map(|anchor| anchor.content.len())
            .sum::<usize>()
            > MAXIMUM_ANCHOR_BYTES
    {
        return Err(LlmError::InvalidArgument(
            "invalid reconstruction anchors or budget",
        ));
    }
    let anchors = anchors
        .iter()
        .map(|anchor| {
            json!({
                "lsn": anchor.lsn.get(),
                "uri": anchor.uri,
                "authority": authority_name(anchor.authority),
                "content": anchor.content,
            })
        })
        .collect::<Vec<_>>();
    Ok(StructuredRequest {
        prompt_id: "reconstruct_1".to_owned(),
        system: PROMPT.to_owned(),
        prompt: json!({ "anchors": anchors }).to_string(),
        json_schema: json!({
            "type": "object",
            "properties": {
                "narrative": { "type": "string" },
                "anchor_lsns": { "type": "array", "items": { "type": "integer" } }
            },
            "required": ["narrative", "anchor_lsns"],
            "additionalProperties": false
        }),
        maximum_output_tokens,
    })
}

#[must_use]
pub fn is_reconstruction(content: &str) -> bool {
    content
        .trim_start()
        .strip_prefix(LABEL)
        .is_some_and(|rest| {
            rest.is_empty() || rest.starts_with(char::is_whitespace) || rest.starts_with(':')
        })
}

pub fn guard_remember(content: &str) -> Result<(), Error> {
    if is_reconstruction(content) {
        Err(Error::new(ErrorCode::ForbiddenKind))
    } else {
        Ok(())
    }
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
