#![allow(clippy::missing_errors_doc)]

use hm_compose::bundle::{self, ActivationBundle, ActivationRequest};
use hm_compose::tokens::{FallbackWeights, TokenCounter};
use hm_core::{ActorId, ConversationId, Error, ErrorCode, LSN, UtcNanos};
use hm_ledger::checkpoint::{SigningKeyPair, signing_key_pair_for};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_ledger::idempotency::{
    Admission, BatchEvent, BatchIdentity, ConnectionId, DedupTable, rollback_torn_batch,
};
use hm_ledger::keyring::{KeyEncryptionKey, KeyHierarchy, OsEntropy, UserId};
use hm_ledger::mmr::Hash as MmrHash;
use hm_ledger::mmr_store::{MmrStore, VerificationStatus};
use hm_ledger::rotate::rotate_keys;
use hm_ledger::segment::{AppendRequest, SegmentLog, SegmentLogOptions};
use hm_ledger::shred::{crypto_shred, encode_deletion_receipt};
use hm_ledger::tripwire::TripwireSet;
use hm_proj::checkpoint::{
    CheckpointRead, encode_checkpoint_cursor, latest_checkpoint, turn_conversation,
};
use hm_proj::lexical::LexicalProjection;
use hm_proj::rebuild::rebuild_projection_stream;
use hm_proj::store::{ProjectionId, ProjectionStore};
use hm_proj::timeline::{ConversationRecord, read_conversation_record, read_conversation_records};
use hm_schema::event::{self, Boundary, CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    Authority, Checkpoint, EventEnvelope, EventPayload, Retention, Sensitivity,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, mpsc, oneshot};

const COMMAND_QUEUE: usize = 256;

#[derive(Clone, Debug)]
pub struct ActorConfig {
    pub actor_directory: PathBuf,
    pub actor: ActorId,
    pub user: UserId,
    pub kek: KeyEncryptionKey,
    pub projection_map_bytes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IncomingEvent {
    pub kind: EventKind,
    pub conversation: ConversationId,
    pub payload: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppendOutcome {
    pub first_lsn: LSN,
    pub last_lsn: LSN,
    pub duplicate: bool,
    pub leaf_count: u64,
    pub last_leaf_hash: MmrHash,
    pub mmr_root: MmrHash,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CheckpointOutcome {
    pub lsn: LSN,
    pub duplicate: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IntegrityReceipt {
    pub lsn: LSN,
    pub leaf_hash: MmrHash,
    pub root: MmrHash,
    pub checkpoint_lsn: LSN,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecallItem {
    pub lsn: LSN,
    pub kind: EventKind,
    pub conversation: ConversationId,
    pub wall_timestamp_ns: UtcNanos,
    pub payload: Vec<u8>,
    pub score_q32: u64,
}

#[derive(Clone, Debug)]
pub enum RecallRequest {
    Lexical {
        query: String,
        limit: usize,
    },
    Timeline {
        conversation: ConversationId,
        since_lsn: LSN,
        limit: usize,
    },
}

#[derive(Clone, Debug)]
pub struct ActivateRequest {
    pub conversation: ConversationId,
    pub query: String,
    pub turn_text: String,
    pub budget_tokens: usize,
    pub token_weights: FallbackWeights,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppliedState {
    pub last_lsn: LSN,
    pub event_count: u64,
    pub rolling_digest: [u8; 32],
}

impl Default for AppliedState {
    fn default() -> Self {
        Self {
            last_lsn: LSN::new(0),
            event_count: 0,
            rolling_digest: [0; 32],
        }
    }
}

impl AppliedState {
    fn apply(&mut self, frame: &Frame) -> Result<(), Error> {
        if frame.header.lsn.get() != self.last_lsn.get() + 1 {
            return Err(Error::new(ErrorCode::SequenceViolation).at_lsn(frame.header.lsn));
        }
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"hypermind.applied-state.v1\0");
        hasher.update(&self.rolling_digest);
        hasher.update(&frame.header.lsn.get().to_le_bytes());
        hasher.update(&[frame.header.kind as u8]);
        hasher.update(&frame.header.wall_timestamp_ns.get().to_le_bytes());
        hasher.update(&frame.header.actor.get().to_le_bytes());
        hasher.update(frame.header.conversation.as_bytes());
        hasher.update(blake3::hash(&frame.sealed_payload).as_bytes());
        self.rolling_digest = *hasher.finalize().as_bytes();
        self.last_lsn = frame.header.lsn;
        self.event_count = self
            .event_count
            .checked_add(1)
            .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionStat {
    pub name: &'static str,
    pub applied_lsn: LSN,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActorStats {
    pub actor: ActorId,
    pub log_events: u64,
    pub log_bytes: u64,
    pub projections: Vec<ProjectionStat>,
    pub applied: AppliedState,
}

#[derive(Clone)]
pub struct ActorEngine {
    actor: ActorId,
    commands: mpsc::Sender<Command>,
    events: broadcast::Sender<Frame>,
}

enum Command {
    Append(
        Vec<IncomingEvent>,
        oneshot::Sender<Result<AppendOutcome, Error>>,
    ),
    IdempotentAppend(
        ConnectionId,
        u64,
        Vec<IncomingEvent>,
        oneshot::Sender<Result<AppendOutcome, Error>>,
    ),
    Checkpoint(
        ConnectionId,
        u64,
        Vec<u8>,
        Vec<u8>,
        oneshot::Sender<Result<CheckpointOutcome, Error>>,
    ),
    LatestCheckpoint(
        Vec<u8>,
        oneshot::Sender<Result<Option<CheckpointRead>, Error>>,
    ),
    FramesSince(
        LSN,
        Option<ConversationId>,
        usize,
        oneshot::Sender<Result<Vec<Frame>, Error>>,
    ),
    NextClientSequence(ConnectionId, oneshot::Sender<Result<u64, Error>>),
    Recall(
        RecallRequest,
        oneshot::Sender<Result<Vec<RecallItem>, Error>>,
    ),
    Activate(
        Box<ActivateRequest>,
        oneshot::Sender<Result<ActivationBundle, Error>>,
    ),
    VerificationStatus(oneshot::Sender<Result<VerificationStatus, Error>>),
    IntegrityAt(LSN, oneshot::Sender<Result<IntegrityReceipt, Error>>),
    RebuildProjection(String, oneshot::Sender<Result<LSN, Error>>),
    RotateKeys(KeyEncryptionKey, oneshot::Sender<Result<(), Error>>),
    GuardTripwires(Vec<LSN>, oneshot::Sender<Result<(), Error>>),
    CryptoDelete(oneshot::Sender<Result<Vec<u8>, Error>>),
    Stats(oneshot::Sender<Result<ActorStats, Error>>),
    Shutdown(oneshot::Sender<()>),
}

struct WriterState {
    config: ActorConfig,
    log: SegmentLog,
    keys: KeyHierarchy,
    projections: ProjectionStore,
    plaintext_frames: Vec<Frame>,
    kinds: Vec<event::EventKind>,
    authorities: Vec<Authority>,
    applied: AppliedState,
    dedup: DedupTable,
    last_wall_timestamp_ns: i64,
    events: broadcast::Sender<Frame>,
    mmr: MmrStore,
    signing_keys: SigningKeyPair,
    tripwires: TripwireSet,
}

impl ActorEngine {
    pub async fn open(config: ActorConfig) -> Result<Self, Error> {
        Self::open_with_tripwires(config, []).await
    }

    pub async fn open_with_tripwires(
        config: ActorConfig,
        tripwire_lsns: impl IntoIterator<Item = LSN>,
    ) -> Result<Self, Error> {
        if config.actor.get() == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let tripwires = TripwireSet::seeded(tripwire_lsns)?;
        let actor = config.actor;
        let (commands, receiver) = mpsc::channel(COMMAND_QUEUE);
        let (events, _) = broadcast::channel(COMMAND_QUEUE);
        let writer_events = events.clone();
        let (initialized_tx, initialized_rx) = oneshot::channel();
        std::thread::Builder::new()
            .name(format!("hm-actor-{}", actor.get()))
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build();
                let Ok(runtime) = runtime else {
                    let _ = initialized_tx.send(Err(Error::new(ErrorCode::OpenFailed)));
                    return;
                };
                runtime.block_on(async move {
                    match WriterState::open(config, writer_events, tripwires) {
                        Ok(state) => {
                            let _ = initialized_tx.send(Ok(()));
                            writer_loop(state, receiver).await;
                        }
                        Err(error) => {
                            let _ = initialized_tx.send(Err(error));
                        }
                    }
                });
            })
            .map_err(|_| Error::new(ErrorCode::OpenFailed))?;
        initialized_rx
            .await
            .map_err(|_| Error::new(ErrorCode::OpenFailed))??;
        Ok(Self {
            actor,
            commands,
            events,
        })
    }

    #[must_use]
    pub const fn actor(&self) -> ActorId {
        self.actor
    }

    pub async fn append(&self, events: Vec<IncomingEvent>) -> Result<AppendOutcome, Error> {
        request(&self.commands, |reply| Command::Append(events, reply)).await
    }

    pub async fn append_idempotent(
        &self,
        connection_id: ConnectionId,
        client_seq: u64,
        events: Vec<IncomingEvent>,
    ) -> Result<AppendOutcome, Error> {
        request(&self.commands, |reply| {
            Command::IdempotentAppend(connection_id, client_seq, events, reply)
        })
        .await
    }

    pub async fn write_checkpoint(
        &self,
        connection_id: ConnectionId,
        client_seq: u64,
        turn_id: Vec<u8>,
        blob: Vec<u8>,
    ) -> Result<CheckpointOutcome, Error> {
        request(&self.commands, |reply| {
            Command::Checkpoint(connection_id, client_seq, turn_id, blob, reply)
        })
        .await
    }

    pub async fn latest_checkpoint(
        &self,
        turn_id: Vec<u8>,
    ) -> Result<Option<CheckpointRead>, Error> {
        request(&self.commands, |reply| {
            Command::LatestCheckpoint(turn_id, reply)
        })
        .await
    }

    pub async fn frames_since(
        &self,
        since_lsn: LSN,
        conversation: Option<ConversationId>,
        maximum_frames: usize,
    ) -> Result<Vec<Frame>, Error> {
        request(&self.commands, |reply| {
            Command::FramesSince(since_lsn, conversation, maximum_frames, reply)
        })
        .await
    }

    pub async fn next_client_sequence(&self, connection_id: ConnectionId) -> Result<u64, Error> {
        request(&self.commands, |reply| {
            Command::NextClientSequence(connection_id, reply)
        })
        .await
    }

    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<Frame> {
        self.events.subscribe()
    }

    pub async fn recall(&self, request_value: RecallRequest) -> Result<Vec<RecallItem>, Error> {
        request(&self.commands, |reply| {
            Command::Recall(request_value, reply)
        })
        .await
    }

    pub async fn activate(
        &self,
        request_value: ActivateRequest,
    ) -> Result<ActivationBundle, Error> {
        request(&self.commands, |reply| {
            Command::Activate(Box::new(request_value), reply)
        })
        .await
    }

    pub async fn stats(&self) -> Result<ActorStats, Error> {
        request(&self.commands, Command::Stats).await
    }

    pub async fn verification_status(&self) -> Result<VerificationStatus, Error> {
        request(&self.commands, Command::VerificationStatus).await
    }

    pub async fn integrity_at(&self, lsn: LSN) -> Result<IntegrityReceipt, Error> {
        request(&self.commands, |reply| Command::IntegrityAt(lsn, reply)).await
    }

    pub async fn rebuild_projection(&self, name: String) -> Result<LSN, Error> {
        request(&self.commands, |reply| {
            Command::RebuildProjection(name, reply)
        })
        .await
    }

    pub async fn rotate_keys(&self, new_kek: KeyEncryptionKey) -> Result<(), Error> {
        request(&self.commands, |reply| Command::RotateKeys(new_kek, reply)).await
    }

    pub async fn guard_tripwires(&self, lsns: Vec<LSN>) -> Result<(), Error> {
        request(&self.commands, |reply| Command::GuardTripwires(lsns, reply)).await
    }

    pub async fn crypto_delete(&self) -> Result<Vec<u8>, Error> {
        request(&self.commands, Command::CryptoDelete).await
    }

    pub async fn shutdown(self) -> Result<(), Error> {
        let (reply, response) = oneshot::channel();
        self.commands
            .send(Command::Shutdown(reply))
            .await
            .map_err(|_| Error::new(ErrorCode::OperationUnavailable))?;
        response
            .await
            .map_err(|_| Error::new(ErrorCode::OperationUnavailable))
    }
}

async fn request<T>(
    commands: &mpsc::Sender<Command>,
    create: impl FnOnce(oneshot::Sender<Result<T, Error>>) -> Command,
) -> Result<T, Error> {
    let (reply, response) = oneshot::channel();
    commands
        .send(create(reply))
        .await
        .map_err(|_| Error::new(ErrorCode::OperationUnavailable))?;
    response
        .await
        .map_err(|_| Error::new(ErrorCode::OperationUnavailable))?
}

async fn writer_loop(mut state: WriterState, mut commands: mpsc::Receiver<Command>) {
    while let Some(command) = commands.recv().await {
        match command {
            Command::Append(events, reply) => {
                let before = state.plaintext_frames.len();
                let result = state.append(events);
                if result.as_ref().is_ok_and(|outcome| !outcome.duplicate) {
                    state.publish_from(before);
                }
                let _ = reply.send(result);
            }
            Command::IdempotentAppend(connection, sequence, events, reply) => {
                let before = state.plaintext_frames.len();
                let result = state.append_idempotent(connection, sequence, events);
                if result.as_ref().is_ok_and(|outcome| !outcome.duplicate) {
                    state.publish_from(before);
                }
                let _ = reply.send(result);
            }
            Command::Checkpoint(connection, sequence, turn_id, blob, reply) => {
                let before = state.plaintext_frames.len();
                let result = state.write_checkpoint(connection, sequence, &turn_id, &blob);
                if result.as_ref().is_ok_and(|outcome| !outcome.duplicate) {
                    state.publish_from(before);
                }
                let _ = reply.send(result);
            }
            Command::LatestCheckpoint(turn_id, reply) => {
                let _ = reply.send(state.latest_checkpoint(&turn_id));
            }
            Command::FramesSince(since_lsn, conversation, maximum, reply) => {
                let _ = reply.send(state.frames_since(since_lsn, conversation, maximum));
            }
            Command::NextClientSequence(connection, reply) => {
                let _ = reply.send(state.next_client_sequence(&connection));
            }
            Command::Recall(request, reply) => {
                let _ = reply.send(state.recall(request));
            }
            Command::Activate(request, reply) => {
                let _ = reply.send(state.activate(*request));
            }
            Command::VerificationStatus(reply) => {
                let _ = reply.send(Ok(state.mmr.verification_status()));
            }
            Command::IntegrityAt(lsn, reply) => {
                let _ = reply.send(state.integrity_at(lsn));
            }
            Command::RebuildProjection(name, reply) => {
                let _ = reply.send(state.rebuild_projection(&name));
            }
            Command::RotateKeys(new_kek, reply) => {
                let _ = reply.send(state.rotate_keys(new_kek));
            }
            Command::GuardTripwires(lsns, reply) => {
                let _ = reply.send(state.tripwires.guard(lsns));
            }
            Command::CryptoDelete(reply) => {
                let _ = reply.send(state.crypto_delete());
                return;
            }
            Command::Stats(reply) => {
                let _ = reply.send(state.stats());
            }
            Command::Shutdown(reply) => {
                drop(state);
                let _ = reply.send(());
                return;
            }
        }
    }
}

impl WriterState {
    fn open(
        config: ActorConfig,
        events: broadcast::Sender<Frame>,
        tripwires: TripwireSet,
    ) -> Result<Self, Error> {
        fs::create_dir_all(&config.actor_directory)
            .map_err(|_| Error::new(ErrorCode::OpenFailed))?;
        let mut entropy = OsEntropy;
        let keys = KeyHierarchy::open_or_create(
            &config.actor_directory,
            config.actor,
            config.user,
            &config.kek,
            &mut entropy,
            true,
        )?;
        let mut log = SegmentLog::open(
            &config.actor_directory,
            config.actor,
            SegmentLogOptions::default(),
        )?;
        rollback_torn_batch(&mut log, &keys)?;
        let signing_keys = signing_key_pair_for(&keys);
        let sealed_frames = log.read_all()?;
        let mut mmr = MmrStore::open(
            &config.actor_directory,
            config.actor,
            signing_keys.public_key,
        )?;
        loop {
            let repaired = mmr.verify_and_repair_bounded(&sealed_frames)?;
            if repaired.complete {
                break;
            }
        }
        if mmr.verification_status().leaf_count != 0
            && mmr.verification_status().last_checkpoint_lsn.get()
                != mmr.verification_status().leaf_count
        {
            mmr.create_checkpoint(&signing_keys)?;
        }
        let projections =
            ProjectionStore::open(&config.actor_directory, config.projection_map_bytes)?;
        let mut kinds = Vec::new();
        let mut authorities = Vec::new();
        let mut applied = AppliedState::default();
        let mut plaintext_frames = Vec::new();
        let mut last_wall_timestamp_ns = i64::MIN;
        for mut frame in log.read_all()? {
            frame.sealed_payload = keys.unseal(&frame.header, &frame.sealed_payload)?;
            let kind = schema_kind(frame.header.kind)?;
            let verified = event::verify_event_with_history(
                &frame.sealed_payload,
                kind,
                Boundary::Disk,
                &ActorHistory::new(&kinds, &authorities),
            )?;
            applied.apply(&frame)?;
            kinds.push(kind);
            authorities.push(verified.envelope.authority);
            last_wall_timestamp_ns =
                last_wall_timestamp_ns.max(frame.header.wall_timestamp_ns.get());
            plaintext_frames.push(frame);
        }
        let rebuilt =
            rebuild_projection_stream(&projections, &plaintext_frames, false, usize::MAX)?;
        if !rebuilt.complete {
            return Err(Error::new(ErrorCode::ProjectionCheckpoint));
        }
        let dedup = DedupTable::rebuild(&plaintext_frames)?;
        Ok(Self {
            config,
            log,
            keys,
            projections,
            plaintext_frames,
            kinds,
            authorities,
            applied,
            dedup,
            last_wall_timestamp_ns,
            events,
            mmr,
            signing_keys,
            tripwires,
        })
    }

    fn append(&mut self, events: Vec<IncomingEvent>) -> Result<AppendOutcome, Error> {
        if events.is_empty() || events.len() > hm_schema::protocol::MAXIMUM_BATCH_EVENTS {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let first_lsn = self.log.next_lsn().get();
        let mut verified_kinds = self.kinds.clone();
        let mut verified_authorities = self.authorities.clone();
        for (index, incoming) in events.iter().enumerate() {
            let kind = schema_kind(incoming.kind)?;
            let verified = event::verify_event_with_history(
                &incoming.payload,
                kind,
                Boundary::Socket,
                &ActorHistory::new(&verified_kinds, &verified_authorities),
            )
            .map_err(|error| error.at_lsn(LSN::new(first_lsn + index as u64)))?;
            verified_kinds.push(kind);
            verified_authorities.push(verified.envelope.authority);
        }
        let now = wall_time_ns()?;
        let mut entropy = OsEntropy;
        let mut plaintext = Vec::with_capacity(events.len());
        let mut sealed = Vec::with_capacity(events.len());
        for (index, incoming) in events.into_iter().enumerate() {
            let lsn = LSN::new(first_lsn + index as u64);
            let index =
                i64::try_from(index).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
            let time = now
                .max(self.last_wall_timestamp_ns.saturating_add(1))
                .saturating_add(index);
            let header = FrameHeader {
                lsn,
                kind: incoming.kind,
                wall_timestamp_ns: UtcNanos::new(time),
                actor: self.config.actor,
                conversation: incoming.conversation,
            };
            sealed.push(AppendRequest {
                kind: incoming.kind,
                wall_timestamp_ns: header.wall_timestamp_ns,
                conversation: incoming.conversation,
                sealed_payload: self.keys.seal(&header, &incoming.payload, &mut entropy)?,
            });
            plaintext.push(Frame {
                header,
                sealed_payload: incoming.payload,
            });
        }
        let commit = self.log.append_batch(&sealed)?;
        for (frame, sealed_event) in plaintext.iter().zip(&sealed) {
            self.mmr
                .append_sealed(&frame.header, &sealed_event.sealed_payload)?;
        }
        self.mmr.create_checkpoint(&self.signing_keys)?;
        self.plaintext_frames.extend(plaintext.iter().cloned());
        rebuild_projection_stream(&self.projections, &self.plaintext_frames, false, usize::MAX)?;
        for frame in &plaintext {
            self.applied.apply(frame)?;
        }
        self.kinds = verified_kinds;
        self.authorities = verified_authorities;
        self.last_wall_timestamp_ns = plaintext
            .last()
            .map_or(self.last_wall_timestamp_ns, |frame| {
                frame.header.wall_timestamp_ns.get()
            });
        let mmr = self.mmr.verification_status();
        Ok(AppendOutcome {
            first_lsn: commit.first_lsn,
            last_lsn: commit.last_lsn,
            duplicate: false,
            leaf_count: mmr.leaf_count,
            last_leaf_hash: self.mmr.leaf_hash(mmr.leaf_count - 1)?,
            mmr_root: mmr.root,
        })
    }

    fn append_idempotent(
        &mut self,
        connection_id: ConnectionId,
        client_seq: u64,
        events: Vec<IncomingEvent>,
    ) -> Result<AppendOutcome, Error> {
        if events.is_empty() || events.len() > hm_schema::protocol::MAXIMUM_BATCH_EVENTS {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let count =
            u32::try_from(events.len()).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
        let mut history_kinds = self.kinds.clone();
        let mut history_authorities = self.authorities.clone();
        let mut prepared = Vec::with_capacity(events.len());
        for (index, incoming) in events.into_iter().enumerate() {
            let kind = schema_kind(incoming.kind)?;
            let mut envelope = event::verify_event_with_history(
                &incoming.payload,
                kind,
                Boundary::Socket,
                &ActorHistory::new(&history_kinds, &history_authorities),
            )?
            .envelope;
            envelope.connection_id = Some(connection_id.to_vec());
            envelope.client_seq = client_seq;
            envelope.client_event_index =
                u32::try_from(index).map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
            envelope.client_event_count = count;
            prepared.push(IncomingEvent {
                kind: incoming.kind,
                conversation: incoming.conversation,
                payload: encode_event_envelope(&envelope),
            });
            history_kinds.push(kind);
            history_authorities.push(envelope.authority);
        }
        let batch_events: Vec<BatchEvent<'_>> = prepared
            .iter()
            .map(|incoming| BatchEvent {
                kind: incoming.kind,
                conversation: incoming.conversation,
                plaintext_payload: &incoming.payload,
            })
            .collect();
        let identity = BatchIdentity {
            connection_id,
            client_seq,
            events: &batch_events,
        };
        if let Admission::Duplicate(prior) = self.dedup.admit(&identity)? {
            let mmr = self.mmr.verification_status();
            return Ok(AppendOutcome {
                first_lsn: prior.first_lsn,
                last_lsn: prior.last_lsn,
                duplicate: true,
                leaf_count: mmr.leaf_count,
                last_leaf_hash: self.mmr.leaf_hash(prior.last_lsn.get() - 1)?,
                mmr_root: mmr.root,
            });
        }
        let outcome = self.append(prepared.clone())?;
        self.dedup
            .record(&identity, outcome.first_lsn, outcome.last_lsn)?;
        Ok(outcome)
    }

    fn write_checkpoint(
        &mut self,
        connection_id: ConnectionId,
        client_seq: u64,
        turn_id: &[u8],
        blob: &[u8],
    ) -> Result<CheckpointOutcome, Error> {
        let cursor = encode_checkpoint_cursor(turn_id, blob)?;
        let incoming = IncomingEvent {
            kind: EventKind::Checkpoint,
            conversation: turn_conversation(turn_id),
            payload: encode_event_envelope(&EventEnvelope {
                schema_version: CURRENT_SCHEMA_VERSION,
                payload: EventPayload::Checkpoint(Box::new(Checkpoint { cursor })),
                connection_id: None,
                client_seq: 0,
                client_event_index: 0,
                client_event_count: 0,
                origin_actor: self.config.actor.get(),
                run_id: None,
                model_provenance: None,
                authority: Authority::RuntimeFact,
                retention: Retention::CurrentState,
                sensitivity: Sensitivity::Personal,
                event_time_ns: 0,
            }),
        };
        let outcome = self.append_idempotent(connection_id, client_seq, vec![incoming])?;
        Ok(CheckpointOutcome {
            lsn: outcome.first_lsn,
            duplicate: outcome.duplicate,
        })
    }

    fn latest_checkpoint(&self, turn_id: &[u8]) -> Result<Option<CheckpointRead>, Error> {
        latest_checkpoint(&self.projections.begin_snapshot()?, turn_id)
    }

    fn frames_since(
        &self,
        since_lsn: LSN,
        conversation: Option<ConversationId>,
        maximum_frames: usize,
    ) -> Result<Vec<Frame>, Error> {
        if maximum_frames == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        Ok(self
            .plaintext_frames
            .iter()
            .filter(|frame| {
                frame.header.lsn.get() > since_lsn.get()
                    && conversation.is_none_or(|value| value == frame.header.conversation)
            })
            .take(maximum_frames)
            .cloned()
            .collect())
    }

    fn next_client_sequence(&self, connection_id: &ConnectionId) -> Result<u64, Error> {
        self.dedup.get(connection_id).map_or(Ok(1), |state| {
            state
                .client_seq
                .checked_add(1)
                .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))
        })
    }

    fn publish_from(&self, index: usize) {
        for frame in self.plaintext_frames.iter().skip(index) {
            let _ = self.events.send(frame.clone());
        }
    }

    fn recall(&self, request: RecallRequest) -> Result<Vec<RecallItem>, Error> {
        let snapshot = self.projections.begin_snapshot()?;
        match request {
            RecallRequest::Lexical { query, limit } => {
                LexicalProjection::query(&snapshot, &query, limit)?
                    .into_iter()
                    .filter_map(|hit| match read_conversation_record(&snapshot, hit.lsn) {
                        Ok(Some(record)) => Some(Ok(recall_item(record, hit.score_q32))),
                        Ok(None) => None,
                        Err(error) => Some(Err(error)),
                    })
                    .collect()
            }
            RecallRequest::Timeline {
                conversation,
                since_lsn,
                limit,
            } => Ok(read_conversation_records(
                &snapshot,
                conversation,
                bundle::MAXIMUM_CONVERSATION_RECORDS,
            )?
            .into_iter()
            .filter(|record| record.lsn.get() > since_lsn.get())
            .take(limit)
            .map(|record| recall_item(record, 0))
            .collect()),
        }
    }

    fn activate(&self, request: ActivateRequest) -> Result<ActivationBundle, Error> {
        let counter = TokenCounter::for_model("wire-fallback", None, request.token_weights)?;
        let snapshot = self.projections.begin_snapshot()?;
        let bundle = bundle::activate(
            &snapshot,
            &ActivationRequest {
                actor: self.config.actor,
                conversation: request.conversation,
                query: request.query,
                turn_text: request.turn_text,
                budget_tokens: request.budget_tokens,
                token_counter: &counter,
                maximum_candidates: bundle::MAXIMUM_CANDIDATES,
                maximum_conversation_records: bundle::MAXIMUM_CONVERSATION_RECORDS,
            },
        )?;
        self.tripwires.guard(
            bundle
                .sections
                .iter()
                .flat_map(|section| &section.items)
                .flat_map(|item| item.provenance.iter().copied()),
        )?;
        Ok(bundle)
    }

    fn integrity_at(&self, lsn: LSN) -> Result<IntegrityReceipt, Error> {
        if lsn.get() == 0 || lsn.get() > self.mmr.verification_status().leaf_count {
            return Err(Error::new(ErrorCode::InvalidArgument).at_lsn(lsn));
        }
        Ok(IntegrityReceipt {
            lsn,
            leaf_hash: self.mmr.leaf_hash(lsn.get() - 1)?,
            root: self.mmr.root_at(lsn.get())?,
            checkpoint_lsn: self.mmr.verification_status().last_checkpoint_lsn,
        })
    }

    fn rebuild_projection(&mut self, name: &str) -> Result<LSN, Error> {
        let projection = [
            ProjectionId::Bm25,
            ProjectionId::IntentFrame,
            ProjectionId::WorkLedger,
            ProjectionId::ConversationHeads,
            ProjectionId::Bindings,
        ]
        .into_iter()
        .find(|projection| projection.name() == name)
        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
        let progress =
            rebuild_projection_stream(&self.projections, &self.plaintext_frames, true, usize::MAX)?;
        if !progress.complete {
            return Err(Error::new(ErrorCode::ProjectionCheckpoint).at_lsn(progress.applied_lsn));
        }
        self.projections.begin_snapshot()?.checkpoint(projection)
    }

    fn rotate_keys(&mut self, new_kek: KeyEncryptionKey) -> Result<(), Error> {
        let mut entropy = OsEntropy;
        rotate_keys(
            &self.config.actor_directory,
            self.config.actor,
            self.config.user,
            &self.config.kek,
            &new_kek,
            &mut entropy,
        )?;
        self.config.kek = new_kek;
        Ok(())
    }

    fn crypto_delete(self) -> Result<Vec<u8>, Error> {
        let status = self.mmr.verification_status();
        if status.leaf_count == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let receipt = crypto_shred(
            self.keys,
            LSN::new(status.leaf_count),
            status.root,
            &self.signing_keys,
        )?;
        Ok(encode_deletion_receipt(&receipt).to_vec())
    }

    fn stats(&self) -> Result<ActorStats, Error> {
        let snapshot = self.projections.begin_snapshot()?;
        let mut projections = Vec::with_capacity(ProjectionId::COUNT);
        for projection in [
            ProjectionId::BeliefStore,
            ProjectionId::EntityIndex,
            ProjectionId::VectorLane,
            ProjectionId::Bm25,
            ProjectionId::TemporalLadder,
            ProjectionId::IntentFrame,
            ProjectionId::WorkLedger,
            ProjectionId::ConversationHeads,
            ProjectionId::Bindings,
        ] {
            projections.push(ProjectionStat {
                name: projection.name(),
                applied_lsn: snapshot.checkpoint(projection)?,
            });
        }
        Ok(ActorStats {
            actor: self.config.actor,
            log_events: self.applied.event_count,
            log_bytes: directory_bytes(&self.config.actor_directory.join("log"))?,
            projections,
            applied: self.applied,
        })
    }
}

fn recall_item(record: ConversationRecord, score_q32: u64) -> RecallItem {
    RecallItem {
        lsn: record.lsn,
        kind: record.kind,
        conversation: record.conversation,
        wall_timestamp_ns: record.wall_timestamp_ns,
        payload: record.payload,
        score_q32,
    }
}

struct ActorHistory<'a> {
    kinds: &'a [event::EventKind],
    authorities: &'a [Authority],
}

impl<'a> ActorHistory<'a> {
    const fn new(kinds: &'a [event::EventKind], authorities: &'a [Authority]) -> Self {
        Self { kinds, authorities }
    }

    fn position(lsn: LSN) -> Option<usize> {
        lsn.get()
            .checked_sub(1)
            .and_then(|position| usize::try_from(position).ok())
    }
}

impl event::EventHistory for ActorHistory<'_> {
    fn kind_at(&self, lsn: LSN) -> Option<event::EventKind> {
        Self::position(lsn).and_then(|position| self.kinds.get(position).copied())
    }

    fn authority_at(&self, lsn: LSN) -> Option<Authority> {
        Self::position(lsn).and_then(|position| self.authorities.get(position).copied())
    }
}

fn schema_kind(kind: EventKind) -> Result<event::EventKind, Error> {
    event::EventKind::try_from(kind as u8).map_err(|()| Error::new(ErrorCode::ForbiddenKind))
}

fn wall_time_ns() -> Result<i64, Error> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error::new(ErrorCode::InvariantViolation))?
        .as_nanos();
    i64::try_from(nanos).map_err(|_| Error::new(ErrorCode::CapacityExceeded))
}

fn directory_bytes(path: &Path) -> Result<u64, Error> {
    let mut total = 0_u64;
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(_) => return Err(Error::new(ErrorCode::ReadFailed)),
    };
    for entry in entries {
        let metadata = entry
            .map_err(|_| Error::new(ErrorCode::ReadFailed))?
            .metadata()
            .map_err(|_| Error::new(ErrorCode::ReadFailed))?;
        if metadata.is_file() {
            total = total
                .checked_add(metadata.len())
                .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?;
        }
    }
    Ok(total)
}
