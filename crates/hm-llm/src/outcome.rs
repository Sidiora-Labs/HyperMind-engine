use crate::{LlmError, Usage};
use serde_json::Value;

pub const RESPONSE_CONTRACT_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResponseOutcome {
    Refused,
    Truncated,
    Malformed,
    Incomplete,
}

impl ResponseOutcome {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Refused => "refused",
            Self::Truncated => "truncated",
            Self::Malformed => "malformed",
            Self::Incomplete => "incomplete",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResponseFault {
    pub version: u16,
    pub outcome: ResponseOutcome,
    pub model_id: String,
    pub detail: String,
    pub requested_output_tokens: u32,
    pub usage: Usage,
}

impl ResponseFault {
    #[must_use]
    pub fn new(
        outcome: ResponseOutcome,
        model_id: &str,
        detail: String,
        requested_output_tokens: u32,
        usage: Usage,
    ) -> Self {
        Self {
            version: RESPONSE_CONTRACT_VERSION,
            outcome,
            model_id: model_id.to_owned(),
            detail,
            requested_output_tokens,
            usage,
        }
    }
}

impl From<ResponseFault> for LlmError {
    fn from(fault: ResponseFault) -> Self {
        Self::Response(fault)
    }
}

pub fn decode_structured_text(
    content: Option<&str>,
    model_id: &str,
    requested_output_tokens: u32,
    usage: Usage,
) -> Result<Value, ResponseFault> {
    let text = content.unwrap_or_default().trim();
    if text.is_empty() {
        return Err(ResponseFault::new(
            ResponseOutcome::Incomplete,
            model_id,
            "structured response carried no content".to_owned(),
            requested_output_tokens,
            usage,
        ));
    }
    serde_json::from_str(text).map_err(|error| {
        ResponseFault::new(
            ResponseOutcome::Malformed,
            model_id,
            error.to_string(),
            requested_output_tokens,
            usage,
        )
    })
}
