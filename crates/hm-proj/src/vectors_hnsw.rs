#![allow(clippy::missing_errors_doc)]

use crate::vectors::{VectorHit, hamming};
use hm_core::{Error, ErrorCode, LSN};
use hm_index::{hnsw::HnswIndex, simd::simd_dot};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 8] = b"HMHNSW01";
const CHECKPOINT_INTERVAL: usize = 256;

#[derive(Clone, Debug)]
pub(crate) struct HnswVector {
    pub target_lsn: u64,
    pub quantized: Vec<i8>,
    pub binary_prefilter: Vec<u8>,
}

#[derive(Debug)]
pub struct HnswProjection {
    path: PathBuf,
    index: HnswIndex,
    records: BTreeMap<u64, HnswVector>,
    source: blake3::Hasher,
    checkpoint_count: usize,
}

impl HnswProjection {
    pub(crate) fn open(
        path: PathBuf,
        generation: &str,
        space: &str,
        dimensions: usize,
        records: Vec<HnswVector>,
    ) -> Result<Self, Error> {
        let mut source = blake3::Hasher::new();
        source.update(MAGIC);
        source.update(&(generation.len() as u64).to_le_bytes());
        source.update(generation.as_bytes());
        source.update(&(space.len() as u64).to_le_bytes());
        source.update(space.as_bytes());
        source.update(&(dimensions as u64).to_le_bytes());
        let checkpoint = match fs::read(&path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(_) => return Err(Error::new(ErrorCode::ReadFailed)),
        };
        let (index, checkpoint_count) = if let Some(bytes) = checkpoint {
            if bytes.len() < 80 || &bytes[..8] != MAGIC {
                return Err(Error::new(ErrorCode::VectorIndexCorrupt));
            }
            let count = usize::try_from(u64::from_le_bytes(bytes[8..16].try_into().unwrap()))
                .map_err(|_| Error::new(ErrorCode::VectorIndexCorrupt))?;
            if count > records.len()
                || blake3::hash(&bytes[..bytes.len() - 32]).as_bytes() != &bytes[bytes.len() - 32..]
            {
                return Err(Error::new(ErrorCode::VectorIndexCorrupt));
            }
            let mut prefix = source.clone();
            for record in &records[..count] {
                hash_record(&mut prefix, record);
            }
            if prefix.finalize().as_bytes() != &bytes[16..48] {
                return Err(Error::new(ErrorCode::VectorIndexCorrupt));
            }
            let index = HnswIndex::restore(dimensions, &bytes[48..bytes.len() - 32])?;
            if index.len() != count {
                return Err(Error::new(ErrorCode::VectorIndexCorrupt));
            }
            (index, count)
        } else {
            (HnswIndex::new(dimensions, records.len())?, 0)
        };
        let mut result = Self {
            path,
            index,
            records: BTreeMap::new(),
            source,
            checkpoint_count,
        };
        for (position, record) in records.into_iter().enumerate() {
            if position >= checkpoint_count {
                result.index.add(record.target_lsn, &record.quantized)?;
            }
            hash_record(&mut result.source, &record);
            if result.records.insert(record.target_lsn, record).is_some() {
                return Err(Error::new(ErrorCode::VectorIndexCorrupt));
            }
        }
        if result.records.len() != checkpoint_count {
            result.checkpoint()?;
        }
        Ok(result)
    }

    pub(crate) fn append(&mut self, record: HnswVector) -> Result<(), Error> {
        if self.records.contains_key(&record.target_lsn) {
            return Err(Error::new(ErrorCode::VectorIndexCorrupt));
        }
        self.index.add(record.target_lsn, &record.quantized)?;
        hash_record(&mut self.source, &record);
        self.records.insert(record.target_lsn, record);
        if self.records.len() - self.checkpoint_count >= CHECKPOINT_INTERVAL {
            self.checkpoint()?;
        }
        Ok(())
    }

    pub fn search(
        &self,
        query: &[i8],
        binary: &[u8],
        prefilter_limit: usize,
        limit: usize,
    ) -> Result<Vec<VectorHit>, Error> {
        let mut candidates: Vec<_> = self
            .records
            .values()
            .map(|record| (hamming(binary, &record.binary_prefilter), record.target_lsn))
            .collect();
        let admitted_count = prefilter_limit.max(limit);
        if candidates.len() > admitted_count {
            candidates.select_nth_unstable(admitted_count);
            candidates.truncate(admitted_count);
        }
        let admitted: BTreeSet<_> = candidates.into_iter().map(|(_, key)| key).collect();
        let keys = self
            .index
            .search(query, &admitted, limit.saturating_mul(4).max(32))?;
        let mut hits = Vec::with_capacity(keys.len());
        for key in keys {
            let record = self
                .records
                .get(&key)
                .ok_or_else(|| Error::new(ErrorCode::VectorIndexCorrupt))?;
            hits.push(VectorHit {
                target_lsn: LSN::new(key),
                score: simd_dot(query, &record.quantized)
                    .ok_or_else(|| Error::new(ErrorCode::VectorIndexCorrupt))?,
                hamming_distance: hamming(binary, &record.binary_prefilter),
            });
        }
        hits.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| left.hamming_distance.cmp(&right.hamming_distance))
                .then_with(|| left.target_lsn.cmp(&right.target_lsn))
        });
        hits.truncate(limit);
        Ok(hits)
    }

    pub fn checkpoint(&mut self) -> Result<(), Error> {
        let mut bytes = MAGIC.to_vec();
        bytes.extend_from_slice(&(self.records.len() as u64).to_le_bytes());
        bytes.extend_from_slice(self.source.finalize().as_bytes());
        bytes.extend_from_slice(&self.index.checkpoint()?);
        let checksum = blake3::hash(&bytes);
        bytes.extend_from_slice(checksum.as_bytes());
        write_checkpoint(&self.path, &bytes)?;
        self.checkpoint_count = self.records.len();
        Ok(())
    }

    #[must_use]
    pub const fn checkpoint_count(&self) -> usize {
        self.checkpoint_count
    }
}

fn hash_record(hasher: &mut blake3::Hasher, record: &HnswVector) {
    hasher.update(&record.target_lsn.to_le_bytes());
    for value in &record.quantized {
        hasher.update(&value.to_ne_bytes());
    }
    hasher.update(&record.binary_prefilter);
}

fn write_checkpoint(path: &Path, bytes: &[u8]) -> Result<(), Error> {
    let temporary = path.with_extension("hnsw-next");
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temporary)
        .map_err(|_| Error::new(ErrorCode::OpenFailed))?;
    file.write_all(bytes)
        .map_err(|_| Error::new(ErrorCode::WriteFailed))?;
    file.sync_all()
        .map_err(|_| Error::new(ErrorCode::SyncFailed))?;
    fs::rename(temporary, path).map_err(|_| Error::new(ErrorCode::WriteFailed))?;
    File::open(
        path.parent()
            .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?,
    )
    .and_then(|directory| directory.sync_all())
    .map_err(|_| Error::new(ErrorCode::SyncFailed))
}
