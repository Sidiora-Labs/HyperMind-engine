#![allow(clippy::missing_errors_doc)]

use crate::checkpoint::verify_frame;
use crate::store::{KeyValue, Mutation, ProjectionId, ProjectionStore, ReadSnapshot};
use hm_core::{ConversationId, Error, ErrorCode, LSN};
use hm_ledger::frame::{EventKind, Frame};
use hm_schema::events::{EffectState, EventPayload, ResultStatus};

const WORK_PREFIX: u8 = b'W';
const CALL_LSN_INDEX_PREFIX: u8 = b'N';
const EFFECT_INDEX_PREFIX: u8 = b'E';

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum WorkKind {
    ToolCall = 0,
    Effect = 1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum WorkState {
    Dispatched = 0,
    Committed = 1,
    Returned = 2,
    OutcomeUnknown = 3,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkItem {
    pub kind: WorkKind,
    pub state: WorkState,
    pub tool_call_lsn: LSN,
    pub state_lsn: LSN,
    pub call_id: Vec<u8>,
    pub tool_name: Vec<u8>,
    pub arguments: Vec<u8>,
    pub effect_id: Vec<u8>,
    pub detail: Vec<u8>,
    pub requires_reconciliation: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LedgerRebuildProgress {
    pub applied_lsn: LSN,
    pub applied_frames: usize,
    pub complete: bool,
}

pub struct WorkLedgerProjection;

impl WorkLedgerProjection {
    #[allow(clippy::too_many_lines)]
    pub fn apply_event(store: &ProjectionStore, frame: &Frame) -> Result<(), Error> {
        if !matches!(
            frame.header.kind,
            EventKind::ToolCall | EventKind::ToolResult | EventKind::Effect | EventKind::Outcome
        ) {
            return store.apply(ProjectionId::WorkLedger, frame.header.lsn, &[]);
        }
        let snapshot = store.begin_snapshot()?;
        let envelope = verify_frame(&snapshot, frame)?;
        let mut mutations = Vec::new();
        match (frame.header.kind, envelope.payload) {
            (EventKind::ToolCall, EventPayload::ToolCall(value)) => {
                let item = WorkItem {
                    kind: WorkKind::ToolCall,
                    state: WorkState::Dispatched,
                    tool_call_lsn: frame.header.lsn,
                    state_lsn: frame.header.lsn,
                    call_id: value.call_id,
                    tool_name: value.tool_name.into_bytes(),
                    arguments: value.arguments,
                    effect_id: Vec::new(),
                    detail: Vec::new(),
                    requires_reconciliation: true,
                };
                let primary = work_key(
                    frame.header.conversation,
                    frame.header.lsn,
                    WorkKind::ToolCall,
                    &[],
                );
                mutations.push(Mutation::put(primary.clone(), encode_item(&item)));
                mutations.push(Mutation::put(lsn_index_key(frame.header.lsn), primary));
            }
            (EventKind::ToolResult, EventPayload::ToolResult(value)) => {
                let source =
                    read_indexed(&snapshot, &lsn_index_key(LSN::new(value.tool_call_lsn)))?;
                let item = decode_item(&source.value)?;
                let conversation = conversation_from_work_key(&source.key)?;
                if conversation != frame.header.conversation || item.call_id != value.call_id {
                    return Err(Error::new(ErrorCode::OrderingViolation).at_lsn(frame.header.lsn));
                }
                update_item(
                    &mut mutations,
                    source,
                    from_result_status(value.status),
                    frame.header.lsn,
                    &value.result,
                )?;
            }
            (EventKind::Effect, EventPayload::Effect(value)) => {
                let call_source =
                    read_indexed(&snapshot, &lsn_index_key(LSN::new(value.tool_call_lsn)))?;
                let call = decode_item(&call_source.value)?;
                let conversation = conversation_from_work_key(&call_source.key)?;
                if conversation != frame.header.conversation {
                    return Err(Error::new(ErrorCode::OrderingViolation).at_lsn(frame.header.lsn));
                }
                let index_key = effect_index_key(&value.effect_id);
                if snapshot
                    .get(ProjectionId::WorkLedger, &index_key)?
                    .is_some()
                {
                    let source = read_indexed(&snapshot, &index_key)?;
                    let existing = decode_item(&source.value)?;
                    if existing.tool_call_lsn.get() != value.tool_call_lsn {
                        return Err(
                            Error::new(ErrorCode::OrderingViolation).at_lsn(frame.header.lsn)
                        );
                    }
                    update_item(
                        &mut mutations,
                        source,
                        from_effect_state(value.state),
                        frame.header.lsn,
                        &[],
                    )?;
                } else {
                    let state = from_effect_state(value.state);
                    let item = WorkItem {
                        kind: WorkKind::Effect,
                        state,
                        tool_call_lsn: LSN::new(value.tool_call_lsn),
                        state_lsn: frame.header.lsn,
                        call_id: call.call_id,
                        tool_name: call.tool_name,
                        arguments: call.arguments,
                        effect_id: value.effect_id.clone(),
                        detail: Vec::new(),
                        requires_reconciliation: state != WorkState::Returned,
                    };
                    let primary = work_key(
                        conversation,
                        item.tool_call_lsn,
                        WorkKind::Effect,
                        &item.effect_id,
                    );
                    mutations.push(Mutation::put(primary.clone(), encode_item(&item)));
                    mutations.push(Mutation::put(index_key, primary));
                }
            }
            (EventKind::Outcome, EventPayload::Outcome(value)) => {
                let source = read_indexed(&snapshot, &effect_index_key(&value.effect_id))?;
                if conversation_from_work_key(&source.key)? != frame.header.conversation {
                    return Err(Error::new(ErrorCode::OrderingViolation).at_lsn(frame.header.lsn));
                }
                update_item(
                    &mut mutations,
                    source,
                    from_result_status(value.status),
                    frame.header.lsn,
                    &value.detail,
                )?;
            }
            _ => {
                return Err(Error::new(ErrorCode::InvariantViolation).at_lsn(frame.header.lsn));
            }
        }
        drop(snapshot);
        store.apply(ProjectionId::WorkLedger, frame.header.lsn, &mutations)
    }

    pub fn rebuild(
        store: &ProjectionStore,
        frames: &[Frame],
        reset: bool,
        maximum_frames: usize,
    ) -> Result<LedgerRebuildProgress, Error> {
        if reset {
            store.reset(ProjectionId::WorkLedger)?;
        }
        let snapshot = store.begin_snapshot()?;
        let checkpoint = snapshot.checkpoint(ProjectionId::WorkLedger)?.get();
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
        Ok(LedgerRebuildProgress {
            applied_lsn: LSN::new(applied_lsn),
            applied_frames: count,
            complete: applied_lsn == frames.len() as u64,
        })
    }

    pub fn read_conversation(
        snapshot: &ReadSnapshot<'_>,
        conversation: ConversationId,
        limit: usize,
    ) -> Result<Vec<WorkItem>, Error> {
        if limit == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let mut ledger = snapshot
            .scan_prefix_reverse(ProjectionId::WorkLedger, &work_prefix(conversation), limit)?
            .iter()
            .map(|item| decode_item(&item.value))
            .collect::<Result<Vec<_>, _>>()?;
        ledger.reverse();
        Ok(ledger)
    }
}

#[must_use]
pub const fn transition_allowed(current: WorkState, next: WorkState) -> bool {
    current as u8 == next as u8
        || matches!(next, WorkState::Returned)
        || matches!(current, WorkState::Dispatched)
        || matches!(
            (current, next),
            (WorkState::Committed, WorkState::OutcomeUnknown)
        )
}

fn update_item(
    mutations: &mut Vec<Mutation>,
    source: KeyValue,
    next: WorkState,
    state_lsn: LSN,
    detail: &[u8],
) -> Result<(), Error> {
    let mut item = decode_item(&source.value)?;
    if !transition_allowed(item.state, next) {
        return Err(Error::new(ErrorCode::OrderingViolation)
            .at_lsn(state_lsn)
            .at_offset(item.state_lsn.get()));
    }
    item.state = next;
    item.state_lsn = state_lsn;
    item.detail = detail.to_vec();
    item.requires_reconciliation = next != WorkState::Returned;
    mutations.push(Mutation::put(source.key, encode_item(&item)));
    Ok(())
}

fn read_indexed(snapshot: &ReadSnapshot<'_>, index_key: &[u8]) -> Result<KeyValue, Error> {
    let primary = snapshot
        .get(ProjectionId::WorkLedger, index_key)?
        .ok_or_else(|| Error::new(ErrorCode::WorkItemNotFound))?;
    let value = snapshot
        .get(ProjectionId::WorkLedger, &primary)?
        .ok_or_else(|| Error::new(ErrorCode::WorkLedgerCorrupt))?;
    Ok(KeyValue {
        key: primary,
        value,
    })
}

fn work_prefix(conversation: ConversationId) -> Vec<u8> {
    let mut key = Vec::with_capacity(17);
    key.push(WORK_PREFIX);
    key.extend_from_slice(conversation.as_bytes());
    key
}

fn work_key(
    conversation: ConversationId,
    tool_call_lsn: LSN,
    kind: WorkKind,
    effect_id: &[u8],
) -> Vec<u8> {
    let mut key = work_prefix(conversation);
    key.extend_from_slice(&tool_call_lsn.get().to_be_bytes());
    key.push(kind as u8);
    key.extend_from_slice(effect_id);
    key
}

fn lsn_index_key(tool_call_lsn: LSN) -> [u8; 9] {
    let mut key = [0_u8; 9];
    key[0] = CALL_LSN_INDEX_PREFIX;
    key[1..].copy_from_slice(&tool_call_lsn.get().to_be_bytes());
    key
}

fn effect_index_key(effect_id: &[u8]) -> Vec<u8> {
    let mut key = Vec::with_capacity(1 + effect_id.len());
    key.push(EFFECT_INDEX_PREFIX);
    key.extend_from_slice(effect_id);
    key
}

fn conversation_from_work_key(key: &[u8]) -> Result<ConversationId, Error> {
    if key.len() < 26 || key[0] != WORK_PREFIX {
        return Err(Error::new(ErrorCode::WorkLedgerCorrupt));
    }
    Ok(ConversationId::new(
        key[1..17]
            .try_into()
            .map_err(|_| Error::new(ErrorCode::WorkLedgerCorrupt))?,
    ))
}

fn encode_item(item: &WorkItem) -> Vec<u8> {
    let mut value = Vec::new();
    value.push(item.kind as u8);
    value.push(item.state as u8);
    value.extend_from_slice(&item.tool_call_lsn.get().to_le_bytes());
    value.extend_from_slice(&item.state_lsn.get().to_le_bytes());
    append_bytes(&mut value, &item.call_id);
    append_bytes(&mut value, &item.tool_name);
    append_bytes(&mut value, &item.arguments);
    append_bytes(&mut value, &item.effect_id);
    append_bytes(&mut value, &item.detail);
    value
}

fn decode_item(value: &[u8]) -> Result<WorkItem, Error> {
    let kind = match value.first() {
        Some(0) => WorkKind::ToolCall,
        Some(1) => WorkKind::Effect,
        _ => return Err(Error::new(ErrorCode::WorkLedgerCorrupt)),
    };
    let state = match value.get(1) {
        Some(0) => WorkState::Dispatched,
        Some(1) => WorkState::Committed,
        Some(2) => WorkState::Returned,
        Some(3) => WorkState::OutcomeUnknown,
        _ => return Err(Error::new(ErrorCode::WorkLedgerCorrupt)),
    };
    let mut offset = 2;
    let tool_call_lsn = LSN::new(read_u64(value, &mut offset)?);
    let state_lsn = LSN::new(read_u64(value, &mut offset)?);
    let call_id = read_bytes(value, &mut offset)?;
    let tool_name = read_bytes(value, &mut offset)?;
    let arguments = read_bytes(value, &mut offset)?;
    let effect_id = read_bytes(value, &mut offset)?;
    let detail = read_bytes(value, &mut offset)?;
    if tool_call_lsn.get() == 0
        || state_lsn.get() == 0
        || call_id.is_empty()
        || tool_name.is_empty()
        || offset != value.len()
        || (kind == WorkKind::ToolCall && !effect_id.is_empty())
        || (kind == WorkKind::Effect && effect_id.is_empty())
    {
        return Err(Error::new(ErrorCode::WorkLedgerCorrupt));
    }
    Ok(WorkItem {
        kind,
        state,
        tool_call_lsn,
        state_lsn,
        call_id,
        tool_name,
        arguments,
        effect_id,
        detail,
        requires_reconciliation: state != WorkState::Returned,
    })
}

const fn from_effect_state(state: EffectState) -> WorkState {
    match state {
        EffectState::Dispatched => WorkState::Dispatched,
        EffectState::Committed => WorkState::Committed,
        EffectState::Returned => WorkState::Returned,
        EffectState::OutcomeUnknown => WorkState::OutcomeUnknown,
    }
}

const fn from_result_status(status: ResultStatus) -> WorkState {
    if matches!(status, ResultStatus::OutcomeUnknown) {
        WorkState::OutcomeUnknown
    } else {
        WorkState::Returned
    }
}

fn append_bytes(output: &mut Vec<u8>, value: &[u8]) {
    output.extend_from_slice(&(value.len() as u64).to_le_bytes());
    output.extend_from_slice(value);
}

fn read_u64(bytes: &[u8], offset: &mut usize) -> Result<u64, Error> {
    let value = bytes
        .get(*offset..*offset + 8)
        .ok_or_else(|| Error::new(ErrorCode::WorkLedgerCorrupt))?
        .try_into()
        .map(u64::from_le_bytes)
        .map_err(|_| Error::new(ErrorCode::WorkLedgerCorrupt))?;
    *offset += 8;
    Ok(value)
}

fn read_bytes(bytes: &[u8], offset: &mut usize) -> Result<Vec<u8>, Error> {
    let length = usize::try_from(read_u64(bytes, offset)?)
        .map_err(|_| Error::new(ErrorCode::WorkLedgerCorrupt))?;
    let value = bytes
        .get(*offset..*offset + length)
        .ok_or_else(|| Error::new(ErrorCode::WorkLedgerCorrupt))?
        .to_vec();
    *offset += length;
    Ok(value)
}
