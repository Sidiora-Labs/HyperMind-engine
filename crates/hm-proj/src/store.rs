#![allow(clippy::missing_errors_doc)]

use heed::types::Bytes;
use heed::{Database, Env, EnvOpenOptions, RoTxn, WithoutTls};
use hm_core::{Error, ErrorCode, LSN};
use std::ops::Bound::{Excluded, Included, Unbounded};
use std::path::Path;
use std::thread::{self, ThreadId};

const PROJECTION_NAMES: [&str; ProjectionId::COUNT] = [
    "belief_store",
    "entity_index",
    "vector_lane",
    "bm25",
    "temporal_ladder",
    "intent_frame",
    "work_ledger",
    "conversation_heads",
    "bindings",
    "memories",
    "graph",
    "fsrs",
    "runs",
];

type ByteDatabase = Database<Bytes, Bytes>;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum ProjectionId {
    BeliefStore,
    EntityIndex,
    VectorLane,
    Bm25,
    TemporalLadder,
    IntentFrame,
    WorkLedger,
    ConversationHeads,
    Bindings,
    Memories,
    Graph,
    Fsrs,
    Runs,
}

impl ProjectionId {
    pub const COUNT: usize = 13;

    #[must_use]
    pub const fn name(self) -> &'static str {
        PROJECTION_NAMES[self as usize]
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MutationKind {
    Put,
    Delete,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Mutation {
    pub kind: MutationKind,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}

impl Mutation {
    #[must_use]
    pub fn put(key: impl Into<Vec<u8>>, value: impl Into<Vec<u8>>) -> Self {
        Self {
            kind: MutationKind::Put,
            key: key.into(),
            value: value.into(),
        }
    }

    #[must_use]
    pub fn delete(key: impl Into<Vec<u8>>) -> Self {
        Self {
            kind: MutationKind::Delete,
            key: key.into(),
            value: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyValue {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}

pub struct ProjectionStore {
    environment: Env<WithoutTls>,
    databases: [ByteDatabase; ProjectionId::COUNT],
    metadata_database: ByteDatabase,
    writer_thread: ThreadId,
}

pub struct ReadSnapshot<'environment> {
    transaction: RoTxn<'environment, WithoutTls>,
    databases: [ByteDatabase; ProjectionId::COUNT],
    metadata_database: ByteDatabase,
    epoch: usize,
}

impl ProjectionStore {
    pub fn open(actor_directory: impl AsRef<Path>, map_bytes: usize) -> Result<Self, Error> {
        if map_bytes < 1024 * 1024 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let directory = actor_directory.as_ref().join("projections");
        std::fs::create_dir_all(&directory).map_err(|_| Error::new(ErrorCode::OpenFailed))?;
        let mut options = EnvOpenOptions::new().read_txn_without_tls();
        options.map_size(map_bytes).max_dbs(
            u32::try_from(ProjectionId::COUNT)
                .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?
                + 1,
        );
        // SAFETY: this process opens each actor's dedicated projection directory once and all
        // access remains owned by the returned heed environment.
        let environment = unsafe { options.open(&directory) }.map_err(database_error)?;
        let mut transaction = environment.write_txn().map_err(database_error)?;
        let metadata_database = environment
            .create_database::<Bytes, Bytes>(&mut transaction, Some("projection_checkpoints"))
            .map_err(database_error)?;
        let mut opened = Vec::with_capacity(ProjectionId::COUNT);
        for name in PROJECTION_NAMES {
            opened.push(
                environment
                    .create_database::<Bytes, Bytes>(&mut transaction, Some(name))
                    .map_err(database_error)?,
            );
        }
        transaction.commit().map_err(database_error)?;
        let databases = opened
            .try_into()
            .map_err(|_| Error::new(ErrorCode::InvariantViolation))?;
        Ok(Self {
            environment,
            databases,
            metadata_database,
            writer_thread: thread::current().id(),
        })
    }

    pub fn apply(
        &self,
        projection: ProjectionId,
        applied_lsn: LSN,
        mutations: &[Mutation],
    ) -> Result<(), Error> {
        if projection == ProjectionId::BeliefStore {
            return Err(Error::new(ErrorCode::BeliefWriteGate).at_lsn(applied_lsn));
        }
        self.apply_internal(projection, applied_lsn, mutations)
    }

    pub(crate) fn apply_belief(
        &self,
        applied_lsn: LSN,
        mutations: &[Mutation],
    ) -> Result<(), Error> {
        self.apply_internal(ProjectionId::BeliefStore, applied_lsn, mutations)
    }

    pub fn reset(&self, projection: ProjectionId) -> Result<(), Error> {
        self.require_writer()?;
        let mut transaction = self.environment.write_txn().map_err(database_error)?;
        self.database(projection)
            .clear(&mut transaction)
            .map_err(database_error)?;
        self.metadata_database
            .put(
                &mut transaction,
                projection.name().as_bytes(),
                &0_u64.to_le_bytes(),
            )
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)
    }

    pub fn begin_snapshot(&self) -> Result<ReadSnapshot<'_>, Error> {
        let transaction = self.environment.read_txn().map_err(database_error)?;
        let epoch = transaction.id();
        Ok(ReadSnapshot {
            transaction,
            databases: self.databases,
            metadata_database: self.metadata_database,
            epoch,
        })
    }

    fn apply_internal(
        &self,
        projection: ProjectionId,
        applied_lsn: LSN,
        mutations: &[Mutation],
    ) -> Result<(), Error> {
        self.require_writer()?;
        if applied_lsn.get() == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let mut transaction = self.environment.write_txn().map_err(database_error)?;
        let checkpoint = read_checkpoint(&transaction, self.metadata_database, projection)?;
        if checkpoint == u64::MAX || applied_lsn.get() != checkpoint + 1 {
            return Err(Error::new(ErrorCode::ProjectionCheckpoint)
                .at_lsn(applied_lsn)
                .at_offset(checkpoint));
        }
        let database = self.database(projection);
        for mutation in mutations {
            if mutation.key.is_empty() {
                return Err(Error::new(ErrorCode::InvalidArgument).at_lsn(applied_lsn));
            }
            match mutation.kind {
                MutationKind::Put => database
                    .put(&mut transaction, &mutation.key, &mutation.value)
                    .map_err(database_error)?,
                MutationKind::Delete => {
                    database
                        .delete(&mut transaction, &mutation.key)
                        .map_err(database_error)?;
                }
            }
        }
        self.metadata_database
            .put(
                &mut transaction,
                projection.name().as_bytes(),
                &applied_lsn.get().to_le_bytes(),
            )
            .map_err(database_error)?;
        transaction.commit().map_err(database_error)
    }

    fn require_writer(&self) -> Result<(), Error> {
        if thread::current().id() == self.writer_thread {
            Ok(())
        } else {
            Err(Error::new(ErrorCode::WriterViolation))
        }
    }

    fn database(&self, projection: ProjectionId) -> ByteDatabase {
        self.databases[projection as usize]
    }
}

impl ReadSnapshot<'_> {
    #[must_use]
    pub const fn epoch(&self) -> usize {
        self.epoch
    }

    pub fn get(&self, projection: ProjectionId, key: &[u8]) -> Result<Option<Vec<u8>>, Error> {
        if key.is_empty() {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        self.database(projection)
            .get(&self.transaction, key)
            .map(|value| value.map(<[u8]>::to_vec))
            .map_err(database_error)
    }

    pub fn first_at_or_after(
        &self,
        projection: ProjectionId,
        key: &[u8],
    ) -> Result<Option<KeyValue>, Error> {
        if key.is_empty() {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let mut range = self
            .database(projection)
            .range(&self.transaction, &(Included(key), Unbounded))
            .map_err(database_error)?;
        range
            .next()
            .transpose()
            .map(|item| item.map(key_value))
            .map_err(database_error)
    }

    pub fn scan_prefix(
        &self,
        projection: ProjectionId,
        prefix: &[u8],
        limit: usize,
    ) -> Result<Vec<KeyValue>, Error> {
        self.scan_prefix_from(projection, prefix, prefix, limit)
    }

    pub fn scan_prefix_from(
        &self,
        projection: ProjectionId,
        prefix: &[u8],
        first_key: &[u8],
        limit: usize,
    ) -> Result<Vec<KeyValue>, Error> {
        validate_scan(prefix, first_key, limit)?;
        let range = self
            .database(projection)
            .range(&self.transaction, &(Included(first_key), Unbounded))
            .map_err(database_error)?;
        collect_prefix(range, prefix, limit)
    }

    pub fn scan_prefix_reverse(
        &self,
        projection: ProjectionId,
        prefix: &[u8],
        limit: usize,
    ) -> Result<Vec<KeyValue>, Error> {
        if prefix.is_empty() || limit == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let mut upper = prefix.to_vec();
        let bound = increment_prefix(&mut upper).map_or(Unbounded, |()| Excluded(upper.as_slice()));
        let range = self
            .database(projection)
            .rev_range(&self.transaction, &(Unbounded, bound))
            .map_err(database_error)?;
        collect_prefix(range, prefix, limit)
    }

    pub fn scan_prefix_reverse_before(
        &self,
        projection: ProjectionId,
        prefix: &[u8],
        before_key: &[u8],
        limit: usize,
    ) -> Result<Vec<KeyValue>, Error> {
        validate_scan(prefix, before_key, limit)?;
        let range = self
            .database(projection)
            .rev_range(&self.transaction, &(Unbounded, Excluded(before_key)))
            .map_err(database_error)?;
        collect_prefix(range, prefix, limit)
    }

    pub fn checkpoint(&self, projection: ProjectionId) -> Result<LSN, Error> {
        read_checkpoint(&self.transaction, self.metadata_database, projection).map(LSN::new)
    }

    pub fn canonical_dump(&self, projection: ProjectionId) -> Result<Vec<u8>, Error> {
        let mut output = self.checkpoint(projection)?.get().to_le_bytes().to_vec();
        let iterator = self
            .database(projection)
            .iter(&self.transaction)
            .map_err(database_error)?;
        for item in iterator {
            let (key, value) = item.map_err(database_error)?;
            output.extend_from_slice(&(key.len() as u64).to_le_bytes());
            output.extend_from_slice(key);
            output.extend_from_slice(&(value.len() as u64).to_le_bytes());
            output.extend_from_slice(value);
        }
        Ok(output)
    }

    fn database(&self, projection: ProjectionId) -> ByteDatabase {
        self.databases[projection as usize]
    }
}

fn read_checkpoint(
    transaction: &RoTxn<'_>,
    metadata: ByteDatabase,
    projection: ProjectionId,
) -> Result<u64, Error> {
    let value = metadata
        .get(transaction, projection.name().as_bytes())
        .map_err(database_error)?;
    match value {
        None => Ok(0),
        Some(bytes) => bytes
            .try_into()
            .map(u64::from_le_bytes)
            .map_err(|_| Error::new(ErrorCode::InvariantViolation)),
    }
}

fn validate_scan(prefix: &[u8], key: &[u8], limit: usize) -> Result<(), Error> {
    if prefix.is_empty() || key.is_empty() || !key.starts_with(prefix) || limit == 0 {
        Err(Error::new(ErrorCode::InvalidArgument))
    } else {
        Ok(())
    }
}

fn collect_prefix<'transaction>(
    iterator: impl Iterator<Item = Result<(&'transaction [u8], &'transaction [u8]), heed::Error>>,
    prefix: &[u8],
    limit: usize,
) -> Result<Vec<KeyValue>, Error> {
    let mut output = Vec::new();
    for item in iterator {
        let (key, value) = item.map_err(database_error)?;
        if !key.starts_with(prefix) {
            break;
        }
        output.push(key_value((key, value)));
        if output.len() == limit {
            break;
        }
    }
    Ok(output)
}

fn key_value((key, value): (&[u8], &[u8])) -> KeyValue {
    KeyValue {
        key: key.to_vec(),
        value: value.to_vec(),
    }
}

fn increment_prefix(prefix: &mut Vec<u8>) -> Option<()> {
    for index in (0..prefix.len()).rev() {
        if prefix[index] != u8::MAX {
            prefix[index] += 1;
            prefix.truncate(index + 1);
            return Some(());
        }
    }
    None
}

#[allow(clippy::needless_pass_by_value)]
fn database_error(error: heed::Error) -> Error {
    let code = match error {
        heed::Error::Mdb(heed::MdbError::MapFull) => ErrorCode::MapFull,
        _ => ErrorCode::BackendUnavailable,
    };
    Error::new(code)
}
