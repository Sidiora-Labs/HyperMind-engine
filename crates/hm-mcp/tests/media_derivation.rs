#![forbid(unsafe_code)]

use hm_core::{ActorId, ConversationId, ErrorCode, LSN};
use hm_cortex::media::{MediaDerivationKind, MediaSubject};
use hm_ledger::frame::EventKind;
use hm_llm::openai_compat::OpenAiCompatible;
use hm_llm::{ModelTier, Pricing, ProviderConfig, RecordedTransport};
use hm_mcp::{
    ActivateInput, Envelope, McpServer, MediaRuntime, RecallFilters, RecallInput, RecallMode,
    RememberDerive, RememberInput, RememberKind,
};
use hm_schema::event::{
    Boundary, CURRENT_SCHEMA_VERSION, EventHistory, encode_event_envelope, verify_event,
};
use hm_schema::events::{
    Authority, EventEnvelope, EventPayload, MediaRef, ProviderFrame, Retention, Sensitivity,
};
use hm_schema::validate::authority::validate_observed_evidence;
use hm_serve::actor::{ActorConfig, ActorEngine, IncomingEvent};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

const MEDIA_URI: &str = "https://example.test/media/briefing.mp3";
const MEDIA_TYPE: &str = "audio/mpeg";
const MEDIA_BYTES: &[u8] = b"hypermind media derivation fixture bytes 0123456789";
const CONVERSATION: &str = "briefing";
const FIXTURES: &str = include_str!("fixtures/media-derivation-centra.json");

fn config(path: &Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("7"),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

fn provider_config() -> ProviderConfig {
    ProviderConfig {
        endpoint: "https://fixture.invalid/v1/chat/completions".to_owned(),
        api_key: Some("fixture-key".to_owned()),
        model: "openrouter/openai/gpt-4o-mini".to_owned(),
        tier: ModelTier::Economy,
        pricing: Pricing {
            input_microusd_per_million_tokens: 150_000,
            output_microusd_per_million_tokens: 600_000,
        },
    }
}

fn provider(index: usize) -> Arc<OpenAiCompatible<RecordedTransport>> {
    let recorded: Value = serde_json::from_str(FIXTURES).unwrap();
    let selected = Value::Array(vec![recorded[index].clone()]);
    Arc::new(
        OpenAiCompatible::new(
            provider_config(),
            RecordedTransport::from_json(&selected.to_string()).unwrap(),
        )
        .unwrap(),
    )
}

fn expected_transcript() -> String {
    let recorded: Value = serde_json::from_str(FIXTURES).unwrap();
    let content = recorded[0]["response"]["body"]["choices"][0]["message"]["content"]
        .as_str()
        .unwrap();
    let parsed: Value = serde_json::from_str(content).unwrap();
    parsed["transcript"].as_str().unwrap().to_owned()
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(
        String::with_capacity(bytes.len() * 2),
        |mut output, byte| {
            let _ = write!(output, "{byte:02x}");
            output
        },
    )
}

fn observed(
    kind: EventKind,
    conversation: ConversationId,
    payload: EventPayload,
    index: u32,
) -> IncomingEvent {
    IncomingEvent {
        kind,
        conversation,
        payload: encode_event_envelope(&EventEnvelope {
            schema_version: CURRENT_SCHEMA_VERSION,
            payload,
            connection_id: None,
            client_seq: 0,
            client_event_index: index,
            client_event_count: 2,
            origin_actor: 0,
            run_id: None,
            model_provenance: None,
            authority: Authority::ExternalObserved,
            retention: Retention::Durable,
            sensitivity: Sensitivity::Personal,
            event_time_ns: 0,
        }),
    }
}

async fn seed(actor: &ActorEngine, media_type: &str) -> (u64, u64) {
    let digest = blake3::hash(MEDIA_BYTES);
    let outcome = actor
        .append(vec![
            observed(
                EventKind::MediaRef,
                ConversationId::derive(CONVERSATION),
                EventPayload::MediaRef(Box::new(MediaRef {
                    uri: MEDIA_URI.to_owned(),
                    media_type: media_type.to_owned(),
                    digest: digest.as_bytes().to_vec(),
                })),
                0,
            ),
            observed(
                EventKind::ProviderFrame,
                ConversationId::derive(&format!("hm-media-v1/{}", hex(digest.as_bytes()))),
                EventPayload::ProviderFrame(Box::new(ProviderFrame {
                    provider: "hm-web-source@1".to_owned(),
                    api_content: MEDIA_BYTES.to_vec(),
                })),
                1,
            ),
        ])
        .await
        .unwrap();
    (outcome.first_lsn.get(), outcome.last_lsn.get())
}

fn subject(media_lsn: u64) -> MediaSubject {
    MediaSubject {
        media_lsn,
        media_type: MEDIA_TYPE.to_owned(),
        source_uri: MEDIA_URI.to_owned(),
        digest: *blake3::hash(MEDIA_BYTES).as_bytes(),
    }
}

fn derive_input(media_lsn: u64) -> RememberInput {
    RememberInput {
        conversation: CONVERSATION.to_owned(),
        content: String::new(),
        kind: RememberKind::Document,
        chunk_bytes: None,
        anchor: None,
        retention: None,
        sensitivity: None,
        vocabulary: None,
        source: None,
        derive: Some(RememberDerive { media_lsn }),
        source_delivery: None,
        source_settlement: None,
        document: None,
    }
}

fn lexical(query: &str) -> RecallInput {
    RecallInput {
        mode: RecallMode::Lexical,
        query: query.to_owned(),
        conversation: CONVERSATION.to_owned(),
        limit: 32,
        since_lsn: 0,
        filters: RecallFilters::default(),
    }
}

struct LedgerHistory {
    records: BTreeMap<u64, (hm_schema::event::EventKind, Authority)>,
}

impl EventHistory for LedgerHistory {
    fn kind_at(&self, lsn: LSN) -> Option<hm_schema::event::EventKind> {
        self.records.get(&lsn.get()).map(|record| record.0)
    }

    fn authority_at(&self, lsn: LSN) -> Option<Authority> {
        self.records.get(&lsn.get()).map(|record| record.1)
    }
}

async fn ledger_history(actor: &ActorEngine) -> LedgerHistory {
    let mut records = BTreeMap::new();
    for frame in actor
        .frames_since(LSN::new(0), None, usize::MAX)
        .await
        .unwrap()
    {
        let kind = hm_schema::event::EventKind::try_from(frame.header.kind as u8).unwrap();
        let verified = verify_event(&frame.sealed_payload, kind, Boundary::Disk).unwrap();
        records.insert(frame.header.lsn.get(), (kind, verified.envelope.authority));
    }
    LedgerHistory { records }
}

fn derived_range(envelope: &Envelope) -> (u64, u64) {
    (
        envelope.items[0]["derived_first_lsn"].as_u64().unwrap(),
        envelope.items[0]["derived_last_lsn"].as_u64().unwrap(),
    )
}

#[tokio::test]
async fn audio_derivation_appends_derived_inference_text_with_provenance() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(temporary.path())).await.unwrap();
    let (media_lsn, _) = seed(&actor, MEDIA_TYPE).await;
    let recorded = provider(0);
    let server = McpServer::new(actor.clone()).with_media_runtime(MediaRuntime {
        provider: recorded.clone(),
    });

    let envelope = server.remember_envelope(derive_input(media_lsn)).await;
    assert!(envelope.ok, "{envelope:?}");
    assert_eq!(recorded.transport().remaining(), 0);

    let item = &envelope.items[0];
    assert_eq!(item["media_lsn"].as_u64(), Some(media_lsn));
    assert_eq!(item["kind"].as_str(), Some("transcript"));
    assert_eq!(
        item["model"].as_str(),
        Some("openrouter/openai/gpt-4o-mini")
    );
    assert_eq!(item["prompt_id"].as_str(), Some("media-transcript"));
    assert_eq!(item["prompt_version"].as_u64(), Some(1));
    assert_eq!(item["language"].as_str(), Some("en"));
    assert_eq!(item["confidence_micros"].as_u64(), Some(940_000));
    assert_eq!(item["already_derived"].as_bool(), Some(false));

    let budget = envelope.budget.as_ref().unwrap();
    assert_eq!(budget["input_tokens"].as_u64(), Some(512));
    assert_eq!(budget["output_tokens"].as_u64(), Some(24));
    assert_eq!(budget["cost_microusd"].as_u64(), Some(92));

    let (first, last) = derived_range(&envelope);
    assert!(first > media_lsn && last >= first);
    let expected_call_id =
        hm_cortex::media::call_id(&subject(media_lsn), MediaDerivationKind::Transcript);
    let mut recovered = String::new();
    for lsn in first..=last {
        let verified = actor.verified_event(LSN::new(lsn)).await.unwrap();
        assert_eq!(verified.kind, hm_schema::event::EventKind::UserMsg);
        assert_eq!(verified.envelope.authority, Authority::DerivedInference);
        let provenance = verified.envelope.model_provenance.as_ref().unwrap();
        assert_eq!(provenance.prompt_id, "media-transcript");
        assert_eq!(provenance.prompt_version, 1);
        assert!((provenance.temperature - 0.0).abs() < f32::EPSILON);
        let call_id = provenance.call_id.as_deref().unwrap();
        assert_eq!(call_id.len(), 32);
        assert_eq!(call_id, expected_call_id.as_slice());
        let EventPayload::UserMsg(message) = verified.envelope.payload else {
            panic!("derived event is not a user message");
        };
        recovered.push_str(&String::from_utf8(message.content).unwrap());
    }
    assert_eq!(recovered, expected_transcript());

    let mut expected_provenance = vec![format!("hm://7/lsn/{media_lsn}")];
    for lsn in first..=last {
        expected_provenance.push(format!("hm://7/lsn/{lsn}"));
    }
    assert_eq!(envelope.provenance, expected_provenance);

    drop(server);
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn derivation_is_recorded_in_the_media_catalog_and_is_idempotent() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(temporary.path())).await.unwrap();
    let (media_lsn, _) = seed(&actor, MEDIA_TYPE).await;
    let recorded = provider(0);
    let server = McpServer::new(actor.clone()).with_media_runtime(MediaRuntime {
        provider: recorded.clone(),
    });

    let first_call = server.remember_envelope(derive_input(media_lsn)).await;
    assert!(first_call.ok, "{first_call:?}");
    let (_, derived_last) = derived_range(&first_call);
    assert_eq!(recorded.transport().remaining(), 0);

    let digest = *blake3::hash(MEDIA_BYTES).as_bytes();
    assert_eq!(
        hm_proj::media::derivation_call_id(&digest, hm_proj::media::TRANSCRIPT_KIND_BYTE),
        hm_cortex::media::call_id(&subject(media_lsn), MediaDerivationKind::Transcript)
    );

    let catalog = actor.media_catalog(false, 16).await.unwrap();
    assert_eq!(catalog.len(), 1);
    assert_eq!(catalog[0].digest, digest.to_vec());
    assert_eq!(catalog[0].derived_lsn, derived_last);
    assert_ne!(catalog[0].derived_lsn, 0);
    assert_eq!(catalog[0].derived_prompt_id, "media-transcript");
    assert!(actor.media_catalog(true, 16).await.unwrap().is_empty());

    let before = actor.stats().await.unwrap().applied.last_lsn;
    let second_call = server.remember_envelope(derive_input(media_lsn)).await;
    assert!(second_call.ok, "{second_call:?}");
    assert_eq!(
        second_call.items[0]["already_derived"].as_bool(),
        Some(true)
    );
    assert_eq!(derived_range(&second_call), (derived_last, derived_last));
    assert_eq!(recorded.transport().remaining(), 0);
    assert_eq!(actor.stats().await.unwrap().applied.last_lsn, before);

    drop(server);
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn derived_text_is_recallable_but_never_observed() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(temporary.path())).await.unwrap();
    let (media_lsn, _) = seed(&actor, MEDIA_TYPE).await;
    let recorded = provider(0);
    let server = McpServer::new(actor.clone()).with_media_runtime(MediaRuntime {
        provider: recorded.clone(),
    });

    let derived = server.remember_envelope(derive_input(media_lsn)).await;
    assert!(derived.ok, "{derived:?}");
    let (first, last) = derived_range(&derived);

    let transcript = expected_transcript();
    let term = transcript.split_whitespace().nth(1).unwrap();
    let recalled = server.recall_envelope(lexical(term)).await;
    assert!(recalled.ok, "{recalled:?}");
    let hits = recalled
        .items
        .iter()
        .filter_map(|item| item["lsn"].as_u64())
        .collect::<Vec<_>>();
    assert!(hits.contains(&first), "{recalled:?}");
    for item in &recalled.items {
        if item["lsn"].as_u64() == Some(first) {
            assert_eq!(item["authority"].as_str(), Some("derived_inference"));
        }
    }

    let history = ledger_history(&actor).await;
    for lsn in first..=last {
        let refused = validate_observed_evidence(&[lsn], &history).unwrap_err();
        assert_eq!(refused.code, ErrorCode::CitationInvalid);
    }
    assert!(validate_observed_evidence(&[media_lsn], &history).is_ok());

    drop(server);
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn missing_media_unsupported_type_refusal_and_absent_runtime_append_nothing() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(temporary.path())).await.unwrap();
    let (media_lsn, retained_lsn) = seed(&actor, MEDIA_TYPE).await;
    let recorded = provider(0);
    let server = McpServer::new(actor.clone()).with_media_runtime(MediaRuntime {
        provider: recorded.clone(),
    });

    let cases: [(u64, ErrorCode); 3] = [
        (retained_lsn, ErrorCode::InvalidArgument),
        (media_lsn + 64, ErrorCode::InvalidArgument),
        (0, ErrorCode::InvalidArgument),
    ];
    for (lsn, code) in cases {
        let before = actor.stats().await.unwrap().applied.last_lsn;
        let envelope = server.remember_envelope(derive_input(lsn)).await;
        assert!(!envelope.ok, "{lsn}: {envelope:?}");
        assert_eq!(envelope.items[0]["error"].as_str(), Some(code.as_str()));
        assert_eq!(actor.stats().await.unwrap().applied.last_lsn, before);
    }

    let mut both = derive_input(media_lsn);
    both.content = "a transcript supplied by hand".to_owned();
    let refused = server.remember_envelope(both).await;
    assert!(!refused.ok);
    assert_eq!(
        refused.items[0]["error"].as_str(),
        Some(ErrorCode::InvalidArgument.as_str())
    );

    let before = actor.stats().await.unwrap().applied.last_lsn;
    let absent = McpServer::new(actor.clone())
        .remember_envelope(derive_input(media_lsn))
        .await;
    assert!(!absent.ok);
    assert_eq!(
        absent.items[0]["error"].as_str(),
        Some(ErrorCode::OperationUnavailable.as_str())
    );
    assert_eq!(actor.stats().await.unwrap().applied.last_lsn, before);
    assert_eq!(recorded.transport().remaining(), 1);

    let refusing = provider(1);
    let refusing_server = McpServer::new(actor.clone()).with_media_runtime(MediaRuntime {
        provider: refusing.clone(),
    });
    let before = actor.stats().await.unwrap().applied.last_lsn;
    let declined = refusing_server
        .remember_envelope(derive_input(media_lsn))
        .await;
    assert!(!declined.ok, "{declined:?}");
    assert_eq!(
        declined.items[0]["error"].as_str(),
        Some(ErrorCode::OperationUnavailable.as_str())
    );
    assert_eq!(refusing.transport().remaining(), 0);
    assert_eq!(actor.stats().await.unwrap().applied.last_lsn, before);
    assert!(actor.media_catalog(true, 16).await.unwrap().len() == 1);

    drop(server);
    drop(refusing_server);
    actor.shutdown().await.unwrap();

    let unsupported = tempfile::tempdir().unwrap();
    let other = ActorEngine::open(config(unsupported.path())).await.unwrap();
    let (pdf_lsn, _) = seed(&other, "application/pdf").await;
    let idle = provider(0);
    let pdf_server = McpServer::new(other.clone()).with_media_runtime(MediaRuntime {
        provider: idle.clone(),
    });
    let before = other.stats().await.unwrap().applied.last_lsn;
    let envelope = pdf_server.remember_envelope(derive_input(pdf_lsn)).await;
    assert!(!envelope.ok, "{envelope:?}");
    assert_eq!(
        envelope.items[0]["error"].as_str(),
        Some(ErrorCode::OperationUnavailable.as_str())
    );
    assert_eq!(idle.transport().remaining(), 1);
    assert_eq!(other.stats().await.unwrap().applied.last_lsn, before);
    drop(pdf_server);
    other.shutdown().await.unwrap();
}

#[tokio::test]
async fn recall_and_activate_never_reach_the_derivation_job() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(temporary.path())).await.unwrap();
    seed(&actor, MEDIA_TYPE).await;
    let recorded = provider(0);
    let server = McpServer::new(actor.clone()).with_media_runtime(MediaRuntime {
        provider: recorded.clone(),
    });

    for mode in [
        RecallMode::Lexical,
        RecallMode::Entity,
        RecallMode::Timeline,
    ] {
        let mut input = lexical("freight");
        input.mode = mode;
        let _ = server.recall_envelope(input).await;
        assert_eq!(recorded.transport().remaining(), 1);
    }

    let activated = server
        .activate_envelope(ActivateInput {
            conversation: CONVERSATION.to_owned(),
            query: "freight manifest".to_owned(),
            turn_text: String::new(),
            budget_tokens: 512,
        })
        .await;
    assert!(activated.ok, "{activated:?}");
    assert_eq!(recorded.transport().remaining(), 1);

    drop(server);
    actor.shutdown().await.unwrap();
}
