use crate::config::ServerConfig;
use hm_context::Scope;
use hm_core::{Error, ErrorCode};
use serde::{Deserialize, Serialize};
use std::{fs::File, io::Read, os::unix::fs::MetadataExt, path::Path};

pub const CONTEXT_CONFIG_VERSION: u32 = 1;
pub const MAX_CONTEXT_CONFIG_BYTES: u64 = 16 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrustedContextConfig {
    pub version: u32,
    pub actor: u16,
    pub scope: Scope,
}

impl TrustedContextConfig {
    pub fn validate(&self, server: &ServerConfig, selected_actor: u16) -> Result<(), Error> {
        if self.version != CONTEXT_CONFIG_VERSION {
            return Err(Error::new(ErrorCode::SchemaVersion));
        }
        if self.actor == 0 || selected_actor == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        self.scope
            .validate()
            .map_err(|_| Error::new(ErrorCode::InvalidArgument))?;
        if self.actor != selected_actor
            || !server
                .actors
                .iter()
                .any(|capability| capability.actor == selected_actor)
        {
            return Err(Error::new(ErrorCode::CapabilityDenied));
        }
        Ok(())
    }
}

pub fn load(
    path: impl AsRef<Path>,
    server: &ServerConfig,
    selected_actor: u16,
) -> Result<TrustedContextConfig, Error> {
    let config = read(path.as_ref())?;
    config.validate(server, selected_actor)?;
    Ok(config)
}

pub fn load_for_server(
    path: impl AsRef<Path>,
    server: &ServerConfig,
) -> Result<TrustedContextConfig, Error> {
    let config = read(path.as_ref())?;
    config.validate(server, config.actor)?;
    Ok(config)
}

fn read(path: &Path) -> Result<TrustedContextConfig, Error> {
    let descriptor = rustix::fs::open(
        path,
        rustix::fs::OFlags::RDONLY
            | rustix::fs::OFlags::CLOEXEC
            | rustix::fs::OFlags::NOFOLLOW
            | rustix::fs::OFlags::NONBLOCK,
        rustix::fs::Mode::empty(),
    )
    .map_err(|_| Error::new(ErrorCode::OpenFailed))?;
    let mut file = File::from(descriptor);
    let metadata = file
        .metadata()
        .map_err(|_| Error::new(ErrorCode::ReadFailed))?;
    validate_file(&metadata)?;
    if metadata.len() > MAX_CONTEXT_CONFIG_BYTES {
        return Err(Error::new(ErrorCode::InvalidLength));
    }
    let mut bytes = Vec::new();
    (&mut file)
        .take(MAX_CONTEXT_CONFIG_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::new(ErrorCode::ReadFailed))?;
    if bytes.len() as u64 > MAX_CONTEXT_CONFIG_BYTES {
        return Err(Error::new(ErrorCode::InvalidLength));
    }
    let after = file
        .metadata()
        .map_err(|_| Error::new(ErrorCode::ReadFailed))?;
    validate_file(&after)?;
    if metadata.len() != after.len()
        || metadata.mtime() != after.mtime()
        || metadata.mtime_nsec() != after.mtime_nsec()
        || metadata.ctime() != after.ctime()
        || metadata.ctime_nsec() != after.ctime_nsec()
    {
        return Err(Error::new(ErrorCode::CapabilityDenied));
    }
    let config: TrustedContextConfig =
        serde_json::from_slice(&bytes).map_err(|_| Error::new(ErrorCode::InvalidArgument))?;
    Ok(config)
}

fn validate_file(metadata: &std::fs::Metadata) -> Result<(), Error> {
    if !metadata.is_file()
        || metadata.uid() != rustix::process::geteuid().as_raw()
        || !matches!(metadata.mode() & 0o7777, 0o400 | 0o600)
    {
        return Err(Error::new(ErrorCode::CapabilityDenied));
    }
    Ok(())
}
