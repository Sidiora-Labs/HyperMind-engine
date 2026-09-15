#![allow(clippy::missing_errors_doc)]

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use hm_core::{ActorId, Error, ErrorCode};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

pub type KeyEncryptionKey = [u8; 32];
pub type UserId = [u8; 16];

pub const KEYRING_MAGIC: &[u8; 8] = b"NCKEY001";
pub const KEYRING_BYTES: usize = 176;

const KEYRING_VERSION: u32 = 1;
const NONCE_BYTES: usize = 24;
const WRAPPED_KEY_BYTES: usize = 48;
const USER_KEY_DOMAIN: &[u8] = b"neocortex.user-key.v1";
const DATA_KEY_DOMAIN: &[u8] = b"neocortex.data-key.v1";

pub trait EntropySource {
    fn fill(&mut self, destination: &mut [u8]) -> Result<(), Error>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct OsEntropy;

impl EntropySource for OsEntropy {
    fn fill(&mut self, destination: &mut [u8]) -> Result<(), Error> {
        getrandom::fill(destination).map_err(|_| Error::new(ErrorCode::ReadFailed))
    }
}

pub struct KeyHierarchy {
    pub(crate) actor: ActorId,
    pub(crate) user: UserId,
    _user_key: Zeroizing<[u8; 32]>,
    pub(crate) data_key: Zeroizing<[u8; 32]>,
    keyring_path: PathBuf,
}

impl KeyHierarchy {
    pub fn open_or_create(
        actor_directory: impl AsRef<Path>,
        actor: ActorId,
        user: UserId,
        kek: &KeyEncryptionKey,
        entropy: &mut impl EntropySource,
        allow_create: bool,
    ) -> Result<Self, Error> {
        let key_directory = actor_directory.as_ref().join("keys");
        let keyring_path = key_directory.join("KEYRING");
        if key_directory.join("DELETION_RECEIPT").exists()
            || key_directory.join("DELETION_RECEIPT.pending").exists()
        {
            return Err(Error::new(ErrorCode::KeyDestroyed));
        }
        match File::open(&keyring_path) {
            Ok(file) => Self::open_existing(file, keyring_path, actor, user, kek),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && allow_create => {
                fs::create_dir_all(&key_directory)
                    .map_err(|error| io_error(ErrorCode::OpenFailed, &error))?;
                Self::create_new(&key_directory, keyring_path, actor, user, kek, entropy)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Err(Error::new(ErrorCode::LegacyPlaintext))
            }
            Err(error) => Err(io_error(ErrorCode::OpenFailed, &error)),
        }
    }

    #[must_use]
    pub fn actor(&self) -> ActorId {
        self.actor
    }

    #[must_use]
    pub fn user(&self) -> &UserId {
        &self.user
    }

    #[must_use]
    pub fn keyring_path(&self) -> &Path {
        &self.keyring_path
    }

    fn create_new(
        key_directory: &Path,
        keyring_path: PathBuf,
        actor: ActorId,
        user: UserId,
        kek: &KeyEncryptionKey,
        entropy: &mut impl EntropySource,
    ) -> Result<Self, Error> {
        let mut user_key = Zeroizing::new([0_u8; 32]);
        let mut data_key = Zeroizing::new([0_u8; 32]);
        let mut user_nonce = [0_u8; NONCE_BYTES];
        let mut data_nonce = [0_u8; NONCE_BYTES];
        entropy.fill(user_key.as_mut())?;
        entropy.fill(data_key.as_mut())?;
        entropy.fill(&mut user_nonce)?;
        entropy.fill(&mut data_nonce)?;

        let wrapped_user = encrypt_key(
            kek,
            &user_nonce,
            user_key.as_ref(),
            &key_aad(USER_KEY_DOMAIN, actor, &user),
        )?;
        let wrapped_data = encrypt_key(
            &user_key,
            &data_nonce,
            data_key.as_ref(),
            &key_aad(DATA_KEY_DOMAIN, actor, &user),
        )?;
        if wrapped_user.len() != WRAPPED_KEY_BYTES || wrapped_data.len() != WRAPPED_KEY_BYTES {
            return Err(Error::new(ErrorCode::InvariantViolation));
        }

        let mut bytes = [0_u8; KEYRING_BYTES];
        bytes[..8].copy_from_slice(KEYRING_MAGIC);
        bytes[8..12].copy_from_slice(&KEYRING_VERSION.to_le_bytes());
        bytes[12..14].copy_from_slice(&actor.get().to_le_bytes());
        bytes[16..32].copy_from_slice(&user);
        bytes[32..56].copy_from_slice(&user_nonce);
        bytes[56..104].copy_from_slice(&wrapped_user);
        bytes[104..128].copy_from_slice(&data_nonce);
        bytes[128..176].copy_from_slice(&wrapped_data);

        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&keyring_path)
            .map_err(|error| io_error(ErrorCode::OpenFailed, &error))?;
        file.write_all(&bytes)
            .map_err(|error| io_error(ErrorCode::WriteFailed, &error))?;
        file.sync_data()
            .map_err(|error| io_error(ErrorCode::SyncFailed, &error))?;
        sync_directory(key_directory)?;

        Ok(Self {
            actor,
            user,
            _user_key: user_key,
            data_key,
            keyring_path,
        })
    }

    fn open_existing(
        mut file: File,
        keyring_path: PathBuf,
        actor: ActorId,
        user: UserId,
        kek: &KeyEncryptionKey,
    ) -> Result<Self, Error> {
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|error| io_error(ErrorCode::ReadFailed, &error))?;
        if bytes.len() != KEYRING_BYTES
            || bytes.get(..8) != Some(KEYRING_MAGIC.as_slice())
            || read_u32(&bytes, 8)? != KEYRING_VERSION
            || bytes.get(14..16) != Some(&[0, 0])
        {
            return Err(Error::new(ErrorCode::CryptoAuthentication));
        }
        let stored_actor = ActorId::new(read_u16(&bytes, 12)?);
        let stored_user = copy_array::<16>(&bytes, 16)?;
        if stored_actor != actor || stored_user != user {
            return Err(Error::new(ErrorCode::CryptoAuthentication));
        }
        let user_nonce = copy_array::<NONCE_BYTES>(&bytes, 32)?;
        let data_nonce = copy_array::<NONCE_BYTES>(&bytes, 104)?;
        let wrapped_user = bytes
            .get(56..104)
            .ok_or_else(|| Error::new(ErrorCode::CryptoAuthentication))?;
        let wrapped_data = bytes
            .get(128..176)
            .ok_or_else(|| Error::new(ErrorCode::CryptoAuthentication))?;
        let user_key = Zeroizing::new(decrypt_key(
            kek,
            &user_nonce,
            wrapped_user,
            &key_aad(USER_KEY_DOMAIN, actor, &user),
        )?);
        let data_key = Zeroizing::new(decrypt_key(
            &user_key,
            &data_nonce,
            wrapped_data,
            &key_aad(DATA_KEY_DOMAIN, actor, &user),
        )?);
        Ok(Self {
            actor,
            user,
            _user_key: user_key,
            data_key,
            keyring_path,
        })
    }
}

fn key_aad(domain: &[u8], actor: ActorId, user: &UserId) -> Vec<u8> {
    let mut aad = Vec::with_capacity(domain.len() + 2 + user.len());
    aad.extend_from_slice(domain);
    aad.extend_from_slice(&actor.get().to_le_bytes());
    aad.extend_from_slice(user);
    aad
}

fn encrypt_key(
    key: &[u8; 32],
    nonce: &[u8; NONCE_BYTES],
    plaintext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, Error> {
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key));
    cipher
        .encrypt(
            XNonce::from_slice(nonce),
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| Error::new(ErrorCode::CryptoAuthentication))
}

fn decrypt_key(
    key: &[u8; 32],
    nonce: &[u8; NONCE_BYTES],
    ciphertext: &[u8],
    aad: &[u8],
) -> Result<[u8; 32], Error> {
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key));
    let plaintext = cipher
        .decrypt(
            XNonce::from_slice(nonce),
            Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map_err(|_| Error::new(ErrorCode::CryptoAuthentication))?;
    plaintext
        .as_slice()
        .try_into()
        .map_err(|_| Error::new(ErrorCode::CryptoAuthentication))
}

fn copy_array<const N: usize>(bytes: &[u8], offset: usize) -> Result<[u8; N], Error> {
    bytes
        .get(offset..offset + N)
        .ok_or_else(|| Error::new(ErrorCode::CryptoAuthentication))?
        .try_into()
        .map_err(|_| Error::new(ErrorCode::CryptoAuthentication))
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, Error> {
    Ok(u16::from_le_bytes(copy_array(bytes, offset)?))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, Error> {
    Ok(u32::from_le_bytes(copy_array(bytes, offset)?))
}

pub(crate) fn sync_directory(directory: &Path) -> Result<(), Error> {
    File::open(directory)
        .and_then(|file| file.sync_all())
        .map_err(|error| io_error(ErrorCode::SyncFailed, &error))
}

pub(crate) fn io_error(code: ErrorCode, error: &std::io::Error) -> Error {
    Error::new(code).with_system_error(error.raw_os_error().unwrap_or_default())
}
