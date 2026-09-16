#![allow(clippy::missing_errors_doc)]

use crate::checkpoint::verify_frame;
use crate::generation::{decode, encode};
use crate::store::{Mutation, ProjectionId, ProjectionStore, ReadSnapshot};
use hm_core::{Error, ErrorCode};
use hm_ledger::frame::{EventKind, Frame};
use hm_schema::events::{EventPayload, ProcedureSupport};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

const PROCEDURE_PREFIX: u8 = b'P';
const IMPORTED_PREFIX: u8 = b'I';
const BODY_PREFIX: u8 = b'B';

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ProcedureState {
    Tentative,
    Supported,
    Adopted,
    Imported,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProcedureRecord {
    pub procedure_id: Vec<u8>,
    pub strategy: String,
    pub expected_outcomes: Vec<String>,
    pub preconditions: Vec<String>,
    pub supports: Vec<ProcedureSupport>,
    pub failures: Vec<u64>,
    pub counterexamples: Vec<u64>,
    pub state: ProcedureState,
    pub version_lsn: u64,
    pub previous_lsn: u64,
    pub adopted_lsn: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlaybookMetadata {
    pub procedure_id: Vec<u8>,
    pub name: String,
    pub declared_tools: Vec<String>,
    pub source_uri: String,
    pub source_digest: Vec<u8>,
    pub playbook_version: u16,
    pub instruction_bytes: u64,
    pub imported_lsn: u64,
}

pub struct ProceduresProjection;

impl ProceduresProjection {
    #[allow(clippy::too_many_lines)]
    pub fn apply_event(store: &ProjectionStore, frame: &Frame) -> Result<(), Error> {
        if !matches!(
            frame.header.kind,
            EventKind::ProcedureMined
                | EventKind::ProcedureRevised
                | EventKind::ProcedureAdopted
                | EventKind::ProcedureImported
        ) {
            return store.apply(ProjectionId::Procedures, frame.header.lsn, &[]);
        }
        let snapshot = store.begin_snapshot()?;
        let envelope = verify_frame(&snapshot, frame)?;
        let mut mutations = Vec::new();
        match envelope.payload {
            EventPayload::ProcedureMined(value) => {
                let key = procedure_key(&value.procedure_id);
                if snapshot.get(ProjectionId::Procedures, &key)?.is_some() {
                    return Err(Error::new(ErrorCode::AlreadyExists).at_lsn(frame.header.lsn));
                }
                let record = ProcedureRecord {
                    procedure_id: value.procedure_id,
                    strategy: value.strategy,
                    expected_outcomes: value.expected_outcomes,
                    preconditions: value.preconditions,
                    state: supported_state(&value.supports),
                    supports: value.supports,
                    failures: value.failures.unwrap_or_default(),
                    counterexamples: value.counterexamples.unwrap_or_default(),
                    version_lsn: frame.header.lsn.get(),
                    previous_lsn: 0,
                    adopted_lsn: 0,
                };
                mutations.push(Mutation::put(key, encode(&record)?));
            }
            EventPayload::ProcedureRevised(value) => {
                let key = procedure_key(&value.procedure_id);
                let previous = snapshot
                    .get(ProjectionId::Procedures, &key)?
                    .ok_or_else(|| {
                        Error::new(ErrorCode::OrderingViolation).at_lsn(frame.header.lsn)
                    })
                    .and_then(|bytes| decode::<ProcedureRecord>(&bytes))?;
                if previous.version_lsn != value.previous_lsn
                    || previous.state == ProcedureState::Adopted
                {
                    return Err(Error::new(ErrorCode::IdempotencyConflict).at_lsn(frame.header.lsn));
                }
                let record = ProcedureRecord {
                    procedure_id: value.procedure_id,
                    strategy: value.strategy,
                    expected_outcomes: value.expected_outcomes,
                    preconditions: value.preconditions,
                    state: supported_state(&value.supports),
                    supports: value.supports,
                    failures: value.failures.unwrap_or_default(),
                    counterexamples: value.counterexamples.unwrap_or_default(),
                    version_lsn: frame.header.lsn.get(),
                    previous_lsn: previous.version_lsn,
                    adopted_lsn: 0,
                };
                mutations.push(Mutation::put(key, encode(&record)?));
            }
            EventPayload::ProcedureAdopted(value) => {
                let key = procedure_key(&value.procedure_id);
                let mut record = snapshot
                    .get(ProjectionId::Procedures, &key)?
                    .ok_or_else(|| {
                        Error::new(ErrorCode::OrderingViolation).at_lsn(frame.header.lsn)
                    })
                    .and_then(|bytes| decode::<ProcedureRecord>(&bytes))?;
                if record.version_lsn != value.procedure_lsn
                    || !matches!(
                        record.state,
                        ProcedureState::Supported | ProcedureState::Imported
                    )
                {
                    return Err(Error::new(ErrorCode::OrderingViolation).at_lsn(frame.header.lsn));
                }
                record.state = ProcedureState::Adopted;
                record.adopted_lsn = frame.header.lsn.get();
                mutations.push(Mutation::put(key, encode(&record)?));
            }
            EventPayload::ProcedureImported(value) => {
                let key = procedure_key(&value.procedure_id);
                if snapshot.get(ProjectionId::Procedures, &key)?.is_some() {
                    return Err(Error::new(ErrorCode::AlreadyExists).at_lsn(frame.header.lsn));
                }
                let metadata = PlaybookMetadata {
                    procedure_id: value.procedure_id.clone(),
                    name: value.name,
                    declared_tools: value.declared_tools,
                    source_uri: value.source_uri,
                    source_digest: value.source_digest,
                    playbook_version: value.playbook_version,
                    instruction_bytes: value.instructions.len() as u64,
                    imported_lsn: frame.header.lsn.get(),
                };
                let record = ProcedureRecord {
                    procedure_id: value.procedure_id.clone(),
                    strategy: value.strategy,
                    expected_outcomes: value.expected_outcomes,
                    preconditions: value.preconditions,
                    supports: Vec::new(),
                    failures: Vec::new(),
                    counterexamples: Vec::new(),
                    state: ProcedureState::Imported,
                    version_lsn: frame.header.lsn.get(),
                    previous_lsn: 0,
                    adopted_lsn: 0,
                };
                mutations.push(Mutation::put(key, encode(&record)?));
                mutations.push(Mutation::put(
                    prefixed_key(IMPORTED_PREFIX, &value.procedure_id),
                    encode(&metadata)?,
                ));
                mutations.push(Mutation::put(
                    prefixed_key(BODY_PREFIX, &value.procedure_id),
                    value.instructions,
                ));
            }
            _ => return Err(Error::new(ErrorCode::InvalidKind).at_lsn(frame.header.lsn)),
        }
        drop(snapshot);
        store.apply(ProjectionId::Procedures, frame.header.lsn, &mutations)
    }

    pub fn get(
        snapshot: &ReadSnapshot<'_>,
        procedure_id: &[u8],
    ) -> Result<Option<ProcedureRecord>, Error> {
        snapshot
            .get(ProjectionId::Procedures, &procedure_key(procedure_id))?
            .map(|bytes| decode(&bytes))
            .transpose()
    }

    pub fn playbook_metadata(
        snapshot: &ReadSnapshot<'_>,
        procedure_id: &[u8],
    ) -> Result<Option<PlaybookMetadata>, Error> {
        snapshot
            .get(
                ProjectionId::Procedures,
                &prefixed_key(IMPORTED_PREFIX, procedure_id),
            )?
            .map(|bytes| decode(&bytes))
            .transpose()
    }

    pub fn playbook_instructions(
        snapshot: &ReadSnapshot<'_>,
        procedure_id: &[u8],
    ) -> Result<Option<Vec<u8>>, Error> {
        snapshot.get(
            ProjectionId::Procedures,
            &prefixed_key(BODY_PREFIX, procedure_id),
        )
    }

    pub fn list(snapshot: &ReadSnapshot<'_>, limit: usize) -> Result<Vec<ProcedureRecord>, Error> {
        if !(1..=4096).contains(&limit) {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        snapshot
            .scan_prefix(ProjectionId::Procedures, &[PROCEDURE_PREFIX], limit)?
            .into_iter()
            .map(|entry| decode(&entry.value))
            .collect()
    }
}

fn supported_state(supports: &[ProcedureSupport]) -> ProcedureState {
    let roots = supports
        .iter()
        .map(|support| support.source_root.as_slice())
        .collect::<BTreeSet<_>>();
    let conversations = supports
        .iter()
        .map(|support| support.conversation.as_slice())
        .collect::<BTreeSet<_>>();
    if roots.len() >= 3 && conversations.len() >= 2 {
        ProcedureState::Supported
    } else {
        ProcedureState::Tentative
    }
}

fn procedure_key(id: &[u8]) -> Vec<u8> {
    prefixed_key(PROCEDURE_PREFIX, id)
}

fn prefixed_key(prefix: u8, id: &[u8]) -> Vec<u8> {
    let mut key = Vec::with_capacity(id.len() + 1);
    key.push(prefix);
    key.extend_from_slice(id);
    key
}
