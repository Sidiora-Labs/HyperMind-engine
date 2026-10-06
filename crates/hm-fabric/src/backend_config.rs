use crate::bus::{BackendCapabilities, Bus};
use crate::bus_memory::{MemoryBus, MemoryBusLimits};
use crate::postgres::{
    PostgresDescriptor, PostgresError, PostgresOperationalStore, PostgresSecretHandle,
};
use crate::storage::{CanonicalRoot, FencedStore, Migration, StorageError, resolve_recovery};
use hm_context::types::{digest_bytes, validate_id};
use hm_context::{ContextError, Scope};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::path::{Component, Path, PathBuf};

pub const BACKEND_DESCRIPTOR_VERSION: u32 = 1;
const OWNER_MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "backend_binding",
    sql: "CREATE TABLE backend_descriptor_binding(id INTEGER PRIMARY KEY CHECK(id=1), descriptor TEXT NOT NULL, digest TEXT NOT NULL);",
}];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackendDescriptor {
    pub version: u32,
    pub scope: Scope,
    pub home: HomeDescriptor,
    pub operational: OperationalBackendDescriptor,
    pub bus: BusBackendDescriptor,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HomeDescriptor {
    pub path: PathBuf,
    pub base: Option<PathBuf>,
    pub missing: MissingHomePolicy,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "policy", rename_all = "snake_case", deny_unknown_fields)]
pub enum MissingHomePolicy {
    Refuse,
    Create { max_missing: usize },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "backend", rename_all = "snake_case", deny_unknown_fields)]
pub enum OperationalBackendDescriptor {
    Sqlite,
    Postgres {
        descriptor: PostgresDescriptor,
        secret_ref: String,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "backend", rename_all = "snake_case", deny_unknown_fields)]
pub enum BusBackendDescriptor {
    Sqlite,
    ProcessLocal { limits: MemoryBusLimits },
    Nats { server_url: String },
}
#[derive(Debug, thiserror::Error)]
pub enum BackendConfigError {
    #[error("invalid backend descriptor: {0}")]
    InvalidDescriptor(String),
    #[error("relative home requires an explicit existing base")]
    RelativeBaseRequired,
    #[error("home is missing; explicit creation policy required")]
    MissingHome,
    #[error("backend home identity collision")]
    IdentityMismatch,
    #[error("backend unavailable: {backend}: {reason}")]
    Unavailable { backend: String, reason: String },
    #[error("secret resolver unavailable for opaque handle")]
    SecretUnavailable,
    #[error("storage: {0}")]
    Storage(#[from] StorageError),
    #[error("PostgreSQL: {0}")]
    Postgres(#[from] PostgresError),
    #[error("bus: {0}")]
    Context(#[from] ContextError),
    #[error("filesystem: {0}")]
    Io(#[from] std::io::Error),
    #[error("descriptor serialization: {0}")]
    Json(#[from] serde_json::Error),
}

pub struct ValidatedDescriptor {
    descriptor: BackendDescriptor,
    path: PathBuf,
    existing_ancestor: PathBuf,
    missing_components: usize,
    identity: String,
    digest: String,
}
impl ValidatedDescriptor {
    pub fn home_path(&self) -> &Path {
        &self.path
    }
    pub fn identity_digest(&self) -> &str {
        &self.digest
    }
    pub fn descriptor(&self) -> &BackendDescriptor {
        &self.descriptor
    }
    pub fn missing_components(&self) -> usize {
        self.missing_components
    }
}

pub fn validate_descriptor(
    descriptor: &BackendDescriptor,
) -> Result<ValidatedDescriptor, BackendConfigError> {
    descriptor.scope.validate()?;
    if descriptor.version != BACKEND_DESCRIPTOR_VERSION
        || descriptor.home.path.as_os_str().is_empty()
    {
        return Err(BackendConfigError::InvalidDescriptor(
            "unsupported version or empty home".into(),
        ));
    }
    if descriptor
        .home
        .path
        .components()
        .any(|part| matches!(part, Component::ParentDir))
    {
        return Err(BackendConfigError::InvalidDescriptor(
            "home traversal is refused".into(),
        ));
    }
    match &descriptor.operational {
        OperationalBackendDescriptor::Sqlite => {}
        OperationalBackendDescriptor::Postgres {
            descriptor: postgres,
            secret_ref,
        } => {
            postgres.validate()?;
            validate_id(secret_ref)?;
            if postgres.scope != descriptor.scope {
                return Err(BackendConfigError::IdentityMismatch);
            }
        }
    }
    match &descriptor.bus {
        BusBackendDescriptor::ProcessLocal { limits } => {
            if [
                limits.resources,
                limits.events,
                limits.queue_depth,
                limits.consumers,
                limits.grants,
                limits.registers,
                limits.dead_letters,
            ]
            .iter()
            .any(|value| *value == 0 || *value > 100_000)
            {
                return Err(BackendConfigError::InvalidDescriptor(
                    "invalid process-local bus limits".into(),
                ));
            }
        }
        BusBackendDescriptor::Nats { server_url } => {
            if server_url.len() > 2048
                || !server_url.starts_with("nats://")
                || server_url.contains(['@', '?', '#', '%'])
                || server_url.chars().any(char::is_whitespace)
            {
                return Err(BackendConfigError::InvalidDescriptor(
                    "invalid or credential-bearing NATS endpoint".into(),
                ));
            }
        }
        BusBackendDescriptor::Sqlite => {}
    }
    let path = if descriptor.home.path.is_absolute() {
        if descriptor.home.base.is_some() {
            return Err(BackendConfigError::InvalidDescriptor(
                "absolute home cannot carry an ignored base".into(),
            ));
        }
        descriptor.home.path.clone()
    } else {
        let base = descriptor
            .home
            .base
            .as_ref()
            .ok_or(BackendConfigError::RelativeBaseRequired)?;
        if !base.is_absolute() {
            return Err(BackendConfigError::RelativeBaseRequired);
        }
        CanonicalRoot::admit(base)?
            .path()
            .join(&descriptor.home.path)
    };
    let max_missing = match descriptor.home.missing {
        MissingHomePolicy::Refuse => 0,
        MissingHomePolicy::Create { max_missing } if max_missing > 0 && max_missing <= 16 => {
            max_missing
        }
        _ => {
            return Err(BackendConfigError::InvalidDescriptor(
                "home creation bound must be 1 through 16".into(),
            ));
        }
    };
    if max_missing == 0 && !path.exists() {
        return Err(BackendConfigError::MissingHome);
    }
    let recovery = resolve_recovery(&path, max_missing)?;
    if recovery.missing_components == 0 {
        CanonicalRoot::admit(&recovery.resolved)?;
    }
    let mut normalized = descriptor.clone();
    normalized.home = HomeDescriptor {
        path: recovery.resolved.clone(),
        base: None,
        missing: MissingHomePolicy::Refuse,
    };
    let identity = serde_json::to_string(&normalized)?;
    let digest = digest_bytes(identity.as_bytes());
    Ok(ValidatedDescriptor {
        descriptor: normalized,
        path: recovery.resolved,
        existing_ancestor: recovery.existing_ancestor,
        missing_components: recovery.missing_components,
        identity,
        digest,
    })
}

pub enum SelectedOperationalStore {
    Sqlite(FencedStore),
    Postgres(PostgresOperationalStore),
}
pub enum SelectedBus {
    Sqlite(Bus),
    ProcessLocal(MemoryBus),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BackendReadiness {
    pub identity_digest: String,
    pub owner_epoch: u64,
    pub operational_epoch: u64,
    pub operational_backend: String,
    pub bus_capabilities: BackendCapabilities,
}
pub struct SelectedBackends {
    pub operational: SelectedOperationalStore,
    pub bus: SelectedBus,
    pub owner: FencedStore,
    home: CanonicalRoot,
    descriptor: BackendDescriptor,
    identity: String,
    digest: String,
}
impl SelectedBackends {
    pub fn home(&self) -> &Path {
        self.home.path()
    }
    pub fn descriptor(&self) -> &BackendDescriptor {
        &self.descriptor
    }
    pub fn identity_digest(&self) -> &str {
        &self.digest
    }
    pub fn readiness(&mut self) -> Result<BackendReadiness, BackendConfigError> {
        let owner_epoch = self.owner.epoch();
        self.owner.read(|db| {
            let epoch: i64 =
                db.query_row("SELECT epoch FROM hm_store_meta WHERE id=1", [], |row| {
                    row.get(0)
                })?;
            let (identity, digest): (String, String) = db.query_row(
                "SELECT descriptor,digest FROM backend_descriptor_binding WHERE id=1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
            if epoch as u64 != owner_epoch || identity != self.identity || digest != self.digest {
                return Err(StorageError::StaleEpoch);
            }
            Ok(())
        })?;
        let (operational_backend, operational_epoch) = match &mut self.operational {
            SelectedOperationalStore::Sqlite(store) => {
                let epoch = store.epoch();
                store.read(|db| {
                    let current: i64 =
                        db.query_row("SELECT epoch FROM hm_store_meta WHERE id=1", [], |row| {
                            row.get(0)
                        })?;
                    if current as u64 != epoch {
                        return Err(StorageError::StaleEpoch);
                    }
                    Ok(())
                })?;
                ("sqlite".to_owned(), epoch)
            }
            SelectedOperationalStore::Postgres(store) => {
                let epoch = store.epoch();
                store.read(epoch, |tx| {
                    tx.query_one("SELECT 1", &[])?;
                    Ok(())
                })?;
                ("postgresql".to_owned(), epoch)
            }
        };
        let bus_capabilities = match &self.bus {
            SelectedBus::Sqlite(bus) => bus.capabilities(),
            SelectedBus::ProcessLocal(bus) => bus.capabilities(),
        };
        Ok(BackendReadiness {
            identity_digest: self.digest.clone(),
            owner_epoch,
            operational_epoch,
            operational_backend,
            bus_capabilities,
        })
    }
}

pub fn select_backend(
    validated: ValidatedDescriptor,
    migrations: &[Migration],
    mut resolve_secret: impl FnMut(&str) -> Result<PostgresSecretHandle, BackendConfigError>,
) -> Result<SelectedBackends, BackendConfigError> {
    if matches!(&validated.descriptor.bus, BusBackendDescriptor::Nats { .. }) {
        return Err(BackendConfigError::Unavailable {
            backend: "nats".into(),
            reason: "native selection requires an authenticated provisioned NATS adapter".into(),
        });
    }
    let secret = match &validated.descriptor.operational {
        OperationalBackendDescriptor::Postgres { secret_ref, .. } => {
            Some(resolve_secret(secret_ref)?)
        }
        OperationalBackendDescriptor::Sqlite => None,
    };
    let home = create_home(&validated)?;
    let mut owner = FencedStore::open(home.path().join("backend-owner.sqlite"), OWNER_MIGRATIONS)?;
    let old = owner.read(|db| {
        use rusqlite::OptionalExtension;
        Ok(db
            .query_row(
                "SELECT descriptor,digest FROM backend_descriptor_binding WHERE id=1",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?)
    })?;
    if old.is_some_and(|(identity, digest)| {
        identity != validated.identity || digest != validated.digest
    }) {
        return Err(BackendConfigError::IdentityMismatch);
    }
    owner.transaction(owner.epoch(), |tx| {
        tx.execute(
            "INSERT OR IGNORE INTO backend_descriptor_binding VALUES(1,?1,?2)",
            rusqlite::params![validated.identity, validated.digest],
        )?;
        Ok(())
    })?;
    let operational = match &validated.descriptor.operational {
        OperationalBackendDescriptor::Sqlite => SelectedOperationalStore::Sqlite(
            FencedStore::open(home.path().join("backend-operations.sqlite"), migrations)?,
        ),
        OperationalBackendDescriptor::Postgres { descriptor, .. } => {
            SelectedOperationalStore::Postgres(PostgresOperationalStore::open(
                descriptor,
                secret
                    .as_ref()
                    .ok_or(BackendConfigError::SecretUnavailable)?,
                migrations,
            )?)
        }
    };
    let bus = match &validated.descriptor.bus {
        BusBackendDescriptor::Sqlite => {
            private_file(&home.path().join("backend-bus.sqlite"))?;
            private_file(&home.path().join("backend-bus.sqlite.writer-lock"))?;
            SelectedBus::Sqlite(Bus::open(
                home.path().join("backend-bus.sqlite"),
                validated.descriptor.scope.clone(),
            )?)
        }
        BusBackendDescriptor::ProcessLocal { limits } => {
            SelectedBus::ProcessLocal(MemoryBus::new(validated.descriptor.scope.clone(), *limits)?)
        }
        BusBackendDescriptor::Nats { .. } => {
            return Err(BackendConfigError::Unavailable {
                backend: "nats".into(),
                reason: "native selection requires an authenticated provisioned NATS adapter"
                    .into(),
            });
        }
    };
    let mut selected = SelectedBackends {
        operational,
        bus,
        owner,
        home,
        descriptor: validated.descriptor,
        identity: validated.identity,
        digest: validated.digest,
    };
    selected.readiness()?;
    Ok(selected)
}
fn create_home(validated: &ValidatedDescriptor) -> Result<CanonicalRoot, BackendConfigError> {
    if validated.missing_components == 0 {
        let root = CanonicalRoot::admit(&validated.path)?;
        if root.path() != validated.path {
            return Err(BackendConfigError::IdentityMismatch);
        }
        return Ok(root);
    }
    let suffix = validated
        .path
        .strip_prefix(&validated.existing_ancestor)
        .map_err(|_| BackendConfigError::IdentityMismatch)?;
    let mut current = validated.existing_ancestor.clone();
    for component in suffix.components() {
        let Component::Normal(component) = component else {
            return Err(BackendConfigError::IdentityMismatch);
        };
        current.push(component);
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        match builder.create(&current) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
        let metadata = fs::symlink_metadata(&current)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(BackendConfigError::IdentityMismatch);
        }
    }
    let root = CanonicalRoot::admit(&validated.path)?;
    if root.path() != validated.path {
        return Err(BackendConfigError::IdentityMismatch);
    }
    Ok(root)
}
fn private_file(path: &Path) -> Result<(), BackendConfigError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err(BackendConfigError::IdentityMismatch);
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                if metadata.nlink() != 1 {
                    return Err(BackendConfigError::IdentityMismatch);
                }
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}
