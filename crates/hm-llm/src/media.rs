use crate::{LlmError, ModelTier, StructuredResponse};
use serde_json::Value;

pub const MAXIMUM_ATTACHMENT_BYTES: usize = 8 * 1024 * 1024;

pub const SUPPORTED_AUDIO_MEDIA_TYPES: [&str; 5] = [
    "audio/mp4",
    "audio/mpeg",
    "audio/ogg",
    "audio/wav",
    "audio/webm",
];

pub const SUPPORTED_IMAGE_MEDIA_TYPES: [&str; 4] =
    ["image/gif", "image/jpeg", "image/png", "image/webp"];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MediaModality {
    Audio,
    Image,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaAttachment {
    pub modality: MediaModality,
    pub media_type: String,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MediaRequest {
    pub prompt_id: String,
    pub system: String,
    pub prompt: String,
    pub json_schema: Value,
    pub maximum_output_tokens: u32,
    pub attachment: MediaAttachment,
}

pub trait MediaProvider: Send + Sync {
    fn model_id(&self) -> &str;
    fn tier(&self) -> ModelTier;
    fn describe_media(&self, request: &MediaRequest) -> Result<StructuredResponse, LlmError>;
}

#[must_use]
pub fn audio_format_token(media_type: &str) -> Option<&'static str> {
    match media_type {
        "audio/mp4" => Some("mp4"),
        "audio/mpeg" => Some("mp3"),
        "audio/ogg" => Some("ogg"),
        "audio/wav" => Some("wav"),
        "audio/webm" => Some("webm"),
        _ => None,
    }
}

pub fn validate_media_request(request: &MediaRequest) -> Result<(), LlmError> {
    if request.prompt_id.is_empty()
        || request.prompt.is_empty()
        || request.maximum_output_tokens == 0
        || !request.json_schema.is_object()
    {
        return Err(LlmError::InvalidArgument("media request is incomplete"));
    }
    let attachment = &request.attachment;
    if attachment.bytes.is_empty() {
        return Err(LlmError::InvalidArgument(
            "media attachment carries no bytes",
        ));
    }
    if attachment.bytes.len() > MAXIMUM_ATTACHMENT_BYTES {
        return Err(LlmError::InvalidArgument(
            "media attachment exceeds the attachment ceiling",
        ));
    }
    let admitted = match attachment.modality {
        MediaModality::Audio => {
            SUPPORTED_AUDIO_MEDIA_TYPES.contains(&attachment.media_type.as_str())
                && audio_format_token(&attachment.media_type).is_some()
        }
        MediaModality::Image => {
            SUPPORTED_IMAGE_MEDIA_TYPES.contains(&attachment.media_type.as_str())
        }
    };
    if admitted {
        Ok(())
    } else {
        Err(LlmError::InvalidArgument(
            "media type is outside the modality allowlist",
        ))
    }
}
