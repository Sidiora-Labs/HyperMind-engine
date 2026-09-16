#![allow(clippy::missing_errors_doc)]

use crate::actor::{ActivateRequest, ActorConfig, ActorEngine, IncomingEvent, RecallRequest};
use crate::admin::{self, LatencyHistograms};
use crate::auth::{self, Principal};
use crate::config::ServerConfig;
use crate::errors::{MutationEffectState, mutation_effect_state};
use crate::protocol::{FrameParser, encode_frame};
use crate::requests::{asof, attest, checkpoint, subscribe};
use hm_compose::canonical::canonical_bytes;
use hm_compose::tokens::FallbackWeights;
use hm_core::{ActorId, ConversationId, Error, ErrorCode, LSN};
use hm_ledger::frame::EventKind;
use hm_schema::protocol::{
    MAXIMUM_BATCH_EVENTS, MAXIMUM_PROTOCOL_PAYLOAD_BYTES, verify_wire_envelope,
};
use hm_schema::wire::{
    AppendAck, AttestAck, BytesResult, CheckpointAck, CheckpointResult, ErrorDetail, Event,
    FrameRecord, HealthResult, MutationEffectState as WireMutationEffectState, ProjectionStat,
    RecallMode as WireRecallMode, RecallResult, Request, RequestPayload, Response, ResponsePayload,
    ResponseStatus, StatsResult, SubscriptionAck, TranscriptResult, Welcome, WireEnvelope,
    WirePayload,
};
use std::collections::BTreeMap;
use std::future::Future;
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{Semaphore, mpsc, watch};

const MAXIMUM_SUBSCRIPTIONS: u32 = 64;

pub trait ToolDispatcher: Send + Sync {
    fn dispatch(
        &self,
        actor: ActorEngine,
        verb: String,
        arguments_json: Vec<u8>,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<Vec<u8>, Error>> + Send + '_>>;
}

pub struct UdsServer {
    config: Arc<ServerConfig>,
    actors: Arc<BTreeMap<u16, ActorEngine>>,
    listener: UnixListener,
    active_connections: Arc<AtomicUsize>,
    latencies: LatencyHistograms,
    tool_dispatcher: Option<Arc<dyn ToolDispatcher>>,
}

impl UdsServer {
    pub async fn bind(config: ServerConfig) -> Result<Self, Error> {
        std::fs::create_dir_all(&config.data_directory)
            .map_err(|_| Error::new(ErrorCode::OpenFailed))?;
        let mut actors = BTreeMap::new();
        for capability in &config.actors {
            let actor = ActorId::new(capability.actor);
            let engine = ActorEngine::open(ActorConfig {
                actor_directory: config.actor_directory(capability.actor),
                actor,
                user: config.user,
                kek: config.kek,
                projection_map_bytes: config.projection_map_bytes,
            })
            .await?;
            actors.insert(capability.actor, engine);
        }
        if let Some(parent) = config.socket_path.parent() {
            std::fs::create_dir_all(parent).map_err(|_| Error::new(ErrorCode::OpenFailed))?;
        }
        match std::fs::symlink_metadata(&config.socket_path) {
            Ok(metadata) if metadata.file_type().is_socket() => {
                std::fs::remove_file(&config.socket_path)
                    .map_err(|_| Error::new(ErrorCode::OpenFailed))?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Ok(_) | Err(_) => return Err(Error::new(ErrorCode::OpenFailed)),
        }
        let listener = UnixListener::bind(&config.socket_path)
            .map_err(|_| Error::new(ErrorCode::OpenFailed))?;
        std::fs::set_permissions(&config.socket_path, std::fs::Permissions::from_mode(0o600))
            .map_err(|_| Error::new(ErrorCode::OpenFailed))?;
        Ok(Self {
            config: Arc::new(config),
            actors: Arc::new(actors),
            listener,
            active_connections: Arc::new(AtomicUsize::new(0)),
            latencies: LatencyHistograms::default(),
            tool_dispatcher: None,
        })
    }

    #[must_use]
    pub fn with_tool_dispatcher(mut self, dispatcher: Arc<dyn ToolDispatcher>) -> Self {
        self.tool_dispatcher = Some(dispatcher);
        self
    }

    pub async fn serve_until(self, shutdown: impl Future<Output = ()>) -> Result<(), Error> {
        let permits = Arc::new(Semaphore::new(self.config.maximum_connections));
        tokio::pin!(shutdown);
        loop {
            tokio::select! {
                () = &mut shutdown => return Ok(()),
                accepted = self.listener.accept() => {
                    let (stream, _) = accepted.map_err(|_| Error::new(ErrorCode::ReadFailed))?;
                    let Ok(permit) = permits.clone().try_acquire_owned() else {
                        continue;
                    };
                    let config = Arc::clone(&self.config);
                    let actors = Arc::clone(&self.actors);
                    let active = Arc::clone(&self.active_connections);
                    let latencies = self.latencies.clone();
                    let tool_dispatcher = self.tool_dispatcher.clone();
                    active.fetch_add(1, Ordering::Relaxed);
                    tokio::spawn(async move {
                        let _permit = permit;
                        let _ = handle_connection(
                            stream,
                            config,
                            actors,
                            Arc::clone(&active),
                            latencies,
                            tool_dispatcher,
                        )
                        .await;
                        active.fetch_sub(1, Ordering::Relaxed);
                    });
                }
            }
        }
    }
}

impl Drop for UdsServer {
    fn drop(&mut self) {
        if std::fs::symlink_metadata(&self.config.socket_path)
            .is_ok_and(|metadata| metadata.file_type().is_socket())
        {
            let _ = std::fs::remove_file(&self.config.socket_path);
        }
    }
}

struct Session {
    actor: Option<u16>,
    admin: bool,
    proto_version: u16,
    connection_id: [u8; 16],
    next_subscription_id: u64,
    subscriptions: usize,
}

#[allow(clippy::too_many_lines)]
async fn handle_connection(
    stream: UnixStream,
    config: Arc<ServerConfig>,
    actors: Arc<BTreeMap<u16, ActorEngine>>,
    active_connections: Arc<AtomicUsize>,
    latencies: LatencyHistograms,
    tool_dispatcher: Option<Arc<dyn ToolDispatcher>>,
) -> Result<(), Error> {
    let (mut reader, mut writer) = stream.into_split();
    let (output, mut queued) = mpsc::channel::<Vec<u8>>(config.maximum_output_frames);
    let output_bytes = Arc::new(AtomicUsize::new(0));
    let writer_bytes = Arc::clone(&output_bytes);
    let writer_task = tokio::spawn(async move {
        while let Some(frame) = queued.recv().await {
            let result = writer.write_all(&frame).await;
            writer_bytes.fetch_sub(frame.len(), Ordering::Relaxed);
            result.map_err(|_| Error::new(ErrorCode::WriteFailed))?;
        }
        Ok::<(), Error>(())
    });
    let mut parser = FrameParser::default();
    let mut buffer = vec![0; 64 * 1024];
    let mut session = None;
    let (disconnect, mut disconnected) = watch::channel(false);
    loop {
        let count = tokio::select! {
            changed = disconnected.changed() => {
                if changed.is_err() || *disconnected.borrow() {
                    break;
                }
                continue;
            }
            read = reader.read(&mut buffer) => {
                read.map_err(|_| Error::new(ErrorCode::ReadFailed))?
            }
        };
        if count == 0 {
            break;
        }
        for payload in parser.push(&buffer[..count])? {
            let envelope = verify_wire_envelope(&payload)?;
            if session.is_none() {
                let WirePayload::Hello(hello) = envelope.payload else {
                    return Err(Error::new(ErrorCode::CapabilityDenied));
                };
                if envelope.proto_version != hello.proto_version {
                    return Err(Error::new(ErrorCode::ProtocolVersion));
                }
                let authenticated = authenticate(
                    &config,
                    &hello.connection_id,
                    &hello.capability_token,
                    hello.proto_version,
                )?;
                let next_client_seq = match authenticated.actor {
                    Some(actor_id) => {
                        actors
                            .get(&actor_id)
                            .ok_or_else(|| Error::new(ErrorCode::CapabilityDenied))?
                            .next_client_sequence(authenticated.connection_id)
                            .await?
                    }
                    None => 1,
                };
                let welcome = Welcome {
                    proto_version: hello.proto_version,
                    actor_ns: authenticated.actor.unwrap_or_default(),
                    admin: authenticated.admin,
                    maximum_frame_bytes: u32::try_from(MAXIMUM_PROTOCOL_PAYLOAD_BYTES)
                        .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?,
                    maximum_batch_events: u32::try_from(MAXIMUM_BATCH_EVENTS)
                        .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?,
                    maximum_subscriptions: MAXIMUM_SUBSCRIPTIONS,
                    next_client_seq,
                };
                enqueue(
                    &output,
                    &output_bytes,
                    config.maximum_output_bytes,
                    wire(
                        authenticated.proto_version,
                        WirePayload::Welcome(Box::new(welcome)),
                    )?,
                )?;
                session = Some(authenticated);
                continue;
            }
            let WirePayload::Request(request) = envelope.payload else {
                return Err(Error::new(ErrorCode::ProtocolInvalid));
            };
            let mutation = mutation_request(&request.payload);
            if let Err(error) = hm_schema::protocol::validate_request(&request) {
                let current = session.as_ref().expect("session established");
                enqueue(
                    &output,
                    &output_bytes,
                    config.maximum_output_bytes,
                    error_wire(current.proto_version, request.request_id, error, mutation)?,
                )?;
                continue;
            }
            let Request {
                request_id,
                payload: request_payload,
            } = *request;
            let current = session.as_mut().expect("session established");
            if let RequestPayload::Subscribe(request_value) = request_payload {
                let response = handle_subscribe(
                    current,
                    request_id,
                    *request_value,
                    &actors,
                    &output,
                    &output_bytes,
                    config.maximum_output_bytes,
                    disconnect.clone(),
                )
                .await;
                if let Err(error) = response {
                    enqueue(
                        &output,
                        &output_bytes,
                        config.maximum_output_bytes,
                        error_wire(current.proto_version, request_id, error, false)?,
                    )?;
                }
                continue;
            }
            let operation = request_name(&request_payload);
            let started = std::time::Instant::now();
            let response = handle_request(
                current,
                request_id,
                request_payload,
                &actors,
                active_connections.load(Ordering::Relaxed),
                &latencies,
                tool_dispatcher.as_deref(),
            )
            .await;
            latencies.observe(operation, started.elapsed());
            let envelope = match response {
                Ok(payload) => response_wire(current.proto_version, payload.0, payload.1, true)?,
                Err((request_id, error)) => {
                    error_wire(current.proto_version, request_id, error, mutation)?
                }
            };
            enqueue(
                &output,
                &output_bytes,
                config.maximum_output_bytes,
                envelope,
            )?;
        }
    }
    let _ = disconnect.send(true);
    drop(output);
    writer_task
        .await
        .map_err(|_| Error::new(ErrorCode::WriteFailed))??;
    Ok(())
}

fn authenticate(
    config: &ServerConfig,
    connection_id: &[u8],
    token: &[u8],
    proto_version: u16,
) -> Result<Session, Error> {
    let connection_id = connection_id
        .try_into()
        .map_err(|_| Error::new(ErrorCode::ProtocolInvalid))?;
    let principal = auth::authenticate(config, token)?;
    Ok(Session {
        actor: match principal {
            Principal::Actor(actor) => Some(actor),
            Principal::Admin => None,
        },
        admin: principal == Principal::Admin,
        proto_version,
        connection_id,
        next_subscription_id: 1,
        subscriptions: 0,
    })
}

#[allow(clippy::too_many_lines)]
async fn handle_request(
    session: &Session,
    request_id: u64,
    request_payload: RequestPayload,
    actors: &BTreeMap<u16, ActorEngine>,
    active_connections: usize,
    latencies: &LatencyHistograms,
    tool_dispatcher: Option<&dyn ToolDispatcher>,
) -> Result<(u64, ResponsePayload), (u64, Error)> {
    let request = Request {
        request_id,
        payload: request_payload,
    };
    let principal = session.actor.map_or(Principal::Admin, Principal::Actor);
    auth::authorize(principal, &request.payload).map_err(|error| (request_id, error))?;
    if let RequestPayload::Health(_) = request.payload {
        return Ok((
            request_id,
            ResponsePayload::HealthResult(Box::new(HealthResult {
                ready: true,
                actor_count: u32::try_from(actors.len())
                    .map_err(|_| (request_id, Error::new(ErrorCode::CapacityExceeded)))?,
                active_connections: u32::try_from(active_connections)
                    .map_err(|_| (request_id, Error::new(ErrorCode::CapacityExceeded)))?,
            })),
        ));
    }
    if let RequestPayload::LatencyHistograms(_) = request.payload {
        return Ok((
            request_id,
            ResponsePayload::LatencyResult(Box::new(latencies.snapshot())),
        ));
    }
    let actor_id = match &request.payload {
        RequestPayload::Stats(stats) => stats.actor,
        RequestPayload::VerifyStatus(verify) => verify.actor,
        RequestPayload::RebuildProjection(rebuild) => rebuild.actor,
        RequestPayload::CryptoDelete(delete) => delete.actor,
        _ => session
            .actor
            .ok_or((request_id, Error::new(ErrorCode::CapabilityDenied)))?,
    };
    let actor = actors
        .get(&actor_id)
        .ok_or((request_id, Error::new(ErrorCode::CapabilityDenied)))?;
    let payload = match request.payload {
        RequestPayload::ToolRequest(value) => {
            let dispatcher =
                tool_dispatcher.ok_or((request_id, Error::new(ErrorCode::OperationUnavailable)))?;
            let bytes = dispatcher
                .dispatch(actor.clone(), value.verb, value.arguments_json)
                .await
                .map_err(|error| (request_id, error))?;
            ResponsePayload::BytesResult(Box::new(BytesResult { bytes }))
        }
        RequestPayload::Append(append) => {
            let mut incoming = Vec::with_capacity(append.events.len());
            for event in append.events {
                incoming.push(IncomingEvent {
                    kind: EventKind::try_from(event.kind).map_err(|error| (request_id, error))?,
                    conversation: conversation(&event.conversation)
                        .map_err(|error| (request_id, error))?,
                    payload: event.payload,
                });
            }
            let outcome = actor
                .append_idempotent(session.connection_id, append.client_seq, incoming)
                .await
                .map_err(|error| (request_id, error))?;
            ResponsePayload::AppendAck(Box::new(AppendAck {
                client_seq: append.client_seq,
                first_lsn: outcome.first_lsn.get(),
                last_lsn: outcome.last_lsn.get(),
                duplicate: outcome.duplicate,
                leaf_count: outcome.leaf_count,
                last_leaf_hash: Some(outcome.last_leaf_hash.to_vec()),
                mmr_root: Some(outcome.mmr_root.to_vec()),
            }))
        }
        RequestPayload::Activate(activate) => {
            let weights = activate
                .token_weights
                .ok_or((request_id, Error::new(ErrorCode::ProtocolInvalid)))?;
            let mut per_byte_q8 = [0; 256];
            per_byte_q8.copy_from_slice(&weights);
            let bundle = actor
                .activate(ActivateRequest {
                    conversation: conversation(&activate.conversation)
                        .map_err(|error| (request_id, error))?,
                    query: String::from_utf8(activate.query)
                        .map_err(|_| (request_id, Error::new(ErrorCode::ProtocolInvalid)))?,
                    turn_text: String::from_utf8(activate.turn_text.unwrap_or_default())
                        .map_err(|_| (request_id, Error::new(ErrorCode::ProtocolInvalid)))?,
                    budget_tokens: usize::try_from(activate.budget_tokens)
                        .map_err(|_| (request_id, Error::new(ErrorCode::CapacityExceeded)))?,
                    token_weights: FallbackWeights {
                        per_byte_q8,
                        item_overhead: activate.token_item_overhead,
                    },
                })
                .await
                .map_err(|error| (request_id, error))?;
            ResponsePayload::BytesResult(Box::new(BytesResult {
                bytes: canonical_bytes(&bundle).map_err(|error| (request_id, error))?,
            }))
        }
        RequestPayload::Transcript(transcript) => {
            let mut records = actor
                .recall(RecallRequest::Timeline {
                    conversation: conversation(&transcript.conversation)
                        .map_err(|error| (request_id, error))?,
                    since_lsn: LSN::new(transcript.since_lsn),
                    limit: transcript.limit as usize + 1,
                })
                .await
                .map_err(|error| (request_id, error))?;
            let truncated = records.len() > transcript.limit as usize;
            records.truncate(transcript.limit as usize);
            ResponsePayload::TranscriptResult(Box::new(TranscriptResult {
                records: records
                    .into_iter()
                    .map(|record| FrameRecord {
                        lsn: record.lsn.get(),
                        kind: record.kind as u8,
                        wall_timestamp_ns: record.wall_timestamp_ns.get(),
                        actor: actor_id,
                        conversation: record.conversation.into_bytes().to_vec(),
                        payload: record.payload,
                    })
                    .collect(),
                truncated,
            }))
        }
        RequestPayload::Recall(recall) => {
            let query = String::from_utf8(recall.query)
                .map_err(|_| (request_id, Error::new(ErrorCode::ProtocolInvalid)))?;
            let recall_request = match (recall.mode, recall.level) {
                (WireRecallMode::ListWindows, 0) => RecallRequest::Semantic {
                    query,
                    limit: recall.limit as usize,
                },
                (WireRecallMode::ListWindows, 1) => RecallRequest::Lexical {
                    query,
                    limit: recall.limit as usize,
                },
                (WireRecallMode::ListWindows, 2) => RecallRequest::Entity {
                    query,
                    turn_text: String::new(),
                    limit: recall.limit as usize,
                },
                (WireRecallMode::ListWindows, 3) => RecallRequest::Temporal {
                    start_ns: recall.start_ns,
                    end_ns: recall.end_ns,
                    limit: recall.limit as usize,
                },
                (WireRecallMode::OpenWindow, 0) => RecallRequest::Near {
                    anchor: query,
                    query: String::new(),
                    turn_text: String::new(),
                    limit: recall.limit as usize,
                },
                _ => return Err((request_id, Error::new(ErrorCode::ProtocolInvalid))),
            };
            let items = actor
                .recall(recall_request)
                .await
                .map_err(|error| (request_id, error))?;
            ResponsePayload::RecallResult(Box::new(RecallResult {
                windows: None,
                members: Some(items.into_iter().map(|item| item.lsn.get()).collect()),
            }))
        }
        RequestPayload::AsOf(value) => ResponsePayload::BeliefResult(Box::new(
            asof::read(actor, *value)
                .await
                .map_err(|error| (request_id, error))?,
        )),
        RequestPayload::Checkpoint(value) => {
            let outcome = checkpoint::write(
                actor,
                session.connection_id,
                value.client_seq,
                value.turn_id,
                value.blob,
            )
            .await
            .map_err(|error| (request_id, error))?;
            ResponsePayload::CheckpointAck(Box::new(CheckpointAck {
                lsn: outcome.lsn.get(),
            }))
        }
        RequestPayload::LatestCheckpoint(value) => {
            let result = checkpoint::latest(actor, value.turn_id)
                .await
                .map_err(|error| (request_id, error))?;
            ResponsePayload::CheckpointResult(Box::new(match result {
                Some(value) => CheckpointResult {
                    present: true,
                    lsn: value.lsn.get(),
                    blob: Some(value.blob),
                },
                None => CheckpointResult {
                    present: false,
                    lsn: 0,
                    blob: None,
                },
            }))
        }
        RequestPayload::Attest(value) => {
            let outcome = attest::write(actor, session.connection_id, *value)
                .await
                .map_err(|error| (request_id, error))?;
            ResponsePayload::AttestAck(Box::new(AttestAck {
                first_lsn: outcome.first_lsn.get(),
                last_lsn: outcome.last_lsn.get(),
                count: u32::try_from(outcome.last_lsn.get() - outcome.first_lsn.get() + 1)
                    .map_err(|_| (request_id, Error::new(ErrorCode::CapacityExceeded)))?,
            }))
        }
        RequestPayload::Stats(_) => {
            let stats = actor.stats().await.map_err(|error| (request_id, error))?;
            ResponsePayload::StatsResult(Box::new(StatsResult {
                actor: actor_id,
                log_events: stats.log_events,
                log_bytes: stats.log_bytes,
                projection_stats: stats
                    .projections
                    .into_iter()
                    .map(|projection| ProjectionStat {
                        name: projection.name.to_owned(),
                        applied_lsn: projection.applied_lsn.get(),
                    })
                    .collect(),
            }))
        }
        RequestPayload::VerifyStatus(_) => ResponsePayload::VerifyResult(Box::new(
            admin::verify(actor)
                .await
                .map_err(|error| (request_id, error))?,
        )),
        RequestPayload::RebuildProjection(value) => ResponsePayload::RebuildResult(Box::new(
            admin::rebuild(actor, value.name)
                .await
                .map_err(|error| (request_id, error))?,
        )),
        RequestPayload::CryptoDelete(_) => ResponsePayload::DeleteResult(Box::new(
            admin::crypto_delete(actor)
                .await
                .map_err(|error| (request_id, error))?,
        )),
        _ => return Err((request_id, Error::new(ErrorCode::OperationUnavailable))),
    };
    Ok((request_id, payload))
}

const fn request_name(request: &RequestPayload) -> &'static str {
    match request {
        RequestPayload::Append(_) => "append",
        RequestPayload::Activate(_) => "activate",
        RequestPayload::Transcript(_) => "transcript",
        RequestPayload::Recall(_) => "recall",
        RequestPayload::AsOf(_) => "asof",
        RequestPayload::Checkpoint(_) => "checkpoint",
        RequestPayload::LatestCheckpoint(_) => "latest_checkpoint",
        RequestPayload::Attest(_) => "attest",
        RequestPayload::Subscribe(_) => "subscribe",
        RequestPayload::Health(_) => "health",
        RequestPayload::Stats(_) => "stats",
        RequestPayload::LatencyHistograms(_) => "latency_histograms",
        RequestPayload::VerifyStatus(_) => "verify_status",
        RequestPayload::RebuildProjection(_) => "rebuild_projection",
        RequestPayload::CryptoDelete(_) => "crypto_delete",
        RequestPayload::ToolRequest(_) => "tool_request",
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_subscribe(
    session: &mut Session,
    request_id: u64,
    request: hm_schema::wire::Subscribe,
    actors: &BTreeMap<u16, ActorEngine>,
    output: &mpsc::Sender<Vec<u8>>,
    output_bytes: &Arc<AtomicUsize>,
    maximum_output_bytes: usize,
    disconnect: watch::Sender<bool>,
) -> Result<(), Error> {
    if session.admin || session.subscriptions >= MAXIMUM_SUBSCRIPTIONS as usize {
        return Err(Error::new(if session.admin {
            ErrorCode::CapabilityDenied
        } else {
            ErrorCode::CapacityExceeded
        }));
    }
    let actor_id = session
        .actor
        .ok_or_else(|| Error::new(ErrorCode::CapabilityDenied))?;
    let actor = actors
        .get(&actor_id)
        .ok_or_else(|| Error::new(ErrorCode::CapabilityDenied))?;
    let conversation = request
        .conversation
        .as_deref()
        .map(conversation)
        .transpose()?;
    let available = output.capacity().saturating_sub(1);
    if available == 0 {
        return Err(Error::new(ErrorCode::CapacityExceeded));
    }
    let mut started = subscribe::start(
        actor,
        LSN::new(request.since_lsn),
        conversation,
        available + 1,
    )
    .await?;
    if started.replay.len() > available {
        return Err(Error::new(ErrorCode::CapacityExceeded));
    }
    let subscription_id = session.next_subscription_id;
    session.next_subscription_id = session
        .next_subscription_id
        .checked_add(1)
        .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?;
    enqueue(
        output,
        output_bytes,
        maximum_output_bytes,
        response_wire(
            session.proto_version,
            request_id,
            ResponsePayload::SubscriptionAck(Box::new(SubscriptionAck { subscription_id })),
            true,
        )?,
    )?;
    for frame in &started.replay {
        enqueue(
            output,
            output_bytes,
            maximum_output_bytes,
            event_wire(session.proto_version, subscription_id, frame)?,
        )?;
    }
    session.subscriptions += 1;
    let output = output.clone();
    let output_bytes = Arc::clone(output_bytes);
    let proto_version = session.proto_version;
    let mut stop = disconnect.subscribe();
    tokio::spawn(async move {
        loop {
            let received = tokio::select! {
                changed = stop.changed() => {
                    if changed.is_err() || *stop.borrow() {
                        return;
                    }
                    continue;
                }
                received = started.receiver.recv() => received,
            };
            let frame = match received {
                Ok(frame) => frame,
                Err(
                    tokio::sync::broadcast::error::RecvError::Lagged(_)
                    | tokio::sync::broadcast::error::RecvError::Closed,
                ) => {
                    let _ = disconnect.send(true);
                    return;
                }
            };
            if frame.header.lsn.get() <= started.replay_tail.get()
                || conversation.is_some_and(|value| value != frame.header.conversation)
            {
                continue;
            }
            let queued = event_wire(proto_version, subscription_id, &frame)
                .and_then(|wire| enqueue(&output, &output_bytes, maximum_output_bytes, wire));
            if queued.is_err() {
                let _ = disconnect.send(true);
                return;
            }
        }
    });
    Ok(())
}

fn mutation_request(payload: &RequestPayload) -> bool {
    if let RequestPayload::ToolRequest(value) = payload {
        return matches!(
            value.verb.as_str(),
            "remember"
                | "believe"
                | "retract"
                | "dispute"
                | "intend"
                | "bind"
                | "predict"
                | "outcome"
                | "attest"
                | "consolidate"
                | "forget"
        );
    }
    matches!(
        payload,
        RequestPayload::Append(_)
            | RequestPayload::Checkpoint(_)
            | RequestPayload::Attest(_)
            | RequestPayload::RebuildProjection(_)
            | RequestPayload::CryptoDelete(_)
    )
}

fn error_wire(
    proto_version: u16,
    request_id: u64,
    error: Error,
    mutation: bool,
) -> Result<Vec<u8>, Error> {
    let effect_state = if mutation {
        match mutation_effect_state(error) {
            MutationEffectState::NotDispatched => WireMutationEffectState::NotDispatched,
            MutationEffectState::Unknown => WireMutationEffectState::Unknown,
            MutationEffectState::Rejected => WireMutationEffectState::Rejected,
        }
    } else {
        WireMutationEffectState::None
    };
    response_wire(
        proto_version,
        request_id,
        ResponsePayload::ErrorDetail(Box::new(ErrorDetail {
            code: error.code as u8,
            system_error: error.system_error,
            lsn: error.lsn.get(),
            offset: error.offset,
            effect_state,
        })),
        false,
    )
}

fn event_wire(
    proto_version: u16,
    subscription_id: u64,
    frame: &hm_ledger::frame::Frame,
) -> Result<Vec<u8>, Error> {
    wire(
        proto_version,
        WirePayload::Event(Box::new(Event {
            subscription_id,
            lsn: frame.header.lsn.get(),
            kind: frame.header.kind as u8,
            wall_timestamp_ns: frame.header.wall_timestamp_ns.get(),
            actor: frame.header.actor.get(),
            conversation: frame.header.conversation.into_bytes().to_vec(),
            payload: frame.sealed_payload.clone(),
        })),
    )
}

fn conversation(bytes: &[u8]) -> Result<ConversationId, Error> {
    Ok(ConversationId::new(
        bytes
            .try_into()
            .map_err(|_| Error::new(ErrorCode::ProtocolInvalid))?,
    ))
}

fn response_wire(
    proto_version: u16,
    request_id: u64,
    payload: ResponsePayload,
    ok: bool,
) -> Result<Vec<u8>, Error> {
    wire(
        proto_version,
        WirePayload::Response(Box::new(Response {
            request_id,
            status: if ok {
                ResponseStatus::Ok
            } else {
                ResponseStatus::Error
            },
            payload: Some(payload),
        })),
    )
}

fn wire(proto_version: u16, payload: WirePayload) -> Result<Vec<u8>, Error> {
    encode_frame(&hm_schema::protocol::encode_wire_envelope(&WireEnvelope {
        proto_version,
        payload,
    }))
}

fn enqueue(
    output: &mpsc::Sender<Vec<u8>>,
    queued_bytes: &AtomicUsize,
    maximum_bytes: usize,
    frame: Vec<u8>,
) -> Result<(), Error> {
    let length = frame.len();
    queued_bytes
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current
                .checked_add(length)
                .filter(|updated| *updated <= maximum_bytes)
        })
        .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?;
    if output.try_send(frame).is_err() {
        queued_bytes.fetch_sub(length, Ordering::Relaxed);
        return Err(Error::new(ErrorCode::CapacityExceeded));
    }
    Ok(())
}
