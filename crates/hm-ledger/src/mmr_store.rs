#![allow(clippy::missing_errors_doc)]

use crate::checkpoint::{
    Checkpoint, PublicKey, SigningKeyPair, load_checkpoint, verify_checkpoint_signature,
    write_checkpoint,
};
use crate::frame::{Frame, FrameHeader};
use crate::keyring::{io_error, sync_directory};
use crate::mmr::{AppendResult, Hash, Mmr, Node, RangeProof, hash_frame_sealed};
use hm_core::{ActorId, Error, ErrorCode, LSN};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

pub const NODE_RECORD_BYTES: usize = 60;
pub const REPAIR_BATCH_FRAMES: usize = 1_024;
const NODE_CHECKSUM_OFFSET: usize = 56;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VerificationStatus {
    pub root: Hash,
    pub leaf_count: u64,
    pub last_checkpoint_lsn: LSN,
    pub verified: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RepairStatus {
    pub repaired_leaves: usize,
    pub leaf_count: u64,
    pub complete: bool,
}

pub struct MmrStore {
    mmr_directory: PathBuf,
    checkpoint_directory: PathBuf,
    actor: ActorId,
    expected_public_key: PublicKey,
    mmr: Mmr,
    status: VerificationStatus,
}

impl MmrStore {
    pub fn open(
        actor_directory: &Path,
        actor: ActorId,
        expected_public_key: PublicKey,
    ) -> Result<Self, Error> {
        if actor.get() == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let mmr_directory = actor_directory.join("mmr");
        let checkpoint_directory = mmr_directory.join("checkpoints");
        fs::create_dir_all(&checkpoint_directory)
            .map_err(|error| io_error(ErrorCode::OpenFailed, &error))?;
        let mmr = restore_file(&mmr_directory.join("peaks"))?;
        let mut checkpoints = Vec::new();
        for entry in fs::read_dir(&checkpoint_directory)
            .map_err(|error| io_error(ErrorCode::ReadFailed, &error))?
        {
            let entry = entry.map_err(|error| io_error(ErrorCode::ReadFailed, &error))?;
            if !entry
                .file_type()
                .map_err(|error| io_error(ErrorCode::ReadFailed, &error))?
                .is_file()
            {
                continue;
            }
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if let Some(lsn) = parse_checkpoint_lsn(&name)? {
                checkpoints.push((lsn, entry.path()));
            }
        }
        checkpoints.sort_by_key(|item| item.0);
        let mut status = VerificationStatus {
            root: mmr.root(),
            leaf_count: mmr.leaf_count(),
            last_checkpoint_lsn: LSN::new(0),
            verified: false,
        };
        for (filename_lsn, path) in checkpoints {
            let checkpoint = load_checkpoint(&path)?;
            verify_checkpoint_signature(&checkpoint, &expected_public_key)?;
            let root = mmr.root_at(checkpoint.leaf_count)?;
            if checkpoint.actor != actor
                || checkpoint.lsn != filename_lsn
                || checkpoint.lsn.get() != checkpoint.leaf_count
                || checkpoint.root != root
            {
                return Err(Error::new(ErrorCode::CheckpointMismatch).at_lsn(checkpoint.lsn));
            }
            status.last_checkpoint_lsn = checkpoint.lsn;
            status.verified = true;
        }
        Ok(Self {
            mmr_directory,
            checkpoint_directory,
            actor,
            expected_public_key,
            mmr,
            status,
        })
    }

    pub fn append_frame(&mut self, frame: &Frame) -> Result<Hash, Error> {
        self.append_sealed(&frame.header, &frame.sealed_payload)
    }

    pub fn append_sealed(
        &mut self,
        header: &FrameHeader,
        sealed_payload: &[u8],
    ) -> Result<Hash, Error> {
        if header.actor != self.actor || header.lsn.get() != self.mmr.leaf_count() + 1 {
            return Err(Error::new(ErrorCode::SequenceViolation).at_lsn(header.lsn));
        }
        let mut next = self.mmr.clone();
        let append = next.append(hash_frame_sealed(header, sealed_payload));
        self.persist_nodes(&append)?;
        self.mmr = next;
        self.status.root = append.root;
        self.status.leaf_count = self.mmr.leaf_count();
        Ok(append.root)
    }

    pub fn create_checkpoint(&mut self, keys: &SigningKeyPair) -> Result<Checkpoint, Error> {
        if keys.public_key != self.expected_public_key || self.mmr.leaf_count() == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument).at_offset(self.mmr.leaf_count()));
        }
        let checkpoint = keys.sign_checkpoint(
            self.actor,
            LSN::new(self.mmr.leaf_count()),
            self.mmr.leaf_count(),
            self.mmr.root(),
        )?;
        write_checkpoint(&self.checkpoint_directory, &checkpoint)?;
        self.status.last_checkpoint_lsn = checkpoint.lsn;
        self.status.verified = true;
        Ok(checkpoint)
    }

    pub fn leaf_hash(&self, index: u64) -> Result<Hash, Error> {
        let index = usize::try_from(index).map_err(|_| Error::new(ErrorCode::InvalidArgument))?;
        self.mmr
            .leaves()
            .get(index)
            .copied()
            .ok_or_else(|| Error::new(ErrorCode::InvalidArgument).at_offset(index as u64))
    }

    pub fn verify_leaf_hashes(&self, first_index: u64, hashes: &[Hash]) -> Result<(), Error> {
        let start = usize::try_from(first_index)
            .map_err(|_| Error::new(ErrorCode::InvalidArgument).at_offset(first_index))?;
        let end = start
            .checked_add(hashes.len())
            .filter(|end| *end <= self.mmr.leaves().len())
            .ok_or_else(|| Error::new(ErrorCode::InvalidArgument).at_offset(first_index))?;
        if self.mmr.leaves()[start..end] == *hashes {
            Ok(())
        } else {
            Err(Error::new(ErrorCode::CheckpointMismatch).at_offset(first_index))
        }
    }

    pub fn root_at(&self, leaf_count: u64) -> Result<Hash, Error> {
        self.mmr.root_at(leaf_count)
    }

    pub fn prove_range(
        &self,
        range_start: u64,
        range_leaf_count: u64,
    ) -> Result<RangeProof, Error> {
        self.mmr.prove_range(range_start, range_leaf_count)
    }

    pub fn rewind_to(&mut self, leaf_count: u64) -> Result<(), Error> {
        if self.mmr.leaf_count() <= leaf_count {
            return Ok(());
        }
        if self.status.last_checkpoint_lsn.get() > leaf_count {
            return Err(Error::new(ErrorCode::CheckpointMismatch)
                .at_lsn(self.status.last_checkpoint_lsn)
                .at_offset(leaf_count));
        }
        let record_count = leaf_count
            .checked_mul(2)
            .and_then(|value| value.checked_sub(u64::from(leaf_count.count_ones())))
            .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?;
        let path = self.mmr_directory.join("peaks");
        let file = OpenOptions::new()
            .write(true)
            .open(&path)
            .map_err(|error| io_error(ErrorCode::OpenFailed, &error))?;
        file.set_len(record_count * NODE_RECORD_BYTES as u64)
            .map_err(|error| io_error(ErrorCode::WriteFailed, &error))?;
        file.sync_data()
            .map_err(|error| io_error(ErrorCode::SyncFailed, &error))?;
        sync_directory(&self.mmr_directory)?;
        self.mmr = restore_file(&path)?;
        self.status.root = self.mmr.root();
        self.status.leaf_count = self.mmr.leaf_count();
        Ok(())
    }

    pub fn verify_and_repair_bounded(&mut self, frames: &[Frame]) -> Result<RepairStatus, Error> {
        let frame_count =
            u64::try_from(frames.len()).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
        if self.mmr.leaf_count() > frame_count {
            self.rewind_to(frame_count)?;
        }
        for (index, stored) in self.mmr.leaves().iter().enumerate() {
            let frame = frames
                .get(index)
                .ok_or_else(|| Error::new(ErrorCode::CheckpointMismatch))?;
            if *stored != hash_frame_sealed(&frame.header, &frame.sealed_payload) {
                return Err(Error::new(ErrorCode::CheckpointMismatch).at_lsn(frame.header.lsn));
            }
        }
        let start = usize::try_from(self.mmr.leaf_count())
            .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
        let end = start.saturating_add(REPAIR_BATCH_FRAMES).min(frames.len());
        for frame in &frames[start..end] {
            self.append_frame(frame)?;
        }
        Ok(RepairStatus {
            repaired_leaves: end - start,
            leaf_count: self.mmr.leaf_count(),
            complete: end == frames.len(),
        })
    }

    #[must_use]
    pub const fn mmr(&self) -> &Mmr {
        &self.mmr
    }

    #[must_use]
    pub const fn verification_status(&self) -> VerificationStatus {
        self.status
    }

    fn persist_nodes(&self, append: &AppendResult) -> Result<(), Error> {
        let path = self.mmr_directory.join("peaks");
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .mode(0o600)
            .open(&path)
            .map_err(|error| io_error(ErrorCode::OpenFailed, &error))?;
        for node in &append.created_nodes {
            file.write_all(&encode_node(node))
                .map_err(|error| io_error(ErrorCode::WriteFailed, &error))?;
        }
        file.sync_data()
            .map_err(|error| io_error(ErrorCode::SyncFailed, &error))?;
        sync_directory(&self.mmr_directory)
    }
}

pub fn restore_peak_records(encoded: &[u8]) -> Result<Mmr, Error> {
    if !encoded.len().is_multiple_of(NODE_RECORD_BYTES) {
        return Err(Error::new(ErrorCode::Truncated).at_offset(encoded.len() as u64));
    }
    let records = encoded
        .chunks_exact(NODE_RECORD_BYTES)
        .enumerate()
        .map(|(index, record)| decode_node(record).map_err(|error| error.at_offset(index as u64)))
        .collect::<Result<Vec<_>, _>>()?;
    restore_nodes(&records)
}

fn restore_file(path: &Path) -> Result<Mmr, Error> {
    match fs::read(path) {
        Ok(bytes) => restore_peak_records(&bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Mmr::default()),
        Err(error) => Err(io_error(ErrorCode::ReadFailed, &error)),
    }
}

fn restore_nodes(records: &[Node]) -> Result<Mmr, Error> {
    let mut mmr = Mmr::default();
    let mut index = 0;
    while index < records.len() {
        let record = records[index];
        if record.height != 0 || record.leaf_count != 1 || record.start != mmr.leaf_count() {
            return Err(Error::new(ErrorCode::CheckpointMismatch).at_offset(index as u64));
        }
        let append = mmr.append(record.hash);
        let end = index
            .checked_add(append.created_nodes.len())
            .filter(|end| *end <= records.len())
            .ok_or_else(|| Error::new(ErrorCode::Truncated).at_offset(index as u64))?;
        if append.created_nodes != records[index..end] {
            return Err(Error::new(ErrorCode::CheckpointMismatch).at_offset(index as u64));
        }
        index = end;
    }
    Ok(mmr)
}

fn encode_node(node: &Node) -> [u8; NODE_RECORD_BYTES] {
    let mut encoded = [0_u8; NODE_RECORD_BYTES];
    encoded[0] = node.height;
    encoded[8..16].copy_from_slice(&node.start.to_le_bytes());
    encoded[16..24].copy_from_slice(&node.leaf_count.to_le_bytes());
    encoded[24..56].copy_from_slice(&node.hash);
    let checksum = node_checksum(&encoded);
    encoded[NODE_CHECKSUM_OFFSET..].copy_from_slice(&checksum.to_le_bytes());
    encoded
}

fn decode_node(encoded: &[u8]) -> Result<Node, Error> {
    if encoded.len() != NODE_RECORD_BYTES
        || read_u32(encoded, NODE_CHECKSUM_OFFSET)? != node_checksum(encoded)
    {
        return Err(Error::new(ErrorCode::ChecksumMismatch));
    }
    Ok(Node {
        height: encoded[0],
        start: read_u64(encoded, 8)?,
        leaf_count: read_u64(encoded, 16)?,
        hash: copy_array(encoded, 24)?,
    })
}

fn node_checksum(encoded: &[u8]) -> u32 {
    let mut copy = encoded.to_vec();
    copy[NODE_CHECKSUM_OFFSET..NODE_CHECKSUM_OFFSET + 4].fill(0);
    crc32c::crc32c(&copy)
}

fn parse_checkpoint_lsn(name: &str) -> Result<Option<LSN>, Error> {
    let Some(number) = name.strip_suffix(".ckpt") else {
        return Ok(None);
    };
    if number.len() != 20 || !number.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    number
        .parse::<u64>()
        .map(LSN::new)
        .map(Some)
        .map_err(|_| Error::new(ErrorCode::InvalidArgument))
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
