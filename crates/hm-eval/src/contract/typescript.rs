#![allow(clippy::cast_precision_loss, clippy::missing_errors_doc)]

use crate::bench::execution::repository_root;
use crate::bench::gateway::DynError;
use crate::contract::daemon::{self, ContractDaemon};
use crate::contract::{
    ContractObservation, Phase, RawDocument, Unavailable, observe_raw, write_scenario,
};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Mutex;

pub const SDK: &str = "typescript";
pub const TRANSPORT: &str = "unix";
pub const CLIENT_BUNDLE: &str = "sdk/typescript/packages/client/dist/index.js";

const SDK_DIRECTORY: &str = "sdk/typescript";
const LEG_SCRIPT: &str = "eval/contract/typescript-leg.mjs";
const SCENARIO_FILE: &str = "scenario.v1.json";

static BUILD: Mutex<()> = Mutex::new(());

pub fn availability() -> Result<(), Unavailable> {
    availability_in(&repository_root())
}

pub fn availability_in(root: &Path) -> Result<(), Unavailable> {
    let unavailable = |reason: String| Unavailable {
        sdk: SDK.to_owned(),
        reason,
    };
    match Command::new("node").arg("--version").output() {
        Ok(output) if output.status.success() => {}
        Ok(output) => {
            return Err(unavailable(format!(
                "node --version exited with {}",
                output.status
            )));
        }
        Err(error) => return Err(unavailable(format!("node is not on PATH: {error}"))),
    }
    let bundle = root.join(CLIENT_BUNDLE);
    if !bundle.is_file() {
        return Err(unavailable(format!(
            "the compiled client is missing at {}; build it with `npm --prefix {SDK_DIRECTORY} run build -w @hypermind/render -w @hypermind/client`",
            bundle.display()
        )));
    }
    let script = root.join(LEG_SCRIPT);
    if !script.is_file() {
        return Err(unavailable(format!(
            "the leg script is missing at {}",
            script.display()
        )));
    }
    daemon::binary().map_err(|error| unavailable(error.to_string()))?;
    Ok(())
}

pub fn ensure_built() -> Result<(), DynError> {
    let root = repository_root();
    let bundle = root.join(CLIENT_BUNDLE);
    let _serialized = BUILD
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if bundle.is_file() {
        return Ok(());
    }
    let output = Command::new("npm")
        .arg("--prefix")
        .arg(root.join(SDK_DIRECTORY))
        .args([
            "run",
            "build",
            "-w",
            "@hypermind/render",
            "-w",
            "@hypermind/client",
        ])
        .stdin(Stdio::null())
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "the typescript client workspace exited with {} while building: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    if !bundle.is_file() {
        return Err(format!(
            "{} is still absent after the client workspace build",
            bundle.display()
        )
        .into());
    }
    Ok(())
}

pub fn run_leg(
    socket: &Path,
    conversation: &str,
    phase: Phase,
    out: &Path,
) -> Result<Vec<u8>, DynError> {
    let root = repository_root();
    let directory = out.parent().ok_or("the leg output path has no parent")?;
    let scenario = directory.join(SCENARIO_FILE);
    write_scenario(&scenario, conversation)?;
    let phase_name = serde_json::to_value(phase)?
        .as_str()
        .ok_or("the scenario phase is not a string")?
        .to_owned();
    let mut command = Command::new("node");
    command
        .arg(root.join(LEG_SCRIPT))
        .arg("--client")
        .arg(root.join(CLIENT_BUNDLE))
        .arg("--socket")
        .arg(socket)
        .arg("--token")
        .arg(ContractDaemon::token_hex())
        .arg("--scenario")
        .arg(&scenario)
        .arg("--phase")
        .arg(&phase_name)
        .arg("--out")
        .arg(out)
        .current_dir(&root)
        .stdin(Stdio::null());
    for key in daemon::runtime_variables() {
        command.env_remove(key);
    }
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "the typescript leg exited with {} during {phase_name}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(std::fs::read(out)?)
}

pub fn observation(
    socket: &Path,
    conversation: &str,
    root: &Path,
    daemon: &mut ContractDaemon,
) -> Result<ContractObservation, DynError> {
    ensure_built()?;
    let before = run_leg(
        socket,
        conversation,
        Phase::BeforeRestart,
        &root.join("typescript-before-restart.json"),
    )?;
    daemon.restart()?;
    let after = run_leg(
        socket,
        conversation,
        Phase::AfterRestart,
        &root.join("typescript-after-restart.json"),
    )?;
    let mut document: RawDocument = serde_json::from_slice(&before)?;
    let tail: RawDocument = serde_json::from_slice(&after)?;
    if document.format != tail.format
        || document.sdk != tail.sdk
        || document.transport != tail.transport
    {
        return Err("the typescript leg changed its identity across the restart".into());
    }
    if document.sdk != SDK || document.transport != TRANSPORT {
        return Err(format!(
            "the typescript leg reported sdk {} over {}",
            document.sdk, document.transport
        )
        .into());
    }
    document.calls.extend(tail.calls);
    observe_raw(&serde_json::to_vec(&document)?)
}
