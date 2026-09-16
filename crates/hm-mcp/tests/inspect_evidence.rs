#![forbid(unsafe_code)]

use hm_core::{ActorId, ConversationId};
use hm_mcp::{
    AttestDisposition, AttestInput, InspectInput, McpServer, RememberInput, RememberKind,
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

async fn evidence(server: &McpServer, lsn: u64) -> Value {
    let envelope = server
        .inspect_envelope(InspectInput {
            uri: Some(format!("hm://7/evidence/{lsn}")),
            ..InspectInput::default()
        })
        .await;
    assert!(envelope.ok, "inspect failed: {:?}", envelope.items);
    let mut matched = envelope
        .items
        .into_iter()
        .filter(|item| item["surface"].as_str() == Some("evidence"))
        .collect::<Vec<_>>();
    assert_eq!(matched.len(), 1);
    matched.remove(0)
}

#[tokio::test]
async fn evidence_surface_reports_the_answers_that_used_a_record() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor);

    let first = remember(&server, "alpha", "The shipping ledger closed on Tuesday.").await;
    let second = remember(&server, "beta", "The audit reopened the shipping ledger.").await;

    let used_lsn = attest(&server, first, AttestDisposition::Used, "answer-one").await;
    let helpful_lsn = attest(&server, first, AttestDisposition::Helpful, "answer-two").await;
    let ignored_lsn = attest(&server, second, AttestDisposition::Ignored, "answer-three").await;
    assert!(used_lsn < helpful_lsn && helpful_lsn < ignored_lsn);

    let item = evidence(&server, first).await;
    assert_eq!(item["lsn"].as_u64(), Some(first));
    assert_eq!(item["counts_from"].as_str(), Some("ledger_scan"));
    assert_eq!(item["attestations"]["used"].as_u64(), Some(1));
    assert_eq!(item["attestations"]["ignored"].as_u64(), Some(0));
    assert_eq!(item["attestations"]["helpful"].as_u64(), Some(1));
    assert_eq!(item["attestations"]["harmful"].as_u64(), Some(0));
    assert_eq!(item["truncated"].as_bool(), Some(false));

    let answers = item["answers"].as_array().unwrap();
    assert_eq!(answers.len(), 2);
    assert_eq!(answers[0]["attestation_lsn"].as_u64(), Some(used_lsn));
    assert_eq!(answers[0]["disposition"].as_str(), Some("used"));
    assert_eq!(answers[1]["attestation_lsn"].as_u64(), Some(helpful_lsn));
    assert_eq!(answers[1]["disposition"].as_str(), Some("helpful"));
    let conversation = ConversationId::derive("hypermind.attest").to_string();
    assert_eq!(
        answers[0]["conversation"].as_str(),
        Some(conversation.as_str())
    );
    assert_eq!(
        answers[1]["conversation"].as_str(),
        Some(conversation.as_str())
    );

    let other = evidence(&server, second).await;
    assert_eq!(other["attestations"]["ignored"].as_u64(), Some(1));
    assert_eq!(other["attestations"]["used"].as_u64(), Some(0));
    let other_answers = other["answers"].as_array().unwrap();
    assert_eq!(other_answers.len(), 1);
    assert_eq!(
        other_answers[0]["attestation_lsn"].as_u64(),
        Some(ignored_lsn)
    );
}

#[tokio::test]
async fn evidence_path_edges_name_the_referencing_field() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor);

    let first = remember(&server, "alpha", "The shipping ledger closed on Tuesday.").await;
    let attestation = attest(&server, first, AttestDisposition::Used, "answer-one").await;

    let envelope = server
        .inspect_envelope(InspectInput {
            uri: Some(format!("hm://7/lsn/{attestation}")),
            ..InspectInput::default()
        })
        .await;
    assert!(envelope.ok, "inspect failed: {:?}", envelope.items);
    let path = &envelope.items[0]["evidence_path"];
    assert_eq!(path["label"].as_str(), Some("evidence_path"));
    assert_eq!(path["root_lsn"].as_u64(), Some(attestation));
    assert_eq!(path["visited"].as_u64(), Some(2));
    let edges = path["edges"].as_array().unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0]["from"].as_u64(), Some(attestation));
    assert_eq!(edges[0]["to"].as_u64(), Some(first));
    assert_eq!(edges[0]["relation"].as_str(), Some("target_lsn"));
    assert_eq!(
        envelope.provenance,
        [
            format!("hm://7/lsn/{attestation}"),
            format!("hm://7/lsn/{first}")
        ]
    );
}

#[tokio::test]
async fn evidence_surface_is_empty_for_an_unattested_record() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor);

    let lsn = remember(&server, "alpha", "Nothing has cited this record yet.").await;

    let envelope = server
        .inspect_envelope(InspectInput {
            uri: Some(format!("hm://7/evidence/{lsn}")),
            ..InspectInput::default()
        })
        .await;
    assert!(envelope.ok, "inspect failed: {:?}", envelope.items);
    let item = evidence(&server, lsn).await;
    assert_eq!(item["attestations"]["used"].as_u64(), Some(0));
    assert_eq!(item["attestations"]["ignored"].as_u64(), Some(0));
    assert_eq!(item["attestations"]["helpful"].as_u64(), Some(0));
    assert_eq!(item["attestations"]["harmful"].as_u64(), Some(0));
    assert!(item["answers"].as_array().unwrap().is_empty());
    assert_eq!(item["truncated"].as_bool(), Some(false));
    assert_eq!(envelope.provenance, [format!("hm://7/lsn/{lsn}")]);
}
