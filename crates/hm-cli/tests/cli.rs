use hm_serve::config::load;
use hm_serve::uds::UdsServer;
use serde_json::Value;
use std::process::Command;

fn hm(arguments: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_hm"))
        .args(arguments)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "hm {arguments:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn init_doctor_and_embedded_commands_are_json_capable() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().to_str().unwrap();
    let initialized = hm(&["init", "--path", root, "--actor", "7", "--json"]);
    assert_eq!(initialized["ok"], true);
    let config = temporary.path().join("hypermind.conf");
    let config = config.to_str().unwrap();

    let remembered = hm(&[
        "remember",
        "--config",
        config,
        "--conversation",
        "cli-test",
        "--content",
        "cli heliotrope fact",
        "--embedded",
        "--json",
    ]);
    assert_eq!(remembered["first_lsn"], 1);

    let recalled = hm(&[
        "recall",
        "--config",
        config,
        "--query",
        "heliotrope",
        "--embedded",
        "--json",
    ]);
    assert_eq!(recalled["items"].as_array().unwrap().len(), 1);

    let activated = hm(&[
        "activate",
        "--config",
        config,
        "--conversation",
        "cli-test",
        "--query",
        "heliotrope",
        "--budget-tokens",
        "1024",
        "--embedded",
        "--json",
    ]);
    assert!(activated["bundle_hash"].as_str().unwrap().len() == 64);
    assert!(activated["sections"].is_array());

    let consolidated = hm(&[
        "consolidate",
        "run",
        "--config",
        config,
        "--mode",
        "both",
        "--cadence-key",
        "cli-night-1",
        "--max-llm-calls",
        "8",
        "--max-tokens",
        "16000",
        "--max-microusd",
        "50000",
        "--max-wall-ms",
        "30000",
        "--json",
    ]);
    assert_eq!(consolidated["ok"], true);
    let run_id = consolidated["items"][0]["run_id"].as_str().unwrap();
    let listed = hm(&["consolidate", "list", "--config", config, "--json"]);
    assert_eq!(listed["items"].as_array().unwrap().len(), 1);
    let retracted = hm(&[
        "consolidate",
        "retract",
        "--config",
        config,
        "--run-id",
        run_id,
        "--reason",
        "cli rollback",
        "--json",
    ]);
    assert_eq!(retracted["items"][0]["status"], "retracted");

    let diagnosed = hm(&["doctor", "--config", config, "--json"]);
    assert_eq!(diagnosed["ok"], true);
    assert!(
        diagnosed["toolchain"]
            .as_str()
            .unwrap()
            .starts_with("rustc ")
    );
    assert_eq!(diagnosed["socket_ready"], false);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn commands_use_the_running_daemon_when_not_embedded() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().to_str().unwrap();
    hm(&["init", "--path", root, "--json"]);
    let config_path = temporary.path().join("hypermind.conf");
    let config = load(&config_path).unwrap();
    let server = UdsServer::bind(config).await.unwrap();
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let server_task = tokio::spawn(server.serve_until(async {
        let _ = shutdown_rx.await;
    }));
    let config_text = config_path.to_str().unwrap();

    let remembered = hm(&[
        "remember",
        "--config",
        config_text,
        "--conversation",
        "daemon-cli",
        "--content",
        "daemon heliotrope fact",
        "--json",
    ]);
    assert_eq!(remembered["first_lsn"], 1);
    let recalled = hm(&[
        "recall",
        "--config",
        config_text,
        "--query",
        "heliotrope",
        "--json",
    ]);
    assert_eq!(recalled["lsns"], serde_json::json!([1]));
    let activated = hm(&[
        "activate",
        "--config",
        config_text,
        "--conversation",
        "daemon-cli",
        "--query",
        "heliotrope",
        "--budget-tokens",
        "1024",
        "--json",
    ]);
    assert!(activated["canonical_bundle_base64"].as_str().unwrap().len() > 8);

    shutdown_tx.send(()).unwrap();
    server_task.await.unwrap().unwrap();
}
