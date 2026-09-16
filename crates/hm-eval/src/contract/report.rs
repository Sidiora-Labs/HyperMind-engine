#![allow(clippy::cast_precision_loss, clippy::missing_errors_doc)]

use crate::bench::execution::repository_root;
use crate::bench::gateway::{DynError, write_json};
use crate::contract::daemon::ContractDaemon;
use crate::contract::{
    CONTRACT_CONVERSATION, ContractComparison, ContractObservation, REFERENCE_SDK, Unavailable,
    Unsupported, compare, python, reference_leg, scenario, typescript,
};
use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};

pub const REPORT_FORMAT: &str = "hypermind.cross-sdk-contract-report.v1";
pub const RESULT_PATH: &str = "eval/results/cross-sdk-contract.json";
pub const GO_SDK: &str = "go";
pub const MIGRATION_STEP: &str = "migrate-import";
pub const MIGRATION_SURFACES: &[&str] = &[typescript::SDK];
pub const MINIMUM_PARTICIPANTS: usize = 2;

const KNOWN_SDKS: [&str; 4] = [REFERENCE_SDK, typescript::SDK, python::SDK, GO_SDK];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContractReport {
    pub format: String,
    pub comparison: ContractComparison,
    pub scenario_steps: usize,
    pub restarted: bool,
}

#[must_use]
pub fn exposes_migration_surface(sdk: &str) -> bool {
    MIGRATION_SURFACES.contains(&sdk)
}

#[must_use]
pub fn migration_unsupported() -> Vec<Unsupported> {
    KNOWN_SDKS
        .iter()
        .copied()
        .filter(|sdk| !exposes_migration_surface(sdk))
        .map(|sdk| Unsupported {
            sdk: sdk.to_owned(),
            step: MIGRATION_STEP.to_owned(),
        })
        .collect()
}

#[must_use]
pub fn go_availability() -> Unavailable {
    let toolchain = match Command::new("go")
        .arg("version")
        .stdin(Stdio::null())
        .output()
    {
        Ok(output) if output.status.success() => format!(
            "the go toolchain reports {}",
            String::from_utf8_lossy(&output.stdout).trim()
        ),
        Ok(output) => format!("`go version` exited with {}", output.status),
        Err(error) => format!("the go toolchain is not on PATH: {error}"),
    };
    Unavailable {
        sdk: GO_SDK.to_owned(),
        reason: format!(
            "{toolchain}; no go contract leg exists, so the go sdk is declared unavailable by inspection and is never reported as agreeing"
        ),
    }
}

pub fn certify(comparison: &ContractComparison) -> Result<(), DynError> {
    if comparison.participants.len() < MINIMUM_PARTICIPANTS {
        return Err(format!(
            "the cross-sdk contract needs {MINIMUM_PARTICIPANTS} participating legs but only {:?} participated; unavailable: {}",
            comparison.participants,
            describe(&comparison.unavailable)
        )
        .into());
    }
    if let Some(divergence) = comparison.divergences.first() {
        return Err(format!(
            "the participating legs {:?} diverged at step {} on {}: {:?}",
            comparison.participants, divergence.step, divergence.field, divergence.values
        )
        .into());
    }
    Ok(())
}

fn describe(unavailable: &[Unavailable]) -> String {
    if unavailable.is_empty() {
        return "none".to_owned();
    }
    unavailable
        .iter()
        .map(|entry| format!("{} ({})", entry.sdk, entry.reason))
        .collect::<Vec<_>>()
        .join("; ")
}

pub async fn run() -> Result<ContractReport, DynError> {
    let conversation = CONTRACT_CONVERSATION;
    let mut observations: Vec<ContractObservation> = Vec::new();
    let mut unavailable = vec![go_availability()];
    let mut restarted = false;

    let reference_root = tempfile::tempdir()?;
    observations.push(reference_leg(reference_root.path(), conversation).await?);

    let typescript_ready = typescript::ensure_built()
        .map_err(|error| Unavailable {
            sdk: typescript::SDK.to_owned(),
            reason: error.to_string(),
        })
        .and_then(|()| typescript::availability());
    match typescript_ready {
        Ok(()) => {
            let root = tempfile::tempdir()?;
            let mut daemon = ContractDaemon::start(root.path())?;
            let socket = daemon.socket().to_path_buf();
            let observed = typescript::observation(&socket, conversation, root.path(), &mut daemon);
            let stopped = daemon.stop();
            observations.push(observed?);
            stopped?;
            restarted = true;
        }
        Err(entry) => unavailable.push(entry),
    }

    match python::availability() {
        Ok(()) => {
            let root = tempfile::tempdir()?;
            observations.push(python::observation(root.path(), conversation)?);
            restarted = true;
        }
        Err(entry) => unavailable.push(entry),
    }

    let report = ContractReport {
        format: REPORT_FORMAT.to_owned(),
        comparison: compare(&observations, unavailable, migration_unsupported()),
        scenario_steps: scenario(conversation).len(),
        restarted,
    };
    write_json(&repository_root().join(RESULT_PATH), &report)?;
    certify(&report.comparison)?;
    Ok(report)
}
