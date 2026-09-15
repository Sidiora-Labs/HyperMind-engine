#![allow(clippy::missing_errors_doc)]

use crate::frame::{EventKind, Frame, FrameHeader};
use hm_core::{ActorId, ConversationId, Error, ErrorCode, LSN, UtcNanos};
use std::thread::ThreadId;

pub const EVENT_KIND_COUNT: usize = 21;
const RECOVERY_BATCH_SIZE: usize = 1_024;

pub trait Clock: Send {
    fn now_ns(&mut self) -> Result<UtcNanos, Error>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StorageCommit {
    pub first_lsn: LSN,
    pub last_lsn: LSN,
}

pub trait Storage: Send {
    fn append(&mut self, frames: &[Frame]) -> Result<StorageCommit, Error>;
    fn recover(&mut self) -> Result<Vec<Frame>, Error>;
    fn read_from(&mut self, first_lsn: LSN, maximum_frames: usize) -> Result<Vec<Frame>, Error>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplyRequest {
    pub kind: EventKind,
    pub conversation: ConversationId,
    pub plaintext_payload: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApplyAck {
    pub lsn: LSN,
    pub wall_timestamp_ns: UtcNanos,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppliedState {
    last_lsn: LSN,
    event_count: u64,
    kind_counts: [u64; EVENT_KIND_COUNT],
    digest: [u8; 32],
}

impl Default for AppliedState {
    fn default() -> Self {
        Self {
            last_lsn: LSN::new(0),
            event_count: 0,
            kind_counts: [0; EVENT_KIND_COUNT],
            digest: *blake3::hash(&[]).as_bytes(),
        }
    }
}

impl AppliedState {
    pub fn apply(&mut self, frame: &Frame) -> Result<(), Error> {
        let expected = self
            .last_lsn
            .get()
            .checked_add(1)
            .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))?;
        if frame.header.lsn.get() != expected {
            return Err(Error::new(ErrorCode::SequenceViolation).at_lsn(frame.header.lsn));
        }
        let kind_index = usize::from(frame.header.kind as u8 - 1);
        self.event_count = self
            .event_count
            .checked_add(1)
            .ok_or_else(|| Error::new(ErrorCode::InvariantViolation).at_lsn(frame.header.lsn))?;
        self.kind_counts[kind_index] = self.kind_counts[kind_index]
            .checked_add(1)
            .ok_or_else(|| Error::new(ErrorCode::InvariantViolation).at_lsn(frame.header.lsn))?;
        let frame_hash = plaintext_frame_hash(frame);
        let mut material = [0_u8; 65];
        material[0] = 0x53;
        material[1..33].copy_from_slice(&self.digest);
        material[33..].copy_from_slice(&frame_hash);
        self.digest = *blake3::hash(&material).as_bytes();
        self.last_lsn = frame.header.lsn;
        Ok(())
    }

    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut output = Vec::with_capacity(16 + EVENT_KIND_COUNT * 8 + 32);
        output.extend_from_slice(&self.last_lsn.get().to_le_bytes());
        output.extend_from_slice(&self.event_count.to_le_bytes());
        for count in self.kind_counts {
            output.extend_from_slice(&count.to_le_bytes());
        }
        output.extend_from_slice(&self.digest);
        output
    }

    #[must_use]
    pub const fn last_lsn(&self) -> LSN {
        self.last_lsn
    }

    #[must_use]
    pub const fn event_count(&self) -> u64 {
        self.event_count
    }

    #[must_use]
    pub const fn kind_counts(&self) -> &[u64; EVENT_KIND_COUNT] {
        &self.kind_counts
    }

    #[must_use]
    pub const fn digest(&self) -> &[u8; 32] {
        &self.digest
    }
}

pub struct ApplyLoop<'environment> {
    clock: &'environment mut dyn Clock,
    storage: &'environment mut dyn Storage,
    actor: ActorId,
    maximum_batch_size: usize,
    writer_thread: ThreadId,
    state: AppliedState,
}

impl<'environment> ApplyLoop<'environment> {
    pub fn boot(
        clock: &'environment mut dyn Clock,
        storage: &'environment mut dyn Storage,
        actor: ActorId,
        maximum_batch_size: usize,
    ) -> Result<Self, Error> {
        if actor.get() == 0 || maximum_batch_size == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        storage.recover()?;
        let mut state = AppliedState::default();
        let mut next_lsn = LSN::new(1);
        loop {
            let recovered = storage.read_from(next_lsn, RECOVERY_BATCH_SIZE)?;
            if recovered.is_empty() {
                break;
            }
            for frame in &recovered {
                if frame.header.actor != actor {
                    return Err(Error::new(ErrorCode::SequenceViolation).at_lsn(frame.header.lsn));
                }
                state.apply(frame)?;
            }
            if recovered.len() < RECOVERY_BATCH_SIZE {
                break;
            }
            next_lsn = LSN::new(
                recovered
                    .last()
                    .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))?
                    .header
                    .lsn
                    .get()
                    .checked_add(1)
                    .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))?,
            );
        }
        Ok(Self {
            clock,
            storage,
            actor,
            maximum_batch_size,
            writer_thread: std::thread::current().id(),
            state,
        })
    }

    pub fn apply_batch(&mut self, requests: &[ApplyRequest]) -> Result<Vec<ApplyAck>, Error> {
        if std::thread::current().id() != self.writer_thread {
            return Err(Error::new(ErrorCode::WriterViolation));
        }
        if requests.is_empty() || requests.len() > self.maximum_batch_size {
            return Err(Error::new(ErrorCode::InvalidLength));
        }
        let mut next_lsn = self
            .state
            .last_lsn
            .get()
            .checked_add(1)
            .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))?;
        let mut frames = Vec::with_capacity(requests.len());
        let mut acknowledgements = Vec::with_capacity(requests.len());
        for request in requests {
            let wall_timestamp_ns = self.clock.now_ns()?;
            frames.push(Frame {
                header: FrameHeader {
                    lsn: LSN::new(next_lsn),
                    kind: request.kind,
                    wall_timestamp_ns,
                    actor: self.actor,
                    conversation: request.conversation,
                },
                sealed_payload: request.plaintext_payload.clone(),
            });
            acknowledgements.push(ApplyAck {
                lsn: LSN::new(next_lsn),
                wall_timestamp_ns,
            });
            next_lsn = next_lsn
                .checked_add(1)
                .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))?;
        }
        let commit = self.storage.append(&frames)?;
        if commit.first_lsn != frames[0].header.lsn
            || commit.last_lsn
                != frames
                    .last()
                    .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))?
                    .header
                    .lsn
        {
            return Err(Error::new(ErrorCode::DurabilityFailure).at_lsn(commit.last_lsn));
        }
        for frame in &frames {
            self.state.apply(frame)?;
        }
        Ok(acknowledgements)
    }

    #[must_use]
    pub const fn state(&self) -> &AppliedState {
        &self.state
    }
}

fn plaintext_frame_hash(frame: &Frame) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&[0x03]);
    hasher.update(&frame.header.lsn.get().to_le_bytes());
    hasher.update(&[frame.header.kind as u8]);
    hasher.update(
        &frame
            .header
            .wall_timestamp_ns
            .get()
            .cast_unsigned()
            .to_le_bytes(),
    );
    hasher.update(&frame.header.actor.get().to_le_bytes());
    hasher.update(frame.header.conversation.as_bytes());
    hasher.update(&(frame.sealed_payload.len() as u64).to_le_bytes());
    hasher.update(&frame.sealed_payload);
    *hasher.finalize().as_bytes()
}
