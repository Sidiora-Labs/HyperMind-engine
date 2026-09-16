#![allow(clippy::missing_errors_doc)]

use crate::keyring::{EntropySource, KeyHierarchy, UserId, io_error, sync_directory};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use hm_core::{ActorId, Error, ErrorCode};
use std::fs::{self, DirBuilder, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

pub const CREDENTIAL_MAGIC: &[u8; 8] = b"HMCRED01";
pub const MAXIMUM_CREDENTIAL_BYTES: usize = 4096;

const NONCE_BYTES: usize = 24;
const TAG_BYTES: usize = 16;
const HEADER_BYTES: usize = CREDENTIAL_MAGIC.len() + 4 + NONCE_BYTES;
const CREDENTIAL_EXTENSION: &str = ".cred";
const PENDING_EXTENSION: &str = ".cred.pending";
const CREDENTIAL_DOMAIN: &[u8] = b"hypermind.connector-credential.v1";
const CONSENT_KEY_CONTEXT: &str = "hypermind.source-consent-key.v1";
const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

pub struct CredentialVault {
    directory: PathBuf,
}

impl CredentialVault {
    pub fn open(actor_directory: &Path) -> Result<Self, Error> {
        let directory = actor_directory.join("connectors");
        DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&directory)
            .map_err(|error| io_error(ErrorCode::OpenFailed, &error))?;
        Ok(Self { directory })
    }

    pub fn store(
        &self,
        keys: &KeyHierarchy,
        provider: &str,
        connector_id: &[u8; 16],
        version: u32,
        secret: &[u8],
        entropy: &mut impl EntropySource,
    ) -> Result<(), Error> {
        if version == 0 || provider.is_empty() || secret.is_empty() {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        if secret.len() > MAXIMUM_CREDENTIAL_BYTES {
            return Err(Error::new(ErrorCode::CapacityExceeded));
        }
        let connector_directory = self.connector_directory(connector_id);
        DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&connector_directory)
            .map_err(|error| io_error(ErrorCode::OpenFailed, &error))?;
        let final_path = connector_directory.join(format!("{version:010}{CREDENTIAL_EXTENSION}"));
        if final_path.exists() {
            return Err(Error::new(ErrorCode::AlreadyExists));
        }

        let mut nonce = [0_u8; NONCE_BYTES];
        entropy.fill(&mut nonce)?;
        let aad = credential_aad(keys.actor(), keys.user(), version, provider, connector_id);
        let ciphertext = XChaCha20Poly1305::new(Key::from_slice(keys.data_key.as_ref()))
            .encrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: secret,
                    aad: &aad,
                },
            )
            .map_err(|_| Error::new(ErrorCode::CryptoAuthentication))?;

        let mut bytes = Vec::with_capacity(HEADER_BYTES + ciphertext.len());
        bytes.extend_from_slice(CREDENTIAL_MAGIC);
        bytes.extend_from_slice(&version.to_le_bytes());
        bytes.extend_from_slice(&nonce);
        bytes.extend_from_slice(&ciphertext);

        let pending_path = connector_directory.join(format!("{version:010}{PENDING_EXTENSION}"));
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&pending_path)
            .map_err(|error| io_error(ErrorCode::OpenFailed, &error))?;
        file.write_all(&bytes)
            .map_err(|error| io_error(ErrorCode::WriteFailed, &error))?;
        file.sync_data()
            .map_err(|error| io_error(ErrorCode::SyncFailed, &error))?;
        drop(file);
        fs::rename(&pending_path, &final_path)
            .map_err(|error| io_error(ErrorCode::WriteFailed, &error))?;
        sync_directory(&connector_directory)
    }

    pub fn secret(
        &self,
        keys: &KeyHierarchy,
        provider: &str,
        connector_id: &[u8; 16],
        version: u32,
    ) -> Result<Zeroizing<Vec<u8>>, Error> {
        if version == 0 || provider.is_empty() {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let path = self
            .connector_directory(connector_id)
            .join(format!("{version:010}{CREDENTIAL_EXTENSION}"));
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(Error::new(ErrorCode::InvalidArgument));
            }
            Err(error) => return Err(io_error(ErrorCode::ReadFailed, &error)),
        };
        if bytes.len() < HEADER_BYTES + TAG_BYTES
            || bytes.get(..CREDENTIAL_MAGIC.len()) != Some(CREDENTIAL_MAGIC.as_slice())
        {
            return Err(Error::new(ErrorCode::LegacyPlaintext));
        }
        let stored_version = bytes
            .get(CREDENTIAL_MAGIC.len()..CREDENTIAL_MAGIC.len() + 4)
            .and_then(|slice| <[u8; 4]>::try_from(slice).ok())
            .map(u32::from_le_bytes)
            .ok_or_else(|| Error::new(ErrorCode::CryptoAuthentication))?;
        if stored_version != version {
            return Err(Error::new(ErrorCode::CryptoAuthentication));
        }
        let nonce = bytes
            .get(CREDENTIAL_MAGIC.len() + 4..HEADER_BYTES)
            .and_then(|slice| <[u8; NONCE_BYTES]>::try_from(slice).ok())
            .ok_or_else(|| Error::new(ErrorCode::CryptoAuthentication))?;
        let ciphertext = bytes
            .get(HEADER_BYTES..)
            .ok_or_else(|| Error::new(ErrorCode::CryptoAuthentication))?;
        let aad = credential_aad(keys.actor(), keys.user(), version, provider, connector_id);
        let plaintext = XChaCha20Poly1305::new(Key::from_slice(keys.data_key.as_ref()))
            .decrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: ciphertext,
                    aad: &aad,
                },
            )
            .map_err(|_| Error::new(ErrorCode::CryptoAuthentication))?;
        Ok(Zeroizing::new(plaintext))
    }

    pub fn versions(&self, connector_id: &[u8; 16]) -> Result<Vec<u32>, Error> {
        let connector_directory = self.connector_directory(connector_id);
        let entries = match fs::read_dir(&connector_directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(io_error(ErrorCode::ReadFailed, &error)),
        };
        let mut versions = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| io_error(ErrorCode::ReadFailed, &error))?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            let Some(stem) = name.strip_suffix(CREDENTIAL_EXTENSION) else {
                continue;
            };
            if stem.len() != 10 || !stem.bytes().all(|byte| byte.is_ascii_digit()) {
                continue;
            }
            let Ok(version) = stem.parse::<u32>() else {
                continue;
            };
            if version != 0 {
                versions.push(version);
            }
        }
        versions.sort_unstable();
        Ok(versions)
    }

    pub fn active_version(&self, connector_id: &[u8; 16]) -> Result<u32, Error> {
        self.versions(connector_id)?
            .last()
            .copied()
            .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))
    }

    fn connector_directory(&self, connector_id: &[u8; 16]) -> PathBuf {
        self.directory.join(hex_name(connector_id))
    }
}

#[must_use]
pub fn consent_key(keys: &KeyHierarchy) -> Zeroizing<[u8; 32]> {
    let root = Zeroizing::new(blake3::derive_key(
        CONSENT_KEY_CONTEXT,
        keys.data_key.as_ref(),
    ));
    let mut identity = Vec::with_capacity(2 + 16);
    identity.extend_from_slice(&keys.actor().get().to_le_bytes());
    identity.extend_from_slice(keys.user());
    Zeroizing::new(*blake3::keyed_hash(&root, &identity).as_bytes())
}

fn credential_aad(
    actor: ActorId,
    user: &UserId,
    version: u32,
    provider: &str,
    connector_id: &[u8; 16],
) -> Vec<u8> {
    let mut aad = Vec::with_capacity(CREDENTIAL_DOMAIN.len() + 26 + provider.len());
    aad.extend_from_slice(CREDENTIAL_DOMAIN);
    aad.push(0);
    aad.extend_from_slice(&actor.get().to_le_bytes());
    aad.extend_from_slice(user);
    aad.extend_from_slice(&version.to_le_bytes());
    aad.extend_from_slice(provider.as_bytes());
    aad.push(0);
    aad.extend_from_slice(connector_id);
    aad
}

fn hex_name(connector_id: &[u8; 16]) -> String {
    let mut name = String::with_capacity(connector_id.len() * 2);
    for byte in connector_id {
        name.push(char::from(HEX_DIGITS[usize::from(byte >> 4)]));
        name.push(char::from(HEX_DIGITS[usize::from(byte & 0x0f)]));
    }
    name
}
