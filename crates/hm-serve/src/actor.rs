#![allow(clippy::missing_errors_doc)]

use crate::anticipation::{self, WakeDecision, WakeEvaluation};
use hm_compose::bundle::{self, ActivationBundle, ActivationRequest};
use hm_compose::lanes::relation::{self, RelationHit};
use hm_compose::preference::{PreferenceProfile, adjust_q32};
use hm_compose::tokens::{FallbackWeights, TokenCounter};
use hm_core::telemetry::{Attribute, SpanBuilder, SpanKind, SpanOutcome};
use hm_core::{ActorId, ConversationId, Error, ErrorCode, LSN, UtcNanos};
use hm_cortex::attention::AttentionFactors;
use hm_cortex::connectors::{
    ConsentGrant, DEFAULT_FRESHNESS_WINDOW_NS, DeliveryEnvelope, VerifiedDelivery,
    mint_consent_state, verify_consent_state, verify_delivery,
};
use hm_ledger::checkpoint::{SigningKeyPair, signing_key_pair_for};
use hm_ledger::credentials::{CredentialVault, consent_key};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_ledger::idempotency::{
    Admission, BatchEvent, BatchIdentity, ConnectionId, DedupTable, rollback_torn_batch,
};
use hm_ledger::keyring::EntropySource;
use hm_ledger::keyring::{KeyEncryptionKey, KeyHierarchy, OsEntropy, UserId};
use hm_ledger::mmr::Hash as MmrHash;
use hm_ledger::mmr_store::{MmrStore, VerificationStatus};
use hm_ledger::rotate::rotate_keys;
use hm_ledger::segment::{AppendRequest, SegmentLog, SegmentLogOptions};
use hm_ledger::shred::{crypto_shred, encode_deletion_receipt};
use hm_ledger::tripwire::TripwireSet;
use hm_proj::attention::{AttentionProjection, AttentionRecord};
use hm_proj::attestations::{
    AttestationsProjection, MAXIMUM_PREFERENCE_TARGETS, PREFERENCE_NEUTRAL_Q16, PreferenceWeight,
};
use hm_proj::beliefs::{BeliefAsOf, BeliefAsOfResult, BeliefProjection};
use hm_proj::checkpoint::{
    CheckpointRead, encode_checkpoint_cursor, latest_checkpoint, turn_conversation,
};
use hm_proj::connectors::{
    ConnectorRecord, ConnectorRegistryProjection, DeliveryRecord, RevisionRecord,
};
use hm_proj::documents::{ChunkRecord, DocumentRecord, DocumentsProjection, ExtractionRecord};
use hm_proj::entities::EntityProjection;
use hm_proj::graph::{EdgeRecord, GraphProjection};
use hm_proj::intentions::{IntentionRecord, IntentionStatus, IntentionsProjection};
use hm_proj::lexical::LexicalProjection;
use hm_proj::media::{MediaCatalogProjection, MediaRecord};
use hm_proj::memories::{MemoryProjection, MemoryRecord};
use hm_proj::predictions::{
    CalibrationCounters, MechanismFailures, PredictionRecord, PredictionsProjection,
};
use hm_proj::procedures::{ProcedureRecord, ProcedureState, ProceduresProjection};
use hm_proj::rebuild::rebuild_projection_stream;
use hm_proj::runs::RunsProjection;
use hm_proj::store::{ProjectionId, ProjectionStore, ReadSnapshot};
use hm_proj::timeline::{ConversationRecord, read_conversation_record, read_conversation_records};
use hm_proj::vectors::{VectorEntry, VectorLane};
use hm_proj::vocabulary::{AliasProposal, VocabularyProjection, VocabularyRecord};
use hm_schema::event::{self, Boundary, CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    Authority, BeliefType, Checkpoint, ConnectorState, EventEnvelope, EventPayload, IntentionSet,
    OutcomeAssessment, PredicateKind, ProcedureRevised, Retention, Sensitivity,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, mpsc, oneshot};

const COMMAND_QUEUE: usize = 256;

pub const MAXIMUM_GRAPH_NEIGHBOURS: usize = 256;

const MAXIMUM_MEDIA_CATALOG_ROWS: usize = 256;

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
    pub preference_q16: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationResult {
    pub edge_id: Vec<u8>,
    pub relation: String,
    pub source_id: Vec<u8>,
    pub target_id: Vec<u8>,
    pub event_lsn: LSN,
    pub weight_micros: u32,
    pub support_lsns: Vec<LSN>,
    pub score_q32: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphNeighbour {
    pub edge: EdgeRecord,
    pub outgoing: bool,
    pub endpoint: Option<MemoryRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphNeighbourhood {
    pub generation: u64,
    pub node: Option<MemoryRecord>,
    pub neighbours: Vec<GraphNeighbour>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentState {
    pub generation: u64,
    pub document: DocumentRecord,
    pub extraction: Option<ExtractionRecord>,
    pub chunks: Vec<ChunkRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceSignatureRequest {
    pub connector_id: [u8; 16],
    pub provider: String,
    pub credential_version: u32,
    pub delivery_id: Vec<u8>,
    pub event_name: String,
    pub signed_at_ns: i64,
    pub body: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConsentMint {
    pub state: String,
    pub nonce: [u8; 16],
    pub expires_at_ns: i64,
}

#[derive(Clone, Debug)]
pub enum RecallRequest {
    Vector {
        space_id: String,
        query: Vec<i8>,
        binary_prefilter: Vec<u8>,
        limit: usize,
    },
    Semantic {
        query: String,
        limit: usize,
    },
    Lexical {
        query: String,
        limit: usize,
    },
    Entity {
        query: String,
        turn_text: String,
        limit: usize,
    },
    Near {
        anchor: String,
        query: String,
        turn_text: String,
        limit: usize,
    },
    Temporal {
        start_ns: i64,
        end_ns: i64,
        limit: usize,
    },
    Timeline {
        conversation: ConversationId,
        since_lsn: LSN,
        limit: usize,
    },
    Relation {
        space_id: String,
        query: Vec<i8>,
        binary_prefilter: Vec<u8>,
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
    VerifiedEvent(LSN, oneshot::Sender<Result<event::VerifiedEvent, Error>>),
    EvaluateWake(
        LSN,
        AttentionFactors,
        Option<i64>,
        oneshot::Sender<Result<WakeEvaluation, Error>>,
    ),
    Intention(
        Vec<u8>,
        oneshot::Sender<Result<Option<IntentionRecord>, Error>>,
    ),
    Prediction(
        Vec<u8>,
        oneshot::Sender<Result<Option<PredictionRecord>, Error>>,
    ),
    AttentionHistory(usize, oneshot::Sender<Result<Vec<AttentionRecord>, Error>>),
    Preferences(usize, oneshot::Sender<Result<Vec<PreferenceWeight>, Error>>),
    Calibration(oneshot::Sender<Result<Vec<(PredicateKind, CalibrationCounters)>, Error>>),
    Vocabularies(usize, oneshot::Sender<Result<Vec<VocabularyRecord>, Error>>),
    AliasProposals(
        String,
        u32,
        usize,
        oneshot::Sender<Result<Vec<AliasProposal>, Error>>,
    ),
    MechanismFailures(String, oneshot::Sender<Result<MechanismFailures, Error>>),
    Procedure(
        Vec<u8>,
        oneshot::Sender<Result<Option<ProcedureRecord>, Error>>,
    ),
    Procedures(usize, oneshot::Sender<Result<Vec<ProcedureRecord>, Error>>),
    MediaCatalog(
        bool,
        usize,
        oneshot::Sender<Result<Vec<MediaRecord>, Error>>,
    ),
    GraphNeighbourhood(
        Vec<u8>,
        i64,
        usize,
        oneshot::Sender<Result<GraphNeighbourhood, Error>>,
    ),
    Memories(usize, oneshot::Sender<Result<Vec<MemoryRecord>, Error>>),
    DocumentState(
        Vec<u8>,
        oneshot::Sender<Result<Option<DocumentState>, Error>>,
    ),
    Relations(usize, oneshot::Sender<Result<Vec<EdgeRecord>, Error>>),
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
    RelationRecall(
        RecallRequest,
        oneshot::Sender<Result<Vec<RelationResult>, Error>>,
    ),
    Activate(
        Box<ActivateRequest>,
        oneshot::Sender<Result<ActivationBundle, Error>>,
    ),
    AsOf(
        BeliefType,
        String,
        BeliefAsOf,
        oneshot::Sender<Result<BeliefAsOfResult, Error>>,
    ),
    VerificationStatus(oneshot::Sender<Result<VerificationStatus, Error>>),
    IntegrityAt(LSN, oneshot::Sender<Result<IntegrityReceipt, Error>>),
    RebuildProjection(String, oneshot::Sender<Result<LSN, Error>>),
    RotateKeys(KeyEncryptionKey, oneshot::Sender<Result<(), Error>>),
    GuardTripwires(Vec<LSN>, oneshot::Sender<Result<(), Error>>),
    CryptoDelete(oneshot::Sender<Result<Vec<u8>, Error>>),
    Stats(oneshot::Sender<Result<ActorStats, Error>>),
    StoreConnectorCredential(
        String,
        [u8; 16],
        u32,
        Vec<u8>,
        oneshot::Sender<Result<(), Error>>,
    ),
    VerifySourceDelivery(
        Box<SourceSignatureRequest>,
        oneshot::Sender<Result<VerifiedDelivery, Error>>,
    ),
    MintConsentState(
        String,
        [u8; 16],
        i64,
        oneshot::Sender<Result<ConsentMint, Error>>,
    ),
    RedeemConsentState(
        String,
        [u8; 16],
        String,
        oneshot::Sender<Result<ConsentGrant, Error>>,
    ),
    Connectors(usize, oneshot::Sender<Result<Vec<ConnectorRecord>, Error>>),
    SourceDeliveries(
        [u8; 16],
        usize,
        oneshot::Sender<Result<Vec<DeliveryRecord>, Error>>,
    ),
    SourceRevisions(
        [u8; 16],
        usize,
        oneshot::Sender<Result<Vec<RevisionRecord>, Error>>,
    ),
    Shutdown(oneshot::Sender<()>),
}

struct WriterState {
    config: ActorConfig,
    log: SegmentLog,
    keys: KeyHierarchy,
    credentials: CredentialVault,
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
    vector_lanes: BTreeMap<String, VectorLane>,
    vector_digests: BTreeMap<(String, u64), [u8; 32]>,
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
        let span = ingestion_span(self.actor, events.len());
        let outcome = request(&self.commands, |reply| Command::Append(events, reply)).await;
        finish_ingestion(span, outcome.is_ok());
        outcome
    }

    pub async fn append_idempotent(
        &self,
        connection_id: ConnectionId,
        client_seq: u64,
        events: Vec<IncomingEvent>,
    ) -> Result<AppendOutcome, Error> {
        let span = ingestion_span(self.actor, events.len());
        let outcome = request(&self.commands, |reply| {
            Command::IdempotentAppend(connection_id, client_seq, events, reply)
        })
        .await;
        finish_ingestion(span, outcome.is_ok());
        outcome
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

    pub async fn relation_recall(
        &self,
        request_value: RecallRequest,
    ) -> Result<Vec<RelationResult>, Error> {
        request(&self.commands, |reply| {
            Command::RelationRecall(request_value, reply)
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

    pub async fn as_of(
        &self,
        belief_type: BeliefType,
        canonical_identity: String,
        as_of: BeliefAsOf,
    ) -> Result<BeliefAsOfResult, Error> {
        request(&self.commands, |reply| {
            Command::AsOf(belief_type, canonical_identity, as_of, reply)
        })
        .await
    }

    pub async fn relations(&self, limit: usize) -> Result<Vec<EdgeRecord>, Error> {
        request(&self.commands, |reply| Command::Relations(limit, reply)).await
    }

    pub async fn stats(&self) -> Result<ActorStats, Error> {
        request(&self.commands, Command::Stats).await
    }

    pub async fn evaluate_wake(
        &self,
        observation_lsn: LSN,
        factors: AttentionFactors,
        rearm_at_ns: Option<i64>,
    ) -> Result<WakeEvaluation, Error> {
        request(&self.commands, |reply| {
            Command::EvaluateWake(observation_lsn, factors, rearm_at_ns, reply)
        })
        .await
    }

    pub async fn verified_event(&self, lsn: LSN) -> Result<event::VerifiedEvent, Error> {
        request(&self.commands, |reply| Command::VerifiedEvent(lsn, reply)).await
    }

    pub async fn intention(&self, id: Vec<u8>) -> Result<Option<IntentionRecord>, Error> {
        request(&self.commands, |reply| Command::Intention(id, reply)).await
    }

    pub async fn prediction(&self, id: Vec<u8>) -> Result<Option<PredictionRecord>, Error> {
        request(&self.commands, |reply| Command::Prediction(id, reply)).await
    }

    pub async fn attention_history(&self, limit: usize) -> Result<Vec<AttentionRecord>, Error> {
        request(&self.commands, |reply| {
            Command::AttentionHistory(limit, reply)
        })
        .await
    }

    pub async fn preferences(&self, limit: usize) -> Result<Vec<PreferenceWeight>, Error> {
        request(&self.commands, |reply| Command::Preferences(limit, reply)).await
    }

    pub async fn calibration(&self) -> Result<Vec<(PredicateKind, CalibrationCounters)>, Error> {
        request(&self.commands, Command::Calibration).await
    }

    pub async fn vocabularies(&self, limit: usize) -> Result<Vec<VocabularyRecord>, Error> {
        request(&self.commands, |reply| Command::Vocabularies(limit, reply)).await
    }

    pub async fn alias_proposals(
        &self,
        observed_name: String,
        threshold_q16: u32,
        limit: usize,
    ) -> Result<Vec<AliasProposal>, Error> {
        request(&self.commands, |reply| {
            Command::AliasProposals(observed_name, threshold_q16, limit, reply)
        })
        .await
    }

    pub async fn mechanism_failures(&self, mechanism: String) -> Result<MechanismFailures, Error> {
        request(&self.commands, |reply| {
            Command::MechanismFailures(mechanism, reply)
        })
        .await
    }

    pub async fn procedure(&self, id: Vec<u8>) -> Result<Option<ProcedureRecord>, Error> {
        request(&self.commands, |reply| Command::Procedure(id, reply)).await
    }

    pub async fn procedures(&self, limit: usize) -> Result<Vec<ProcedureRecord>, Error> {
        request(&self.commands, |reply| Command::Procedures(limit, reply)).await
    }

    pub async fn media_catalog(
        &self,
        pending_only: bool,
        limit: usize,
    ) -> Result<Vec<MediaRecord>, Error> {
        request(&self.commands, |reply| {
            Command::MediaCatalog(pending_only, limit, reply)
        })
        .await
    }

    pub async fn graph_neighbourhood(
        &self,
        node_id: Vec<u8>,
        valid_at_ns: i64,
        limit: usize,
    ) -> Result<GraphNeighbourhood, Error> {
        request(&self.commands, |reply| {
            Command::GraphNeighbourhood(node_id, valid_at_ns, limit, reply)
        })
        .await
    }

    pub async fn memories(&self, limit: usize) -> Result<Vec<MemoryRecord>, Error> {
        request(&self.commands, |reply| Command::Memories(limit, reply)).await
    }

    pub async fn document_state(
        &self,
        document_id: Vec<u8>,
    ) -> Result<Option<DocumentState>, Error> {
        request(&self.commands, |reply| {
            Command::DocumentState(document_id, reply)
        })
        .await
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

    pub async fn store_connector_credential(
        &self,
        provider: String,
        connector_id: [u8; 16],
        version: u32,
        secret: Vec<u8>,
    ) -> Result<(), Error> {
        request(&self.commands, |reply| {
            Command::StoreConnectorCredential(provider, connector_id, version, secret, reply)
        })
        .await
    }

    pub async fn verify_source_delivery(
        &self,
        request_value: SourceSignatureRequest,
    ) -> Result<VerifiedDelivery, Error> {
        request(&self.commands, |reply| {
            Command::VerifySourceDelivery(Box::new(request_value), reply)
        })
        .await
    }

    pub async fn mint_consent_state(
        &self,
        provider: String,
        connector_id: [u8; 16],
        ttl_ns: i64,
    ) -> Result<ConsentMint, Error> {
        request(&self.commands, |reply| {
            Command::MintConsentState(provider, connector_id, ttl_ns, reply)
        })
        .await
    }

    pub async fn redeem_consent_state(
        &self,
        provider: String,
        connector_id: [u8; 16],
        state: String,
    ) -> Result<ConsentGrant, Error> {
        request(&self.commands, |reply| {
            Command::RedeemConsentState(provider, connector_id, state, reply)
        })
        .await
    }

    pub async fn connectors(&self, limit: usize) -> Result<Vec<ConnectorRecord>, Error> {
        request(&self.commands, |reply| Command::Connectors(limit, reply)).await
    }

    pub async fn source_deliveries(
        &self,
        connector_id: [u8; 16],
        limit: usize,
    ) -> Result<Vec<DeliveryRecord>, Error> {
        request(&self.commands, |reply| {
            Command::SourceDeliveries(connector_id, limit, reply)
        })
        .await
    }

    pub async fn source_revisions(
        &self,
        connector_id: [u8; 16],
        limit: usize,
    ) -> Result<Vec<RevisionRecord>, Error> {
        request(&self.commands, |reply| {
            Command::SourceRevisions(connector_id, limit, reply)
        })
        .await
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

fn ingestion_span(actor: ActorId, events: usize) -> Option<SpanBuilder> {
    let mut span = hm_core::telemetry::start_span(SpanKind::Ingestion, "hypermind.ingestion")?;
    span.attribute(Attribute::Integer(
        "hypermind.actor",
        i64::from(actor.get()),
    ));
    span.attribute(Attribute::Integer(
        "hypermind.ingestion.events",
        i64::try_from(events).unwrap_or(i64::MAX),
    ));
    Some(span)
}

fn finish_ingestion(span: Option<SpanBuilder>, appended: bool) {
    if let Some(span) = span {
        span.finish(if appended {
            SpanOutcome::Ok
        } else {
            SpanOutcome::Error
        });
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

fn document_state(
    snapshot: &ReadSnapshot<'_>,
    document_id: &[u8],
) -> Result<Option<DocumentState>, Error> {
    let Some(document) = DocumentsProjection::document(snapshot, document_id)? else {
        return Ok(None);
    };
    let generation = RunsProjection::active_generation(snapshot)?;
    let extraction = match DocumentsProjection::extraction(snapshot, generation, document_id) {
        Ok(record) => record,
        Err(error) if error.code == ErrorCode::OperationUnavailable => None,
        Err(error) => return Err(error),
    };
    let chunks = match DocumentsProjection::chunks(snapshot, generation, document_id) {
        Ok(records) => records,
        Err(error) if error.code == ErrorCode::OperationUnavailable => Vec::new(),
        Err(error) => return Err(error),
    };
    Ok(Some(DocumentState {
        generation,
        document,
        extraction,
        chunks,
    }))
}

fn neighbourhood(
    snapshot: &ReadSnapshot<'_>,
    node_id: &[u8],
    valid_at_ns: i64,
    limit: usize,
) -> Result<GraphNeighbourhood, Error> {
    let generation = RunsProjection::active_generation(snapshot)?;
    let node = MemoryProjection::get_visible(snapshot, generation, node_id)?;
    let edges = GraphProjection::neighbours(snapshot, generation, node_id, valid_at_ns, limit)?;
    let mut neighbours = Vec::with_capacity(edges.len());
    for edge in edges {
        let outgoing = edge.source_id == node_id;
        let endpoint = if outgoing {
            &edge.target_id
        } else {
            &edge.source_id
        };
        let endpoint = MemoryProjection::get_visible(snapshot, generation, endpoint)?;
        neighbours.push(GraphNeighbour {
            edge,
            outgoing,
            endpoint,
        });
    }
    Ok(GraphNeighbourhood {
        generation,
        node,
        neighbours,
    })
}

#[allow(clippy::too_many_lines)]
async fn writer_loop(mut state: WriterState, mut commands: mpsc::Receiver<Command>) {
    while let Some(command) = commands.recv().await {
        match command {
            Command::VerifiedEvent(lsn, reply) => {
                let result = state.verified_event(lsn);
                let _ = reply.send(result);
            }
            Command::EvaluateWake(lsn, factors, rearm_at, reply) => {
                let before = state.plaintext_frames.len();
                let result = state.evaluate_wake(lsn, factors, rearm_at);
                state.publish_from(before);
                let _ = reply.send(result);
            }
            Command::Intention(id, reply) => {
                let result = state
                    .projections
                    .begin_snapshot()
                    .and_then(|snapshot| IntentionsProjection::get(&snapshot, &id));
                let _ = reply.send(result);
            }
            Command::Prediction(id, reply) => {
                let result = state
                    .projections
                    .begin_snapshot()
                    .and_then(|snapshot| PredictionsProjection::get(&snapshot, &id));
                let _ = reply.send(result);
            }
            Command::AttentionHistory(limit, reply) => {
                let result = state
                    .projections
                    .begin_snapshot()
                    .and_then(|snapshot| AttentionProjection::recent(&snapshot, limit));
                let _ = reply.send(result);
            }
            Command::Preferences(limit, reply) => {
                let result = state.projections.begin_snapshot().and_then(|snapshot| {
                    AttestationsProjection::recent_preferences(&snapshot, limit)
                });
                let _ = reply.send(result);
            }
            Command::Calibration(reply) => {
                let result = state.projections.begin_snapshot().and_then(|snapshot| {
                    [
                        PredicateKind::ObjectExists,
                        PredicateKind::RevisionEquals,
                        PredicateKind::DigestEquals,
                        PredicateKind::ReceiptMatches,
                        PredicateKind::PropertySatisfies,
                        PredicateKind::ProcessTerminated,
                        PredicateKind::AnswerCommitted,
                    ]
                    .into_iter()
                    .map(|kind| {
                        PredictionsProjection::calibration(&snapshot, kind)
                            .map(|counters| (kind, counters))
                    })
                    .collect()
                });
                let _ = reply.send(result);
            }
            Command::Vocabularies(limit, reply) => {
                let result = state
                    .projections
                    .begin_snapshot()
                    .and_then(|snapshot| VocabularyProjection::list(&snapshot, limit));
                let _ = reply.send(result);
            }
            Command::AliasProposals(observed_name, threshold_q16, limit, reply) => {
                let result = state.projections.begin_snapshot().and_then(|snapshot| {
                    VocabularyProjection::suggest(&snapshot, &observed_name, threshold_q16, limit)
                });
                let _ = reply.send(result);
            }
            Command::MechanismFailures(mechanism, reply) => {
                let result = state.projections.begin_snapshot().and_then(|snapshot| {
                    PredictionsProjection::mechanism_failures(&snapshot, &mechanism)
                });
                let _ = reply.send(result);
            }
            Command::Procedure(id, reply) => {
                let result = state
                    .projections
                    .begin_snapshot()
                    .and_then(|snapshot| ProceduresProjection::get(&snapshot, &id));
                let _ = reply.send(result);
            }
            Command::Procedures(limit, reply) => {
                let result = state
                    .projections
                    .begin_snapshot()
                    .and_then(|snapshot| ProceduresProjection::list(&snapshot, limit));
                let _ = reply.send(result);
            }
            Command::MediaCatalog(pending_only, limit, reply) => {
                let result = if limit == 0 {
                    Err(Error::new(ErrorCode::InvalidArgument))
                } else {
                    let limit = limit.min(MAXIMUM_MEDIA_CATALOG_ROWS);
                    state.projections.begin_snapshot().and_then(|snapshot| {
                        if pending_only {
                            MediaCatalogProjection::pending(&snapshot, limit)
                        } else {
                            MediaCatalogProjection::list(&snapshot, limit)
                        }
                    })
                };
                let _ = reply.send(result);
            }
            Command::GraphNeighbourhood(node_id, valid_at_ns, limit, reply) => {
                let result =
                    if node_id.is_empty() || limit == 0 || limit > MAXIMUM_GRAPH_NEIGHBOURS {
                        Err(Error::new(ErrorCode::InvalidArgument))
                    } else {
                        state.projections.begin_snapshot().and_then(|snapshot| {
                            neighbourhood(&snapshot, &node_id, valid_at_ns, limit)
                        })
                    };
                let _ = reply.send(result);
            }
            Command::Memories(limit, reply) => {
                let result = if limit == 0 || limit > MAXIMUM_GRAPH_NEIGHBOURS {
                    Err(Error::new(ErrorCode::InvalidArgument))
                } else {
                    state.projections.begin_snapshot().and_then(|snapshot| {
                        let generation = RunsProjection::active_generation(&snapshot)?;
                        MemoryProjection::list_visible(&snapshot, generation, limit)
                    })
                };
                let _ = reply.send(result);
            }
            Command::DocumentState(document_id, reply) => {
                let result = if document_id.is_empty() {
                    Err(Error::new(ErrorCode::InvalidArgument))
                } else {
                    state
                        .projections
                        .begin_snapshot()
                        .and_then(|snapshot| document_state(&snapshot, &document_id))
                };
                let _ = reply.send(result);
            }
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
            Command::RelationRecall(request, reply) => {
                let _ = reply.send(state.relation_recall(request));
            }
            Command::Activate(request, reply) => {
                let _ = reply.send(state.activate(*request));
            }
            Command::AsOf(belief_type, canonical_identity, as_of, reply) => {
                let result = state.as_of(belief_type, &canonical_identity, as_of);
                let _ = reply.send(result);
            }
            Command::Relations(limit, reply) => {
                let _ = reply.send(state.relations(limit));
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
            Command::StoreConnectorCredential(provider, connector_id, version, secret, reply) => {
                let result =
                    state.store_connector_credential(&provider, &connector_id, version, &secret);
                let _ = reply.send(result);
            }
            Command::VerifySourceDelivery(request, reply) => {
                let _ = reply.send(state.verify_source_delivery(&request));
            }
            Command::MintConsentState(provider, connector_id, ttl_ns, reply) => {
                let _ = reply.send(state.mint_source_consent(&provider, &connector_id, ttl_ns));
            }
            Command::RedeemConsentState(provider, connector_id, encoded, reply) => {
                let _ = reply.send(state.redeem_source_consent(&provider, &connector_id, &encoded));
            }
            Command::Connectors(limit, reply) => {
                let result = state.projections.begin_snapshot().and_then(|snapshot| {
                    ConnectorRegistryProjection::list_connectors(&snapshot, limit)
                });
                let _ = reply.send(result);
            }
            Command::SourceDeliveries(connector_id, limit, reply) => {
                let result = state.projections.begin_snapshot().and_then(|snapshot| {
                    ConnectorRegistryProjection::recent_deliveries(&snapshot, &connector_id, limit)
                });
                let _ = reply.send(result);
            }
            Command::SourceRevisions(connector_id, limit, reply) => {
                let result = state.projections.begin_snapshot().and_then(|snapshot| {
                    ConnectorRegistryProjection::recent_revisions(&snapshot, &connector_id, limit)
                });
                let _ = reply.send(result);
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
    fn verified_event(&self, lsn: LSN) -> Result<event::VerifiedEvent, Error> {
        self.tripwires.guard([lsn])?;
        let index = usize::try_from(lsn.get())
            .ok()
            .and_then(|lsn| lsn.checked_sub(1))
            .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
        let frame = self
            .plaintext_frames
            .get(index)
            .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
        event::verify_event_with_history(
            &frame.sealed_payload,
            schema_kind(frame.header.kind)?,
            Boundary::Disk,
            &ActorHistory::new(&self.kinds[..index], &self.authorities[..index]),
        )
    }

    fn validate_anticipation(&self, payload: &EventPayload) -> Result<(), Error> {
        let snapshot = self.projections.begin_snapshot()?;
        match payload {
            EventPayload::IntentionSet(value) => {
                if IntentionsProjection::get(&snapshot, &value.intention_id)?.is_some() {
                    return Err(Error::new(ErrorCode::AlreadyExists));
                }
            }
            EventPayload::IntentionCancelled(value) => {
                if IntentionsProjection::get(&snapshot, &value.intention_id)?
                    .is_none_or(|record| record.status == IntentionStatus::Cancelled)
                {
                    return Err(Error::new(ErrorCode::OrderingViolation));
                }
            }
            EventPayload::Predicted(value) => {
                let prior = PredictionsProjection::get(&snapshot, &value.prediction_id)?;
                if prior.map_or(value.revision != 1, |prior| {
                    prior.revision.checked_add(1) != Some(value.revision)
                }) {
                    return Err(Error::new(ErrorCode::OrderingViolation));
                }
            }
            EventPayload::OutcomeObserved(value) => {
                let prior = PredictionsProjection::get(&snapshot, &value.prediction_id)?
                    .ok_or_else(|| Error::new(ErrorCode::OrderingViolation))?;
                if prior.revision != value.revision
                    || prior
                        .assessment
                        .is_some_and(|assessment| assessment != OutcomeAssessment::Pending)
                {
                    return Err(Error::new(ErrorCode::IdempotencyConflict));
                }
            }
            EventPayload::ProcedureMined(value) => {
                if ProceduresProjection::get(&snapshot, &value.procedure_id)?.is_some() {
                    return Err(Error::new(ErrorCode::AlreadyExists));
                }
            }
            EventPayload::ProcedureRevised(value) => {
                if ProceduresProjection::get(&snapshot, &value.procedure_id)?.is_none_or(|prior| {
                    prior.version_lsn != value.previous_lsn
                        || prior.state == ProcedureState::Adopted
                }) {
                    return Err(Error::new(ErrorCode::IdempotencyConflict));
                }
            }
            EventPayload::ProcedureAdopted(value) => {
                if ProceduresProjection::get(&snapshot, &value.procedure_id)?.is_none_or(|prior| {
                    prior.version_lsn != value.procedure_lsn
                        || prior.state != ProcedureState::Supported
                }) {
                    return Err(Error::new(ErrorCode::OrderingViolation));
                }
            }
            EventPayload::SourceConnectorBound(value) => {
                if ConnectorRegistryProjection::consent_redeemed(&snapshot, &value.consent_nonce)? {
                    return Err(Error::new(ErrorCode::IdempotencyConflict));
                }
            }
            EventPayload::SourceDeliveryAccepted(value) => {
                if !connector_is_bound(&snapshot, &value.connector_id)? {
                    return Err(Error::new(ErrorCode::CapabilityDenied));
                }
                if ConnectorRegistryProjection::delivery(
                    &snapshot,
                    &value.connector_id,
                    &value.delivery_id,
                )?
                .is_some()
                {
                    return Err(Error::new(ErrorCode::IdempotencyConflict));
                }
            }
            EventPayload::SourceDeliverySettled(value) => {
                let prior = ConnectorRegistryProjection::delivery(
                    &snapshot,
                    &value.connector_id,
                    &value.delivery_id,
                )?
                .ok_or_else(|| Error::new(ErrorCode::OrderingViolation))?;
                if value.attempt <= prior.attempt {
                    return Err(Error::new(ErrorCode::OrderingViolation));
                }
            }
            EventPayload::SourceRevisionObserved(value) => {
                if !connector_is_bound(&snapshot, &value.connector_id)? {
                    return Err(Error::new(ErrorCode::CapabilityDenied));
                }
            }
            _ => {}
        }
        Ok(())
    }

    #[allow(clippy::too_many_lines, clippy::cast_possible_truncation)]
    fn evaluate_wake(
        &mut self,
        observation_lsn: LSN,
        factors: AttentionFactors,
        rearm_at_ns: Option<i64>,
    ) -> Result<WakeEvaluation, Error> {
        self.tripwires.guard([observation_lsn])?;
        let source = self
            .plaintext_frames
            .get(
                usize::try_from(observation_lsn.get())
                    .ok()
                    .and_then(|lsn| lsn.checked_sub(1))
                    .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?,
            )
            .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
        let envelope = event::verify_event_with_history(
            &source.sealed_payload,
            schema_kind(source.header.kind)?,
            Boundary::Disk,
            &ActorHistory::new(&self.kinds, &self.authorities),
        )?
        .envelope;
        let signal = anticipation::signal(&source.header, &envelope)?;
        let snapshot = self.projections.begin_snapshot()?;
        let pending = IntentionsProjection::pending_by_trigger(
            &snapshot,
            anticipation::trigger_kind(&signal),
            hm_proj::intentions::MAXIMUM_INTENTIONS_PER_TRIGGER,
        )?;
        let mut actions = Vec::new();
        for record in pending {
            if record.set_lsn >= observation_lsn.get() {
                continue;
            }
            let intention = IntentionSet {
                intention_id: record.intention_id,
                objective: record.objective,
                trigger: Some(record.trigger),
                expires_at_ns: record.expires_at_ns,
                reply_route: record.reply_route,
            };
            let Some(fired) = hm_cortex::prospective::evaluate(&intention, &signal) else {
                continue;
            };
            let decision =
                hm_cortex::attention::decide(&intention.intention_id, &fired.wake_id, factors)?;
            let rearmed = if decision.decision == hm_schema::events::AttentionDecision::Schedule {
                let next = rearm_at_ns
                    .filter(|next| *next > signal.now_ns)
                    .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?;
                Some(
                    hm_cortex::prospective::rearm(&intention, decision.decision, next)
                        .ok_or_else(|| Error::new(ErrorCode::InvalidArgument))?,
                )
            } else {
                None
            };
            let source = self
                .plaintext_frames
                .get(record.set_lsn as usize - 1)
                .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))?;
            actions.push((source.header.conversation, fired, decision, rearmed));
        }
        drop(snapshot);
        let mut result = WakeEvaluation {
            observation_lsn: observation_lsn.get(),
            fired: Vec::new(),
        };
        for (conversation, fired, decision, rearmed) in actions {
            let mut events = vec![
                runtime_event(
                    EventKind::IntentionFired,
                    EventPayload::IntentionFired(Box::new(fired.clone())),
                    conversation,
                    Authority::RuntimeFact,
                ),
                runtime_event(
                    EventKind::AttentionDecided,
                    EventPayload::AttentionDecided(Box::new(decision.clone())),
                    conversation,
                    Authority::DerivedInference,
                ),
            ];
            if let Some(rearmed) = &rearmed {
                events.push(runtime_event(
                    EventKind::IntentionSet,
                    EventPayload::IntentionSet(Box::new(rearmed.clone())),
                    conversation,
                    Authority::RuntimeFact,
                ));
            }
            let mut identity = blake3::Hasher::new();
            identity.update(b"hypermind.wake.commit.v1\0");
            identity.update(&fired.wake_id);
            let mut connection = [0; 16];
            connection.copy_from_slice(&identity.finalize().as_bytes()[..16]);
            let committed = self.append_idempotent(connection, 1, events)?;
            result.fired.push(WakeDecision {
                intention_id: fired.intention_id,
                wake_id: fired.wake_id,
                decision: decision.decision,
                reason: decision.reason,
                fired_lsn: committed.first_lsn.get(),
                decision_lsn: committed.first_lsn.get() + 1,
                rearmed_intention_id: rearmed.map(|value| value.intention_id),
            });
        }
        Ok(result)
    }

    fn mine_procedures(&mut self) -> Result<(), Error> {
        if !self
            .plaintext_frames
            .iter()
            .any(|frame| frame.header.kind == EventKind::LoopClosed)
        {
            return Ok(());
        }
        let history = ActorHistory::new(&self.kinds, &self.authorities);
        let decoded = self
            .plaintext_frames
            .iter()
            .filter(|frame| {
                matches!(
                    frame.header.kind,
                    EventKind::LoopOpened
                        | EventKind::LoopClosed
                        | EventKind::ToolCall
                        | EventKind::ToolResult
                        | EventKind::Effect
                        | EventKind::Outcome
                )
            })
            .map(|frame| {
                event::verify_event_with_history(
                    &frame.sealed_payload,
                    schema_kind(frame.header.kind)?,
                    Boundary::Disk,
                    &history,
                )
                .map(|event| (frame.header, event.envelope))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mined = anticipation::mined_procedures(&decoded)?;
        for value in mined {
            if value.supports.is_empty() {
                continue;
            }
            let prior = ProceduresProjection::get(
                &self.projections.begin_snapshot()?,
                &value.procedure_id,
            )?;
            let (kind, payload) = if let Some(prior) = prior {
                if prior.state == ProcedureState::Adopted
                    || (prior.supports == value.supports
                        && prior.failures == value.failures.clone().unwrap_or_default()
                        && prior.counterexamples
                            == value.counterexamples.clone().unwrap_or_default())
                {
                    continue;
                }
                (
                    EventKind::ProcedureRevised,
                    EventPayload::ProcedureRevised(Box::new(ProcedureRevised {
                        procedure_id: value.procedure_id,
                        previous_lsn: prior.version_lsn,
                        strategy: value.strategy,
                        expected_outcomes: value.expected_outcomes,
                        preconditions: value.preconditions,
                        supports: value.supports,
                        failures: value.failures,
                        counterexamples: value.counterexamples,
                    })),
                )
            } else {
                (
                    EventKind::ProcedureMined,
                    EventPayload::ProcedureMined(Box::new(value)),
                )
            };
            self.append(vec![runtime_event(
                kind,
                payload,
                ConversationId::derive("hypermind-procedural-learning"),
                Authority::DerivedInference,
            )])?;
        }
        Ok(())
    }

    fn store_connector_credential(
        &self,
        provider: &str,
        connector_id: &[u8; 16],
        version: u32,
        secret: &[u8],
    ) -> Result<(), Error> {
        let mut entropy = OsEntropy;
        self.credentials.store(
            &self.keys,
            provider,
            connector_id,
            version,
            secret,
            &mut entropy,
        )
    }

    fn verify_source_delivery(
        &self,
        request: &SourceSignatureRequest,
    ) -> Result<VerifiedDelivery, Error> {
        let secret = self.credentials.secret(
            &self.keys,
            &request.provider,
            &request.connector_id,
            request.credential_version,
        )?;
        let envelope = DeliveryEnvelope {
            connector_id: request.connector_id,
            delivery_id: &request.delivery_id,
            event_name: &request.event_name,
            signed_at_ns: request.signed_at_ns,
            body: &request.body,
            signature: &request.signature,
        };
        verify_delivery(
            secret.as_slice(),
            &envelope,
            wall_time_ns()?,
            DEFAULT_FRESHNESS_WINDOW_NS,
        )
    }

    fn mint_source_consent(
        &self,
        provider: &str,
        connector_id: &[u8; 16],
        ttl_ns: i64,
    ) -> Result<ConsentMint, Error> {
        if provider.is_empty() || ttl_ns <= 0 {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let expires_at_ns = wall_time_ns()?
            .checked_add(ttl_ns)
            .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?;
        let mut nonce = [0_u8; 16];
        let mut entropy = OsEntropy;
        entropy.fill(&mut nonce)?;
        let key = consent_key(&self.keys);
        let state =
            mint_consent_state(key.as_slice(), provider, connector_id, nonce, expires_at_ns);
        Ok(ConsentMint {
            state: state.encoded,
            nonce: state.nonce,
            expires_at_ns: state.expires_at_ns,
        })
    }

    fn redeem_source_consent(
        &self,
        provider: &str,
        connector_id: &[u8; 16],
        encoded: &str,
    ) -> Result<ConsentGrant, Error> {
        let key = consent_key(&self.keys);
        verify_consent_state(
            key.as_slice(),
            provider,
            connector_id,
            encoded,
            wall_time_ns()?,
        )
    }

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
        let credentials = CredentialVault::open(&config.actor_directory)?;
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
        let (vector_lanes, vector_digests) =
            open_vector_lanes(&config.actor_directory, &plaintext_frames)?;
        let vector_checkpoint = projections
            .begin_snapshot()?
            .checkpoint(ProjectionId::VectorLane)?;
        for frame in plaintext_frames
            .iter()
            .filter(|frame| frame.header.lsn > vector_checkpoint)
        {
            projections.apply(ProjectionId::VectorLane, frame.header.lsn, &[])?;
        }
        let mut state = Self {
            config,
            log,
            keys,
            credentials,
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
            vector_lanes,
            vector_digests,
        };
        state.mine_procedures()?;
        Ok(state)
    }

    #[allow(clippy::too_many_lines)]
    fn append(&mut self, events: Vec<IncomingEvent>) -> Result<AppendOutcome, Error> {
        if events.is_empty() || events.len() > hm_schema::protocol::MAXIMUM_BATCH_EVENTS {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let first_lsn = self.log.next_lsn().get();
        let mut verified_kinds = self.kinds.clone();
        let mut verified_authorities = self.authorities.clone();
        let mut batch_vectors = BTreeMap::new();
        let mut batch_dimensions = BTreeMap::new();
        let mut anticipation_ids = std::collections::BTreeSet::new();
        for (index, incoming) in events.iter().enumerate() {
            let kind = schema_kind(incoming.kind)?;
            let verified = event::verify_event_with_history(
                &incoming.payload,
                kind,
                Boundary::Socket,
                &ActorHistory::new(&verified_kinds, &verified_authorities),
            )
            .map_err(|error| error.at_lsn(LSN::new(first_lsn + index as u64)))?;
            self.validate_anticipation(&verified.envelope.payload)?;
            let key = match &verified.envelope.payload {
                EventPayload::IntentionSet(value) => Some((0, value.intention_id.clone())),
                EventPayload::IntentionCancelled(value) => Some((0, value.intention_id.clone())),
                EventPayload::Predicted(value) => Some((1, value.prediction_id.clone())),
                EventPayload::OutcomeObserved(value) => Some((1, value.prediction_id.clone())),
                EventPayload::ProcedureMined(value) => Some((2, value.procedure_id.clone())),
                EventPayload::ProcedureRevised(value) => Some((2, value.procedure_id.clone())),
                EventPayload::ProcedureAdopted(value) => Some((2, value.procedure_id.clone())),
                EventPayload::SourceDeliveryAccepted(value) => Some((
                    3,
                    source_delivery_key(&value.connector_id, &value.delivery_id),
                )),
                _ => None,
            };
            if key.is_some_and(|key| !anticipation_ids.insert(key)) {
                return Err(Error::new(ErrorCode::IdempotencyConflict));
            }
            if let EventPayload::Embedding(embedding) = &verified.envelope.payload {
                if embedding.target_lsn >= first_lsn + index as u64 {
                    return Err(Error::new(ErrorCode::InvalidArgument));
                }
                let key = (embedding.space_id.clone(), embedding.target_lsn);
                let digest = embedding_digest(embedding);
                if self
                    .vector_lanes
                    .get(&embedding.space_id)
                    .is_some_and(|lane| lane.dimensions() != embedding.dimension as usize)
                    || self
                        .vector_digests
                        .get(&key)
                        .is_some_and(|prior| *prior != digest)
                    || batch_vectors
                        .insert(key, digest)
                        .is_some_and(|prior| prior != digest)
                    || batch_dimensions
                        .insert(embedding.space_id.clone(), embedding.dimension)
                        .is_some_and(|prior| prior != embedding.dimension)
                {
                    return Err(Error::new(ErrorCode::InvalidArgument));
                }
            }
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
            self.apply_vector_frame(frame)?;
        }
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
        let outcome = AppendOutcome {
            first_lsn: commit.first_lsn,
            last_lsn: commit.last_lsn,
            duplicate: false,
            leaf_count: mmr.leaf_count,
            last_leaf_hash: self.mmr.leaf_hash(mmr.leaf_count - 1)?,
            mmr_root: mmr.root,
        };
        if plaintext
            .iter()
            .any(|frame| frame.header.kind == EventKind::LoopClosed)
        {
            self.mine_procedures()?;
        }
        Ok(outcome)
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

    fn apply_vector_frame(&mut self, frame: &Frame) -> Result<(), Error> {
        if frame.header.kind == EventKind::Embedding {
            let verified = event::verify_event(
                &frame.sealed_payload,
                event::EventKind::Embedding,
                Boundary::Disk,
            )?;
            let EventPayload::Embedding(embedding) = verified.envelope.payload else {
                return Err(Error::new(ErrorCode::InvariantViolation));
            };
            let key = (embedding.space_id.clone(), embedding.target_lsn);
            if !self.vector_digests.contains_key(&key) {
                if !self.vector_lanes.contains_key(&embedding.space_id) {
                    let lane = VectorLane::open(
                        self.config.actor_directory.join("vectors"),
                        &embedding.space_id,
                        &embedding.space_id,
                        embedding.dimension as usize,
                    )?;
                    self.vector_lanes.insert(embedding.space_id.clone(), lane);
                }
                self.vector_lanes
                    .get(&embedding.space_id)
                    .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))?
                    .append(
                        LSN::new(embedding.target_lsn),
                        &embedding.quantized,
                        &embedding.binary_prefilter,
                    )?;
                self.vector_digests
                    .insert(key, embedding_digest(&embedding));
            }
        }
        self.projections
            .apply(ProjectionId::VectorLane, frame.header.lsn, &[])
    }

    #[allow(clippy::too_many_lines)]
    fn recall(&self, request: RecallRequest) -> Result<Vec<RecallItem>, Error> {
        let snapshot = self.projections.begin_snapshot()?;
        let ranked = matches!(
            &request,
            RecallRequest::Vector { .. }
                | RecallRequest::Lexical { .. }
                | RecallRequest::Entity { .. }
                | RecallRequest::Near { .. }
        );
        let mut items: Vec<RecallItem> = match request {
            RecallRequest::Vector {
                space_id,
                query,
                binary_prefilter,
                limit,
            } => {
                if limit == 0 || limit > bundle::MAXIMUM_CANDIDATES {
                    return Err(Error::new(ErrorCode::InvalidArgument));
                }
                let lane = self
                    .vector_lanes
                    .get(&space_id)
                    .ok_or_else(|| Error::new(ErrorCode::OperationUnavailable))?;
                let hits =
                    lane.search(&query, &binary_prefilter, limit.saturating_mul(4), limit)?;
                self.tripwires
                    .guard(hits.iter().map(|hit| hit.target_lsn))?;
                hits.into_iter()
                    .filter_map(
                        |hit| match read_conversation_record(&snapshot, hit.target_lsn) {
                            Ok(Some(record)) => Some(Ok(recall_item(
                                record,
                                u64::try_from(hit.score.max(0)).unwrap_or(0),
                            ))),
                            Ok(None) => None,
                            Err(error) => Some(Err(error)),
                        },
                    )
                    .collect()
            }
            RecallRequest::Semantic { query, limit } => {
                let _ = (query, limit);
                Err(Error::new(ErrorCode::OperationUnavailable))
            }
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
            RecallRequest::Entity {
                query,
                turn_text,
                limit,
            } => EntityProjection::query(&snapshot, &query, &turn_text, limit)?
                .into_iter()
                .filter_map(|hit| match read_conversation_record(&snapshot, hit.lsn) {
                    Ok(Some(record)) => Some(Ok(recall_item(record, hit.score() as u64))),
                    Ok(None) => None,
                    Err(error) => Some(Err(error)),
                })
                .collect(),
            RecallRequest::Near {
                anchor,
                query,
                turn_text,
                limit,
            } => {
                let entity_query = if query.is_empty() {
                    anchor
                } else {
                    format!("{anchor} {query}")
                };
                let mut lsns = std::collections::BTreeSet::new();
                let mut output = Vec::new();
                for hit in EntityProjection::query(&snapshot, &entity_query, &turn_text, limit)? {
                    if let Some(record) = read_conversation_record(&snapshot, hit.lsn)? {
                        lsns.insert(hit.lsn);
                        output.push(recall_item(record, hit.score() as u64));
                    }
                }
                if output.len() < limit && !query.is_empty() {
                    for hit in LexicalProjection::query(&snapshot, &query, limit)? {
                        if lsns.insert(hit.lsn)
                            && let Some(record) = read_conversation_record(&snapshot, hit.lsn)?
                        {
                            output.push(recall_item(record, hit.score_q32));
                            if output.len() == limit {
                                break;
                            }
                        }
                    }
                }
                Ok(output)
            }
            RecallRequest::Temporal {
                start_ns,
                end_ns,
                limit,
            } => {
                if start_ns > end_ns || limit == 0 {
                    return Err(Error::new(ErrorCode::InvalidArgument));
                }
                let mut output = Vec::new();
                for frame in self.plaintext_frames.iter().rev() {
                    if (start_ns..=end_ns).contains(&frame.header.wall_timestamp_ns.get())
                        && let Some(record) = read_conversation_record(&snapshot, frame.header.lsn)?
                    {
                        output.push(recall_item(record, 0));
                        if output.len() == limit {
                            break;
                        }
                    }
                }
                Ok(output)
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
            RecallRequest::Relation { .. } => Err(Error::new(ErrorCode::InvalidArgument)),
        }?;
        if !ranked || items.is_empty() || items.len() > MAXIMUM_PREFERENCE_TARGETS {
            return Ok(items);
        }
        let targets = items.iter().map(|item| item.lsn).collect::<Vec<_>>();
        let preferences = PreferenceProfile::load(&snapshot, &targets)?;
        if preferences.is_neutral() {
            return Ok(items);
        }
        for item in &mut items {
            let weight = preferences.weight_q16(item.lsn);
            item.score_q32 = adjust_q32(item.score_q32, weight)?;
            item.preference_q16 = weight;
        }
        items.sort_by(|left, right| {
            right
                .score_q32
                .cmp(&left.score_q32)
                .then_with(|| left.lsn.cmp(&right.lsn))
        });
        Ok(items)
    }

    fn as_of(
        &self,
        belief_type: BeliefType,
        canonical_identity: &str,
        as_of: BeliefAsOf,
    ) -> Result<BeliefAsOfResult, Error> {
        BeliefProjection::read_as_of(
            &self.projections.begin_snapshot()?,
            belief_type,
            canonical_identity,
            as_of,
        )
    }

    fn relations(&self, limit: usize) -> Result<Vec<EdgeRecord>, Error> {
        if limit == 0 || limit > MAXIMUM_GRAPH_NEIGHBOURS {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let snapshot = self.projections.begin_snapshot()?;
        let generation = RunsProjection::active_generation(&snapshot)?;
        GraphProjection::list_edges(&snapshot, generation, self.last_wall_timestamp_ns, limit)
    }

    fn relation_recall(&self, request: RecallRequest) -> Result<Vec<RelationResult>, Error> {
        let RecallRequest::Relation {
            space_id,
            query,
            binary_prefilter,
            limit,
        } = request
        else {
            return Err(Error::new(ErrorCode::InvalidArgument));
        };
        if limit == 0 || limit > bundle::MAXIMUM_CANDIDATES {
            return Err(Error::new(ErrorCode::InvalidArgument));
        }
        let lane = self
            .vector_lanes
            .get(&space_id)
            .ok_or_else(|| Error::new(ErrorCode::OperationUnavailable))?;
        let hits = lane.search(&query, &binary_prefilter, limit.saturating_mul(4), limit)?;
        self.tripwires
            .guard(hits.iter().map(|hit| hit.target_lsn))?;
        let snapshot = self.projections.begin_snapshot()?;
        let generation = RunsProjection::active_generation(&snapshot)?;
        let visible = GraphProjection::list_edges(
            &snapshot,
            generation,
            self.last_wall_timestamp_ns,
            bundle::MAXIMUM_CANDIDATES,
        )?
        .into_iter()
        .map(|record| record.event_lsn)
        .collect::<BTreeSet<_>>();
        let mut resolved = BTreeMap::new();
        let mut lane_hits = Vec::with_capacity(hits.len());
        for hit in hits {
            let Some(record) = GraphProjection::edge_at(&snapshot, hit.target_lsn.get())? else {
                continue;
            };
            if !visible.contains(&record.event_lsn) {
                continue;
            }
            let support = support_lsns(&record);
            lane_hits.push(RelationHit {
                edge_id: record.edge_id.clone(),
                event_lsn: hit.target_lsn,
                weight_micros: record.weight_micros,
                score: hit.score,
                support_lsns: support.clone(),
            });
            resolved.insert(
                (record.edge_id.clone(), hit.target_lsn.get()),
                (
                    record,
                    support,
                    u64::try_from(hit.score.max(0)).unwrap_or(0),
                ),
            );
        }
        if lane_hits.is_empty() {
            return Ok(Vec::new());
        }
        let ranked = relation::rank(&lane_hits, limit.min(relation::MAXIMUM_RELATION_CANDIDATES))?;
        let mut output = Vec::with_capacity(ranked.ranking.candidates.len());
        for candidate in ranked.ranking.candidates {
            let key = (candidate.canonical_id, candidate.lsn.get());
            let Some((record, support, score_q32)) = resolved.remove(&key) else {
                return Err(Error::new(ErrorCode::InvariantViolation));
            };
            output.push(RelationResult {
                edge_id: record.edge_id,
                relation: record.relation,
                source_id: record.source_id,
                target_id: record.target_id,
                event_lsn: candidate.lsn,
                weight_micros: record.weight_micros,
                support_lsns: support,
                score_q32,
            });
        }
        Ok(output)
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
            ProjectionId::EntityIndex,
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

type VectorState = (
    BTreeMap<String, VectorLane>,
    BTreeMap<(String, u64), [u8; 32]>,
);

fn runtime_event(
    kind: EventKind,
    payload: EventPayload,
    conversation: ConversationId,
    authority: Authority,
) -> IncomingEvent {
    IncomingEvent {
        kind,
        conversation,
        payload: encode_event_envelope(&EventEnvelope {
            schema_version: CURRENT_SCHEMA_VERSION,
            payload,
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 0,
            origin_actor: 0,
            run_id: None,
            model_provenance: None,
            authority,
            retention: Retention::Durable,
            sensitivity: Sensitivity::Personal,
            event_time_ns: 0,
        }),
    }
}

fn open_vector_lanes(directory: &Path, frames: &[Frame]) -> Result<VectorState, Error> {
    let mut entries: BTreeMap<String, Vec<VectorEntry>> = BTreeMap::new();
    let mut digests = BTreeMap::new();
    for frame in frames
        .iter()
        .filter(|frame| frame.header.kind == EventKind::Embedding)
    {
        let verified = event::verify_event(
            &frame.sealed_payload,
            event::EventKind::Embedding,
            Boundary::Disk,
        )?;
        let EventPayload::Embedding(embedding) = verified.envelope.payload else {
            return Err(Error::new(ErrorCode::InvariantViolation));
        };
        if embedding.target_lsn >= frame.header.lsn.get() {
            return Err(Error::new(ErrorCode::VectorIndexCorrupt));
        }
        let key = (embedding.space_id.clone(), embedding.target_lsn);
        let digest = embedding_digest(&embedding);
        if let Some(prior) = digests.insert(key, digest) {
            if prior != digest {
                return Err(Error::new(ErrorCode::VectorIndexCorrupt));
            }
            continue;
        }
        entries
            .entry(embedding.space_id)
            .or_default()
            .push(VectorEntry {
                target_lsn: LSN::new(embedding.target_lsn),
                quantized: embedding.quantized,
                binary_prefilter: embedding.binary_prefilter,
            });
    }
    let mut lanes = BTreeMap::new();
    for (space, records) in entries {
        let dimensions = records
            .first()
            .ok_or_else(|| Error::new(ErrorCode::InvariantViolation))?
            .quantized
            .len();
        if records
            .iter()
            .any(|record| record.quantized.len() != dimensions)
        {
            return Err(Error::new(ErrorCode::VectorIndexCorrupt));
        }
        let lane = VectorLane::open(directory.join("vectors"), &space, &space, dimensions)?;
        lane.replay(&records)?;
        lanes.insert(space, lane);
    }
    Ok((lanes, digests))
}

fn embedding_digest(embedding: &hm_schema::events::Embedding) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(&embedding.dimension.to_le_bytes());
    for value in &embedding.quantized {
        hasher.update(&value.to_ne_bytes());
    }
    hasher.update(&embedding.binary_prefilter);
    *hasher.finalize().as_bytes()
}

fn recall_item(record: ConversationRecord, score_q32: u64) -> RecallItem {
    RecallItem {
        lsn: record.lsn,
        kind: record.kind,
        conversation: record.conversation,
        wall_timestamp_ns: record.wall_timestamp_ns,
        payload: record.payload,
        score_q32,
        preference_q16: PREFERENCE_NEUTRAL_Q16,
    }
}

fn support_lsns(record: &EdgeRecord) -> Vec<LSN> {
    let mut support = BTreeSet::new();
    for citation in &record.citations {
        for lsn in [citation.first_lsn, citation.last_lsn] {
            if lsn != 0 {
                support.insert(lsn);
            }
        }
    }
    support.into_iter().map(LSN::new).collect()
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

fn connector_is_bound(snapshot: &ReadSnapshot<'_>, connector_id: &[u8]) -> Result<bool, Error> {
    Ok(
        ConnectorRegistryProjection::connector(snapshot, connector_id)?
            .is_some_and(|record| record.state == u8::from(ConnectorState::Bound)),
    )
}

fn source_delivery_key(connector_id: &[u8], delivery_id: &[u8]) -> Vec<u8> {
    let mut key = Vec::with_capacity(connector_id.len() + 1 + delivery_id.len());
    key.extend_from_slice(connector_id);
    key.push(0);
    key.extend_from_slice(delivery_id);
    key
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
