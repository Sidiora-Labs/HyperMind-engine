use crate::bus::Register;
use hm_context::{ContextError, Scope};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StreamFamily {
    RoomPosts,
    WakeSignals,
    PeerDeliveries,
    EffectIntents,
    DeadLetterEffects,
    ModuleEvents,
}
impl StreamFamily {
    pub const ALL: [Self; 6] = [
        Self::RoomPosts,
        Self::WakeSignals,
        Self::PeerDeliveries,
        Self::EffectIntents,
        Self::DeadLetterEffects,
        Self::ModuleEvents,
    ];
    pub fn token(self) -> &'static str {
        match self {
            Self::RoomPosts => "room-posts",
            Self::WakeSignals => "wake-signals",
            Self::PeerDeliveries => "peer-deliveries",
            Self::EffectIntents => "effect-intents",
            Self::DeadLetterEffects => "dead-letter-effects",
            Self::ModuleEvents => "module-events",
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BusTopology {
    pub scope: Scope,
    pub root: String,
    pub census_bucket: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DurableNames {
    pub agent: String,
    pub module: String,
}
pub fn validate_token(token: &str) -> Result<(), ContextError> {
    if token.len() > 96
        || !token
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        || !token
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(ContextError::Invalid("invalid scoped bus token".into()));
    }
    Ok(())
}
impl BusTopology {
    pub fn new(scope: Scope) -> Result<Self, ContextError> {
        scope.validate()?;
        let mut parts = vec![scope.owner_id.as_str(), scope.project_id.as_str()];
        if let Some(workspace) = &scope.workspace_id {
            parts.push(workspace);
        }
        for part in &parts {
            validate_token(part)?;
        }
        let root = format!("hm.{}", parts.join("."));
        let census_bucket = format!("HM_CENSUS_{}", parts.join("_"));
        Ok(Self {
            scope,
            root,
            census_bucket,
        })
    }
    fn scoped_tokens(&self) -> String {
        self.root[3..].replace('.', "_")
    }
    pub fn stream_subject(&self, family: StreamFamily) -> String {
        format!("{}.{}.>", self.root, family.token())
    }
    pub fn stream_name(&self, family: StreamFamily) -> String {
        format!(
            "HM_{}_{}",
            self.scoped_tokens().to_ascii_uppercase(),
            family.token().replace('-', "_").to_ascii_uppercase()
        )
    }
    pub fn subject(
        &self,
        family: StreamFamily,
        principal: &str,
        event: &str,
    ) -> Result<String, ContextError> {
        validate_token(principal)?;
        validate_token(event)?;
        Ok(format!(
            "{}.{}.{}.{}",
            self.root,
            family.token(),
            principal,
            event
        ))
    }
    pub fn durables(&self, agent: &str, module: &str) -> Result<DurableNames, ContextError> {
        validate_token(agent)?;
        validate_token(module)?;
        let scope = self.scoped_tokens();
        Ok(DurableNames {
            agent: format!("hm_{scope}_agent_{agent}"),
            module: format!("hm_{scope}_module_{module}"),
        })
    }
    pub fn validate_families(&self) -> Result<(), ContextError> {
        let names: std::collections::BTreeSet<_> = StreamFamily::ALL
            .iter()
            .map(|f| self.stream_subject(*f))
            .collect();
        if names.len() != 6 {
            return Err(ContextError::Conflict);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RegisterSnapshot {
    pub revision: u64,
    pub entries: Vec<(String, Register)>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RegisterUpdate {
    Put { key: String, register: Register },
    Delete { key: String, revision: u64 },
}
impl RegisterUpdate {
    pub fn revision(&self) -> u64 {
        match self {
            Self::Put { register, .. } => register.revision,
            Self::Delete { revision, .. } => *revision,
        }
    }
}
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct CensusGuard {
    revision: Option<u64>,
    present: bool,
}
impl CensusGuard {
    pub fn observe_snapshot(&mut self, snapshot: &RegisterSnapshot, key: &str) {
        if self.revision.is_some_and(|r| r > snapshot.revision) {
            return;
        }
        self.revision = Some(snapshot.revision);
        self.present = snapshot.entries.iter().any(|(k, _)| k == key);
    }
    pub fn observe(&mut self, update: &RegisterUpdate, key: &str) {
        if self.revision.is_some_and(|r| r >= update.revision()) {
            return;
        }
        match update {
            RegisterUpdate::Put { key: k, .. } if k == key => {
                self.present = true;
                self.revision = Some(update.revision());
            }
            RegisterUpdate::Delete { key: k, .. } if k == key => {
                self.present = false;
                self.revision = Some(update.revision());
            }
            _ => {}
        }
    }
    pub fn disconnect(&mut self) {
        self.revision = None;
        self.present = false;
    }
    pub fn proven_absent(&self) -> bool {
        self.revision.is_some() && !self.present
    }
    pub fn revision(&self) -> Option<u64> {
        self.revision
    }
}
