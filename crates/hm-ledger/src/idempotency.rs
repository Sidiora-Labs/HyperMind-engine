#![allow(clippy::missing_errors_doc)]

use crate::frame::{EventKind, Frame};
use crate::keyring::KeyHierarchy;
use crate::segment::SegmentLog;
use hm_core::{ConversationId, Error, ErrorCode, LSN};
use hm_schema::event::{CURRENT_SCHEMA_VERSION, MAXIMUM_EVENT_BYTES};
use hm_schema::events::{EventEnvelope, EventEnvelopeRef};
use planus::ReadAsRoot;
use std::collections::BTreeMap;

pub const CONNECTION_ID_BYTES: usize = 16;
pub const MAXIMUM_BATCH_EVENTS: usize = 256;
pub const MAXIMUM_LOGICAL_CONNECTIONS: usize = 4_096;

pub type ConnectionId = [u8; CONNECTION_ID_BYTES];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BatchEvent<'payload> {
    pub kind: EventKind,
    pub conversation: ConversationId,
    pub plaintext_payload: &'payload [u8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BatchIdentity<'payload> {
    pub connection_id: ConnectionId,
    pub client_seq: u64,
    pub events: &'payload [BatchEvent<'payload>],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DedupState {
    pub client_seq: u64,
    pub first_lsn: LSN,
    pub last_lsn: LSN,
    pub digest: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Admission {
    Fresh { digest: [u8; 32] },
    Duplicate(DedupState),
}

#[derive(Clone, Debug)]
pub struct DedupTable {
    entries: BTreeMap<ConnectionId, DedupState>,
    maximum_connections: usize,
}

impl Default for DedupTable {
    fn default() -> Self {
        Self::new(MAXIMUM_LOGICAL_CONNECTIONS)
    }
}

impl DedupTable {
    #[must_use]
    pub const fn new(maximum_connections: usize) -> Self {
        Self {
            entries: BTreeMap::new(),
            maximum_connections,
        }
    }

    pub fn admit(&self, batch: &BatchIdentity<'_>) -> Result<Admission, Error> {
        validate_batch(batch)?;
        let digest = request_digest(batch.events)?;
        match self.entries.get(&batch.connection_id) {
            Some(prior) if batch.client_seq == prior.client_seq => {
                if digest == prior.digest {
                    Ok(Admission::Duplicate(*prior))
                } else {
                    Err(Error::new(ErrorCode::IdempotencyConflict))
                }
            }
            Some(prior)
                if prior
                    .client_seq
                    .checked_add(1)
                    .is_some_and(|next| batch.client_seq == next) =>
            {
                Ok(Admission::Fresh { digest })
            }
            None if batch.client_seq == 1 && self.entries.len() < self.maximum_connections => {
                Ok(Admission::Fresh { digest })
            }
            None if self.entries.len() >= self.maximum_connections => {
                Err(Error::new(ErrorCode::CapacityExceeded))
            }
            Some(_) | None => Err(Error::new(ErrorCode::SequenceViolation)),
        }
    }

    pub fn record(
        &mut self,
        batch: &BatchIdentity<'_>,
        first_lsn: LSN,
        last_lsn: LSN,
    ) -> Result<DedupState, Error> {
        let Admission::Fresh { digest } = self.admit(batch)? else {
            return Err(Error::new(ErrorCode::AlreadyExists));
        };
        let count = last_lsn
            .get()
            .checked_sub(first_lsn.get())
            .and_then(|difference| difference.checked_add(1))
            .ok_or_else(|| Error::new(ErrorCode::SequenceViolation))?;
        if count != batch.events.len() as u64 {
            return Err(Error::new(ErrorCode::SequenceViolation));
        }
        let state = DedupState {
            client_seq: batch.client_seq,
            first_lsn,
            last_lsn,
            digest,
        };
        self.entries.insert(batch.connection_id, state);
        Ok(state)
    }

    pub fn rebuild(plaintext_frames: &[Frame]) -> Result<Self, Error> {
        let mut table = Self::default();
        let mut index = 0;
        while index < plaintext_frames.len() {
            let first = &plaintext_frames[index];
            let envelope = read_envelope(&first.sealed_payload)?;
            let Some(connection_id) = parse_connection_id(envelope.connection_id.as_deref())?
            else {
                index += 1;
                continue;
            };
            let count = usize::try_from(envelope.client_event_count)
                .map_err(|_| Error::new(ErrorCode::IdempotencyConflict).at_lsn(first.header.lsn))?;
            if envelope.client_seq == 0
                || count == 0
                || count > MAXIMUM_BATCH_EVENTS
                || envelope.client_event_index != 0
                || index + count > plaintext_frames.len()
            {
                return Err(Error::new(ErrorCode::IdempotencyConflict).at_lsn(first.header.lsn));
            }
            let frames = &plaintext_frames[index..index + count];
            let mut events = Vec::with_capacity(count);
            for (event_index, frame) in frames.iter().enumerate() {
                let member = read_envelope(&frame.sealed_payload)?;
                let expected_index = u32::try_from(event_index)
                    .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
                let expected_count =
                    u32::try_from(count).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
                if parse_connection_id(member.connection_id.as_deref())? != Some(connection_id)
                    || member.client_seq != envelope.client_seq
                    || member.client_event_index != expected_index
                    || member.client_event_count != expected_count
                {
                    return Err(Error::new(ErrorCode::IdempotencyConflict).at_lsn(frame.header.lsn));
                }
                events.push(BatchEvent {
                    kind: frame.header.kind,
                    conversation: frame.header.conversation,
                    plaintext_payload: &frame.sealed_payload,
                });
            }
            let batch = BatchIdentity {
                connection_id,
                client_seq: envelope.client_seq,
                events: &events,
            };
            table.record(&batch, frames[0].header.lsn, frames[count - 1].header.lsn)?;
            index += count;
        }
        Ok(table)
    }

    #[must_use]
    pub fn get(&self, connection_id: &ConnectionId) -> Option<&DedupState> {
        self.entries.get(connection_id)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

pub fn rollback_torn_batch(
    log: &mut SegmentLog,
    keys: &KeyHierarchy,
) -> Result<Option<LSN>, Error> {
    let next = log.next_lsn().get();
    if next <= 1 {
        return Ok(None);
    }
    let tail_lsn = LSN::new(next - 1);
    let mut frames = log.read_from(tail_lsn, 1)?;
    let frame = frames
        .pop()
        .filter(|frame| frame.header.lsn == tail_lsn)
        .ok_or_else(|| Error::new(ErrorCode::ReadFailed).at_lsn(tail_lsn))?;
    let plaintext = keys.unseal(&frame.header, &frame.sealed_payload)?;
    let mut plaintext_frame = frame;
    plaintext_frame.sealed_payload = plaintext;
    let envelope = read_envelope(&plaintext_frame.sealed_payload)?;
    if parse_connection_id(envelope.connection_id.as_deref())?.is_none()
        || envelope.client_event_count == 0
    {
        return Ok(None);
    }
    let index = u64::from(envelope.client_event_index);
    let count = u64::from(envelope.client_event_count);
    if index + 1 == count {
        return Ok(None);
    }
    if index + 1 > count || index >= tail_lsn.get() {
        return Err(Error::new(ErrorCode::InvariantViolation).at_lsn(tail_lsn));
    }
    let retained_lsn = LSN::new(tail_lsn.get() - index - 1);
    log.truncate_to(retained_lsn)?;
    Ok(Some(retained_lsn))
}

fn validate_batch(batch: &BatchIdentity<'_>) -> Result<(), Error> {
    if batch.client_seq == 0 || batch.events.is_empty() || batch.events.len() > MAXIMUM_BATCH_EVENTS
    {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let event_count =
        u32::try_from(batch.events.len()).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
    for (index, event) in batch.events.iter().enumerate() {
        let envelope = read_envelope(event.plaintext_payload)?;
        let expected_index =
            u32::try_from(index).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
        if parse_connection_id(envelope.connection_id.as_deref())? != Some(batch.connection_id)
            || envelope.client_seq != batch.client_seq
            || envelope.client_event_index != expected_index
            || envelope.client_event_count != event_count
        {
            return Err(Error::new(ErrorCode::IdempotencyConflict));
        }
    }
    Ok(())
}

fn request_digest(events: &[BatchEvent<'_>]) -> Result<[u8; 32], Error> {
    if events.is_empty() {
        return Err(Error::new(ErrorCode::InvalidLength));
    }
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"hypermind.idempotency-request.v1\0");
    for event in events {
        hasher.update(&[event.kind as u8]);
        hasher.update(event.conversation.as_bytes());
        hasher.update(blake3::hash(event.plaintext_payload).as_bytes());
    }
    Ok(*hasher.finalize().as_bytes())
}

fn read_envelope(encoded: &[u8]) -> Result<EventEnvelope, Error> {
    if encoded.is_empty()
        || encoded.len() > MAXIMUM_EVENT_BYTES
        || encoded.get(4..8) != Some(b"NCEV".as_slice())
    {
        return Err(Error::new(ErrorCode::SchemaInvalid));
    }
    let envelope_ref = EventEnvelopeRef::read_as_root(encoded)
        .map_err(|_| Error::new(ErrorCode::SchemaInvalid))?;
    let envelope =
        EventEnvelope::try_from(envelope_ref).map_err(|_| Error::new(ErrorCode::SchemaInvalid))?;
    if envelope.schema_version == 0 || envelope.schema_version > CURRENT_SCHEMA_VERSION {
        return Err(Error::new(ErrorCode::SchemaVersion));
    }
    Ok(envelope)
}

fn parse_connection_id(value: Option<&[u8]>) -> Result<Option<ConnectionId>, Error> {
    value
        .map(|bytes| {
            bytes
                .try_into()
                .map_err(|_| Error::new(ErrorCode::IdempotencyConflict))
        })
        .transpose()
}
