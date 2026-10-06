use crate::backend_config::{
    BackendConfigError, BackendReadiness, SelectedBackends, SelectedBus, SelectedOperationalStore,
};
use crate::bus::{Event, Grant};
use crate::bus_nats::{NatsBus, NatsReplay};
use crate::effects::EffectReceipt;
use crate::runtime::WorkerResult;
use crate::storage::Migration;
use hm_context::types::{Scope, digest_bytes, validate_id};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
};

pub const RUNTIME_BACKEND_MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "selected_runtime_records",
    sql: "CREATE TABLE runtime_backend_records(key TEXT PRIMARY KEY, revision BIGINT NOT NULL, body TEXT NOT NULL, digest TEXT NOT NULL);",
}];
const PRINCIPAL: &str = "runtime";
const STREAM: &str = "runtime_receipts";
#[derive(Debug, thiserror::Error)]
pub enum BackendRuntimeError {
    #[error(transparent)]
    Config(#[from] BackendConfigError),
    #[error("operational record conflict")]
    Conflict,
    #[error("operational record corrupt")]
    Corrupt,
    #[error("selected backend owner stopped")]
    Stopped,
    #[error("publication {0} is uncertain; automatic replay refused")]
    Uncertain(String),
    #[error("serialization: {0}")]
    Json(#[from] serde_json::Error),
    #[error("bus: {0}")]
    Context(#[from] hm_context::ContextError),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OperationalRecord {
    pub key: String,
    pub revision: u64,
    pub value: Value,
    pub digest: String,
}
#[derive(Clone, Debug)]
pub struct RecordChange {
    pub key: String,
    pub expected: Option<u64>,
    pub value: Value,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicationState {
    Pending,
    Dispatched,
    Uncertain,
    Published,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimePublication {
    pub id: String,
    pub digest: String,
    pub receipt: EffectReceipt,
    pub state: PublicationState,
    pub event_sequence: Option<u64>,
}

type Job = Box<dyn FnOnce(&mut SelectedBackends) + Send>;
enum Command {
    Run(Job),
    Stop,
}
#[derive(Clone)]
pub struct RuntimeRecordStore {
    sender: mpsc::Sender<Command>,
}
impl RuntimeRecordStore {
    fn ask<T: Send + 'static>(
        &self,
        work: impl FnOnce(&mut SelectedBackends) -> Result<T, BackendRuntimeError> + Send + 'static,
    ) -> Result<T, BackendRuntimeError> {
        let (send, receive) = mpsc::sync_channel(1);
        self.sender
            .send(Command::Run(Box::new(move |selected| {
                let result = work(selected);
                let _ = send.send(result);
            })))
            .map_err(|_| BackendRuntimeError::Stopped)?;
        receive.recv().map_err(|_| BackendRuntimeError::Stopped)?
    }
    pub fn get(&self, key: &str) -> Result<Option<OperationalRecord>, BackendRuntimeError> {
        validate_id(key)?;
        let key = key.to_owned();
        self.ask(move |selected| get_record(selected, &key))
    }
    pub fn compare_exchange(
        &self,
        change: RecordChange,
    ) -> Result<OperationalRecord, BackendRuntimeError> {
        Ok(self.compare_exchange_batch(vec![change])?.remove(0))
    }
    pub fn compare_exchange_batch(
        &self,
        changes: Vec<RecordChange>,
    ) -> Result<Vec<OperationalRecord>, BackendRuntimeError> {
        if changes.is_empty() || changes.len() > 128 {
            return Err(BackendRuntimeError::Conflict);
        }
        let mut keys = std::collections::BTreeSet::new();
        for change in &changes {
            validate_id(&change.key)?;
            if !keys.insert(&change.key)
                || change
                    .expected
                    .is_some_and(|revision| revision == 0 || revision >= i64::MAX as u64)
            {
                return Err(BackendRuntimeError::Conflict);
            }
        }
        self.ask(move |selected| cas_records(selected, changes))
    }
    fn list(
        &self,
        prefix: &str,
        limit: u32,
    ) -> Result<Vec<OperationalRecord>, BackendRuntimeError> {
        if limit == 0 || limit > 1024 {
            return Err(BackendRuntimeError::Conflict);
        }
        let prefix = format!("{prefix}%");
        self.ask(move |selected| list_records(selected, &prefix, limit))
    }
}
pub struct BackendRuntime {
    records: RuntimeRecordStore,
    thread: Option<thread::JoinHandle<()>>,
    remote: Option<NatsBus>,
    home: PathBuf,
    scope: Scope,
    owner_epoch: u64,
    identity_digest: String,
    outbox_prefix: String,
}
impl BackendRuntime {
    pub fn from_selected(mut selected: SelectedBackends) -> Result<Self, BackendRuntimeError> {
        let home = selected.home().to_owned();
        let scope = selected.descriptor().scope.clone();
        let owner_epoch = selected.owner.epoch();
        let identity_digest = selected.identity_digest().to_owned();
        let outbox_prefix = format!("runtime.outbox.{identity_digest}.");
        let remote = match &selected.bus {
            SelectedBus::Nats(bus) => Some(bus.clone()),
            _ => None,
        };
        let (sender, receiver) = mpsc::channel();
        let (ready_send, ready_receive) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name("hypermind-operational-owner".into())
            .spawn(move || {
                let ready = (|| {
                    selected.readiness()?;
                    match &mut selected.bus {
                        SelectedBus::Sqlite(bus) => bus.grant(Grant {
                            principal: PRINCIPAL.into(),
                            stream: STREAM.into(),
                            publish: true,
                            subscribe: true,
                            register: false,
                        })?,
                        SelectedBus::ProcessLocal(bus) => bus.grant(Grant {
                            principal: PRINCIPAL.into(),
                            stream: STREAM.into(),
                            publish: true,
                            subscribe: true,
                            register: false,
                        })?,
                        SelectedBus::Nats(_) => {}
                    }
                    get_record(&mut selected, "runtime.schema.probe")?;
                    Ok::<(), BackendRuntimeError>(())
                })();
                let succeeded = ready.is_ok();
                let _ = ready_send.send(ready);
                if !succeeded {
                    return;
                }
                while let Ok(command) = receiver.recv() {
                    match command {
                        Command::Run(work) => work(&mut selected),
                        Command::Stop => break,
                    }
                }
            })
            .map_err(|_| BackendRuntimeError::Stopped)?;
        ready_receive
            .recv()
            .map_err(|_| BackendRuntimeError::Stopped)??;
        let backend = Self {
            records: RuntimeRecordStore { sender },
            thread: Some(worker),
            remote,
            home,
            scope,
            owner_epoch,
            identity_digest,
            outbox_prefix,
        };
        for record in backend.records.list(&backend.outbox_prefix, 1024)? {
            let mut publication: RuntimePublication = serde_json::from_value(record.value)?;
            if publication.state == PublicationState::Dispatched {
                publication.state = PublicationState::Uncertain;
                backend.update_publication(record.key, record.revision, publication)?;
            }
        }
        Ok(backend)
    }
    pub fn home(&self) -> &Path {
        &self.home
    }
    pub fn scope(&self) -> &Scope {
        &self.scope
    }
    pub fn owner_epoch(&self) -> u64 {
        self.owner_epoch
    }
    pub fn record_store(&self) -> RuntimeRecordStore {
        self.records.clone()
    }
    pub fn readiness(&self) -> Result<BackendReadiness, BackendRuntimeError> {
        self.records.ask(|selected| Ok(selected.readiness()?))
    }
    pub fn fence(&self) -> Result<(), BackendRuntimeError> {
        self.readiness()?;
        Ok(())
    }
    pub fn put_result(&self, result: &WorkerResult) -> Result<(), BackendRuntimeError> {
        let key = format!(
            "runtime.result.{}.{}",
            self.identity_digest,
            digest_bytes(result.id.as_bytes())
        );
        let value = serde_json::to_value(result)?;
        if let Some(old) = self.records.get(&key)? {
            if old.value != value {
                return Err(BackendRuntimeError::Conflict);
            }
            return Ok(());
        }
        self.records.compare_exchange(RecordChange {
            key,
            expected: None,
            value,
        })?;
        Ok(())
    }
    pub fn result(&self, id: &str) -> Result<Option<WorkerResult>, BackendRuntimeError> {
        validate_id(id)?;
        self.records
            .get(&format!(
                "runtime.result.{}.{}",
                self.identity_digest,
                digest_bytes(id.as_bytes())
            ))?
            .map(|record| serde_json::from_value(record.value).map_err(BackendRuntimeError::from))
            .transpose()
    }
    pub fn enqueue_receipt(&self, receipt: &EffectReceipt) -> Result<(), BackendRuntimeError> {
        if receipt.intent.scope != self.scope {
            return Err(hm_context::ContextError::ScopeMismatch.into());
        }
        let key = format!(
            "{}{:020}.{:020}",
            self.outbox_prefix, receipt.cursor.epoch, receipt.cursor.sequence
        );
        if let Some(old) = self.records.get(&key)? {
            let previous: RuntimePublication = serde_json::from_value(old.value)?;
            if previous.receipt != *receipt {
                return Err(BackendRuntimeError::Conflict);
            }
            return Ok(());
        }
        let digest = digest_bytes(&serde_json::to_vec(receipt)?);
        let id = digest_bytes(
            format!(
                "{}:{}:{}",
                self.identity_digest, receipt.cursor.epoch, receipt.cursor.sequence
            )
            .as_bytes(),
        );
        let publication = RuntimePublication {
            id,
            digest,
            receipt: receipt.clone(),
            state: PublicationState::Pending,
            event_sequence: None,
        };
        self.records.compare_exchange(RecordChange {
            key,
            expected: None,
            value: serde_json::to_value(publication)?,
        })?;
        Ok(())
    }
    fn update_publication(
        &self,
        key: String,
        revision: u64,
        publication: RuntimePublication,
    ) -> Result<(), BackendRuntimeError> {
        self.records.compare_exchange(RecordChange {
            key,
            expected: Some(revision),
            value: serde_json::to_value(publication)?,
        })?;
        Ok(())
    }
    pub fn publications(&self, limit: u32) -> Result<Vec<RuntimePublication>, BackendRuntimeError> {
        self.records
            .list(&self.outbox_prefix, limit)?
            .into_iter()
            .map(|record| serde_json::from_value(record.value).map_err(BackendRuntimeError::from))
            .collect()
    }
    pub async fn flush_events(&self) -> Result<usize, BackendRuntimeError> {
        self.fence()?;
        let mut count = 0;
        for record in self.records.list(&self.outbox_prefix, 1024)? {
            let mut publication: RuntimePublication = serde_json::from_value(record.value)?;
            match publication.state {
                PublicationState::Published => continue,
                PublicationState::Dispatched | PublicationState::Uncertain => {
                    return Err(BackendRuntimeError::Uncertain(publication.id));
                }
                PublicationState::Pending => {}
            }
            publication.state = PublicationState::Dispatched;
            self.update_publication(record.key.clone(), record.revision, publication.clone())?;
            let dispatch_revision = record.revision + 1;
            let event = if let Some(bus) = &self.remote {
                bus.append_once(&publication.id, publication.digest.as_bytes())
                    .await
                    .map_err(BackendRuntimeError::from)
            } else {
                let id = publication.id.clone();
                let digest = publication.digest.clone();
                self.records.ask(move |selected| {
                    selected.readiness()?;
                    Ok(match &mut selected.bus {
                        SelectedBus::Sqlite(bus) => {
                            bus.append_once(PRINCIPAL, STREAM, &id, digest.as_bytes())?
                        }
                        SelectedBus::ProcessLocal(bus) => {
                            bus.append_once(PRINCIPAL, STREAM, &id, digest.as_bytes())?
                        }
                        SelectedBus::Nats(_) => return Err(BackendRuntimeError::Stopped),
                    })
                })
            };
            match event {
                Ok(event) => {
                    publication.state = PublicationState::Published;
                    publication.event_sequence = Some(event.sequence);
                    self.update_publication(record.key, dispatch_revision, publication)?;
                    count += 1;
                }
                Err(error) => {
                    publication.state = PublicationState::Uncertain;
                    self.update_publication(record.key, dispatch_revision, publication)?;
                    return Err(error);
                }
            }
        }
        Ok(count)
    }
    pub async fn replay_events(
        &self,
        after: u64,
        limit: u32,
    ) -> Result<NatsReplay, BackendRuntimeError> {
        self.fence()?;
        if let Some(bus) = &self.remote {
            return Ok(bus.replay(after, limit).await?);
        }
        self.records.ask(move |selected| {
            selected.readiness()?;
            let events = match &selected.bus {
                SelectedBus::Sqlite(bus) => bus.replay(PRINCIPAL, STREAM, after, limit)?,
                SelectedBus::ProcessLocal(bus) => bus.replay(PRINCIPAL, STREAM, after, limit)?,
                SelectedBus::Nats(_) => return Err(BackendRuntimeError::Stopped),
            };
            let cursor = events.last().map_or(after, |event| event.sequence);
            Ok(NatsReplay {
                events,
                gaps: vec![],
                cursor,
            })
        })
    }
    pub fn reconcile_publication(
        &self,
        id: &str,
        event: &Event,
    ) -> Result<(), BackendRuntimeError> {
        for record in self.records.list(&self.outbox_prefix, 1024)? {
            let mut publication: RuntimePublication = serde_json::from_value(record.value)?;
            if publication.id == id {
                if event.payload != publication.digest.as_bytes()
                    || !matches!(
                        publication.state,
                        PublicationState::Uncertain | PublicationState::Dispatched
                    )
                {
                    return Err(BackendRuntimeError::Conflict);
                }
                publication.state = PublicationState::Published;
                publication.event_sequence = Some(event.sequence);
                return self.update_publication(record.key, record.revision, publication);
            }
        }
        Err(BackendRuntimeError::Conflict)
    }
}
impl Drop for BackendRuntime {
    fn drop(&mut self) {
        let _ = self.records.sender.send(Command::Stop);
        if let Some(worker) = self.thread.take() {
            let _ = worker.join();
        }
    }
}

fn decode_record(
    key: String,
    revision: i64,
    body: String,
    digest: String,
) -> Result<OperationalRecord, BackendRuntimeError> {
    if revision <= 0 || digest_bytes(body.as_bytes()) != digest {
        return Err(BackendRuntimeError::Corrupt);
    }
    Ok(OperationalRecord {
        key,
        revision: revision as u64,
        value: serde_json::from_str(&body)?,
        digest,
    })
}
fn get_record(
    selected: &mut SelectedBackends,
    key: &str,
) -> Result<Option<OperationalRecord>, BackendRuntimeError> {
    selected.readiness()?;
    let row = match &mut selected.operational {
        SelectedOperationalStore::Sqlite(store) => store
            .read(|db| {
                use rusqlite::OptionalExtension;
                Ok(db
                    .query_row(
                        "SELECT revision,body,digest FROM runtime_backend_records WHERE key=?1",
                        [key],
                        |row| {
                            Ok((
                                row.get::<_, i64>(0)?,
                                row.get::<_, String>(1)?,
                                row.get::<_, String>(2)?,
                            ))
                        },
                    )
                    .optional()?)
            })
            .map_err(BackendConfigError::from)?,
        SelectedOperationalStore::Postgres(store) => store
            .read(store.epoch(), |tx| {
                Ok(tx
                    .query_opt(
                        "SELECT revision,body,digest FROM runtime_backend_records WHERE key=$1",
                        &[&key],
                    )?
                    .map(|row| {
                        (
                            row.get::<_, i64>(0),
                            row.get::<_, String>(1),
                            row.get::<_, String>(2),
                        )
                    }))
            })
            .map_err(BackendConfigError::from)?,
    };
    row.map(|(revision, body, digest)| decode_record(key.into(), revision, body, digest))
        .transpose()
}
fn list_records(
    selected: &mut SelectedBackends,
    prefix: &str,
    limit: u32,
) -> Result<Vec<OperationalRecord>, BackendRuntimeError> {
    selected.readiness()?;
    let rows:Vec<(String,i64,String,String)>=match &mut selected.operational {
        SelectedOperationalStore::Sqlite(store)=>store.read(|db| {let mut statement=db.prepare("SELECT key,revision,body,digest FROM runtime_backend_records WHERE key LIKE ?1 ORDER BY key LIMIT ?2")?;let rows=statement.query_map(rusqlite::params![prefix,limit],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?)))?;Ok(rows.collect::<Result<_,_>>()?) }).map_err(BackendConfigError::from)?,
        SelectedOperationalStore::Postgres(store)=>store.read(store.epoch(),|tx|Ok(tx.query("SELECT key,revision,body,digest FROM runtime_backend_records WHERE key LIKE $1 ORDER BY key LIMIT $2",&[&prefix,&(limit as i64)])?.into_iter().map(|row|(row.get(0),row.get(1),row.get(2),row.get(3))).collect())).map_err(BackendConfigError::from)?,
    };
    rows.into_iter()
        .map(|(key, revision, body, digest)| decode_record(key, revision, body, digest))
        .collect()
}
fn cas_records(
    selected: &mut SelectedBackends,
    changes: Vec<RecordChange>,
) -> Result<Vec<OperationalRecord>, BackendRuntimeError> {
    selected.readiness()?;
    let prepared: Vec<(String, Option<i64>, i64, String, String, Value)> = changes
        .into_iter()
        .map(|change| {
            let body = serde_json::to_string(&change.value)?;
            let digest = digest_bytes(body.as_bytes());
            let revision = change.expected.map_or(1, |old| old + 1) as i64;
            Ok((
                change.key,
                change.expected.map(|old| old as i64),
                revision,
                body,
                digest,
                change.value,
            ))
        })
        .collect::<Result<_, serde_json::Error>>()?;
    let applied=match &mut selected.operational {
        SelectedOperationalStore::Sqlite(store)=>store.transaction(store.epoch(),|tx| {
            use rusqlite::OptionalExtension;
            for (key,expected,..) in &prepared {let old:Option<i64>=tx.query_row("SELECT revision FROM runtime_backend_records WHERE key=?1",[key],|row|row.get(0)).optional()?;if old!=*expected{return Ok(false);}}
            for (key,_,revision,body,digest,_) in &prepared {tx.execute("INSERT INTO runtime_backend_records VALUES(?1,?2,?3,?4) ON CONFLICT(key) DO UPDATE SET revision=excluded.revision,body=excluded.body,digest=excluded.digest",rusqlite::params![key,revision,body,digest])?;}Ok(true)
        }).map_err(BackendConfigError::from)?,
        SelectedOperationalStore::Postgres(store)=>store.transaction(store.epoch(),|tx| {
            for (key,expected,..) in &prepared {let old=tx.query_opt("SELECT revision FROM runtime_backend_records WHERE key=$1",&[key])?.map(|row|row.get::<_,i64>(0));if old!=*expected{return Ok(false);}}
            for (key,_,revision,body,digest,_) in &prepared {tx.execute("INSERT INTO runtime_backend_records VALUES($1,$2,$3,$4) ON CONFLICT(key) DO UPDATE SET revision=excluded.revision,body=excluded.body,digest=excluded.digest",&[key,revision,body,digest])?;}Ok(true)
        }).map_err(BackendConfigError::from)?,
    };
    if !applied {
        return Err(BackendRuntimeError::Conflict);
    }
    Ok(prepared
        .into_iter()
        .map(|(key, _, revision, _, digest, value)| OperationalRecord {
            key,
            revision: revision as u64,
            value,
            digest,
        })
        .collect())
}
