#![forbid(unsafe_code)]

use hm_core::ActorId;
use hm_mcp::{
    AttestDisposition, AttestInput, Envelope, InspectInput, McpServer, RememberInput, RememberKind,
};
use hm_serve::actor::{ActorConfig, ActorEngine};
use serde_json::Value;

fn actor_config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("7"),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

async fn remember(server: &McpServer, conversation: &str, content: &str) -> u64 {
    let stored = server
        .remember_envelope(RememberInput {
            conversation: conversation.to_owned(),
            content: content.to_owned(),
            kind: RememberKind::Document,
            chunk_bytes: None,
            anchor: None,
            retention: None,
            sensitivity: None,
            vocabulary: None,
            source: None,
            derive: None,
            source_delivery: None,
            source_settlement: None,
            document: None,
        })
        .await;
    assert!(stored.ok, "remember failed: {:?}", stored.items);
    assert_eq!(stored.items[0]["first_lsn"], stored.items[0]["last_lsn"]);
    stored.items[0]["first_lsn"].as_u64().unwrap()
}

async fn attest(
    server: &McpServer,
    target: u64,
    disposition: AttestDisposition,
    idempotency_key: &str,
) -> u64 {
    let attested = server
        .attest_envelope(AttestInput {
            provenance: vec![format!("hm://7/lsn/{target}")],
            disposition,
            idempotency_key: idempotency_key.to_owned(),
        })
        .await;
    assert!(attested.ok, "attest failed: {:?}", attested.items);
    assert_eq!(
        attested.items[0]["first_lsn"],
        attested.items[0]["last_lsn"]
    );
    attested.items[0]["first_lsn"].as_u64().unwrap()
}

async fn preview_envelope(server: &McpServer, lsn: u64) -> Envelope {
    server
        .inspect_envelope(InspectInput {
            uri: Some(format!("hm://7/removal/{lsn}")),
            ..InspectInput::default()
        })
        .await
}

async fn preview(server: &McpServer, lsn: u64) -> Value {
    let envelope = preview_envelope(server, lsn).await;
    assert!(envelope.ok, "inspect failed: {:?}", envelope.items);
    let mut matched = envelope
        .items
        .into_iter()
        .filter(|item| item["diagnostic"].as_str() == Some("removal_preview"))
        .collect::<Vec<_>>();
    assert_eq!(matched.len(), 1);
    matched.remove(0)
}

async fn status(server: &McpServer) -> Value {
    let envelope = server.inspect_envelope(InspectInput::default()).await;
    assert!(envelope.ok, "inspect failed: {:?}", envelope.items);
    envelope.items[0].clone()
}

fn numbers(value: &Value) -> Vec<u64> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry.as_u64().unwrap())
        .collect()
}

async fn seeded(path: &std::path::Path) -> (McpServer, u64, u64, u64, u64) {
    let actor = ActorEngine::open(actor_config(path)).await.unwrap();
    let server = McpServer::new(actor);
    let first = remember(&server, "alpha", "The shipping ledger closed on Tuesday.").await;
    let second = remember(&server, "beta", "The audit reopened the shipping ledger.").await;
    let used = attest(&server, first, AttestDisposition::Used, "k1").await;
    let helpful = attest(&server, first, AttestDisposition::Helpful, "k2").await;
    let ignored = attest(&server, second, AttestDisposition::Ignored, "k3").await;
    assert!(used < helpful && helpful < ignored);
    (server, first, second, used, helpful)
}

#[tokio::test]
async fn removal_preview_reports_orphaned_records() {
    let temporary = tempfile::tempdir().unwrap();
    let (server, first, _second, used, helpful) = seeded(temporary.path()).await;

    let item = preview(&server, first).await;
    assert_eq!(item["target_lsn"].as_u64(), Some(first));
    assert_eq!(item["performs_retraction"].as_bool(), Some(false));
    assert_eq!(item["island_count"].as_u64(), Some(2));
    assert_eq!(numbers(&item["orphaned_lsns"]), [used, helpful]);
    assert_eq!(numbers(&item["direct_dependents"]), [used, helpful]);
    assert_eq!(item["attestation_counts"]["used"].as_u64(), Some(1));
    assert_eq!(item["attestation_counts"]["helpful"].as_u64(), Some(1));
    assert_eq!(item["attestation_counts"]["ignored"].as_u64(), Some(0));
    assert_eq!(item["attestation_counts"]["harmful"].as_u64(), Some(0));
    assert_eq!(item["truncated"].as_bool(), Some(false));
    assert_eq!(item["component_records"].as_u64(), Some(3));
    let scanned = item["scanned_frames"].as_u64().unwrap();
    assert!(scanned >= 5, "scanned only {scanned} frames");

    let again = preview(&server, first).await;
    assert_eq!(
        serde_json::to_string(&item).unwrap(),
        serde_json::to_string(&again).unwrap()
    );
}

#[tokio::test]
async fn removal_preview_is_empty_for_a_small_component() {
    let temporary = tempfile::tempdir().unwrap();
    let (server, _first, second, _used, _helpful) = seeded(temporary.path()).await;

    let item = preview(&server, second).await;
    assert_eq!(item["target_lsn"].as_u64(), Some(second));
    assert!(item["orphaned_lsns"].as_array().unwrap().is_empty());
    assert_eq!(item["island_count"].as_u64(), Some(0));
    assert_eq!(item["component_records"].as_u64(), Some(2));
    assert_eq!(item["attestation_counts"]["ignored"].as_u64(), Some(1));
}

#[tokio::test]
async fn removal_preview_writes_nothing() {
    let temporary = tempfile::tempdir().unwrap();
    let (server, first, second, _used, _helpful) = seeded(temporary.path()).await;

    let before = status(&server).await;
    let _ = preview(&server, first).await;
    let _ = preview(&server, second).await;
    let after = status(&server).await;

    assert_eq!(before["applied_lsn"], after["applied_lsn"]);
    assert_eq!(before["log_events"], after["log_events"]);
    assert_eq!(before["applied_digest"], after["applied_digest"]);
}

#[tokio::test]
async fn removal_preview_rejects_an_unknown_record() {
    let temporary = tempfile::tempdir().unwrap();
    let (server, _first, _second, _used, helpful) = seeded(temporary.path()).await;

    let envelope = preview_envelope(&server, helpful + 1_000).await;
    assert!(!envelope.ok, "an unknown record must be refused");
    assert_eq!(
        envelope.items[0]["error"].as_str(),
        Some("kInvalidArgument")
    );
    assert_eq!(envelope.items[0]["lsn"].as_u64(), Some(helpful + 1_000));
}
