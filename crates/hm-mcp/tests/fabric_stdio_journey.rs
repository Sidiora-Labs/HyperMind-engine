#![cfg(target_os = "linux")]

use hm_context::digest_bytes;
use hm_fabric::bus_memory::MemoryBusLimits;
use rmcp::{
    ServiceExt,
    model::CallToolRequestParams,
    service::{RoleClient, RunningService},
};
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::process::{Child, Command};
fn private(path: &Path, bytes: &[u8]) {
    let mut f = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o600)
        .open(path)
        .unwrap();
    f.write_all(bytes).unwrap();
}
fn scope() -> Value {
    json!({"owner_id":"stdio-operator","project_id":"stdio-journey","workspace_id":null})
}
fn command(root: &Path, fabric: Option<&Path>) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_hm-mcp"));
    c.arg("--config")
        .arg(root.join("server.conf"))
        .arg("--context-scope")
        .arg(root.join("context.json"))
        .env_remove("HM_FABRIC_CONFIG")
        .env("HM_EMBED_PROVIDER", "off")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    if let Some(path) = fabric {
        c.env("HM_FABRIC_CONFIG", path);
    }
    c
}
struct Client {
    service: RunningService<RoleClient, ()>,
    child: Child,
}
impl Client {
    async fn start(root: &Path, fabric: Option<&Path>) -> Self {
        let mut child = command(root, fabric).spawn().unwrap();
        let output = child.stdout.take().unwrap();
        let input = child.stdin.take().unwrap();
        let service = tokio::time::timeout(Duration::from_secs(10), ().serve((output, input)))
            .await
            .unwrap()
            .unwrap();
        Self { service, child }
    }
    async fn call(&self, verb: &str, input: Value) -> Value {
        tokio::time::timeout(
            Duration::from_secs(10),
            self.service.call_tool(
                CallToolRequestParams::new(verb.to_owned())
                    .with_arguments(input.as_object().unwrap().clone()),
            ),
        )
        .await
        .unwrap()
        .unwrap()
        .structured_content
        .unwrap()
    }
    async fn close(mut self, workers: &[u32]) {
        self.service.cancel().await.unwrap();
        let status = tokio::time::timeout(Duration::from_secs(10), self.child.wait())
            .await
            .unwrap()
            .unwrap();
        assert!(status.success(), "stdio shutdown status {status}");
        for pid in workers {
            assert!(
                !PathBuf::from(format!("/proc/{pid}")).exists(),
                "owned worker survived stdio EOF"
            );
        }
    }
}
fn children(pid: u32) -> Vec<u32> {
    let mut pids = Vec::new();
    for task in fs::read_dir(format!("/proc/{pid}/task")).unwrap() {
        if let Ok(text) = fs::read_to_string(task.unwrap().path().join("children")) {
            pids.extend(text.split_whitespace().map(|p| p.parse::<u32>().unwrap()));
        }
    }
    pids.sort_unstable();
    pids.dedup();
    pids
}
async fn refused(root: &Path, path: &Path, home: &Path) {
    let mut child = command(root, Some(path)).spawn().unwrap();
    drop(child.stdin.take());
    let status = tokio::time::timeout(Duration::from_secs(10), child.wait())
        .await
        .unwrap()
        .unwrap();
    assert!(!status.success());
    assert_eq!(
        fs::read_dir(home).unwrap().count(),
        0,
        "refused startup opened backend"
    );
    assert!(children_if_live(child.id()).is_empty());
}
fn children_if_live(pid: Option<u32>) -> Vec<u32> {
    pid.filter(|pid| PathBuf::from(format!("/proc/{pid}")).exists())
        .map(children)
        .unwrap_or_default()
}
#[tokio::test]
async fn actual_stdio_startup_replay_restart_and_refusal() {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let home = root.path().join("backend");
    fs::create_dir(&home).unwrap();
    private(&root.path().join("server.conf"),format!("socket={}\ndata={}\nuser={}\nkek={}\nadmin_token={}\nactor=7:{}\nprojection_map_bytes=16777216\n",root.path().join("hm.sock").display(),root.path().join("data").display(),"01".repeat(16),"02".repeat(32),"03".repeat(32),"04".repeat(32)).as_bytes());
    private(
        &root.path().join("context.json"),
        &serde_json::to_vec(&json!({"version":1,"actor":7,"scope":scope()})).unwrap(),
    );
    let missing = Client::start(root.path(), None).await;
    let unavailable = missing
        .call("inspect", json!({"uri":"hm://7/context-fabric"}))
        .await;
    assert_eq!(unavailable["ok"], false);
    assert!(children(missing.child.id().unwrap()).is_empty());
    missing.close(&[]).await;
    assert_eq!(fs::read_dir(&home).unwrap().count(), 0);
    let absent = root.path().join("absent.json");
    refused(root.path(), &absent, &home).await;
    let worker = PathBuf::from(
        std::env::var_os("HM_FABRIC_WORKER_EXE")
            .expect("actual hm fabric-worker executable required"),
    );
    assert!(worker.is_absolute() && worker.is_file());
    let good = json!({"version":1,"actor":7,"backend":{"version":1,"scope":scope(),"home":{"path":home,"base":null,"missing":{"policy":"refuse"}},"operational":{"backend":"sqlite"},"bus":{"backend":"process_local","limits":MemoryBusLimits::default()}},"worker":{"module_id":"native-digest","command":worker,"args":["fabric-worker"],"cwd":home},"client_key":vec![7;32],"worker_key":vec![8;32]});
    let path = root.path().join("fabric.json");
    let bad = root.path().join("bad.json");
    let mut foreign = good.clone();
    foreign["backend"]["scope"]["owner_id"] = json!("foreign");
    private(&bad, &serde_json::to_vec(&foreign).unwrap());
    refused(root.path(), &bad, &home).await;
    private(&bad, &serde_json::to_vec(&good).unwrap());
    fs::set_permissions(&bad, fs::Permissions::from_mode(0o644)).unwrap();
    refused(root.path(), &bad, &home).await;
    private(&path, &serde_json::to_vec(&good).unwrap());
    let client = Client::start(root.path(), Some(&path)).await;
    let workers = children(client.child.id().unwrap());
    assert_eq!(workers.len(), 1);
    let request = json!({"conversation":"fabric","content":"","kind":"user","context":{"operation":"fabric","request":{"version":1,"scope":scope(),"request_id":"stdio-digest","action":{"action":"dispatch","operation":"digest","bytes":b"actual stdio bytes".to_vec(),"timeout_ms":10000}}}});
    let first = client.call("remember", request.clone()).await;
    assert_eq!(first["ok"], true, "{first}");
    assert_eq!(
        first["items"][0]["result"]["digest"],
        digest_bytes(b"actual stdio bytes")
    );
    assert_eq!(
        client.call("remember", request.clone()).await["items"],
        first["items"]
    );
    let before = client
        .call("inspect", json!({"uri":"hm://7/context-fabric"}))
        .await;
    assert_eq!(before["ok"], true);
    assert_eq!(
        before["items"][0]["descriptors"]["effect_authority"],
        "selected_operational"
    );
    client.close(&workers).await;
    let restarted = Client::start(root.path(), Some(&path)).await;
    let workers = children(restarted.child.id().unwrap());
    assert_eq!(workers.len(), 1);
    let replay = restarted.call("remember", request).await;
    assert_eq!(replay["items"], first["items"]);
    let after = restarted
        .call("inspect", json!({"uri":"hm://7/context-fabric"}))
        .await;
    assert_eq!(
        after["items"][0]["receipts"], before["items"][0]["receipts"],
        "replay appended effect receipts"
    );
    restarted.close(&workers).await;
    assert!(!home.join("effects.sqlite").exists());
}
