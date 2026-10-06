use crate::bus::{
    validate_name, BackendCapabilities, DeadLetter, Delivery, Disposition, Event, Grant, Register,
};
use crate::bus_contract::{BusTopology, RegisterSnapshot, RegisterUpdate, StreamFamily};
use hm_context::{ContextError, Scope};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};

static TOPOLOGY_BINDINGS: LazyLock<Mutex<BTreeMap<String, String>>> =
    LazyLock::new(|| Mutex::new(BTreeMap::new()));
fn bind_topology(topology: &BusTopology) -> Result<(), ContextError> {
    let identity = topology.scope.digest()?;
    let mut bindings = TOPOLOGY_BINDINGS
        .lock()
        .map_err(|_| ContextError::Unavailable("topology registry poisoned".into()))?;
    let mut names: Vec<String> = StreamFamily::ALL
        .iter()
        .map(|f| topology.stream_name(*f))
        .collect();
    names.push(topology.census_bucket.clone());
    if names
        .iter()
        .any(|n| bindings.get(n).is_some_and(|id| id != &identity))
    {
        return Err(ContextError::Conflict);
    }
    let missing = names.iter().filter(|n| !bindings.contains_key(*n)).count();
    if bindings.len() + missing > 10000 {
        return Err(ContextError::Capacity);
    }
    for name in names {
        bindings.insert(name, identity.clone());
    }
    Ok(())
}
fn validate_resource(value: &str) -> Result<(), ContextError> {
    if value.is_empty()
        || value.len() > 400
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(ContextError::Invalid("invalid bus resource".into()));
    }
    Ok(())
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Stream,
    Queue,
    Register,
    Consumer,
    Inbox,
    DeadLetter,
}
impl ResourceKind {
    fn token(self) -> &'static str {
        match self {
            Self::Stream => "stream",
            Self::Queue => "queue",
            Self::Register => "register",
            Self::Consumer => "consumer",
            Self::Inbox => "inbox",
            Self::DeadLetter => "dead_letter",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CensusEntry {
    pub family: ResourceKind,
    pub name: String,
    pub entries: usize,
    pub in_flight: usize,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct MemoryBusLimits {
    pub resources: usize,
    pub events: usize,
    pub queue_depth: usize,
    pub consumers: usize,
    pub grants: usize,
    pub registers: usize,
    pub dead_letters: usize,
}
impl Default for MemoryBusLimits {
    fn default() -> Self {
        Self {
            resources: 128,
            events: 4096,
            queue_depth: 256,
            consumers: 256,
            grants: 256,
            registers: 256,
            dead_letters: 256,
        }
    }
}
impl MemoryBusLimits {
    fn validate(self) -> Result<(), ContextError> {
        if [
            self.resources,
            self.events,
            self.queue_depth,
            self.consumers,
            self.grants,
            self.registers,
            self.dead_letters,
        ]
        .iter()
        .any(|n| *n == 0 || *n > 100_000)
        {
            return Err(ContextError::Invalid("invalid memory bus bounds".into()));
        }
        Ok(())
    }
}
#[derive(Clone)]
pub struct MemoryBus {
    scope: Scope,
    topology: BusTopology,
    namespace: String,
    limits: MemoryBusLimits,
    state: Arc<Mutex<State>>,
}
#[derive(Default)]
struct State {
    sequence: u64,
    grants: BTreeMap<(String, String), Grant>,
    streams: BTreeMap<String, Vec<Event>>,
    subscriptions: BTreeMap<(String, String), Subscription>,
    queues: BTreeMap<String, Queue>,
    registers: BTreeMap<(String, String), Register>,
    dead: Vec<DeadLetter>,
    dedup: BTreeMap<(String, String), Event>,
    workers: BTreeMap<(String, String), String>,
    register_revisions: BTreeMap<String, u64>,
    register_updates: BTreeMap<String, Vec<RegisterUpdate>>,
}
struct Subscription {
    principal: String,
    cursor: u64,
    attempts: u32,
    max_attempts: u32,
    flight: Option<Flight>,
}
#[derive(Clone)]
struct Flight {
    principal: String,
    subscriber: String,
    sequence: u64,
    attempt: u32,
    deadline: i64,
}
struct Queue {
    max_attempts: u32,
    entries: VecDeque<QueueItem>,
}
struct QueueItem {
    event: Event,
    attempts: u32,
    last_subscriber: String,
    flight: Option<Flight>,
}
fn digest(value: &[u8]) -> Result<(), ContextError> {
    if value.len() != 64
        || !value
            .iter()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b))
    {
        return Err(ContextError::Invalid(
            "bus values must be lowercase SHA256 references".into(),
        ));
    }
    Ok(())
}
fn deadline(now: i64, lease: i64) -> Result<i64, ContextError> {
    if now < 0 || lease <= 0 {
        return Err(ContextError::Invalid("invalid lease".into()));
    }
    now.checked_add(lease).ok_or(ContextError::Capacity)
}
fn authorize(
    state: &State,
    principal: &str,
    resource: &str,
    right: fn(&Grant) -> bool,
) -> Result<(), ContextError> {
    validate_name(principal)?;
    validate_resource(resource)?;
    if state
        .grants
        .get(&(principal.into(), resource.into()))
        .is_some_and(right)
    {
        Ok(())
    } else {
        Err(ContextError::ScopeMismatch)
    }
}
fn next_sequence(state: &mut State) -> Result<u64, ContextError> {
    state.sequence = state
        .sequence
        .checked_add(1)
        .ok_or(ContextError::Capacity)?;
    Ok(state.sequence)
}
fn flight_matches(
    flight: &Option<Flight>,
    principal: &str,
    delivery: &Delivery,
    now: i64,
) -> Result<(), ContextError> {
    if now < 0 {
        return Err(ContextError::Invalid("invalid time".into()));
    }
    match flight {
        Some(f)
            if f.principal == principal
                && f.subscriber == delivery.subscriber
                && f.sequence == delivery.event.sequence
                && f.attempt == delivery.attempt
                && f.deadline > now =>
        {
            Ok(())
        }
        _ => Err(ContextError::Stale),
    }
}
impl MemoryBus {
    pub fn new(scope: Scope, limits: MemoryBusLimits) -> Result<Self, ContextError> {
        limits.validate()?;
        let namespace = format!("hm.{}", scope.digest()?);
        let topology = BusTopology::new(scope.clone())?;
        bind_topology(&topology)?;
        Ok(Self {
            scope,
            topology,
            namespace,
            limits,
            state: Arc::new(Mutex::new(State::default())),
        })
    }
    fn lock(&self) -> Result<MutexGuard<'_, State>, ContextError> {
        self.state
            .lock()
            .map_err(|_| ContextError::Unavailable("memory bus owner poisoned".into()))
    }
    pub fn topology(&self) -> &BusTopology {
        &self.topology
    }
    pub fn append_family(
        &self,
        principal: &str,
        family: StreamFamily,
        payload: &[u8],
    ) -> Result<Event, ContextError> {
        self.append(principal, &self.topology.stream_name(family), payload)
    }
    pub fn scope(&self) -> &Scope {
        &self.scope
    }
    pub fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            backend: "process_local".into(),
            durable: false,
            replay: true,
            register_cas: true,
            distributed: false,
        }
    }
    pub fn scoped_name(
        &self,
        family: ResourceKind,
        components: &[&str],
    ) -> Result<String, ContextError> {
        if components.is_empty() || components.len() > 3 {
            return Err(ContextError::Invalid("invalid name components".into()));
        }
        for component in components {
            validate_resource(component)?;
        }
        Ok(format!(
            "{}.{}.{}",
            self.namespace,
            family.token(),
            components.join(".")
        ))
    }
    pub fn grant(&self, grant: Grant) -> Result<(), ContextError> {
        validate_name(&grant.principal)?;
        validate_resource(&grant.stream)?;
        let mut state = self.lock()?;
        let key = (grant.principal.clone(), grant.stream.clone());
        if !state.grants.contains_key(&key) && state.grants.len() >= self.limits.grants {
            return Err(ContextError::Capacity);
        }
        state.grants.insert(key, grant);
        Ok(())
    }
    pub fn revoke(&self, principal: &str, resource: &str) -> Result<(), ContextError> {
        validate_name(principal)?;
        validate_resource(resource)?;
        self.lock()?
            .grants
            .remove(&(principal.into(), resource.into()));
        Ok(())
    }
    pub fn append(
        &self,
        principal: &str,
        stream: &str,
        payload: &[u8],
    ) -> Result<Event, ContextError> {
        self.append_inner(principal, stream, None, payload)
    }
    pub fn append_once(
        &self,
        principal: &str,
        stream: &str,
        id: &str,
        payload: &[u8],
    ) -> Result<Event, ContextError> {
        hm_context::types::validate_id(id)?;
        self.append_inner(principal, stream, Some(id), payload)
    }
    fn append_inner(
        &self,
        principal: &str,
        stream: &str,
        id: Option<&str>,
        payload: &[u8],
    ) -> Result<Event, ContextError> {
        digest(payload)?;
        let mut state = self.lock()?;
        authorize(&state, principal, stream, |g| g.publish)?;
        if state.queues.contains_key(stream) {
            return Err(ContextError::Conflict);
        }
        if let Some(id) = id {
            if let Some(existing) = state.dedup.get(&(stream.into(), id.into())) {
                return if existing.payload == payload {
                    Ok(existing.clone())
                } else {
                    Err(ContextError::Conflict)
                };
            }
        }
        if (!state.streams.contains_key(stream)
            && state.streams.len() + state.queues.len() >= self.limits.resources)
            || state.streams.values().map(Vec::len).sum::<usize>() >= self.limits.events
        {
            return Err(ContextError::Capacity);
        }
        let event = Event {
            sequence: next_sequence(&mut state)?,
            stream: stream.into(),
            payload: payload.into(),
        };
        state
            .streams
            .entry(stream.into())
            .or_default()
            .push(event.clone());
        if let Some(id) = id {
            state
                .dedup
                .insert((stream.into(), id.into()), event.clone());
        }
        Ok(event)
    }
    pub fn replay(
        &self,
        principal: &str,
        stream: &str,
        after: u64,
        limit: u32,
    ) -> Result<Vec<Event>, ContextError> {
        let state = self.lock()?;
        authorize(&state, principal, stream, |g| g.subscribe)?;
        if limit as usize > self.limits.events {
            return Err(ContextError::Capacity);
        }
        Ok(state
            .streams
            .get(stream)
            .into_iter()
            .flatten()
            .filter(|e| e.sequence > after)
            .take(limit as usize)
            .cloned()
            .collect())
    }
    pub fn subscribe(
        &self,
        principal: &str,
        stream: &str,
        subscriber: &str,
        max_attempts: u32,
    ) -> Result<(), ContextError> {
        validate_name(subscriber)?;
        if max_attempts == 0 || max_attempts > 1000 {
            return Err(ContextError::Invalid("invalid attempt bound".into()));
        }
        let mut state = self.lock()?;
        authorize(&state, principal, stream, |g| g.subscribe)?;
        let key = (stream.into(), subscriber.into());
        if let Some(s) = state.subscriptions.get(&key) {
            return if s.principal == principal && s.max_attempts == max_attempts {
                Ok(())
            } else {
                Err(ContextError::Conflict)
            };
        }
        if state.subscriptions.len() + state.workers.len() >= self.limits.consumers {
            return Err(ContextError::Capacity);
        }
        state.subscriptions.insert(
            key,
            Subscription {
                principal: principal.into(),
                cursor: 0,
                attempts: 0,
                max_attempts,
                flight: None,
            },
        );
        Ok(())
    }
    pub fn next(
        &self,
        principal: &str,
        stream: &str,
        subscriber: &str,
        now_ms: i64,
        lease_ms: i64,
    ) -> Result<Option<Delivery>, ContextError> {
        let end = deadline(now_ms, lease_ms)?;
        validate_name(subscriber)?;
        let mut state = self.lock()?;
        authorize(&state, principal, stream, |g| g.subscribe)?;
        let key = (stream.into(), subscriber.into());
        loop {
            let s = state
                .subscriptions
                .get(&key)
                .ok_or_else(|| ContextError::Invalid("unknown subscription".into()))?;
            if s.principal != principal {
                return Err(ContextError::ScopeMismatch);
            }
            if s.flight.as_ref().is_some_and(|f| f.deadline > now_ms) {
                return Ok(None);
            }
            let event = state
                .streams
                .get(stream)
                .into_iter()
                .flatten()
                .find(|e| e.sequence > s.cursor)
                .cloned();
            let Some(event) = event else { return Ok(None) };
            if s.attempts >= s.max_attempts {
                let attempt = s.attempts;
                if state.dead.len() >= self.limits.dead_letters {
                    return Err(ContextError::Capacity);
                }
                state.dead.push(DeadLetter {
                    delivery: Delivery {
                        event: event.clone(),
                        subscriber: subscriber.into(),
                        attempt,
                    },
                    reason: "attempts_exhausted".into(),
                });
                let s = state.subscriptions.get_mut(&key).unwrap();
                s.cursor = event.sequence;
                s.attempts = 0;
                s.flight = None;
                continue;
            }
            let s = state.subscriptions.get_mut(&key).unwrap();
            s.attempts += 1;
            s.flight = Some(Flight {
                principal: principal.into(),
                subscriber: subscriber.into(),
                sequence: event.sequence,
                attempt: s.attempts,
                deadline: end,
            });
            return Ok(Some(Delivery {
                event,
                subscriber: subscriber.into(),
                attempt: s.attempts,
            }));
        }
    }
    pub fn disposition(
        &self,
        principal: &str,
        delivery: &Delivery,
        disposition: Disposition,
        now_ms: i64,
    ) -> Result<(), ContextError> {
        let mut state = self.lock()?;
        authorize(&state, principal, &delivery.event.stream, |g| g.subscribe)?;
        let key = (delivery.event.stream.clone(), delivery.subscriber.clone());
        let s = state.subscriptions.get(&key).ok_or(ContextError::Stale)?;
        flight_matches(&s.flight, principal, delivery, now_ms)?;
        let canonical = state
            .streams
            .get(&delivery.event.stream)
            .into_iter()
            .flatten()
            .find(|e| e.sequence == delivery.event.sequence)
            .ok_or(ContextError::Stale)?;
        if canonical != &delivery.event {
            return Err(ContextError::Conflict);
        }
        if matches!(disposition, Disposition::Term) {
            if state.dead.len() >= self.limits.dead_letters {
                return Err(ContextError::Capacity);
            }
            state.dead.push(DeadLetter {
                delivery: delivery.clone(),
                reason: "terminated".into(),
            });
        }
        let s = state.subscriptions.get_mut(&key).unwrap();
        match disposition {
            Disposition::Ack | Disposition::Term => {
                s.cursor = delivery.event.sequence;
                s.attempts = 0;
                s.flight = None;
            }
            Disposition::Nak => s.flight = None,
            Disposition::Progress { lease_ms } => {
                s.flight.as_mut().unwrap().deadline = deadline(now_ms, lease_ms)?
            }
        }
        Ok(())
    }
    pub fn queue_create(
        &self,
        principal: &str,
        queue: &str,
        max_attempts: u32,
    ) -> Result<(), ContextError> {
        if max_attempts == 0 || max_attempts > 1000 {
            return Err(ContextError::Invalid("invalid attempt bound".into()));
        }
        let mut state = self.lock()?;
        authorize(&state, principal, queue, |g| g.publish)?;
        if state.streams.contains_key(queue) {
            return Err(ContextError::Conflict);
        }
        if let Some(q) = state.queues.get(queue) {
            return if q.max_attempts == max_attempts {
                Ok(())
            } else {
                Err(ContextError::Conflict)
            };
        }
        if state.streams.len() + state.queues.len() >= self.limits.resources {
            return Err(ContextError::Capacity);
        }
        state.queues.insert(
            queue.into(),
            Queue {
                max_attempts,
                entries: VecDeque::new(),
            },
        );
        Ok(())
    }
    pub fn queue_push(
        &self,
        principal: &str,
        queue: &str,
        payload: &[u8],
    ) -> Result<Event, ContextError> {
        digest(payload)?;
        let mut state = self.lock()?;
        authorize(&state, principal, queue, |g| g.publish)?;
        let q = state
            .queues
            .get(queue)
            .ok_or_else(|| ContextError::Invalid("unknown queue".into()))?;
        if q.entries.len() >= self.limits.queue_depth {
            return Err(ContextError::Capacity);
        }
        let event = Event {
            sequence: next_sequence(&mut state)?,
            stream: queue.into(),
            payload: payload.into(),
        };
        state
            .queues
            .get_mut(queue)
            .unwrap()
            .entries
            .push_back(QueueItem {
                event: event.clone(),
                attempts: 0,
                last_subscriber: String::new(),
                flight: None,
            });
        Ok(event)
    }
    pub fn queue_next(
        &self,
        principal: &str,
        queue: &str,
        subscriber: &str,
        now_ms: i64,
        lease_ms: i64,
    ) -> Result<Option<Delivery>, ContextError> {
        validate_name(subscriber)?;
        let end = deadline(now_ms, lease_ms)?;
        let mut state = self.lock()?;
        authorize(&state, principal, queue, |g| g.subscribe)?;
        if !state.queues.contains_key(queue) {
            return Err(ContextError::Invalid("unknown queue".into()));
        }
        let worker_key = (queue.into(), subscriber.into());
        if let Some(owner) = state.workers.get(&worker_key) {
            if owner != principal {
                return Err(ContextError::ScopeMismatch);
            }
        } else {
            if state.workers.len() + state.subscriptions.len() >= self.limits.consumers {
                return Err(ContextError::Capacity);
            }
            state.workers.insert(worker_key, principal.into());
        }
        loop {
            let q = state.queues.get(queue).unwrap();
            let Some(item) = q.entries.front() else {
                return Ok(None);
            };
            if item.flight.as_ref().is_some_and(|f| f.deadline > now_ms) {
                return Ok(None);
            }
            if item.attempts >= q.max_attempts {
                if state.dead.len() >= self.limits.dead_letters {
                    return Err(ContextError::Capacity);
                }
                let dead = DeadLetter {
                    delivery: Delivery {
                        event: item.event.clone(),
                        subscriber: item.last_subscriber.clone(),
                        attempt: item.attempts,
                    },
                    reason: "attempts_exhausted".into(),
                };
                state.dead.push(dead);
                state.queues.get_mut(queue).unwrap().entries.pop_front();
                continue;
            }
            let item = state
                .queues
                .get_mut(queue)
                .unwrap()
                .entries
                .front_mut()
                .unwrap();
            item.attempts += 1;
            item.last_subscriber = subscriber.into();
            item.flight = Some(Flight {
                principal: principal.into(),
                subscriber: subscriber.into(),
                sequence: item.event.sequence,
                attempt: item.attempts,
                deadline: end,
            });
            return Ok(Some(Delivery {
                event: item.event.clone(),
                subscriber: subscriber.into(),
                attempt: item.attempts,
            }));
        }
    }
    pub fn queue_disposition(
        &self,
        principal: &str,
        delivery: &Delivery,
        disposition: Disposition,
        now_ms: i64,
    ) -> Result<(), ContextError> {
        let mut state = self.lock()?;
        authorize(&state, principal, &delivery.event.stream, |g| g.subscribe)?;
        let item = state
            .queues
            .get(&delivery.event.stream)
            .and_then(|q| q.entries.front())
            .ok_or(ContextError::Stale)?;
        flight_matches(&item.flight, principal, delivery, now_ms)?;
        if item.event != delivery.event {
            return Err(ContextError::Conflict);
        }
        if matches!(disposition, Disposition::Term) {
            if state.dead.len() >= self.limits.dead_letters {
                return Err(ContextError::Capacity);
            }
            state.dead.push(DeadLetter {
                delivery: delivery.clone(),
                reason: "terminated".into(),
            });
        }
        let q = state.queues.get_mut(&delivery.event.stream).unwrap();
        match disposition {
            Disposition::Ack | Disposition::Term => {
                q.entries.pop_front();
            }
            Disposition::Nak => q.entries.front_mut().unwrap().flight = None,
            Disposition::Progress { lease_ms } => {
                q.entries
                    .front_mut()
                    .unwrap()
                    .flight
                    .as_mut()
                    .unwrap()
                    .deadline = deadline(now_ms, lease_ms)?
            }
        }
        Ok(())
    }
    pub fn dead_letters(
        &self,
        principal: &str,
        resource: &str,
        subscriber: &str,
    ) -> Result<Vec<DeadLetter>, ContextError> {
        validate_name(subscriber)?;
        let state = self.lock()?;
        authorize(&state, principal, resource, |g| g.subscribe)?;
        let owner = state
            .subscriptions
            .get(&(resource.into(), subscriber.into()))
            .map(|s| s.principal.as_str())
            .or_else(|| {
                state
                    .workers
                    .get(&(resource.into(), subscriber.into()))
                    .map(String::as_str)
            });
        if owner != Some(principal) {
            return Err(ContextError::ScopeMismatch);
        }
        Ok(state
            .dead
            .iter()
            .filter(|d| d.delivery.event.stream == resource && d.delivery.subscriber == subscriber)
            .cloned()
            .collect())
    }
    pub fn register_get(
        &self,
        principal: &str,
        resource: &str,
        name: &str,
    ) -> Result<Option<Register>, ContextError> {
        validate_name(name)?;
        let state = self.lock()?;
        authorize(&state, principal, resource, |g| g.register)?;
        Ok(state
            .registers
            .get(&(resource.into(), name.into()))
            .cloned())
    }
    pub fn register_cas(
        &self,
        principal: &str,
        resource: &str,
        name: &str,
        expected: u64,
        value: &[u8],
    ) -> Result<Register, ContextError> {
        digest(value)?;
        validate_name(name)?;
        let mut state = self.lock()?;
        authorize(&state, principal, resource, |g| g.register)?;
        let key = (resource.into(), name.into());
        let current = state.registers.get(&key);
        if current.map_or(0, |r| r.revision) != expected {
            return Err(ContextError::Conflict);
        }
        if current.is_none() && state.registers.len() >= self.limits.registers {
            return Err(ContextError::Capacity);
        }
        if state.register_updates.values().map(Vec::len).sum::<usize>() >= self.limits.events {
            return Err(ContextError::Capacity);
        }
        let revision = state
            .register_revisions
            .get(resource)
            .copied()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(ContextError::Capacity)?;
        let result = Register {
            revision,
            value: value.into(),
        };
        state.registers.insert(key, result.clone());
        state.register_revisions.insert(resource.into(), revision);
        state
            .register_updates
            .entry(resource.into())
            .or_default()
            .push(RegisterUpdate::Put {
                key: name.into(),
                register: result.clone(),
            });
        Ok(result)
    }
    pub fn register_delete(
        &self,
        principal: &str,
        resource: &str,
        name: &str,
        expected: u64,
    ) -> Result<u64, ContextError> {
        validate_name(name)?;
        let mut state = self.lock()?;
        authorize(&state, principal, resource, |g| g.register)?;
        let key = (resource.into(), name.into());
        if state.registers.get(&key).map(|r| r.revision) != Some(expected) {
            return Err(ContextError::Conflict);
        }
        if state.register_updates.values().map(Vec::len).sum::<usize>() >= self.limits.events {
            return Err(ContextError::Capacity);
        }
        let revision = state
            .register_revisions
            .get(resource)
            .copied()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(ContextError::Capacity)?;
        state.registers.remove(&key);
        state.register_revisions.insert(resource.into(), revision);
        state
            .register_updates
            .entry(resource.into())
            .or_default()
            .push(RegisterUpdate::Delete {
                key: name.into(),
                revision,
            });
        Ok(revision)
    }
    pub fn register_snapshot(
        &self,
        principal: &str,
        resource: &str,
    ) -> Result<RegisterSnapshot, ContextError> {
        let state = self.lock()?;
        authorize(&state, principal, resource, |g| g.register)?;
        Ok(RegisterSnapshot {
            revision: state.register_revisions.get(resource).copied().unwrap_or(0),
            entries: state
                .registers
                .iter()
                .filter(|((r, _), _)| r == resource)
                .map(|((_, key), value)| (key.clone(), value.clone()))
                .collect(),
        })
    }
    pub fn register_watch_after(
        &self,
        principal: &str,
        resource: &str,
        revision: u64,
        limit: usize,
    ) -> Result<Vec<RegisterUpdate>, ContextError> {
        if limit == 0 || limit > self.limits.events {
            return Err(ContextError::Capacity);
        }
        let state = self.lock()?;
        authorize(&state, principal, resource, |g| g.register)?;
        if revision > state.register_revisions.get(resource).copied().unwrap_or(0) {
            return Err(ContextError::Stale);
        }
        Ok(state
            .register_updates
            .get(resource)
            .into_iter()
            .flatten()
            .filter(|u| u.revision() > revision)
            .take(limit)
            .cloned()
            .collect())
    }
    pub fn census(&self, principal: &str, limit: usize) -> Result<Vec<CensusEntry>, ContextError> {
        validate_name(principal)?;
        if limit == 0 || limit > 100_000 {
            return Err(ContextError::Capacity);
        }
        let state = self.lock()?;
        let mut result = Vec::new();
        let visible = |resource: &str| {
            state
                .grants
                .get(&(principal.into(), resource.into()))
                .is_some_and(|g| g.subscribe)
        };
        for (name, events) in &state.streams {
            if visible(name) {
                result.push(CensusEntry {
                    family: ResourceKind::Stream,
                    name: self.scoped_name(ResourceKind::Stream, &[name])?,
                    entries: events.len(),
                    in_flight: state
                        .subscriptions
                        .iter()
                        .filter(|((s, _), v)| s == name && v.flight.is_some())
                        .count(),
                });
            }
        }
        for (name, queue) in &state.queues {
            if visible(name) {
                result.push(CensusEntry {
                    family: ResourceKind::Queue,
                    name: self.scoped_name(ResourceKind::Queue, &[name])?,
                    entries: queue.entries.len(),
                    in_flight: queue.entries.iter().filter(|i| i.flight.is_some()).count(),
                });
            }
        }
        for ((resource, name), _) in &state.registers {
            if state
                .grants
                .get(&(principal.into(), resource.clone()))
                .is_some_and(|g| g.register)
            {
                result.push(CensusEntry {
                    family: ResourceKind::Register,
                    name: self.scoped_name(ResourceKind::Register, &[resource, name])?,
                    entries: 1,
                    in_flight: 0,
                });
            }
        }
        for ((resource, subscriber), s) in &state.subscriptions {
            if s.principal == principal && visible(resource) {
                for family in [ResourceKind::Consumer, ResourceKind::Inbox] {
                    result.push(CensusEntry {
                        family,
                        name: self.scoped_name(family, &["stream", resource, subscriber])?,
                        entries: 1,
                        in_flight: usize::from(s.flight.is_some()),
                    });
                }
            }
        }
        for ((resource, subscriber), owner) in &state.workers {
            if owner == principal && visible(resource) {
                for family in [ResourceKind::Consumer, ResourceKind::Inbox] {
                    result.push(CensusEntry {
                        family,
                        name: self.scoped_name(family, &["queue", resource, subscriber])?,
                        entries: 1,
                        in_flight: state
                            .queues
                            .get(resource)
                            .into_iter()
                            .flat_map(|q| &q.entries)
                            .filter(|i| {
                                i.flight
                                    .as_ref()
                                    .is_some_and(|f| f.subscriber == *subscriber)
                            })
                            .count(),
                    });
                }
            }
        }
        let mut dead_counts: BTreeMap<(String, String), usize> = BTreeMap::new();
        for d in &state.dead {
            let resource = &d.delivery.event.stream;
            let subscriber = &d.delivery.subscriber;
            if visible(resource)
                && (state
                    .subscriptions
                    .get(&(resource.clone(), subscriber.clone()))
                    .is_some_and(|s| s.principal == principal)
                    || state
                        .workers
                        .get(&(resource.clone(), subscriber.clone()))
                        .is_some_and(|p| p == principal))
            {
                *dead_counts
                    .entry((resource.clone(), subscriber.clone()))
                    .or_default() += 1;
            }
        }
        for ((resource, subscriber), entries) in dead_counts {
            result.push(CensusEntry {
                family: ResourceKind::DeadLetter,
                name: self.scoped_name(ResourceKind::DeadLetter, &[&resource, &subscriber])?,
                entries,
                in_flight: 0,
            });
        }
        if result.len() > limit {
            return Err(ContextError::Capacity);
        }
        result.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(result)
    }
}
