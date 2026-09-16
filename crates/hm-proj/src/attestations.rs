#![allow(clippy::missing_errors_doc)]

use crate::checkpoint::verify_frame;
use crate::generation::{decode, encode};
use crate::store::{Mutation, ProjectionId, ProjectionStore, ReadSnapshot};
use hm_core::{Error, ErrorCode, LSN};
use hm_ledger::frame::{EventKind, Frame};
use hm_schema::events::{AttestationDisposition, EventPayload};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const TARGET_PREFIX: u8 = b'T';
const PREFERENCE_PREFIX: u8 = b'P';

pub const PREFERENCE_NEUTRAL_Q16: u32 = 65_536;
pub const PREFERENCE_MINIMUM_Q16: u32 = 49_152;
pub const PREFERENCE_MAXIMUM_Q16: u32 = 81_920;
pub const PREFERENCE_RATE_Q16: u32 = 13_107;
pub const MAXIMUM_PREFERENCE_TARGETS: usize = 4096;

const PREFERENCE_USED_TARGET_Q16: u32 = 73_728;
const PREFERENCE_IGNORED_TARGET_Q16: u32 = 57_344;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AttestationRecord {
    pub target_lsn: u64,
    pub used: u64,
    pub ignored: u64,
    pub helpful: u64,
    pub harmful: u64,
    pub last_attestation_lsn: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PreferenceWeight {
    pub target_lsn: u64,
    pub weight_q16: u32,
    pub observations: u64,
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
        let weight_key = preference_key(value.target_lsn);
        let mut weight = snapshot
            .get(ProjectionId::Attestations, &weight_key)?
            .map(|bytes| decode::<PreferenceWeight>(&bytes))
            .transpose()?
            .unwrap_or(PreferenceWeight {
                target_lsn: value.target_lsn,
                weight_q16: PREFERENCE_NEUTRAL_Q16,
                observations: 0,
                last_attestation_lsn: 0,
            });
        weight.weight_q16 =
            next_preference_q16(weight.weight_q16, disposition_target_q16(value.disposition));
        weight.observations = weight.observations.saturating_add(1);
        weight.last_attestation_lsn = frame.header.lsn.get();
        drop(snapshot);
        store.apply(
            ProjectionId::Attestations,
            frame.header.lsn,
            &[
                Mutation::put(key, encode(&record)?),
                Mutation::put(weight_key, encode(&weight)?),
            ],
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

    pub fn preference(
        snapshot: &ReadSnapshot<'_>,
        target_lsn: LSN,
    ) -> Result<Option<PreferenceWeight>, Error> {
        snapshot
            .get(
                ProjectionId::Attestations,
                &preference_key(target_lsn.get()),
            )?
            .map(|bytes| decode(&bytes))
            .transpose()
    }

    pub fn preferences(
        snapshot: &ReadSnapshot<'_>,
        targets: &[LSN],
    ) -> Result<BTreeMap<u64, u32>, Error> {
        if targets.len() > MAXIMUM_PREFERENCE_TARGETS {
            return Err(Error::new(ErrorCode::CapacityExceeded));
        }
        let mut weights = BTreeMap::new();
        for target in targets {
            if target.get() == 0 {
                return Err(Error::new(ErrorCode::InvalidArgument));
            }
            if let Some(weight) = Self::preference(snapshot, *target)? {
                weights.insert(weight.target_lsn, weight.weight_q16);
            }
        }
        Ok(weights)
    }

    pub fn recent_preferences(
        snapshot: &ReadSnapshot<'_>,
        limit: usize,
    ) -> Result<Vec<PreferenceWeight>, Error> {
        if limit == 0 || limit > MAXIMUM_PREFERENCE_TARGETS {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        snapshot
            .scan_prefix_reverse(ProjectionId::Attestations, &[PREFERENCE_PREFIX], limit)?
            .into_iter()
            .map(|entry| decode(&entry.value))
            .collect()
    }
}

const fn disposition_target_q16(disposition: AttestationDisposition) -> u32 {
    match disposition {
        AttestationDisposition::Used => PREFERENCE_USED_TARGET_Q16,
        AttestationDisposition::Ignored => PREFERENCE_IGNORED_TARGET_Q16,
        AttestationDisposition::Helpful => PREFERENCE_MAXIMUM_Q16,
        AttestationDisposition::Harmful => PREFERENCE_MINIMUM_Q16,
    }
}

#[allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
const fn next_preference_q16(weight_q16: u32, target_q16: u32) -> u32 {
    let weight = weight_q16 as i64;
    let step = ((target_q16 as i64 - weight) * PREFERENCE_RATE_Q16 as i64) >> 16;
    let next = weight + step;
    if next < PREFERENCE_MINIMUM_Q16 as i64 {
        PREFERENCE_MINIMUM_Q16
    } else if next > PREFERENCE_MAXIMUM_Q16 as i64 {
        PREFERENCE_MAXIMUM_Q16
    } else {
        next as u32
    }
}

fn target_key(lsn: u64) -> [u8; 9] {
    let mut key = [0; 9];
    key[0] = TARGET_PREFIX;
    key[1..].copy_from_slice(&lsn.to_be_bytes());
    key
}

fn preference_key(lsn: u64) -> [u8; 9] {
    let mut key = [0; 9];
    key[0] = PREFERENCE_PREFIX;
    key[1..].copy_from_slice(&lsn.to_be_bytes());
    key
}
