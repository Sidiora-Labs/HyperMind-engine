use fs2::FileExt;
use rusqlite::{Connection, Transaction, TransactionBehavior};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("storage IO: {0}")]
    Io(#[from] std::io::Error),
    #[error("storage SQL: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("writer already active")]
    Busy,
    #[error("stale writer epoch")]
    StaleEpoch,
    #[error("store version {store} exceeds supported version {supported}")]
    StoreAhead { store: u32, supported: u32 },
    #[error("migration sequence or checksum mismatch")]
    MigrationMismatch,
    #[error("invalid or unbounded path")]
    InvalidPath,
    #[error("writer epoch exhausted")]
    EpochExhausted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalRoot(PathBuf);
impl CanonicalRoot {
    pub fn admit(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let path = fs::canonicalize(path)?;
        if !path.is_dir() {
            return Err(StorageError::InvalidPath);
        }
        Ok(Self(path))
    }
    pub fn path(&self) -> &Path {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryPath {
    pub existing_ancestor: PathBuf,
    pub resolved: PathBuf,
    pub missing_components: usize,
}
pub fn resolve_recovery(
    path: impl AsRef<Path>,
    max_missing: usize,
) -> Result<RecoveryPath, StorageError> {
    let path = path.as_ref();
    if path
        .components()
        .any(|part| matches!(part, Component::ParentDir))
    {
        return Err(StorageError::InvalidPath);
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut ancestor = absolute.clone();
    let mut missing = Vec::new();
    loop {
        match fs::symlink_metadata(&ancestor) {
            Ok(_) => break,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if missing.len() >= max_missing {
                    return Err(StorageError::InvalidPath);
                }
                missing.push(
                    ancestor
                        .file_name()
                        .ok_or(StorageError::InvalidPath)?
                        .to_owned(),
                );
                if !ancestor.pop() {
                    return Err(StorageError::InvalidPath);
                }
            }
            Err(error) => return Err(error.into()),
        }
    }
    let ancestor = fs::canonicalize(ancestor)?;
    if !missing.is_empty() && !ancestor.is_dir() {
        return Err(StorageError::InvalidPath);
    }
    let mut resolved = ancestor.clone();
    for component in missing.iter().rev() {
        resolved.push(component);
    }
    Ok(RecoveryPath {
        existing_ancestor: ancestor,
        resolved,
        missing_components: missing.len(),
    })
}

#[derive(Clone, Copy, Debug)]
pub struct Migration {
    pub version: u32,
    pub name: &'static str,
    pub sql: &'static str,
}
impl Migration {
    pub fn checksum(&self) -> String {
        let mut hash = Sha256::new();
        hash.update(self.version.to_be_bytes());
        hash.update(self.name.as_bytes());
        hash.update([0]);
        hash.update(self.sql.as_bytes());
        format!("{:x}", hash.finalize())
    }
}

pub struct FencedStore {
    connection: Connection,
    _lock: File,
    path: PathBuf,
    epoch: u64,
    identity: FileIdentity,
    lock_path: PathBuf,
    lock_identity: FileIdentity,
}
impl FencedStore {
    pub fn open(path: impl AsRef<Path>, migrations: &[Migration]) -> Result<Self, StorageError> {
        for (index, migration) in migrations.iter().enumerate() {
            if migration.version as usize != index + 1 || migration.name.is_empty() {
                return Err(StorageError::MigrationMismatch);
            }
        }
        let input = path.as_ref();
        let parent = fs::canonicalize(
            input
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new(".")),
        )?;
        if !parent.is_dir() {
            return Err(StorageError::InvalidPath);
        }
        let path = parent.join(input.file_name().ok_or(StorageError::InvalidPath)?);
        reject_symlink(&path)?;
        let mut lock_name = path
            .file_name()
            .ok_or(StorageError::InvalidPath)?
            .to_os_string();
        lock_name.push(".writer-lock");
        let lock_path = path.with_file_name(lock_name);
        reject_symlink(&lock_path)?;
        let lock = private_file(&lock_path)?;
        lock.try_lock_exclusive().map_err(|error| {
            if error.kind() == std::io::ErrorKind::WouldBlock {
                StorageError::Busy
            } else {
                StorageError::Io(error)
            }
        })?;
        let database = private_file(&path)?;
        drop(database);
        let mut connection = Connection::open(&path)?;
        connection.execute_batch(
            "PRAGMA journal_mode=DELETE; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;",
        )?;
        connection.execute_batch("CREATE TABLE IF NOT EXISTS hm_store_meta (id INTEGER PRIMARY KEY CHECK(id=1), epoch INTEGER NOT NULL); INSERT OR IGNORE INTO hm_store_meta VALUES(1,0); CREATE TABLE IF NOT EXISTS hm_store_migrations(version INTEGER PRIMARY KEY, name TEXT NOT NULL, checksum TEXT NOT NULL);")?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let applied: Vec<(u32, String, String)> = {
            let mut statement = tx.prepare(
                "SELECT version,name,checksum FROM hm_store_migrations ORDER BY version",
            )?;
            let rows =
                statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
            rows.collect::<Result<_, _>>()?
        };
        let supported = migrations.len() as u32;
        if let Some((store, _, _)) = applied.last() {
            if *store > supported {
                return Err(StorageError::StoreAhead {
                    store: *store,
                    supported,
                });
            }
        }
        for (index, (version, name, checksum)) in applied.iter().enumerate() {
            let expected = migrations
                .get(index)
                .ok_or(StorageError::MigrationMismatch)?;
            if *version != expected.version
                || *name != expected.name
                || *checksum != expected.checksum()
            {
                return Err(StorageError::MigrationMismatch);
            }
        }
        for migration in &migrations[applied.len()..] {
            tx.execute_batch(migration.sql)?;
            tx.execute(
                "INSERT INTO hm_store_migrations VALUES(?1,?2,?3)",
                rusqlite::params![migration.version, migration.name, migration.checksum()],
            )?;
        }
        let previous: i64 =
            tx.query_row("SELECT epoch FROM hm_store_meta WHERE id=1", [], |row| {
                row.get(0)
            })?;
        let epoch = previous
            .checked_add(1)
            .filter(|epoch| *epoch > 0)
            .ok_or(StorageError::EpochExhausted)?;
        tx.execute("UPDATE hm_store_meta SET epoch=?1 WHERE id=1", [epoch])?;
        tx.commit()?;
        let identity = FileIdentity::read(&path)?;
        let lock_identity = FileIdentity::read(&lock_path)?;
        Ok(Self {
            connection,
            _lock: lock,
            path,
            epoch: epoch as u64,
            identity,
            lock_path,
            lock_identity,
        })
    }
    fn check_identity(&self) -> Result<(), StorageError> {
        if FileIdentity::read(&self.path)? != self.identity
            || FileIdentity::read(&self.lock_path)? != self.lock_identity
        {
            return Err(StorageError::StaleEpoch);
        }
        Ok(())
    }
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn read<T>(
        &self,
        read: impl FnOnce(&Connection) -> Result<T, StorageError>,
    ) -> Result<T, StorageError> {
        self.check_identity()?;
        self.connection.pragma_update(None, "query_only", true)?;
        let guard = ReadGuard(&self.connection);
        let result = read(&self.connection);
        drop(guard);
        result
    }
    pub fn transaction<T>(
        &mut self,
        expected_epoch: u64,
        write: impl FnOnce(&Transaction<'_>) -> Result<T, StorageError>,
    ) -> Result<T, StorageError> {
        self.check_identity()?;
        if expected_epoch != self.epoch {
            return Err(StorageError::StaleEpoch);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let epoch: i64 = tx.query_row("SELECT epoch FROM hm_store_meta WHERE id=1", [], |row| {
            row.get(0)
        })?;
        if epoch as u64 != expected_epoch {
            return Err(StorageError::StaleEpoch);
        }
        let output = write(&tx)?;
        let current: i64 =
            tx.query_row("SELECT epoch FROM hm_store_meta WHERE id=1", [], |row| {
                row.get(0)
            })?;
        if current as u64 != expected_epoch {
            return Err(StorageError::StaleEpoch);
        }
        if FileIdentity::read(&self.path)? != self.identity
            || FileIdentity::read(&self.lock_path)? != self.lock_identity
        {
            return Err(StorageError::StaleEpoch);
        }
        tx.commit()?;
        Ok(output)
    }
}
fn reject_symlink(path: &Path) -> Result<(), StorageError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(StorageError::InvalidPath)
        }
        Ok(metadata) => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                if metadata.nlink() != 1 {
                    return Err(StorageError::InvalidPath);
                }
            }
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}
fn private_file(path: &Path) -> Result<File, StorageError> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true);
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
    Ok(file)
}

struct ReadGuard<'a>(&'a Connection);
impl Drop for ReadGuard<'_> {
    fn drop(&mut self) {
        let _ = self.0.pragma_update(None, "query_only", false);
    }
}

#[derive(Eq, PartialEq)]
struct FileIdentity {
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
    #[cfg(not(unix))]
    created: Option<std::time::SystemTime>,
}
impl FileIdentity {
    fn read(path: &Path) -> Result<Self, StorageError> {
        reject_symlink(path)?;
        let metadata = fs::metadata(path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            Ok(Self {
                device: metadata.dev(),
                inode: metadata.ino(),
            })
        }
        #[cfg(not(unix))]
        {
            Ok(Self {
                created: metadata.created().ok(),
            })
        }
    }
}
