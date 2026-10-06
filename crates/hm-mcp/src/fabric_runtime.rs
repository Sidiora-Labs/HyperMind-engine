use crate::dispatcher::McpToolDispatcher;
use hm_core::{Error, ErrorCode};
use hm_fabric::{
    backend_config::*,
    backend_runtime::{BackendRuntime, RUNTIME_BACKEND_MIGRATIONS},
    bus_nats::NatsCredentials,
    postgres::PostgresSecretHandle,
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
    #[serde(default)]
    postgres_secrets: BTreeMap<String, PathBuf>,
    #[serde(default)]
    nats_secrets: BTreeMap<String, PathBuf>,
}
fn invalid() -> Error {
    Error::new(ErrorCode::InvalidArgument)
}
fn read_owner_file(path: PathBuf, limit: u64) -> Result<Vec<u8>, Error> {
    let fd = rustix::fs::open(
        path,
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
        || meta.len() > limit
    {
        return Err(Error::new(ErrorCode::CapabilityDenied));
    }
    let mut bytes = Vec::new();
    (&mut file)
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| invalid())?;
    let after = file.metadata().map_err(|_| invalid())?;
    if bytes.len() as u64 > limit
        || meta.len() != after.len()
        || meta.mtime() != after.mtime()
        || meta.mtime_nsec() != after.mtime_nsec()
        || meta.ctime() != after.ctime()
        || meta.ctime_nsec() != after.ctime_nsec()
    {
        return Err(Error::new(ErrorCode::CapabilityDenied));
    }
    Ok(bytes)
}
pub async fn configure_from_env(
    dispatcher: McpToolDispatcher,
    bindings: &[TrustedContextConfig],
) -> Result<McpToolDispatcher, Error> {
    let Some(path) = std::env::var_os("HM_FABRIC_CONFIG") else {
        return Ok(dispatcher);
    };
    let bytes = read_owner_file(PathBuf::from(path), 65536)?;
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
    {
        return Err(invalid());
    }
    let nats_credential = match &config.backend.bus {
        BusBackendDescriptor::ProcessLocal { .. } if config.nats_secrets.is_empty() => None,
        BusBackendDescriptor::ProvisionedNats { secret_ref, .. }
            if config.nats_secrets.len() == 1 =>
        {
            let path = config.nats_secrets.get(secret_ref).ok_or_else(invalid)?;
            if !path.is_absolute() || std::fs::canonicalize(path).map_err(|_| invalid())? != *path {
                return Err(invalid());
            }
            let secret: NatsCredentials =
                serde_json::from_slice(&read_owner_file(path.clone(), 8192)?)
                    .map_err(|_| invalid())?;
            if secret.username.is_empty()
                || secret.username.len() > 128
                || secret.password.is_empty()
                || secret.password.len() > 4096
                || secret.username.contains(['\0', '\r', '\n'])
                || secret.password.contains(['\0', '\r', '\n'])
            {
                return Err(invalid());
            }
            Some((secret_ref.clone(), secret))
        }
        _ => return Err(invalid()),
    };
    let credential = match &config.backend.operational {
        OperationalBackendDescriptor::Sqlite if config.postgres_secrets.is_empty() => None,
        OperationalBackendDescriptor::Postgres { secret_ref, .. }
            if config.postgres_secrets.len() == 1 =>
        {
            let path = config
                .postgres_secrets
                .get(secret_ref)
                .ok_or_else(invalid)?;
            if !path.is_absolute() {
                return Err(invalid());
            }
            let bytes = read_owner_file(path.clone(), 8192)?;
            let password = String::from_utf8(bytes).map_err(|_| invalid())?;
            if password.is_empty() || password.contains(['\0', '\r', '\n']) {
                return Err(invalid());
            }
            Some((secret_ref.clone(), PostgresSecretHandle::new(password)))
        }
        _ => return Err(invalid()),
    };
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
        {
            let mut credential = credential;
            move |reference| match credential.take() {
                Some((expected, secret)) if reference == expected => Ok(secret),
                _ => Err(BackendConfigError::SecretUnavailable),
            }
        },
        {
            let mut credential = nats_credential;
            move |reference| match credential.take() {
                Some((expected, secret)) if reference == expected => Ok(secret),
                _ => Err(BackendConfigError::SecretUnavailable),
            }
        },
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
