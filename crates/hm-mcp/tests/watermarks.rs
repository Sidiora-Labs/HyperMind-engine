#![forbid(unsafe_code)]
#![allow(clippy::too_many_lines)]

use hm_core::ActorId;
use hm_mcp::{
    ConsolidateAction, ConsolidateBudget, ConsolidateInput, ConsolidateMode, McpServer,
    RememberInput, RememberKind,
};
use hm_serve::actor::{ActorConfig, ActorEngine};

fn actor_config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("7"),
        actor: ActorId::new(7),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 32 * 1024 * 1024,
    }
}

fn run_input(cadence_key: &str) -> ConsolidateInput {
    ConsolidateInput {
        action: ConsolidateAction::Run,
        mode: Some(ConsolidateMode::Nrem),
        scope: Some("watermarks".to_owned()),
        cadence_key: Some(cadence_key.to_owned()),
        budget: Some(ConsolidateBudget {
            max_llm_calls: 8,
            max_tokens: 16_000,
            max_microusd: 50_000,
            max_wall_ms: 30_000,
        }),
        run_id: None,
        reason: None,
    }
}

fn list_input() -> ConsolidateInput {
    ConsolidateInput {
        action: ConsolidateAction::List,
        mode: None,
        scope: None,
        cadence_key: None,
        budget: None,
        run_id: None,
        reason: None,
    }
}

async fn remember(server: &McpServer, content: &str) {
    let envelope = server
        .remember_envelope(RememberInput {
            conversation: "watermark-window".to_owned(),
            content: content.to_owned(),
            kind: RememberKind::User,
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
        })
        .await;
    assert!(envelope.ok, "{envelope:?}");
}

async fn head(actor: &ActorEngine) -> u64 {
    actor.stats().await.unwrap().applied.last_lsn.get()
}

async fn watermark(server: &McpServer) -> u64 {
    let listed = server.consolidate_envelope(list_input()).await;
    assert!(listed.ok, "{listed:?}");
    let watermarks = listed.health["watermarks"].as_array().unwrap().clone();
    assert_eq!(watermarks.len(), 1, "{watermarks:?}");
    watermarks[0]["through_lsn"].as_u64().unwrap()
}

#[tokio::test]
async fn incremental_runs_consume_disjoint_source_windows_and_retraction_rewinds_the_watermark() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(actor_config(temporary.path()))
        .await
        .unwrap();
    let server = McpServer::new(actor.clone());

    let empty = server.consolidate_envelope(run_input("night-0")).await;
    assert!(empty.ok, "{empty:?}");
    assert_eq!(empty.items[0]["extracted"], false);
    assert_eq!(head(&actor).await, 0);

    for index in 0..4 {
        remember(&server, &format!("deployment observation {index}")).await;
    }
    let head_before_first = head(&actor).await;
    assert_eq!(head_before_first, 4);

    let first = server.consolidate_envelope(run_input("night-1")).await;
    assert!(first.ok, "{first:?}");
    assert_eq!(first.items[0]["status"], "published");
    assert_eq!(first.items[0]["extracted"], true);
    assert_eq!(first.items[0]["source_first_lsn"], 1);
    assert_eq!(first.items[0]["source_last_lsn"], head_before_first);
    assert_eq!(first.items[0]["source_events"], 4);
    assert_eq!(watermark(&server).await, head_before_first);

    let head_after_first = head(&actor).await;
    let replay = server.consolidate_envelope(run_input("night-1")).await;
    assert!(replay.ok, "{replay:?}");
    assert_eq!(replay.items[0]["duplicate"], true);
    assert_eq!(replay.items[0]["run_id"], first.items[0]["run_id"]);
    assert_eq!(head(&actor).await, head_after_first);

    for index in 4..6 {
        remember(&server, &format!("deployment observation {index}")).await;
    }
    let head_before_second = head(&actor).await;
    let second = server.consolidate_envelope(run_input("night-2")).await;
    assert!(second.ok, "{second:?}");
    assert_eq!(second.items[0]["status"], "published");
    assert_eq!(second.items[0]["source_first_lsn"], head_before_first + 1);
    assert_eq!(second.items[0]["source_last_lsn"], head_before_second);
    assert_eq!(second.items[0]["source_events"], 2);
    assert_eq!(watermark(&server).await, head_before_second);

    let retracted = server
        .consolidate_envelope(ConsolidateInput {
            action: ConsolidateAction::Retract,
            run_id: second.items[0]["run_id"].as_str().map(str::to_owned),
            reason: Some("rewind the extraction window".to_owned()),
            mode: None,
            scope: None,
            cadence_key: None,
            budget: None,
        })
        .await;
    assert!(retracted.ok, "{retracted:?}");
    assert_eq!(watermark(&server).await, head_before_first);

    let head_before_third = head(&actor).await;
    let third = server.consolidate_envelope(run_input("night-3")).await;
    assert!(third.ok, "{third:?}");
    assert_eq!(third.items[0]["status"], "published");
    assert_eq!(
        third.items[0]["source_first_lsn"],
        second.items[0]["source_first_lsn"]
    );
    assert_eq!(third.items[0]["source_last_lsn"], head_before_third);
    assert_eq!(third.items[0]["source_events"], 2);
    assert_eq!(watermark(&server).await, head_before_third);

    drop(server);
    actor.shutdown().await.unwrap();
}
