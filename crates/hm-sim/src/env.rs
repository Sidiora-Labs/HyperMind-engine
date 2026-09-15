#![allow(clippy::missing_errors_doc)]

use crate::fault::{FaultPlan, next_random};
use hm_core::{Error, ErrorCode, LSN, UtcNanos};
use hm_ledger::apply::{Clock, Storage, StorageCommit};
use hm_ledger::frame::{self, FRAME_HEADER_SIZE, Frame, MAXIMUM_FRAME_BYTES};
use hm_ledger::keyring::EntropySource;

pub struct SimClock {
    next_ns: i64,
}

impl SimClock {
    #[must_use]
    pub const fn new(initial_ns: i64) -> Self {
        Self {
            next_ns: initial_ns,
        }
    }
}

impl Default for SimClock {
    fn default() -> Self {
        Self::new(1_000_000)
    }
}

impl Clock for SimClock {
    fn now_ns(&mut self) -> Result<UtcNanos, Error> {
        let result = self.next_ns;
        self.next_ns = self
            .next_ns
            .checked_add(1_000)
            .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))?;
        Ok(UtcNanos::new(result))
    }
}

pub struct SimEntropy {
    state: u64,
}

impl SimEntropy {
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn fill(&mut self, output: &mut [u8]) {
        if self.state == 0 {
            self.state = 0x9e37_79b9_7f4a_7c15;
        }
        for byte in output {
            *byte = next_random(&mut self.state).to_le_bytes()[0];
        }
    }
}

impl EntropySource for SimEntropy {
    fn fill(&mut self, destination: &mut [u8]) -> Result<(), Error> {
        Self::fill(self, destination);
        Ok(())
    }
}

pub struct SimulatedStorage {
    plan: FaultPlan,
    durable_bytes: Vec<u8>,
    volatile_bytes: Vec<u8>,
    next_lsn: u64,
}

impl Default for SimulatedStorage {
    fn default() -> Self {
        Self {
            plan: FaultPlan::default(),
            durable_bytes: Vec::new(),
            volatile_bytes: Vec::new(),
            next_lsn: 1,
        }
    }
}

impl SimulatedStorage {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub const fn set_fault_plan(&mut self, plan: FaultPlan) {
        self.plan = plan;
    }

    pub fn crash(&mut self) {
        self.volatile_bytes.clear();
        if self.parse_durable(true).is_err() {
            self.next_lsn = 1;
        }
    }

    pub fn corrupt_durable_byte(&mut self, offset: usize, mask: u8) -> Result<(), Error> {
        let byte = self
            .durable_bytes
            .get_mut(offset)
            .ok_or_else(|| Error::new(ErrorCode::InvalidArgument).at_offset(offset as u64))?;
        *byte ^= mask;
        Ok(())
    }

    #[must_use]
    pub fn durable_bytes(&self) -> &[u8] {
        &self.durable_bytes
    }

    fn parse_durable(&mut self, repair_tail: bool) -> Result<Vec<Frame>, Error> {
        let mut frames = Vec::new();
        let mut offset = 0;
        let read_size = self.plan.maximum_read_bytes.max(1);
        while offset < self.durable_bytes.len() {
            let prefix_count = 4_usize.min(self.durable_bytes.len() - offset);
            let mut prefix = Vec::with_capacity(prefix_count);
            for read_offset in (0..prefix_count).step_by(read_size) {
                let count = read_size.min(prefix_count - read_offset);
                prefix.extend_from_slice(
                    &self.durable_bytes[offset + read_offset..offset + read_offset + count],
                );
            }
            let Some(length) = frame::encoded_frame_length(&prefix) else {
                if repair_tail {
                    self.durable_bytes.truncate(offset);
                    break;
                }
                return Err(Error::new(ErrorCode::Truncated).at_offset(offset as u64));
            };
            if !(FRAME_HEADER_SIZE..=MAXIMUM_FRAME_BYTES).contains(&length)
                || length > self.durable_bytes.len() - offset
            {
                if repair_tail {
                    self.durable_bytes.truncate(offset);
                    break;
                }
                return Err(Error::new(ErrorCode::Truncated).at_offset(offset as u64));
            }
            let mut encoded = Vec::with_capacity(length);
            for read_offset in (0..length).step_by(read_size) {
                let count = read_size.min(length - read_offset);
                encoded.extend_from_slice(
                    &self.durable_bytes[offset + read_offset..offset + read_offset + count],
                );
            }
            match frame::decode(&encoded) {
                Ok(frame) => {
                    let expected_lsn = frames.len() as u64 + 1;
                    if frame.header.lsn.get() != expected_lsn {
                        return Err(Error::new(ErrorCode::SequenceViolation)
                            .at_lsn(frame.header.lsn)
                            .at_offset(offset as u64));
                    }
                    frames.push(frame);
                    offset += length;
                }
                Err(error) => {
                    let final_frame = offset + length == self.durable_bytes.len();
                    if repair_tail && final_frame {
                        self.durable_bytes.truncate(offset);
                        break;
                    }
                    return Err(Error::new(ErrorCode::InteriorCorruption)
                        .with_system_error(error.system_error)
                        .at_lsn(error.lsn)
                        .at_offset(offset as u64));
                }
            }
        }
        self.next_lsn = frames.len() as u64 + 1;
        Ok(frames)
    }
}

impl Storage for SimulatedStorage {
    fn append(&mut self, frames: &[Frame]) -> Result<StorageCommit, Error> {
        if frames.is_empty() {
            return Err(Error::new(ErrorCode::InvalidLength));
        }
        let mut encoded_batch = Vec::new();
        let mut expected_lsn = self.next_lsn;
        for frame in frames {
            if frame.header.lsn.get() != expected_lsn {
                return Err(Error::new(ErrorCode::SequenceViolation).at_lsn(frame.header.lsn));
            }
            encoded_batch.extend_from_slice(&frame::encode(frame)?);
            expected_lsn = expected_lsn
                .checked_add(1)
                .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))?;
        }

        let chunk_size = self.plan.maximum_write_bytes.max(1);
        let mut completed = vec![0; encoded_batch.len()];
        let mut offsets: Vec<_> = (0..encoded_batch.len()).step_by(chunk_size).collect();
        if self.plan.reverse_write_completion {
            offsets.reverse();
        }
        for offset in offsets {
            let count = chunk_size.min(encoded_batch.len() - offset);
            completed[offset..offset + count]
                .copy_from_slice(&encoded_batch[offset..offset + count]);
        }

        let durable_count = self.plan.torn_after_bytes.min(completed.len());
        if durable_count != completed.len() {
            self.durable_bytes
                .extend_from_slice(&completed[..durable_count]);
            self.volatile_bytes.clear();
            return Err(Error::new(ErrorCode::ProcessKilled)
                .at_lsn(frames[0].header.lsn)
                .at_offset(durable_count as u64));
        }

        self.volatile_bytes.extend_from_slice(&completed);
        self.next_lsn = expected_lsn;
        if !self.plan.fsync_lies {
            self.durable_bytes.extend_from_slice(&self.volatile_bytes);
            self.volatile_bytes.clear();
        }
        let commit = StorageCommit {
            first_lsn: frames[0].header.lsn,
            last_lsn: frames[frames.len() - 1].header.lsn,
        };
        if !self.plan.fsync_lies
            && (commit.first_lsn.get()..=commit.last_lsn.get())
                .contains(&self.plan.kill_after_durable_lsn)
        {
            return Err(Error::new(ErrorCode::ProcessKilled)
                .at_lsn(LSN::new(self.plan.kill_after_durable_lsn)));
        }
        Ok(commit)
    }

    fn recover(&mut self) -> Result<Vec<Frame>, Error> {
        self.parse_durable(true)
    }

    fn read_from(&mut self, first_lsn: LSN, maximum_frames: usize) -> Result<Vec<Frame>, Error> {
        if first_lsn.get() == 0 || maximum_frames == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let mut frames = self.parse_durable(false)?;
        if first_lsn.get() > frames.len() as u64 {
            return Ok(Vec::new());
        }
        let begin = usize::try_from(first_lsn.get() - 1)
            .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
        let end = (begin + maximum_frames).min(frames.len());
        Ok(frames.drain(begin..end).collect())
    }
}
