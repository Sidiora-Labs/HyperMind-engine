#![forbid(unsafe_code)]

use hm_core::ErrorCode;
use hm_cortex::authority::{CitedSource, authority_for_event, derive_authority};
use hm_cortex::ingest::{IngestResult, IngestSource, ingest};
use hm_schema::event::{CURRENT_SCHEMA_VERSION, EventKind};
use hm_schema::events::{
    Authority, EventEnvelope, EventPayload, ProviderFrame, Reasoning, Retention, Sensitivity,
    ToolResult, UserMsg,
};

fn envelope(payload: EventPayload) -> EventEnvelope {
    EventEnvelope {
        schema_version: CURRENT_SCHEMA_VERSION,
        payload,
        connection_id: None,
        client_seq: 0,
        client_event_index: 0,
        client_event_count: 0,
        origin_actor: 0,
        run_id: None,
        model_provenance: None,
        authority: Authority::DerivedInference,
        retention: Retention::Durable,
        sensitivity: Sensitivity::Secret,
        event_time_ns: 0,
    }
}

#[test]
fn ingest_assigns_authority_from_trusted_source_and_caps_automatic_sensitivity() {
    let cases = [
        (
            EventKind::UserMsg,
            IngestSource::User,
            EventPayload::UserMsg(Box::new(UserMsg {
                content: b"user assertion".to_vec(),
            })),
            Authority::UserAsserted,
        ),
        (
            EventKind::Reasoning,
            IngestSource::Assistant,
            EventPayload::Reasoning(Box::new(Reasoning {
                content: b"model text".to_vec(),
            })),
            Authority::AssistantGenerated,
        ),
        (
            EventKind::ToolResult,
            IngestSource::ToolRuntime,
            EventPayload::ToolResult(Box::new(ToolResult {
                call_id: b"call-1".to_vec(),
                tool_call_lsn: 1,
                result: b"observed bytes".to_vec(),
                ..ToolResult::default()
            })),
            Authority::ToolObserved,
        ),
        (
            EventKind::ProviderFrame,
            IngestSource::ExternalFeed,
            EventPayload::ProviderFrame(Box::new(ProviderFrame {
                provider: "feed".to_owned(),
                api_content: b"external bytes".to_vec(),
            })),
            Authority::ExternalObserved,
        ),
    ];
    for (kind, source, payload, expected) in cases {
        let IngestResult::Append(accepted) = ingest(kind, source, envelope(payload), true).unwrap()
        else {
            panic!("durable event must append");
        };
        assert_eq!(accepted.authority, expected);
        assert_eq!(accepted.sensitivity, Sensitivity::Personal);
        assert_eq!(authority_for_event(kind), expected);
    }
}

#[test]
fn assistant_content_cannot_be_promoted_and_memory_sources_are_refused() {
    let tool_payload = EventPayload::ToolResult(Box::new(ToolResult {
        call_id: b"call-1".to_vec(),
        tool_call_lsn: 1,
        result: b"model supplied".to_vec(),
        ..ToolResult::default()
    }));
    assert_eq!(
        ingest(
            EventKind::ToolResult,
            IngestSource::Assistant,
            envelope(tool_payload.clone()),
            true,
        )
        .unwrap_err()
        .code,
        ErrorCode::InvalidKind
    );
    assert_eq!(
        ingest(
            EventKind::ToolResult,
            IngestSource::Memory("memory.summary".to_owned()),
            envelope(tool_payload),
            true,
        )
        .unwrap_err()
        .code,
        ErrorCode::ForbiddenKind
    );
}

#[test]
fn only_an_exact_cited_byte_range_preserves_authority() {
    let bytes = b"prefix EXACT QUOTE suffix";
    let source = CitedSource {
        authority: Authority::ToolObserved,
        bytes,
        cited_range: 7..18,
    };
    assert_eq!(
        derive_authority(&source, b"EXACT QUOTE"),
        Authority::ToolObserved
    );
    assert_eq!(
        derive_authority(&source, b"exact quote"),
        Authority::DerivedInference
    );
    let assistant = CitedSource {
        authority: Authority::AssistantGenerated,
        bytes: b"model output",
        cited_range: 0..12,
    };
    assert_eq!(
        derive_authority(&assistant, b"model output"),
        Authority::AssistantGenerated
    );
}

#[test]
fn do_not_store_returns_only_a_receipt_and_explicit_secret_is_preserved() {
    let mut value = envelope(EventPayload::UserMsg(Box::new(UserMsg {
        content: b"ephemeral secret".to_vec(),
    })));
    value.retention = Retention::DoNotStore;
    let IngestResult::DoNotStore(receipt) =
        ingest(EventKind::UserMsg, IngestSource::User, value, true).unwrap()
    else {
        panic!("do_not_store must not append");
    };
    assert_eq!(receipt.kind, EventKind::UserMsg);
    assert_eq!(receipt.authority, Authority::UserAsserted);
    assert_ne!(receipt.content_hash, [0; 32]);

    let IngestResult::Append(explicit) = ingest(
        EventKind::UserMsg,
        IngestSource::User,
        envelope(EventPayload::UserMsg(Box::new(UserMsg {
            content: b"explicit secret".to_vec(),
        }))),
        false,
    )
    .unwrap() else {
        panic!("durable event must append");
    };
    assert_eq!(explicit.sensitivity, Sensitivity::Secret);
}
