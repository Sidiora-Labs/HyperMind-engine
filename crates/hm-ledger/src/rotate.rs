#![allow(clippy::missing_errors_doc)]

use crate::keyring::{
    EntropySource, KEYRING_BYTES, KEYRING_MAGIC, KeyEncryptionKey, UserId, io_error, sync_directory,
};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use hm_core::{ActorId, Error, ErrorCode};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use zeroize::Zeroizing;

const NONCE_BYTES: usize = 24;
const USER_KEY_DOMAIN: &[u8] = b"neocortex.user-key.v1";
const DATA_KEY_DOMAIN: &[u8] = b"neocortex.data-key.v1";

pub fn rotate_keys(
    actor_directory: &Path,
    actor: ActorId,
    user: UserId,
    old_kek: &KeyEncryptionKey,
    new_kek: &KeyEncryptionKey,
    entropy: &mut impl EntropySource,
) -> Result<(), Error> {
    if old_kek == new_kek {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let key_directory = actor_directory.join("keys");
    if key_directory.join("DELETION_RECEIPT").exists()
        || key_directory.join("DELETION_RECEIPT.pending").exists()
    {
        return Err(Error::new(ErrorCode::KeyDestroyed));
    }
    let keyring = key_directory.join("KEYRING");
    let current = fs::read(&keyring).map_err(|error| io_error(ErrorCode::ReadFailed, &error))?;
    validate_keyring(&current, actor, &user)?;
    let old_user_key = Zeroizing::new(decrypt(
        old_kek,
        &copy_array(&current, 32)?,
        &current[56..104],
        &key_aad(USER_KEY_DOMAIN, actor, &user),
    )?);
    let data_key = Zeroizing::new(decrypt(
        &old_user_key,
        &copy_array(&current, 104)?,
        &current[128..176],
        &key_aad(DATA_KEY_DOMAIN, actor, &user),
    )?);
    let mut new_user_key = Zeroizing::new([0_u8; 32]);
    let mut user_nonce = [0_u8; NONCE_BYTES];
    let mut data_nonce = [0_u8; NONCE_BYTES];
    entropy.fill(new_user_key.as_mut())?;
    entropy.fill(&mut user_nonce)?;
    entropy.fill(&mut data_nonce)?;
    let wrapped_user = encrypt(
        new_kek,
        &user_nonce,
        new_user_key.as_ref(),
        &key_aad(USER_KEY_DOMAIN, actor, &user),
    )?;
    let wrapped_data = encrypt(
        &new_user_key,
        &data_nonce,
        data_key.as_ref(),
        &key_aad(DATA_KEY_DOMAIN, actor, &user),
    )?;
    let mut encoded = [0_u8; KEYRING_BYTES];
    encoded[..8].copy_from_slice(KEYRING_MAGIC);
    encoded[8..12].copy_from_slice(&1_u32.to_le_bytes());
    encoded[12..14].copy_from_slice(&actor.get().to_le_bytes());
    encoded[16..32].copy_from_slice(&user);
    encoded[32..56].copy_from_slice(&user_nonce);
    encoded[56..104].copy_from_slice(&wrapped_user);
    encoded[104..128].copy_from_slice(&data_nonce);
    encoded[128..176].copy_from_slice(&wrapped_data);

    let pending = key_directory.join("KEYRING.rotating");
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&pending)
        .map_err(|error| io_error(ErrorCode::OpenFailed, &error))?;
    file.write_all(&encoded)
        .map_err(|error| io_error(ErrorCode::WriteFailed, &error))?;
    file.sync_data()
        .map_err(|error| io_error(ErrorCode::SyncFailed, &error))?;
    fs::rename(&pending, &keyring).map_err(|error| io_error(ErrorCode::WriteFailed, &error))?;
    sync_directory(&key_directory)
}

fn validate_keyring(bytes: &[u8], actor: ActorId, user: &UserId) -> Result<(), Error> {
    if bytes.len() != KEYRING_BYTES
        || bytes.get(..8) != Some(KEYRING_MAGIC.as_slice())
        || bytes.get(8..12) != Some(1_u32.to_le_bytes().as_slice())
        || bytes.get(12..14) != Some(actor.get().to_le_bytes().as_slice())
        || bytes.get(16..32) != Some(user.as_slice())
    {
        Err(Error::new(ErrorCode::CryptoAuthentication))
    } else {
        Ok(())
    }
}

fn key_aad(domain: &[u8], actor: ActorId, user: &UserId) -> Vec<u8> {
    let mut aad = Vec::with_capacity(domain.len() + 2 + user.len());
    aad.extend_from_slice(domain);
    aad.extend_from_slice(&actor.get().to_le_bytes());
    aad.extend_from_slice(user);
    aad
}

fn encrypt(
    key: &[u8; 32],
    nonce: &[u8; 24],
    plaintext: &[u8],
    aad: &[u8],
) -> Result<Vec<u8>, Error> {
    XChaCha20Poly1305::new(Key::from_slice(key))
        .encrypt(
            XNonce::from_slice(nonce),
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| Error::new(ErrorCode::CryptoAuthentication))
}

fn decrypt(
    key: &[u8; 32],
    nonce: &[u8; 24],
    ciphertext: &[u8],
    aad: &[u8],
) -> Result<[u8; 32], Error> {
    let plaintext = XChaCha20Poly1305::new(Key::from_slice(key))
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
