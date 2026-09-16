#![allow(clippy::missing_errors_doc)]

use crate::checkpoint::verify_frame;
use crate::generation::{decode, encode};
use crate::store::{Mutation, ProjectionId, ProjectionStore, ReadSnapshot};
use hm_core::{Error, ErrorCode, LSN};
use hm_ledger::frame::{EventKind, Frame};
use hm_schema::events::{EventPayload, WakeTrigger};
use serde::{Deserialize, Serialize};

const INTENTION_PREFIX: u8 = b'I';
const TRIGGER_PREFIX: u8 = b'T';
const WAKE_PREFIX: u8 = b'W';
pub const MAXIMUM_INTENTIONS_PER_TRIGGER: usize = 4096;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum TriggerKind {
    At,
    Schedule,
    ChildTerminal,
    ProcessExit,
    FileChanged,
    RepositoryChanged,
    ChannelMessage,
    ExternalCondition,
    UserResponse,
    EntityMentioned,
    LoopClosed,
    PredictionResolved,
    BeliefChanged,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum IntentionStatus {
    Pending,
    Fired,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct IntentionRecord {
    pub intention_id: Vec<u8>,
    pub objective: Vec<u8>,
    pub trigger: WakeTrigger,
    pub trigger_kind: TriggerKind,
    pub expires_at_ns: i64,
    pub reply_route: String,
    pub set_lsn: u64,
    pub status_lsn: u64,
    pub status: IntentionStatus,
    pub wake_id: Option<Vec<u8>>,
    pub trigger_lsn: u64,
    pub cancellation_reason: Option<String>,
}

pub struct IntentionsProjection;

impl IntentionsProjection {
    pub fn apply_event(store: &ProjectionStore, frame: &Frame) -> Result<(), Error> {
        if !matches!(
            frame.header.kind,
            EventKind::IntentionSet | EventKind::IntentionFired | EventKind::IntentionCancelled
        ) {
            return store.apply(ProjectionId::Intentions, frame.header.lsn, &[]);
        }
        let snapshot = store.begin_snapshot()?;
        let envelope = verify_frame(&snapshot, frame)?;
        let mut mutations = Vec::new();
        match envelope.payload {
            EventPayload::IntentionSet(value) => {
                let key = intention_key(&value.intention_id);
                if snapshot.get(ProjectionId::Intentions, &key)?.is_some() {
                    return Err(Error::new(ErrorCode::AlreadyExists).at_lsn(frame.header.lsn));
                }
                let trigger = value
                    .trigger
                    .ok_or_else(|| Error::new(ErrorCode::SchemaInvalid))?;
                let kind = trigger_kind(&trigger);
                let record = IntentionRecord {
                    intention_id: value.intention_id,
                    objective: value.objective,
                    trigger,
                    trigger_kind: kind,
                    expires_at_ns: value.expires_at_ns,
                    reply_route: value.reply_route,
                    set_lsn: frame.header.lsn.get(),
                    status_lsn: frame.header.lsn.get(),
                    status: IntentionStatus::Pending,
                    wake_id: None,
                    trigger_lsn: 0,
                    cancellation_reason: None,
                };
                mutations.push(Mutation::put(key, encode(&record)?));
                mutations.push(Mutation::put(
                    trigger_index_key(kind, &record.intention_id),
                    frame.header.lsn.get().to_le_bytes(),
                ));
            }
            EventPayload::IntentionFired(value) => {
                let wake_key = wake_key(&value.wake_id);
                if let Some(existing) = snapshot.get(ProjectionId::Intentions, &wake_key)? {
                    if existing != value.intention_id {
                        return Err(
                            Error::new(ErrorCode::IdempotencyConflict).at_lsn(frame.header.lsn)
                        );
                    }
                } else {
                    let key = intention_key(&value.intention_id);
                    let mut record = read_required(&snapshot, &key, frame.header.lsn)?;
                    if record.status != IntentionStatus::Pending
                        || value.trigger_lsn >= frame.header.lsn.get()
                    {
                        return Err(
                            Error::new(ErrorCode::OrderingViolation).at_lsn(frame.header.lsn)
                        );
                    }
                    record.status = IntentionStatus::Fired;
                    record.status_lsn = frame.header.lsn.get();
                    record.wake_id = Some(value.wake_id.clone());
                    record.trigger_lsn = value.trigger_lsn;
                    mutations.push(Mutation::put(key, encode(&record)?));
                    mutations.push(Mutation::put(wake_key, value.intention_id));
                }
            }
            EventPayload::IntentionCancelled(value) => {
                let key = intention_key(&value.intention_id);
                let mut record = read_required(&snapshot, &key, frame.header.lsn)?;
                if record.status == IntentionStatus::Cancelled {
                    return Err(Error::new(ErrorCode::AlreadyExists).at_lsn(frame.header.lsn));
                }
                record.status = IntentionStatus::Cancelled;
                record.status_lsn = frame.header.lsn.get();
                record.cancellation_reason = Some(value.reason);
                mutations.push(Mutation::put(key, encode(&record)?));
            }
            _ => return Err(Error::new(ErrorCode::InvalidKind).at_lsn(frame.header.lsn)),
        }
        drop(snapshot);
        store.apply(ProjectionId::Intentions, frame.header.lsn, &mutations)
    }

    pub fn get(
        snapshot: &ReadSnapshot<'_>,
        intention_id: &[u8],
    ) -> Result<Option<IntentionRecord>, Error> {
        snapshot
            .get(ProjectionId::Intentions, &intention_key(intention_id))?
            .map(|bytes| decode(&bytes))
            .transpose()
    }

    pub fn pending_by_trigger(
        snapshot: &ReadSnapshot<'_>,
        kind: TriggerKind,
        limit: usize,
    ) -> Result<Vec<IntentionRecord>, Error> {
        if limit == 0 || limit > MAXIMUM_INTENTIONS_PER_TRIGGER {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        snapshot
            .scan_prefix(
                ProjectionId::Intentions,
                &[TRIGGER_PREFIX, kind as u8],
                limit,
            )?
            .into_iter()
            .filter_map(|entry| {
                let id = entry.key.get(2..)?;
                match Self::get(snapshot, id) {
                    Ok(Some(record)) if record.status == IntentionStatus::Pending => {
                        Some(Ok(record))
                    }
                    Ok(_) => None,
                    Err(error) => Some(Err(error)),
                }
            })
            .collect()
    }
}

fn read_required(
    snapshot: &ReadSnapshot<'_>,
    key: &[u8],
    lsn: LSN,
) -> Result<IntentionRecord, Error> {
    snapshot
        .get(ProjectionId::Intentions, key)?
        .ok_or_else(|| Error::new(ErrorCode::OrderingViolation).at_lsn(lsn))
        .and_then(|bytes| decode(&bytes))
}

fn intention_key(id: &[u8]) -> Vec<u8> {
    let mut key = Vec::with_capacity(id.len() + 1);
    key.push(INTENTION_PREFIX);
    key.extend_from_slice(id);
    key
}

fn trigger_index_key(kind: TriggerKind, id: &[u8]) -> Vec<u8> {
    let mut key = Vec::with_capacity(id.len() + 2);
    key.extend_from_slice(&[TRIGGER_PREFIX, kind as u8]);
    key.extend_from_slice(id);
    key
}

fn wake_key(id: &[u8]) -> Vec<u8> {
    let mut key = Vec::with_capacity(id.len() + 1);
    key.push(WAKE_PREFIX);
    key.extend_from_slice(id);
    key
}

fn trigger_kind(trigger: &WakeTrigger) -> TriggerKind {
    match trigger {
        WakeTrigger::WakeAt(_) => TriggerKind::At,
        WakeTrigger::WakeSchedule(_) => TriggerKind::Schedule,
        WakeTrigger::WakeChildTerminal(_) => TriggerKind::ChildTerminal,
        WakeTrigger::WakeProcessExit(_) => TriggerKind::ProcessExit,
        WakeTrigger::WakeFileChanged(_) => TriggerKind::FileChanged,
        WakeTrigger::WakeRepositoryChanged(_) => TriggerKind::RepositoryChanged,
        WakeTrigger::WakeChannelMessage(_) => TriggerKind::ChannelMessage,
        WakeTrigger::WakeExternalCondition(_) => TriggerKind::ExternalCondition,
        WakeTrigger::WakeUserResponse(_) => TriggerKind::UserResponse,
        WakeTrigger::WakeEntityMentioned(_) => TriggerKind::EntityMentioned,
        WakeTrigger::WakeLoopClosed(_) => TriggerKind::LoopClosed,
        WakeTrigger::WakePredictionResolved(_) => TriggerKind::PredictionResolved,
        WakeTrigger::WakeBeliefChanged(_) => TriggerKind::BeliefChanged,
    }
}
