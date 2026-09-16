#![forbid(unsafe_code)]

use hm_llm::media::{
    MAXIMUM_ATTACHMENT_BYTES, MediaAttachment, MediaModality, MediaProvider, MediaRequest,
};
use hm_llm::openai_compat::OpenAiCompatible;
use hm_llm::{LlmError, ModelTier, Pricing, ProviderConfig, RecordedTransport};
use serde_json::json;

const AUDIO_FIXTURE: &str = include_str!("fixtures/media-audio.json");
const IMAGE_FIXTURE: &str = include_str!("fixtures/media-image.json");

fn config() -> ProviderConfig {
    ProviderConfig {
        endpoint: "https://fixture.invalid/v1/chat/completions".to_owned(),
        api_key: Some("fixture-key".to_owned()),
        model: "fixture-model".to_owned(),
        tier: ModelTier::Capable,
        pricing: Pricing {
            input_microusd_per_million_tokens: 1_000_000,
            output_microusd_per_million_tokens: 2_000_000,
        },
    }
}

fn transcript_request(media_type: &str, bytes: Vec<u8>) -> MediaRequest {
    MediaRequest {
        prompt_id: "media_transcript_1".to_owned(),
        system: "Transcribe only what is audible.".to_owned(),
        prompt: "Transcribe the attached recording.".to_owned(),
        json_schema: json!({
            "type": "object",
            "properties": {
                "transcript": {"type": "string"},
                "language": {"type": ["string", "null"]},
                "confidence_micros": {"type": "integer"},
                "refusal": {"type": ["string", "null"]}
            },
            "required": ["transcript", "language", "confidence_micros", "refusal"],
            "additionalProperties": false
        }),
        maximum_output_tokens: 256,
        attachment: MediaAttachment {
            modality: MediaModality::Audio,
            media_type: media_type.to_owned(),
            bytes,
        },
    }
}

fn description_request(media_type: &str, bytes: Vec<u8>) -> MediaRequest {
    MediaRequest {
        prompt_id: "media_description_1".to_owned(),
        system: "Describe only what is visible.".to_owned(),
        prompt: "Describe the attached frame.".to_owned(),
        json_schema: json!({
            "type": "object",
            "properties": {
                "description": {"type": "string"},
                "detected_text": {"type": ["string", "null"]},
                "confidence_micros": {"type": "integer"},
                "refusal": {"type": ["string", "null"]}
            },
            "required": ["description", "detected_text", "confidence_micros", "refusal"],
            "additionalProperties": false
        }),
        maximum_output_tokens: 256,
        attachment: MediaAttachment {
            modality: MediaModality::Image,
            media_type: media_type.to_owned(),
            bytes,
        },
    }
}

#[test]
fn audio_transcription_round_trips_through_the_recorded_wire() {
    let transport = RecordedTransport::from_json(AUDIO_FIXTURE).unwrap();
    let provider = OpenAiCompatible::new(config(), transport).unwrap();
    let response = provider
        .describe_media(&transcript_request(
            "audio/mpeg",
            b"fixture-audio-bytes".to_vec(),
        ))
        .unwrap();
    assert_eq!(response.model_id, "fixture-model");
    assert_eq!(response.tier, ModelTier::Capable);
    assert_eq!(response.value["transcript"], "the ledger seals every frame");
    assert_eq!(response.value["language"], "en");
    assert_eq!(response.value["confidence_micros"], 940_000);
    assert!(response.value["refusal"].is_null());
    assert_eq!(response.usage.input_tokens, 12);
    assert_eq!(response.usage.output_tokens, 5);
    assert_eq!(response.usage.cache_read_tokens, 3);
    assert_eq!(response.usage.cost_microusd, 22);
    assert_eq!(provider.transport().remaining(), 0);
}

#[test]
fn image_description_round_trips_through_the_recorded_wire() {
    assert!(IMAGE_FIXTURE.contains("data:image/png;base64,Zml4dHVyZS1pbWFnZS1ieXRlcw=="));
    let transport = RecordedTransport::from_json(IMAGE_FIXTURE).unwrap();
    let provider = OpenAiCompatible::new(config(), transport).unwrap();
    let response = provider
        .describe_media(&description_request(
            "image/png",
            b"fixture-image-bytes".to_vec(),
        ))
        .unwrap();
    assert_eq!(
        response.value["description"],
        "a sealed ledger page on a dark table"
    );
    assert!(response.value["detected_text"].is_null());
    assert_eq!(response.value["confidence_micros"], 820_000);
    assert_eq!(response.usage.input_tokens, 20);
    assert_eq!(response.usage.output_tokens, 7);
    assert_eq!(response.usage.cache_read_tokens, 4);
    assert_eq!(response.usage.cost_microusd, 34);
    assert_eq!(provider.transport().remaining(), 0);
}

#[test]
fn unsupported_and_oversize_attachments_are_refused_before_the_wire() {
    let transport = RecordedTransport::from_json(AUDIO_FIXTURE).unwrap();
    let provider = OpenAiCompatible::new(config(), transport).unwrap();
    let refused = [
        transcript_request("audio/flac", b"fixture-audio-bytes".to_vec()),
        description_request("image/svg+xml", b"fixture-image-bytes".to_vec()),
        transcript_request("audio/mpeg", Vec::new()),
        transcript_request("audio/mpeg", vec![0; MAXIMUM_ATTACHMENT_BYTES + 1]),
    ];
    for request in &refused {
        assert!(
            matches!(
                provider.describe_media(request),
                Err(LlmError::InvalidArgument(_))
            ),
            "{} attachment of {} bytes reached the wire",
            request.attachment.media_type,
            request.attachment.bytes.len()
        );
    }
    assert_eq!(provider.transport().remaining(), 1);
}
