use hm_fabric::bus_memory::MemoryBusLimits;
use hm_schema::{
    protocol::{encode_wire_envelope, verify_wire_envelope},
    wire::{
        Hello, Request, RequestPayload, ResponsePayload, ToolRequest, WireEnvelope, WirePayload,
    },
};
use hm_serve::protocol::{FrameParser, encode_frame};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt, symlink},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};
fn private(path: &Path, bytes: &[u8]) {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
}
fn scope() -> Value {
    json!({"owner_id":"fabric-operator","project_id":"daemon-journey","workspace_id":null})
}
fn startup(home: &Path) -> Value {
    json!({"version":1,"actor":7,"backend":{"version":1,"scope":scope(),"home":{"path":home,"base":null,"missing":{"policy":"refuse"}},"operational":{"backend":"sqlite"},"bus":{"backend":"process_local","limits":MemoryBusLimits::default()}},"worker":{"module_id":"native-digest","command":env!("CARGO_BIN_EXE_hm"),"args":["fabric-worker"],"cwd":home},"client_key":vec![7;32],"worker_key":vec![8;32]})
}
struct Daemon(Child);
impl Drop for Daemon {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_some() {
            return;
        }
        let _ = Command::new("/bin/kill")
            .args(["-TERM", &self.0.id().to_string()])
            .status();
        for _ in 0..300 {
            if self.0.try_wait().ok().flatten().is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn start(root: &Path, fabric: &Path) -> Daemon {
    Daemon(
        Command::new(env!("CARGO_BIN_EXE_hm"))
            .args(["serve", "--config"])
            .arg(root.join("server.conf"))
            .arg("--context-scope")
            .arg(root.join("context.json"))
            .env("HM_FABRIC_CONFIG", fabric)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    )
}
async fn socket(root: &Path, daemon: &mut Daemon) -> UnixStream {
    for _ in 0..300 {
        assert!(
            daemon.0.try_wait().unwrap().is_none(),
            "actual daemon exited during startup"
        );
        if let Ok(stream) = UnixStream::connect(root.join("hm.sock")).await {
            return stream;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("actual daemon socket readiness timed out")
}
async fn exchange(stream: &mut UnixStream, payload: WirePayload) -> WireEnvelope {
    let bytes = encode_frame(&encode_wire_envelope(&WireEnvelope {
        proto_version: 2,
        payload,
    }))
    .unwrap();
    stream.write_all(&bytes).await.unwrap();
    let mut header = [0; 8];
    stream.read_exact(&mut header).await.unwrap();
    let length = u32::from_le_bytes(header[..4].try_into().unwrap()) as usize;
    assert!(length < 1024 * 1024);
    let mut payload = vec![0; length];
    stream.read_exact(&mut payload).await.unwrap();
    let parsed = FrameParser::default()
        .push(&[header.as_slice(), payload.as_slice()].concat())
        .unwrap();
    verify_wire_envelope(&parsed[0]).unwrap()
}
async fn hello(stream: &mut UnixStream) {
    assert!(matches!(
        exchange(
            stream,
            WirePayload::Hello(Box::new(Hello {
                proto_version: 2,
                connection_id: vec![9; 16],
                capability_token: vec![4; 32]
            }))
        )
        .await
        .payload,
        WirePayload::Welcome(_)
    ));
}
async fn tool(stream: &mut UnixStream, id: u64, verb: &str, input: Value) -> Value {
    let reply = exchange(
        stream,
        WirePayload::Request(Box::new(Request {
            request_id: id,
            payload: RequestPayload::ToolRequest(Box::new(ToolRequest {
                verb: verb.into(),
                arguments_json: serde_json::to_vec(&input).unwrap(),
            })),
        })),
    )
    .await;
    let WirePayload::Response(reply) = reply.payload else {
        panic!("expected actual tool response")
    };
    let Some(ResponsePayload::BytesResult(reply)) = reply.payload else {
        panic!("expected actual tool bytes")
    };
    serde_json::from_slice(&reply.bytes).unwrap()
}
fn children(pid: u32) -> Vec<u32> {
    let mut result = Vec::new();
    for task in fs::read_dir(format!("/proc/{pid}/task")).unwrap() {
        if let Ok(text) = fs::read_to_string(task.unwrap().path().join("children")) {
            result.extend(
                text.split_whitespace()
                    .map(|pid| pid.parse::<u32>().unwrap()),
            );
        }
    }
    result.sort_unstable();
    result.dedup();
    result
}
async fn stop(daemon: &mut Daemon, worker: u32) {
    assert!(
        Command::new("/bin/kill")
            .args(["-TERM", &daemon.0.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    for _ in 0..300 {
        if let Some(status) = daemon.0.try_wait().unwrap() {
            assert!(status.success());
            assert!(
                !PathBuf::from(format!("/proc/{worker}")).exists(),
                "owned worker survived daemon shutdown"
            );
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("daemon shutdown timed out")
}
async fn refused(root: &Path, config: &Path, home: &Path) {
    let mut daemon = start(root, config);
    for _ in 0..300 {
        if let Some(status) = daemon.0.try_wait().unwrap() {
            assert!(!status.success());
            assert_eq!(
                fs::read_dir(home).unwrap().count(),
                0,
                "rejected startup opened operational owner"
            );
            assert!(!root.join("hm.sock").exists());
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("invalid startup remained alive")
}
#[tokio::test]
async fn actual_trusted_startup_uds_replay_restart_and_refusal() {
    let root = PathBuf::from(
        std::env::var_os("HM_FABRIC_JOURNEY_ROOT").expect("private isolated journey root required"),
    );
    fs::create_dir_all(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let home = root.join("backend");
    fs::create_dir(&home).unwrap();
    private(&root.join("server.conf"),format!("socket={}\ndata={}\nuser={}\nkek={}\nadmin_token={}\nactor=7:{}\nprojection_map_bytes=16777216\n",root.join("hm.sock").display(),root.join("data").display(),"01".repeat(16),"02".repeat(32),"03".repeat(32),"04".repeat(32)).as_bytes());
    private(
        &root.join("context.json"),
        &serde_json::to_vec(&json!({"version":1,"actor":7,"scope":scope()})).unwrap(),
    );
    let good = startup(&home);
    let config = root.join("fabric.json");
    private(&config, &serde_json::to_vec(&good).unwrap());
    let bad = root.join("bad.json");
    let mut wrong = good.clone();
    wrong["backend"]["scope"]["owner_id"] = json!("foreign");
    private(&bad, &serde_json::to_vec(&wrong).unwrap());
    refused(&root, &bad, &home).await;
    wrong = good.clone();
    wrong["actor"] = json!(8);
    private(&bad, &serde_json::to_vec(&wrong).unwrap());
    refused(&root, &bad, &home).await;
    private(&bad, &serde_json::to_vec(&good).unwrap());
    fs::set_permissions(&bad, fs::Permissions::from_mode(0o644)).unwrap();
    refused(&root, &bad, &home).await;
    fs::set_permissions(&bad, fs::Permissions::from_mode(0o600)).unwrap();
    let link = root.join("linked.json");
    symlink(&config, &link).unwrap();
    refused(&root, &link, &home).await;
    assert!(
        Command::new("/bin/chown")
            .args(["65534"])
            .arg(&bad)
            .status()
            .unwrap()
            .success()
    );
    refused(&root, &bad, &home).await;
    let mut daemon = start(&root, &config);
    let mut stream = socket(&root, &mut daemon).await;
    hello(&mut stream).await;
    let workers = children(daemon.0.id());
    assert_eq!(workers.len(), 1, "expected one actual worker child");
    let input = json!({"conversation":"fabric","content":"","kind":"user","context":{"operation":"fabric","request":{"version":1,"scope":scope(),"request_id":"daemon-digest","action":{"action":"dispatch","operation":"digest","bytes":b"actual daemon bytes".to_vec(),"timeout_ms":10000}}}});
    let first = tool(&mut stream, 1, "remember", input.clone()).await;
    assert_eq!(first["ok"], true);
    assert_eq!(
        first["items"][0]["result"]["digest"],
        format!("{:x}", Sha256::digest(b"actual daemon bytes"))
    );
    let repeat = tool(&mut stream, 2, "remember", input.clone()).await;
    assert_eq!(first["items"], repeat["items"]);
    let inspect = tool(
        &mut stream,
        3,
        "inspect",
        json!({"uri":"hm://7/context-fabric"}),
    )
    .await;
    assert_eq!(inspect["ok"], true);
    assert_eq!(
        inspect["items"][0]["descriptors"]["effect_authority"],
        "selected_operational"
    );
    drop(stream);
    stop(&mut daemon, workers[0]).await;
    drop(daemon);
    let mut daemon = start(&root, &config);
    let mut stream = socket(&root, &mut daemon).await;
    hello(&mut stream).await;
    let workers = children(daemon.0.id());
    assert_eq!(workers.len(), 1);
    let replay = tool(&mut stream, 4, "remember", input).await;
    assert_eq!(first["items"], replay["items"]);
    drop(stream);
    stop(&mut daemon, workers[0]).await;
}
