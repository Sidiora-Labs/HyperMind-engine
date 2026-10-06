use crate::types::{digest_bytes, validate_id, ContextError, Scope, SourceMessage};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemporalTimes {
    #[serde(default, with = "crate::types::optional_timestamp_wire")]
    pub occurred_at_ns: Option<i64>,
    #[serde(with = "crate::types::timestamp_wire")]
    pub recorded_at_ns: i64,
    #[serde(default, with = "crate::types::optional_timestamp_wire")]
    pub valid_from_ns: Option<i64>,
    #[serde(default, with = "crate::types::optional_timestamp_wire")]
    pub valid_until_ns: Option<i64>,
    #[serde(default, with = "crate::types::optional_timestamp_wire")]
    pub verified_at_ns: Option<i64>,
}
impl TemporalTimes {
    pub fn validate(&self) -> Result<(), ContextError> {
        if matches!((self.valid_from_ns, self.valid_until_ns), (Some(a), Some(b)) if a > b) {
            return Err(ContextError::Invalid("reversed validity interval".into()));
        }
        Ok(())
    }
    pub fn local_occurrence_ns(
        &self,
        utc_offset_seconds: i32,
    ) -> Result<Option<i64>, ContextError> {
        validate_offset(utc_offset_seconds)?;
        self.occurred_at_ns
            .map(|ns| {
                ns.checked_add(i64::from(utc_offset_seconds) * 1_000_000_000)
                    .ok_or(ContextError::Capacity)
            })
            .transpose()
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceIdentity {
    pub id: String,
    pub digest: String,
    pub ordinal: u64,
}
impl SourceIdentity {
    pub fn validate(&self) -> Result<(), ContextError> {
        validate_id(&self.id)?;
        if self.digest.len() != 64
            || !self
                .digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(ContextError::Invalid("invalid source digest".into()));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemporalSource {
    pub source: SourceIdentity,
    pub times: TemporalTimes,
}
impl TemporalSource {
    pub fn from_message(message: &SourceMessage) -> Result<Self, ContextError> {
        message.validate()?;
        Ok(Self {
            source: SourceIdentity {
                id: message.id.clone(),
                digest: message.source_digest.clone(),
                ordinal: message.ordinal,
            },
            times: TemporalTimes {
                occurred_at_ns: message.occurred_at_ns,
                recorded_at_ns: message.recorded_at_ns,
                valid_from_ns: None,
                valid_until_ns: None,
                verified_at_ns: None,
            },
        })
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemporalGap {
    pub marker: String,
    pub source: SourceIdentity,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClockProvenance {
    SourceDeclaredUtc,
    Unknown,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemporalClocks {
    pub source: SourceIdentity,
    pub occurred_at: ClockProvenance,
    pub recorded_at: ClockProvenance,
    pub valid_from: ClockProvenance,
    pub valid_until: ClockProvenance,
    pub verified_at: ClockProvenance,
}
impl TemporalSource {
    pub fn clock_provenance(&self) -> TemporalClocks {
        let declared = |time: Option<i64>| {
            if time.is_some() {
                ClockProvenance::SourceDeclaredUtc
            } else {
                ClockProvenance::Unknown
            }
        };
        TemporalClocks {
            source: self.source.clone(),
            occurred_at: declared(self.times.occurred_at_ns),
            recorded_at: ClockProvenance::SourceDeclaredUtc,
            valid_from: declared(self.times.valid_from_ns),
            valid_until: declared(self.times.valid_until_ns),
            verified_at: declared(self.times.verified_at_ns),
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemporalContext {
    pub scope: Scope,
    pub session_id: String,
    pub utc_offset_seconds: Option<i32>,
    #[serde(default)]
    pub clock_provenance: Vec<TemporalClocks>,
    pub sources: Vec<TemporalSource>,
    pub coverage: Vec<SourceIdentity>,
    pub gaps: Vec<TemporalGap>,
}
impl TemporalContext {
    pub fn build(
        scope: Scope,
        session_id: String,
        utc_offset_seconds: i32,
        sources: Vec<TemporalSource>,
        expected: Vec<SourceIdentity>,
    ) -> Result<Self, ContextError> {
        Self::build_with_offset(
            scope,
            session_id,
            Some(utc_offset_seconds),
            sources,
            expected,
        )
    }
    pub fn build_with_offset(
        scope: Scope,
        session_id: String,
        utc_offset_seconds: Option<i32>,
        mut sources: Vec<TemporalSource>,
        mut expected: Vec<SourceIdentity>,
    ) -> Result<Self, ContextError> {
        scope.validate()?;
        validate_id(&session_id)?;
        if let Some(offset) = utc_offset_seconds {
            validate_offset(offset)?;
        }
        expected.sort_by(|a, b| (a.ordinal, &a.id).cmp(&(b.ordinal, &b.id)));
        sources.sort_by(|a, b| {
            (a.source.ordinal, &a.source.id).cmp(&(b.source.ordinal, &b.source.id))
        });
        let mut ids = BTreeSet::new();
        let mut ordinals = BTreeSet::new();
        for source in &expected {
            source.validate()?;
            if !ids.insert(&source.id) || !ordinals.insert(source.ordinal) {
                return Err(ContextError::Conflict);
            }
        }
        let mut seen = BTreeSet::new();
        for source in &sources {
            source.source.validate()?;
            source.times.validate()?;
            if !expected.contains(&source.source) || !seen.insert(&source.source.id) {
                return Err(ContextError::Conflict);
            }
        }
        let coverage = sources.iter().map(|s| s.source.clone()).collect();
        let gaps = expected
            .iter()
            .filter(|s| !seen.contains(&s.id))
            .map(|source| {
                let bytes = serde_json::to_vec(&("temporal-gap-v1", &scope, &session_id, source))?;
                Ok(TemporalGap {
                    marker: format!("gap:{}", digest_bytes(&bytes)),
                    source: source.clone(),
                })
            })
            .collect::<Result<Vec<_>, ContextError>>()?;
        Ok(Self {
            scope,
            session_id,
            utc_offset_seconds,
            clock_provenance: sources
                .iter()
                .map(TemporalSource::clock_provenance)
                .collect(),
            sources,
            coverage,
            gaps,
        })
    }
}
pub fn validate_offset(offset: i32) -> Result<(), ContextError> {
    if !(-86400..=86400).contains(&offset) {
        return Err(ContextError::Invalid("UTC offset exceeds one day".into()));
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalIdentity {
    pub namespace: String,
    pub external_id: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityBinding {
    pub scope: Scope,
    pub actor: u16,
    pub aliases: Vec<ExternalIdentity>,
}
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityRegistry {
    bindings: Vec<IdentityBinding>,
}
impl IdentityRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn bindings(&self) -> &[IdentityBinding] {
        &self.bindings
    }
    pub fn bind(
        &mut self,
        scope: Scope,
        actor: u16,
        mut aliases: Vec<ExternalIdentity>,
    ) -> Result<(), ContextError> {
        scope.validate()?;
        for alias in &aliases {
            validate_id(&alias.namespace)?;
            validate_id(&alias.external_id)?;
        }
        aliases.sort();
        aliases.dedup();
        for existing in &self.bindings {
            if (existing.scope == scope && existing.actor != actor)
                || (existing.actor == actor && existing.scope != scope)
            {
                return Err(ContextError::Conflict);
            }
            if existing.scope.owner_id == scope.owner_id
                && existing.scope != scope
                && aliases.iter().any(|a| existing.aliases.contains(a))
            {
                return Err(ContextError::Conflict);
            }
        }
        if let Some(existing) = self.bindings.iter_mut().find(|b| b.scope == scope) {
            existing.aliases.extend(aliases);
            existing.aliases.sort();
            existing.aliases.dedup();
        } else {
            self.bindings.push(IdentityBinding {
                scope,
                actor,
                aliases,
            });
            self.bindings.sort_by(|a, b| a.scope.cmp(&b.scope));
        }
        Ok(())
    }
    pub fn resolve(
        &self,
        owner: &str,
        namespace: &str,
        external: &str,
    ) -> Option<&IdentityBinding> {
        self.bindings.iter().find(|b| {
            b.scope.owner_id == owner
                && b.aliases
                    .iter()
                    .any(|a| a.namespace == namespace && a.external_id == external)
        })
    }
    pub fn actor_for_scope(&self, scope: &Scope) -> Option<u16> {
        self.bindings
            .iter()
            .find(|b| &b.scope == scope)
            .map(|b| b.actor)
    }
    pub fn load(path: &Path) -> Result<Self, ContextError> {
        let decoded: Self = serde_json::from_slice(&std::fs::read(path)?)?;
        let mut checked = Self::new();
        for binding in decoded.bindings {
            checked.bind(binding.scope, binding.actor, binding.aliases)?;
        }
        Ok(checked)
    }
    pub fn save(&self, path: &Path) -> Result<(), ContextError> {
        let temp = path.with_extension("identity-pending");
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        let result = (|| -> Result<(), ContextError> {
            file.write_all(&serde_json::to_vec(self)?)?;
            file.sync_all()?;
            std::fs::rename(&temp, path)?;
            std::fs::File::open(
                path.parent()
                    .filter(|p| !p.as_os_str().is_empty())
                    .unwrap_or(Path::new(".")),
            )?
            .sync_all()?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(temp);
        }
        result
    }
}
pub fn canonical_workspace(path: &Path) -> Result<PathBuf, ContextError> {
    let canonical = std::fs::canonicalize(path)?;
    if !canonical.is_dir() {
        return Err(ContextError::Invalid("workspace is not a directory".into()));
    }
    Ok(canonical)
}
