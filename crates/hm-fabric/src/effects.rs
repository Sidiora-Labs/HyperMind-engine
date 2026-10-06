use fs2::FileExt;
use hm_context::types::{digest_bytes, validate_id, ContextError, Cursor, Scope};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    path::Path,
    time::Duration,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectState {
    Prepared,
    Dispatched,
    Uncertain,
    Terminal,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectOutcome {
    Succeeded,
    Failed,
    NotApplied,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EffectObservation {
    pub outcome: EffectOutcome,
    pub evidence: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EffectIntent {
    pub scope: Scope,
    pub key: String,
    pub kind: String,
    pub payload: Vec<u8>,
    pub payload_digest: String,
    pub state: EffectState,
    pub version: u64,
    pub observation: Option<EffectObservation>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EffectReceipt {
    pub contract_version: u32,
    pub cursor: Cursor,
    pub intent: EffectIntent,
}

pub struct EffectStore {
    connection: Connection,
    _lock: File,
}
fn db(error: rusqlite::Error) -> ContextError {
    ContextError::Unavailable(error.to_string())
}
impl EffectStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ContextError> {
        let path = path.as_ref();
        if path.as_os_str().is_empty() || path == Path::new(":memory:") {
            return Err(ContextError::Invalid("file storage required".into()));
        }
        let mut lock_path = path.as_os_str().to_os_string();
        lock_path.push(".effects.lock");
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)?;
        lock.try_lock_exclusive()
            .map_err(|_| ContextError::Conflict)?;
        let mut connection = Connection::open(path).map_err(db)?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(db)?;
        connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
          CREATE TABLE IF NOT EXISTS effect_intents(scope TEXT NOT NULL, key TEXT NOT NULL, data TEXT NOT NULL, PRIMARY KEY(scope,key));
          CREATE TABLE IF NOT EXISTS effect_receipts(sequence INTEGER PRIMARY KEY AUTOINCREMENT, scope TEXT NOT NULL, data TEXT NOT NULL);
          CREATE TABLE IF NOT EXISTS effect_acks(scope TEXT NOT NULL, consumer TEXT NOT NULL, sequence INTEGER NOT NULL, PRIMARY KEY(scope,consumer));").map_err(db)?;
        let tx = connection.transaction().map_err(db)?;
        let recovered = {
            let mut statement = tx.prepare("SELECT data FROM effect_intents").map_err(db)?;
            let data = statement
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(db)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(db)?;
            data.into_iter()
                .map(|data| serde_json::from_str::<EffectIntent>(&data))
                .collect::<Result<Vec<_>, _>>()?
        };
        for mut intent in recovered {
            if intent.state == EffectState::Dispatched {
                intent.state = EffectState::Uncertain;
                intent.version += 1;
                persist(&tx, &intent)?;
            }
        }
        tx.commit().map_err(db)?;
        Ok(Self {
            connection,
            _lock: lock,
        })
    }
    pub fn prepare(
        &mut self,
        scope: &Scope,
        key: &str,
        kind: &str,
        payload: &[u8],
    ) -> Result<EffectIntent, ContextError> {
        scope.validate()?;
        validate_id(key)?;
        validate_id(kind)?;
        if payload.len() > 16 * 1024 * 1024 {
            return Err(ContextError::Capacity);
        }
        let tx = self.connection.transaction().map_err(db)?;
        if let Some(intent) = read(&tx, scope, key)? {
            if intent.kind != kind || intent.payload != payload {
                return Err(ContextError::Conflict);
            }
            return Ok(intent);
        }
        let intent = EffectIntent {
            scope: scope.clone(),
            key: key.into(),
            kind: kind.into(),
            payload: payload.into(),
            payload_digest: digest_bytes(payload),
            state: EffectState::Prepared,
            version: 1,
            observation: None,
        };
        persist(&tx, &intent)?;
        tx.commit().map_err(db)?;
        Ok(intent)
    }
    pub fn get(&self, scope: &Scope, key: &str) -> Result<Option<EffectIntent>, ContextError> {
        scope.validate()?;
        validate_id(key)?;
        read(&self.connection, scope, key)
    }
    pub fn begin_dispatch(
        &mut self,
        scope: &Scope,
        key: &str,
        version: u64,
    ) -> Result<EffectIntent, ContextError> {
        self.transition(
            scope,
            key,
            version,
            EffectState::Prepared,
            EffectState::Dispatched,
            None,
        )
    }
    pub fn mark_uncertain(
        &mut self,
        scope: &Scope,
        key: &str,
        version: u64,
    ) -> Result<EffectIntent, ContextError> {
        self.transition(
            scope,
            key,
            version,
            EffectState::Dispatched,
            EffectState::Uncertain,
            None,
        )
    }
    pub fn complete(
        &mut self,
        scope: &Scope,
        key: &str,
        version: u64,
        observation: EffectObservation,
    ) -> Result<EffectIntent, ContextError> {
        self.transition(
            scope,
            key,
            version,
            EffectState::Dispatched,
            EffectState::Terminal,
            Some(observation),
        )
    }
    pub fn reconcile(
        &mut self,
        scope: &Scope,
        key: &str,
        version: u64,
        observation: EffectObservation,
    ) -> Result<EffectIntent, ContextError> {
        self.transition(
            scope,
            key,
            version,
            EffectState::Uncertain,
            EffectState::Terminal,
            Some(observation),
        )
    }
    fn transition(
        &mut self,
        scope: &Scope,
        key: &str,
        version: u64,
        from: EffectState,
        to: EffectState,
        observation: Option<EffectObservation>,
    ) -> Result<EffectIntent, ContextError> {
        scope.validate()?;
        validate_id(key)?;
        if observation.as_ref().is_some_and(|value| {
            value.evidence.trim().is_empty() || value.evidence.len() > 1024 * 1024
        }) {
            return Err(ContextError::Invalid("observed evidence required".into()));
        }
        let tx = self.connection.transaction().map_err(db)?;
        let mut intent =
            read(&tx, scope, key)?.ok_or_else(|| ContextError::Invalid("unknown effect".into()))?;
        if intent.version != version {
            return Err(ContextError::Stale);
        }
        if intent.state != from {
            return Err(ContextError::Conflict);
        }
        intent.state = to;
        intent.version = intent
            .version
            .checked_add(1)
            .ok_or(ContextError::Capacity)?;
        intent.observation = observation;
        persist(&tx, &intent)?;
        tx.commit().map_err(db)?;
        Ok(intent)
    }
    pub fn receipts(
        &self,
        scope: &Scope,
        after: Cursor,
        limit: usize,
    ) -> Result<Vec<EffectReceipt>, ContextError> {
        scope.validate()?;
        after.validate()?;
        if after.epoch != 1 {
            return Err(ContextError::Stale);
        }
        if limit == 0 || limit > 4096 {
            return Err(ContextError::Capacity);
        }
        let mut statement = self.connection.prepare("SELECT sequence,data FROM effect_receipts WHERE scope=?1 AND sequence>?2 ORDER BY sequence LIMIT ?3").map_err(db)?;
        let rows = statement
            .query_map(
                params![scope.digest()?, after.sequence as i64, limit as i64],
                |row| Ok((row.get::<_, i64>(0)? as u64, row.get::<_, String>(1)?)),
            )
            .map_err(db)?;
        rows.map(|row| {
            let (sequence, data) = row.map_err(db)?;
            Ok(EffectReceipt {
                contract_version: 1,
                cursor: Cursor { epoch: 1, sequence },
                intent: serde_json::from_str(&data)?,
            })
        })
        .collect()
    }
    pub fn acknowledged(&self, scope: &Scope, consumer: &str) -> Result<Cursor, ContextError> {
        scope.validate()?;
        validate_id(consumer)?;
        let sequence: i64 = self
            .connection
            .query_row(
                "SELECT sequence FROM effect_acks WHERE scope=?1 AND consumer=?2",
                params![scope.digest()?, consumer],
                |row| row.get(0),
            )
            .optional()
            .map_err(db)?
            .unwrap_or(0);
        Ok(Cursor {
            epoch: 1,
            sequence: sequence as u64,
        })
    }
    pub fn acknowledge(
        &mut self,
        scope: &Scope,
        consumer: &str,
        cursor: Cursor,
    ) -> Result<(), ContextError> {
        scope.validate()?;
        validate_id(consumer)?;
        cursor.validate()?;
        if cursor.epoch != 1 {
            return Err(ContextError::Stale);
        }
        let tx = self.connection.transaction().map_err(db)?;
        let scope_key = scope.digest()?;
        let exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM effect_receipts WHERE scope=?1 AND sequence=?2)",
                params![scope_key, cursor.sequence as i64],
                |row| row.get(0),
            )
            .map_err(db)?;
        if !exists && cursor.sequence != 0 {
            return Err(ContextError::ScopeMismatch);
        }
        let old: i64 = tx
            .query_row(
                "SELECT sequence FROM effect_acks WHERE scope=?1 AND consumer=?2",
                params![scope_key, consumer],
                |row| row.get(0),
            )
            .optional()
            .map_err(db)?
            .unwrap_or(0);
        if cursor.sequence < old as u64 {
            return Err(ContextError::Stale);
        }
        tx.execute("INSERT INTO effect_acks VALUES(?1,?2,?3) ON CONFLICT(scope,consumer) DO UPDATE SET sequence=excluded.sequence",params![scope_key,consumer,cursor.sequence as i64]).map_err(db)?;
        tx.commit().map_err(db)?;
        Ok(())
    }
}
fn read(
    connection: &Connection,
    scope: &Scope,
    key: &str,
) -> Result<Option<EffectIntent>, ContextError> {
    let data: Option<String> = connection
        .query_row(
            "SELECT data FROM effect_intents WHERE scope=?1 AND key=?2",
            params![scope.digest()?, key],
            |row| row.get(0),
        )
        .optional()
        .map_err(db)?;
    data.map(|data| serde_json::from_str(&data).map_err(ContextError::from))
        .transpose()
}
fn persist(tx: &Transaction<'_>, intent: &EffectIntent) -> Result<(), ContextError> {
    let scope = intent.scope.digest()?;
    let data = serde_json::to_string(intent)?;
    tx.execute("INSERT INTO effect_intents VALUES(?1,?2,?3) ON CONFLICT(scope,key) DO UPDATE SET data=excluded.data",params![scope,intent.key,data]).map_err(db)?;
    tx.execute(
        "INSERT INTO effect_receipts(scope,data) VALUES(?1,?2)",
        params![scope, data],
    )
    .map_err(db)?;
    Ok(())
}
