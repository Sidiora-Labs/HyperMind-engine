#![allow(clippy::missing_errors_doc)]

use crate::store::{Mutation, ProjectionId, ProjectionStore, ReadSnapshot};
use hm_core::{Error, ErrorCode, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, Frame};
use hm_schema::event::{self, Boundary};
use hm_schema::events::EventPayload;
use std::collections::{BTreeMap, BTreeSet};

const TASK_PREFIX: u8 = b'T';
const SCOPE_PREFIX: u8 = b'S';
pub const MAXIMUM_REQUIRED_BINDINGS: usize = 128;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum BindingTarget {
    Task(Vec<u8>),
    Scope(Vec<u8>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingRecord {
    pub target: BindingTarget,
    pub canonical_entity: String,
    pub property: String,
    pub evidence_lsn: LSN,
    pub revision: Vec<u8>,
    pub freshness_requirement_ns: u64,
    pub effective_time_ns: UtcNanos,
    pub binding_lsn: LSN,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingRequirement {
    pub canonical_entity: String,
    pub property: String,
    pub revision: Option<Vec<u8>>,
    pub freshness_requirement_ns: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingStatus {
    Resolved,
    Missing,
    Stale,
    Conflicting,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingResolution {
    pub requirement: BindingRequirement,
    pub status: BindingStatus,
    pub binding: Option<BindingRecord>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BindingsRebuildProgress {
    pub applied_lsn: LSN,
    pub applied_frames: usize,
    pub complete: bool,
}

pub struct BindingsProjection;

impl BindingsProjection {
    pub fn apply_event(store: &ProjectionStore, frame: &Frame) -> Result<(), Error> {
        if frame.header.kind != EventKind::Binding {
            return store.apply(ProjectionId::Bindings, frame.header.lsn, &[]);
        }
        let verified = event::verify_event(
            &frame.sealed_payload,
            event::EventKind::Binding,
            Boundary::Disk,
        )
        .map_err(|error| error.at_lsn(frame.header.lsn))?;
        let EventPayload::Binding(value) = verified.envelope.payload else {
            return Err(Error::new(ErrorCode::InvariantViolation).at_lsn(frame.header.lsn));
        };
        let target = match (value.task, value.scope) {
            (Some(task), None) => BindingTarget::Task(task),
            (None, Some(scope)) => BindingTarget::Scope(scope),
            _ => return Err(Error::new(ErrorCode::SchemaInvalid).at_lsn(frame.header.lsn)),
        };
        let effective_time_ns = if verified.envelope.event_time_ns == 0 {
            frame.header.wall_timestamp_ns
        } else {
            UtcNanos::new(verified.envelope.event_time_ns)
        };
        let record = BindingRecord {
            target,
            canonical_entity: value.canonical_entity,
            property: value.property,
            evidence_lsn: LSN::new(value.evidence_lsn),
            revision: value.revision,
            freshness_requirement_ns: value.freshness_requirement_ns,
            effective_time_ns,
            binding_lsn: frame.header.lsn,
        };
        let key = binding_key(&record.target, &record.canonical_entity, &record.property)?;
        let snapshot = store.begin_snapshot()?;
        let existing = snapshot.get(ProjectionId::Bindings, &key)?;
        if existing.is_none()
            && snapshot
                .scan_prefix(
                    ProjectionId::Bindings,
                    &target_prefix(&record.target)?,
                    MAXIMUM_REQUIRED_BINDINGS,
                )?
                .len()
                >= MAXIMUM_REQUIRED_BINDINGS
        {
            return Err(Error::new(ErrorCode::CapacityExceeded).at_lsn(frame.header.lsn));
        }
        let should_replace = existing
            .as_deref()
            .map(decode_value)
            .transpose()?
            .is_none_or(|current| {
                (record.effective_time_ns, record.binding_lsn)
                    >= (current.effective_time_ns, current.binding_lsn)
            });
        drop(snapshot);
        let mutations = if should_replace {
            vec![Mutation::put(key, encode_value(&record)?)]
        } else {
            Vec::new()
        };
        store.apply(ProjectionId::Bindings, frame.header.lsn, &mutations)
    }

    pub fn rebuild(
        store: &ProjectionStore,
        frames: &[Frame],
        reset: bool,
        maximum_frames: usize,
    ) -> Result<BindingsRebuildProgress, Error> {
        if reset {
            store.reset(ProjectionId::Bindings)?;
        }
        let snapshot = store.begin_snapshot()?;
        let checkpoint = snapshot.checkpoint(ProjectionId::Bindings)?.get();
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
        Ok(BindingsRebuildProgress {
            applied_lsn: LSN::new(applied_lsn),
            applied_frames: count,
            complete: applied_lsn == frames.len() as u64,
        })
    }

    pub fn read_target(
        snapshot: &ReadSnapshot<'_>,
        target: &BindingTarget,
    ) -> Result<Vec<BindingRecord>, Error> {
        snapshot
            .scan_prefix(
                ProjectionId::Bindings,
                &target_prefix(target)?,
                MAXIMUM_REQUIRED_BINDINGS,
            )?
            .iter()
            .map(|item| decode_record(target.clone(), &item.key, &item.value))
            .collect()
    }
}

pub fn resolve_required_bindings(
    snapshot: &ReadSnapshot<'_>,
    task: &[u8],
    requirements: &[BindingRequirement],
    now_ns: UtcNanos,
) -> Result<Vec<BindingResolution>, Error> {
    if task.is_empty() || requirements.len() > MAXIMUM_REQUIRED_BINDINGS {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let mut required_identities = BTreeSet::new();
    for requirement in requirements {
        if requirement.canonical_entity.is_empty()
            || requirement.property.is_empty()
            || requirement.revision.as_ref().is_some_and(Vec::is_empty)
            || requirement.freshness_requirement_ns == Some(0)
            || !required_identities.insert((
                requirement.canonical_entity.as_str(),
                requirement.property.as_str(),
            ))
        {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
    }
    let target = BindingTarget::Task(task.to_vec());
    let records = BindingsProjection::read_target(snapshot, &target)?;
    let by_identity: BTreeMap<(&str, &str), &BindingRecord> = records
        .iter()
        .map(|record| {
            (
                (record.canonical_entity.as_str(), record.property.as_str()),
                record,
            )
        })
        .collect();
    requirements
        .iter()
        .map(|requirement| {
            let binding = by_identity
                .get(&(
                    requirement.canonical_entity.as_str(),
                    requirement.property.as_str(),
                ))
                .copied();
            let status = match binding {
                None => BindingStatus::Missing,
                Some(record)
                    if requirement
                        .revision
                        .as_ref()
                        .is_some_and(|revision| *revision != record.revision) =>
                {
                    BindingStatus::Conflicting
                }
                Some(record)
                    if is_stale(
                        record,
                        requirement
                            .freshness_requirement_ns
                            .unwrap_or(record.freshness_requirement_ns),
                        now_ns,
                    ) =>
                {
                    BindingStatus::Stale
                }
                Some(_) => BindingStatus::Resolved,
            };
            Ok(BindingResolution {
                requirement: requirement.clone(),
                status,
                binding: binding.cloned(),
            })
        })
        .collect()
}

fn is_stale(record: &BindingRecord, freshness_ns: u64, now_ns: UtcNanos) -> bool {
    now_ns
        .get()
        .checked_sub(record.effective_time_ns.get())
        .and_then(|age| u64::try_from(age).ok())
        .is_some_and(|age| age > freshness_ns)
}

fn target_prefix(target: &BindingTarget) -> Result<Vec<u8>, Error> {
    let (prefix, identifier) = match target {
        BindingTarget::Task(value) => (TASK_PREFIX, value.as_slice()),
        BindingTarget::Scope(value) => (SCOPE_PREFIX, value.as_slice()),
    };
    if identifier.is_empty() {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let length =
        u32::try_from(identifier.len()).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
    let mut key = Vec::with_capacity(5 + identifier.len());
    key.push(prefix);
    key.extend_from_slice(&length.to_be_bytes());
    key.extend_from_slice(identifier);
    Ok(key)
}

fn binding_key(target: &BindingTarget, entity: &str, property: &str) -> Result<Vec<u8>, Error> {
    let mut key = target_prefix(target)?;
    let length =
        u32::try_from(entity.len()).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
    key.extend_from_slice(&length.to_be_bytes());
    key.extend_from_slice(entity.as_bytes());
    key.extend_from_slice(property.as_bytes());
    Ok(key)
}

fn encode_value(record: &BindingRecord) -> Result<Vec<u8>, Error> {
    let revision_length = u32::try_from(record.revision.len())
        .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
    let mut value = Vec::with_capacity(36 + record.revision.len());
    value.extend_from_slice(&record.effective_time_ns.get().to_le_bytes());
    value.extend_from_slice(&record.binding_lsn.get().to_le_bytes());
    value.extend_from_slice(&record.evidence_lsn.get().to_le_bytes());
    value.extend_from_slice(&record.freshness_requirement_ns.to_le_bytes());
    value.extend_from_slice(&revision_length.to_le_bytes());
    value.extend_from_slice(&record.revision);
    Ok(value)
}

fn decode_record(target: BindingTarget, key: &[u8], value: &[u8]) -> Result<BindingRecord, Error> {
    let prefix = target_prefix(&target)?;
    if !key.starts_with(&prefix) || key.len() < prefix.len() + 4 {
        return Err(Error::new(ErrorCode::InvariantViolation));
    }
    let entity_length = usize::try_from(u32::from_be_bytes(
        key[prefix.len()..prefix.len() + 4]
            .try_into()
            .map_err(|_| Error::new(ErrorCode::InvariantViolation))?,
    ))
    .map_err(|_| Error::new(ErrorCode::InvariantViolation))?;
    let entity_start = prefix.len() + 4;
    let entity_end = entity_start
        .checked_add(entity_length)
        .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))?;
    let canonical_entity = String::from_utf8(
        key.get(entity_start..entity_end)
            .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))?
            .to_vec(),
    )
    .map_err(|_| Error::new(ErrorCode::InvariantViolation))?;
    let property = String::from_utf8(
        key.get(entity_end..)
            .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))?
            .to_vec(),
    )
    .map_err(|_| Error::new(ErrorCode::InvariantViolation))?;
    let decoded = decode_value(value)?;
    Ok(BindingRecord {
        target,
        canonical_entity,
        property,
        evidence_lsn: decoded.evidence_lsn,
        revision: decoded.revision,
        freshness_requirement_ns: decoded.freshness_requirement_ns,
        effective_time_ns: decoded.effective_time_ns,
        binding_lsn: decoded.binding_lsn,
    })
}

fn decode_value(value: &[u8]) -> Result<BindingRecord, Error> {
    if value.len() < 36 {
        return Err(Error::new(ErrorCode::InvariantViolation));
    }
    let effective_time_ns = UtcNanos::new(read_i64(value, 0)?);
    let binding_lsn = LSN::new(read_u64(value, 8)?);
    let evidence_lsn = LSN::new(read_u64(value, 16)?);
    let freshness_requirement_ns = read_u64(value, 24)?;
    let revision_length = usize::try_from(read_u32(value, 32)?)
        .map_err(|_| Error::new(ErrorCode::InvariantViolation))?;
    if binding_lsn.get() == 0
        || evidence_lsn.get() == 0
        || freshness_requirement_ns == 0
        || revision_length == 0
        || value.len() != 36 + revision_length
    {
        return Err(Error::new(ErrorCode::InvariantViolation));
    }
    Ok(BindingRecord {
        target: BindingTarget::Task(Vec::new()),
        canonical_entity: String::new(),
        property: String::new(),
        evidence_lsn,
        revision: value[36..].to_vec(),
        freshness_requirement_ns,
        effective_time_ns,
        binding_lsn,
    })
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, Error> {
    bytes
        .get(offset..offset + 4)
        .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))?
        .try_into()
        .map(u32::from_le_bytes)
        .map_err(|_| Error::new(ErrorCode::InvariantViolation))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, Error> {
    bytes
        .get(offset..offset + 8)
        .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))?
        .try_into()
        .map(u64::from_le_bytes)
        .map_err(|_| Error::new(ErrorCode::InvariantViolation))
}

fn read_i64(bytes: &[u8], offset: usize) -> Result<i64, Error> {
    bytes
        .get(offset..offset + 8)
        .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))?
        .try_into()
        .map(i64::from_le_bytes)
        .map_err(|_| Error::new(ErrorCode::InvariantViolation))
}
