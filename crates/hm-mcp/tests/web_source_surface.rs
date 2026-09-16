#![forbid(unsafe_code)]

use hm_core::{ActorId, ConversationId, ErrorCode, LSN};
use hm_ledger::frame::EventKind;
use hm_mcp::tools::websource::{
    CrawlPolicy, FetchResponse, RecordedFetchTransport, WEB_SOURCE_PROVIDER_LABEL,
    WebSourceRuntime, media_conversation,
};
use hm_mcp::{
    McpServer, RecallFilters, RecallInput, RecallMode, RememberInput, RememberKind, RememberSource,
};
use hm_schema::event::{Boundary, verify_event};
use hm_schema::events::{Authority, EventPayload};
use hm_serve::actor::{ActorConfig, ActorEngine};
use std::path::Path;
use std::sync::Arc;

const PAGE: &[u8] =
    b"<html><head><title>Release</title></head><body><p>The mitigation ran in frankfurt.</p></body></html>";
const AUDIO: &[u8] = &[0xff, 0xfb, 0x90, 0x44, 0x00, 0x01, 0x02, 0x03];

fn config(path: &Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("7"),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

fn policy() -> CrawlPolicy {
    CrawlPolicy {
        allowed_hosts: vec!["sources.example".to_owned()],
        maximum_bytes: 65536,
        maximum_redirects: 3,
        minimum_interval_ms: 0,
        request_timeout_ms: 1000,
        allow_cross_host_redirect: false,
    }
}

fn response(content_type: &str, body: &[u8]) -> FetchResponse {
    FetchResponse {
        status: 200,
        content_type: Some(content_type.to_owned()),
        content_length: None,
        location: None,
        body: body.to_vec(),
    }
}

fn runtime(url: &str, response: FetchResponse) -> (WebSourceRuntime, Arc<RecordedFetchTransport>) {
    let transport = Arc::new(RecordedFetchTransport::new(vec![(
        url.to_owned(),
        response,
    )]));
    (
        WebSourceRuntime::new(policy(), transport.clone()),
        transport,
    )
}

fn source_input(conversation: &str, url: &str) -> RememberInput {
    RememberInput {
        conversation: conversation.to_owned(),
        content: String::new(),
        kind: RememberKind::Document,
        chunk_bytes: None,
        anchor: None,
        retention: None,
        sensitivity: None,
        vocabulary: None,
        source: Some(RememberSource {
            url: url.to_owned(),
        }),
        derive: None,
        source_delivery: None,
        source_settlement: None,
        document: None,
    }
}

fn payload_of(frame: &hm_ledger::frame::Frame) -> (EventPayload, Authority) {
    let kind = hm_schema::event::EventKind::try_from(frame.header.kind as u8).unwrap();
    let verified = verify_event(&frame.sealed_payload, kind, Boundary::Disk).unwrap();
    (verified.envelope.payload, verified.envelope.authority)
}

#[tokio::test]
async fn textual_source_is_retained_recorded_and_indexed() {
    let directory = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    let url = "https://sources.example/release";
    let (web_source, transport) = runtime(url, response("text/html; charset=utf-8", PAGE));
    let server = McpServer::new(actor.clone()).with_web_source_runtime(web_source);
    let envelope = server
        .remember_envelope(source_input("release-review", url))
        .await;
    assert!(envelope.ok, "{envelope:?}");
    assert_eq!(transport.remaining(), 0);
    let digest = *blake3::hash(PAGE).as_bytes();
    let item = &envelope.items[0];
    assert_eq!(item["final_url"], url);
    assert_eq!(item["media_type"], "text/html");
    assert_eq!(item["digest"], blake3::hash(PAGE).to_hex().to_string());
    assert_eq!(item["bytes"], PAGE.len());
    assert_eq!(item["redirects"], serde_json::json!([]));
    let media_ref_lsn = item["media_ref_lsn"].as_u64().unwrap();
    let retained_lsn = item["retained_lsn"].as_u64().unwrap();
    let text_first = item["text_first_lsn"].as_u64().unwrap();
    let text_last = item["text_last_lsn"].as_u64().unwrap();
    assert_eq!(media_ref_lsn, 1);
    assert_eq!(retained_lsn, 2);
    assert_eq!(text_first, 3);
    assert_eq!(text_last, envelope.items[0]["last_lsn"].as_u64().unwrap());
    assert!(envelope.gaps.is_empty(), "{envelope:?}");
    assert_eq!(
        envelope.provenance,
        (media_ref_lsn..=text_last)
            .map(|lsn| format!("hm://7/lsn/{lsn}"))
            .collect::<Vec<_>>()
    );

    let frames = actor.frames_since(LSN::new(0), None, 32).await.unwrap();
    assert_eq!(frames.len() as u64, text_last);
    let media = &frames[0];
    assert_eq!(media.header.kind, EventKind::MediaRef);
    assert_eq!(
        media.header.conversation,
        ConversationId::derive("release-review")
    );
    let (payload, authority) = payload_of(media);
    assert_eq!(authority, Authority::ExternalObserved);
    let EventPayload::MediaRef(reference) = payload else {
        panic!("expected a media reference");
    };
    assert_eq!(reference.uri, url);
    assert_eq!(reference.media_type, "text/html");
    assert_eq!(reference.digest, digest.to_vec());

    let retained = &frames[1];
    assert_eq!(retained.header.kind, EventKind::ProviderFrame);
    assert_eq!(retained.header.conversation, media_conversation(&digest));
    let (payload, authority) = payload_of(retained);
    assert_eq!(authority, Authority::ExternalObserved);
    let EventPayload::ProviderFrame(frame) = payload else {
        panic!("expected a retained provider frame");
    };
    assert_eq!(frame.provider, WEB_SOURCE_PROVIDER_LABEL);
    assert_eq!(frame.api_content, PAGE.to_vec());

    for frame in &frames[2..] {
        assert_eq!(frame.header.kind, EventKind::UserMsg);
        assert_eq!(
            frame.header.conversation,
            ConversationId::derive("release-review")
        );
        let (payload, authority) = payload_of(frame);
        assert_eq!(authority, Authority::ExternalObserved);
        let EventPayload::UserMsg(message) = payload else {
            panic!("expected extracted text");
        };
        assert!(!message.content.is_empty());
    }

    let recalled = server
        .recall_envelope(RecallInput {
            mode: RecallMode::Lexical,
            query: "frankfurt".to_owned(),
            conversation: String::new(),
            limit: 8,
            since_lsn: 0,
            filters: RecallFilters::default(),
        })
        .await;
    assert!(recalled.ok, "{recalled:?}");
    let hits: Vec<u64> = recalled
        .items
        .iter()
        .filter_map(|item| item["lsn"].as_u64())
        .collect();
    assert!(
        hits.iter()
            .any(|lsn| (text_first..=text_last).contains(lsn)),
        "{recalled:?}"
    );
    drop(server);
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn non_textual_source_retains_the_original_and_reports_a_pending_gap() {
    let directory = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    let url = "https://sources.example/briefing.mp3";
    let (web_source, transport) = runtime(url, response("audio/mpeg", AUDIO));
    let server = McpServer::new(actor.clone()).with_web_source_runtime(web_source);
    let envelope = server
        .remember_envelope(source_input("briefings", url))
        .await;
    assert!(envelope.ok, "{envelope:?}");
    assert_eq!(transport.remaining(), 0);
    assert_eq!(envelope.items[0]["media_type"], "audio/mpeg");
    assert!(envelope.items[0].get("text_first_lsn").is_none());
    assert_eq!(
        envelope.gaps,
        vec![serde_json::json!({
            "kind": "media_derivation_pending",
            "media_lsn": envelope.items[0]["media_ref_lsn"].as_u64().unwrap(),
            "media_type": "audio/mpeg",
        })]
    );
    let frames = actor.frames_since(LSN::new(0), None, 32).await.unwrap();
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].header.kind, EventKind::MediaRef);
    assert_eq!(frames[1].header.kind, EventKind::ProviderFrame);
    let (payload, _) = payload_of(&frames[1]);
    let EventPayload::ProviderFrame(frame) = payload else {
        panic!("expected a retained provider frame");
    };
    assert_eq!(frame.api_content, AUDIO.to_vec());
    drop(server);
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn policy_refusal_and_missing_runtime_are_reported_without_a_write() {
    let directory = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    let url = "https://sources.example/release";
    let (web_source, transport) = runtime(url, response("text/plain", b"stored"));
    let server = McpServer::new(actor.clone()).with_web_source_runtime(web_source);
    let before = actor.stats().await.unwrap().log_events;
    let refused = server
        .remember_envelope(source_input(
            "release-review",
            "https://elsewhere.example/page",
        ))
        .await;
    assert!(!refused.ok);
    assert_eq!(
        refused.items[0]["error"],
        ErrorCode::CapabilityDenied.as_str()
    );
    assert_eq!(refused.effect_state.as_deref(), Some("not_dispatched"));
    assert_eq!(transport.remaining(), 1);
    assert_eq!(actor.stats().await.unwrap().log_events, before);

    let unconfigured = McpServer::new(actor.clone())
        .remember_envelope(source_input("release-review", url))
        .await;
    assert!(!unconfigured.ok);
    assert_eq!(
        unconfigured.items[0]["error"],
        ErrorCode::OperationUnavailable.as_str()
    );
    assert_eq!(unconfigured.effect_state.as_deref(), Some("unknown"));
    assert_eq!(actor.stats().await.unwrap().log_events, before);
    drop(server);
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn content_and_source_are_mutually_exclusive() {
    let directory = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    let url = "https://sources.example/release";
    let (web_source, transport) = runtime(url, response("text/plain", b"stored"));
    let server = McpServer::new(actor.clone()).with_web_source_runtime(web_source);

    let mut both = source_input("release-review", url);
    both.content = "The mitigation ran in frankfurt.".to_owned();
    let rejected = server.remember_envelope(both).await;
    assert!(!rejected.ok);
    assert_eq!(
        rejected.items[0]["error"],
        ErrorCode::InvalidArgument.as_str()
    );

    let mut neither = source_input("release-review", url);
    neither.source = None;
    let empty = server.remember_envelope(neither).await;
    assert!(!empty.ok);
    assert_eq!(empty.items[0]["error"], ErrorCode::InvalidArgument.as_str());
    assert_eq!(transport.remaining(), 1);
    assert_eq!(actor.stats().await.unwrap().log_events, 0);

    let mut content_only = source_input("release-review", url);
    content_only.source = None;
    content_only.content = "The mitigation ran in frankfurt.".to_owned();
    let stored = server.remember_envelope(content_only).await;
    assert!(stored.ok, "{stored:?}");
    assert_eq!(stored.items[0]["first_lsn"], 1);
    assert_eq!(actor.stats().await.unwrap().log_events, 1);
    assert_eq!(transport.remaining(), 1);
    drop(server);
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn retained_media_stays_out_of_the_caller_conversation() {
    let directory = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    let url = "https://sources.example/release";
    let (web_source, _transport) = runtime(url, response("text/html; charset=utf-8", PAGE));
    let server = McpServer::new(actor.clone()).with_web_source_runtime(web_source);
    let envelope = server
        .remember_envelope(source_input("release-review", url))
        .await;
    assert!(envelope.ok, "{envelope:?}");
    let digest = *blake3::hash(PAGE).as_bytes();

    let caller = actor
        .frames_since(
            LSN::new(0),
            Some(ConversationId::derive("release-review")),
            32,
        )
        .await
        .unwrap();
    assert_eq!(caller[0].header.kind, EventKind::MediaRef);
    assert!(
        caller
            .iter()
            .all(|frame| frame.header.kind != EventKind::ProviderFrame)
    );
    assert!(
        caller
            .iter()
            .skip(1)
            .all(|frame| frame.header.kind == EventKind::UserMsg)
    );

    let media = actor
        .frames_since(LSN::new(0), Some(media_conversation(&digest)), 32)
        .await
        .unwrap();
    assert_eq!(media.len(), 1);
    assert_eq!(media[0].header.kind, EventKind::ProviderFrame);
    drop(server);
    actor.shutdown().await.unwrap();
}
