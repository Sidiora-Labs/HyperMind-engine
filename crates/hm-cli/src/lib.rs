#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc)]

use anyhow::{Context, Result, anyhow};
use base64::Engine as _;
use clap::{Parser, Subcommand, ValueEnum};
use hm_core::{ActorId, ConversationId, ErrorCode};
use hm_ledger::frame::EventKind;
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    Authority, DeliveredMsg, EventEnvelope, EventPayload, Retention, Sensitivity, UserMsg,
};
use hm_schema::protocol::{CURRENT_PROTOCOL_VERSION, encode_wire_envelope, verify_wire_envelope};
use hm_schema::wire::{
    Activate, Append, AppendEvent, Hello, Recall, RecallMode, Request, RequestPayload,
    ResponsePayload, WireEnvelope, WirePayload,
};
use hm_serve::config::{ActorCapability, ServerConfig, load};
use hm_serve::embedded::{EmbeddedConfig, HyperMind, MemoryKind, RenderModel, render};
use hm_serve::protocol::{FrameParser, encode_frame};
use hm_serve::uds::UdsServer;
use serde_json::{Value, json};
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::os::unix::fs::{FileTypeExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

mod consolidate;
mod verify;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Parser)]
#[command(name = "hm", version, about = "HyperMind memory engine")]
struct Cli {
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Init {
        #[arg(long)]
        path: PathBuf,
        #[arg(long, default_value_t = 1)]
        actor: u16,
    },
    Serve {
        #[arg(long)]
        config: PathBuf,
    },
    Doctor {
        #[arg(long)]
        config: PathBuf,
    },
    Remember {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        conversation: String,
        #[arg(long)]
        content: String,
        #[arg(long, value_enum, default_value_t = MessageKind::User)]
        kind: MessageKind,
        #[arg(long)]
        embedded: bool,
    },
    Recall {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        query: String,
        #[arg(long, default_value_t = 32)]
        limit: usize,
        #[arg(long)]
        embedded: bool,
    },
    Activate {
        #[arg(long)]
        config: PathBuf,
        #[arg(long)]
        conversation: String,
        #[arg(long, default_value = "")]
        query: String,
        #[arg(long)]
        budget_tokens: usize,
        #[arg(long)]
        embedded: bool,
    },
    Consolidate {
        #[command(subcommand)]
        command: consolidate::Command,
    },
    Verify {
        actor_directory: PathBuf,
        actor: u16,
        checkpoint: PathBuf,
        public_key: String,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum MessageKind {
    User,
    Assistant,
}

pub async fn run(arguments: impl IntoIterator<Item = impl Into<OsString> + Clone>) -> Result<()> {
    let cli = Cli::try_parse_from(arguments)?;
    let value = execute(cli.command).await?;
    if !cli.json
        && let Some(display) = value.get("display").and_then(Value::as_str)
    {
        println!("{display}");
        return Ok(());
    }
    if cli.json || std::env::var_os("NO_COLOR").is_some() {
        println!("{}", serde_json::to_string(&value)?);
    } else {
        println!("{}", serde_json::to_string_pretty(&value)?);
    }
    Ok(())
}

async fn execute(command: Command) -> Result<Value> {
    match command {
        Command::Init { path, actor } => initialize(&path, actor).await,
        Command::Serve { config } => {
            let dispatcher =
                tokio::task::spawn_blocking(hm_mcp::dispatcher::McpToolDispatcher::from_env)
                    .await??;
            let server = UdsServer::bind(load(&config)?)
                .await?
                .with_tool_dispatcher(std::sync::Arc::new(dispatcher));
            server
                .serve_until(async {
                    let _ = tokio::signal::ctrl_c().await;
                })
                .await?;
            Ok(json!({"ok": true, "stopped": true}))
        }
        Command::Doctor { config } => doctor(&config),
        Command::Remember {
            config,
            conversation,
            content,
            kind,
            embedded,
        } => remember(&config, &conversation, &content, kind, embedded).await,
        Command::Recall {
            config,
            query,
            limit,
            embedded,
        } => recall(&config, &query, limit, embedded).await,
        Command::Activate {
            config,
            conversation,
            query,
            budget_tokens,
            embedded,
        } => activate(&config, &conversation, &query, budget_tokens, embedded).await,
        Command::Consolidate { command } => consolidate::execute(command).await,
        Command::Verify {
            actor_directory,
            actor,
            checkpoint,
            public_key,
        } => verify::run(&actor_directory, actor, &checkpoint, &public_key),
    }
}

async fn initialize(path: &Path, actor: u16) -> Result<Value> {
    if actor == 0 {
        return Err(anyhow!(ErrorCode::InvalidArgument));
    }
    fs::create_dir_all(path)?;
    let data = path.join("data");
    fs::create_dir_all(&data)?;
    let socket = path.join("hypermind.sock");
    let config_path = path.join("hypermind.conf");
    let mut user = [0; 16];
    let mut kek = [0; 32];
    let mut admin_token = [0; 32];
    let mut actor_token = [0; 32];
    getrandom::fill(&mut user)?;
    getrandom::fill(&mut kek)?;
    getrandom::fill(&mut admin_token)?;
    getrandom::fill(&mut actor_token)?;
    let contents = format!(
        "socket={}\ndata={}\nuser={}\nkek={}\nadmin_token={}\nactor={actor}:{}\nprojection_map_bytes={}\n",
        socket.display(),
        data.display(),
        hex(&user),
        hex(&kek),
        hex(&admin_token),
        hex(&actor_token),
        256 * 1024 * 1024,
    );
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&config_path)
        .with_context(|| format!("create {}", config_path.display()))?;
    file.write_all(contents.as_bytes())?;
    file.sync_all()?;
    let hypermind = HyperMind::open(
        &data,
        EmbeddedConfig {
            actor: ActorId::new(actor),
            user,
            kek,
            projection_map_bytes: 256 * 1024 * 1024,
        },
    )
    .await?;
    let actor_handle = hypermind.actor();
    drop(hypermind);
    actor_handle.shutdown().await?;
    Ok(json!({
        "ok": true,
        "config": config_path,
        "actor": actor,
        "socket": socket,
    }))
}

fn doctor(path: &Path) -> Result<Value> {
    let config = load(path)?;
    let rustc = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned());
    let actor_directories: Vec<Value> = config
        .actors
        .iter()
        .map(|actor| {
            let path = config.actor_directory(actor.actor);
            json!({"actor": actor.actor, "path": path, "present": path.is_dir()})
        })
        .collect();
    let socket_ready = fs::symlink_metadata(&config.socket_path)
        .is_ok_and(|metadata| metadata.file_type().is_socket());
    Ok(json!({
        "ok": rustc.is_some() && actor_directories.iter().all(|actor| actor["present"] == true),
        "toolchain": rustc,
        "actor_directories": actor_directories,
        "socket_ready": socket_ready,
        "missing_models": [],
    }))
}

async fn remember(
    path: &Path,
    conversation: &str,
    content: &str,
    kind: MessageKind,
    embedded: bool,
) -> Result<Value> {
    let config = load(path)?;
    if embedded {
        let hypermind = open_embedded(&config).await?;
        let session = hypermind.session(conversation);
        let lsn = session
            .remember(
                match kind {
                    MessageKind::User => MemoryKind::User,
                    MessageKind::Assistant => MemoryKind::Assistant,
                },
                content,
            )
            .await?;
        let actor = hypermind.actor();
        drop(session);
        drop(hypermind);
        actor.shutdown().await?;
        return Ok(json!({"ok": true, "first_lsn": lsn.get(), "last_lsn": lsn.get()}));
    }
    let capability = first_actor(&config)?;
    let mut client = DaemonClient::connect(&config, capability).await?;
    let (event_kind, payload, authority) = message_payload(kind, content);
    let response = client
        .request(RequestPayload::Append(Box::new(Append {
            client_seq: 1,
            events: vec![AppendEvent {
                kind: event_kind as u8,
                conversation: ConversationId::derive(conversation).into_bytes().to_vec(),
                payload: encode_message(payload, authority),
            }],
        })))
        .await?;
    let ResponsePayload::AppendAck(ack) = response else {
        return Err(anyhow!(ErrorCode::ProtocolInvalid));
    };
    Ok(json!({"ok": true, "first_lsn": ack.first_lsn, "last_lsn": ack.last_lsn}))
}

async fn recall(path: &Path, query: &str, limit: usize, embedded: bool) -> Result<Value> {
    let config = load(path)?;
    if embedded {
        let hypermind = open_embedded(&config).await?;
        let session = hypermind.session("cli-recall");
        let items = session.recall(query, limit).await?;
        let value = json!({
            "ok": true,
            "items": items.iter().map(|item| json!({
                "lsn": item.lsn.get(),
                "conversation": item.conversation.to_string(),
                "score_q32": item.score_q32,
            })).collect::<Vec<_>>(),
        });
        let actor = hypermind.actor();
        drop(session);
        drop(hypermind);
        actor.shutdown().await?;
        return Ok(value);
    }
    let capability = first_actor(&config)?;
    let mut client = DaemonClient::connect(&config, capability).await?;
    let response = client
        .request(RequestPayload::Recall(Box::new(Recall {
            query: query.as_bytes().to_vec(),
            limit: u32::try_from(limit)?,
            mode: RecallMode::ListWindows,
            level: 1,
            start_ns: 0,
            end_ns: 0,
        })))
        .await?;
    let ResponsePayload::RecallResult(result) = response else {
        return Err(anyhow!(ErrorCode::ProtocolInvalid));
    };
    Ok(json!({"ok": true, "lsns": result.members.unwrap_or_default()}))
}

async fn activate(
    path: &Path,
    conversation: &str,
    query: &str,
    budget_tokens: usize,
    embedded: bool,
) -> Result<Value> {
    let config = load(path)?;
    if embedded {
        let hypermind = open_embedded(&config).await?;
        let session = hypermind.session(conversation);
        let bundle = session.activate(query, budget_tokens).await?;
        let rendered = render(&bundle, RenderModel::PlainText)?;
        let value = json!({
            "ok": true,
            "bundle_hash": hex(&bundle.bundle_hash),
            "spent_tokens": bundle.spent_tokens,
            "sections": rendered.sections.iter().map(|section| json!({
                "tier": format!("{:?}", section.tier).to_lowercase(),
                "items": section.items.iter().map(|item| json!({
                    "role": item.role,
                    "authority": authority_name(item.source_authority),
                    "trust": "untrusted_memory",
                    "uri": item.provenance_uri,
                    "content": item.content,
                })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        });
        let actor = hypermind.actor();
        drop(session);
        drop(hypermind);
        actor.shutdown().await?;
        return Ok(value);
    }
    let capability = first_actor(&config)?;
    let mut client = DaemonClient::connect(&config, capability).await?;
    let response = client
        .request(RequestPayload::Activate(Box::new(Activate {
            conversation: ConversationId::derive(conversation).into_bytes().to_vec(),
            query: query.as_bytes().to_vec(),
            turn_text: None,
            budget_tokens: u64::try_from(budget_tokens)?,
            query_embedding: None,
            query_binary_prefilter: None,
            temporal_from_ns: 0,
            temporal_to_ns: 0,
            token_weights: Some(vec![256; 256]),
            token_item_overhead: 0,
        })))
        .await?;
    let ResponsePayload::BytesResult(result) = response else {
        return Err(anyhow!(ErrorCode::ProtocolInvalid));
    };
    Ok(json!({
        "ok": true,
        "canonical_bundle_base64": base64::engine::general_purpose::STANDARD.encode(result.bytes),
    }))
}

fn first_actor(config: &ServerConfig) -> Result<&ActorCapability> {
    config
        .actors
        .first()
        .ok_or_else(|| anyhow!(ErrorCode::InvalidArgument))
}

async fn open_embedded(config: &ServerConfig) -> Result<HyperMind> {
    let actor = first_actor(config)?;
    Ok(HyperMind::open(
        &config.data_directory,
        EmbeddedConfig {
            actor: ActorId::new(actor.actor),
            user: config.user,
            kek: config.kek,
            projection_map_bytes: config.projection_map_bytes,
        },
    )
    .await?)
}

fn message_payload(kind: MessageKind, content: &str) -> (EventKind, EventPayload, Authority) {
    match kind {
        MessageKind::User => (
            EventKind::UserMsg,
            EventPayload::UserMsg(Box::new(UserMsg {
                content: content.as_bytes().to_vec(),
            })),
            Authority::UserAsserted,
        ),
        MessageKind::Assistant => (
            EventKind::DeliveredMsg,
            EventPayload::DeliveredMsg(Box::new(DeliveredMsg {
                content: content.as_bytes().to_vec(),
            })),
            Authority::AssistantGenerated,
        ),
    }
}

fn encode_message(payload: EventPayload, authority: Authority) -> Vec<u8> {
    encode_event_envelope(&EventEnvelope {
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
    })
}

struct DaemonClient {
    stream: UnixStream,
    next_request_id: u64,
}

impl DaemonClient {
    async fn connect(config: &ServerConfig, capability: &ActorCapability) -> Result<Self> {
        let mut stream = UnixStream::connect(&config.socket_path).await?;
        let mut connection_id = [0; 16];
        getrandom::fill(&mut connection_id)?;
        let hello = WireEnvelope {
            proto_version: CURRENT_PROTOCOL_VERSION,
            payload: WirePayload::Hello(Box::new(Hello {
                proto_version: CURRENT_PROTOCOL_VERSION,
                connection_id: connection_id.to_vec(),
                capability_token: capability.token.to_vec(),
            })),
        };
        exchange(&mut stream, hello).await?;
        Ok(Self {
            stream,
            next_request_id: 1,
        })
    }

    async fn request(&mut self, payload: RequestPayload) -> Result<ResponsePayload> {
        let request_id = self.next_request_id;
        self.next_request_id += 1;
        let envelope = exchange(
            &mut self.stream,
            WireEnvelope {
                proto_version: CURRENT_PROTOCOL_VERSION,
                payload: WirePayload::Request(Box::new(Request {
                    request_id,
                    payload,
                })),
            },
        )
        .await?;
        let WirePayload::Response(response) = envelope.payload else {
            return Err(anyhow!(ErrorCode::ProtocolInvalid));
        };
        if response.request_id != request_id {
            return Err(anyhow!(ErrorCode::ProtocolInvalid));
        }
        match response.payload {
            Some(ResponsePayload::ErrorDetail(error)) => Err(anyhow!(
                ErrorCode::try_from(error.code).unwrap_or(ErrorCode::ProtocolInvalid)
            )),
            Some(payload) => Ok(payload),
            None => Err(anyhow!(ErrorCode::ProtocolInvalid)),
        }
    }
}

async fn exchange(stream: &mut UnixStream, envelope: WireEnvelope) -> Result<WireEnvelope> {
    let frame = encode_frame(&encode_wire_envelope(&envelope))?;
    stream.write_all(&frame).await?;
    let mut header = [0; 8];
    stream.read_exact(&mut header).await?;
    let length = u32::from_le_bytes([header[0], header[1], header[2], header[3]]) as usize;
    let mut payload = vec![0; length];
    stream.read_exact(&mut payload).await?;
    let mut parser = FrameParser::default();
    let parsed = parser.push(&[header.as_slice(), payload.as_slice()].concat())?;
    parsed
        .first()
        .ok_or_else(|| anyhow!(ErrorCode::ProtocolInvalid))
        .and_then(|payload| verify_wire_envelope(payload).map_err(Into::into))
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

const fn authority_name(authority: Authority) -> &'static str {
    match authority {
        Authority::UserAsserted => "user_asserted",
        Authority::ExternalObserved => "external_observed",
        Authority::ToolObserved => "tool_observed",
        Authority::RuntimeFact => "runtime_fact",
        Authority::AssistantGenerated => "assistant_generated",
        Authority::DerivedInference => "derived_inference",
    }
}
