use crate::dispatcher::McpToolDispatcher;
use hm_core::{Error, ErrorCode};
use hm_fabric::{
    backend_config::*,
    backend_runtime::{BackendRuntime, RUNTIME_BACKEND_MIGRATIONS},
    runtime::RuntimeConfig,
    supervisor::{Probe, ProcessSpec, RestartPolicy},
};
use hm_serve::{context_config::TrustedContextConfig, fabric_service::TrustedMetadata};
use serde::Deserialize;
use std::{
    collections::BTreeMap, fs::File, io::Read, os::unix::fs::MetadataExt, path::PathBuf,
    time::Duration,
};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Worker {
    module_id: String,
    command: PathBuf,
    args: Vec<String>,
    cwd: PathBuf,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Startup {
    version: u32,
    actor: u16,
    backend: BackendDescriptor,
    worker: Worker,
    client_key: [u8; 32],
    worker_key: [u8; 32],
}
fn invalid() -> Error {
    Error::new(ErrorCode::InvalidArgument)
}
pub async fn configure_from_env(
    dispatcher: McpToolDispatcher,
    bindings: &[TrustedContextConfig],
) -> Result<McpToolDispatcher, Error> {
    let Some(path) = std::env::var_os("HM_FABRIC_CONFIG") else {
        return Ok(dispatcher);
    };
    let fd = rustix::fs::open(
        PathBuf::from(path),
        rustix::fs::OFlags::RDONLY
            | rustix::fs::OFlags::NOFOLLOW
            | rustix::fs::OFlags::NONBLOCK
            | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )
    .map_err(|_| Error::new(ErrorCode::OpenFailed))?;
    let mut file = File::from(fd);
    let meta = file.metadata().map_err(|_| invalid())?;
    if !meta.is_file()
        || meta.uid() != rustix::process::geteuid().as_raw()
        || meta.mode() & 0o777 != 0o600
        || meta.len() > 65536
    {
        return Err(Error::new(ErrorCode::CapabilityDenied));
    }
    let mut bytes = Vec::new();
    (&mut file)
        .take(65537)
        .read_to_end(&mut bytes)
        .map_err(|_| invalid())?;
    let after = file.metadata().map_err(|_| invalid())?;
    if bytes.len() > 65536
        || meta.len() != after.len()
        || meta.mtime() != after.mtime()
        || meta.mtime_nsec() != after.mtime_nsec()
        || meta.ctime() != after.ctime()
        || meta.ctime_nsec() != after.ctime_nsec()
    {
        return Err(Error::new(ErrorCode::CapabilityDenied));
    }
    let config: Startup = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
    let binding = bindings
        .iter()
        .find(|binding| {
            binding.actor == config.actor
                && binding.scope == config.backend.scope
                && binding.version == 1
        })
        .ok_or_else(|| Error::new(ErrorCode::CapabilityDenied))?
        .clone();
    if config.version != 1
        || config.client_key == [0; 32]
        || config.worker_key == [0; 32]
        || config.client_key == config.worker_key
        || !matches!(
            config.backend.operational,
            OperationalBackendDescriptor::Sqlite
        )
        || !matches!(
            config.backend.bus,
            BusBackendDescriptor::ProcessLocal { .. }
        )
    {
        return Err(invalid());
    }
    hm_context::validate_id(&config.worker.module_id).map_err(|_| invalid())?;
    if !config.worker.command.is_absolute()
        || !config.worker.cwd.is_absolute()
        || config.worker.args.len() > 64
        || config.worker.args.iter().any(|arg| arg.len() > 4096)
    {
        return Err(invalid());
    }
    let descriptor = validate_descriptor(&config.backend).map_err(|_| invalid())?;
    let root = descriptor.home_path().to_path_buf();
    if std::fs::canonicalize(&config.worker.cwd).map_err(|_| invalid())? != root {
        return Err(invalid());
    }
    let selected = select_backend_async(
        descriptor,
        RUNTIME_BACKEND_MIGRATIONS,
        |_| Err(BackendConfigError::SecretUnavailable),
        |_| Err(BackendConfigError::SecretUnavailable),
    )
    .await
    .map_err(|_| Error::new(ErrorCode::OperationUnavailable))?;
    let backend = BackendRuntime::from_selected(selected)
        .map_err(|_| Error::new(ErrorCode::OperationUnavailable))?;
    let worker = ProcessSpec {
        module_id: config.worker.module_id,
        command: config.worker.command,
        args: config.worker.args,
        env: BTreeMap::new(),
        cwd: root.clone(),
        readiness: Probe::ProcessAlive,
        health: Probe::ProcessAlive,
        readiness_timeout: Duration::from_secs(3),
        shutdown_timeout: Duration::from_secs(2),
        stderr_bytes: 8192,
        drain_message: None,
        restart: RestartPolicy {
            max_restarts: 0,
            initial_backoff: Duration::ZERO,
            max_backoff: Duration::ZERO,
        },
    };
    dispatcher
        .with_fabric_startup(
            binding,
            RuntimeConfig::new(
                root,
                config.backend.scope,
                config.client_key,
                config.worker_key,
            ),
            backend,
            worker,
            TrustedMetadata::default(),
        )
        .await
}
