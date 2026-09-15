#![allow(clippy::missing_errors_doc)]

use crate::checkpoint::verify_frame;
use crate::store::{KeyValue, Mutation, ProjectionId, ProjectionStore, ReadSnapshot};
use hm_core::{ConversationId, Error, ErrorCode, LSN};
use hm_ledger::frame::{EventKind, Frame};
use hm_schema::events::{EventPayload, LoopCloseReason};

const OBJECTIVE_PREFIX: u8 = b'I';
const OPEN_PREFIX: u8 = b'O';
const CLOSED_PREFIX: u8 = b'C';
const LOOP_INDEX_PREFIX: u8 = b'L';
pub const MAXIMUM_OPEN_LOOPS: usize = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum LoopClosure {
    Done = 0,
    Abandoned = 1,
    HandedOff = 2,
    Superseded = 3,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntentObjective {
    pub set_lsn: LSN,
    pub content: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenLoop {
    pub opened_lsn: LSN,
    pub loop_id: Vec<u8>,
    pub objective: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClosedLoop {
    pub opened_lsn: LSN,
    pub closed_lsn: LSN,
    pub reason: LoopClosure,
    pub loop_id: Vec<u8>,
    pub objective: Vec<u8>,
    pub cause: Vec<u8>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IntentFrameView {
    pub objective: Option<IntentObjective>,
    pub open_loops: Vec<OpenLoop>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntentRebuildProgress {
    pub applied_lsn: LSN,
    pub applied_frames: usize,
    pub complete: bool,
}

pub struct IntentFrameProjection;

impl IntentFrameProjection {
    pub fn apply_event(store: &ProjectionStore, frame: &Frame) -> Result<(), Error> {
        if !matches!(
            frame.header.kind,
            EventKind::IntentSet | EventKind::LoopOpened | EventKind::LoopClosed
        ) {
            return store.apply(ProjectionId::IntentFrame, frame.header.lsn, &[]);
        }
        let snapshot = store.begin_snapshot()?;
        let envelope = verify_frame(&snapshot, frame)?;
        let mut mutations = Vec::new();
        match (frame.header.kind, envelope.payload) {
            (EventKind::IntentSet, EventPayload::IntentSet(value)) => {
                mutations.push(Mutation::put(
                    conversation_key(OBJECTIVE_PREFIX, frame.header.conversation, &[]),
                    encode_objective(frame.header.lsn, &value.objective),
                ));
            }
            (EventKind::LoopOpened, EventPayload::LoopOpened(value)) => {
                let index_key = loop_index_key(&value.loop_id);
                if snapshot
                    .get(ProjectionId::IntentFrame, &index_key)?
                    .is_some()
                {
                    return Err(Error::new(ErrorCode::AlreadyExists).at_lsn(frame.header.lsn));
                }
                let prefix = conversation_key(OPEN_PREFIX, frame.header.conversation, &[]);
                if snapshot
                    .scan_prefix(ProjectionId::IntentFrame, &prefix, MAXIMUM_OPEN_LOOPS)?
                    .len()
                    >= MAXIMUM_OPEN_LOOPS
                {
                    return Err(Error::new(ErrorCode::InvariantViolation).at_lsn(frame.header.lsn));
                }
                let primary =
                    conversation_key(OPEN_PREFIX, frame.header.conversation, &value.loop_id);
                mutations.push(Mutation::put(
                    primary,
                    encode_objective(frame.header.lsn, &value.objective),
                ));
                mutations.push(Mutation::put(
                    index_key,
                    encode_loop_index(frame.header.conversation, frame.header.lsn, false),
                ));
            }
            (EventKind::LoopClosed, EventPayload::LoopClosed(value)) => {
                let index_key = loop_index_key(&value.loop_id);
                let index = snapshot
                    .get(ProjectionId::IntentFrame, &index_key)?
                    .ok_or_else(|| Error::new(ErrorCode::LoopNotFound).at_lsn(frame.header.lsn))?;
                let (conversation, _, closed) = decode_loop_index(&index)?;
                if closed {
                    return Err(Error::new(ErrorCode::LoopNotFound).at_lsn(frame.header.lsn));
                }
                if conversation != frame.header.conversation {
                    return Err(Error::new(ErrorCode::OrderingViolation).at_lsn(frame.header.lsn));
                }
                let open_key = conversation_key(OPEN_PREFIX, conversation, &value.loop_id);
                let encoded_open = snapshot
                    .get(ProjectionId::IntentFrame, &open_key)?
                    .ok_or_else(|| {
                        Error::new(ErrorCode::IntentFrameCorrupt).at_lsn(frame.header.lsn)
                    })?;
                let open = decode_open(&KeyValue {
                    key: open_key.clone(),
                    value: encoded_open,
                })?;
                let reason = closure(value.reason);
                mutations.push(Mutation::delete(open_key));
                mutations.push(Mutation::put(
                    conversation_key(CLOSED_PREFIX, conversation, &value.loop_id),
                    encode_closed(&open, frame.header.lsn, reason, &value.cause),
                ));
                mutations.push(Mutation::put(
                    index_key,
                    encode_loop_index(conversation, open.opened_lsn, true),
                ));
            }
            _ => {
                return Err(Error::new(ErrorCode::InvariantViolation).at_lsn(frame.header.lsn));
            }
        }
        drop(snapshot);
        store.apply(ProjectionId::IntentFrame, frame.header.lsn, &mutations)
    }

    pub fn rebuild(
        store: &ProjectionStore,
        frames: &[Frame],
        reset: bool,
        maximum_frames: usize,
    ) -> Result<IntentRebuildProgress, Error> {
        if reset {
            store.reset(ProjectionId::IntentFrame)?;
        }
        let snapshot = store.begin_snapshot()?;
        let checkpoint = snapshot.checkpoint(ProjectionId::IntentFrame)?.get();
        drop(snapshot);
        if checkpoint > frames.len() as u64 {
            return Err(Error::new(ErrorCode::ProjectionCheckpoint).at_lsn(LSN::new(checkpoint)));
        }
        let start =
            usize::try_from(checkpoint).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
        let count = frames.len().saturating_sub(start).min(maximum_frames);
        for (offset, frame) in frames.iter().skip(start).take(count).enumerate() {
            let expected = checkpoint + offset as u64 + 1;
            if frame.header.lsn.get() != expected {
                return Err(Error::new(ErrorCode::SequenceViolation).at_lsn(frame.header.lsn));
            }
            Self::apply_event(store, frame)?;
        }
        let applied_lsn = checkpoint + count as u64;
        Ok(IntentRebuildProgress {
            applied_lsn: LSN::new(applied_lsn),
            applied_frames: count,
            complete: applied_lsn == frames.len() as u64,
        })
    }

    pub fn read(
        snapshot: &ReadSnapshot<'_>,
        conversation: ConversationId,
    ) -> Result<IntentFrameView, Error> {
        let objective_key = conversation_key(OBJECTIVE_PREFIX, conversation, &[]);
        let mut objective = snapshot
            .get(ProjectionId::IntentFrame, &objective_key)?
            .map(|value| decode_objective(&value))
            .transpose()?;
        if objective.is_none() && conversation != ConversationId::new([0; 16]) {
            objective = snapshot
                .get(
                    ProjectionId::IntentFrame,
                    &conversation_key(OBJECTIVE_PREFIX, ConversationId::new([0; 16]), &[]),
                )?
                .map(|value| decode_objective(&value))
                .transpose()?;
        }
        let prefix = conversation_key(OPEN_PREFIX, conversation, &[]);
        let mut open_loops = snapshot
            .scan_prefix(ProjectionId::IntentFrame, &prefix, MAXIMUM_OPEN_LOOPS)?
            .iter()
            .map(decode_open)
            .collect::<Result<Vec<_>, _>>()?;
        open_loops.sort_by_key(|item| item.opened_lsn);
        Ok(IntentFrameView {
            objective,
            open_loops,
        })
    }

    pub fn read_closed(
        snapshot: &ReadSnapshot<'_>,
        conversation: ConversationId,
    ) -> Result<Vec<ClosedLoop>, Error> {
        let prefix = conversation_key(CLOSED_PREFIX, conversation, &[]);
        let mut loops = snapshot
            .scan_prefix(ProjectionId::IntentFrame, &prefix, MAXIMUM_OPEN_LOOPS)?
            .iter()
            .map(decode_closed)
            .collect::<Result<Vec<_>, _>>()?;
        loops.sort_by_key(|item| item.closed_lsn);
        Ok(loops)
    }
}

fn conversation_key(prefix: u8, conversation: ConversationId, suffix: &[u8]) -> Vec<u8> {
    let mut key = Vec::with_capacity(17 + suffix.len());
    key.push(prefix);
    key.extend_from_slice(conversation.as_bytes());
    key.extend_from_slice(suffix);
    key
}

fn loop_index_key(loop_id: &[u8]) -> Vec<u8> {
    let mut key = Vec::with_capacity(1 + loop_id.len());
    key.push(LOOP_INDEX_PREFIX);
    key.extend_from_slice(loop_id);
    key
}

fn encode_objective(lsn: LSN, content: &[u8]) -> Vec<u8> {
    let mut value = Vec::with_capacity(16 + content.len());
    append_u64(&mut value, lsn.get());
    append_bytes(&mut value, content);
    value
}

fn decode_objective(value: &[u8]) -> Result<IntentObjective, Error> {
    let mut offset = 0;
    let set_lsn = LSN::new(read_u64(value, &mut offset)?);
    let content = read_bytes(value, &mut offset)?;
    if set_lsn.get() == 0 || offset != value.len() {
        return Err(Error::new(ErrorCode::IntentFrameCorrupt));
    }
    Ok(IntentObjective { set_lsn, content })
}

fn decode_open(item: &KeyValue) -> Result<OpenLoop, Error> {
    if item.key.len() <= 17 || item.key[0] != OPEN_PREFIX {
        return Err(Error::new(ErrorCode::IntentFrameCorrupt));
    }
    let objective = decode_objective(&item.value)?;
    Ok(OpenLoop {
        opened_lsn: objective.set_lsn,
        loop_id: item.key[17..].to_vec(),
        objective: objective.content,
    })
}

fn encode_closed(open: &OpenLoop, closed_lsn: LSN, reason: LoopClosure, cause: &[u8]) -> Vec<u8> {
    let mut value = Vec::new();
    append_u64(&mut value, open.opened_lsn.get());
    append_u64(&mut value, closed_lsn.get());
    value.push(reason as u8);
    append_bytes(&mut value, &open.objective);
    append_bytes(&mut value, cause);
    value
}

fn decode_closed(item: &KeyValue) -> Result<ClosedLoop, Error> {
    if item.key.len() <= 17 || item.key[0] != CLOSED_PREFIX {
        return Err(Error::new(ErrorCode::IntentFrameCorrupt));
    }
    let mut offset = 0;
    let opened_lsn = LSN::new(read_u64(&item.value, &mut offset)?);
    let closed_lsn = LSN::new(read_u64(&item.value, &mut offset)?);
    let reason = match item.value.get(offset).copied() {
        Some(0) => LoopClosure::Done,
        Some(1) => LoopClosure::Abandoned,
        Some(2) => LoopClosure::HandedOff,
        Some(3) => LoopClosure::Superseded,
        _ => return Err(Error::new(ErrorCode::IntentFrameCorrupt)),
    };
    offset += 1;
    let objective = read_bytes(&item.value, &mut offset)?;
    let cause = read_bytes(&item.value, &mut offset)?;
    if opened_lsn.get() == 0 || closed_lsn.get() == 0 || offset != item.value.len() {
        return Err(Error::new(ErrorCode::IntentFrameCorrupt));
    }
    Ok(ClosedLoop {
        opened_lsn,
        closed_lsn,
        reason,
        loop_id: item.key[17..].to_vec(),
        objective,
        cause,
    })
}

fn encode_loop_index(conversation: ConversationId, opened_lsn: LSN, closed: bool) -> Vec<u8> {
    let mut value = Vec::with_capacity(25);
    value.extend_from_slice(conversation.as_bytes());
    append_u64(&mut value, opened_lsn.get());
    value.push(u8::from(closed));
    value
}

fn decode_loop_index(value: &[u8]) -> Result<(ConversationId, LSN, bool), Error> {
    if value.len() != 25 || value[24] > 1 {
        return Err(Error::new(ErrorCode::IntentFrameCorrupt));
    }
    let conversation = ConversationId::new(
        value[..16]
            .try_into()
            .map_err(|_| Error::new(ErrorCode::IntentFrameCorrupt))?,
    );
    let opened_lsn = LSN::new(u64::from_le_bytes(
        value[16..24]
            .try_into()
            .map_err(|_| Error::new(ErrorCode::IntentFrameCorrupt))?,
    ));
    if opened_lsn.get() == 0 {
        return Err(Error::new(ErrorCode::IntentFrameCorrupt));
    }
    Ok((conversation, opened_lsn, value[24] == 1))
}

const fn closure(reason: LoopCloseReason) -> LoopClosure {
    match reason {
        LoopCloseReason::Done => LoopClosure::Done,
        LoopCloseReason::Abandoned => LoopClosure::Abandoned,
        LoopCloseReason::HandedOff => LoopClosure::HandedOff,
        LoopCloseReason::Superseded => LoopClosure::Superseded,
    }
}

fn append_u64(output: &mut Vec<u8>, value: u64) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn append_bytes(output: &mut Vec<u8>, value: &[u8]) {
    append_u64(output, value.len() as u64);
    output.extend_from_slice(value);
}

fn read_u64(bytes: &[u8], offset: &mut usize) -> Result<u64, Error> {
    let value = bytes
        .get(*offset..*offset + 8)
        .ok_or_else(|| Error::new(ErrorCode::IntentFrameCorrupt))?
        .try_into()
        .map(u64::from_le_bytes)
        .map_err(|_| Error::new(ErrorCode::IntentFrameCorrupt))?;
    *offset += 8;
    Ok(value)
}

fn read_bytes(bytes: &[u8], offset: &mut usize) -> Result<Vec<u8>, Error> {
    let length = usize::try_from(read_u64(bytes, offset)?)
        .map_err(|_| Error::new(ErrorCode::IntentFrameCorrupt))?;
    let value = bytes
        .get(*offset..*offset + length)
        .ok_or_else(|| Error::new(ErrorCode::IntentFrameCorrupt))?
        .to_vec();
    *offset += length;
    Ok(value)
}
