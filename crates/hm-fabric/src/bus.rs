use fs2::FileExt;
use hm_context::{ContextError, Scope};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackendCapabilities {
    pub backend: String,
    pub durable: bool,
    pub replay: bool,
    pub register_cas: bool,
    pub distributed: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Grant {
    pub principal: String,
    pub stream: String,
    pub publish: bool,
    pub subscribe: bool,
    pub register: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Event {
    pub sequence: u64,
    pub stream: String,
    pub payload: Vec<u8>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Delivery {
    pub event: Event,
    pub subscriber: String,
    pub attempt: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Disposition {
    Ack,
    Nak,
    Term,
    Progress { lease_ms: i64 },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Register {
    pub revision: u64,
    pub value: Vec<u8>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeadLetter {
    pub delivery: Delivery,
    pub reason: String,
}
pub struct Bus {
    connection: Connection,
    scope: String,
    _writer_lock: File,
    path: PathBuf,
}
fn db(e: rusqlite::Error) -> ContextError {
    ContextError::Unavailable(e.to_string())
}
pub fn validate_name(name: &str) -> Result<(), ContextError> {
    if name.is_empty()
        || name.len() > 128
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(ContextError::Invalid("invalid bus name".into()));
    }
    Ok(())
}
pub fn scoped_name(scope: &Scope, name: &str) -> Result<String, ContextError> {
    validate_name(name)?;
    Ok(format!("hm.{}.{}", scope.digest()?, name))
}
impl Bus {
    pub fn open(path: impl AsRef<Path>, scope: Scope) -> Result<Self, ContextError> {
        let scope = scope.digest()?;
        let input = path.as_ref();
        let parent = fs::canonicalize(
            input
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new(".")),
        )?;
        let path = parent.join(
            input
                .file_name()
                .ok_or_else(|| ContextError::Invalid("invalid bus path".into()))?,
        );
        reject_alias(&path)?;
        let mut lock_name = path.file_name().unwrap().to_os_string();
        lock_name.push(".writer-lock");
        let lock_path = path.with_file_name(lock_name);
        reject_alias(&lock_path)?;
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let writer_lock = options.open(lock_path)?;
        writer_lock.try_lock_exclusive().map_err(|e| {
            if e.kind() == std::io::ErrorKind::WouldBlock {
                ContextError::Conflict
            } else {
                ContextError::Io(e)
            }
        })?;
        let connection = Connection::open(&path).map_err(db)?;
        connection
            .busy_timeout(std::time::Duration::from_secs(5))
            .map_err(db)?;
        connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
CREATE TABLE IF NOT EXISTS bus_events(scope TEXT, seq INTEGER PRIMARY KEY AUTOINCREMENT, stream TEXT, payload BLOB);
CREATE TABLE IF NOT EXISTS bus_grants(scope TEXT, principal TEXT, stream TEXT, publish INTEGER, subscribe INTEGER, reg INTEGER, PRIMARY KEY(scope,principal,stream));
CREATE TABLE IF NOT EXISTS bus_subscriptions(scope TEXT, stream TEXT, subscriber TEXT, cursor INTEGER, attempts INTEGER, max_attempts INTEGER, lease INTEGER, PRIMARY KEY(scope,stream,subscriber));
CREATE TABLE IF NOT EXISTS bus_dead(scope TEXT, stream TEXT, subscriber TEXT, seq INTEGER, attempt INTEGER, payload BLOB, reason TEXT, PRIMARY KEY(scope,stream,subscriber,seq));
CREATE TABLE IF NOT EXISTS bus_dedup(scope TEXT, stream TEXT, id TEXT, seq INTEGER NOT NULL, PRIMARY KEY(scope,stream,id));
CREATE TABLE IF NOT EXISTS bus_registers(scope TEXT, stream TEXT, name TEXT, revision INTEGER, value BLOB, PRIMARY KEY(scope,stream,name));").map_err(db)?;
        Ok(Self {
            connection,
            scope,
            _writer_lock: writer_lock,
            path,
        })
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn append_once(
        &mut self,
        principal: &str,
        stream: &str,
        id: &str,
        payload: &[u8],
    ) -> Result<Event, ContextError> {
        self.authorize(principal, stream, "publish")?;
        hm_context::types::validate_id(id)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db)?;
        let existing: Option<Event> = tx.query_row("SELECT e.seq,e.payload FROM bus_dedup d JOIN bus_events e ON e.seq=d.seq AND e.scope=d.scope AND e.stream=d.stream WHERE d.scope=?1 AND d.stream=?2 AND d.id=?3", params![self.scope,stream,id], |r| Ok(Event{sequence:r.get::<_,i64>(0)? as u64,stream:stream.into(),payload:r.get(1)?})).optional().map_err(db)?;
        if let Some(event) = existing {
            if event.payload != payload {
                return Err(ContextError::Conflict);
            }
            tx.commit().map_err(db)?;
            return Ok(event);
        }
        tx.execute(
            "INSERT INTO bus_events(scope,stream,payload) VALUES(?1,?2,?3)",
            params![self.scope, stream, payload],
        )
        .map_err(db)?;
        let sequence = tx.last_insert_rowid();
        tx.execute(
            "INSERT INTO bus_dedup VALUES(?1,?2,?3,?4)",
            params![self.scope, stream, id, sequence],
        )
        .map_err(db)?;
        tx.commit().map_err(db)?;
        Ok(Event {
            sequence: sequence as u64,
            stream: stream.into(),
            payload: payload.into(),
        })
    }
    pub fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            backend: "sqlite".into(),
            durable: true,
            replay: true,
            register_cas: true,
            distributed: false,
        }
    }
    pub fn grant(&mut self, grant: Grant) -> Result<(), ContextError> {
        validate_name(&grant.principal)?;
        validate_name(&grant.stream)?;
        self.connection.execute("INSERT INTO bus_grants VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(scope,principal,stream) DO UPDATE SET publish=excluded.publish,subscribe=excluded.subscribe,reg=excluded.reg",params![self.scope,grant.principal,grant.stream,grant.publish,grant.subscribe,grant.register]).map_err(db)?;
        Ok(())
    }
    fn authorize(&self, principal: &str, stream: &str, right: &str) -> Result<(), ContextError> {
        validate_name(principal)?;
        validate_name(stream)?;
        let sql =
            format!("SELECT {right} FROM bus_grants WHERE scope=?1 AND principal=?2 AND stream=?3");
        let allowed: Option<bool> = self
            .connection
            .query_row(&sql, params![self.scope, principal, stream], |r| r.get(0))
            .optional()
            .map_err(db)?;
        if allowed == Some(true) {
            Ok(())
        } else {
            Err(ContextError::ScopeMismatch)
        }
    }
    pub fn append(
        &mut self,
        principal: &str,
        stream: &str,
        payload: &[u8],
    ) -> Result<Event, ContextError> {
        self.authorize(principal, stream, "publish")?;
        self.connection
            .execute(
                "INSERT INTO bus_events(scope,stream,payload) VALUES(?1,?2,?3)",
                params![self.scope, stream, payload],
            )
            .map_err(db)?;
        Ok(Event {
            sequence: self.connection.last_insert_rowid() as u64,
            stream: stream.into(),
            payload: payload.into(),
        })
    }
    pub fn replay(
        &self,
        principal: &str,
        stream: &str,
        after: u64,
        limit: u32,
    ) -> Result<Vec<Event>, ContextError> {
        self.authorize(principal, stream, "subscribe")?;
        if after > i64::MAX as u64 || limit > 10000 {
            return Err(ContextError::Capacity);
        }
        let mut q=self.connection.prepare("SELECT seq,payload FROM bus_events WHERE scope=?1 AND stream=?2 AND seq>?3 ORDER BY seq LIMIT ?4").map_err(db)?;
        let rows = q
            .query_map(params![self.scope, stream, after as i64, limit], |r| {
                Ok(Event {
                    sequence: r.get::<_, i64>(0)? as u64,
                    stream: stream.into(),
                    payload: r.get(1)?,
                })
            })
            .map_err(db)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db)
    }
    pub fn subscribe(
        &mut self,
        principal: &str,
        stream: &str,
        subscriber: &str,
        max_attempts: u32,
    ) -> Result<(), ContextError> {
        self.authorize(principal, stream, "subscribe")?;
        validate_name(subscriber)?;
        if max_attempts == 0 || max_attempts > 1000 {
            return Err(ContextError::Invalid("invalid attempt bound".into()));
        }
        self.connection
            .execute(
                "INSERT INTO bus_subscriptions VALUES(?1,?2,?3,0,0,?4,0) ON CONFLICT DO NOTHING",
                params![self.scope, stream, subscriber, max_attempts],
            )
            .map_err(db)?;
        let bound:u32=self.connection.query_row("SELECT max_attempts FROM bus_subscriptions WHERE scope=?1 AND stream=?2 AND subscriber=?3",params![self.scope,stream,subscriber],|r|r.get(0)).map_err(db)?;
        if bound != max_attempts {
            return Err(ContextError::Conflict);
        }
        Ok(())
    }
    pub fn next(
        &mut self,
        principal: &str,
        stream: &str,
        subscriber: &str,
        now_ms: i64,
        lease_ms: i64,
    ) -> Result<Option<Delivery>, ContextError> {
        self.authorize(principal, stream, "subscribe")?;
        validate_name(subscriber)?;
        let deadline = deadline(now_ms, lease_ms)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db)?;
        let (mut cursor, mut attempts, max, lease):(u64,u32,u32,i64)=tx.query_row("SELECT cursor,attempts,max_attempts,lease FROM bus_subscriptions WHERE scope=?1 AND stream=?2 AND subscriber=?3",params![self.scope,stream,subscriber],|r|Ok((r.get::<_,i64>(0)? as u64,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(db)?.ok_or(ContextError::Invalid("unknown subscription".into()))?;
        if lease > now_ms {
            return Ok(None);
        }
        loop {
            let event=tx.query_row("SELECT seq,payload FROM bus_events WHERE scope=?1 AND stream=?2 AND seq>?3 ORDER BY seq LIMIT 1",params![self.scope,stream,cursor as i64],|r|Ok(Event{sequence:r.get::<_,i64>(0)? as u64,stream:stream.into(),payload:r.get(1)?})).optional().map_err(db)?;
            let Some(event) = event else {
                tx.commit().map_err(db)?;
                return Ok(None);
            };
            if attempts >= max {
                tx.execute(
                    "INSERT INTO bus_dead VALUES(?1,?2,?3,?4,?5,?6,'attempts_exhausted')",
                    params![
                        self.scope,
                        stream,
                        subscriber,
                        event.sequence as i64,
                        attempts,
                        event.payload
                    ],
                )
                .map_err(db)?;
                cursor = event.sequence;
                attempts = 0;
                tx.execute("UPDATE bus_subscriptions SET cursor=?4,attempts=0,lease=0 WHERE scope=?1 AND stream=?2 AND subscriber=?3",params![self.scope,stream,subscriber,cursor as i64]).map_err(db)?;
                continue;
            }
            attempts += 1;
            tx.execute("UPDATE bus_subscriptions SET attempts=?4,lease=?5 WHERE scope=?1 AND stream=?2 AND subscriber=?3",params![self.scope,stream,subscriber,attempts,deadline]).map_err(db)?;
            tx.commit().map_err(db)?;
            return Ok(Some(Delivery {
                event,
                subscriber: subscriber.into(),
                attempt: attempts,
            }));
        }
    }
    pub fn disposition(
        &mut self,
        principal: &str,
        delivery: &Delivery,
        disposition: Disposition,
        now_ms: i64,
    ) -> Result<(), ContextError> {
        self.authorize(principal, &delivery.event.stream, "subscribe")?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db)?;
        let (cursor,attempt,lease):(u64,u32,i64)=tx.query_row("SELECT cursor,attempts,lease FROM bus_subscriptions WHERE scope=?1 AND stream=?2 AND subscriber=?3",params![self.scope,delivery.event.stream,delivery.subscriber],|r|Ok((r.get::<_,i64>(0)? as u64,r.get(1)?,r.get(2)?))).map_err(db)?;
        let seq:Option<i64>=tx.query_row("SELECT seq FROM bus_events WHERE scope=?1 AND stream=?2 AND seq>?3 ORDER BY seq LIMIT 1",params![self.scope,delivery.event.stream,cursor as i64],|r|r.get(0)).optional().map_err(db)?;
        if seq != Some(delivery.event.sequence as i64)
            || attempt != delivery.attempt
            || attempt == 0
            || lease <= now_ms
        {
            return Err(ContextError::Stale);
        }
        let (cursor, attempt, lease) = match disposition {
            Disposition::Ack => (delivery.event.sequence, 0, 0),
            Disposition::Nak => (cursor, attempt, 0),
            Disposition::Progress { lease_ms } => (cursor, attempt, deadline(now_ms, lease_ms)?),
            Disposition::Term => {
                tx.execute("INSERT INTO bus_dead SELECT scope,stream,?3,seq,?4,payload,'terminated' FROM bus_events WHERE scope=?1 AND stream=?2 AND seq=?5",params![self.scope,delivery.event.stream,delivery.subscriber,attempt,delivery.event.sequence as i64]).map_err(db)?;
                (delivery.event.sequence, 0, 0)
            }
        };
        tx.execute("UPDATE bus_subscriptions SET cursor=?4,attempts=?5,lease=?6 WHERE scope=?1 AND stream=?2 AND subscriber=?3",params![self.scope,delivery.event.stream,delivery.subscriber,cursor as i64,attempt,lease]).map_err(db)?;
        tx.commit().map_err(db)
    }
    pub fn dead_letters(
        &self,
        principal: &str,
        stream: &str,
        subscriber: &str,
    ) -> Result<Vec<DeadLetter>, ContextError> {
        self.authorize(principal, stream, "subscribe")?;
        let mut q=self.connection.prepare("SELECT seq,attempt,payload,reason FROM bus_dead WHERE scope=?1 AND stream=?2 AND subscriber=?3 ORDER BY seq").map_err(db)?;
        let rows = q
            .query_map(params![self.scope, stream, subscriber], |r| {
                Ok(DeadLetter {
                    delivery: Delivery {
                        event: Event {
                            sequence: r.get::<_, i64>(0)? as u64,
                            stream: stream.into(),
                            payload: r.get(2)?,
                        },
                        subscriber: subscriber.into(),
                        attempt: r.get(1)?,
                    },
                    reason: r.get(3)?,
                })
            })
            .map_err(db)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(db)
    }
    pub fn register_get(
        &self,
        principal: &str,
        stream: &str,
        name: &str,
    ) -> Result<Option<Register>, ContextError> {
        self.authorize(principal, stream, "reg")?;
        validate_name(name)?;
        self.connection
            .query_row(
                "SELECT revision,value FROM bus_registers WHERE scope=?1 AND stream=?2 AND name=?3",
                params![self.scope, stream, name],
                |r| {
                    Ok(Register {
                        revision: r.get::<_, i64>(0)? as u64,
                        value: r.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(db)
    }
    pub fn register_cas(
        &mut self,
        principal: &str,
        stream: &str,
        name: &str,
        expected: u64,
        value: &[u8],
    ) -> Result<Register, ContextError> {
        self.authorize(principal, stream, "reg")?;
        validate_name(name)?;
        if expected >= i64::MAX as u64 {
            return Err(ContextError::Capacity);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db)?;
        let current: Option<i64> = tx
            .query_row(
                "SELECT revision FROM bus_registers WHERE scope=?1 AND stream=?2 AND name=?3",
                params![self.scope, stream, name],
                |r| r.get(0),
            )
            .optional()
            .map_err(db)?;
        if current.unwrap_or(0) != expected as i64 {
            return Err(ContextError::Conflict);
        }
        let revision = expected + 1;
        tx.execute("INSERT INTO bus_registers VALUES(?1,?2,?3,?4,?5) ON CONFLICT(scope,stream,name) DO UPDATE SET revision=excluded.revision,value=excluded.value",params![self.scope,stream,name,revision as i64,value]).map_err(db)?;
        tx.commit().map_err(db)?;
        Ok(Register {
            revision,
            value: value.into(),
        })
    }
}
fn deadline(now: i64, lease: i64) -> Result<i64, ContextError> {
    if now < 0 || lease <= 0 {
        return Err(ContextError::Invalid("invalid lease".into()));
    }
    now.checked_add(lease).ok_or(ContextError::Capacity)
}

fn reject_alias(path: &Path) -> Result<(), ContextError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(ContextError::Invalid("invalid bus file".into()));
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                if metadata.nlink() != 1 {
                    return Err(ContextError::Invalid("aliased bus file".into()));
                }
            }
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(ContextError::Io(e)),
    }
}
