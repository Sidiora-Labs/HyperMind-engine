#![allow(clippy::missing_errors_doc)]

use hm_compose::bundle::{self, ActivationBundle, ActivationRequest};
use hm_compose::tokens::{FallbackWeights, TokenCounter};
use hm_core::{ActorId, ConversationId, Error, ErrorCode, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_ledger::keyring::{KeyEncryptionKey, KeyHierarchy, OsEntropy, UserId};
use hm_ledger::segment::{AppendRequest, SegmentLog, SegmentLogOptions};
use hm_proj::lexical::LexicalProjection;
use hm_proj::rebuild::rebuild_projection_stream;
use hm_proj::store::{ProjectionId, ProjectionStore};
use hm_proj::timeline::{ConversationRecord, read_conversation_record, read_conversation_records};
use hm_schema::event::{self, Boundary};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, oneshot};

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
}

enum Command {
    Append(
        Vec<IncomingEvent>,
        oneshot::Sender<Result<AppendOutcome, Error>>,
    ),
    Recall(
        RecallRequest,
        oneshot::Sender<Result<Vec<RecallItem>, Error>>,
    ),
    Activate(
        Box<ActivateRequest>,
        oneshot::Sender<Result<ActivationBundle, Error>>,
    ),
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
    applied: AppliedState,
    last_wall_timestamp_ns: i64,
}

impl ActorEngine {
    pub async fn open(config: ActorConfig) -> Result<Self, Error> {
        if config.actor.get() == 0 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let actor = config.actor;
        let (commands, receiver) = mpsc::channel(COMMAND_QUEUE);
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
                    match WriterState::open(config) {
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
        Ok(Self { actor, commands })
    }

    #[must_use]
    pub const fn actor(&self) -> ActorId {
        self.actor
    }

    pub async fn append(&self, events: Vec<IncomingEvent>) -> Result<AppendOutcome, Error> {
        request(&self.commands, |reply| Command::Append(events, reply)).await
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
                let _ = reply.send(state.append(events));
            }
            Command::Recall(request, reply) => {
                let _ = reply.send(state.recall(request));
            }
            Command::Activate(request, reply) => {
                let _ = reply.send(state.activate(*request));
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
    fn open(config: ActorConfig) -> Result<Self, Error> {
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
        let log = SegmentLog::open(
            &config.actor_directory,
            config.actor,
            SegmentLogOptions::default(),
        )?;
        let projections =
            ProjectionStore::open(&config.actor_directory, config.projection_map_bytes)?;
        let mut kinds = Vec::new();
        let mut applied = AppliedState::default();
        let mut plaintext_frames = Vec::new();
        let mut last_wall_timestamp_ns = i64::MIN;
        for mut frame in log.read_all()? {
            frame.sealed_payload = keys.unseal(&frame.header, &frame.sealed_payload)?;
            let kind = schema_kind(frame.header.kind)?;
            event::verify_event_with_history(
                &frame.sealed_payload,
                kind,
                Boundary::Disk,
                &|lsn: LSN| {
                    lsn.get()
                        .checked_sub(1)
                        .and_then(|index| usize::try_from(index).ok())
                        .and_then(|index| kinds.get(index))
                        .copied()
                },
            )?;
            applied.apply(&frame)?;
            kinds.push(kind);
            last_wall_timestamp_ns =
                last_wall_timestamp_ns.max(frame.header.wall_timestamp_ns.get());
            plaintext_frames.push(frame);
        }
        let rebuilt =
            rebuild_projection_stream(&projections, &plaintext_frames, false, usize::MAX)?;
        if !rebuilt.complete {
            return Err(Error::new(ErrorCode::ProjectionCheckpoint));
        }
        Ok(Self {
            config,
            log,
            keys,
            projections,
            plaintext_frames,
            kinds,
            applied,
            last_wall_timestamp_ns,
        })
    }

    fn append(&mut self, events: Vec<IncomingEvent>) -> Result<AppendOutcome, Error> {
        if events.is_empty() || events.len() > hm_schema::protocol::MAXIMUM_BATCH_EVENTS {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let first_lsn = self.log.next_lsn().get();
        let mut verified_kinds = self.kinds.clone();
        for (index, incoming) in events.iter().enumerate() {
            let kind = schema_kind(incoming.kind)?;
            event::verify_event_with_history(
                &incoming.payload,
                kind,
                Boundary::Socket,
                &|lsn: LSN| {
                    lsn.get()
                        .checked_sub(1)
                        .and_then(|position| usize::try_from(position).ok())
                        .and_then(|position| verified_kinds.get(position))
                        .copied()
                },
            )
            .map_err(|error| error.at_lsn(LSN::new(first_lsn + index as u64)))?;
            verified_kinds.push(kind);
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
        self.plaintext_frames.extend(plaintext.iter().cloned());
        rebuild_projection_stream(&self.projections, &self.plaintext_frames, false, usize::MAX)?;
        for frame in &plaintext {
            self.applied.apply(frame)?;
        }
        self.kinds = verified_kinds;
        self.last_wall_timestamp_ns = plaintext
            .last()
            .map_or(self.last_wall_timestamp_ns, |frame| {
                frame.header.wall_timestamp_ns.get()
            });
        Ok(AppendOutcome {
            first_lsn: commit.first_lsn,
            last_lsn: commit.last_lsn,
        })
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
        bundle::activate(
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
        )
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
