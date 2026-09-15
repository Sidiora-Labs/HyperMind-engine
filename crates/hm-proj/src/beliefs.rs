#![allow(clippy::missing_errors_doc)]

use crate::protected::{SecurityEvent, enforce_direct_write};
use crate::store::{Mutation, ProjectionId, ProjectionStore, ReadSnapshot};
use hm_core::{Error, ErrorCode, LSN};
use hm_ledger::frame::{EventKind, Frame};
use hm_schema::event::{self, Boundary, EventHistory};
use hm_schema::events::{
    Assertion, AssertionClaim, Authority, BeliefType, EventEnvelope, EventPayload,
    ProposedAssertion, ProvenanceRange, ResultStatus, Retract,
};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const HEAD_PREFIX: u8 = b'H';
const VERSION_PREFIX: u8 = b'V';
const ID_PREFIX: u8 = b'I';
const DOMAIN_PREFIX: u8 = b'D';
const CONFLICT_PREFIX: u8 = b'X';
const TOOL_CALL_PREFIX: u8 = b'C';
const TOOL_RESULT_PREFIX: u8 = b'T';
const PENDING_PREFIX: u8 = b'P';
const PENDING_ID_PREFIX: u8 = b'Q';
const SECURITY_PREFIX: u8 = b'S';

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BeliefProvenance {
    pub first_lsn: u64,
    pub last_lsn: u64,
    pub byte_start: u32,
    pub byte_end: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BeliefConflictEdge {
    pub other_type: BeliefType,
    pub other_canonical_identity: String,
    pub created_lsn: u64,
    pub resolved_lsn: u64,
    pub obligated_surfacing: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BeliefRecord {
    pub belief_type: BeliefType,
    pub belief_id: Vec<u8>,
    pub canonical_identity: String,
    pub conflict_domain: String,
    pub value: Vec<u8>,
    pub claim: AssertionClaim,
    pub event_time_ns: i64,
    pub observation_lsn: u64,
    pub valid_from_ns: i64,
    pub valid_to_ns: i64,
    pub version: u64,
    pub supersedes_version: u64,
    pub provenance: Vec<BeliefProvenance>,
    pub conflict_edges: Vec<BeliefConflictEdge>,
    pub authority: Authority,
    pub tombstoned: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PendingProposal {
    pub belief_id: Vec<u8>,
    pub belief_type: BeliefType,
    pub canonical_identity: String,
    pub conflict_domain: String,
    pub value: Vec<u8>,
    pub claim: AssertionClaim,
    pub event_time_ns: i64,
    pub observation_lsn: u64,
    pub valid_from_ns: i64,
    pub valid_to_ns: i64,
    pub provenance: Vec<BeliefProvenance>,
    pub authority: Authority,
    pub run_id: Option<Vec<u8>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BeliefAsOf {
    ValidAt(i64),
    KnownAt(LSN),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BeliefAsOfAxis {
    ValidTime,
    KnownLsn,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BeliefAsOfResult {
    pub axis: BeliefAsOfAxis,
    pub record: Option<BeliefRecord>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BeliefRebuildProgress {
    pub applied_lsn: LSN,
    pub applied_frames: usize,
    pub complete: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct StoredConflict {
    other_head_key: Vec<u8>,
    created_lsn: u64,
    resolved_lsn: u64,
}

#[derive(Clone)]
struct WorkingSet<'snapshot, 'environment> {
    snapshot: &'snapshot ReadSnapshot<'environment>,
    heads: BTreeMap<Vec<u8>, Option<BeliefRecord>>,
    ids: BTreeMap<Vec<u8>, Option<Vec<u8>>>,
    domains: BTreeMap<Vec<u8>, Vec<Vec<u8>>>,
    conflicts: BTreeMap<Vec<u8>, Vec<StoredConflict>>,
    dirty_domains: BTreeSet<Vec<u8>>,
    dirty_conflicts: BTreeSet<Vec<u8>>,
    mutations: Vec<Mutation>,
}

impl<'snapshot, 'environment> WorkingSet<'snapshot, 'environment> {
    fn new(snapshot: &'snapshot ReadSnapshot<'environment>) -> Self {
        Self {
            snapshot,
            heads: BTreeMap::new(),
            ids: BTreeMap::new(),
            domains: BTreeMap::new(),
            conflicts: BTreeMap::new(),
            dirty_domains: BTreeSet::new(),
            dirty_conflicts: BTreeSet::new(),
            mutations: Vec::new(),
        }
    }

    fn head(&mut self, key: &[u8]) -> Result<Option<BeliefRecord>, Error> {
        if !self.heads.contains_key(key) {
            self.heads
                .insert(key.to_vec(), load_head(self.snapshot, key)?);
        }
        Ok(self.heads.get(key).cloned().flatten())
    }

    fn id_head(&mut self, key: &[u8]) -> Result<Option<Vec<u8>>, Error> {
        if !self.ids.contains_key(key) {
            self.ids.insert(
                key.to_vec(),
                self.snapshot.get(ProjectionId::BeliefStore, key)?,
            );
        }
        Ok(self.ids.get(key).cloned().flatten())
    }

    fn domain(&mut self, key: &[u8]) -> Result<Vec<Vec<u8>>, Error> {
        if !self.domains.contains_key(key) {
            let members = self
                .snapshot
                .get(ProjectionId::BeliefStore, key)?
                .map_or_else(|| Ok(Vec::new()), |bytes| decode(&bytes))?;
            validate_sorted_keys(&members)?;
            self.domains.insert(key.to_vec(), members);
        }
        self.domains
            .get(key)
            .cloned()
            .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))
    }

    fn conflicts(&mut self, head: &[u8]) -> Result<Vec<StoredConflict>, Error> {
        if !self.conflicts.contains_key(head) {
            let values = self
                .snapshot
                .get(ProjectionId::BeliefStore, &conflict_key(head))?
                .map_or_else(|| Ok(Vec::new()), |bytes| decode(&bytes))?;
            validate_conflicts(&values)?;
            self.conflicts.insert(head.to_vec(), values);
        }
        self.conflicts
            .get(head)
            .cloned()
            .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))
    }

    fn finish(mut self) -> Result<Vec<Mutation>, Error> {
        for key in self.dirty_domains {
            let members = self
                .domains
                .get(&key)
                .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))?;
            self.mutations.push(Mutation::put(key, encode(members)?));
        }
        for head in self.dirty_conflicts {
            let conflicts = self
                .conflicts
                .get(&head)
                .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))?;
            self.mutations
                .push(Mutation::put(conflict_key(&head), encode(conflicts)?));
        }
        Ok(self.mutations)
    }
}

pub struct BeliefProjection;

impl BeliefProjection {
    #[allow(clippy::too_many_lines)]
    pub fn apply_event(store: &ProjectionStore, frame: &Frame) -> Result<(), Error> {
        let snapshot = store.begin_snapshot()?;
        let mutations = match frame.header.kind {
            EventKind::ToolCall => {
                let envelope = verify(frame, event::EventKind::ToolCall, &snapshot)?;
                let EventPayload::ToolCall(_) = envelope.payload else {
                    return Err(Error::new(ErrorCode::InvariantViolation).at_lsn(frame.header.lsn));
                };
                vec![Mutation::put(
                    marker_key(TOOL_CALL_PREFIX, frame.header.lsn),
                    [1],
                )]
            }
            EventKind::ToolResult => {
                let envelope = verify(frame, event::EventKind::ToolResult, &snapshot)?;
                let EventPayload::ToolResult(result) = envelope.payload else {
                    return Err(Error::new(ErrorCode::InvariantViolation).at_lsn(frame.header.lsn));
                };
                if result.status == ResultStatus::Ok
                    && envelope.authority == Authority::ToolObserved
                {
                    vec![Mutation::put(
                        marker_key(TOOL_RESULT_PREFIX, frame.header.lsn),
                        [1],
                    )]
                } else {
                    Vec::new()
                }
            }
            EventKind::Assertion => {
                let envelope = verify(frame, event::EventKind::Assertion, &snapshot)?;
                let EventPayload::Assertion(assertion) = &envelope.payload else {
                    return Err(Error::new(ErrorCode::InvariantViolation).at_lsn(frame.header.lsn));
                };
                if let Err(security) = enforce_direct_write(
                    assertion.belief_type,
                    &assertion.belief_id,
                    envelope.authority,
                    envelope.run_id.as_deref(),
                    frame.header.lsn,
                ) {
                    security_mutation(&security, 0)?
                } else {
                    let mut working = WorkingSet::new(&snapshot);
                    match apply_assertion(&mut working, assertion, &envelope, frame.header.lsn) {
                        Ok(()) => {
                            clear_pending_head(
                                &mut working,
                                assertion.belief_type,
                                &assertion.canonical_identity,
                            )?;
                            working.finish()?
                        }
                        Err(error) if policy_rejection(error.code) => Vec::new(),
                        Err(error) => return Err(error),
                    }
                }
            }
            EventKind::Consolidation => {
                let envelope = verify(frame, event::EventKind::Consolidation, &snapshot)?;
                let EventPayload::Consolidation(consolidation) = &envelope.payload else {
                    return Err(Error::new(ErrorCode::InvariantViolation).at_lsn(frame.header.lsn));
                };
                let mut working = WorkingSet::new(&snapshot);
                let mut security = Vec::new();
                let mut policy_rejected = false;
                for assertion in &consolidation.assertions {
                    if let Err(event) = enforce_direct_write(
                        assertion.belief_type,
                        &assertion.belief_id,
                        envelope.authority,
                        envelope.run_id.as_deref(),
                        frame.header.lsn,
                    ) {
                        security.push(event);
                        break;
                    }
                    if let Err(error) =
                        apply_assertion(&mut working, assertion, &envelope, frame.header.lsn)
                    {
                        if policy_rejection(error.code) {
                            policy_rejected = true;
                            break;
                        }
                        return Err(error);
                    }
                }
                if policy_rejected {
                    Vec::new()
                } else if security.is_empty() {
                    working.finish()?
                } else {
                    security_mutation(&security[0], 0)?
                }
            }
            EventKind::Retract => {
                let envelope = verify(frame, event::EventKind::Retract, &snapshot)?;
                let EventPayload::Retract(retract) = &envelope.payload else {
                    return Err(Error::new(ErrorCode::InvariantViolation).at_lsn(frame.header.lsn));
                };
                let mut working = WorkingSet::new(&snapshot);
                match apply_retract(&mut working, retract, &envelope, frame.header.lsn) {
                    Ok(()) => working.finish()?,
                    Err(ApplyRetractError::Security(security)) => security_mutation(&security, 0)?,
                    Err(ApplyRetractError::Policy) => Vec::new(),
                    Err(ApplyRetractError::Fatal(error)) => return Err(error),
                }
            }
            EventKind::ProposedAssertion => {
                let envelope = verify(frame, event::EventKind::ProposedAssertion, &snapshot)?;
                let EventPayload::ProposedAssertion(proposal) = &envelope.payload else {
                    return Err(Error::new(ErrorCode::InvariantViolation).at_lsn(frame.header.lsn));
                };
                if proposal.claim == AssertionClaim::NegativeExistence
                    && !has_tool_evidence(&snapshot, &proposal.provenance, frame.header.lsn)?
                {
                    Vec::new()
                } else {
                    store_proposal(&snapshot, proposal, &envelope, frame.header.lsn)?
                }
            }
            _ => Vec::new(),
        };
        drop(snapshot);
        store.apply_belief(frame.header.lsn, &mutations)
    }

    pub fn rebuild(
        store: &ProjectionStore,
        frames: &[Frame],
        reset: bool,
        maximum_frames: usize,
    ) -> Result<BeliefRebuildProgress, Error> {
        if reset {
            store.reset(ProjectionId::BeliefStore)?;
        }
        let checkpoint = store
            .begin_snapshot()?
            .checkpoint(ProjectionId::BeliefStore)?
            .get();
        let begin = usize::try_from(checkpoint)
            .map_err(|_| Error::new(ErrorCode::CapacityExceeded).at_lsn(LSN::new(checkpoint)))?;
        if begin > frames.len() {
            return Err(Error::new(ErrorCode::ProjectionCheckpoint)
                .at_lsn(LSN::new(checkpoint))
                .at_offset(frames.len() as u64));
        }
        let apply_count = (frames.len() - begin).min(maximum_frames);
        for (offset, frame) in frames[begin..begin + apply_count].iter().enumerate() {
            let expected = checkpoint
                .checked_add(offset as u64)
                .and_then(|value| value.checked_add(1))
                .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?;
            if frame.header.lsn.get() != expected {
                return Err(Error::new(ErrorCode::SequenceViolation).at_lsn(frame.header.lsn));
            }
            Self::apply_event(store, frame)?;
        }
        let applied_lsn = checkpoint
            .checked_add(apply_count as u64)
            .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?;
        Ok(BeliefRebuildProgress {
            applied_lsn: LSN::new(applied_lsn),
            applied_frames: apply_count,
            complete: applied_lsn == frames.len() as u64,
        })
    }

    pub fn read_as_of(
        snapshot: &ReadSnapshot<'_>,
        belief_type: BeliefType,
        canonical_identity: &str,
        as_of: BeliefAsOf,
    ) -> Result<BeliefAsOfResult, Error> {
        if canonical_identity.is_empty() {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let checkpoint = snapshot.checkpoint(ProjectionId::BeliefStore)?.get();
        let (axis, known_lsn, valid_time_ns) = match as_of {
            BeliefAsOf::ValidAt(valid_time_ns) => {
                (BeliefAsOfAxis::ValidTime, checkpoint, Some(valid_time_ns))
            }
            BeliefAsOf::KnownAt(known_lsn) if known_lsn.get() != 0 => {
                (BeliefAsOfAxis::KnownLsn, known_lsn.get(), None)
            }
            BeliefAsOf::KnownAt(_) => return Err(Error::new(ErrorCode::InvalidArgument)),
        };
        let head = head_key(belief_type, canonical_identity)?;
        let Some(current) = load_head(snapshot, &head)? else {
            return Ok(BeliefAsOfResult { axis, record: None });
        };
        let mut version = current.version;
        while version != 0 {
            let mut record = load_version(snapshot, &head, version)?;
            if record.observation_lsn <= known_lsn {
                if record.tombstoned {
                    return Ok(BeliefAsOfResult { axis, record: None });
                }
                if valid_time_ns.is_none_or(|time| valid_at(&record, time)) {
                    record.conflict_edges = load_conflict_edges(snapshot, &head, known_lsn)?;
                    return Ok(BeliefAsOfResult {
                        axis,
                        record: Some(record),
                    });
                }
            }
            version = record.supersedes_version;
        }
        Ok(BeliefAsOfResult { axis, record: None })
    }

    pub fn read_heads(
        snapshot: &ReadSnapshot<'_>,
        limit: usize,
    ) -> Result<Vec<BeliefRecord>, Error> {
        if limit == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let known_lsn = snapshot.checkpoint(ProjectionId::BeliefStore)?.get();
        let items = snapshot.scan_prefix(ProjectionId::BeliefStore, &[HEAD_PREFIX], limit)?;
        let mut heads = Vec::with_capacity(items.len());
        for item in items {
            let Some(mut record) = load_head(snapshot, &item.key)? else {
                return Err(Error::new(ErrorCode::BeliefCorrupt));
            };
            if !record.tombstoned {
                record.conflict_edges = load_conflict_edges(snapshot, &item.key, known_lsn)?;
                heads.push(record);
            }
        }
        Ok(heads)
    }

    pub fn read_pending(
        snapshot: &ReadSnapshot<'_>,
        limit: usize,
    ) -> Result<Vec<PendingProposal>, Error> {
        if limit == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        snapshot
            .scan_prefix(ProjectionId::BeliefStore, &[PENDING_PREFIX], limit)?
            .into_iter()
            .map(|item| decode(&item.value))
            .collect()
    }

    pub fn read_security_events(
        snapshot: &ReadSnapshot<'_>,
        limit: usize,
    ) -> Result<Vec<SecurityEvent>, Error> {
        if limit == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        snapshot
            .scan_prefix(ProjectionId::BeliefStore, &[SECURITY_PREFIX], limit)?
            .into_iter()
            .map(|item| decode(&item.value))
            .collect()
    }
}

fn apply_assertion(
    working: &mut WorkingSet<'_, '_>,
    assertion: &Assertion,
    envelope: &EventEnvelope,
    observation_lsn: LSN,
) -> Result<(), Error> {
    if assertion.claim == AssertionClaim::NegativeExistence
        && !has_tool_evidence(working.snapshot, &assertion.provenance, observation_lsn)?
    {
        return Err(Error::new(ErrorCode::NegativeExistenceUncorroborated).at_lsn(observation_lsn));
    }
    let head = head_key(assertion.belief_type, &assertion.canonical_identity)?;
    let id = id_key(&assertion.belief_id)?;
    if working.id_head(&id)?.is_some_and(|indexed| indexed != head) {
        return Err(Error::new(ErrorCode::BeliefIdConflict).at_lsn(observation_lsn));
    }
    let current = working.head(&head)?;
    let incoming_domain = assertion.conflict_domain.clone().unwrap_or_default();
    if current
        .as_ref()
        .is_some_and(|record| record.conflict_domain != incoming_domain)
    {
        return Err(Error::new(ErrorCode::BeliefIdConflict).at_lsn(observation_lsn));
    }
    if let Some(record) = &current
        && record.belief_id == assertion.belief_id
    {
        if same_assertion(record, assertion, envelope) {
            return Ok(());
        }
        return Err(Error::new(ErrorCode::BeliefIdConflict).at_lsn(observation_lsn));
    }

    let mut conflicting_heads = Vec::new();
    let domain = if incoming_domain.is_empty() {
        None
    } else {
        let key = domain_key(assertion.belief_type, &incoming_domain)?;
        let members = working.domain(&key)?;
        for member in &members {
            if member == &head {
                continue;
            }
            let existing = working
                .head(member)?
                .ok_or_else(|| Error::new(ErrorCode::BeliefCorrupt))?;
            if !existing.tombstoned
                && existing.belief_type == assertion.belief_type
                && existing.conflict_domain == incoming_domain
                && existing.canonical_identity != assertion.canonical_identity
            {
                conflicting_heads.push(member.clone());
            }
        }
        Some((key, members))
    };

    let version = current.as_ref().map_or(1, |record| record.version + 1);
    let record = BeliefRecord {
        belief_type: assertion.belief_type,
        belief_id: assertion.belief_id.clone(),
        canonical_identity: assertion.canonical_identity.clone(),
        conflict_domain: incoming_domain,
        value: assertion.value.clone(),
        claim: assertion.claim,
        event_time_ns: envelope.event_time_ns,
        observation_lsn: observation_lsn.get(),
        valid_from_ns: assertion.valid_from_ns,
        valid_to_ns: assertion.valid_to_ns,
        version,
        supersedes_version: current.as_ref().map_or(0, |value| value.version),
        provenance: copy_provenance(&assertion.provenance),
        conflict_edges: Vec::new(),
        authority: envelope.authority,
        tombstoned: false,
    };
    validate_record(&record)?;
    working
        .mutations
        .push(Mutation::put(version_key(&head, version), encode(&record)?));
    working
        .mutations
        .push(Mutation::put(head.clone(), version.to_le_bytes()));
    working
        .mutations
        .push(Mutation::put(id.clone(), head.clone()));
    working.heads.insert(head.clone(), Some(record));
    working.ids.insert(id, Some(head.clone()));

    if let Some((domain_key, mut members)) = domain {
        if members.binary_search(&head).is_err() {
            members.push(head.clone());
            members.sort();
            working.domains.insert(domain_key.clone(), members);
            working.dirty_domains.insert(domain_key);
        }
        for other in conflicting_heads {
            add_conflict(working, &head, &other, observation_lsn)?;
            add_conflict(working, &other, &head, observation_lsn)?;
        }
    }
    Ok(())
}

enum ApplyRetractError {
    Security(SecurityEvent),
    Policy,
    Fatal(Error),
}

fn apply_retract(
    working: &mut WorkingSet<'_, '_>,
    retract: &Retract,
    envelope: &EventEnvelope,
    observation_lsn: LSN,
) -> Result<(), ApplyRetractError> {
    let id = id_key(&retract.belief_id).map_err(ApplyRetractError::Fatal)?;
    let Some(head) = working.id_head(&id).map_err(ApplyRetractError::Fatal)? else {
        if let Some((key, pending)) = pending_for_id(working.snapshot, &retract.belief_id)
            .map_err(ApplyRetractError::Fatal)?
        {
            if let Err(security) = enforce_direct_write(
                pending.belief_type,
                &retract.belief_id,
                envelope.authority,
                envelope.run_id.as_deref(),
                observation_lsn,
            ) {
                return Err(ApplyRetractError::Security(security));
            }
            delete_pending(working, key, &retract.belief_id).map_err(ApplyRetractError::Fatal)?;
            return Ok(());
        }
        return Err(ApplyRetractError::Policy);
    };
    let current = working
        .head(&head)
        .map_err(ApplyRetractError::Fatal)?
        .ok_or_else(|| ApplyRetractError::Fatal(Error::new(ErrorCode::BeliefCorrupt)))?;
    if let Err(security) = enforce_direct_write(
        current.belief_type,
        &retract.belief_id,
        envelope.authority,
        envelope.run_id.as_deref(),
        observation_lsn,
    ) {
        return Err(ApplyRetractError::Security(security));
    }
    if current.belief_id != retract.belief_id {
        return Err(ApplyRetractError::Policy);
    }
    if current.tombstoned {
        return Ok(());
    }
    let mut tombstone = current.clone();
    tombstone.value.clear();
    tombstone.event_time_ns = envelope.event_time_ns;
    tombstone.observation_lsn = observation_lsn.get();
    tombstone.version += 1;
    tombstone.supersedes_version = current.version;
    tombstone.provenance = copy_provenance(&retract.provenance);
    tombstone.authority = envelope.authority;
    tombstone.tombstoned = true;
    validate_record(&tombstone).map_err(ApplyRetractError::Fatal)?;
    working.mutations.push(Mutation::put(
        version_key(&head, tombstone.version),
        encode(&tombstone).map_err(ApplyRetractError::Fatal)?,
    ));
    working
        .mutations
        .push(Mutation::put(head.clone(), tombstone.version.to_le_bytes()));
    working.heads.insert(head.clone(), Some(tombstone));
    resolve_conflicts(working, &head, observation_lsn).map_err(ApplyRetractError::Fatal)?;
    clear_pending_id(working, &retract.belief_id).map_err(ApplyRetractError::Fatal)?;
    Ok(())
}

fn store_proposal(
    snapshot: &ReadSnapshot<'_>,
    proposal: &ProposedAssertion,
    envelope: &EventEnvelope,
    observation_lsn: LSN,
) -> Result<Vec<Mutation>, Error> {
    let proposal = PendingProposal {
        belief_id: proposal.belief_id.clone(),
        belief_type: proposal.belief_type,
        canonical_identity: proposal.canonical_identity.clone(),
        conflict_domain: proposal.conflict_domain.clone().unwrap_or_default(),
        value: proposal.value.clone(),
        claim: proposal.claim,
        event_time_ns: envelope.event_time_ns,
        observation_lsn: observation_lsn.get(),
        valid_from_ns: proposal.valid_from_ns,
        valid_to_ns: proposal.valid_to_ns,
        provenance: copy_provenance(&proposal.provenance),
        authority: envelope.authority,
        run_id: envelope.run_id.clone(),
    };
    let head = head_key(proposal.belief_type, &proposal.canonical_identity)?;
    let key = pending_key(&head, &proposal.belief_id)?;
    let id = pending_id_key(&proposal.belief_id)?;
    if snapshot
        .get(ProjectionId::BeliefStore, &id)?
        .is_some_and(|existing| existing != key)
    {
        return Ok(Vec::new());
    }
    Ok(vec![
        Mutation::put(key.clone(), encode(&proposal)?),
        Mutation::put(id, key),
    ])
}

fn clear_pending_head(
    working: &mut WorkingSet<'_, '_>,
    belief_type: BeliefType,
    canonical_identity: &str,
) -> Result<(), Error> {
    let mut prefix = vec![PENDING_PREFIX];
    prefix.extend_from_slice(&head_key(belief_type, canonical_identity)?);
    for item in working
        .snapshot
        .scan_prefix(ProjectionId::BeliefStore, &prefix, 4096)?
    {
        let proposal: PendingProposal = decode(&item.value)?;
        working.mutations.push(Mutation::delete(item.key));
        working
            .mutations
            .push(Mutation::delete(pending_id_key(&proposal.belief_id)?));
    }
    Ok(())
}

fn clear_pending_id(working: &mut WorkingSet<'_, '_>, belief_id: &[u8]) -> Result<bool, Error> {
    let Some((key, _)) = pending_for_id(working.snapshot, belief_id)? else {
        return Ok(false);
    };
    delete_pending(working, key, belief_id)?;
    Ok(true)
}

fn pending_for_id(
    snapshot: &ReadSnapshot<'_>,
    belief_id: &[u8],
) -> Result<Option<(Vec<u8>, PendingProposal)>, Error> {
    let id = pending_id_key(belief_id)?;
    let Some(key) = snapshot.get(ProjectionId::BeliefStore, &id)? else {
        return Ok(None);
    };
    let bytes = snapshot
        .get(ProjectionId::BeliefStore, &key)?
        .ok_or_else(|| Error::new(ErrorCode::BeliefCorrupt))?;
    let proposal: PendingProposal = decode(&bytes)?;
    if proposal.belief_id != belief_id {
        return Err(Error::new(ErrorCode::BeliefCorrupt));
    }
    Ok(Some((key, proposal)))
}

fn delete_pending(
    working: &mut WorkingSet<'_, '_>,
    key: Vec<u8>,
    belief_id: &[u8],
) -> Result<(), Error> {
    let id = pending_id_key(belief_id)?;
    working.mutations.push(Mutation::delete(key));
    working.mutations.push(Mutation::delete(id));
    Ok(())
}

fn has_tool_evidence(
    snapshot: &ReadSnapshot<'_>,
    provenance: &[ProvenanceRange],
    observation_lsn: LSN,
) -> Result<bool, Error> {
    for range in provenance {
        let Some(last_before) = observation_lsn.get().checked_sub(1) else {
            continue;
        };
        let last = range.last_lsn.min(last_before);
        if range.first_lsn > last {
            continue;
        }
        let lower = marker_key(TOOL_RESULT_PREFIX, LSN::new(range.first_lsn));
        let upper = marker_key(TOOL_RESULT_PREFIX, LSN::new(last));
        if snapshot
            .first_at_or_after(ProjectionId::BeliefStore, &lower)?
            .is_some_and(|item| item.key.as_slice() <= upper.as_slice())
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn add_conflict(
    working: &mut WorkingSet<'_, '_>,
    head: &[u8],
    other: &[u8],
    observation_lsn: LSN,
) -> Result<(), Error> {
    let mut conflicts = working.conflicts(head)?;
    if conflicts
        .iter()
        .any(|edge| edge.other_head_key == other && edge.resolved_lsn == 0)
    {
        return Ok(());
    }
    conflicts.push(StoredConflict {
        other_head_key: other.to_vec(),
        created_lsn: observation_lsn.get(),
        resolved_lsn: 0,
    });
    conflicts.sort_by(|left, right| {
        left.other_head_key
            .cmp(&right.other_head_key)
            .then_with(|| left.created_lsn.cmp(&right.created_lsn))
    });
    working.conflicts.insert(head.to_vec(), conflicts);
    working.dirty_conflicts.insert(head.to_vec());
    Ok(())
}

fn resolve_conflicts(
    working: &mut WorkingSet<'_, '_>,
    head: &[u8],
    observation_lsn: LSN,
) -> Result<(), Error> {
    let mut own = working.conflicts(head)?;
    let peers: Vec<Vec<u8>> = own
        .iter()
        .filter(|edge| edge.resolved_lsn == 0)
        .map(|edge| edge.other_head_key.clone())
        .collect();
    for edge in &mut own {
        if edge.resolved_lsn == 0 {
            edge.resolved_lsn = observation_lsn.get();
        }
    }
    working.conflicts.insert(head.to_vec(), own);
    working.dirty_conflicts.insert(head.to_vec());
    for peer in peers {
        let mut peer_edges = working.conflicts(&peer)?;
        let Some(edge) = peer_edges
            .iter_mut()
            .find(|edge| edge.other_head_key == head && edge.resolved_lsn == 0)
        else {
            return Err(Error::new(ErrorCode::BeliefCorrupt));
        };
        edge.resolved_lsn = observation_lsn.get();
        working.conflicts.insert(peer.clone(), peer_edges);
        working.dirty_conflicts.insert(peer);
    }
    Ok(())
}

fn load_head(snapshot: &ReadSnapshot<'_>, head: &[u8]) -> Result<Option<BeliefRecord>, Error> {
    let Some(bytes) = snapshot.get(ProjectionId::BeliefStore, head)? else {
        return Ok(None);
    };
    if bytes.len() != 8 {
        return Err(Error::new(ErrorCode::BeliefCorrupt));
    }
    let version = u64::from_le_bytes(
        bytes
            .try_into()
            .map_err(|_| Error::new(ErrorCode::BeliefCorrupt))?,
    );
    if version == 0 {
        return Err(Error::new(ErrorCode::BeliefCorrupt));
    }
    load_version(snapshot, head, version).map(Some)
}

fn load_version(
    snapshot: &ReadSnapshot<'_>,
    head: &[u8],
    version: u64,
) -> Result<BeliefRecord, Error> {
    let bytes = snapshot
        .get(ProjectionId::BeliefStore, &version_key(head, version))?
        .ok_or_else(|| Error::new(ErrorCode::BeliefCorrupt))?;
    let record: BeliefRecord = decode(&bytes)?;
    validate_record(&record)?;
    if record.version != version
        || head_key(record.belief_type, &record.canonical_identity)? != head
    {
        return Err(Error::new(ErrorCode::BeliefCorrupt));
    }
    Ok(record)
}

fn load_conflict_edges(
    snapshot: &ReadSnapshot<'_>,
    head: &[u8],
    known_lsn: u64,
) -> Result<Vec<BeliefConflictEdge>, Error> {
    let conflicts: Vec<StoredConflict> = snapshot
        .get(ProjectionId::BeliefStore, &conflict_key(head))?
        .map_or_else(|| Ok(Vec::new()), |bytes| decode(&bytes))?;
    validate_conflicts(&conflicts)?;
    conflicts
        .into_iter()
        .filter(|edge| {
            edge.created_lsn <= known_lsn
                && (edge.resolved_lsn == 0 || edge.resolved_lsn > known_lsn)
        })
        .map(|edge| {
            let (other_type, other_canonical_identity) = decode_head_key(&edge.other_head_key)?;
            Ok(BeliefConflictEdge {
                other_type,
                other_canonical_identity,
                created_lsn: edge.created_lsn,
                resolved_lsn: edge.resolved_lsn,
                obligated_surfacing: true,
            })
        })
        .collect()
}

fn validate_record(record: &BeliefRecord) -> Result<(), Error> {
    if record.belief_id.is_empty()
        || record.canonical_identity.is_empty()
        || record.provenance.is_empty()
        || record.observation_lsn == 0
        || record.version == 0
        || (record.valid_to_ns != 0 && record.valid_to_ns < record.valid_from_ns)
        || (record.version == 1 && record.supersedes_version != 0)
        || (record.version > 1 && record.supersedes_version + 1 != record.version)
        || (record.tombstoned && !record.value.is_empty())
        || (!record.tombstoned && record.value.is_empty())
        || record.provenance.iter().any(|range| {
            range.first_lsn == 0
                || range.last_lsn < range.first_lsn
                || range.byte_end <= range.byte_start
        })
    {
        Err(Error::new(ErrorCode::BeliefCorrupt).at_lsn(LSN::new(record.observation_lsn)))
    } else {
        Ok(())
    }
}

fn validate_sorted_keys(keys: &[Vec<u8>]) -> Result<(), Error> {
    if keys.iter().any(|key| decode_head_key(key).is_err())
        || keys.windows(2).any(|pair| pair[0] >= pair[1])
    {
        Err(Error::new(ErrorCode::BeliefCorrupt))
    } else {
        Ok(())
    }
}

fn validate_conflicts(conflicts: &[StoredConflict]) -> Result<(), Error> {
    if conflicts.iter().any(|edge| {
        decode_head_key(&edge.other_head_key).is_err()
            || edge.created_lsn == 0
            || (edge.resolved_lsn != 0 && edge.resolved_lsn <= edge.created_lsn)
    }) || conflicts.windows(2).any(|pair| {
        (&pair[0].other_head_key, pair[0].created_lsn)
            >= (&pair[1].other_head_key, pair[1].created_lsn)
    }) {
        Err(Error::new(ErrorCode::BeliefCorrupt))
    } else {
        Ok(())
    }
}

fn same_assertion(record: &BeliefRecord, assertion: &Assertion, envelope: &EventEnvelope) -> bool {
    !record.tombstoned
        && record.belief_type == assertion.belief_type
        && record.belief_id == assertion.belief_id
        && record.canonical_identity == assertion.canonical_identity
        && record.conflict_domain == assertion.conflict_domain.clone().unwrap_or_default()
        && record.value == assertion.value
        && record.claim == assertion.claim
        && record.valid_from_ns == assertion.valid_from_ns
        && record.valid_to_ns == assertion.valid_to_ns
        && record.event_time_ns == envelope.event_time_ns
        && record.authority == envelope.authority
        && record.provenance == copy_provenance(&assertion.provenance)
}

fn valid_at(record: &BeliefRecord, valid_time_ns: i64) -> bool {
    record.valid_from_ns <= valid_time_ns
        && (record.valid_to_ns == 0 || valid_time_ns <= record.valid_to_ns)
}

fn copy_provenance(provenance: &[ProvenanceRange]) -> Vec<BeliefProvenance> {
    provenance
        .iter()
        .map(|range| BeliefProvenance {
            first_lsn: range.first_lsn,
            last_lsn: range.last_lsn,
            byte_start: range.byte_start,
            byte_end: range.byte_end,
        })
        .collect()
}

fn head_key(belief_type: BeliefType, canonical_identity: &str) -> Result<Vec<u8>, Error> {
    let length = u32::try_from(canonical_identity.len())
        .map_err(|_| Error::new(ErrorCode::InvalidLength))?;
    let mut key = Vec::with_capacity(canonical_identity.len() + 6);
    key.push(HEAD_PREFIX);
    key.push(belief_type as u8);
    key.extend_from_slice(&length.to_le_bytes());
    key.extend_from_slice(canonical_identity.as_bytes());
    Ok(key)
}

fn decode_head_key(key: &[u8]) -> Result<(BeliefType, String), Error> {
    if key.len() < 6 || key[0] != HEAD_PREFIX {
        return Err(Error::new(ErrorCode::BeliefCorrupt));
    }
    let belief_type =
        BeliefType::try_from(key[1]).map_err(|_| Error::new(ErrorCode::BeliefCorrupt))?;
    let length = u32::from_le_bytes(
        key[2..6]
            .try_into()
            .map_err(|_| Error::new(ErrorCode::BeliefCorrupt))?,
    ) as usize;
    if length == 0 || key.len() != length + 6 {
        return Err(Error::new(ErrorCode::BeliefCorrupt));
    }
    let identity = std::str::from_utf8(&key[6..])
        .map_err(|_| Error::new(ErrorCode::BeliefCorrupt))?
        .to_owned();
    Ok((belief_type, identity))
}

fn version_key(head: &[u8], version: u64) -> Vec<u8> {
    let mut key = head.to_vec();
    key[0] = VERSION_PREFIX;
    key.extend_from_slice(&version.to_le_bytes());
    key
}

fn id_key(belief_id: &[u8]) -> Result<Vec<u8>, Error> {
    length_key(ID_PREFIX, belief_id)
}

fn pending_id_key(belief_id: &[u8]) -> Result<Vec<u8>, Error> {
    length_key(PENDING_ID_PREFIX, belief_id)
}

fn length_key(prefix: u8, bytes: &[u8]) -> Result<Vec<u8>, Error> {
    let length = u32::try_from(bytes.len()).map_err(|_| Error::new(ErrorCode::InvalidLength))?;
    if length == 0 {
        return Err(Error::new(ErrorCode::InvalidArgument));
    }
    let mut key = Vec::with_capacity(bytes.len() + 5);
    key.push(prefix);
    key.extend_from_slice(&length.to_le_bytes());
    key.extend_from_slice(bytes);
    Ok(key)
}

fn domain_key(belief_type: BeliefType, domain: &str) -> Result<Vec<u8>, Error> {
    let length = u32::try_from(domain.len()).map_err(|_| Error::new(ErrorCode::InvalidLength))?;
    let mut key = Vec::with_capacity(domain.len() + 6);
    key.push(DOMAIN_PREFIX);
    key.push(belief_type as u8);
    key.extend_from_slice(&length.to_le_bytes());
    key.extend_from_slice(domain.as_bytes());
    Ok(key)
}

fn conflict_key(head: &[u8]) -> Vec<u8> {
    let mut key = Vec::with_capacity(head.len() + 1);
    key.push(CONFLICT_PREFIX);
    key.extend_from_slice(head);
    key
}

fn marker_key(prefix: u8, lsn: LSN) -> [u8; 9] {
    let mut key = [0_u8; 9];
    key[0] = prefix;
    key[1..].copy_from_slice(&lsn.get().to_be_bytes());
    key
}

fn pending_key(head: &[u8], belief_id: &[u8]) -> Result<Vec<u8>, Error> {
    let length =
        u32::try_from(belief_id.len()).map_err(|_| Error::new(ErrorCode::InvalidLength))?;
    let mut key = Vec::with_capacity(1 + head.len() + 4 + belief_id.len());
    key.push(PENDING_PREFIX);
    key.extend_from_slice(head);
    key.extend_from_slice(&length.to_le_bytes());
    key.extend_from_slice(belief_id);
    Ok(key)
}

fn security_key(lsn: u64, index: u32) -> [u8; 13] {
    let mut key = [0_u8; 13];
    key[0] = SECURITY_PREFIX;
    key[1..9].copy_from_slice(&lsn.to_be_bytes());
    key[9..].copy_from_slice(&index.to_be_bytes());
    key
}

fn security_mutation(event: &SecurityEvent, index: u32) -> Result<Vec<Mutation>, Error> {
    Ok(vec![Mutation::put(
        security_key(event.observation_lsn, index),
        encode(event)?,
    )])
}

fn policy_rejection(code: ErrorCode) -> bool {
    matches!(
        code,
        ErrorCode::NegativeExistenceUncorroborated
            | ErrorCode::BeliefIdConflict
            | ErrorCode::BeliefNotFound
    )
}

fn encode<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, Error> {
    bincode::serialize(value).map_err(|_| Error::new(ErrorCode::BeliefCorrupt))
}

fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, Error> {
    bincode::deserialize(bytes).map_err(|_| Error::new(ErrorCode::BeliefCorrupt))
}

struct ToolCallHistory<'snapshot, 'environment> {
    snapshot: &'snapshot ReadSnapshot<'environment>,
}

impl EventHistory for ToolCallHistory<'_, '_> {
    fn kind_at(&self, lsn: LSN) -> Option<event::EventKind> {
        self.snapshot
            .get(
                ProjectionId::BeliefStore,
                &marker_key(TOOL_CALL_PREFIX, lsn),
            )
            .ok()
            .flatten()
            .map(|_| event::EventKind::ToolCall)
    }
}

fn verify(
    frame: &Frame,
    kind: event::EventKind,
    snapshot: &ReadSnapshot<'_>,
) -> Result<EventEnvelope, Error> {
    event::verify_event_with_history(
        &frame.sealed_payload,
        kind,
        Boundary::Disk,
        &ToolCallHistory { snapshot },
    )
    .map(|verified| verified.envelope)
    .map_err(|error| error.at_lsn(frame.header.lsn))
}
