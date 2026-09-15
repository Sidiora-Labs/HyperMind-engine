#![allow(clippy::missing_errors_doc)]

use crate::frame::{self, EventKind, FRAME_HEADER_SIZE, Frame, FrameHeader, MAXIMUM_FRAME_BYTES};
use crate::keyring::{io_error, sync_directory};
use hm_core::{ActorId, ConversationId, Error, ErrorCode, LSN, UtcNanos};
use std::fs::{self, File, OpenOptions};
use std::os::unix::fs::{FileExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

pub const DEFAULT_SEGMENT_BYTES: u64 = 128 * 1024 * 1024;
const MANIFEST_MAGIC: &[u8; 4] = b"NCMF";
const MANIFEST_NAME: &str = "MANIFEST";
const MANIFEST_VERSION: u32 = 1;
const MANIFEST_HEADER_BYTES: usize = 16;
const MANIFEST_ENTRY_BYTES: usize = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WriteBackend {
    Auto,
    Pwrite,
    IoUringDirect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SegmentLogOptions {
    pub segment_bytes: u64,
    pub maximum_frame_bytes: usize,
    pub direct_alignment: u32,
    pub maximum_io_bytes: usize,
    pub backend: WriteBackend,
}

impl Default for SegmentLogOptions {
    fn default() -> Self {
        Self {
            segment_bytes: DEFAULT_SEGMENT_BYTES,
            maximum_frame_bytes: MAXIMUM_FRAME_BYTES,
            direct_alignment: 4096,
            maximum_io_bytes: usize::MAX,
            backend: WriteBackend::Auto,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppendRequest {
    pub kind: EventKind,
    pub wall_timestamp_ns: UtcNanos,
    pub conversation: ConversationId,
    pub sealed_payload: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommitResult {
    pub first_lsn: LSN,
    pub last_lsn: LSN,
    pub backend: WriteBackend,
    pub segment_count: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecoveryReport {
    pub next_lsn: LSN,
    pub recovered_tail_lsn: LSN,
    pub segment_count: u32,
    pub truncated_torn_tail: bool,
    pub rebuilt_manifest: bool,
}

impl Default for RecoveryReport {
    fn default() -> Self {
        Self {
            next_lsn: LSN::new(1),
            recovered_tail_lsn: LSN::new(0),
            segment_count: 0,
            truncated_torn_tail: false,
            rebuilt_manifest: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SegmentEntry {
    id: u64,
    first_lsn: u64,
    last_lsn: u64,
    physical_bytes: u64,
}

pub struct SegmentLog {
    log_directory: PathBuf,
    actor: ActorId,
    options: SegmentLogOptions,
    segments: Vec<SegmentEntry>,
    recovery_report: RecoveryReport,
    next_lsn: u64,
}

impl SegmentLog {
    pub fn open(
        actor_directory: impl AsRef<Path>,
        actor: ActorId,
        options: SegmentLogOptions,
    ) -> Result<Self, Error> {
        if actor.get() == 0 || !valid_options(options) {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let log_directory = actor_directory.as_ref().join("log");
        fs::create_dir_all(&log_directory)
            .map_err(|error| io_error(ErrorCode::OpenFailed, &error))?;
        let mut paths = segment_paths(&log_directory)?;
        paths.sort_unstable_by_key(|(id, _)| *id);
        for pair in paths.windows(2) {
            if pair[1].0 != pair[0].0 + 1 {
                return Err(Error::new(ErrorCode::SequenceViolation));
            }
        }

        let mut segments = Vec::with_capacity(paths.len());
        let mut next_lsn = 1_u64;
        let mut report = RecoveryReport::default();
        for (index, (id, path)) in paths.iter().enumerate() {
            let physical_tail = index + 1 == paths.len();
            let scan = scan_segment(
                path,
                *id,
                actor,
                next_lsn,
                options,
                physical_tail,
                true,
                false,
            )?;
            if scan.truncated {
                report.truncated_torn_tail = true;
                report.recovered_tail_lsn = LSN::new(scan.recovered_lsn);
            }
            next_lsn = scan.next_lsn;
            segments.push(scan.entry);
        }
        report.next_lsn = LSN::new(next_lsn);
        report.segment_count =
            u32::try_from(segments.len()).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;

        let canonical_manifest = encode_manifest(&segments)?;
        let manifest_path = log_directory.join(MANIFEST_NAME);
        let manifest_matches = match fs::read(&manifest_path) {
            Ok(existing) => existing == canonical_manifest,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => return Err(io_error(ErrorCode::ReadFailed, &error)),
        };
        report.rebuilt_manifest = !manifest_matches;
        let mut log = Self {
            log_directory,
            actor,
            options,
            segments,
            recovery_report: report,
            next_lsn,
        };
        if !manifest_matches {
            log.write_manifest()?;
        }
        sync_directory(&log.log_directory)?;
        Ok(log)
    }

    #[must_use]
    pub fn recovery_report(&self) -> &RecoveryReport {
        &self.recovery_report
    }

    #[must_use]
    pub fn next_lsn(&self) -> LSN {
        LSN::new(self.next_lsn)
    }

    #[allow(clippy::too_many_lines)]
    pub fn append_batch(&mut self, requests: &[AppendRequest]) -> Result<CommitResult, Error> {
        if requests.is_empty() {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        if self.options.backend == WriteBackend::IoUringDirect {
            return Err(Error::new(ErrorCode::BackendUnavailable));
        }
        let mut encoded = Vec::with_capacity(requests.len());
        for (index, request) in requests.iter().enumerate() {
            let lsn = self
                .next_lsn
                .checked_add(index as u64)
                .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?;
            let frame = Frame {
                header: FrameHeader {
                    lsn: LSN::new(lsn),
                    kind: request.kind,
                    wall_timestamp_ns: request.wall_timestamp_ns,
                    actor: self.actor,
                    conversation: request.conversation,
                },
                sealed_payload: request.sealed_payload.clone(),
            };
            let bytes = frame::encode(&frame)?;
            if bytes.len() > self.options.maximum_frame_bytes {
                return Err(Error::new(ErrorCode::InvalidLength).at_lsn(frame.header.lsn));
            }
            encoded.push(bytes);
        }

        let first_lsn = self.next_lsn;
        let mut request_index = 0_usize;
        let mut touched_segments = 0_u32;
        while request_index < encoded.len() {
            self.ensure_segment();
            let last_index = self.segments.len() - 1;
            let mut start = align_up(
                self.segments[last_index].physical_bytes,
                u64::from(self.options.direct_alignment),
            )?;
            if start >= self.options.segment_bytes {
                self.push_segment()?;
                start = 0;
            }
            let last_index = self.segments.len() - 1;
            let mut group_end = request_index;
            let mut logical_bytes = 0_u64;
            while group_end < encoded.len() {
                let candidate = logical_bytes
                    .checked_add(encoded[group_end].len() as u64)
                    .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?;
                let physical = align_up(candidate, u64::from(self.options.direct_alignment))?;
                if start + physical > self.options.segment_bytes {
                    break;
                }
                logical_bytes = candidate;
                group_end += 1;
            }
            if group_end == request_index {
                if self.segments[last_index].first_lsn == 0 {
                    return Err(Error::new(ErrorCode::SegmentFull)
                        .at_lsn(LSN::new(first_lsn + request_index as u64)));
                }
                self.push_segment()?;
                continue;
            }
            let physical_bytes = usize::try_from(align_up(
                logical_bytes,
                u64::from(self.options.direct_alignment),
            )?)
            .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
            let gap = usize::try_from(start - self.segments[last_index].physical_bytes)
                .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
            let mut group = vec![0_u8; gap + physical_bytes];
            let mut cursor = gap;
            for bytes in &encoded[request_index..group_end] {
                group[cursor..cursor + bytes.len()].copy_from_slice(bytes);
                cursor += bytes.len();
            }
            let write_offset = self.segments[last_index].physical_bytes;
            let path = segment_path(&self.log_directory, self.segments[last_index].id);
            write_group(&path, &group, write_offset, self.options.maximum_io_bytes)?;
            let segment = &mut self.segments[last_index];
            if segment.first_lsn == 0 {
                segment.first_lsn = first_lsn + request_index as u64;
            }
            segment.last_lsn = first_lsn + group_end as u64 - 1;
            segment.physical_bytes = start + physical_bytes as u64;
            touched_segments = touched_segments
                .checked_add(1)
                .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?;
            request_index = group_end;
        }

        self.next_lsn = self
            .next_lsn
            .checked_add(requests.len() as u64)
            .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?;
        self.recovery_report.next_lsn = LSN::new(self.next_lsn);
        self.recovery_report.segment_count = u32::try_from(self.segments.len())
            .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
        self.write_manifest()?;
        sync_directory(&self.log_directory)?;
        Ok(CommitResult {
            first_lsn: LSN::new(first_lsn),
            last_lsn: LSN::new(self.next_lsn - 1),
            backend: WriteBackend::Pwrite,
            segment_count: touched_segments,
        })
    }

    pub fn read_all(&self) -> Result<Vec<Frame>, Error> {
        self.read_from(LSN::new(1), usize::MAX)
    }

    pub fn read_from(&self, first_lsn: LSN, maximum_frames: usize) -> Result<Vec<Frame>, Error> {
        if first_lsn.get() == 0 || maximum_frames == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let mut frames = Vec::new();
        let mut expected = 1_u64;
        for (index, segment) in self.segments.iter().enumerate() {
            let scan = scan_segment(
                &segment_path(&self.log_directory, segment.id),
                segment.id,
                self.actor,
                expected,
                self.options,
                index + 1 == self.segments.len(),
                false,
                true,
            )?;
            expected = scan.next_lsn;
            for frame in scan.frames {
                if frame.header.lsn.get() >= first_lsn.get() && frames.len() < maximum_frames {
                    frames.push(frame);
                }
            }
            if frames.len() == maximum_frames {
                break;
            }
        }
        Ok(frames)
    }

    pub fn truncate_to(&mut self, last_lsn: LSN) -> Result<(), Error> {
        if self.next_lsn == 1 || last_lsn.get() >= self.next_lsn - 1 {
            return Ok(());
        }
        let retained: Vec<Frame> = self
            .read_all()?
            .into_iter()
            .filter(|frame| frame.header.lsn.get() <= last_lsn.get())
            .collect();
        for segment in &self.segments {
            fs::remove_file(segment_path(&self.log_directory, segment.id))
                .map_err(|error| io_error(ErrorCode::WriteFailed, &error))?;
        }
        self.segments.clear();
        self.next_lsn = 1;
        if retained.is_empty() {
            self.recovery_report.next_lsn = LSN::new(1);
            self.recovery_report.segment_count = 0;
            self.write_manifest()?;
            sync_directory(&self.log_directory)?;
        } else {
            let requests: Vec<AppendRequest> = retained
                .into_iter()
                .map(|frame| AppendRequest {
                    kind: frame.header.kind,
                    wall_timestamp_ns: frame.header.wall_timestamp_ns,
                    conversation: frame.header.conversation,
                    sealed_payload: frame.sealed_payload,
                })
                .collect();
            self.append_batch(&requests)?;
        }
        Ok(())
    }

    fn ensure_segment(&mut self) {
        if self.segments.is_empty() {
            self.segments.push(SegmentEntry {
                id: 1,
                first_lsn: 0,
                last_lsn: 0,
                physical_bytes: 0,
            });
        }
    }

    fn push_segment(&mut self) -> Result<(), Error> {
        let id = self
            .segments
            .last()
            .map_or(1, |entry| entry.id.saturating_add(1));
        if id == u64::MAX {
            return Err(Error::new(ErrorCode::CapacityExceeded));
        }
        self.segments.push(SegmentEntry {
            id,
            first_lsn: 0,
            last_lsn: 0,
            physical_bytes: 0,
        });
        Ok(())
    }

    fn write_manifest(&mut self) -> Result<(), Error> {
        let bytes = encode_manifest(&self.segments)?;
        let path = self.log_directory.join(MANIFEST_NAME);
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .mode(0o600)
            .open(&path)
            .map_err(|error| io_error(ErrorCode::OpenFailed, &error))?;
        pwrite_all(&file, &bytes, 0, self.options.maximum_io_bytes)?;
        file.set_len(bytes.len() as u64)
            .map_err(|error| io_error(ErrorCode::WriteFailed, &error))?;
        file.sync_data()
            .map_err(|error| io_error(ErrorCode::SyncFailed, &error))
    }
}

struct ScanResult {
    entry: SegmentEntry,
    next_lsn: u64,
    recovered_lsn: u64,
    truncated: bool,
    frames: Vec<Frame>,
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn scan_segment(
    path: &Path,
    id: u64,
    actor: ActorId,
    expected_lsn: u64,
    options: SegmentLogOptions,
    physical_tail: bool,
    recover_tail: bool,
    collect_frames: bool,
) -> Result<ScanResult, Error> {
    let bytes = fs::read(path).map_err(|error| io_error(ErrorCode::ReadFailed, &error))?;
    if bytes.len() as u64 > options.segment_bytes {
        return Err(Error::new(ErrorCode::InteriorCorruption));
    }
    let mut offset = 0_usize;
    let mut next_lsn = expected_lsn;
    let mut first_lsn = 0_u64;
    let mut last_lsn = 0_u64;
    let mut frames = Vec::new();
    let mut truncate_at = None;

    while offset < bytes.len() {
        if bytes.len() - offset < 4 {
            if bytes[offset..].iter().all(|byte| *byte == 0) {
                break;
            }
            truncate_at = recoverable_tail(physical_tail, recover_tail, offset, next_lsn)?;
            break;
        }
        let declared = frame::encoded_frame_length(&bytes[offset..]).unwrap_or_default();
        if declared == 0 {
            let next = usize::try_from(align_up(
                offset as u64 + 1,
                u64::from(options.direct_alignment),
            )?)
            .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
            let padding_end = next.min(bytes.len());
            if bytes[offset..padding_end].iter().any(|byte| *byte != 0) {
                return Err(interior(next_lsn, offset));
            }
            offset = next;
            continue;
        }
        let end = offset.checked_add(declared);
        if declared < FRAME_HEADER_SIZE
            || declared > options.maximum_frame_bytes
            || end.is_none()
            || end.is_some_and(|value| value > bytes.len())
        {
            if has_valid_later(
                &bytes,
                offset + 1,
                actor,
                next_lsn,
                options.maximum_frame_bytes,
            ) {
                return Err(interior(next_lsn, offset));
            }
            truncate_at = recoverable_tail(physical_tail, recover_tail, offset, next_lsn)?;
            break;
        }
        let end = end.ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?;
        let decoded = frame::decode(&bytes[offset..end]);
        let frame = match decoded {
            Ok(frame) => frame,
            Err(error) => {
                if has_valid_later(
                    &bytes,
                    offset + 1,
                    actor,
                    next_lsn,
                    options.maximum_frame_bytes,
                ) {
                    return Err(interior(next_lsn, offset));
                }
                if error.code == ErrorCode::ChecksumMismatch
                    && bytes[end..].iter().any(|byte| *byte != 0)
                {
                    return Err(interior(next_lsn, offset));
                }
                truncate_at = recoverable_tail(physical_tail, recover_tail, offset, next_lsn)?;
                break;
            }
        };
        if frame.header.actor != actor {
            return Err(Error::new(ErrorCode::SequenceViolation)
                .at_lsn(frame.header.lsn)
                .at_offset(offset as u64));
        }
        if frame.header.lsn.get() != next_lsn {
            return Err(Error::new(ErrorCode::SequenceViolation)
                .at_lsn(frame.header.lsn)
                .at_offset(offset as u64));
        }
        if first_lsn == 0 {
            first_lsn = next_lsn;
        }
        last_lsn = next_lsn;
        next_lsn += 1;
        if collect_frames {
            frames.push(frame);
        }
        offset = end;
    }

    let mut physical_bytes = bytes.len() as u64;
    let mut truncated = false;
    let mut recovered_lsn = 0;
    if let Some(truncate_offset) = truncate_at {
        let file = OpenOptions::new()
            .write(true)
            .open(path)
            .map_err(|error| io_error(ErrorCode::OpenFailed, &error))?;
        file.set_len(truncate_offset as u64)
            .map_err(|error| io_error(ErrorCode::WriteFailed, &error))?;
        file.sync_data()
            .map_err(|error| io_error(ErrorCode::SyncFailed, &error))?;
        physical_bytes = truncate_offset as u64;
        truncated = true;
        recovered_lsn = next_lsn;
    }

    Ok(ScanResult {
        entry: SegmentEntry {
            id,
            first_lsn,
            last_lsn,
            physical_bytes,
        },
        next_lsn,
        recovered_lsn,
        truncated,
        frames,
    })
}

fn recoverable_tail(
    physical_tail: bool,
    recover_tail: bool,
    offset: usize,
    lsn: u64,
) -> Result<Option<usize>, Error> {
    if physical_tail && recover_tail {
        Ok(Some(offset))
    } else {
        Err(interior(lsn, offset))
    }
}

fn has_valid_later(
    bytes: &[u8],
    start: usize,
    actor: ActorId,
    expected_lsn: u64,
    maximum: usize,
) -> bool {
    let last = bytes.len().saturating_sub(FRAME_HEADER_SIZE);
    for offset in start..=last {
        let Some(length) = frame::encoded_frame_length(&bytes[offset..]) else {
            continue;
        };
        if !(FRAME_HEADER_SIZE..=maximum).contains(&length) || offset + length > bytes.len() {
            continue;
        }
        if frame::decode(&bytes[offset..offset + length]).is_ok_and(|candidate| {
            candidate.header.actor == actor && candidate.header.lsn.get() > expected_lsn
        }) {
            return true;
        }
    }
    false
}

fn interior(lsn: u64, offset: usize) -> Error {
    Error::new(ErrorCode::InteriorCorruption)
        .at_lsn(LSN::new(lsn))
        .at_offset(offset as u64)
}

fn valid_options(options: SegmentLogOptions) -> bool {
    options.maximum_frame_bytes >= FRAME_HEADER_SIZE
        && options.maximum_frame_bytes <= MAXIMUM_FRAME_BYTES
        && options.maximum_io_bytes > 0
        && (512..=4096).contains(&options.direct_alignment)
        && options.direct_alignment.is_power_of_two()
        && options.segment_bytes >= u64::from(options.direct_alignment)
        && options
            .segment_bytes
            .is_multiple_of(u64::from(options.direct_alignment))
        && options.maximum_frame_bytes as u64 <= options.segment_bytes
        && u32::try_from(options.segment_bytes).is_ok()
}

fn segment_paths(directory: &Path) -> Result<Vec<(u64, PathBuf)>, Error> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(directory).map_err(|error| io_error(ErrorCode::ReadFailed, &error))? {
        let entry = entry.map_err(|error| io_error(ErrorCode::ReadFailed, &error))?;
        let file_type = entry
            .file_type()
            .map_err(|error| io_error(ErrorCode::ReadFailed, &error))?;
        if !file_type.is_file() {
            continue;
        }
        let filename = entry.file_name();
        let filename = filename.to_string_lossy();
        if !filename.ends_with(".seg") {
            continue;
        }
        if filename.len() != 24 || !filename[..20].bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let id = filename[..20]
            .parse::<u64>()
            .map_err(|_| Error::new(ErrorCode::InvalidArgument))?;
        if id == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        paths.push((id, entry.path()));
    }
    Ok(paths)
}

fn segment_path(directory: &Path, id: u64) -> PathBuf {
    directory.join(format!("{id:020}.seg"))
}

fn write_group(path: &Path, bytes: &[u8], offset: u64, maximum_io: usize) -> Result<(), Error> {
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .read(true)
        .mode(0o600)
        .open(path)
        .map_err(|error| io_error(ErrorCode::OpenFailed, &error))?;
    pwrite_all(&file, bytes, offset, maximum_io)?;
    file.sync_data()
        .map_err(|error| io_error(ErrorCode::SyncFailed, &error))
}

fn pwrite_all(file: &File, bytes: &[u8], offset: u64, maximum_io: usize) -> Result<(), Error> {
    let mut written = 0_usize;
    while written < bytes.len() {
        let count = (bytes.len() - written).min(maximum_io);
        let write_offset = offset
            .checked_add(written as u64)
            .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?;
        let actual = file
            .write_at(&bytes[written..written + count], write_offset)
            .map_err(|error| io_error(ErrorCode::WriteFailed, &error).at_offset(write_offset))?;
        if actual == 0 {
            return Err(Error::new(ErrorCode::WriteFailed).at_offset(write_offset));
        }
        written += actual;
    }
    Ok(())
}

fn encode_manifest(segments: &[SegmentEntry]) -> Result<Vec<u8>, Error> {
    let total = MANIFEST_HEADER_BYTES
        .checked_add(
            MANIFEST_ENTRY_BYTES
                .checked_mul(segments.len())
                .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?,
        )
        .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?;
    let mut bytes = vec![0_u8; total];
    bytes[..4].copy_from_slice(MANIFEST_MAGIC);
    bytes[4..8].copy_from_slice(&MANIFEST_VERSION.to_le_bytes());
    bytes[8..12].copy_from_slice(
        &u32::try_from(segments.len())
            .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?
            .to_le_bytes(),
    );
    for (index, segment) in segments.iter().enumerate() {
        let offset = MANIFEST_HEADER_BYTES + index * MANIFEST_ENTRY_BYTES;
        bytes[offset..offset + 8].copy_from_slice(&segment.id.to_le_bytes());
        bytes[offset + 8..offset + 16].copy_from_slice(&segment.first_lsn.to_le_bytes());
        bytes[offset + 16..offset + 24].copy_from_slice(&segment.last_lsn.to_le_bytes());
        bytes[offset + 24..offset + 32].copy_from_slice(&segment.physical_bytes.to_le_bytes());
    }
    let checksum = crc32c::crc32c(&bytes);
    bytes[12..16].copy_from_slice(&checksum.to_le_bytes());
    Ok(bytes)
}

fn align_up(value: u64, alignment: u64) -> Result<u64, Error> {
    value
        .checked_add(alignment - 1)
        .map(|sum| sum & !(alignment - 1))
        .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))
}
