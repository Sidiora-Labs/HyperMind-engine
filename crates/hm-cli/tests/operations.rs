#![allow(clippy::too_many_lines)]

use hm_core::ActorId;
use hm_ledger::keyring::{KeyHierarchy, OsEntropy};
use hm_serve::config::load;
use hm_serve::uds::UdsServer;
use serde_json::Value;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};
use std::sync::Arc;

fn call(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_hm"))
        .args(args)
        .env_remove("HM_CONSOLIDATION_PROVIDER")
        .env_remove("HM_RECONSTRUCTION_PROVIDER")
        .env_remove("HM_EMBEDDING_PROVIDER")
        .output()
        .unwrap()
}
fn hm(args: &[&str]) -> Value {
    let output = call(args);
    assert!(
        output.status.success(),
        "hm {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn initialize(directory: &Path) -> String {
    hm(&[
        "init",
        "--path",
        directory.to_str().unwrap(),
        "--actor",
        "7",
        "--json",
    ]);
    directory.join("hypermind.conf").to_str().unwrap().into()
}

#[test]
fn real_actor_admin_export_import_rotation_and_signed_shred() {
    let temporary = tempfile::tempdir().unwrap();
    let config_path = initialize(temporary.path());
    hm(&[
        "actor",
        "add",
        "--config",
        &config_path,
        "--actor",
        "8",
        "--json",
    ]);
    assert_eq!(
        hm(&["actor", "list", "--config", &config_path, "--json"])["actors"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    hm(&[
        "remember",
        "--config",
        &config_path,
        "--conversation",
        "operations",
        "--content",
        "heliotrope persists after key rotation",
        "--embedded",
        "--json",
    ]);
    let exported = temporary.path().join("export.jsonl");
    assert_eq!(
        hm(&[
            "export",
            "--config",
            &config_path,
            "--output",
            exported.to_str().unwrap(),
            "--json"
        ])["events"],
        1
    );
    assert_eq!(
        fs::metadata(&exported).unwrap().permissions().mode() & 0o077,
        0
    );
    let destination = tempfile::tempdir().unwrap();
    let destination_config = initialize(destination.path());
    assert_eq!(
        hm(&[
            "import",
            "--config",
            &destination_config,
            "--input",
            exported.to_str().unwrap(),
            "--json"
        ])["verified"],
        true
    );
    assert_eq!(
        hm(&[
            "import",
            "--config",
            &destination_config,
            "--input",
            exported.to_str().unwrap(),
            "--json"
        ])["resumed"],
        true
    );
    assert_eq!(
        hm(&[
            "recall",
            "--config",
            &destination_config,
            "--query",
            "heliotrope",
            "--embedded",
            "--json"
        ])["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let old = load(&config_path).unwrap();
    let key_file = temporary.path().join("new-kek");
    fs::write(&key_file, "ab".repeat(32)).unwrap();
    fs::set_permissions(&key_file, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(
        hm(&[
            "actor",
            "rotate-keys",
            "--config",
            &config_path,
            "--new-kek-file",
            key_file.to_str().unwrap(),
            "--json"
        ])["rotated_actors"],
        serde_json::json!([7, 8])
    );
    let current = load(&config_path).unwrap();
    assert_ne!(old.kek, current.kek);
    for actor in [7, 8] {
        assert!(
            KeyHierarchy::open_or_create(
                current.actor_directory(actor),
                ActorId::new(actor),
                current.user,
                &old.kek,
                &mut OsEntropy,
                false
            )
            .is_err()
        );
        assert!(
            KeyHierarchy::open_or_create(
                current.actor_directory(actor),
                ActorId::new(actor),
                current.user,
                &current.kek,
                &mut OsEntropy,
                false
            )
            .is_ok()
        );
    }
    assert_eq!(
        hm(&[
            "recall",
            "--config",
            &config_path,
            "--query",
            "heliotrope",
            "--embedded",
            "--json"
        ])["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(
        !call(&[
            "actor",
            "shred",
            "--config",
            &config_path,
            "--actor",
            "7",
            "--confirm",
            "8",
            "--json"
        ])
        .status
        .success()
    );
    let deleted = hm(&[
        "actor",
        "shred",
        "--config",
        &config_path,
        "--actor",
        "7",
        "--confirm",
        "7",
        "--json",
    ]);
    assert_eq!(deleted["irreversible"], true);
    assert!(!current.actor_directory(7).join("keys/KEYRING").exists());
    let bytes = fs::read(current.actor_directory(7).join("keys/DELETION_RECEIPT")).unwrap();
    let receipt = hm_ledger::shred::decode_deletion_receipt(&bytes).unwrap();
    hm_ledger::shred::verify_deletion_receipt(&receipt, &receipt.public_key).unwrap();
    assert_eq!(load(&config_path).unwrap().actors[0].actor, 8);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_daemon_doctor_bench_and_live_activation_viewer() {
    let temporary = tempfile::tempdir().unwrap();
    let config_path = initialize(temporary.path());
    let before = hm(&["doctor", "--config", &config_path, "--json"]);
    assert_eq!(before["healthy"], false);
    assert_eq!(before["missing_models"].as_array().unwrap().len(), 2);
    let config = load(&config_path).unwrap();
    let server = UdsServer::bind(config)
        .await
        .unwrap()
        .with_tool_dispatcher(Arc::new(hm_mcp::dispatcher::McpToolDispatcher::default()));
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(server.serve_until(async {
        let _ = stopped.await;
    }));
    hm(&[
        "remember",
        "--config",
        &config_path,
        "--conversation",
        "operations",
        "--content",
        "heliotrope live viewer evidence",
        "--json",
    ]);
    let doctor = hm(&[
        "doctor",
        "--config",
        &config_path,
        "--require-healthy",
        "--json",
    ]);
    assert_eq!(doctor["healthy"], true);
    assert_eq!(
        doctor["actor_directories"][0]["indexes"]["status"],
        "current"
    );
    let bench = hm(&[
        "bench",
        "--config",
        &config_path,
        "--query",
        "heliotrope",
        "--iterations",
        "3",
        "--json",
    ]);
    assert_eq!(bench["encoder"], "lexical_only");
    assert!(bench["hits"].as_u64().unwrap() > 0);
    let tui = call(&[
        "tui",
        "--config",
        &config_path,
        "--conversation",
        "operations",
        "--query",
        "heliotrope",
        "--frames",
        "2",
        "--interval-ms",
        "100",
        "--json",
    ]);
    assert!(
        tui.status.success(),
        "{}",
        String::from_utf8_lossy(&tui.stderr)
    );
    let text = String::from_utf8(tui.stdout).unwrap();
    assert_eq!(text.lines().count(), 2);
    assert!(text.contains("heliotrope live viewer evidence"));
    assert!(
        !call(&[
            "actor",
            "add",
            "--config",
            &config_path,
            "--actor",
            "8",
            "--json"
        ])
        .status
        .success()
    );
    stop.send(()).unwrap();
    server.await.unwrap().unwrap();
}

#[test]
fn cached_model_digest_mismatch_is_reported_without_provider_or_download() {
    let temporary = tempfile::tempdir().unwrap();
    let directory = temporary.path().to_str().unwrap();
    let listed = hm(&["models", "list", "--directory", directory, "--json"]);
    assert!(
        listed["models"]
            .as_array()
            .unwrap()
            .iter()
            .all(|model| model["present"] == false)
    );
    let spec = hm_embed::model::ModelKind::BgeSmallEnV15.spec();
    let cache = temporary
        .path()
        .join(spec.encoder_id.replace('/', "--"))
        .join(spec.revision);
    fs::create_dir_all(&cache).unwrap();
    fs::write(cache.join(spec.model.name), b"corrupt artifact").unwrap();
    let output = call(&[
        "models",
        "ensure",
        "--directory",
        directory,
        "--model",
        "bge-small",
        "--json",
    ]);
    assert!(!output.status.success());
    assert!(!cache.join(spec.tokenizer.name).exists());
    assert_eq!(
        hm(&["models", "list", "--directory", directory, "--json"])["models"][0]["artifacts"][0]["status"],
        "invalid"
    );
}
