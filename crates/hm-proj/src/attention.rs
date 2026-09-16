#![allow(clippy::missing_errors_doc)]

use crate::checkpoint::verify_frame;
use crate::generation::{decode, encode};
use crate::store::{Mutation, ProjectionId, ProjectionStore, ReadSnapshot};
use hm_core::{Error, ErrorCode};
use hm_ledger::frame::{EventKind, Frame};
use hm_schema::events::{AttentionDecision, EventPayload};
use serde::{Deserialize, Serialize};

const HISTORY_PREFIX: u8 = b'H';
pub const MAXIMUM_ATTENTION_HISTORY: usize = 256;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AttentionRecord {
    pub lsn: u64,
    pub intention_id: Vec<u8>,
    pub wake_id: Vec<u8>,
    pub decision: AttentionDecision,
    pub reason: String,
}

pub struct AttentionProjection;

impl AttentionProjection {
    pub fn apply_event(store: &ProjectionStore, frame: &Frame) -> Result<(), Error> {
        if frame.header.kind != EventKind::AttentionDecided {
            return store.apply(ProjectionId::AttentionHistory, frame.header.lsn, &[]);
        }
        let snapshot = store.begin_snapshot()?;
        let envelope = verify_frame(&snapshot, frame)?;
        let EventPayload::AttentionDecided(value) = envelope.payload else {
            return Err(Error::new(ErrorCode::InvalidKind).at_lsn(frame.header.lsn));
        };
        let record = AttentionRecord {
            lsn: frame.header.lsn.get(),
            intention_id: value.intention_id,
            wake_id: value.wake_id,
            decision: value.decision,
            reason: value.reason,
        };
        let mut mutations = vec![Mutation::put(history_key(record.lsn), encode(&record)?)];
        let existing = snapshot.scan_prefix(
            ProjectionId::AttentionHistory,
            &[HISTORY_PREFIX],
            MAXIMUM_ATTENTION_HISTORY,
        )?;
        if existing.len() == MAXIMUM_ATTENTION_HISTORY {
            mutations.push(Mutation::delete(existing[0].key.clone()));
        }
        drop(snapshot);
        store.apply(ProjectionId::AttentionHistory, frame.header.lsn, &mutations)
    }

    pub fn recent(
        snapshot: &ReadSnapshot<'_>,
        limit: usize,
    ) -> Result<Vec<AttentionRecord>, Error> {
        if limit == 0 || limit > MAXIMUM_ATTENTION_HISTORY {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let mut records = snapshot
            .scan_prefix(
                ProjectionId::AttentionHistory,
                &[HISTORY_PREFIX],
                MAXIMUM_ATTENTION_HISTORY,
            )?
            .into_iter()
            .map(|entry| decode(&entry.value))
            .collect::<Result<Vec<_>, _>>()?;
        records.reverse();
        records.truncate(limit);
        Ok(records)
    }
}

fn history_key(lsn: u64) -> [u8; 9] {
    let mut key = [0; 9];
    key[0] = HISTORY_PREFIX;
    key[1..].copy_from_slice(&lsn.to_be_bytes());
    key
}
