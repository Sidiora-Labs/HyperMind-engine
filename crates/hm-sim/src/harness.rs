#![allow(clippy::missing_errors_doc)]

use hm_compose::tokens::FallbackWeights;
use hm_schema::protocol::{CURRENT_PROTOCOL_VERSION, encode_wire_envelope, verify_wire_envelope};
use hm_schema::wire::{
    Activate, BytesResult, Hello, Request, RequestPayload, ResponsePayload, ResponseStatus,
    WireEnvelope, WirePayload,
};
use hm_serve::config::load;
use hm_serve::protocol::{FrameParser, encode_frame};
use hm_serve::uds::UdsServer;
use rmcp::model::CallToolRequestParams;
use rmcp::service::RunningService;
use rmcp::transport::TokioChildProcess;
use rmcp::{RoleClient, ServiceExt};
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

pub type HarnessResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub const ACTOR: u16 = 7;
const ACTOR_TOKEN: [u8; 32] = [0x44; 32];

pub struct JourneyHarness {
    config_path: PathBuf,
    socket_path: PathBuf,
    executable: PathBuf,
}

impl JourneyHarness {
    pub fn create(root: &Path, executable: PathBuf) -> HarnessResult<Self> {
        fs::create_dir_all(root)?;
        let socket_path = root.join("hypermind.sock");
        let data_directory = root.join("data");
        fs::create_dir_all(&data_directory)?;
        let config_path = root.join("hypermind.conf");
        let contents = format!(
            "socket={}\ndata={}\nuser={}\nkek={}\nadmin_token={}\nactor={ACTOR}:{}\nprojection_map_bytes={}\n",
            socket_path.display(),
            data_directory.display(),
            "11".repeat(16),
            "22".repeat(32),
            "33".repeat(32),
            "44".repeat(32),
            16 * 1024 * 1024,
        );
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&config_path)?;
        file.write_all(contents.as_bytes())?;
        file.sync_all()?;
        Ok(Self {
            config_path,
            socket_path,
            executable,
        })
    }

    pub async fn start_mcp(&self) -> HarnessResult<McpClient> {
        self.start_mcp_mode("mcp").await
    }

    #[allow(dead_code)]
    #[must_use]
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    pub async fn start_mcp_mode(&self, mode: &str) -> HarnessResult<McpClient> {
        let mut command = tokio::process::Command::new(&self.executable);
        command.arg(mode).arg(&self.config_path);
        let transport = TokioChildProcess::new(command)?;
        let process_id = transport.id().ok_or("MCP child has no process id")?;
        let service = ().serve(transport).await?;
        Ok(McpClient {
            service,
            process_id,
        })
    }

    pub async fn start_uds(&self) -> HarnessResult<UdsDaemon> {
        let child = Command::new(&self.executable)
            .arg("uds")
            .arg(&self.config_path)
            .spawn()?;
        let started = Instant::now();
        while !self.socket_path.exists() {
            if started.elapsed() > Duration::from_secs(10) {
                return Err("daemon did not create its socket".into());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Ok(UdsDaemon { child })
    }

    pub async fn uds_activate(
        &self,
        conversation: &str,
        query: &str,
        budget_tokens: usize,
    ) -> HarnessResult<Vec<u8>> {
        let mut stream = UnixStream::connect(&self.socket_path).await?;
        let welcome = exchange(
            &mut stream,
            WireEnvelope {
                proto_version: CURRENT_PROTOCOL_VERSION,
                payload: WirePayload::Hello(Box::new(Hello {
                    proto_version: CURRENT_PROTOCOL_VERSION,
                    connection_id: vec![0x55; 16],
                    capability_token: ACTOR_TOKEN.to_vec(),
                })),
            },
        )
        .await?;
        if !matches!(welcome.payload, WirePayload::Welcome(_)) {
            return Err("daemon rejected actor capability".into());
        }
        let weights = FallbackWeights::default();
        let response = exchange(
            &mut stream,
            WireEnvelope {
                proto_version: CURRENT_PROTOCOL_VERSION,
                payload: WirePayload::Request(Box::new(Request {
                    request_id: 1,
                    payload: RequestPayload::Activate(Box::new(Activate {
                        conversation: hm_core::ConversationId::derive(conversation)
                            .into_bytes()
                            .to_vec(),
                        query: query.as_bytes().to_vec(),
                        turn_text: None,
                        budget_tokens: u64::try_from(budget_tokens)?,
                        query_embedding: None,
                        query_binary_prefilter: None,
                        temporal_from_ns: 0,
                        temporal_to_ns: 0,
                        token_weights: Some(weights.per_byte_q8.to_vec()),
                        token_item_overhead: weights.item_overhead,
                    })),
                })),
            },
        )
        .await?;
        let WirePayload::Response(response) = response.payload else {
            return Err("daemon returned a non-response envelope".into());
        };
        if response.status != ResponseStatus::Ok {
            return Err("daemon activation failed".into());
        }
        let Some(ResponsePayload::BytesResult(result)) = response.payload else {
            return Err("daemon activation omitted canonical bytes".into());
        };
        let BytesResult { bytes } = *result;
        Ok(bytes)
    }
}

pub struct McpClient {
    service: RunningService<RoleClient, ()>,
    process_id: u32,
}

impl McpClient {
    pub async fn call(&self, name: &str, arguments: Value) -> HarnessResult<Value> {
        let arguments = arguments
            .as_object()
            .cloned()
            .ok_or("MCP tool arguments must be a JSON object")?;
        let result = self
            .service
            .call_tool(CallToolRequestParams::new(name.to_owned()).with_arguments(arguments))
            .await?;
        if result.is_error == Some(true) {
            return Err(format!("MCP tool {name} returned an error").into());
        }
        let envelope = result
            .structured_content
            .ok_or("MCP tool omitted its structured envelope")?;
        if envelope.get("ok") != Some(&Value::Bool(true)) {
            return Err(format!("MCP tool {name} envelope was not ok: {envelope}").into());
        }
        Ok(envelope)
    }

    pub async fn kill(self) -> HarnessResult<()> {
        let status = Command::new("kill")
            .arg("-KILL")
            .arg(self.process_id.to_string())
            .status()?;
        require_success(status, "kill MCP daemon")?;
        tokio::time::timeout(Duration::from_secs(10), self.service.waiting()).await??;
        Ok(())
    }

    pub async fn close(self) -> HarnessResult<()> {
        self.service.cancel().await?;
        Ok(())
    }
}

pub struct UdsDaemon {
    child: Child,
}

impl UdsDaemon {
    pub fn kill(mut self) -> HarnessResult<()> {
        self.child.kill()?;
        let status = self.child.wait()?;
        if status.success() {
            return Err("daemon unexpectedly exited successfully after SIGKILL".into());
        }
        Ok(())
    }
}

impl Drop for UdsDaemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub async fn run_mcp_daemon(config_path: &Path) -> HarnessResult<()> {
    let config = load(config_path)?;
    let capability = config.actors.first().ok_or("configuration has no actor")?;
    let actor = hm_serve::actor::ActorEngine::open(hm_serve::actor::ActorConfig {
        actor_directory: config.actor_directory(capability.actor),
        actor: hm_core::ActorId::new(capability.actor),
        user: config.user,
        kek: config.kek,
        projection_map_bytes: config.projection_map_bytes,
    })
    .await?;
    hm_mcp::serve_stdio(hm_mcp::McpServer::new(actor))
        .await
        .map_err(|error| error.to_string().into())
}

pub async fn run_uds_daemon(config_path: &Path) -> HarnessResult<()> {
    let server = UdsServer::bind(load(config_path)?).await?;
    server.serve_until(std::future::pending()).await?;
    Ok(())
}

async fn exchange(stream: &mut UnixStream, envelope: WireEnvelope) -> HarnessResult<WireEnvelope> {
    stream
        .write_all(&encode_frame(&encode_wire_envelope(&envelope))?)
        .await?;
    let mut header = [0; 8];
    stream.read_exact(&mut header).await?;
    let length = u32::from_le_bytes(header[..4].try_into()?) as usize;
    let mut payload = vec![0; length];
    stream.read_exact(&mut payload).await?;
    let mut parser = FrameParser::default();
    let frames = parser.push(&[header.as_slice(), payload.as_slice()].concat())?;
    let frame = frames.first().ok_or("daemon returned no frame")?;
    Ok(verify_wire_envelope(frame)?)
}

fn require_success(status: ExitStatus, operation: &str) -> HarnessResult<()> {
    if status.success() {
        Ok(())
    } else {
        Err(format!("{operation} failed with {status}").into())
    }
}
