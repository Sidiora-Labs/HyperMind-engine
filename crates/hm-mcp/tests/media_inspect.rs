#![forbid(unsafe_code)]

use hm_core::{ActorId, ConversationId, ErrorCode};
use hm_ledger::frame::EventKind;
use hm_mcp::{InspectInput, McpServer};
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    Authority, EventEnvelope, EventPayload, MediaRef, ProviderFrame, Retention, Sensitivity,
};
use hm_serve::actor::{ActorConfig, ActorEngine, IncomingEvent};
use serde_json::Value;

const SOURCE_URI: &str = "https://example.test/notes/quarter.txt";
const SOURCE_MEDIA_TYPE: &str = "text/plain";
const SOURCE_BYTES: &[u8] = b"The freight manifest was signed on the eleventh.";

fn config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("7"),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
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

async fn held_media(actor: &ActorEngine) -> (u64, u64) {
    let digest = blake3::hash(SOURCE_BYTES);
    let outcome = actor
        .append(vec![
            observed(
                EventKind::MediaRef,
                ConversationId::derive("quarter-notes"),
                EventPayload::MediaRef(Box::new(MediaRef {
                    uri: SOURCE_URI.to_owned(),
                    media_type: SOURCE_MEDIA_TYPE.to_owned(),
                    digest: digest.as_bytes().to_vec(),
                })),
                0,
            ),
            observed(
                EventKind::ProviderFrame,
                ConversationId::derive(&format!("hm-media-v1/{}", hex(digest.as_bytes()))),
                EventPayload::ProviderFrame(Box::new(ProviderFrame {
                    provider: "hm-web-source@1".to_owned(),
                    api_content: SOURCE_BYTES.to_vec(),
                })),
                1,
            ),
        ])
        .await
        .unwrap();
    (outcome.first_lsn.get(), outcome.last_lsn.get())
}

async fn inspect(server: &McpServer, uri: &str) -> hm_mcp::Envelope {
    let envelope = server
        .inspect_envelope(InspectInput {
            uri: Some(uri.to_owned()),
            ..InspectInput::default()
        })
        .await;
    assert!(envelope.ok, "inspect failed: {:?}", envelope.items);
    envelope
}

fn rows(envelope: &hm_mcp::Envelope) -> Vec<Value> {
    envelope.items[0]["media"].as_array().unwrap().clone()
}

#[tokio::test]
async fn media_inspect_lists_held_and_pending_sources() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(temporary.path())).await.unwrap();
    let (media_ref_lsn, retained_lsn) = held_media(&actor).await;
    let server = McpServer::new(actor);

    let listed = inspect(&server, "hm://7/media").await;
    let listed_rows = rows(&listed);
    assert_eq!(listed_rows.len(), 1);
    let row = &listed_rows[0];
    let digest_hex = hex(blake3::hash(SOURCE_BYTES).as_bytes());
    assert_eq!(digest_hex.len(), 64);
    assert_eq!(row["digest"].as_str(), Some(digest_hex.as_str()));
    assert_eq!(row["uri"].as_str(), Some(SOURCE_URI));
    assert_eq!(row["media_type"].as_str(), Some(SOURCE_MEDIA_TYPE));
    assert_eq!(row["media_ref_lsn"].as_u64(), Some(media_ref_lsn));
    assert_eq!(row["retained_lsn"].as_u64(), Some(retained_lsn));
    assert_eq!(row["derived_lsn"].as_u64(), Some(0));
    assert_eq!(row["derived_prompt_id"].as_str(), Some(""));

    let pending = inspect(&server, "hm://7/media/pending").await;
    assert_eq!(rows(&pending), listed_rows);
}

#[tokio::test]
async fn media_inspect_provenance_lists_every_referenced_lsn() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(temporary.path())).await.unwrap();
    let (media_ref_lsn, retained_lsn) = held_media(&actor).await;
    let server = McpServer::new(actor);

    let envelope = inspect(&server, "hm://7/media").await;
    assert_eq!(
        envelope.provenance,
        [
            format!("hm://7/lsn/{media_ref_lsn}"),
            format!("hm://7/lsn/{retained_lsn}")
        ]
    );
    assert!(
        !envelope
            .provenance
            .iter()
            .any(|entry| entry == "hm://7/lsn/0")
    );
}

#[tokio::test]
async fn existing_inspect_paths_are_unchanged() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(temporary.path())).await.unwrap();
    let (media_ref_lsn, _) = held_media(&actor).await;
    let server = McpServer::new(actor);

    let status = server.inspect_envelope(InspectInput::default()).await;
    assert!(status.ok, "inspect failed: {:?}", status.items);
    assert_eq!(status.items.len(), 1);
    assert_eq!(status.items[0]["actor"].as_u64(), Some(7));
    assert!(status.items[0]["projections"].as_array().is_some());
    assert!(status.items[0]["media"].is_null());

    let walked = inspect(&server, &format!("hm://7/lsn/{media_ref_lsn}")).await;
    assert_eq!(
        walked.items[0]["evidence_path"]["root_lsn"].as_u64(),
        Some(media_ref_lsn)
    );
    assert_eq!(
        walked.items[0]["evidence_path"]["visited"].as_u64(),
        Some(1)
    );
    assert_eq!(walked.items[1]["lsn"].as_u64(), Some(media_ref_lsn));
    assert_eq!(walked.items[1]["kind"].as_str(), Some("mediaref"));
}

#[tokio::test]
async fn media_catalog_limit_is_bounded() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(temporary.path())).await.unwrap();
    held_media(&actor).await;

    let refused = actor.media_catalog(false, 0).await.unwrap_err();
    assert_eq!(refused.code, ErrorCode::InvalidArgument);

    let capped = actor.media_catalog(false, 4096).await.unwrap();
    assert_eq!(capped.len(), 1);
    assert!(capped.len() <= 256);
    assert_eq!(capped[0].uri, SOURCE_URI);

    let pending = actor.media_catalog(true, 4096).await.unwrap();
    assert_eq!(pending, capped);
}
