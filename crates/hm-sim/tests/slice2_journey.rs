#![forbid(unsafe_code)]

use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};
use tokio::net::UnixStream;

type JourneyResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

const TYPESCRIPT_JOURNEY: &str = r#"
const fs = require("node:fs");
const { createHash } = require("node:crypto");
const flatbuffers = require("flatbuffers");
const { Client, PendingWriteError } = require("./packages/client/dist/index.js");
const { EventEnvelope } = require("./packages/client/dist/wire/hypermind/schema/event-envelope.js");
const { EventPayload } = require("./packages/client/dist/wire/hypermind/schema/event-payload.js");
const { ToolCallT } = require("./packages/client/dist/wire/hypermind/schema/tool-call.js");
const { EffectT } = require("./packages/client/dist/wire/hypermind/schema/effect.js");
const { EffectState } = require("./packages/client/dist/wire/hypermind/schema/effect-state.js");
const { Authority } = require("./packages/client/dist/wire/hypermind/schema/authority.js");
const { Retention } = require("./packages/client/dist/wire/hypermind/schema/retention.js");
const { Sensitivity } = require("./packages/client/dist/wire/hypermind/schema/sensitivity.js");

const [socketPath, readyPath, continuePath, resultPath] = process.argv.slice(1);
const text = new TextEncoder();

function envelope(payloadType, payload, authority) {
  const builder = new flatbuffers.Builder(512);
  const payloadOffset = payload.pack(builder);
  EventEnvelope.startEventEnvelope(builder);
  EventEnvelope.addSchemaVersion(builder, 2);
  EventEnvelope.addPayloadType(builder, payloadType);
  EventEnvelope.addPayload(builder, payloadOffset);
  EventEnvelope.addClientEventCount(builder, 1);
  EventEnvelope.addAuthority(builder, authority);
  EventEnvelope.addRetention(builder, Retention.durable);
  EventEnvelope.addSensitivity(builder, Sensitivity.personal);
  const offset = EventEnvelope.endEventEnvelope(builder);
  builder.finish(offset, "NCEV");
  return builder.asUint8Array();
}

async function waitFor(path) {
  for (let attempt = 0; attempt < 1_000; attempt += 1) {
    if (fs.existsSync(path)) return;
    await new Promise((resolve) => setTimeout(resolve, 10));
  }
  throw new Error(`timed out waiting for ${path}`);
}

async function main() {
  const client = await Client.connect({
    socketPath,
    capabilityToken: Buffer.from("44".repeat(32), "hex"),
    connectionId: Uint8Array.from({ length: 16 }, () => 0x72),
    requestTimeoutMs: 2_000,
  });
  const session = client.session("slice2-continuity");
  const conversation = createHash("sha256")
    .update("neocortex-conversation-v1\0")
    .update("slice2-continuity")
    .digest()
    .subarray(0, 16);
  const evidenceLsn = await session.remember("src/state.rs is pinned at revision rev-42");
  await session.intend("open_loop", "slice2-task", "finish the exact file update");
  const binding = await session.bind({
    task: "slice2-task",
    canonicalEntity: "src/state.rs",
    property: "revision",
    evidenceLsn,
    revision: "rev-42",
    freshnessRequirementNs: 60_000_000_000n,
  });
  const toolCall = new ToolCallT(
    [...text.encode("slice2-call")],
    "write_file",
    [...text.encode("path=src/state.rs revision=rev-42")],
  );
  const toolCallLsn = await client.append(
    3,
    Uint8Array.from(conversation),
    envelope(EventPayload.ToolCall, toolCall, Authority.assistant_generated),
  );
  fs.writeFileSync(readyPath, toolCallLsn.toString());
  await waitFor(continuePath);

  const effect = new EffectT(
    [...text.encode("slice2-effect")],
    toolCallLsn,
    EffectState.outcome_unknown,
  );
  let effectPending = false;
  try {
    await client.append(
      8,
      Uint8Array.from(conversation),
      envelope(EventPayload.Effect, effect, Authority.runtime_fact),
    );
  } catch (error) {
    if (!(error instanceof PendingWriteError)) throw error;
    effectPending = true;
  }

  const bundle = await session.activate("revision rev-42", 4096);
  const recalled = await session.recall("revision rev-42", 10);
  const section = (tier) => bundle.sections.find((candidate) => candidate.tier === tier);
  fs.writeFileSync(resultPath, JSON.stringify({
    evidenceLsn: evidenceLsn.toString(),
    toolCallLsn: toolCallLsn.toString(),
    effectPending,
    pendingLength: client.pendingLength,
    nextClientSeq: client.welcome.nextClientSeq.toString(),
    snapshotEpoch: bundle.snapshotEpoch.toString(),
    intent: section("intent").items.map((item) => item.content),
    bindings: section("bindings").items.map((item) => item.content),
    work: section("work_ledger").items.map((item) => item.content),
    gaps: bundle.gaps,
    recalled: recalled.map((lsn) => lsn.toString()),
    bindingStatus: binding.status,
  }));
  client.close();
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
"#;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn slice2_journey() -> JourneyResult<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    build_typescript(&root)?;
    build_daemon(&root)?;

    let temporary = tempfile::tempdir()?;
    let socket = temporary.path().join("hypermind.sock");
    let config = write_config(temporary.path(), &socket)?;
    let executable = root.join("target/debug/hm");
    let mut daemon = Daemon::start(&executable, &config, &socket).await?;

    let ready = temporary.path().join("typescript.ready");
    let resume = temporary.path().join("typescript.resume");
    let result = temporary.path().join("typescript.result.json");
    let typescript = root.join("sdk/typescript");
    let mut client = Command::new("node")
        .args([
            "-e",
            TYPESCRIPT_JOURNEY,
            &socket.to_string_lossy(),
            &ready.to_string_lossy(),
            &resume.to_string_lossy(),
            &result.to_string_lossy(),
        ])
        .current_dir(&typescript)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    wait_for_file(&ready, &mut client, &mut daemon, Duration::from_secs(20)).await?;
    if fs::read_to_string(&ready)?.trim() != "4" {
        return Err("TypeScript client did not reach the dispatched ToolCall at LSN 4".into());
    }
    daemon.kill()?;

    let mut restarted = Daemon::start(&executable, &config, &socket).await?;
    fs::write(&resume, b"resume")?;
    wait_for_file(
        &result,
        &mut client,
        &mut restarted,
        Duration::from_secs(20),
    )
    .await?;
    let output = client.wait_with_output()?;
    require_success(&output, "TypeScript continuity journey")?;
    let report: Value = serde_json::from_slice(&fs::read(&result)?)?;
    assert_report(&report)?;
    restarted.kill()?;

    let gate = Command::new("cargo")
        .args(["run", "-p", "hm-eval", "--", "gate", "slice2"])
        .current_dir(&root)
        .output()?;
    require_success(&gate, "slice2 evaluation gate")?;
    Ok(())
}

fn build_typescript(root: &Path) -> JourneyResult<()> {
    for workspace in ["@hypermind/render", "@hypermind/client"] {
        let output = Command::new("npm")
            .args([
                "--prefix",
                "sdk/typescript",
                "run",
                "build",
                "-w",
                workspace,
            ])
            .current_dir(root)
            .output()?;
        require_success(&output, &format!("build {workspace}"))?;
    }
    Ok(())
}

fn build_daemon(root: &Path) -> JourneyResult<()> {
    let output = Command::new("cargo")
        .args(["build", "-p", "hm-cli"])
        .current_dir(root)
        .output()?;
    require_success(&output, "build hm daemon")
}

fn write_config(directory: &Path, socket: &Path) -> JourneyResult<PathBuf> {
    let data = directory.join("data");
    fs::create_dir_all(&data)?;
    let config = directory.join("hypermind.conf");
    let contents = format!(
        "socket={}\ndata={}\nuser={}\nkek={}\nadmin_token={}\nactor=7:{}\nprojection_map_bytes={}\n",
        socket.display(),
        data.display(),
        "11".repeat(16),
        "22".repeat(32),
        "33".repeat(32),
        "44".repeat(32),
        64 * 1024 * 1024,
    );
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&config)?;
    file.write_all(contents.as_bytes())?;
    file.sync_all()?;
    Ok(config)
}

async fn wait_for_file(
    path: &Path,
    client: &mut Child,
    daemon: &mut Daemon,
    timeout: Duration,
) -> JourneyResult<()> {
    let deadline = Instant::now() + timeout;
    loop {
        if path.exists() {
            return Ok(());
        }
        if let Some(status) = client.try_wait()? {
            return Err(format!("TypeScript client exited early with {status}").into());
        }
        if let Some(status) = daemon.child.try_wait()? {
            return Err(format!("daemon exited early with {status}").into());
        }
        if Instant::now() >= deadline {
            return Err(format!("timed out waiting for {}", path.display()).into());
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

fn assert_report(report: &Value) -> JourneyResult<()> {
    require_string(report, "evidenceLsn", "1")?;
    require_string(report, "toolCallLsn", "4")?;
    require_string(report, "nextClientSeq", "6")?;
    if report
        .get("snapshotEpoch")
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<u64>().ok())
        .is_none_or(|value| value == 0)
    {
        return Err("activation did not carry a valid snapshot epoch".into());
    }
    if report.get("effectPending") != Some(&Value::Bool(true))
        || report.get("pendingLength").and_then(Value::as_u64) != Some(0)
        || report.get("bindingStatus").and_then(Value::as_str) != Some("resolved")
    {
        return Err("TypeScript sequence recovery did not drain exactly one pending effect".into());
    }
    let intent = strings(report, "intent")?;
    if !intent
        .iter()
        .any(|value| value.contains("open_loop") && value.contains("finish the exact file update"))
    {
        return Err("activation omitted the open loop".into());
    }
    let bindings = strings(report, "bindings")?;
    if !bindings.iter().any(|value| {
        value.contains("src/state.rs")
            && value.contains("revision=")
            && value.contains("status=resolved")
    }) {
        return Err("activation omitted the resolved file binding".into());
    }
    let work = strings(report, "work")?;
    if work.len() != 2
        || !work
            .iter()
            .any(|value| value.contains("tool_call") && value.contains("state=dispatched"))
        || !work
            .iter()
            .any(|value| value.contains("effect") && value.contains("state=outcome_unknown"))
    {
        return Err("activation omitted the unreconciled dispatch window".into());
    }
    if report
        .get("recalled")
        .and_then(Value::as_array)
        .is_none_or(|values| !values.iter().any(|value| value.as_str() == Some("1")))
    {
        return Err("post-restart recall omitted the indexed evidence".into());
    }
    if report
        .get("gaps")
        .and_then(Value::as_array)
        .is_none_or(|gaps| !gaps.is_empty())
    {
        return Err("fully budgeted activation unexpectedly degraded".into());
    }
    Ok(())
}

fn require_string(report: &Value, field: &str, expected: &str) -> JourneyResult<()> {
    if report.get(field).and_then(Value::as_str) == Some(expected) {
        Ok(())
    } else {
        Err(format!("report field {field} was not {expected}").into())
    }
}

fn strings<'value>(report: &'value Value, field: &str) -> JourneyResult<Vec<&'value str>> {
    report
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("report field {field} was not an array"))?
        .iter()
        .map(|value| {
            value
                .as_str()
                .ok_or_else(|| format!("report field {field} contained a non-string").into())
        })
        .collect()
}

fn require_success(output: &Output, operation: &str) -> JourneyResult<()> {
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{operation} failed with {}\n{}{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .into())
    }
}

struct Daemon {
    child: Child,
}

impl Daemon {
    async fn start(executable: &Path, config: &Path, socket: &Path) -> JourneyResult<Self> {
        let child = Command::new(executable)
            .args(["serve", "--config"])
            .arg(config)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        let mut daemon = Self { child };
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if UnixStream::connect(socket).await.is_ok() {
                return Ok(daemon);
            }
            if let Some(status) = daemon.child.try_wait()? {
                return Err(format!("daemon exited during startup with {status}").into());
            }
            if Instant::now() >= deadline {
                return Err("daemon did not accept Unix connections".into());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    fn kill(&mut self) -> JourneyResult<()> {
        self.child.kill()?;
        let status = self.child.wait()?;
        if status.success() {
            Err("daemon unexpectedly exited successfully after SIGKILL".into())
        } else {
            Ok(())
        }
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
