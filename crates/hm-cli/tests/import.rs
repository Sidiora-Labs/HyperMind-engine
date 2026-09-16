use hm_cli::import;
use hm_core::{ActorId, LSN};
use hm_proj::fsrs::FsrsProjection;
use hm_proj::graph::GraphProjection;
use hm_proj::memories::MemoryProjection;
use hm_proj::runs::RunsProjection;
use hm_proj::store::ProjectionStore;
use hm_schema::events::{Authority, EventPayload};
use hm_serve::actor::{ActorConfig, ActorEngine};
use hm_serve::config::{ActorCapability, ServerConfig};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn config(directory: &Path) -> ServerConfig {
    ServerConfig {
        socket_path: directory.join("hm.sock"),
        data_directory: directory.join("data"),
        user: [1; 16],
        kek: [2; 32],
        admin_token: [3; 32],
        actors: vec![ActorCapability {
            actor: 7,
            token: [4; 32],
        }],
        maximum_connections: 8,
        maximum_output_frames: 64,
        maximum_output_bytes: 16 * 1024 * 1024,
        projection_map_bytes: 64 * 1024 * 1024,
    }
}

fn export(source: &Path, destination: &Path) {
    let output = Command::new("node")
        .arg(root().join("sdk/typescript/packages/migrate/dist/cli.js"))
        .args(["--format", "json", "--source"])
        .arg(source)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::write(destination, output.stdout).unwrap();
}

#[tokio::test]
async fn original_cortex_vectors_import_verify_and_resume_after_restart() {
    let directory = tempfile::tempdir().unwrap();
    let config = config(directory.path());
    let stream = directory.path().join("source.jsonl");
    export(
        &root().join("eval/fixtures/cortex-store/donor-store.json"),
        &stream,
    );
    let report = import::run(&config, &stream).await.unwrap();
    assert_eq!(report["verified"], true);
    assert_eq!(
        report["counts"],
        json!({"observations":2,"memories":3,"edges":2,"beliefs":2,"ops":2,"signals":2,"generic":2})
    );
    assert_eq!(report["embedding_samples_verified"], 3);
    assert_eq!(
        report["gaps"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|gap| gap["kind"] == "source_observations_unavailable")
            .count(),
        3
    );
    let second = import::run(&config, &stream).await.unwrap();
    assert_eq!(second["resumed"], true);
    assert_eq!(report["events"], second["events"]);
    let store =
        ProjectionStore::open(config.actor_directory(7), config.projection_map_bytes).unwrap();
    let snapshot = store.begin_snapshot().unwrap();
    let generation = RunsProjection::active_generation(&snapshot).unwrap();
    assert_eq!(generation, 1);
    assert_eq!(
        MemoryProjection::list_visible(&snapshot, generation, 10)
            .unwrap()
            .len(),
        3
    );
    let memory = MemoryProjection::get_visible(&snapshot, generation, b"mem-1")
        .unwrap()
        .unwrap();
    assert_eq!(memory.definition, b"def-mem-1");
    let fsrs = FsrsProjection::get_visible(&snapshot, generation, b"mem-1")
        .unwrap()
        .unwrap();
    assert_eq!(fsrs.stability_millis, 270103680);
    assert_eq!(fsrs.difficulty_micros, 721020);
    let edges = GraphProjection::neighbours(&snapshot, generation, b"mem-2", i64::MAX, 10).unwrap();
    assert_eq!(edges.len(), 2);
}

#[tokio::test]
async fn citations_authority_validity_and_full_fsrs_are_preserved_without_invented_tokens() {
    let directory = tempfile::tempdir().unwrap();
    let config = config(directory.path());
    let mut donor: Value = serde_json::from_slice(
        &std::fs::read(root().join("eval/fixtures/cortex-store/donor-store.json")).unwrap(),
    )
    .unwrap();
    donor["memories"]["mem-1"]["source_files"] = json!(["f.md"]);
    donor["memories"]["mem-1"]["memory_origin"] = json!("dream");
    donor["memories"]["mem-1"]["fsrs"]["reps"] = json!(7);
    donor["memories"]["mem-1"]["fsrs"]["lapses"] = json!(2);
    donor["memories"]["mem-1"]["fsrs"]["state"] = json!("relearning");
    donor["observations"]["obs-2"]["source_type"] = json!("assistant");
    donor["beliefs"]["bel-1"]["valid_from"] = json!("2026-05-16T10:00:00.000Z");
    donor["beliefs"]["bel-1"]["valid_to"] = json!("2026-05-17T10:00:00.000Z");
    let source = directory.path().join("linked.json");
    std::fs::write(&source, serde_json::to_vec(&donor).unwrap()).unwrap();
    let stream = directory.path().join("linked.jsonl");
    export(&source, &stream);
    let report = import::run(&config, &stream).await.unwrap();
    let actor = ActorEngine::open(ActorConfig {
        actor_directory: config.actor_directory(7),
        actor: ActorId::new(7),
        user: config.user,
        kek: config.kek,
        projection_map_bytes: config.projection_map_bytes,
    })
    .await
    .unwrap();
    let mappings = report["mappings"].as_array().unwrap();
    let mapping = |kind: &str, id: &str| {
        mappings
            .iter()
            .find(|row| row["kind"] == kind && row["id"] == id)
            .unwrap()
    };
    let obs1 = mapping("observations", "obs-1")["event_lsn"]
        .as_u64()
        .unwrap();
    let obs2 = mapping("observations", "obs-2")["event_lsn"]
        .as_u64()
        .unwrap();
    assert_eq!(
        actor
            .verified_event(LSN::new(obs1))
            .await
            .unwrap()
            .envelope
            .authority,
        Authority::UserAsserted
    );
    assert_eq!(
        actor
            .verified_event(LSN::new(obs2))
            .await
            .unwrap()
            .envelope
            .authority,
        Authority::AssistantGenerated
    );
    let memory = actor
        .verified_event(LSN::new(
            mapping("memories", "mem-1")["event_lsn"].as_u64().unwrap(),
        ))
        .await
        .unwrap()
        .envelope;
    assert_eq!(memory.authority, Authority::DerivedInference);
    let model = memory.model_provenance.unwrap();
    assert_eq!(model.input_tokens + model.output_tokens, 0);
    assert_eq!(model.model_id, "cortex-store-import");
    let EventPayload::MemoryMinted(memory) = memory.payload else {
        panic!("wrong memory event")
    };
    assert_eq!(memory.memory_id, b"mem-1");
    assert_eq!(
        memory
            .citations
            .iter()
            .map(|citation| citation.first_lsn)
            .collect::<Vec<_>>(),
        vec![obs1, obs2]
    );
    let original = actor
        .verified_event(LSN::new(
            mapping("memories", "mem-1")["source_lsn"].as_u64().unwrap(),
        ))
        .await
        .unwrap()
        .envelope;
    let EventPayload::ProviderFrame(original) = original.payload else {
        panic!("wrong archive event")
    };
    let original: Value = serde_json::from_slice(&original.api_content).unwrap();
    assert_eq!(
        original["record"]["fsrs"],
        donor["memories"]["mem-1"]["fsrs"]
    );
    let belief = actor
        .verified_event(LSN::new(
            mapping("beliefs", "bel-1")["event_lsn"].as_u64().unwrap(),
        ))
        .await
        .unwrap()
        .envelope;
    let EventPayload::Assertion(belief) = belief.payload else {
        panic!("wrong belief event")
    };
    assert_eq!(belief.belief_id, b"bel-1");
    assert_eq!(
        belief.valid_to_ns - belief.valid_from_ns,
        86_400_000_000_000
    );
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn truncated_or_corrupt_stream_is_rejected_before_creating_actor() {
    let directory = tempfile::tempdir().unwrap();
    let config = config(directory.path());
    let stream = directory.path().join("source.jsonl");
    export(
        &root().join("eval/fixtures/cortex-store/donor-store.json"),
        &stream,
    );
    let valid = std::fs::read_to_string(&stream).unwrap();
    let lines: Vec<_> = valid.lines().collect();
    std::fs::write(&stream, lines[..lines.len() - 1].join("\n")).unwrap();
    assert!(
        import::run(&config, &stream)
            .await
            .unwrap_err()
            .to_string()
            .contains("counts")
    );
    assert!(!config.actor_directory(7).exists());
    let broken = valid.replacen("zczMPc3MTD6amZk+", "AAAAAAAAAAAAAAAA", 1);
    assert_ne!(valid, broken);
    std::fs::write(&stream, broken).unwrap();
    assert!(import::run(&config, &stream).await.is_err());
    assert!(!config.actor_directory(7).exists());
}
