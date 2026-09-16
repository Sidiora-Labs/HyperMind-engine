use crate::auth::{self, Principal};
use crate::config::ServerConfig;
use crate::protocol::{FrameParser, encode_frame};
use hm_schema::protocol::{MAXIMUM_PROTOCOL_PAYLOAD_BYTES, verify_request, verify_wire_envelope};
use hm_schema::wire::{RequestPayload, WirePayload};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tonic::Status;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ListenerRole {
    Actor,
    Admin,
}

#[derive(Clone)]
pub struct Gateway {
    pub(crate) config: Arc<ServerConfig>,
    pub(crate) role: ListenerRole,
    permits: Arc<Semaphore>,
}

pub(crate) struct Session {
    pub stream: UnixStream,
    pub welcome: Vec<u8>,
    pub principal: Principal,
    pub proto_version: u16,
    pub permit: OwnedSemaphorePermit,
}

impl Gateway {
    #[must_use]
    pub fn new(config: Arc<ServerConfig>, role: ListenerRole) -> Self {
        Self {
            permits: Arc::new(Semaphore::new(config.maximum_connections)),
            config,
            role,
        }
    }

    pub(crate) async fn connect(&self, encoded: &[u8]) -> Result<Session, Status> {
        let envelope = verify_wire_envelope(encoded).map_err(protocol_error)?;
        let WirePayload::Hello(hello) = envelope.payload else {
            return Err(Status::invalid_argument("first envelope must be Hello"));
        };
        if envelope.proto_version != hello.proto_version {
            return Err(Status::invalid_argument("Hello protocol version mismatch"));
        }
        let principal = auth::authenticate(&self.config, &hello.capability_token)
            .map_err(|_| Status::unauthenticated("invalid capability token"))?;
        if (self.role == ListenerRole::Admin) != (principal == Principal::Admin) {
            return Err(Status::permission_denied(
                "actor and admin listeners are separate",
            ));
        }
        let permit = self
            .permits
            .clone()
            .try_acquire_owned()
            .map_err(|_| Status::resource_exhausted("remote connection limit"))?;
        let (stream, welcome) = tokio::time::timeout(Duration::from_secs(30), async {
            let mut stream = UnixStream::connect(&self.config.socket_path)
                .await
                .map_err(|_| Status::unavailable("daemon connection unavailable"))?;
            write_envelope(&mut stream, encoded).await?;
            let welcome = read_envelope(&mut stream).await?;
            Ok::<_, Status>((stream, welcome))
        })
        .await
        .map_err(|_| Status::deadline_exceeded("daemon handshake deadline"))??;
        let verified = verify_wire_envelope(&welcome).map_err(protocol_error)?;
        let WirePayload::Welcome(value) = verified.payload else {
            return Err(Status::unauthenticated(
                "daemon rejected capability handshake",
            ));
        };
        let expected_actor = match principal {
            Principal::Actor(actor) => actor,
            Principal::Admin => 0,
        };
        if value.admin != (principal == Principal::Admin)
            || value.actor_ns != expected_actor
            || value.proto_version != envelope.proto_version
        {
            return Err(Status::permission_denied("daemon identity mismatch"));
        }
        Ok(Session {
            stream,
            welcome,
            principal,
            proto_version: envelope.proto_version,
            permit,
        })
    }

    pub(crate) fn validate_request(
        session: &Session,
        encoded: &[u8],
        subscribe: bool,
    ) -> Result<u64, Status> {
        let request = verify_request(encoded).map_err(protocol_error)?;
        if request.proto_version != session.proto_version {
            return Err(Status::invalid_argument(
                "request protocol differs from Hello",
            ));
        }
        if matches!(request.request.payload, RequestPayload::Subscribe(_)) != subscribe {
            return Err(Status::invalid_argument(
                "Subscribe requires the streaming method",
            ));
        }
        auth::authorize(session.principal, &request.request.payload)
            .map_err(|_| Status::permission_denied("request is outside capability role"))?;
        Ok(request.request.request_id)
    }

    pub(crate) async fn exchange(&self, hello: &[u8], request: &[u8]) -> Result<Vec<u8>, Status> {
        let mut session = self.connect(hello).await.map_err(not_dispatched)?;
        let request_id =
            Self::validate_request(&session, request, false).map_err(not_dispatched)?;
        tokio::time::timeout(Duration::from_secs(30), async {
            write_envelope(&mut session.stream, request).await?;
            let response = read_envelope(&mut session.stream).await?;
            let decoded = verify_wire_envelope(&response).map_err(protocol_error)?;
            if !matches!(decoded.payload, WirePayload::Response(ref value) if value.request_id == request_id) {
                return Err(Status::data_loss("unexpected daemon response"));
            }
            Ok(response)
        }).await.map_err(|_| Status::deadline_exceeded("daemon response deadline"))
            .and_then(std::convert::identity).map_err(unknown_effect)
    }
}

fn not_dispatched(mut status: Status) -> Status {
    status.metadata_mut().insert(
        "effect-state",
        tonic::metadata::MetadataValue::from_static("not_dispatched"),
    );
    status
}

fn unknown_effect(mut status: Status) -> Status {
    status.metadata_mut().insert(
        "effect-state",
        tonic::metadata::MetadataValue::from_static("unknown"),
    );
    status
}

pub(crate) fn protocol_error(error: hm_core::Error) -> Status {
    Status::invalid_argument(error.code.as_str())
}

pub(crate) async fn write_envelope(stream: &mut UnixStream, payload: &[u8]) -> Result<(), Status> {
    let frame = encode_frame(payload).map_err(protocol_error)?;
    stream
        .write_all(&frame)
        .await
        .map_err(|_| Status::unavailable("daemon write failed; mutation effect unknown"))
}

pub(crate) async fn read_envelope(stream: &mut UnixStream) -> Result<Vec<u8>, Status> {
    let mut header = [0; 8];
    stream
        .read_exact(&mut header)
        .await
        .map_err(|_| Status::unavailable("daemon stream ended"))?;
    let length = u32::from_le_bytes(header[..4].try_into().expect("fixed length")) as usize;
    if length == 0 || length > MAXIMUM_PROTOCOL_PAYLOAD_BYTES {
        return Err(Status::data_loss("invalid daemon frame length"));
    }
    let mut frame = Vec::with_capacity(8 + length);
    frame.extend_from_slice(&header);
    frame.resize(8 + length, 0);
    stream
        .read_exact(&mut frame[8..])
        .await
        .map_err(|_| Status::unavailable("daemon frame interrupted"))?;
    let mut parser = FrameParser::default();
    let mut parsed = parser
        .push(&frame)
        .map_err(|_| Status::data_loss("invalid daemon frame checksum"))?;
    let payload = parsed
        .pop()
        .ok_or_else(|| Status::data_loss("missing daemon envelope"))?;
    verify_wire_envelope(&payload).map_err(|_| Status::data_loss("invalid daemon envelope"))?;
    Ok(payload)
}
