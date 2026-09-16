#![allow(clippy::missing_errors_doc)]

use crate::checkpoint::verify_frame;
use crate::generation::{decode, encode};
use crate::store::{Mutation, ProjectionId, ProjectionStore, ReadSnapshot};
use hm_core::{Error, ErrorCode, LSN};
use hm_ledger::frame::{EventKind, Frame};
use hm_schema::events::{AttestationDisposition, EventPayload};
use serde::{Deserialize, Serialize};

const TARGET_PREFIX: u8 = b'T';

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AttestationRecord {
    pub target_lsn: u64,
    pub used: u64,
    pub ignored: u64,
    pub helpful: u64,
    pub harmful: u64,
    pub last_attestation_lsn: u64,
}

pub struct AttestationsProjection;

impl AttestationsProjection {
    pub fn apply_event(store: &ProjectionStore, frame: &Frame) -> Result<(), Error> {
        if frame.header.kind != EventKind::Attestation {
            return store.apply(ProjectionId::Attestations, frame.header.lsn, &[]);
        }
        let snapshot = store.begin_snapshot()?;
        let envelope = verify_frame(&snapshot, frame)?;
        let EventPayload::Attestation(value) = envelope.payload else {
            return Err(Error::new(ErrorCode::InvalidKind).at_lsn(frame.header.lsn));
        };
        let key = target_key(value.target_lsn);
        let mut record = snapshot
            .get(ProjectionId::Attestations, &key)?
            .map(|bytes| decode::<AttestationRecord>(&bytes))
            .transpose()?
            .unwrap_or(AttestationRecord {
                target_lsn: value.target_lsn,
                ..AttestationRecord::default()
            });
        let counter = match value.disposition {
            AttestationDisposition::Used => &mut record.used,
            AttestationDisposition::Ignored => &mut record.ignored,
            AttestationDisposition::Helpful => &mut record.helpful,
            AttestationDisposition::Harmful => &mut record.harmful,
        };
        *counter = counter.saturating_add(1);
        record.last_attestation_lsn = frame.header.lsn.get();
        drop(snapshot);
        store.apply(
            ProjectionId::Attestations,
            frame.header.lsn,
            &[Mutation::put(key, encode(&record)?)],
        )
    }

    pub fn get(
        snapshot: &ReadSnapshot<'_>,
        target_lsn: LSN,
    ) -> Result<Option<AttestationRecord>, Error> {
        snapshot
            .get(ProjectionId::Attestations, &target_key(target_lsn.get()))?
            .map(|bytes| decode(&bytes))
            .transpose()
    }
}

fn target_key(lsn: u64) -> [u8; 9] {
    let mut key = [0; 9];
    key[0] = TARGET_PREFIX;
    key[1..].copy_from_slice(&lsn.to_be_bytes());
    key
}
