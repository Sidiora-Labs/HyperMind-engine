#![allow(clippy::missing_errors_doc)]

use crate::store::{KeyValue, Mutation, ProjectionId, ProjectionStore, ReadSnapshot};
use hm_core::{Error, ErrorCode, LSN};
use hm_ledger::frame::Frame;
use roaring::RoaringTreemap;
use std::io::Cursor;

const WINDOW_PREFIX: u8 = b'W';
const DURATIONS_NS: [i64; 4] = [
    60_000_000_000,
    3_600_000_000_000,
    86_400_000_000_000,
    604_800_000_000_000,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum TemporalLevel {
    Minute,
    Hour,
    Day,
    Week,
}

impl TemporalLevel {
    const fn index(self) -> usize {
        self as usize
    }

    fn child(self) -> Option<Self> {
        match self {
            Self::Minute => None,
            Self::Hour => Some(Self::Minute),
            Self::Day => Some(Self::Hour),
            Self::Week => Some(Self::Day),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TemporalWindow {
    pub level: TemporalLevel,
    pub start_ns: i64,
    pub end_ns: i64,
    pub member_count: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TemporalRebuildProgress {
    pub applied_lsn: LSN,
    pub applied_frames: usize,
    pub complete: bool,
}

pub struct TemporalLadder;

impl TemporalLadder {
    pub fn apply_event(store: &ProjectionStore, frame: &Frame) -> Result<(), Error> {
        let snapshot = store.begin_snapshot()?;
        let mut mutations = Vec::with_capacity(DURATIONS_NS.len());
        for level in [
            TemporalLevel::Minute,
            TemporalLevel::Hour,
            TemporalLevel::Day,
            TemporalLevel::Week,
        ] {
            let start_ns = window_start(frame.header.wall_timestamp_ns.get(), duration(level)?)?;
            let key = window_key(level, start_ns);
            let mut members = decode_bitmap(snapshot.get(ProjectionId::TemporalLadder, &key)?)?;
            members.insert(frame.header.lsn.get());
            mutations.push(Mutation::put(key, encode_bitmap(&members)?));
        }
        drop(snapshot);
        store.apply(ProjectionId::TemporalLadder, frame.header.lsn, &mutations)
    }

    pub fn rebuild(
        store: &ProjectionStore,
        frames: &[Frame],
        reset: bool,
        maximum_frames: usize,
    ) -> Result<TemporalRebuildProgress, Error> {
        if reset {
            store.reset(ProjectionId::TemporalLadder)?;
        }
        let checkpoint = store
            .begin_snapshot()?
            .checkpoint(ProjectionId::TemporalLadder)?
            .get();
        let begin = usize::try_from(checkpoint)
            .map_err(|_| Error::new(ErrorCode::CapacityExceeded).at_lsn(LSN::new(checkpoint)))?;
        if begin > frames.len() {
            return Err(Error::new(ErrorCode::ProjectionCheckpoint)
                .at_lsn(LSN::new(checkpoint))
                .at_offset(frames.len() as u64));
        }
        let apply_count = (frames.len() - begin).min(maximum_frames);
        for (offset, frame) in frames[begin..begin + apply_count].iter().enumerate() {
            let expected = checkpoint
                .checked_add(offset as u64)
                .and_then(|value| value.checked_add(1))
                .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?;
            if frame.header.lsn.get() != expected {
                return Err(Error::new(ErrorCode::SequenceViolation).at_lsn(frame.header.lsn));
            }
            Self::apply_event(store, frame)?;
        }
        let applied_lsn = checkpoint
            .checked_add(apply_count as u64)
            .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?;
        Ok(TemporalRebuildProgress {
            applied_lsn: LSN::new(applied_lsn),
            applied_frames: apply_count,
            complete: applied_lsn == frames.len() as u64,
        })
    }

    pub fn list_windows(
        snapshot: &ReadSnapshot<'_>,
        level: TemporalLevel,
        from_ns: i64,
        to_ns: i64,
        limit: usize,
    ) -> Result<Vec<TemporalWindow>, Error> {
        if from_ns >= to_ns || limit == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let first_start = window_start(from_ns, duration(level)?)?;
        let prefix = [WINDOW_PREFIX, level as u8];
        let items = snapshot.scan_prefix_from(
            ProjectionId::TemporalLadder,
            &prefix,
            &window_key(level, first_start),
            limit,
        )?;
        let mut windows = Vec::with_capacity(items.len());
        for item in items {
            let window = decode_window(&item, level)?;
            if window.start_ns >= to_ns {
                break;
            }
            if window.end_ns > from_ns {
                windows.push(window);
            }
        }
        Ok(windows)
    }

    pub fn open_window(
        snapshot: &ReadSnapshot<'_>,
        window: TemporalWindow,
        limit: usize,
    ) -> Result<Vec<TemporalWindow>, Error> {
        validate_window(window, limit)?;
        let Some(child) = window.level.child() else {
            return Ok(Vec::new());
        };
        Self::list_windows(snapshot, child, window.start_ns, window.end_ns, limit)
    }

    pub fn resolve_members(
        snapshot: &ReadSnapshot<'_>,
        window: TemporalWindow,
        limit: usize,
    ) -> Result<Vec<LSN>, Error> {
        validate_window(window, limit)?;
        let members = decode_bitmap(snapshot.get(
            ProjectionId::TemporalLadder,
            &window_key(window.level, window.start_ns),
        )?)?;
        Ok(members.iter().take(limit).map(LSN::new).collect())
    }
}

fn duration(level: TemporalLevel) -> Result<i64, Error> {
    DURATIONS_NS
        .get(level.index())
        .copied()
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))
}

fn window_start(timestamp_ns: i64, duration_ns: i64) -> Result<i64, Error> {
    timestamp_ns
        .div_euclid(duration_ns)
        .checked_mul(duration_ns)
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))
}

fn window_end(start_ns: i64, duration_ns: i64) -> Result<i64, Error> {
    start_ns
        .checked_add(duration_ns)
        .ok_or_else(|| Error::new(ErrorCode::TemporalLadderCorrupt))
}

fn window_key(level: TemporalLevel, start_ns: i64) -> [u8; 10] {
    let mut key = [0_u8; 10];
    key[0] = WINDOW_PREFIX;
    key[1] = level as u8;
    let sortable = u64::from_ne_bytes(start_ns.to_ne_bytes()) ^ (1_u64 << 63);
    key[2..].copy_from_slice(&sortable.to_be_bytes());
    key
}

fn decode_start(key: &[u8], level: TemporalLevel) -> Result<i64, Error> {
    if key.len() != 10 || key[0] != WINDOW_PREFIX || key[1] != level as u8 {
        return Err(Error::new(ErrorCode::TemporalLadderCorrupt));
    }
    let sortable = u64::from_be_bytes(
        key[2..]
            .try_into()
            .map_err(|_| Error::new(ErrorCode::TemporalLadderCorrupt))?,
    );
    Ok(i64::from_ne_bytes((sortable ^ (1_u64 << 63)).to_ne_bytes()))
}

fn decode_window(item: &KeyValue, level: TemporalLevel) -> Result<TemporalWindow, Error> {
    let start_ns = decode_start(&item.key, level)?;
    let end_ns = window_end(start_ns, duration(level)?)?;
    let members = decode_bitmap(Some(item.value.clone()))?;
    Ok(TemporalWindow {
        level,
        start_ns,
        end_ns,
        member_count: members.len(),
    })
}

fn validate_window(window: TemporalWindow, limit: usize) -> Result<(), Error> {
    if limit == 0
        || window.end_ns <= window.start_ns
        || window.end_ns - window.start_ns != duration(window.level)?
    {
        Err(Error::new(ErrorCode::InvalidArgument))
    } else {
        Ok(())
    }
}

fn decode_bitmap(value: Option<Vec<u8>>) -> Result<RoaringTreemap, Error> {
    match value {
        None => Ok(RoaringTreemap::new()),
        Some(bytes) => RoaringTreemap::deserialize_from(Cursor::new(bytes))
            .map_err(|_| Error::new(ErrorCode::TemporalLadderCorrupt)),
    }
}

fn encode_bitmap(bitmap: &RoaringTreemap) -> Result<Vec<u8>, Error> {
    let mut bytes = Vec::new();
    bitmap
        .serialize_into(&mut bytes)
        .map_err(|_| Error::new(ErrorCode::BackendUnavailable))?;
    Ok(bytes)
}
