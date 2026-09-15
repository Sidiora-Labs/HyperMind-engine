#![allow(clippy::missing_errors_doc)]

use crate::store::{KeyValue, Mutation, ProjectionId, ProjectionStore, ReadSnapshot};
use hm_core::{ConversationId, Error, ErrorCode, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, Frame};

const EVENT_PREFIX: u8 = b'E';
const LSN_INDEX_PREFIX: u8 = b'L';

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConversationRecord {
    pub lsn: LSN,
    pub kind: EventKind,
    pub wall_timestamp_ns: UtcNanos,
    pub conversation: ConversationId,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LatestRecordScan {
    pub record: Option<ConversationRecord>,
    pub next_before_lsn: LSN,
    pub scanned: usize,
}

pub fn apply_timeline(store: &ProjectionStore, frame: &Frame) -> Result<(), Error> {
    let mutations = [
        Mutation::put(frame.header.conversation.into_bytes(), encode_head(frame)),
        Mutation::put(
            event_key(frame.header.conversation, frame.header.lsn),
            encode_record(frame),
        ),
        Mutation::put(
            lsn_index_key(frame.header.lsn),
            event_key(frame.header.conversation, frame.header.lsn),
        ),
    ];
    store.apply(
        ProjectionId::ConversationHeads,
        frame.header.lsn,
        &mutations,
    )
}

pub fn read_conversation_records(
    snapshot: &ReadSnapshot<'_>,
    conversation: ConversationId,
    limit: usize,
) -> Result<Vec<ConversationRecord>, Error> {
    if limit == 0 {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let mut records: Vec<ConversationRecord> = snapshot
        .scan_prefix_reverse(
            ProjectionId::ConversationHeads,
            &event_prefix(conversation),
            limit,
        )?
        .iter()
        .map(decode_record)
        .collect::<Result<_, _>>()?;
    records.reverse();
    Ok(records)
}

pub fn scan_latest_conversation_record_of_kind(
    snapshot: &ReadSnapshot<'_>,
    conversation: ConversationId,
    kind: EventKind,
    before_lsn: LSN,
    maximum_scanned: usize,
) -> Result<LatestRecordScan, Error> {
    if before_lsn.get() == 0 || maximum_scanned == 0 {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let prefix = event_prefix(conversation);
    let mut scan = LatestRecordScan {
        record: None,
        next_before_lsn: before_lsn,
        scanned: 0,
    };
    while scan.scanned < maximum_scanned {
        let chunk = (maximum_scanned - scan.scanned).min(64);
        let items = snapshot.scan_prefix_reverse_before(
            ProjectionId::ConversationHeads,
            &prefix,
            &event_key(conversation, scan.next_before_lsn),
            chunk,
        )?;
        if items.is_empty() {
            scan.next_before_lsn = LSN::new(0);
            break;
        }
        for item in &items {
            let record = decode_record(item)?;
            scan.scanned += 1;
            scan.next_before_lsn = record.lsn;
            if record.kind == kind {
                scan.record = Some(record);
                return Ok(scan);
            }
        }
        if items.len() < chunk {
            scan.next_before_lsn = LSN::new(0);
            break;
        }
    }
    Ok(scan)
}

pub fn read_conversation_record(
    snapshot: &ReadSnapshot<'_>,
    lsn: LSN,
) -> Result<Option<ConversationRecord>, Error> {
    if lsn.get() == 0 {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let Some(key) = snapshot.get(ProjectionId::ConversationHeads, &lsn_index_key(lsn))? else {
        return Ok(None);
    };
    let Some(value) = snapshot.get(ProjectionId::ConversationHeads, &key)? else {
        return Err(Error::new(ErrorCode::InvariantViolation).at_lsn(lsn));
    };
    let record = decode_record(&KeyValue { key, value })?;
    if record.lsn != lsn {
        return Err(Error::new(ErrorCode::InvariantViolation).at_lsn(lsn));
    }
    Ok(Some(record))
}

fn encode_head(frame: &Frame) -> [u8; 17] {
    let mut value = [0_u8; 17];
    value[..8].copy_from_slice(&frame.header.lsn.get().to_le_bytes());
    value[8..16].copy_from_slice(&frame.header.wall_timestamp_ns.get().to_le_bytes());
    value[16] = frame.header.kind as u8;
    value
}

fn event_prefix(conversation: ConversationId) -> Vec<u8> {
    let mut key = Vec::with_capacity(17);
    key.push(EVENT_PREFIX);
    key.extend_from_slice(conversation.as_bytes());
    key
}

fn event_key(conversation: ConversationId, lsn: LSN) -> Vec<u8> {
    let mut key = event_prefix(conversation);
    key.extend_from_slice(&lsn.get().to_be_bytes());
    key
}

fn lsn_index_key(lsn: LSN) -> [u8; 9] {
    let mut key = [0_u8; 9];
    key[0] = LSN_INDEX_PREFIX;
    key[1..].copy_from_slice(&lsn.get().to_be_bytes());
    key
}

fn encode_record(frame: &Frame) -> Vec<u8> {
    let mut value = Vec::with_capacity(9 + frame.sealed_payload.len());
    value.push(frame.header.kind as u8);
    value.extend_from_slice(&frame.header.wall_timestamp_ns.get().to_le_bytes());
    value.extend_from_slice(&frame.sealed_payload);
    value
}

fn decode_record(item: &KeyValue) -> Result<ConversationRecord, Error> {
    if item.key.len() != 25 || item.key[0] != EVENT_PREFIX || item.value.len() < 9 {
        return Err(Error::new(ErrorCode::InvariantViolation));
    }
    let lsn = LSN::new(u64::from_be_bytes(copy_array(&item.key, 17)?));
    let kind = EventKind::try_from(item.value[0])
        .map_err(|_| Error::new(ErrorCode::InvariantViolation))?;
    let wall_timestamp_ns = UtcNanos::new(i64::from_le_bytes(copy_array(&item.value, 1)?));
    let conversation = ConversationId::new(copy_array(&item.key, 1)?);
    if lsn.get() == 0 {
        return Err(Error::new(ErrorCode::InvariantViolation));
    }
    Ok(ConversationRecord {
        lsn,
        kind,
        wall_timestamp_ns,
        conversation,
        payload: item.value[9..].to_vec(),
    })
}

fn copy_array<const N: usize>(bytes: &[u8], offset: usize) -> Result<[u8; N], Error> {
    bytes
        .get(offset..offset + N)
        .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))?
        .try_into()
        .map_err(|_| Error::new(ErrorCode::InvariantViolation))
}
