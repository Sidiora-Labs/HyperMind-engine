use crate::storage::Migration;
use hm_context::types::{Scope, digest_bytes};
use postgres::{Client, Config, NoTls, Transaction};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostgresDescriptor {
    pub scope: Scope,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub database: String,
    pub schema: String,
    pub namespace: String,
}
impl PostgresDescriptor {
    pub fn validate(&self) -> Result<(), PostgresError> {
        self.scope
            .validate()
            .map_err(|_| PostgresError::InvalidDescriptor)?;
        if !matches!(self.host.as_str(), "127.0.0.1" | "::1") || self.port == 0 {
            return Err(PostgresError::InvalidDescriptor);
        }
        for id in [&self.user, &self.database, &self.schema, &self.namespace] {
            if id.is_empty()
                || id.len() > 63
                || !id
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
                || id.as_bytes()[0].is_ascii_digit()
            {
                return Err(PostgresError::InvalidDescriptor);
            }
        }
        if self.schema == "public" || self.schema.starts_with("pg_") {
            return Err(PostgresError::InvalidDescriptor);
        }
        Ok(())
    }
    pub fn identity(&self) -> Result<String, PostgresError> {
        self.validate()?;
        serde_json::to_string(self).map_err(|_| PostgresError::InvalidDescriptor)
    }
    pub fn identity_digest(&self) -> Result<String, PostgresError> {
        Ok(digest_bytes(self.identity()?.as_bytes()))
    }
}

pub struct PostgresSecretHandle {
    password: String,
}
impl PostgresSecretHandle {
    pub fn new(password: String) -> Self {
        Self { password }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WriterStatus {
    Live,
    Lost,
    Stale,
}

#[derive(Debug, thiserror::Error)]
pub enum PostgresError {
    #[error("invalid PostgreSQL operational descriptor")]
    InvalidDescriptor,
    #[error("pre-provisioned least-privilege namespace required")]
    ProvisioningRequired,
    #[error("writer already active")]
    Busy,
    #[error("operational namespace identity mismatch")]
    IdentityMismatch,
    #[error("stale writer fence")]
    StaleEpoch,
    #[error("writer connection lost; explicit reopen required")]
    ConnectionLost,
    #[error("store version {store} exceeds supported version {supported}")]
    StoreAhead { store: i32, supported: u32 },
    #[error("migration sequence or checksum mismatch")]
    MigrationMismatch,
    #[error("writer epoch exhausted")]
    EpochExhausted,
    #[error("PostgreSQL operation failed: {0}")]
    Driver(#[from] postgres::Error),
}

pub struct PostgresOperationalStore {
    client: Client,
    descriptor: PostgresDescriptor,
    epoch: u64,
    lock_key: i64,
    status: WriterStatus,
}
impl PostgresOperationalStore {
    pub fn open(
        descriptor: &PostgresDescriptor,
        secret: &PostgresSecretHandle,
        migrations: &[Migration],
    ) -> Result<Self, PostgresError> {
        descriptor.validate()?;
        for (index, migration) in migrations.iter().enumerate() {
            if migration.version as usize != index + 1
                || migration.name.is_empty()
                || migration.version > i32::MAX as u32
            {
                return Err(PostgresError::MigrationMismatch);
            }
        }
        let mut config = Config::new();
        config
            .host(&descriptor.host)
            .port(descriptor.port)
            .user(&descriptor.user)
            .dbname(&descriptor.database)
            .password(&secret.password)
            .connect_timeout(Duration::from_secs(5));
        let mut client = config.connect(NoTls)?;
        let row = client.query_one("SELECT current_database(), NOT rolsuper AND NOT rolcreatedb AND NOT rolcreaterole AND NOT rolreplication AND NOT rolbypassrls FROM pg_roles WHERE rolname=current_user", &[])?;
        if row.get::<_, String>(0) != descriptor.database || !row.get::<_, bool>(1) {
            return Err(PostgresError::ProvisioningRequired);
        }
        let row = client.query_one("SELECT EXISTS(SELECT 1 FROM pg_namespace WHERE nspname=$1 AND has_schema_privilege(oid,'USAGE') AND has_schema_privilege(oid,'CREATE')) AND NOT has_database_privilege(current_database(),'CREATE')", &[&descriptor.schema])?;
        if !row.get::<_, bool>(0) {
            return Err(PostgresError::ProvisioningRequired);
        }
        client.batch_execute(&format!("SET search_path TO \"{}\", pg_catalog; SET statement_timeout='15s'; SET lock_timeout='5s'; SET idle_in_transaction_session_timeout='20s';", descriptor.schema))?;
        let lock_digest = digest_bytes(
            format!("hypermind.operational.schema.v1:{}", descriptor.schema).as_bytes(),
        );
        let lock_key = u64::from_str_radix(&lock_digest[..16], 16)
            .map_err(|_| PostgresError::InvalidDescriptor)? as i64;
        if !client
            .query_one("SELECT pg_try_advisory_lock($1)", &[&lock_key])?
            .get::<_, bool>(0)
        {
            return Err(PostgresError::Busy);
        }
        let identity = descriptor.identity()?;
        let digest = descriptor.identity_digest()?;
        let epoch = {
            let mut tx = client.transaction()?;
            tx.batch_execute("CREATE TABLE IF NOT EXISTS hm_postgres_meta (singleton INTEGER PRIMARY KEY CHECK(singleton=1), identity TEXT NOT NULL, identity_digest TEXT NOT NULL, epoch BIGINT NOT NULL CHECK(epoch>=0)); CREATE TABLE IF NOT EXISTS hm_postgres_migrations(version INTEGER PRIMARY KEY CHECK(version>0), name TEXT NOT NULL, checksum TEXT NOT NULL);")?;
            tx.execute(
                "INSERT INTO hm_postgres_meta VALUES(1,$1,$2,0) ON CONFLICT(singleton) DO NOTHING",
                &[&identity, &digest],
            )?;
            let row=tx.query_one("SELECT identity,identity_digest,epoch FROM hm_postgres_meta WHERE singleton=1 FOR UPDATE",&[])?;
            if row.get::<_, String>(0) != identity || row.get::<_, String>(1) != digest {
                return Err(PostgresError::IdentityMismatch);
            }
            let applied = tx.query(
                "SELECT version,name,checksum FROM hm_postgres_migrations ORDER BY version",
                &[],
            )?;
            let supported = migrations.len() as u32;
            if let Some(row) = applied.last() {
                let store: i32 = row.get(0);
                if store as i64 > supported as i64 {
                    return Err(PostgresError::StoreAhead { store, supported });
                }
            }
            for (index, row) in applied.iter().enumerate() {
                let expected = migrations
                    .get(index)
                    .ok_or(PostgresError::MigrationMismatch)?;
                if row.get::<_, i32>(0) != expected.version as i32
                    || row.get::<_, String>(1) != expected.name
                    || row.get::<_, String>(2) != expected.checksum()
                {
                    return Err(PostgresError::MigrationMismatch);
                }
            }
            for migration in &migrations[applied.len()..] {
                tx.batch_execute(migration.sql)?;
                tx.execute(
                    "INSERT INTO hm_postgres_migrations VALUES($1,$2,$3)",
                    &[
                        &(migration.version as i32),
                        &migration.name,
                        &migration.checksum(),
                    ],
                )?;
            }
            let previous: i64 = row.get(2);
            let epoch = previous
                .checked_add(1)
                .filter(|value| *value > 0)
                .ok_or(PostgresError::EpochExhausted)?;
            tx.execute(
                "UPDATE hm_postgres_meta SET epoch=$1 WHERE singleton=1",
                &[&epoch],
            )?;
            tx.commit()?;
            epoch as u64
        };
        Ok(Self {
            client,
            descriptor: descriptor.clone(),
            epoch,
            lock_key,
            status: WriterStatus::Live,
        })
    }
    pub fn epoch(&self) -> u64 {
        if self.status() == WriterStatus::Live {
            self.epoch
        } else {
            0
        }
    }
    pub fn descriptor(&self) -> &PostgresDescriptor {
        &self.descriptor
    }
    pub fn status(&self) -> WriterStatus {
        if self.client.is_closed() {
            WriterStatus::Lost
        } else {
            self.status
        }
    }
    fn live(&self, expected: u64) -> Result<(), PostgresError> {
        match self.status() {
            WriterStatus::Lost => return Err(PostgresError::ConnectionLost),
            WriterStatus::Stale => return Err(PostgresError::StaleEpoch),
            WriterStatus::Live => {}
        }
        if expected != self.epoch {
            return Err(PostgresError::StaleEpoch);
        }
        Ok(())
    }
    fn finish<T>(&mut self, result: Result<T, PostgresError>) -> Result<T, PostgresError> {
        if self.client.is_closed() {
            self.status = WriterStatus::Lost;
            return Err(PostgresError::ConnectionLost);
        }
        if matches!(
            &result,
            Err(PostgresError::StaleEpoch | PostgresError::IdentityMismatch)
        ) {
            self.status = WriterStatus::Stale;
        }
        result
    }
    pub fn transaction<T>(
        &mut self,
        expected_epoch: u64,
        write: impl FnOnce(&mut Transaction<'_>) -> Result<T, PostgresError>,
    ) -> Result<T, PostgresError> {
        self.live(expected_epoch)?;
        let result = (|| {
            let mut tx = self.client.transaction()?;
            check_fence(
                &mut tx,
                expected_epoch,
                self.lock_key,
                &self.descriptor,
                true,
            )?;
            let value = write(&mut tx)?;
            check_fence(
                &mut tx,
                expected_epoch,
                self.lock_key,
                &self.descriptor,
                true,
            )?;
            tx.commit()?;
            Ok(value)
        })();
        self.finish(result)
    }
    pub fn read<T>(
        &mut self,
        expected_epoch: u64,
        read: impl FnOnce(&mut Transaction<'_>) -> Result<T, PostgresError>,
    ) -> Result<T, PostgresError> {
        self.live(expected_epoch)?;
        let result = (|| {
            let mut tx = self.client.build_transaction().read_only(true).start()?;
            check_fence(
                &mut tx,
                expected_epoch,
                self.lock_key,
                &self.descriptor,
                false,
            )?;
            let value = read(&mut tx)?;
            check_fence(
                &mut tx,
                expected_epoch,
                self.lock_key,
                &self.descriptor,
                false,
            )?;
            tx.commit()?;
            Ok(value)
        })();
        self.finish(result)
    }
}
fn check_fence(
    tx: &mut Transaction<'_>,
    epoch: u64,
    key: i64,
    descriptor: &PostgresDescriptor,
    writer: bool,
) -> Result<(), PostgresError> {
    let upper = ((key as u64) >> 32) as i64;
    let lower = (key as u64 & 0xffff_ffff) as i64;
    let held:bool=tx.query_one("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE locktype='advisory' AND pid=pg_backend_pid() AND classid=$1::bigint::oid AND objid=$2::bigint::oid AND objsubid=1 AND granted)",&[&upper,&lower])?.get(0);
    if !held {
        return Err(PostgresError::StaleEpoch);
    }
    let sql = if writer {
        "SELECT epoch,identity,identity_digest FROM hm_postgres_meta WHERE singleton=1 FOR UPDATE"
    } else {
        "SELECT epoch,identity,identity_digest FROM hm_postgres_meta WHERE singleton=1"
    };
    let row = tx.query_opt(sql, &[])?.ok_or(PostgresError::StaleEpoch)?;
    if row.get::<_, i64>(0) as u64 != epoch {
        return Err(PostgresError::StaleEpoch);
    }
    if row.get::<_, String>(1) != descriptor.identity()?
        || row.get::<_, String>(2) != descriptor.identity_digest()?
    {
        return Err(PostgresError::IdentityMismatch);
    }
    Ok(())
}
