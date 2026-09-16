#![allow(clippy::missing_errors_doc)]

#[cfg(feature = "hnsw")]
use crate::vectors_hnsw::{HnswProjection, HnswVector};
use hm_core::{Error, ErrorCode, LSN};
use hm_index::simd::simd_dot;
use memmap2::MmapOptions;
use std::fs::{self, File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
#[cfg(feature = "hnsw")]
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

const FILE_MAGIC: &[u8; 8] = b"HMVEC001";
const RECORD_MAGIC: &[u8; 4] = b"HMVR";
const VERSION: u16 = 1;
const CHECKSUM_BYTES: usize = 32;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VectorHit {
    pub target_lsn: LSN,
    pub score: i64,
    pub hamming_distance: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VectorEntry {
    pub target_lsn: LSN,
    pub quantized: Vec<i8>,
    pub binary_prefilter: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct VectorLane {
    path: PathBuf,
    generation_id: String,
    space_id: String,
    dimensions: usize,
    #[cfg(feature = "hnsw")]
    count: Arc<AtomicUsize>,
    #[cfg(feature = "hnsw")]
    hnsw: Arc<Mutex<Option<HnswProjection>>>,
}

impl VectorLane {
    pub fn open(
        directory: impl AsRef<Path>,
        generation_id: &str,
        space_id: &str,
        dimensions: usize,
    ) -> Result<Self, Error> {
        if generation_id.is_empty()
            || space_id.is_empty()
            || dimensions == 0
            || generation_id.len() > usize::from(u16::MAX)
            || space_id.len() > usize::from(u16::MAX)
        {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        fs::create_dir_all(directory.as_ref()).map_err(|_| Error::new(ErrorCode::OpenFailed))?;
        let path = directory.as_ref().join(format!(
            "{}.hmvec",
            blake3::hash(generation_id.as_bytes()).to_hex()
        ));
        let lane = Self {
            path,
            generation_id: generation_id.to_owned(),
            space_id: space_id.to_owned(),
            dimensions,
            #[cfg(feature = "hnsw")]
            count: Arc::new(AtomicUsize::new(0)),
            #[cfg(feature = "hnsw")]
            hnsw: Arc::new(Mutex::new(None)),
        };
        if lane.path.exists() {
            lane.with_map(|bytes| lane.validate_file(bytes))?;
        } else {
            lane.initialize()?;
        }
        #[cfg(feature = "hnsw")]
        {
            let count = lane.validate()?;
            lane.count.store(count, Ordering::Release);
            if count > hm_index::hnsw::HNSW_THRESHOLD {
                *lane
                    .hnsw
                    .lock()
                    .map_err(|_| Error::new(ErrorCode::InvariantViolation))? =
                    Some(lane.open_hnsw()?);
            }
        }
        Ok(lane)
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub fn generation_id(&self) -> &str {
        &self.generation_id
    }

    #[must_use]
    pub fn space_id(&self) -> &str {
        &self.space_id
    }

    #[must_use]
    pub const fn dimensions(&self) -> usize {
        self.dimensions
    }

    pub fn replay(&self, entries: &[VectorEntry]) -> Result<(), Error> {
        let existing = self.with_map(|bytes| {
            let records = self.records(bytes)?;
            if records.len() > entries.len() {
                return Err(Error::new(ErrorCode::ProjectionCheckpoint));
            }
            for (record, expected) in records.iter().zip(entries) {
                if record.target_lsn != expected.target_lsn.get()
                    || record.quantized != expected.quantized
                    || record.binary_prefilter != expected.binary_prefilter
                {
                    return Err(Error::new(ErrorCode::VectorIndexCorrupt));
                }
            }
            Ok(records.len())
        })?;
        if existing != entries.len() {
            let mut file = OpenOptions::new()
                .append(true)
                .open(&self.path)
                .map_err(|_| Error::new(ErrorCode::OpenFailed))?;
            for entry in &entries[existing..] {
                self.write_record(
                    &mut file,
                    entry.target_lsn,
                    &entry.quantized,
                    &entry.binary_prefilter,
                )?;
            }
            file.sync_data()
                .map_err(|_| Error::new(ErrorCode::SyncFailed))?;
        }
        #[cfg(feature = "hnsw")]
        {
            self.count.store(entries.len(), Ordering::Release);
            if entries.len() > hm_index::hnsw::HNSW_THRESHOLD && existing != entries.len() {
                *self
                    .hnsw
                    .lock()
                    .map_err(|_| Error::new(ErrorCode::InvariantViolation))? =
                    Some(self.open_hnsw()?);
            }
        }
        Ok(())
    }

    pub fn append(
        &self,
        target_lsn: LSN,
        quantized: &[i8],
        binary_prefilter: &[u8],
    ) -> Result<(), Error> {
        self.append_record(target_lsn, quantized, binary_prefilter)?;
        #[cfg(feature = "hnsw")]
        {
            let count = self.count.fetch_add(1, Ordering::AcqRel) + 1;
            if count > hm_index::hnsw::HNSW_THRESHOLD {
                let mut projection = self
                    .hnsw
                    .lock()
                    .map_err(|_| Error::new(ErrorCode::InvariantViolation))?;
                if let Some(index) = projection.as_mut() {
                    index.append(HnswVector {
                        target_lsn: target_lsn.get(),
                        quantized: quantized.to_vec(),
                        binary_prefilter: binary_prefilter.to_vec(),
                    })?;
                } else {
                    *projection = Some(self.open_hnsw()?);
                }
            }
        }
        Ok(())
    }

    fn append_record(
        &self,
        target_lsn: LSN,
        quantized: &[i8],
        binary_prefilter: &[u8],
    ) -> Result<(), Error> {
        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.path)
            .map_err(|_| Error::new(ErrorCode::OpenFailed))?;
        self.write_record(&mut file, target_lsn, quantized, binary_prefilter)?;
        file.sync_data()
            .map_err(|_| Error::new(ErrorCode::SyncFailed))
    }

    fn write_record(
        &self,
        file: &mut File,
        target_lsn: LSN,
        quantized: &[i8],
        binary_prefilter: &[u8],
    ) -> Result<(), Error> {
        if target_lsn.get() == 0
            || quantized.len() != self.dimensions
            || binary_prefilter.len() != self.dimensions.div_ceil(8)
        {
            return Err(Error::new(ErrorCode::InvalidArgument).at_lsn(target_lsn));
        }
        let quantized_len =
            u32::try_from(quantized.len()).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
        let binary_len = u32::try_from(binary_prefilter.len())
            .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
        let mut body =
            Vec::with_capacity(16 + quantized.len() + binary_prefilter.len() + CHECKSUM_BYTES);
        body.extend_from_slice(&target_lsn.get().to_le_bytes());
        body.extend_from_slice(&quantized_len.to_le_bytes());
        body.extend_from_slice(&binary_len.to_le_bytes());
        body.extend(quantized.iter().map(|value| value.to_ne_bytes()[0]));
        body.extend_from_slice(binary_prefilter);
        let digest = blake3::hash(&body);
        body.extend_from_slice(digest.as_bytes());
        let body_len =
            u32::try_from(body.len()).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
        file.write_all(RECORD_MAGIC)
            .and_then(|()| file.write_all(&body_len.to_le_bytes()))
            .and_then(|()| file.write_all(&body))
            .map_err(|_| Error::new(ErrorCode::WriteFailed))
    }

    pub fn search(
        &self,
        query: &[i8],
        query_prefilter: &[u8],
        prefilter_limit: usize,
        limit: usize,
    ) -> Result<Vec<VectorHit>, Error> {
        self.validate_query(query, query_prefilter, prefilter_limit, limit)?;
        #[cfg(feature = "hnsw")]
        if let Some(projection) = self
            .hnsw
            .lock()
            .map_err(|_| Error::new(ErrorCode::InvariantViolation))?
            .as_ref()
        {
            return projection.search(query, query_prefilter, prefilter_limit, limit);
        }
        self.search_flat(query, query_prefilter, prefilter_limit, limit)
    }

    pub fn search_flat(
        &self,
        query: &[i8],
        query_prefilter: &[u8],
        prefilter_limit: usize,
        limit: usize,
    ) -> Result<Vec<VectorHit>, Error> {
        self.validate_query(query, query_prefilter, prefilter_limit, limit)?;
        self.with_map(|bytes| {
            let records = self.records(bytes)?;
            let mut candidates: Vec<_> = records
                .into_iter()
                .map(|record| {
                    let hamming_distance = hamming(query_prefilter, record.binary_prefilter);
                    (record, hamming_distance)
                })
                .collect();
            candidates.sort_by_key(|(record, distance)| (*distance, record.target_lsn));
            candidates.truncate(prefilter_limit.max(limit).min(candidates.len()));
            let mut hits = candidates
                .into_iter()
                .map(|(record, hamming_distance)| {
                    let score = simd_dot(query, &record.quantized)
                        .ok_or_else(|| Error::new(ErrorCode::VectorIndexCorrupt))?;
                    Ok(VectorHit {
                        target_lsn: LSN::new(record.target_lsn),
                        score,
                        hamming_distance,
                    })
                })
                .collect::<Result<Vec<_>, Error>>()?;
            hits.sort_by(|left, right| {
                right
                    .score
                    .cmp(&left.score)
                    .then_with(|| left.hamming_distance.cmp(&right.hamming_distance))
                    .then_with(|| left.target_lsn.cmp(&right.target_lsn))
            });
            hits.truncate(limit);
            Ok(hits)
        })
    }

    fn validate_query(
        &self,
        query: &[i8],
        query_prefilter: &[u8],
        prefilter_limit: usize,
        limit: usize,
    ) -> Result<(), Error> {
        if query.len() != self.dimensions
            || query_prefilter.len() != self.dimensions.div_ceil(8)
            || prefilter_limit == 0
            || limit == 0
        {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        Ok(())
    }

    #[cfg(feature = "hnsw")]
    pub fn hnsw_checkpoint_count(&self) -> Result<Option<usize>, Error> {
        Ok(self
            .hnsw
            .lock()
            .map_err(|_| Error::new(ErrorCode::InvariantViolation))?
            .as_ref()
            .map(HnswProjection::checkpoint_count))
    }

    #[cfg(feature = "hnsw")]
    fn open_hnsw(&self) -> Result<HnswProjection, Error> {
        self.with_map(|bytes| {
            let records = self
                .records(bytes)?
                .into_iter()
                .map(|record| HnswVector {
                    target_lsn: record.target_lsn,
                    quantized: record.quantized,
                    binary_prefilter: record.binary_prefilter.to_vec(),
                })
                .collect();
            HnswProjection::open(
                self.path.with_extension("hnsw"),
                &self.generation_id,
                &self.space_id,
                self.dimensions,
                records,
            )
        })
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, Error> {
        self.with_map(|bytes| {
            self.validate_file(bytes)?;
            Ok(bytes.to_vec())
        })
    }

    pub fn validate(&self) -> Result<usize, Error> {
        self.with_map(|bytes| self.records(bytes).map(|records| records.len()))
    }

    fn initialize(&self) -> Result<(), Error> {
        let generation_len = u16::try_from(self.generation_id.len())
            .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
        let space_len = u16::try_from(self.space_id.len())
            .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
        let dimensions =
            u32::try_from(self.dimensions).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&self.path)
            .map_err(|_| Error::new(ErrorCode::OpenFailed))?;
        file.write_all(FILE_MAGIC)
            .and_then(|()| file.write_all(&VERSION.to_le_bytes()))
            .and_then(|()| file.write_all(&dimensions.to_le_bytes()))
            .and_then(|()| file.write_all(&generation_len.to_le_bytes()))
            .and_then(|()| file.write_all(&space_len.to_le_bytes()))
            .and_then(|()| file.write_all(self.generation_id.as_bytes()))
            .and_then(|()| file.write_all(self.space_id.as_bytes()))
            .map_err(|_| Error::new(ErrorCode::WriteFailed))?;
        file.sync_data()
            .map_err(|_| Error::new(ErrorCode::SyncFailed))?;
        File::open(self.path.parent().expect("vector parent"))
            .and_then(|directory| directory.sync_all())
            .map_err(|_| Error::new(ErrorCode::SyncFailed))
    }

    fn with_map<T>(&self, operation: impl FnOnce(&[u8]) -> Result<T, Error>) -> Result<T, Error> {
        let file = File::open(&self.path).map_err(|_| Error::new(ErrorCode::OpenFailed))?;
        // SAFETY: the file is append-only, the returned mapping is read-only, and it cannot outlive file parsing.
        let mapping = unsafe { MmapOptions::new().map(&file) }
            .map_err(|_| Error::new(ErrorCode::ReadFailed))?;
        operation(&mapping)
    }

    fn validate_file(&self, bytes: &[u8]) -> Result<(), Error> {
        self.records(bytes).map(|_| ())
    }

    fn records<'a>(&self, bytes: &'a [u8]) -> Result<Vec<Record<'a>>, Error> {
        let mut cursor = 0;
        if take(bytes, &mut cursor, FILE_MAGIC.len())? != FILE_MAGIC
            || read_u16(bytes, &mut cursor)? != VERSION
            || usize::try_from(read_u32(bytes, &mut cursor)?).ok() != Some(self.dimensions)
        {
            return Err(Error::new(ErrorCode::VectorIndexCorrupt));
        }
        let generation_len = usize::from(read_u16(bytes, &mut cursor)?);
        let space_len = usize::from(read_u16(bytes, &mut cursor)?);
        if take(bytes, &mut cursor, generation_len)? != self.generation_id.as_bytes()
            || take(bytes, &mut cursor, space_len)? != self.space_id.as_bytes()
        {
            return Err(Error::new(ErrorCode::VectorIndexCorrupt));
        }
        let mut records = Vec::new();
        while cursor < bytes.len() {
            let frame_start = cursor;
            if take(bytes, &mut cursor, RECORD_MAGIC.len())? != RECORD_MAGIC {
                return Err(Error::new(ErrorCode::VectorIndexCorrupt).at_offset(frame_start as u64));
            }
            let body_len = usize::try_from(read_u32(bytes, &mut cursor)?)
                .map_err(|_| Error::new(ErrorCode::VectorIndexCorrupt))?;
            let body = take(bytes, &mut cursor, body_len)?;
            if body.len() < 16 + CHECKSUM_BYTES {
                return Err(Error::new(ErrorCode::VectorIndexCorrupt));
            }
            let (content, checksum) = body.split_at(body.len() - CHECKSUM_BYTES);
            if blake3::hash(content).as_bytes() != checksum {
                return Err(Error::new(ErrorCode::ChecksumMismatch).at_offset(frame_start as u64));
            }
            let mut body_cursor = 0;
            let target_lsn = read_u64(content, &mut body_cursor)?;
            let quantized_len = usize::try_from(read_u32(content, &mut body_cursor)?)
                .map_err(|_| Error::new(ErrorCode::VectorIndexCorrupt))?;
            let binary_len = usize::try_from(read_u32(content, &mut body_cursor)?)
                .map_err(|_| Error::new(ErrorCode::VectorIndexCorrupt))?;
            if target_lsn == 0
                || quantized_len != self.dimensions
                || binary_len != self.dimensions.div_ceil(8)
            {
                return Err(Error::new(ErrorCode::VectorIndexCorrupt));
            }
            let quantized = take(content, &mut body_cursor, quantized_len)?
                .iter()
                .map(|byte| i8::from_ne_bytes([*byte]))
                .collect();
            let binary_prefilter = take(content, &mut body_cursor, binary_len)?;
            if body_cursor != content.len() {
                return Err(Error::new(ErrorCode::VectorIndexCorrupt));
            }
            records.push(Record {
                target_lsn,
                quantized,
                binary_prefilter,
            });
        }
        Ok(records)
    }
}

struct Record<'a> {
    target_lsn: u64,
    quantized: Vec<i8>,
    binary_prefilter: &'a [u8],
}

pub(crate) fn hamming(left: &[u8], right: &[u8]) -> u32 {
    left.iter()
        .zip(right)
        .map(|(left, right)| (left ^ right).count_ones())
        .sum()
}

fn take<'a>(bytes: &'a [u8], cursor: &mut usize, count: usize) -> Result<&'a [u8], Error> {
    let end = cursor
        .checked_add(count)
        .ok_or_else(|| Error::new(ErrorCode::VectorIndexCorrupt))?;
    let value = bytes
        .get(*cursor..end)
        .ok_or_else(|| Error::new(ErrorCode::Truncated).at_offset(*cursor as u64))?;
    *cursor = end;
    Ok(value)
}

fn read_u16(bytes: &[u8], cursor: &mut usize) -> Result<u16, Error> {
    take(bytes, cursor, 2).map(|value| u16::from_le_bytes(value.try_into().expect("two bytes")))
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> Result<u32, Error> {
    take(bytes, cursor, 4).map(|value| u32::from_le_bytes(value.try_into().expect("four bytes")))
}

fn read_u64(bytes: &[u8], cursor: &mut usize) -> Result<u64, Error> {
    take(bytes, cursor, 8).map(|value| u64::from_le_bytes(value.try_into().expect("eight bytes")))
}
