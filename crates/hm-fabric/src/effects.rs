use fs2::FileExt;
use hm_context::types::{ContextError, Cursor, Scope, digest_bytes, validate_id};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
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

struct SqliteEffectStore {
    connection: Connection,
    _lock: File,
}
fn db(error: rusqlite::Error) -> ContextError {
    ContextError::Unavailable(error.to_string())
}
impl SqliteEffectStore {
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

pub struct EffectStore {
    backend: EffectBackend,
}
enum EffectBackend {
    Sqlite(SqliteEffectStore),
    Selected(SelectedEffectStore),
}
impl EffectStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ContextError> {
        Ok(Self {
            backend: EffectBackend::Sqlite(SqliteEffectStore::open(path)?),
        })
    }
    pub fn from_backend(
        scope: Scope,
        namespace: String,
        records: crate::backend_runtime::RuntimeRecordStore,
    ) -> Result<Self, ContextError> {
        Ok(Self {
            backend: EffectBackend::Selected(SelectedEffectStore::open(scope, namespace, records)?),
        })
    }
    pub fn prepare(
        &mut self,
        scope: &Scope,
        key: &str,
        kind: &str,
        payload: &[u8],
    ) -> Result<EffectIntent, ContextError> {
        match &mut self.backend {
            EffectBackend::Sqlite(s) => s.prepare(scope, key, kind, payload),
            EffectBackend::Selected(s) => s.prepare(scope, key, kind, payload),
        }
    }
    pub fn get(&self, scope: &Scope, key: &str) -> Result<Option<EffectIntent>, ContextError> {
        match &self.backend {
            EffectBackend::Sqlite(s) => s.get(scope, key),
            EffectBackend::Selected(s) => s.get(scope, key),
        }
    }
    pub fn begin_dispatch(
        &mut self,
        scope: &Scope,
        key: &str,
        version: u64,
    ) -> Result<EffectIntent, ContextError> {
        match &mut self.backend {
            EffectBackend::Sqlite(s) => s.begin_dispatch(scope, key, version),
            EffectBackend::Selected(s) => s.transition(
                scope,
                key,
                version,
                EffectState::Prepared,
                EffectState::Dispatched,
                None,
            ),
        }
    }
    pub fn mark_uncertain(
        &mut self,
        scope: &Scope,
        key: &str,
        version: u64,
    ) -> Result<EffectIntent, ContextError> {
        match &mut self.backend {
            EffectBackend::Sqlite(s) => s.mark_uncertain(scope, key, version),
            EffectBackend::Selected(s) => s.transition(
                scope,
                key,
                version,
                EffectState::Dispatched,
                EffectState::Uncertain,
                None,
            ),
        }
    }
    pub fn complete(
        &mut self,
        scope: &Scope,
        key: &str,
        version: u64,
        observation: EffectObservation,
    ) -> Result<EffectIntent, ContextError> {
        match &mut self.backend {
            EffectBackend::Sqlite(s) => s.complete(scope, key, version, observation),
            EffectBackend::Selected(s) => s.transition(
                scope,
                key,
                version,
                EffectState::Dispatched,
                EffectState::Terminal,
                Some(observation),
            ),
        }
    }
    pub fn reconcile(
        &mut self,
        scope: &Scope,
        key: &str,
        version: u64,
        observation: EffectObservation,
    ) -> Result<EffectIntent, ContextError> {
        match &mut self.backend {
            EffectBackend::Sqlite(s) => s.reconcile(scope, key, version, observation),
            EffectBackend::Selected(s) => s.transition(
                scope,
                key,
                version,
                EffectState::Uncertain,
                EffectState::Terminal,
                Some(observation),
            ),
        }
    }
    pub fn receipts(
        &self,
        scope: &Scope,
        after: Cursor,
        limit: usize,
    ) -> Result<Vec<EffectReceipt>, ContextError> {
        match &self.backend {
            EffectBackend::Sqlite(s) => s.receipts(scope, after, limit),
            EffectBackend::Selected(s) => s.receipts(scope, after, limit),
        }
    }
    pub fn acknowledged(&self, scope: &Scope, consumer: &str) -> Result<Cursor, ContextError> {
        match &self.backend {
            EffectBackend::Sqlite(s) => s.acknowledged(scope, consumer),
            EffectBackend::Selected(s) => s.acknowledged(scope, consumer),
        }
    }
    pub fn acknowledge(
        &mut self,
        scope: &Scope,
        consumer: &str,
        cursor: Cursor,
    ) -> Result<(), ContextError> {
        match &mut self.backend {
            EffectBackend::Sqlite(s) => s.acknowledge(scope, consumer, cursor),
            EffectBackend::Selected(s) => s.acknowledge(scope, consumer, cursor),
        }
    }
}
#[derive(Serialize, Deserialize)]
struct EffectIndex {
    scope: Scope,
    namespace: String,
    sequence: u64,
    intents: std::collections::BTreeSet<String>,
}
#[derive(Serialize, Deserialize)]
struct EffectAck {
    scope: Scope,
    consumer: String,
    cursor: Cursor,
}
struct SelectedEffectStore {
    scope: Scope,
    namespace: String,
    prefix: String,
    records: crate::backend_runtime::RuntimeRecordStore,
}
fn backend_error(error: crate::backend_runtime::BackendRuntimeError) -> ContextError {
    match error {
        crate::backend_runtime::BackendRuntimeError::Conflict => ContextError::Conflict,
        other => ContextError::Unavailable(other.to_string()),
    }
}
impl SelectedEffectStore {
    fn open(
        scope: Scope,
        namespace: String,
        records: crate::backend_runtime::RuntimeRecordStore,
    ) -> Result<Self, ContextError> {
        scope.validate()?;
        if namespace.is_empty() || namespace.len() > 4096 || namespace.chars().any(char::is_control)
        {
            return Err(ContextError::Invalid("effect namespace required".into()));
        }
        let prefix = format!(
            "effects.{}",
            digest_bytes(&serde_json::to_vec(&(&scope, &namespace))?)
        );
        let mut store = Self {
            scope,
            namespace,
            prefix,
            records,
        };
        if store
            .records
            .get(&store.index_key())
            .map_err(backend_error)?
            .is_none()
        {
            let index = EffectIndex {
                scope: store.scope.clone(),
                namespace: store.namespace.clone(),
                sequence: 0,
                intents: Default::default(),
            };
            store
                .records
                .compare_exchange(crate::backend_runtime::RecordChange {
                    key: store.index_key(),
                    expected: None,
                    value: serde_json::to_value(index)?,
                })
                .map_err(backend_error)?;
        }
        let (_, index) = store.index()?;
        for key in index.intents {
            let intent = store
                .get(&store.scope, &key)?
                .ok_or_else(|| ContextError::Unavailable("missing indexed effect".into()))?;
            if intent.state == EffectState::Dispatched {
                let scope = store.scope.clone();
                store.transition(
                    &scope,
                    &key,
                    intent.version,
                    EffectState::Dispatched,
                    EffectState::Uncertain,
                    None,
                )?;
            }
        }
        Ok(store)
    }
    fn check_scope(&self, scope: &Scope) -> Result<(), ContextError> {
        scope.validate()?;
        if scope != &self.scope {
            return Err(ContextError::ScopeMismatch);
        }
        Ok(())
    }
    fn index_key(&self) -> String {
        format!("{}.index", self.prefix)
    }
    fn intent_key(&self, key: &str) -> String {
        format!("{}.intent.{}", self.prefix, digest_bytes(key.as_bytes()))
    }
    fn receipt_key(&self, sequence: u64) -> String {
        format!("{}.receipt.{sequence:020}", self.prefix)
    }
    fn ack_key(&self, consumer: &str) -> String {
        format!("{}.ack.{}", self.prefix, digest_bytes(consumer.as_bytes()))
    }
    fn index(&self) -> Result<(u64, EffectIndex), ContextError> {
        let record = self
            .records
            .get(&self.index_key())
            .map_err(backend_error)?
            .ok_or_else(|| ContextError::Unavailable("effect index missing".into()))?;
        let index: EffectIndex = serde_json::from_value(record.value)?;
        if index.scope != self.scope || index.namespace != self.namespace {
            return Err(ContextError::ScopeMismatch);
        }
        Ok((record.revision, index))
    }
    fn read_intent(
        &self,
        scope: &Scope,
        key: &str,
    ) -> Result<Option<(u64, EffectIntent)>, ContextError> {
        self.check_scope(scope)?;
        validate_id(key)?;
        self.index()?;
        self.records
            .get(&self.intent_key(key))
            .map_err(backend_error)?
            .map(|record| {
                let intent: EffectIntent = serde_json::from_value(record.value)?;
                if intent.scope != *scope || intent.key != key {
                    return Err(ContextError::ScopeMismatch);
                }
                if intent.payload_digest != digest_bytes(&intent.payload) {
                    return Err(ContextError::Unavailable("effect payload corrupt".into()));
                }
                Ok((record.revision, intent))
            })
            .transpose()
    }
    fn get(&self, scope: &Scope, key: &str) -> Result<Option<EffectIntent>, ContextError> {
        Ok(self.read_intent(scope, key)?.map(|(_, intent)| intent))
    }
    fn persist(
        &mut self,
        intent: &EffectIntent,
        expected: Option<u64>,
    ) -> Result<(), ContextError> {
        let (revision, mut index) = self.index()?;
        index.sequence = index
            .sequence
            .checked_add(1)
            .ok_or(ContextError::Capacity)?;
        index.intents.insert(intent.key.clone());
        let receipt = EffectReceipt {
            contract_version: 1,
            cursor: Cursor {
                epoch: 1,
                sequence: index.sequence,
            },
            intent: intent.clone(),
        };
        self.records
            .compare_exchange_batch(vec![
                crate::backend_runtime::RecordChange {
                    key: self.index_key(),
                    expected: Some(revision),
                    value: serde_json::to_value(index)?,
                },
                crate::backend_runtime::RecordChange {
                    key: self.intent_key(&intent.key),
                    expected,
                    value: serde_json::to_value(intent)?,
                },
                crate::backend_runtime::RecordChange {
                    key: self.receipt_key(receipt.cursor.sequence),
                    expected: None,
                    value: serde_json::to_value(receipt)?,
                },
            ])
            .map_err(backend_error)?;
        Ok(())
    }
    fn prepare(
        &mut self,
        scope: &Scope,
        key: &str,
        kind: &str,
        payload: &[u8],
    ) -> Result<EffectIntent, ContextError> {
        self.check_scope(scope)?;
        validate_id(key)?;
        validate_id(kind)?;
        if payload.len() > 16 * 1024 * 1024 {
            return Err(ContextError::Capacity);
        }
        if let Some(intent) = self.get(scope, key)? {
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
        self.persist(&intent, None)?;
        Ok(intent)
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
        if observation
            .as_ref()
            .is_some_and(|o| o.evidence.trim().is_empty() || o.evidence.len() > 1024 * 1024)
        {
            return Err(ContextError::Invalid("observed evidence required".into()));
        }
        let (revision, mut intent) = self
            .read_intent(scope, key)?
            .ok_or_else(|| ContextError::Invalid("unknown effect".into()))?;
        if intent.version != version {
            return Err(ContextError::Stale);
        }
        if intent.state != from {
            return Err(ContextError::Conflict);
        }
        intent.version = intent
            .version
            .checked_add(1)
            .ok_or(ContextError::Capacity)?;
        intent.state = to;
        intent.observation = observation;
        self.persist(&intent, Some(revision))?;
        Ok(intent)
    }
    fn receipts(
        &self,
        scope: &Scope,
        after: Cursor,
        limit: usize,
    ) -> Result<Vec<EffectReceipt>, ContextError> {
        self.check_scope(scope)?;
        after.validate()?;
        if after.epoch != 1 {
            return Err(ContextError::Stale);
        }
        if limit == 0 || limit > 4096 {
            return Err(ContextError::Capacity);
        }
        let (_, index) = self.index()?;
        let mut receipts = Vec::new();
        if after.sequence >= index.sequence {
            return Ok(receipts);
        }
        for sequence in (after.sequence + 1)..=index.sequence {
            let record = self
                .records
                .get(&self.receipt_key(sequence))
                .map_err(backend_error)?
                .ok_or_else(|| ContextError::Unavailable("effect receipt missing".into()))?;
            let receipt: EffectReceipt = serde_json::from_value(record.value)?;
            if receipt.contract_version != 1
                || receipt.cursor != (Cursor { epoch: 1, sequence })
                || receipt.intent.scope != *scope
            {
                return Err(ContextError::ScopeMismatch);
            }
            receipts.push(receipt);
            if receipts.len() == limit {
                break;
            }
        }
        Ok(receipts)
    }
    fn read_ack(
        &self,
        scope: &Scope,
        consumer: &str,
    ) -> Result<Option<(u64, EffectAck)>, ContextError> {
        self.check_scope(scope)?;
        validate_id(consumer)?;
        self.index()?;
        self.records
            .get(&self.ack_key(consumer))
            .map_err(backend_error)?
            .map(|record| {
                let ack: EffectAck = serde_json::from_value(record.value)?;
                if ack.scope != *scope || ack.consumer != consumer || ack.cursor.epoch != 1 {
                    return Err(ContextError::ScopeMismatch);
                }
                Ok((record.revision, ack))
            })
            .transpose()
    }
    fn acknowledged(&self, scope: &Scope, consumer: &str) -> Result<Cursor, ContextError> {
        Ok(self.read_ack(scope, consumer)?.map_or(
            Cursor {
                epoch: 1,
                sequence: 0,
            },
            |(_, ack)| ack.cursor,
        ))
    }
    fn acknowledge(
        &mut self,
        scope: &Scope,
        consumer: &str,
        cursor: Cursor,
    ) -> Result<(), ContextError> {
        cursor.validate()?;
        if cursor.epoch != 1 {
            return Err(ContextError::Stale);
        }
        let old = self.read_ack(scope, consumer)?;
        if cursor.sequence < old.as_ref().map_or(0, |(_, ack)| ack.cursor.sequence) {
            return Err(ContextError::Stale);
        }
        if cursor.sequence != 0 {
            let receipt = self.receipts(
                scope,
                Cursor {
                    epoch: 1,
                    sequence: cursor.sequence - 1,
                },
                1,
            )?;
            if receipt.first().map(|r| r.cursor) != Some(cursor) {
                return Err(ContextError::ScopeMismatch);
            }
        }
        self.records
            .compare_exchange(crate::backend_runtime::RecordChange {
                key: self.ack_key(consumer),
                expected: old.map(|(revision, _)| revision),
                value: serde_json::to_value(EffectAck {
                    scope: scope.clone(),
                    consumer: consumer.into(),
                    cursor,
                })?,
            })
            .map_err(backend_error)?;
        Ok(())
    }
}
