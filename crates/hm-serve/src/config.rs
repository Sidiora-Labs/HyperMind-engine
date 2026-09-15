#![allow(clippy::missing_errors_doc)]

use hm_core::{Error, ErrorCode};
use hm_ledger::keyring::{KeyEncryptionKey, UserId};
use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use subtle::ConstantTimeEq;

pub type CapabilityToken = [u8; 32];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActorCapability {
    pub actor: u16,
    pub token: CapabilityToken,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServerConfig {
    pub socket_path: PathBuf,
    pub data_directory: PathBuf,
    pub user: UserId,
    pub kek: KeyEncryptionKey,
    pub admin_token: CapabilityToken,
    pub actors: Vec<ActorCapability>,
    pub maximum_connections: usize,
    pub maximum_output_frames: usize,
    pub maximum_output_bytes: usize,
    pub projection_map_bytes: usize,
}

impl ServerConfig {
    #[must_use]
    pub fn actor_directory(&self, actor: u16) -> PathBuf {
        self.data_directory.join(actor.to_string())
    }
}

pub fn load(path: impl AsRef<Path>) -> Result<ServerConfig, Error> {
    let path = path.as_ref();
    let metadata = fs::metadata(path).map_err(|_| Error::new(ErrorCode::OpenFailed))?;
    if !metadata.is_file() || metadata.mode() & 0o077 != 0 {
        return Err(Error::new(ErrorCode::CapabilityDenied));
    }
    let text = fs::read_to_string(path).map_err(|_| Error::new(ErrorCode::ReadFailed))?;
    let mut socket_path = None;
    let mut data_directory = None;
    let mut user = None;
    let mut kek = None;
    let mut admin_token = None;
    let mut actors = Vec::new();
    let mut maximum_connections = 128;
    let mut maximum_output_frames = 256;
    let mut maximum_output_bytes = 64 * 1024 * 1024;
    let mut projection_map_bytes = 256 * 1024 * 1024;
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .filter(|(key, value)| !key.is_empty() && !value.is_empty())
            .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
        match key {
            "socket" => socket_path = Some(PathBuf::from(value)),
            "data" => data_directory = Some(PathBuf::from(value)),
            "user" => user = Some(decode_hex(value)?),
            "kek" => kek = Some(decode_hex(value)?),
            "admin_token" => admin_token = Some(decode_hex(value)?),
            "actor" => {
                let (actor, token) = value
                    .split_once(':')
                    .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
                let actor = actor
                    .parse::<u16>()
                    .ok()
                    .filter(|actor| *actor != 0)
                    .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
                actors.push(ActorCapability {
                    actor,
                    token: decode_hex(token)?,
                });
            }
            "maximum_connections" => maximum_connections = parse_size(value, 1, 4096)?,
            "maximum_output_frames" => maximum_output_frames = parse_size(value, 1, 4096)?,
            "maximum_output_bytes" => {
                maximum_output_bytes = parse_size(value, 1024 * 1024, 1024 * 1024 * 1024)?;
            }
            "projection_map_bytes" => {
                projection_map_bytes = parse_size(value, 1024 * 1024, usize::MAX)?;
            }
            _ => return Err(Error::new(ErrorCode::InvalidArgument)),
        }
    }
    actors.sort_unstable_by_key(|actor| actor.actor);
    let admin_token = admin_token.ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
    if actors.is_empty()
        || admin_token == [0; 32]
        || actors.iter().any(|actor| actor.token == [0; 32])
    {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let mut actor_ids = BTreeSet::new();
    let mut tokens = BTreeSet::new();
    for actor in &actors {
        if !actor_ids.insert(actor.actor)
            || !tokens.insert(actor.token)
            || capability_equal(&admin_token, &actor.token)
        {
            return Err(Error::new(ErrorCode::AlreadyExists));
        }
    }
    Ok(ServerConfig {
        socket_path: socket_path.ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?,
        data_directory: data_directory.ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?,
        user: user.ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?,
        kek: kek.ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?,
        admin_token,
        actors,
        maximum_connections,
        maximum_output_frames,
        maximum_output_bytes,
        projection_map_bytes,
    })
}

#[must_use]
pub fn capability_equal(left: &CapabilityToken, right: &[u8]) -> bool {
    right.len() == left.len() && bool::from(left.ct_eq(right))
}

fn parse_size(value: &str, minimum: usize, maximum: usize) -> Result<usize, Error> {
    value
        .parse::<usize>()
        .ok()
        .filter(|parsed| (*parsed >= minimum) && (*parsed <= maximum))
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))
}

fn decode_hex<const N: usize>(encoded: &str) -> Result<[u8; N], Error> {
    if encoded.len() != N * 2 {
        return Err(Error::new(ErrorCode::InvalidLength));
    }
    let mut output = [0; N];
    for (index, byte) in output.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&encoded[index * 2..index * 2 + 2], 16)
            .map_err(|_| Error::new(ErrorCode::InvalidArgument))?;
    }
    Ok(output)
}
