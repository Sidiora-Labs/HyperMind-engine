#![allow(clippy::missing_errors_doc)]
//! Derived media text is bounded by length, refusal and UTF-8 only.
//! `crate::quality::assess_thought` is deliberately not applied here: its
//! generic-phrase and length rules are written for minted definitions and
//! would drop a faithful verbatim transcript.

use hm_llm::media::{
    MediaAttachment, MediaModality, MediaProvider, MediaRequest, validate_media_request,
};
use hm_llm::{LlmError, StructuredResponse, Usage};
use hm_schema::events::{Authority, ModelProvenance};
use serde_json::{Value, json};
use std::fmt::Write;

pub const TRANSCRIPT_PROMPT: &str = include_str!("../../../prompts/media-transcript@1.md");
pub const DESCRIPTION_PROMPT: &str = include_str!("../../../prompts/media-description@1.md");
pub const MAXIMUM_DERIVED_BYTES: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MediaDerivationKind {
    Transcript,
    Description,
}

impl MediaDerivationKind {
    const fn text_field(self) -> &'static str {
        match self {
            Self::Transcript => "transcript",
            Self::Description => "description",
        }
    }

    const fn provenance_prompt_id(self) -> &'static str {
        match self {
            Self::Transcript => "media-transcript",
            Self::Description => "media-description",
        }
    }

    const fn domain_byte(self) -> u8 {
        match self {
            Self::Transcript => 1,
            Self::Description => 2,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaSubject {
    pub media_lsn: u64,
    pub media_type: String,
    pub source_uri: String,
    pub digest: [u8; 32],
}

#[derive(Clone, Debug, PartialEq)]
pub struct MediaDerivation {
    pub kind: MediaDerivationKind,
    pub text: String,
    pub language: Option<String>,
    pub confidence_micros: u32,
    pub authority: Authority,
    pub model_provenance: ModelProvenance,
    pub usage: Usage,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MediaDropReason {
    UnsupportedMediaType,
    Refused(String),
    EmptyText,
    Schema(String),
    Provider(LlmError),
}

#[must_use]
pub fn modality_for(media_type: &str) -> Option<MediaModality> {
    match media_type {
        "audio/mp4" | "audio/mpeg" | "audio/ogg" | "audio/wav" | "audio/webm" => {
            Some(MediaModality::Audio)
        }
        "image/gif" | "image/jpeg" | "image/png" | "image/webp" => Some(MediaModality::Image),
        _ => None,
    }
}

pub fn transcript_request(
    subject: &MediaSubject,
    bytes: Vec<u8>,
) -> Result<MediaRequest, MediaDropReason> {
    build_request(subject, bytes, MediaDerivationKind::Transcript)
}

pub fn description_request(
    subject: &MediaSubject,
    bytes: Vec<u8>,
) -> Result<MediaRequest, MediaDropReason> {
    build_request(subject, bytes, MediaDerivationKind::Description)
}

#[must_use]
pub fn response_schema(kind: MediaDerivationKind) -> Value {
    match kind {
        MediaDerivationKind::Transcript => json!({
            "type": "object",
            "properties": {
                "transcript": {"type": "string"},
                "language": {"type": ["string", "null"]},
                "confidence_micros": {"type": "integer", "minimum": 0, "maximum": 1_000_000},
                "refusal": {"type": ["string", "null"]}
            },
            "required": ["transcript", "language", "confidence_micros", "refusal"],
            "additionalProperties": false
        }),
        MediaDerivationKind::Description => json!({
            "type": "object",
            "properties": {
                "description": {"type": "string"},
                "detected_text": {"type": ["string", "null"]},
                "confidence_micros": {"type": "integer", "minimum": 0, "maximum": 1_000_000},
                "refusal": {"type": ["string", "null"]}
            },
            "required": ["description", "detected_text", "confidence_micros", "refusal"],
            "additionalProperties": false
        }),
    }
}

#[must_use]
pub fn call_id(subject: &MediaSubject, kind: MediaDerivationKind) -> [u8; 32] {
    let mut hash = blake3::Hasher::new();
    hash.update(b"hypermind.media-derivation.v1\0");
    hash.update(&subject.digest);
    hash.update(&[kind.domain_byte()]);
    *hash.finalize().as_bytes()
}

pub fn derive(
    provider: &dyn MediaProvider,
    subject: &MediaSubject,
    bytes: Vec<u8>,
) -> Result<MediaDerivation, MediaDropReason> {
    let kind = match modality_for(&subject.media_type) {
        Some(MediaModality::Audio) => MediaDerivationKind::Transcript,
        Some(MediaModality::Image) => MediaDerivationKind::Description,
        None => return Err(MediaDropReason::UnsupportedMediaType),
    };
    let request = build_request(subject, bytes, kind)?;
    let response = provider
        .describe_media(&request)
        .map_err(MediaDropReason::Provider)?;
    validate_response(subject, kind, &response)
}

fn build_request(
    subject: &MediaSubject,
    bytes: Vec<u8>,
    kind: MediaDerivationKind,
) -> Result<MediaRequest, MediaDropReason> {
    let modality =
        modality_for(&subject.media_type).ok_or(MediaDropReason::UnsupportedMediaType)?;
    let expected = match kind {
        MediaDerivationKind::Transcript => MediaModality::Audio,
        MediaDerivationKind::Description => MediaModality::Image,
    };
    if modality != expected {
        return Err(MediaDropReason::UnsupportedMediaType);
    }
    let (prompt_id, system, instruction, maximum_output_tokens) = match kind {
        MediaDerivationKind::Transcript => (
            "media_transcript_1",
            TRANSCRIPT_PROMPT,
            "Transcribe the attached recording.",
            4_096,
        ),
        MediaDerivationKind::Description => (
            "media_description_1",
            DESCRIPTION_PROMPT,
            "Describe the attached image.",
            1_024,
        ),
    };
    let request = MediaRequest {
        prompt_id: prompt_id.to_owned(),
        system: system.to_owned(),
        prompt: prompt_body(subject, instruction),
        json_schema: response_schema(kind),
        maximum_output_tokens,
        attachment: MediaAttachment {
            modality,
            media_type: subject.media_type.clone(),
            bytes,
        },
    };
    validate_media_request(&request).map_err(MediaDropReason::Provider)?;
    Ok(request)
}

fn prompt_body(subject: &MediaSubject, instruction: &str) -> String {
    let mut prompt = String::new();
    let _ = writeln!(prompt, "media_lsn={}", subject.media_lsn);
    let _ = writeln!(prompt, "media_type={}", subject.media_type);
    let _ = writeln!(prompt, "source_uri={}", subject.source_uri);
    let _ = writeln!(
        prompt,
        "digest={}",
        blake3::Hash::from_bytes(subject.digest).to_hex()
    );
    prompt.push_str("---\n");
    prompt.push_str(instruction);
    prompt.push('\n');
    prompt
}

fn validate_response(
    subject: &MediaSubject,
    kind: MediaDerivationKind,
    response: &StructuredResponse,
) -> Result<MediaDerivation, MediaDropReason> {
    if let Some(refusal) = optional_text(&response.value, "refusal")? {
        return Err(MediaDropReason::Refused(refusal));
    }
    let text = response
        .value
        .get(kind.text_field())
        .and_then(Value::as_str)
        .ok_or_else(|| MediaDropReason::Schema(format!("{} field is absent", kind.text_field())))?
        .trim();
    if text.is_empty() {
        return Err(MediaDropReason::EmptyText);
    }
    if text.len() > MAXIMUM_DERIVED_BYTES {
        return Err(MediaDropReason::Schema(format!(
            "derived text of {} bytes exceeds the derivation ceiling",
            text.len()
        )));
    }
    let confidence_micros = response
        .value
        .get("confidence_micros")
        .and_then(Value::as_u64)
        .filter(|micros| *micros <= 1_000_000)
        .and_then(|micros| u32::try_from(micros).ok())
        .ok_or_else(|| {
            MediaDropReason::Schema("confidence_micros is absent or out of range".to_owned())
        })?;
    let (text, language) = match kind {
        MediaDerivationKind::Transcript => {
            (text.to_owned(), optional_text(&response.value, "language")?)
        }
        MediaDerivationKind::Description => {
            let mut description = text.to_owned();
            if let Some(detected) = optional_text(&response.value, "detected_text")? {
                let _ = write!(description, "\n---\ndetected_text={detected}");
            }
            (description, None)
        }
    };
    if text.len() > MAXIMUM_DERIVED_BYTES {
        return Err(MediaDropReason::Schema(format!(
            "derived text of {} bytes exceeds the derivation ceiling",
            text.len()
        )));
    }
    Ok(MediaDerivation {
        kind,
        text,
        language,
        confidence_micros,
        authority: Authority::DerivedInference,
        model_provenance: ModelProvenance {
            model_id: response.model_id.clone(),
            prompt_id: kind.provenance_prompt_id().to_owned(),
            prompt_version: 1,
            temperature: 0.0,
            call_id: Some(call_id(subject, kind).to_vec()),
            input_tokens: response.usage.input_tokens,
            output_tokens: response.usage.output_tokens,
            cache_read_tokens: response.usage.cache_read_tokens,
            cache_write_tokens: response.usage.cache_write_tokens,
            cost_microusd: response.usage.cost_microusd,
        },
        usage: response.usage,
    })
}

fn optional_text(value: &Value, field: &str) -> Result<Option<String>, MediaDropReason> {
    match value.get(field) {
        Some(Value::Null) => Ok(None),
        Some(Value::String(text)) => {
            let text = text.trim();
            if text.is_empty() {
                Ok(None)
            } else {
                Ok(Some(text.to_owned()))
            }
        }
        _ => Err(MediaDropReason::Schema(format!(
            "{field} is absent or is not a nullable string"
        ))),
    }
}
