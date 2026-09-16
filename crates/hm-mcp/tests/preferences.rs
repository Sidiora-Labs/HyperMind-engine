use hm_core::{ActorId, ConversationId, LSN};
use hm_mcp::{
    AttestDisposition, AttestInput, McpServer, RecallFilters, RecallInput, RecallMode,
    RememberInput, RememberKind,
};
use hm_proj::attestations::{PREFERENCE_MAXIMUM_Q16, PREFERENCE_NEUTRAL_Q16};
use hm_serve::actor::{ActorConfig, ActorEngine, RecallRequest};

const EVIDENCE: &str = "heliotrope alpha heliotrope beta";
const CHUNK_BYTES: usize = 17;

fn actor_config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("7"),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

fn evidence_document() -> RememberInput {
    RememberInput {
        conversation: "evidence".to_owned(),
        content: EVIDENCE.to_owned(),
        kind: RememberKind::Document,
        chunk_bytes: Some(CHUNK_BYTES),
        anchor: None,
        retention: None,
        sensitivity: None,
        vocabulary: None,
        source: None,
        derive: None,
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

async fn attest_eight_times(
    server: &McpServer,
    uri: &str,
    disposition: AttestDisposition,
    prefix: &str,
) {
    for round in 1..=8 {
        let attested = server
            .attest_envelope(AttestInput {
                provenance: vec![uri.to_owned()],
                disposition,
                idempotency_key: format!("{prefix}-round-{round}"),
            })
            .await;
        assert!(attested.ok, "{attested:?}");
        assert_eq!(attested.items[0]["duplicate"], false);
    }
}

#[tokio::test]
async fn recall_without_feedback_is_unchanged_and_neutral() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());
    let remembered = server.remember_envelope(evidence_document()).await;
    assert!(remembered.ok, "{remembered:?}");
    assert_eq!(remembered.provenance.len(), 2);

    let recalled = server.recall_envelope(lexical_recall()).await;
    assert!(recalled.ok, "{recalled:?}");
    assert_eq!(recalled.items.len(), 2);
    assert_eq!(recalled.provenance.len(), 2);
    for (index, item) in recalled.items.iter().enumerate() {
        let score = item["score_q32"].as_u64().unwrap();
        assert!(score > 0, "{item:?}");
        assert_eq!(
            item["preference_q16"].as_u64().unwrap(),
            u64::from(PREFERENCE_NEUTRAL_Q16)
        );
        let uri = item["uri"].as_str().unwrap();
        assert!(!uri.contains("pref="), "{uri}");
        assert!(uri.ends_with(&format!("&score={score}")), "{uri}");
        assert_eq!(recalled.provenance[index], uri);
    }
    assert!(
        recalled.items[0]["lsn"].as_u64().unwrap() < recalled.items[1]["lsn"].as_u64().unwrap()
    );

    drop(server);
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn helpful_feedback_raises_only_the_attested_evidence() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());
    let remembered = server.remember_envelope(evidence_document()).await;
    assert!(remembered.ok, "{remembered:?}");

    let baseline = server.recall_envelope(lexical_recall()).await;
    assert_eq!(baseline.items.len(), 2);
    let target = baseline.items[0].clone();
    let untouched = baseline.items[1].clone();
    attest_eight_times(
        &server,
        target["uri"].as_str().unwrap(),
        AttestDisposition::Helpful,
        "attest",
    )
    .await;

    let adjusted = server.recall_envelope(lexical_recall()).await;
    assert!(adjusted.ok, "{adjusted:?}");
    assert_eq!(adjusted.items.len(), 2);
    let boosted = adjusted
        .items
        .iter()
        .find(|item| item["lsn"] == target["lsn"])
        .unwrap();
    assert!(
        boosted["score_q32"].as_u64().unwrap() > target["score_q32"].as_u64().unwrap(),
        "{boosted:?} {target:?}"
    );
    let weight = boosted["preference_q16"].as_u64().unwrap();
    assert!(weight > u64::from(PREFERENCE_NEUTRAL_Q16), "{weight}");
    assert!(weight <= u64::from(PREFERENCE_MAXIMUM_Q16), "{weight}");
    assert!(
        boosted["uri"]
            .as_str()
            .unwrap()
            .contains(&format!("&pref={weight}")),
        "{boosted:?}"
    );
    assert_eq!(adjusted.items[0]["lsn"], target["lsn"]);

    let kept = adjusted
        .items
        .iter()
        .find(|item| item["lsn"] == untouched["lsn"])
        .unwrap();
    assert_eq!(kept["score_q32"], untouched["score_q32"]);
    assert_eq!(
        kept["preference_q16"].as_u64().unwrap(),
        u64::from(PREFERENCE_NEUTRAL_Q16)
    );
    assert!(!kept["uri"].as_str().unwrap().contains("pref="));

    drop(server);
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn opposed_feedback_inverts_two_equally_scored_items() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());
    let remembered = server.remember_envelope(evidence_document()).await;
    assert!(remembered.ok, "{remembered:?}");

    let baseline = server.recall_envelope(lexical_recall()).await;
    assert_eq!(baseline.items.len(), 2);
    let first = baseline.items[0].clone();
    let second = baseline.items[1].clone();
    assert_eq!(
        first["score_q32"], second["score_q32"],
        "the inversion test needs two equally scored chunks"
    );

    attest_eight_times(
        &server,
        first["uri"].as_str().unwrap(),
        AttestDisposition::Harmful,
        "harmful",
    )
    .await;
    attest_eight_times(
        &server,
        second["uri"].as_str().unwrap(),
        AttestDisposition::Helpful,
        "helpful",
    )
    .await;

    let inverted = server.recall_envelope(lexical_recall()).await;
    assert!(inverted.ok, "{inverted:?}");
    assert_eq!(inverted.items.len(), 2);
    assert_eq!(inverted.items[0]["lsn"], second["lsn"]);
    assert_eq!(inverted.items[1]["lsn"], first["lsn"]);
    assert!(
        inverted.items[0]["score_q32"].as_u64().unwrap()
            > inverted.items[1]["score_q32"].as_u64().unwrap()
    );
    for (adjusted, original) in inverted.items.iter().zip([&second, &first]) {
        assert_eq!(adjusted["kind"], original["kind"]);
        assert_eq!(adjusted["content"], original["content"]);
        assert_eq!(adjusted["authority"], original["authority"]);
    }
    assert!(
        inverted.items[0]["preference_q16"].as_u64().unwrap() > u64::from(PREFERENCE_NEUTRAL_Q16)
    );
    assert!(
        inverted.items[1]["preference_q16"].as_u64().unwrap() < u64::from(PREFERENCE_NEUTRAL_Q16)
    );

    drop(server);
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn temporal_and_timeline_recall_ignore_feedback() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());
    let remembered = server.remember_envelope(evidence_document()).await;
    assert!(remembered.ok, "{remembered:?}");

    let timeline_request = RecallRequest::Timeline {
        conversation: ConversationId::derive("evidence"),
        since_lsn: LSN::new(0),
        limit: 10,
    };
    let timeline_before = actor.recall(timeline_request).await.unwrap();
    assert_eq!(timeline_before.len(), 2);
    let latest_ns = timeline_before
        .iter()
        .map(|item| item.wall_timestamp_ns.get())
        .max()
        .unwrap();
    let temporal_before = actor
        .recall(RecallRequest::Temporal {
            start_ns: 0,
            end_ns: latest_ns,
            limit: 10,
        })
        .await
        .unwrap();
    assert_eq!(temporal_before.len(), 2);

    let recalled = server.recall_envelope(lexical_recall()).await;
    attest_eight_times(
        &server,
        recalled.items[0]["uri"].as_str().unwrap(),
        AttestDisposition::Helpful,
        "attest",
    )
    .await;

    let timeline_after = actor
        .recall(RecallRequest::Timeline {
            conversation: ConversationId::derive("evidence"),
            since_lsn: LSN::new(0),
            limit: 10,
        })
        .await
        .unwrap();
    let temporal_after = actor
        .recall(RecallRequest::Temporal {
            start_ns: 0,
            end_ns: latest_ns,
            limit: 10,
        })
        .await
        .unwrap();
    assert_eq!(timeline_before, timeline_after);
    assert_eq!(temporal_before, temporal_after);
    for item in timeline_after.iter().chain(temporal_after.iter()) {
        assert_eq!(item.score_q32, 0);
        assert_eq!(item.preference_q16, PREFERENCE_NEUTRAL_Q16);
    }

    drop(server);
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn adjusted_order_survives_restart() {
    let temporary = tempfile::tempdir().unwrap();
    let configuration = actor_config(temporary.path());
    let actor = ActorEngine::open(configuration.clone()).await.unwrap();
    let server = McpServer::new(actor.clone());
    let remembered = server.remember_envelope(evidence_document()).await;
    assert!(remembered.ok, "{remembered:?}");

    let baseline = server.recall_envelope(lexical_recall()).await;
    assert_eq!(baseline.items.len(), 2);
    attest_eight_times(
        &server,
        baseline.items[1]["uri"].as_str().unwrap(),
        AttestDisposition::Helpful,
        "attest",
    )
    .await;
    let adjusted = server.recall_envelope(lexical_recall()).await;
    assert!(adjusted.ok, "{adjusted:?}");
    assert_eq!(adjusted.items[0]["lsn"], baseline.items[1]["lsn"]);
    assert!(
        adjusted.items[0]["preference_q16"].as_u64().unwrap() > u64::from(PREFERENCE_NEUTRAL_Q16)
    );

    drop(server);
    actor.shutdown().await.unwrap();

    let reopened = ActorEngine::open(configuration).await.unwrap();
    let server = McpServer::new(reopened.clone());
    let replayed = server.recall_envelope(lexical_recall()).await;
    assert!(replayed.ok, "{replayed:?}");
    assert_eq!(replayed.items, adjusted.items);
    assert_eq!(replayed.provenance, adjusted.provenance);

    drop(server);
    reopened.shutdown().await.unwrap();
}
