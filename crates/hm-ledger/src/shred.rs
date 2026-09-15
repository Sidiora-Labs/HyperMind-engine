#![allow(clippy::missing_errors_doc)]

use crate::checkpoint::{PublicKey, Signature, SigningKeyPair};
use crate::keyring::{KeyHierarchy, UserId, io_error, sync_directory};
use crate::mmr::{Hash, hash_bytes};
use ed25519_dalek::{Signature as DalekSignature, Verifier, VerifyingKey};
use hm_core::{ActorId, Error, ErrorCode, LSN};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

pub const DELETION_RECEIPT_BYTES: usize = 200;
pub const DELETION_RECEIPT_NAME: &str = "DELETION_RECEIPT";
const RECEIPT_MAGIC: &[u8; 8] = b"NCDEL001";
const RECEIPT_VERSION: u32 = 1;
const RECEIPT_MESSAGE_BYTES: usize = 136;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeletionReceipt {
    pub actor: ActorId,
    pub user: UserId,
    pub deleted_at_lsn: LSN,
    pub key_fingerprint: Hash,
    pub checkpoint_root: Hash,
    pub public_key: PublicKey,
    pub signature: Signature,
}

pub fn crypto_shred(
    keys: KeyHierarchy,
    deleted_at_lsn: LSN,
    checkpoint_root: Hash,
    signing_keys: &SigningKeyPair,
) -> Result<DeletionReceipt, Error> {
    if deleted_at_lsn.get() == 0 {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let key_directory = keys
        .keyring_path()
        .parent()
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?
        .to_owned();
    let mut receipt = DeletionReceipt {
        actor: keys.actor(),
        user: *keys.user(),
        deleted_at_lsn,
        key_fingerprint: hash_bytes(keys.data_key.as_ref()),
        checkpoint_root,
        public_key: signing_keys.public_key,
        signature: [0; 64],
    };
    receipt.signature = signing_keys.sign(&receipt_message(&receipt));
    let encoded = encode_deletion_receipt(&receipt);
    let pending = key_directory.join("DELETION_RECEIPT.pending");
    let final_path = key_directory.join(DELETION_RECEIPT_NAME);
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
    fs::remove_file(keys.keyring_path())
        .map_err(|error| io_error(ErrorCode::WriteFailed, &error))?;
    sync_directory(&key_directory)?;
    fs::rename(&pending, &final_path).map_err(|error| io_error(ErrorCode::WriteFailed, &error))?;
    sync_directory(&key_directory)?;
    drop(keys);
    Ok(receipt)
}

pub fn load_deletion_receipt(
    actor_directory: &Path,
    trusted_public_key: &PublicKey,
) -> Result<DeletionReceipt, Error> {
    let path = actor_directory.join("keys").join(DELETION_RECEIPT_NAME);
    let encoded = fs::read(path).map_err(|error| io_error(ErrorCode::OpenFailed, &error))?;
    let receipt = decode_deletion_receipt(&encoded)?;
    verify_deletion_receipt(&receipt, trusted_public_key)?;
    Ok(receipt)
}

pub fn verify_deletion_receipt(
    receipt: &DeletionReceipt,
    trusted_public_key: &PublicKey,
) -> Result<(), Error> {
    if receipt.public_key != *trusted_public_key {
        return Err(Error::new(ErrorCode::SignatureInvalid));
    }
    let key = VerifyingKey::from_bytes(trusted_public_key)
        .map_err(|_| Error::new(ErrorCode::SignatureInvalid))?;
    key.verify(
        &receipt_message(receipt),
        &DalekSignature::from_bytes(&receipt.signature),
    )
    .map_err(|_| Error::new(ErrorCode::SignatureInvalid))
}

#[must_use]
pub fn encode_deletion_receipt(receipt: &DeletionReceipt) -> [u8; DELETION_RECEIPT_BYTES] {
    let mut encoded = [0_u8; DELETION_RECEIPT_BYTES];
    encoded[..RECEIPT_MESSAGE_BYTES].copy_from_slice(&receipt_message(receipt));
    encoded[RECEIPT_MESSAGE_BYTES..].copy_from_slice(&receipt.signature);
    encoded
}

pub fn decode_deletion_receipt(encoded: &[u8]) -> Result<DeletionReceipt, Error> {
    if encoded.len() != DELETION_RECEIPT_BYTES
        || encoded.get(..8) != Some(RECEIPT_MAGIC.as_slice())
        || read_u32(encoded, 8)? != RECEIPT_VERSION
    {
        return Err(Error::new(ErrorCode::InvalidLength));
    }
    Ok(DeletionReceipt {
        actor: ActorId::new(read_u16(encoded, 12)?),
        user: copy_array(encoded, 16)?,
        deleted_at_lsn: LSN::new(read_u64(encoded, 32)?),
        key_fingerprint: copy_array(encoded, 40)?,
        checkpoint_root: copy_array(encoded, 72)?,
        public_key: copy_array(encoded, 104)?,
        signature: copy_array(encoded, 136)?,
    })
}

fn receipt_message(receipt: &DeletionReceipt) -> [u8; RECEIPT_MESSAGE_BYTES] {
    let mut encoded = [0_u8; RECEIPT_MESSAGE_BYTES];
    encoded[..8].copy_from_slice(RECEIPT_MAGIC);
    encoded[8..12].copy_from_slice(&RECEIPT_VERSION.to_le_bytes());
    encoded[12..14].copy_from_slice(&receipt.actor.get().to_le_bytes());
    encoded[16..32].copy_from_slice(&receipt.user);
    encoded[32..40].copy_from_slice(&receipt.deleted_at_lsn.get().to_le_bytes());
    encoded[40..72].copy_from_slice(&receipt.key_fingerprint);
    encoded[72..104].copy_from_slice(&receipt.checkpoint_root);
    encoded[104..136].copy_from_slice(&receipt.public_key);
    encoded
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, Error> {
    Ok(u16::from_le_bytes(copy_array(bytes, offset)?))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, Error> {
    Ok(u32::from_le_bytes(copy_array(bytes, offset)?))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, Error> {
    Ok(u64::from_le_bytes(copy_array(bytes, offset)?))
}

fn copy_array<const N: usize>(bytes: &[u8], offset: usize) -> Result<[u8; N], Error> {
    bytes
        .get(offset..offset + N)
        .ok_or_else(|| Error::new(ErrorCode::Truncated).at_offset(offset as u64))?
        .try_into()
        .map_err(|_| Error::new(ErrorCode::Truncated).at_offset(offset as u64))
}
