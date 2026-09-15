#![allow(clippy::missing_errors_doc)]

use crate::keyring::KeyHierarchy;
use crate::keyring::{io_error, sync_directory};
use crate::mmr::Hash;
use ed25519_dalek::{Signature as DalekSignature, Signer, SigningKey, Verifier, VerifyingKey};
use hm_core::{ActorId, Error, ErrorCode, LSN};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use zeroize::Zeroizing;

pub type SigningSeed = [u8; 32];
pub type PublicKey = [u8; 32];
pub type Signature = [u8; 64];

pub const CHECKPOINT_BYTES: usize = 164;
const CHECKPOINT_MAGIC: u32 = 0x4e43_4350;
const CHECKPOINT_VERSION: u32 = 1;
const CHECKPOINT_CHECKSUM_OFFSET: usize = 160;
const CHECKPOINT_DOMAIN: &[u8] = b"neocortex-checkpoint-v1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SigningKeyPair {
    seed: Zeroizing<SigningSeed>,
    pub public_key: PublicKey,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Checkpoint {
    pub actor: ActorId,
    pub lsn: LSN,
    pub leaf_count: u64,
    pub root: Hash,
    pub public_key: PublicKey,
    pub signature: Signature,
}

#[must_use]
pub fn signing_key_pair_from_seed(seed: SigningSeed) -> SigningKeyPair {
    let signing = SigningKey::from_bytes(&seed);
    SigningKeyPair {
        seed: Zeroizing::new(seed),
        public_key: signing.verifying_key().to_bytes(),
    }
}

#[must_use]
pub fn signing_key_pair_for(keys: &KeyHierarchy) -> SigningKeyPair {
    let seed = blake3::derive_key(
        "hypermind.checkpoint-signing-key.v1",
        keys.data_key.as_ref(),
    );
    signing_key_pair_from_seed(seed)
}

impl SigningKeyPair {
    pub(crate) fn sign(&self, message: &[u8]) -> Signature {
        SigningKey::from_bytes(&self.seed).sign(message).to_bytes()
    }

    pub fn sign_checkpoint(
        &self,
        actor: ActorId,
        lsn: LSN,
        leaf_count: u64,
        root: Hash,
    ) -> Result<Checkpoint, Error> {
        if actor.get() == 0 || lsn.get() == 0 || leaf_count == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let mut checkpoint = Checkpoint {
            actor,
            lsn,
            leaf_count,
            root,
            public_key: self.public_key,
            signature: [0; 64],
        };
        checkpoint.signature = self.sign(&checkpoint_message(&checkpoint));
        Ok(checkpoint)
    }
}

pub fn verify_checkpoint_signature(
    checkpoint: &Checkpoint,
    expected_public_key: &PublicKey,
) -> Result<(), Error> {
    if checkpoint.public_key != *expected_public_key {
        return Err(Error::new(ErrorCode::SignatureInvalid).at_lsn(checkpoint.lsn));
    }
    let key = VerifyingKey::from_bytes(expected_public_key)
        .map_err(|_| Error::new(ErrorCode::SignatureInvalid).at_lsn(checkpoint.lsn))?;
    let signature = DalekSignature::from_bytes(&checkpoint.signature);
    key.verify(&checkpoint_message(checkpoint), &signature)
        .map_err(|_| Error::new(ErrorCode::SignatureInvalid).at_lsn(checkpoint.lsn))
}

#[must_use]
pub fn encode_checkpoint(checkpoint: &Checkpoint) -> [u8; CHECKPOINT_BYTES] {
    let mut encoded = [0_u8; CHECKPOINT_BYTES];
    encoded[0..4].copy_from_slice(&CHECKPOINT_MAGIC.to_le_bytes());
    encoded[4..8].copy_from_slice(&CHECKPOINT_VERSION.to_le_bytes());
    encoded[8..10].copy_from_slice(&checkpoint.actor.get().to_le_bytes());
    encoded[16..24].copy_from_slice(&checkpoint.lsn.get().to_le_bytes());
    encoded[24..32].copy_from_slice(&checkpoint.leaf_count.to_le_bytes());
    encoded[32..64].copy_from_slice(&checkpoint.root);
    encoded[64..96].copy_from_slice(&checkpoint.public_key);
    encoded[96..160].copy_from_slice(&checkpoint.signature);
    let checksum = checksum(&encoded);
    encoded[CHECKPOINT_CHECKSUM_OFFSET..].copy_from_slice(&checksum.to_le_bytes());
    encoded
}

pub fn decode_checkpoint(encoded: &[u8]) -> Result<Checkpoint, Error> {
    if encoded.len() != CHECKPOINT_BYTES {
        return Err(Error::new(ErrorCode::Truncated).at_offset(encoded.len() as u64));
    }
    if read_u32(encoded, 0)? != CHECKPOINT_MAGIC
        || read_u32(encoded, 4)? != CHECKPOINT_VERSION
        || read_u32(encoded, CHECKPOINT_CHECKSUM_OFFSET)? != checksum(encoded)
    {
        return Err(Error::new(ErrorCode::CheckpointMismatch));
    }
    Ok(Checkpoint {
        actor: ActorId::new(read_u16(encoded, 8)?),
        lsn: LSN::new(read_u64(encoded, 16)?),
        leaf_count: read_u64(encoded, 24)?,
        root: copy_array(encoded, 32)?,
        public_key: copy_array(encoded, 64)?,
        signature: copy_array(encoded, 96)?,
    })
}

pub fn load_checkpoint(path: &Path) -> Result<Checkpoint, Error> {
    let encoded = fs::read(path).map_err(|error| io_error(ErrorCode::OpenFailed, &error))?;
    decode_checkpoint(&encoded)
}

pub fn write_checkpoint(directory: &Path, checkpoint: &Checkpoint) -> Result<PathBuf, Error> {
    fs::create_dir_all(directory).map_err(|error| io_error(ErrorCode::OpenFailed, &error))?;
    let path = checkpoint_path(directory, checkpoint.lsn);
    let encoded = encode_checkpoint(checkpoint);
    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
    {
        Ok(mut file) => {
            file.write_all(&encoded)
                .map_err(|error| io_error(ErrorCode::WriteFailed, &error))?;
            file.sync_data()
                .map_err(|error| io_error(ErrorCode::SyncFailed, &error))?;
            sync_directory(directory)?;
            Ok(path)
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if load_checkpoint(&path)? == *checkpoint {
                Ok(path)
            } else {
                Err(Error::new(ErrorCode::AlreadyExists).at_lsn(checkpoint.lsn))
            }
        }
        Err(error) => Err(io_error(ErrorCode::OpenFailed, &error).at_lsn(checkpoint.lsn)),
    }
}

#[must_use]
pub fn checkpoint_path(directory: &Path, lsn: LSN) -> PathBuf {
    directory.join(format!("{:020}.ckpt", lsn.get()))
}

fn checkpoint_message(checkpoint: &Checkpoint) -> Vec<u8> {
    let mut message = Vec::with_capacity(CHECKPOINT_DOMAIN.len() + 50);
    message.extend_from_slice(CHECKPOINT_DOMAIN);
    message.extend_from_slice(&checkpoint.actor.get().to_le_bytes());
    message.extend_from_slice(&checkpoint.lsn.get().to_le_bytes());
    message.extend_from_slice(&checkpoint.leaf_count.to_le_bytes());
    message.extend_from_slice(&checkpoint.root);
    message
}

fn checksum(encoded: &[u8]) -> u32 {
    let mut copy = encoded.to_vec();
    copy[CHECKPOINT_CHECKSUM_OFFSET..CHECKPOINT_CHECKSUM_OFFSET + 4].fill(0);
    crc32c::crc32c(&copy)
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
