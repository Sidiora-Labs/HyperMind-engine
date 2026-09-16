use hm_core::{ActorId, ErrorCode};
use hm_mcp::{
    AttestDisposition, AttestInput, InspectInput, McpServer, RecallFilters, RecallInput,
    RecallMode, RememberInput, RememberKind,
};
use hm_proj::attestations::{
    PREFERENCE_MAXIMUM_Q16, PREFERENCE_MINIMUM_Q16, PREFERENCE_NEUTRAL_Q16,
};
use hm_serve::actor::{ActorConfig, ActorEngine};
use serde_json::Value;

const EVIDENCE: [&str; 3] = ["heliotrope alpha", "heliotrope beta", "heliotrope gamma"];

fn actor_config(root: &std::path::Path, actor: u16) -> ActorConfig {
    ActorConfig {
        actor_directory: root.join(actor.to_string()),
        actor: ActorId::new(actor),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

fn evidence_document(content: &str) -> RememberInput {
    RememberInput {
        conversation: "evidence".to_owned(),
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
        source_sync: None,
    }
}

fn lexical_recall() -> RecallInput {
    RecallInput {
        mode: RecallMode::Lexical,
        query: "heliotrope".to_owned(),
        conversation: String::new(),
        limit: 10,
        since_lsn: 0,
        filters: RecallFilters::default(),
    }
}

async fn seeded(root: &std::path::Path, actor_id: u16) -> (ActorEngine, McpServer) {
    let actor = ActorEngine::open(actor_config(root, actor_id))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());
    for content in EVIDENCE {
        let remembered = server.remember_envelope(evidence_document(content)).await;
        assert!(remembered.ok, "{remembered:?}");
        assert_eq!(remembered.provenance.len(), 1);
    }
    (actor, server)
}

async fn attest(server: &McpServer, uri: &str, disposition: AttestDisposition, key: &str) {
    let attested = server
        .attest_envelope(AttestInput {
            provenance: vec![uri.to_owned()],
            disposition,
            idempotency_key: key.to_owned(),
        })
        .await;
    assert!(attested.ok, "{attested:?}");
}

async fn preferences(server: &McpServer, actor_id: u16) -> hm_mcp::Envelope {
    server
        .inspect_envelope(InspectInput {
            uri: Some(format!("hm://{actor_id}/preferences")),
            ..InspectInput::default()
        })
        .await
}

fn listed(envelope: &hm_mcp::Envelope) -> Vec<Value> {
    envelope.items[0]["preferences"].as_array().unwrap().clone()
}

#[tokio::test]
async fn preferences_surface_lists_bounded_weights_with_provenance() {
    let temporary = tempfile::tempdir().unwrap();
    let (actor, server) = seeded(temporary.path(), 7).await;

    let recalled = server.recall_envelope(lexical_recall()).await;
    assert!(recalled.ok, "{recalled:?}");
    assert_eq!(recalled.items.len(), 3);
    let dispositions = [
        AttestDisposition::Helpful,
        AttestDisposition::Harmful,
        AttestDisposition::Used,
    ];
    for (index, item) in recalled.items.iter().enumerate() {
        attest(
            &server,
            item["uri"].as_str().unwrap(),
            dispositions[index],
            &format!("attest-{index}"),
        )
        .await;
    }

    let envelope = preferences(&server, 7).await;
    assert!(envelope.ok, "{envelope:?}");
    let weights = listed(&envelope);
    assert_eq!(weights.len(), 3);
    let mut previous = u64::MAX;
    for weight in &weights {
        let target_lsn = weight["target_lsn"].as_u64().unwrap();
        assert!(target_lsn < previous, "{weights:?}");
        previous = target_lsn;
        assert_eq!(weight["observations"].as_u64(), Some(1));
        assert!(weight["last_attestation_lsn"].as_u64().unwrap() > target_lsn);
        let weight_q16 = weight["weight_q16"].as_u64().unwrap();
        assert!(
            weight_q16 >= u64::from(PREFERENCE_MINIMUM_Q16),
            "{weight:?}"
        );
        assert!(
            weight_q16 <= u64::from(PREFERENCE_MAXIMUM_Q16),
            "{weight:?}"
        );
        assert!(
            envelope
                .provenance
                .contains(&format!("hm://7/lsn/{target_lsn}")),
            "{envelope:?}"
        );
    }

    let weight_of = |lsn: u64| {
        weights
            .iter()
            .find(|weight| weight["target_lsn"].as_u64() == Some(lsn))
            .unwrap()["weight_q16"]
            .as_u64()
            .unwrap()
    };
    let helpful = recalled.items[0]["lsn"].as_u64().unwrap();
    let harmful = recalled.items[1]["lsn"].as_u64().unwrap();
    assert!(weight_of(helpful) > u64::from(PREFERENCE_NEUTRAL_Q16));
    assert!(weight_of(harmful) < u64::from(PREFERENCE_NEUTRAL_Q16));

    drop(server);
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn preferences_surface_is_empty_without_feedback() {
    let temporary = tempfile::tempdir().unwrap();
    let (actor, server) = seeded(temporary.path(), 7).await;

    let envelope = preferences(&server, 7).await;
    assert!(envelope.ok, "{envelope:?}");
    assert!(listed(&envelope).is_empty(), "{envelope:?}");
    assert!(envelope.provenance.is_empty(), "{envelope:?}");

    drop(server);
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn foreign_actor_preferences_uri_is_rejected() {
    let temporary = tempfile::tempdir().unwrap();
    let (actor, server) = seeded(temporary.path(), 7).await;

    let envelope = preferences(&server, 8).await;
    assert!(!envelope.ok, "{envelope:?}");
    assert_eq!(
        envelope.items[0]["error"].as_str(),
        Some(ErrorCode::InvalidArgument.as_str())
    );

    drop(server);
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn feedback_in_one_actor_does_not_reach_another() {
    let temporary = tempfile::tempdir().unwrap();
    let (seven, seven_server) = seeded(temporary.path(), 7).await;
    let (eight, eight_server) = seeded(temporary.path(), 8).await;

    let before = eight_server.recall_envelope(lexical_recall()).await;
    assert!(before.ok, "{before:?}");
    assert_eq!(before.items.len(), 3);

    let recalled = seven_server.recall_envelope(lexical_recall()).await;
    assert_eq!(recalled.items.len(), 3);
    let target = recalled.items[0]["uri"].as_str().unwrap().to_owned();
    for round in 1..=8 {
        attest(
            &seven_server,
            &target,
            AttestDisposition::Helpful,
            &format!("helpful-{round}"),
        )
        .await;
    }
    let learned = preferences(&seven_server, 7).await;
    assert_eq!(listed(&learned).len(), 1);
    assert!(
        listed(&learned)[0]["weight_q16"].as_u64().unwrap() > u64::from(PREFERENCE_NEUTRAL_Q16)
    );

    let isolated = preferences(&eight_server, 8).await;
    assert!(isolated.ok, "{isolated:?}");
    assert!(listed(&isolated).is_empty(), "{isolated:?}");

    let after = eight_server.recall_envelope(lexical_recall()).await;
    assert!(after.ok, "{after:?}");
    assert_eq!(after.items, before.items);
    for item in &after.items {
        assert_eq!(
            item["preference_q16"].as_u64().unwrap(),
            u64::from(PREFERENCE_NEUTRAL_Q16)
        );
        assert!(!item["uri"].as_str().unwrap().contains("pref="));
    }

    drop(seven_server);
    drop(eight_server);
    seven.shutdown().await.unwrap();
    eight.shutdown().await.unwrap();
}

#[tokio::test]
async fn attention_and_calibration_surfaces_are_unchanged() {
    let temporary = tempfile::tempdir().unwrap();
    let (actor, server) = seeded(temporary.path(), 7).await;

    let recalled = server.recall_envelope(lexical_recall()).await;
    attest(
        &server,
        recalled.items[0]["uri"].as_str().unwrap(),
        AttestDisposition::Helpful,
        "attest-once",
    )
    .await;

    let attention = server
        .inspect_envelope(InspectInput {
            uri: Some("hm://7/attention".to_owned()),
            ..InspectInput::default()
        })
        .await;
    assert!(attention.ok, "{attention:?}");
    assert!(attention.items[0]["attention"].is_array(), "{attention:?}");
    assert!(attention.items[0].get("preferences").is_none());

    let calibration = server
        .inspect_envelope(InspectInput {
            uri: Some("hm://7/calibration".to_owned()),
            ..InspectInput::default()
        })
        .await;
    assert!(calibration.ok, "{calibration:?}");
    assert!(
        !calibration.items[0]["calibration"]
            .as_array()
            .unwrap()
            .is_empty(),
        "{calibration:?}"
    );
    assert!(calibration.items[0].get("preferences").is_none());

    let target_lsn = recalled.items[0]["lsn"].as_u64().unwrap();
    let chain = server
        .inspect_envelope(InspectInput {
            uri: Some(format!("hm://7/lsn/{target_lsn}")),
            ..InspectInput::default()
        })
        .await;
    assert!(chain.ok, "{chain:?}");
    assert_eq!(
        chain.items[0]["evidence_path"]["root_lsn"].as_u64(),
        Some(target_lsn)
    );
    assert!(
        chain
            .provenance
            .contains(&format!("hm://7/lsn/{target_lsn}")),
        "{chain:?}"
    );

    drop(server);
    actor.shutdown().await.unwrap();
}
