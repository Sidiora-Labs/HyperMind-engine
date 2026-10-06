use crate::types::{validate_id, ContextError, Scope};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoteKind {
    Anchor,
    Note,
    Primer,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Predicate {
    True,
    Exists(String),
    Equals { key: String, value: String },
    Not(Box<Predicate>),
    All(Vec<Predicate>),
    Any(Vec<Predicate>),
}
impl Predicate {
    pub fn evaluate(&self, facts: &BTreeMap<String, String>) -> Result<bool, ContextError> {
        fn visit(
            p: &Predicate,
            facts: &BTreeMap<String, String>,
            depth: usize,
            left: &mut usize,
        ) -> Result<bool, ContextError> {
            if depth > 16 || *left == 0 {
                return Err(ContextError::Capacity);
            }
            *left -= 1;
            Ok(match p {
                Predicate::True => true,
                Predicate::Exists(k) => {
                    validate_id(k)?;
                    facts.contains_key(k)
                }
                Predicate::Equals { key, value } => {
                    validate_id(key)?;
                    if value.len() > 4096 {
                        return Err(ContextError::Capacity);
                    }
                    facts.get(key) == Some(value)
                }
                Predicate::Not(p) => !visit(p, facts, depth + 1, left)?,
                Predicate::All(ps) => {
                    let mut result = true;
                    for p in ps {
                        result &= visit(p, facts, depth + 1, left)?;
                    }
                    result
                }
                Predicate::Any(ps) => {
                    let mut result = false;
                    for p in ps {
                        result |= visit(p, facts, depth + 1, left)?;
                    }
                    result
                }
            })
        }
        visit(self, facts, 0, &mut 128)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub kind: NoteKind,
    pub revision: u64,
    pub text: String,
    pub parents: BTreeSet<String>,
    pub contradictions: BTreeSet<String>,
    #[serde(default, with = "crate::types::optional_timestamp_wire")]
    pub expires_at_ns: Option<i64>,
    pub predicate: Predicate,
    pub tombstoned: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grant {
    pub principal: String,
    pub note_id: String,
    pub read: bool,
    pub write: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttributeProposal {
    pub id: String,
    pub key: String,
    pub value: String,
    pub base_revision: u64,
    pub proposer: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotesCommand {
    Create(Note),
    Revise {
        note: Note,
        expected_revision: u64,
    },
    Tombstone {
        id: String,
        expected_revision: u64,
    },
    SetGrant(Grant),
    SetAttributeGrant {
        principal: String,
        key: String,
        write: bool,
    },
    RevokeGrant {
        principal: String,
        note_id: String,
    },
    ProposeAttribute(AttributeProposal),
    AcceptAttribute {
        proposal_id: String,
    },
    RejectAttribute {
        proposal_id: String,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotesEvent {
    pub scope: Scope,
    pub sequence: u64,
    pub principal: String,
    pub command: NotesCommand,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotesProjection {
    scope: Scope,
    sequence: u64,
    notes: BTreeMap<String, Note>,
    grants: BTreeMap<(String, String), Grant>,
    attribute_grants: BTreeSet<(String, String)>,
    proposals: BTreeMap<String, AttributeProposal>,
    attributes: BTreeMap<String, (u64, String)>,
    events: BTreeMap<u64, NotesEvent>,
}
impl NotesProjection {
    pub fn new(scope: Scope) -> Result<Self, ContextError> {
        scope.validate()?;
        Ok(Self {
            scope,
            sequence: 0,
            notes: BTreeMap::new(),
            grants: BTreeMap::new(),
            attribute_grants: BTreeSet::new(),
            proposals: BTreeMap::new(),
            attributes: BTreeMap::new(),
            events: BTreeMap::new(),
        })
    }
    pub fn restore(
        scope: Scope,
        events: impl IntoIterator<Item = NotesEvent>,
    ) -> Result<Self, ContextError> {
        let mut state = Self::new(scope)?;
        for event in events {
            state.replay(event)?;
        }
        Ok(state)
    }
    pub fn plan(&self, principal: &str, command: NotesCommand) -> Result<NotesEvent, ContextError> {
        let event = NotesEvent {
            scope: self.scope.clone(),
            sequence: self.sequence.checked_add(1).ok_or(ContextError::Capacity)?,
            principal: principal.into(),
            command,
        };
        let mut candidate = self.clone();
        candidate.replay(event.clone())?;
        Ok(event)
    }
    pub fn replay(&mut self, event: NotesEvent) -> Result<(), ContextError> {
        if event.scope != self.scope {
            return Err(ContextError::ScopeMismatch);
        }
        if let Some(previous) = self.events.get(&event.sequence) {
            return if previous == &event {
                Ok(())
            } else {
                Err(ContextError::Conflict)
            };
        }
        if event.sequence != self.sequence.checked_add(1).ok_or(ContextError::Capacity)? {
            return Err(ContextError::Stale);
        }
        validate_id(&event.principal)?;
        let mut next = self.clone();
        next.apply(&event.principal, &event.command)?;
        next.sequence = event.sequence;
        next.events.insert(event.sequence, event);
        *self = next;
        Ok(())
    }
    fn owner(&self, p: &str) -> Result<(), ContextError> {
        if p == self.scope.owner_id {
            Ok(())
        } else {
            Err(ContextError::ScopeMismatch)
        }
    }
    fn access(&self, p: &str, id: &str, write: bool) -> Result<(), ContextError> {
        validate_id(p)?;
        validate_id(id)?;
        if p == self.scope.owner_id
            || self.grants.get(&(p.into(), id.into())).is_some_and(|g| {
                if write {
                    g.write
                } else {
                    g.read
                }
            })
        {
            Ok(())
        } else {
            Err(ContextError::ScopeMismatch)
        }
    }
    fn validate_note(&self, n: &Note) -> Result<(), ContextError> {
        validate_id(&n.id)?;
        if n.text.is_empty()
            || n.text.len() > 262144
            || n.parents.len() > 128
            || n.contradictions.len() > 128
            || n.tombstoned
        {
            return Err(ContextError::Invalid("invalid note".into()));
        }
        n.predicate.evaluate(&BTreeMap::new())?;
        for id in n.parents.iter().chain(n.contradictions.iter()) {
            validate_id(id)?;
            if id == &n.id || !self.notes.contains_key(id) {
                return Err(ContextError::Invalid("invalid relation".into()));
            }
        }
        let mut stack: Vec<_> = n.parents.iter().cloned().collect();
        let mut seen = BTreeSet::new();
        while let Some(id) = stack.pop() {
            if id == n.id {
                return Err(ContextError::Conflict);
            }
            if seen.insert(id.clone()) {
                if let Some(parent) = self.notes.get(&id) {
                    stack.extend(parent.parents.iter().cloned());
                }
            }
        }
        Ok(())
    }
    fn apply(&mut self, p: &str, command: &NotesCommand) -> Result<(), ContextError> {
        match command {
            NotesCommand::Create(n) => {
                self.owner(p)?;
                self.validate_note(n)?;
                if n.revision != 1 || self.notes.contains_key(&n.id) {
                    return Err(ContextError::Conflict);
                }
                self.notes.insert(n.id.clone(), n.clone());
            }
            NotesCommand::Revise {
                note: n,
                expected_revision,
            } => {
                self.access(p, &n.id, true)?;
                let old = self
                    .notes
                    .get(&n.id)
                    .ok_or(ContextError::Unavailable("note".into()))?;
                if old.revision != *expected_revision {
                    return Err(ContextError::Stale);
                }
                if old.kind == NoteKind::Anchor || old.tombstoned || old.kind != n.kind {
                    return Err(ContextError::Conflict);
                }
                if n.revision
                    != expected_revision
                        .checked_add(1)
                        .ok_or(ContextError::Capacity)?
                {
                    return Err(ContextError::Stale);
                }
                self.validate_note(n)?;
                self.notes.insert(n.id.clone(), n.clone());
            }
            NotesCommand::Tombstone {
                id,
                expected_revision,
            } => {
                self.access(p, id, true)?;
                let n = self
                    .notes
                    .get_mut(id)
                    .ok_or(ContextError::Unavailable("note".into()))?;
                if n.revision != *expected_revision {
                    return Err(ContextError::Stale);
                }
                if n.tombstoned {
                    return Err(ContextError::Conflict);
                }
                n.revision = n.revision.checked_add(1).ok_or(ContextError::Capacity)?;
                n.tombstoned = true;
            }
            NotesCommand::SetGrant(g) => {
                self.owner(p)?;
                validate_id(&g.principal)?;
                validate_id(&g.note_id)?;
                if !self.notes.contains_key(&g.note_id) {
                    return Err(ContextError::Unavailable("note".into()));
                }
                self.grants
                    .insert((g.principal.clone(), g.note_id.clone()), g.clone());
            }
            NotesCommand::RevokeGrant { principal, note_id } => {
                self.owner(p)?;
                self.grants.remove(&(principal.clone(), note_id.clone()));
            }
            NotesCommand::SetAttributeGrant {
                principal,
                key,
                write,
            } => {
                self.owner(p)?;
                validate_id(principal)?;
                validate_id(key)?;
                let target = (principal.clone(), key.clone());
                if *write {
                    self.attribute_grants.insert(target);
                } else {
                    self.attribute_grants.remove(&target);
                }
            }
            NotesCommand::ProposeAttribute(a) => {
                if p != self.scope.owner_id
                    && !self.attribute_grants.contains(&(p.into(), a.key.clone()))
                {
                    return Err(ContextError::ScopeMismatch);
                }
                validate_id(&a.id)?;
                validate_id(&a.key)?;
                if a.proposer != p || a.value.len() > 4096 || self.proposals.contains_key(&a.id) {
                    return Err(ContextError::Conflict);
                }
                if a.base_revision != self.attributes.get(&a.key).map_or(0, |v| v.0) {
                    return Err(ContextError::Stale);
                }
                self.proposals.insert(a.id.clone(), a.clone());
            }
            NotesCommand::AcceptAttribute { proposal_id } => {
                self.owner(p)?;
                let a = self
                    .proposals
                    .get(proposal_id)
                    .ok_or(ContextError::Unavailable("proposal".into()))?;
                if a.base_revision != self.attributes.get(&a.key).map_or(0, |v| v.0) {
                    return Err(ContextError::Stale);
                }
                self.attributes.insert(
                    a.key.clone(),
                    (
                        a.base_revision
                            .checked_add(1)
                            .ok_or(ContextError::Capacity)?,
                        a.value.clone(),
                    ),
                );
                self.proposals.remove(proposal_id);
            }
            NotesCommand::RejectAttribute { proposal_id } => {
                self.owner(p)?;
                if self.proposals.remove(proposal_id).is_none() {
                    return Err(ContextError::Unavailable("proposal".into()));
                }
            }
        }
        Ok(())
    }
    pub fn read(
        &self,
        principal: &str,
        id: &str,
        now_ns: i64,
        facts: &BTreeMap<String, String>,
    ) -> Result<Option<&Note>, ContextError> {
        self.access(principal, id, false)?;
        let Some(n) = self.notes.get(id) else {
            return Ok(None);
        };
        if n.tombstoned
            || n.expires_at_ns.is_some_and(|time| now_ns >= time)
            || !n.predicate.evaluate(facts)?
        {
            Ok(None)
        } else {
            Ok(Some(n))
        }
    }
    pub fn attribute(
        &self,
        principal: &str,
        key: &str,
    ) -> Result<Option<&(u64, String)>, ContextError> {
        self.owner(principal)?;
        Ok(self.attributes.get(key))
    }
    pub fn proposals(&self, principal: &str) -> Result<Vec<&AttributeProposal>, ContextError> {
        self.owner(principal)?;
        Ok(self.proposals.values().collect())
    }
    pub fn events(&self, principal: &str) -> Result<Vec<&NotesEvent>, ContextError> {
        self.owner(principal)?;
        Ok(self.events.values().collect())
    }
}
