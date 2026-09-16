#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

pub mod bridge;
mod tls;
pub use bridge::{Gateway, ListenerRole};
pub use tls::TlsIdentity;

pub mod wire {
    #![allow(clippy::all, clippy::pedantic)]

    tonic::include_proto!("hypermind.v3");
}

use bridge::{read_envelope, write_envelope};
use hm_schema::protocol::{MAXIMUM_PROTOCOL_PAYLOAD_BYTES, verify_wire_envelope};
use hm_schema::wire::WirePayload;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tokio::net::TcpListener;
use tokio::sync::{OwnedSemaphorePermit, mpsc};
use tokio_stream::Stream;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{Request, Response, Status};
use wire::hyper_mind_server::{HyperMind, HyperMindServer};
use wire::{Envelope, ExchangeRequest};

pub struct GrpcServer {
    listener: TcpListener,
    gateway: Gateway,
    tls: TlsIdentity,
}

impl GrpcServer {
    pub async fn bind(
        address: SocketAddr,
        gateway: Gateway,
        tls: TlsIdentity,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        tls.tonic_config()?;
        let listener = TcpListener::bind(address).await?;
        Ok(Self {
            listener,
            gateway,
            tls,
        })
    }

    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    pub async fn serve_until(
        self,
        shutdown: impl Future<Output = ()> + Send + 'static,
    ) -> Result<(), tonic::transport::Error> {
        let service = HyperMindServer::new(RemoteService(self.gateway))
            .max_decoding_message_size(MAXIMUM_PROTOCOL_PAYLOAD_BYTES + 4096)
            .max_encoding_message_size(MAXIMUM_PROTOCOL_PAYLOAD_BYTES + 1024);
        tonic::transport::Server::builder()
            .tls_config(self.tls.tonic_config().expect("validated TLS identity"))?
            .add_service(service)
            .serve_with_incoming_shutdown(TcpListenerStream::new(self.listener), shutdown)
            .await
    }
}

struct RemoteService(Gateway);

#[tonic::async_trait]
impl HyperMind for RemoteService {
    async fn connect(&self, request: Request<Envelope>) -> Result<Response<Envelope>, Status> {
        let session = self.0.connect(&request.into_inner().ncpr).await?;
        Ok(Response::new(Envelope {
            ncpr: session.welcome,
        }))
    }

    async fn exchange(
        &self,
        request: Request<ExchangeRequest>,
    ) -> Result<Response<Envelope>, Status> {
        let request = request.into_inner();
        let ncpr = self.0.exchange(&request.hello, &request.request).await?;
        Ok(Response::new(Envelope { ncpr }))
    }

    type SubscribeStream = BoundedStream;

    async fn subscribe(
        &self,
        request: Request<ExchangeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        let request = request.into_inner();
        let mut session = self.0.connect(&request.hello).await?;
        let request_id = Gateway::validate_request(&session, &request.request, true)?;
        write_envelope(&mut session.stream, &request.request).await?;
        let ack = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            read_envelope(&mut session.stream),
        )
        .await
        .map_err(|_| Status::deadline_exceeded("subscription acknowledgment deadline"))??;
        let decoded = verify_wire_envelope(&ack).map_err(bridge::protocol_error)?;
        if !matches!(decoded.payload, WirePayload::Response(ref value) if value.request_id == request_id)
        {
            return Err(Status::data_loss("unexpected subscription acknowledgment"));
        }
        let (sender, receiver) = mpsc::channel(self.0.config.maximum_output_frames);
        let bytes = Arc::new(AtomicUsize::new(ack.len()));
        if ack.len() > self.0.config.maximum_output_bytes {
            return Err(Status::resource_exhausted("subscription byte limit"));
        }
        sender
            .try_send(Envelope { ncpr: ack })
            .map_err(|_| Status::resource_exhausted("subscription queue"))?;
        let terminal = Arc::new(Mutex::new(None));
        if matches!(decoded.payload, WirePayload::Response(ref value) if value.status == hm_schema::wire::ResponseStatus::Error)
        {
            return Ok(Response::new(BoundedStream {
                receiver,
                bytes,
                terminal,
                _permit: session.permit,
            }));
        }
        let producer_bytes = bytes.clone();
        let producer_terminal = terminal.clone();
        let maximum_bytes = self.0.config.maximum_output_bytes;
        tokio::spawn(async move {
            loop {
                let read = tokio::select! {
                    () = sender.closed() => return,
                    read = read_envelope(&mut session.stream) => read,
                };
                let result = read.and_then(|ncpr| {
                    let length = ncpr.len();
                    producer_bytes
                        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                            current
                                .checked_add(length)
                                .filter(|value| *value <= maximum_bytes)
                        })
                        .map_err(|_| Status::resource_exhausted("subscription byte limit"))?;
                    if sender.try_send(Envelope { ncpr }).is_err() {
                        producer_bytes.fetch_sub(length, Ordering::Relaxed);
                        return Err(Status::resource_exhausted("subscription frame limit"));
                    }
                    Ok(())
                });
                if let Err(error) = result {
                    *producer_terminal.lock().expect("terminal lock") = Some(error);
                    return;
                }
            }
        });
        Ok(Response::new(BoundedStream {
            receiver,
            bytes,
            terminal,
            _permit: session.permit,
        }))
    }
}

pub struct BoundedStream {
    receiver: mpsc::Receiver<Envelope>,
    bytes: Arc<AtomicUsize>,
    terminal: Arc<Mutex<Option<Status>>>,
    _permit: OwnedSemaphorePermit,
}

impl Stream for BoundedStream {
    type Item = Result<Envelope, Status>;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.receiver.poll_recv(context) {
            Poll::Ready(Some(envelope)) => {
                self.bytes.fetch_sub(envelope.ncpr.len(), Ordering::Relaxed);
                Poll::Ready(Some(Ok(envelope)))
            }
            Poll::Ready(None) => {
                Poll::Ready(self.terminal.lock().expect("terminal lock").take().map(Err))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
