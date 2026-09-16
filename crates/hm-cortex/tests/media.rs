#![forbid(unsafe_code)]

use hm_cortex::media::{
    DESCRIPTION_PROMPT, MediaDerivationKind, MediaDropReason, MediaSubject, TRANSCRIPT_PROMPT,
    call_id, derive, description_request, modality_for, response_schema, transcript_request,
};
use hm_llm::media::{MediaModality, MediaRequest};
use hm_llm::openai_compat::OpenAiCompatible;
use hm_llm::{
    ModelTier, Pricing, ProviderConfig, RecordedTransport, WireFixture, WireRequest, WireResponse,
};
use hm_schema::events::Authority;
use serde_json::{Value, json};
use std::collections::BTreeMap;

const ENDPOINT: &str = "https://fixture.invalid/v1/chat/completions";
const AUDIO_BYTES: &[u8] = b"hypermind-fixture-audio";
const AUDIO_BASE64: &str = "aHlwZXJtaW5kLWZpeHR1cmUtYXVkaW8=";
const IMAGE_BYTES: &[u8] = b"hypermind-fixture-image";
const IMAGE_BASE64: &str = "aHlwZXJtaW5kLWZpeHR1cmUtaW1hZ2U=";

fn subject(media_type: &str) -> MediaSubject {
    MediaSubject {
        media_lsn: 4_096,
        media_type: media_type.to_owned(),
        source_uri: "https://source.invalid/clip".to_owned(),
        digest: [7; 32],
    }
}

fn recorded_provider(fixtures: Vec<WireFixture>) -> OpenAiCompatible<RecordedTransport> {
    OpenAiCompatible::new(
        ProviderConfig {
            endpoint: ENDPOINT.to_owned(),
            api_key: Some("fixture-key".to_owned()),
            model: "fixture-model".to_owned(),
            tier: ModelTier::Capable,
            pricing: Pricing {
                input_microusd_per_million_tokens: 1_000_000,
                output_microusd_per_million_tokens: 2_000_000,
            },
        },
        RecordedTransport::new(fixtures),
    )
    .unwrap()
}

fn audio_part() -> Value {
    json!({
        "type": "input_audio",
        "input_audio": {"data": AUDIO_BASE64, "format": "mp3"}
    })
}

fn image_part() -> Value {
    json!({
        "type": "image_url",
        "image_url": {"url": format!("data:image/png;base64,{IMAGE_BASE64}")}
    })
}

fn fixture(request: &MediaRequest, attached: &Value, output: &Value) -> WireFixture {
    WireFixture {
        request: WireRequest {
            method: "POST".to_owned(),
            url: ENDPOINT.to_owned(),
            headers: BTreeMap::from([
                ("authorization".to_owned(), "Bearer fixture-key".to_owned()),
                ("content-type".to_owned(), "application/json".to_owned()),
            ]),
            body: json!({
                "model": "fixture-model",
                "messages": [
                    {"role": "system", "content": request.system},
                    {
                        "role": "user",
                        "content": [{"type": "text", "text": request.prompt}, attached]
                    }
                ],
                "max_tokens": request.maximum_output_tokens,
                "response_format": {
                    "type": "json_schema",
                    "json_schema": {
                        "name": request.prompt_id,
                        "strict": true,
                        "schema": request.json_schema
                    }
                }
            }),
        },
        response: WireResponse {
            status: 200,
            body: json!({
                "choices": [{"message": {"content": output.to_string()}}],
                "usage": {
                    "prompt_tokens": 120,
                    "completion_tokens": 44,
                    "prompt_tokens_details": {"cached_tokens": 12}
                }
            }),
        },
    }
}

#[test]
fn transcript_request_is_frozen_and_carries_the_subject() {
    let audio = subject("audio/mpeg");
    let request = transcript_request(&audio, AUDIO_BYTES.to_vec()).unwrap();
    assert_eq!(request.prompt_id, "media_transcript_1");
    assert_eq!(request.system, TRANSCRIPT_PROMPT);
    assert_eq!(request.maximum_output_tokens, 4_096);
    assert!(request.prompt.contains("media_lsn=4096\n"));
    assert!(request.prompt.contains("media_type=audio/mpeg\n"));
    assert!(
        request
            .prompt
            .contains("source_uri=https://source.invalid/clip\n")
    );
    assert!(
        request
            .prompt
            .contains(&format!("digest={}\n", "07".repeat(32)))
    );
    assert!(request.prompt.contains("\n---\n"));
    assert_eq!(request.attachment.modality, MediaModality::Audio);
    assert_eq!(request.attachment.media_type, "audio/mpeg");
    assert_eq!(request.attachment.bytes, AUDIO_BYTES);
    assert_eq!(
        transcript_request(&audio, AUDIO_BYTES.to_vec()).unwrap(),
        request
    );

    let image = subject("image/png");
    let description = description_request(&image, IMAGE_BYTES.to_vec()).unwrap();
    assert_eq!(description.prompt_id, "media_description_1");
    assert_eq!(description.system, DESCRIPTION_PROMPT);
    assert_eq!(description.maximum_output_tokens, 1_024);
    assert_eq!(description.attachment.modality, MediaModality::Image);

    let schema = response_schema(MediaDerivationKind::Transcript);
    assert_eq!(schema["additionalProperties"], json!(false));
    assert_eq!(
        schema["required"],
        json!(["transcript", "language", "confidence_micros", "refusal"])
    );
    assert_eq!(
        schema["properties"]["language"]["type"],
        json!(["string", "null"])
    );
    assert_eq!(
        schema["properties"]["refusal"]["type"],
        json!(["string", "null"])
    );
    let description_schema = response_schema(MediaDerivationKind::Description);
    assert_eq!(description_schema["additionalProperties"], json!(false));
    assert_eq!(
        description_schema["required"],
        json!([
            "description",
            "detected_text",
            "confidence_micros",
            "refusal"
        ])
    );
}

#[test]
fn recorded_transcription_is_derived_inference_with_model_provenance() {
    let subject = subject("audio/mpeg");
    let request = transcript_request(&subject, AUDIO_BYTES.to_vec()).unwrap();
    let output = json!({
        "transcript": "the ledger seals every frame",
        "language": "en",
        "confidence_micros": 940_000,
        "refusal": null
    });
    let provider = recorded_provider(vec![fixture(&request, &audio_part(), &output)]);
    let derivation = derive(&provider, &subject, AUDIO_BYTES.to_vec()).unwrap();

    assert_eq!(derivation.kind, MediaDerivationKind::Transcript);
    assert_eq!(derivation.text, "the ledger seals every frame");
    assert_eq!(derivation.language.as_deref(), Some("en"));
    assert_eq!(derivation.confidence_micros, 940_000);
    assert_eq!(derivation.authority, Authority::DerivedInference);

    let provenance = &derivation.model_provenance;
    assert_eq!(provenance.model_id, "fixture-model");
    assert_eq!(provenance.prompt_id, "media-transcript");
    assert_eq!(provenance.prompt_version, 1);
    assert!((provenance.temperature - 0.0).abs() < f32::EPSILON);
    let expected_call_id = call_id(&subject, MediaDerivationKind::Transcript);
    assert_eq!(expected_call_id.len(), 32);
    assert_eq!(
        provenance.call_id.as_deref(),
        Some(expected_call_id.as_slice())
    );
    assert_eq!(
        expected_call_id,
        call_id(&subject, MediaDerivationKind::Transcript)
    );
    assert_ne!(
        expected_call_id,
        call_id(&subject, MediaDerivationKind::Description)
    );
    assert_eq!(provenance.input_tokens, 120);
    assert_eq!(provenance.output_tokens, 44);
    assert_eq!(provenance.cache_read_tokens, 12);
    assert_eq!(provenance.cache_write_tokens, 0);
    assert_eq!(provenance.cost_microusd, 208);
    assert_eq!(derivation.usage.input_tokens, 120);
    assert_eq!(derivation.usage.output_tokens, 44);
    assert_eq!(derivation.usage.cost_microusd, 208);
    assert_eq!(provider.transport().remaining(), 0);
}

#[test]
fn recorded_description_produces_detected_text_and_confidence() {
    let subject = subject("image/png");
    let request = description_request(&subject, IMAGE_BYTES.to_vec()).unwrap();
    let output = json!({
        "description": "a sealed ledger page on a dark table",
        "detected_text": "LEDGER 0042",
        "confidence_micros": 820_000,
        "refusal": null
    });
    let provider = recorded_provider(vec![fixture(&request, &image_part(), &output)]);
    let derivation = derive(&provider, &subject, IMAGE_BYTES.to_vec()).unwrap();

    assert_eq!(derivation.kind, MediaDerivationKind::Description);
    assert!(
        derivation
            .text
            .starts_with("a sealed ledger page on a dark table")
    );
    assert!(derivation.text.contains("detected_text=LEDGER 0042"));
    assert_eq!(derivation.language, None);
    assert!(derivation.confidence_micros <= 1_000_000);
    assert_eq!(derivation.confidence_micros, 820_000);
    assert_eq!(derivation.authority, Authority::DerivedInference);
    assert_eq!(derivation.model_provenance.prompt_id, "media-description");
    assert_eq!(derivation.model_provenance.prompt_version, 1);
    assert_eq!(
        derivation.model_provenance.call_id.as_deref(),
        Some(call_id(&subject, MediaDerivationKind::Description).as_slice())
    );
    assert_eq!(provider.transport().remaining(), 0);
}

#[test]
fn refusals_empty_text_and_bad_shapes_are_typed_drops() {
    let subject = subject("audio/mpeg");
    let request = transcript_request(&subject, AUDIO_BYTES.to_vec()).unwrap();
    let outputs = [
        json!({
            "transcript": "",
            "language": null,
            "confidence_micros": 0,
            "refusal": "the recording carries no speech"
        }),
        json!({
            "transcript": "   ",
            "language": null,
            "confidence_micros": 500_000,
            "refusal": null
        }),
        json!({
            "transcript": "the ledger seals every frame",
            "language": null,
            "confidence_micros": 2_000_000,
            "refusal": null
        }),
        json!({
            "transcript": "the ledger seals every frame",
            "language": null,
            "refusal": null
        }),
    ];
    let provider = recorded_provider(
        outputs
            .iter()
            .map(|output| fixture(&request, &audio_part(), output))
            .collect(),
    );

    assert!(matches!(
        derive(&provider, &subject, AUDIO_BYTES.to_vec()),
        Err(MediaDropReason::Refused(reason)) if reason == "the recording carries no speech"
    ));
    assert_eq!(
        derive(&provider, &subject, AUDIO_BYTES.to_vec()),
        Err(MediaDropReason::EmptyText)
    );
    assert!(matches!(
        derive(&provider, &subject, AUDIO_BYTES.to_vec()),
        Err(MediaDropReason::Schema(_))
    ));
    assert!(matches!(
        derive(&provider, &subject, AUDIO_BYTES.to_vec()),
        Err(MediaDropReason::Schema(_))
    ));
    assert_eq!(provider.transport().remaining(), 0);
}

#[test]
fn unsupported_media_type_never_reaches_the_provider() {
    assert_eq!(modality_for("application/pdf"), None);
    assert_eq!(modality_for("audio/flac"), None);
    assert_eq!(modality_for("audio/ogg"), Some(MediaModality::Audio));
    assert_eq!(modality_for("image/webp"), Some(MediaModality::Image));

    let audio = subject("audio/mpeg");
    let request = transcript_request(&audio, AUDIO_BYTES.to_vec()).unwrap();
    let output = json!({
        "transcript": "unused",
        "language": null,
        "confidence_micros": 1_000,
        "refusal": null
    });
    let provider = recorded_provider(vec![fixture(&request, &audio_part(), &output)]);

    let unsupported = subject("application/pdf");
    assert_eq!(
        derive(&provider, &unsupported, AUDIO_BYTES.to_vec()),
        Err(MediaDropReason::UnsupportedMediaType)
    );
    assert_eq!(
        transcript_request(&unsupported, AUDIO_BYTES.to_vec()),
        Err(MediaDropReason::UnsupportedMediaType)
    );
    assert_eq!(
        transcript_request(&subject("image/png"), IMAGE_BYTES.to_vec()),
        Err(MediaDropReason::UnsupportedMediaType)
    );
    assert_eq!(
        description_request(&audio, AUDIO_BYTES.to_vec()),
        Err(MediaDropReason::UnsupportedMediaType)
    );
    assert_eq!(provider.transport().remaining(), 1);
}
