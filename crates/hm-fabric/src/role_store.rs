use crate::{
    roles::{
        RoleVersion, RunTerminal, TransformDeclaration, TransformInput, TransformOutput,
        TransformRegistry,
    },
    storage::{FencedStore, Migration, StorageError},
};
use hm_context::types::{digest_bytes, validate_id, Authority, ContextBlock, ContextError, Scope};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "durable_role_state",
    sql: "CREATE TABLE role_state(id INTEGER PRIMARY KEY CHECK(id=1), data BLOB NOT NULL);",
}];
#[derive(Debug, thiserror::Error)]
pub enum RoleError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Context(#[from] ContextError),
    #[error("work requires approval")]
    Unapproved,
    #[error("effect is uncertain; redispatch refused")]
    Uncertain,
    #[error("late result requires acknowledgement")]
    LateAcknowledgement,
    #[error("role withdrawn")]
    Withdrawn,
    #[error("role unsupported: {0}")]
    Unsupported(String),
    #[error("deadline expired")]
    Deadline,
}
impl From<serde_json::Error> for RoleError {
    fn from(e: serde_json::Error) -> Self {
        ContextError::Json(e).into()
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RoleKind {
    Tool,
    Runner,
    Compaction,
    Transform,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RoleGrants {
    pub operations: BTreeSet<String>,
    pub authorities: Vec<Authority>,
    pub hooks: BTreeSet<String>,
    pub max_input_tokens: u64,
    pub max_output_tokens: u64,
    pub may_reorder: bool,
}
impl RoleGrants {
    pub fn tightens(&self, old: &Self) -> bool {
        self.operations.is_subset(&old.operations)
            && self.authorities.iter().all(|a| old.authorities.contains(a))
            && self.hooks.is_subset(&old.hooks)
            && self.max_input_tokens <= old.max_input_tokens
            && self.max_output_tokens <= old.max_output_tokens
            && (!self.may_reorder || old.may_reorder)
    }
    fn validate(&self) -> Result<(), RoleError> {
        if self.max_input_tokens == 0
            || self.max_output_tokens == 0
            || self.operations.is_empty()
            || self.authorities.is_empty()
        {
            return Err(ContextError::Invalid("empty role grants".into()).into());
        }
        for id in self.operations.iter().chain(self.hooks.iter()) {
            validate_id(id)?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RoleDescriptor {
    pub id: String,
    pub kind: RoleKind,
    pub pin: RoleVersion,
    pub semantic_version: String,
    pub capabilities: BTreeMap<String, u32>,
    pub grants: RoleGrants,
    pub transform: Option<TransformDeclaration>,
}
impl RoleDescriptor {
    fn validate(&self) -> Result<(), RoleError> {
        validate_id(&self.id)?;
        self.pin.validate()?;
        semver(&self.semantic_version)?;
        self.grants.validate()?;
        if self.capabilities.is_empty() || self.capabilities.values().any(|v| *v == 0) {
            return Err(ContextError::Invalid("invalid role capabilities".into()).into());
        }
        for name in self.capabilities.keys() {
            validate_id(name)?;
        }
        if self.kind == RoleKind::Transform {
            let d = self.transform.as_ref().ok_or(ContextError::Invalid(
                "transform declaration required".into(),
            ))?;
            let mut registry = TransformRegistry::default();
            registry.register(d.clone())?;
            if d.id != self.id
                || d.pin != self.pin
                || d.max_input_tokens > self.grants.max_input_tokens
                || d.max_output_tokens > self.grants.max_output_tokens
                || !d.hooks.is_subset(&self.grants.hooks)
                || d.authorities
                    .iter()
                    .any(|a| !self.grants.authorities.contains(a))
                || (d.may_reorder && !self.grants.may_reorder)
            {
                return Err(ContextError::Conflict.into());
            }
        } else if self.transform.is_some() {
            return Err(
                ContextError::Invalid("transform declaration on another role".into()).into(),
            );
        }
        Ok(())
    }
}
fn semver(value: &str) -> Result<(u64, u64, u64), RoleError> {
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.len() != 3
        || parts.iter().any(|p| {
            p.is_empty()
                || p.len() > 1 && p.starts_with('0')
                || !p.bytes().all(|b| b.is_ascii_digit())
        })
    {
        return Err(
            ContextError::Invalid("semantic version requires major.minor.patch".into()).into(),
        );
    }
    let parse = |p: &str| {
        p.parse::<u64>().map_err(|_| {
            RoleError::Context(ContextError::Invalid("semantic version overflow".into()))
        })
    };
    Ok((parse(parts[0])?, parse(parts[1])?, parse(parts[2])?))
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CatalogReceipt {
    pub descriptor: RoleDescriptor,
    pub revision: u64,
    pub withdrawal: Option<String>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceFence {
    pub epoch: u64,
    pub generation: u64,
    pub digest: String,
    pub start: u64,
    pub end: u64,
}
impl SourceFence {
    fn validate(&self) -> Result<(), RoleError> {
        if self.epoch == 0
            || self.generation == 0
            || self.start >= self.end
            || self.digest.len() != 64
            || !self
                .digest
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Err(ContextError::Invalid("invalid source fence".into()).into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RoleWork {
    pub id: String,
    pub role_id: String,
    pub operation: String,
    pub capability_version: u32,
    pub pin: RoleVersion,
    pub semantic_version: String,
    pub source: SourceFence,
    pub payload: Vec<u8>,
    pub blocks: Vec<ContextBlock>,
    pub hook: String,
    pub deadline_ns: i64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkState {
    Held,
    Approved,
    Dispatched,
    Uncertain,
    AwaitingAcknowledgement,
    Terminal,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DispatchTicket {
    pub id: String,
    pub attempt: u64,
    pub writer_epoch: u64,
    pub catalog_revision: u64,
    pub work_digest: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RoleOutput {
    pub terminal: RunTerminal,
    pub payload: Vec<u8>,
    pub blocks: Vec<ContextBlock>,
    pub grants: RoleGrants,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkRecord {
    pub work: RoleWork,
    pub descriptor: RoleDescriptor,
    pub state: WorkState,
    pub revision: u64,
    pub approval_revision: Option<u64>,
    pub ticket: Option<DispatchTicket>,
    pub output: Option<RoleOutput>,
    pub deliveries: BTreeMap<String, String>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RoleEvent {
    pub sequence: u64,
    pub writer_epoch: u64,
    pub id: String,
    pub state: Option<WorkState>,
    pub detail: String,
}
#[derive(Clone, Serialize, Deserialize)]
struct State {
    scope: Scope,
    catalog: BTreeMap<String, CatalogReceipt>,
    work: BTreeMap<String, WorkRecord>,
    events: Vec<RoleEvent>,
}
impl State {
    fn event(
        &mut self,
        epoch: u64,
        id: &str,
        state: Option<WorkState>,
        detail: &str,
    ) -> Result<(), RoleError> {
        let sequence = u64::try_from(self.events.len())
            .map_err(|_| ContextError::Capacity)?
            .checked_add(1)
            .ok_or(ContextError::Capacity)?;
        self.events.push(RoleEvent {
            sequence,
            writer_epoch: epoch,
            id: id.into(),
            state,
            detail: detail.into(),
        });
        Ok(())
    }
}
pub struct DurableRoleRegistry {
    store: FencedStore,
    scope: Scope,
}
impl DurableRoleRegistry {
    pub fn open(path: impl AsRef<Path>, scope: Scope) -> Result<Self, RoleError> {
        scope.validate()?;
        let mut store = FencedStore::open(path, MIGRATIONS)?;
        let encoded = serde_json::to_vec(&State {
            scope: scope.clone(),
            catalog: BTreeMap::new(),
            work: BTreeMap::new(),
            events: vec![],
        })?;
        store.transaction(store.epoch(), |tx| {
            tx.execute("INSERT OR IGNORE INTO role_state VALUES(1,?1)", [encoded])?;
            Ok(())
        })?;
        let mut registry = Self { store, scope };
        registry.mutate(|state, epoch| {
            let ids = state
                .work
                .iter()
                .filter(|(_, w)| w.state == WorkState::Dispatched)
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>();
            for id in ids {
                let work = state.work.get_mut(&id).ok_or(ContextError::Stale)?;
                work.state = WorkState::Uncertain;
                work.revision += 1;
                state.event(
                    epoch,
                    &id,
                    Some(WorkState::Uncertain),
                    "recovered unobserved dispatch",
                )?;
            }
            Ok(())
        })?;
        Ok(registry)
    }
    pub fn epoch(&self) -> u64 {
        self.store.epoch()
    }
    pub fn scope(&self) -> &Scope {
        &self.scope
    }
    fn snapshot(&self) -> Result<State, RoleError> {
        let bytes: Vec<u8> = self.store.read(|db| {
            Ok(db.query_row("SELECT data FROM role_state WHERE id=1", [], |r| r.get(0))?)
        })?;
        let state: State = serde_json::from_slice(&bytes)?;
        if state.scope != self.scope {
            return Err(ContextError::ScopeMismatch.into());
        }
        Ok(state)
    }
    fn mutate<T>(
        &mut self,
        f: impl FnOnce(&mut State, u64) -> Result<T, RoleError>,
    ) -> Result<T, RoleError> {
        let epoch = self.epoch();
        let mut state = self.snapshot()?;
        let output = f(&mut state, epoch)?;
        let bytes = serde_json::to_vec(&state)?;
        self.store.transaction(epoch, |tx| {
            tx.execute("UPDATE role_state SET data=?1 WHERE id=1", [bytes])?;
            Ok(())
        })?;
        Ok(output)
    }
    pub fn catalog(&self, id: &str) -> Result<Option<CatalogReceipt>, RoleError> {
        Ok(self.snapshot()?.catalog.get(id).cloned())
    }
    pub fn get(&self, id: &str) -> Result<Option<WorkRecord>, RoleError> {
        Ok(self.snapshot()?.work.get(id).cloned())
    }
    pub fn receipts(&self, after: u64) -> Result<Vec<RoleEvent>, RoleError> {
        Ok(self
            .snapshot()?
            .events
            .into_iter()
            .filter(|e| e.sequence > after)
            .collect())
    }
    pub fn register(&mut self, descriptor: RoleDescriptor) -> Result<CatalogReceipt, RoleError> {
        descriptor.validate()?;
        self.mutate(|state, epoch| {
            let revision = if let Some(old) = state.catalog.get(&descriptor.id) {
                if old.descriptor == descriptor {
                    return Ok(old.clone());
                }
                if old.withdrawal.is_some() {
                    return Err(RoleError::Withdrawn);
                }
                if old.descriptor.kind != descriptor.kind
                    || !descriptor.grants.tightens(&old.descriptor.grants)
                    || old.descriptor.capabilities.iter().any(|(name, v)| {
                        descriptor
                            .capabilities
                            .get(name)
                            .is_some_and(|new| new != v)
                    })
                    || descriptor
                        .capabilities
                        .keys()
                        .any(|name| !old.descriptor.capabilities.contains_key(name))
                {
                    return Err(ContextError::Conflict.into());
                }
                if let (Some(new), Some(prior)) = (&descriptor.transform, &old.descriptor.transform)
                {
                    if new.max_input_tokens > prior.max_input_tokens
                        || new.max_output_tokens > prior.max_output_tokens
                        || !new.hooks.is_subset(&prior.hooks)
                        || new
                            .authorities
                            .iter()
                            .any(|a| !prior.authorities.contains(a))
                        || (new.may_reorder && !prior.may_reorder)
                    {
                        return Err(ContextError::Conflict.into());
                    }
                }
                if descriptor.pin != old.descriptor.pin
                    && semver(&descriptor.semantic_version)?
                        <= semver(&old.descriptor.semantic_version)?
                {
                    return Err(ContextError::Conflict.into());
                }
                if descriptor.pin == old.descriptor.pin
                    && descriptor.semantic_version != old.descriptor.semantic_version
                {
                    return Err(ContextError::Conflict.into());
                }
                old.revision.checked_add(1).ok_or(ContextError::Capacity)?
            } else {
                1
            };
            let receipt = CatalogReceipt {
                descriptor: descriptor.clone(),
                revision,
                withdrawal: None,
            };
            state.catalog.insert(descriptor.id.clone(), receipt.clone());
            let held = state
                .work
                .iter()
                .filter(|(_, w)| w.work.role_id == descriptor.id && w.state == WorkState::Approved)
                .map(|(id, _)| id.clone())
                .collect::<Vec<_>>();
            for id in held {
                let work = state.work.get_mut(&id).ok_or(ContextError::Stale)?;
                work.state = WorkState::Held;
                work.approval_revision = None;
                work.revision += 1;
                state.event(
                    epoch,
                    &id,
                    Some(WorkState::Held),
                    "catalog changed; approval invalidated",
                )?;
            }
            state.event(epoch, &descriptor.id, None, "catalog registered")?;
            Ok(receipt)
        })
    }
    pub fn withdraw(&mut self, id: &str, reason: &str) -> Result<CatalogReceipt, RoleError> {
        if reason.trim().is_empty() {
            return Err(ContextError::Invalid("withdrawal reason required".into()).into());
        }
        self.mutate(|state, epoch| {
            let receipt = state.catalog.get_mut(id).ok_or(ContextError::Stale)?;
            if receipt.withdrawal.is_some() {
                return Ok(receipt.clone());
            }
            receipt.withdrawal = Some(reason.into());
            receipt.revision = receipt
                .revision
                .checked_add(1)
                .ok_or(ContextError::Capacity)?;
            let receipt = receipt.clone();
            for work in state
                .work
                .values_mut()
                .filter(|w| w.work.role_id == id && w.state == WorkState::Approved)
            {
                work.state = WorkState::Held;
                work.approval_revision = None;
                work.revision += 1;
            }
            state.event(epoch, id, None, "catalog withdrawn")?;
            Ok(receipt)
        })
    }
    pub fn hold(&mut self, work: RoleWork) -> Result<WorkRecord, RoleError> {
        validate_id(&work.id)?;
        validate_id(&work.role_id)?;
        work.pin.validate()?;
        semver(&work.semantic_version)?;
        work.source.validate()?;
        if work.payload.len() > 16 * 1024 * 1024 {
            return Err(ContextError::Capacity.into());
        }
        self.mutate(|state, epoch| {
            if let Some(old) = state.work.get(&work.id) {
                return if old.work == work {
                    Ok(old.clone())
                } else {
                    Err(ContextError::Conflict.into())
                };
            }
            let catalog = state
                .catalog
                .get(&work.role_id)
                .ok_or(ContextError::Stale)?;
            if catalog.withdrawal.is_some() {
                return Err(RoleError::Withdrawn);
            }
            validate_work(&work, &catalog.descriptor)?;
            if catalog.descriptor.kind == RoleKind::Compaction
                && state.work.values().any(|old| {
                    old.descriptor.kind == RoleKind::Compaction
                        && old.state != WorkState::Terminal
                        && old.work.source.epoch == work.source.epoch
                        && old.work.source.generation == work.source.generation
                        && old.work.source.digest == work.source.digest
                        && old.work.source.start < work.source.end
                        && work.source.start < old.work.source.end
                })
            {
                return Err(ContextError::Conflict.into());
            }
            let record = WorkRecord {
                work: work.clone(),
                descriptor: catalog.descriptor.clone(),
                state: WorkState::Held,
                revision: 1,
                approval_revision: None,
                ticket: None,
                output: None,
                deliveries: BTreeMap::new(),
            };
            state.work.insert(work.id.clone(), record.clone());
            state.event(epoch, &work.id, Some(WorkState::Held), "work held")?;
            Ok(record)
        })
    }
    pub fn approve(&mut self, id: &str, expected_revision: u64) -> Result<WorkRecord, RoleError> {
        self.mutate(|state, epoch| {
            let work = state.work.get_mut(id).ok_or(ContextError::Stale)?;
            if work.revision != expected_revision {
                return Err(ContextError::Stale.into());
            }
            if work.state != WorkState::Held {
                return Err(ContextError::Conflict.into());
            }
            let catalog = state
                .catalog
                .get(&work.work.role_id)
                .ok_or(ContextError::Stale)?;
            if catalog.withdrawal.is_some() {
                return Err(RoleError::Withdrawn);
            }
            validate_work(&work.work, &catalog.descriptor)?;
            work.descriptor = catalog.descriptor.clone();
            work.approval_revision = Some(catalog.revision);
            work.state = WorkState::Approved;
            work.revision += 1;
            let record = work.clone();
            state.event(epoch, id, Some(WorkState::Approved), "work approved")?;
            Ok(record)
        })
    }
    pub fn begin(
        &mut self,
        id: &str,
        expected_epoch: u64,
        current_source: &SourceFence,
        now_ns: i64,
    ) -> Result<DispatchTicket, RoleError> {
        if expected_epoch != self.epoch() {
            return Err(StorageError::StaleEpoch.into());
        }
        self.mutate(|state, epoch| {
            let work = state.work.get_mut(id).ok_or(ContextError::Stale)?;
            if matches!(work.state, WorkState::Dispatched | WorkState::Uncertain) {
                return Err(RoleError::Uncertain);
            }
            if work.state != WorkState::Approved {
                return Err(RoleError::Unapproved);
            }
            if &work.work.source != current_source {
                return Err(ContextError::Stale.into());
            }
            if now_ns >= work.work.deadline_ns {
                return Err(RoleError::Deadline);
            }
            let catalog = state
                .catalog
                .get(&work.work.role_id)
                .ok_or(ContextError::Stale)?;
            if catalog.withdrawal.is_some() {
                return Err(RoleError::Withdrawn);
            }
            if work.approval_revision != Some(catalog.revision) {
                return Err(RoleError::Unapproved);
            }
            validate_work(&work.work, &catalog.descriptor)?;
            let ticket = DispatchTicket {
                id: id.into(),
                attempt: work.ticket.as_ref().map_or(1, |t| t.attempt + 1),
                writer_epoch: epoch,
                catalog_revision: catalog.revision,
                work_digest: digest_bytes(&serde_json::to_vec(&work.work)?),
            };
            work.ticket = Some(ticket.clone());
            work.state = WorkState::Dispatched;
            work.revision += 1;
            state.event(epoch, id, Some(WorkState::Dispatched), "dispatch fenced")?;
            Ok(ticket)
        })
    }
    pub fn mark_uncertain(&mut self, ticket: &DispatchTicket) -> Result<WorkRecord, RoleError> {
        self.mutate(|state, epoch| {
            let work = state.work.get_mut(&ticket.id).ok_or(ContextError::Stale)?;
            if work.ticket.as_ref() != Some(ticket) {
                return Err(ContextError::Stale.into());
            }
            if work.state == WorkState::Uncertain {
                return Ok(work.clone());
            }
            if work.state != WorkState::Dispatched {
                return Err(ContextError::Conflict.into());
            }
            work.state = WorkState::Uncertain;
            work.revision += 1;
            let record = work.clone();
            state.event(
                epoch,
                &ticket.id,
                Some(WorkState::Uncertain),
                "dispatch observation unavailable",
            )?;
            Ok(record)
        })
    }
    pub fn complete(
        &mut self,
        ticket: &DispatchTicket,
        current_source: &SourceFence,
        output: RoleOutput,
        now_ns: i64,
    ) -> Result<WorkRecord, RoleError> {
        self.mutate(|state, epoch| {
            let work = state.work.get_mut(&ticket.id).ok_or(ContextError::Stale)?;
            if work.ticket.as_ref() != Some(ticket) {
                return Err(ContextError::Stale.into());
            }
            validate_output(work, &output, &state.scope)?;
            if let Some(old) = &work.output {
                return if old == &output {
                    Ok(work.clone())
                } else {
                    Err(ContextError::Conflict.into())
                };
            }
            if !matches!(work.state, WorkState::Dispatched | WorkState::Uncertain) {
                return Err(ContextError::Conflict.into());
            }
            let catalog = state
                .catalog
                .get(&work.work.role_id)
                .ok_or(ContextError::Stale)?;
            let late = work.state == WorkState::Uncertain
                || ticket.writer_epoch != epoch
                || catalog.withdrawal.is_some()
                || catalog.revision != ticket.catalog_revision
                || current_source != &work.work.source
                || now_ns >= work.work.deadline_ns;
            work.output = Some(output);
            work.state = if late {
                WorkState::AwaitingAcknowledgement
            } else {
                WorkState::Terminal
            };
            work.revision += 1;
            let record = work.clone();
            state.event(
                epoch,
                &ticket.id,
                Some(record.state),
                if late {
                    "late result held for acknowledgement"
                } else {
                    "terminal outcome accepted"
                },
            )?;
            Ok(record)
        })
    }
    pub fn acknowledge_late(
        &mut self,
        id: &str,
        result_digest: &str,
        expected_epoch: u64,
        current_source: &SourceFence,
    ) -> Result<WorkRecord, RoleError> {
        if expected_epoch != self.epoch() {
            return Err(StorageError::StaleEpoch.into());
        }
        self.mutate(|state, epoch| {
            let work = state.work.get_mut(id).ok_or(ContextError::Stale)?;
            if work.state != WorkState::AwaitingAcknowledgement {
                return Err(RoleError::LateAcknowledgement);
            }
            let output = work.output.as_ref().ok_or(ContextError::Stale)?;
            if digest_bytes(&serde_json::to_vec(output)?) != result_digest
                || &work.work.source != current_source
            {
                return Err(ContextError::Stale.into());
            }
            let catalog = state
                .catalog
                .get(&work.work.role_id)
                .ok_or(ContextError::Stale)?;
            if !output.grants.tightens(&catalog.descriptor.grants) {
                return Err(ContextError::Conflict.into());
            }
            work.state = WorkState::Terminal;
            work.revision += 1;
            let record = work.clone();
            state.event(
                epoch,
                id,
                Some(WorkState::Terminal),
                "late result explicitly acknowledged",
            )?;
            Ok(record)
        })
    }
    pub fn deliver(
        &mut self,
        id: &str,
        delivery_id: &str,
        payload: &[u8],
    ) -> Result<bool, RoleError> {
        validate_id(delivery_id)?;
        self.mutate(|state, epoch| {
            let work = state.work.get_mut(id).ok_or(ContextError::Stale)?;
            if work.state != WorkState::Terminal {
                return Err(RoleError::LateAcknowledgement);
            }
            let output = work.output.as_ref().ok_or(ContextError::Stale)?;
            if output.payload != payload {
                return Err(ContextError::Conflict.into());
            }
            let digest = digest_bytes(payload);
            if let Some(old) = work.deliveries.get(delivery_id) {
                return if old == &digest {
                    Ok(false)
                } else {
                    Err(ContextError::Conflict.into())
                };
            }
            work.deliveries.insert(delivery_id.into(), digest);
            state.event(
                epoch,
                id,
                Some(WorkState::Terminal),
                "terminal delivery recorded",
            )?;
            Ok(true)
        })
    }
}
fn validate_work(work: &RoleWork, descriptor: &RoleDescriptor) -> Result<(), RoleError> {
    if !descriptor.grants.operations.contains(&work.operation)
        || descriptor.capabilities.get(&work.operation) != Some(&work.capability_version)
    {
        return Err(ContextError::Conflict.into());
    }
    if work.pin != descriptor.pin || work.semantic_version != descriptor.semantic_version {
        return Err(ContextError::Conflict.into());
    }
    let tokens = work.blocks.iter().try_fold(0u64, |n, b| {
        n.checked_add(b.tokens).ok_or(ContextError::Capacity)
    })?;
    if tokens > descriptor.grants.max_input_tokens
        || work
            .blocks
            .iter()
            .any(|b| !descriptor.grants.authorities.contains(&b.authority))
    {
        return Err(ContextError::Capacity.into());
    }
    if !descriptor.grants.hooks.contains(&work.hook) {
        return Err(ContextError::Conflict.into());
    }
    Ok(())
}
fn validate_output(
    record: &WorkRecord,
    output: &RoleOutput,
    scope: &Scope,
) -> Result<(), RoleError> {
    if !output.grants.tightens(&record.descriptor.grants) || output.payload.len() > 16 * 1024 * 1024
    {
        return Err(ContextError::Conflict.into());
    }
    match &output.terminal {
        RunTerminal::Completed { result_digest }
            if result_digest != &digest_bytes(&output.payload) =>
        {
            return Err(ContextError::Conflict.into())
        }
        RunTerminal::Failed { reason } if reason.trim().is_empty() => {
            return Err(ContextError::Invalid("failure reason required".into()).into())
        }
        _ => {}
    }
    let tokens = output.blocks.iter().try_fold(0u64, |n, b| {
        n.checked_add(b.tokens).ok_or(ContextError::Capacity)
    })?;
    if tokens > output.grants.max_output_tokens
        || output
            .blocks
            .iter()
            .any(|b| !output.grants.authorities.contains(&b.authority))
    {
        return Err(ContextError::Capacity.into());
    }
    if record.descriptor.kind == RoleKind::Transform {
        let mut registry = TransformRegistry::default();
        registry.register(
            record
                .descriptor
                .transform
                .clone()
                .ok_or(ContextError::Stale)?,
        )?;
        registry.validate_output(
            &record.descriptor.id,
            &record.descriptor.pin,
            &TransformInput {
                scope: scope.clone(),
                hook: record.work.hook.clone(),
                blocks: record.work.blocks.clone(),
            },
            &TransformOutput {
                blocks: output.blocks.clone(),
            },
        )?;
    }
    Ok(())
}
