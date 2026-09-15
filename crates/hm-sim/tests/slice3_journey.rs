#![forbid(unsafe_code)]
#![allow(clippy::too_many_lines)]

#[path = "../src/harness.rs"]
#[allow(dead_code)]
mod harness;

use base64::Engine as _;
use harness::{ACTOR, HarnessResult, JourneyHarness};
use hm_core::{ActorId, ConversationId, ErrorCode, LSN};
use hm_ledger::checkpoint::{Checkpoint, load_checkpoint};
use hm_ledger::frame::EventKind;
use hm_ledger::shred::{decode_deletion_receipt, verify_deletion_receipt};
use hm_mcp::McpServer;
use hm_schema::event::{CURRENT_SCHEMA_VERSION, encode_event_envelope};
use hm_schema::events::{
    Authority, DeliveredMsg, EventEnvelope, EventPayload, Retention, Sensitivity, ToolCall,
    ToolResult, UserMsg,
};
use hm_schema::protocol::{CURRENT_PROTOCOL_VERSION, encode_wire_envelope, verify_wire_envelope};
use hm_schema::wire::{
    Append, AppendAck, AppendEvent, Hello, Request, RequestPayload, ResponsePayload, WireEnvelope,
    WirePayload,
};
use hm_serve::actor::{ActorConfig, ActorEngine};
use hm_serve::config::load;
use hm_serve::protocol::{FrameParser, encode_frame};
use rmcp::model::CallToolRequestParams;
use rmcp::service::RunningService;
use rmcp::transport::TokioChildProcess;
use rmcp::{RoleClient, ServiceExt};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

const ACTOR_TOKEN: [u8; 32] = [0x44; 32];
const ADMIN_TOKEN_HEX: &str = "3333333333333333333333333333333333333333333333333333333333333333";

#[tokio::main]
async fn main() {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let result = match arguments.as_slice() {
        [mode, config] if mode == "mcp" => run_mcp_daemon(Path::new(config)).await,
        [mode, config] if mode == "uds" => harness::run_uds_daemon(Path::new(config)).await,
        [] => slice3_journey().await,
        _ => Err("invalid slice3 journey helper invocation".into()),
    };
    if let Err(error) = result {
        eprintln!("slice3 journey failed: {error}");
        std::process::exit(1);
    }
}

async fn slice3_journey() -> HarnessResult<()> {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    build_cli(&repository)?;
    let temporary = tempfile::tempdir()?;
    let executable = std::env::current_exe()?;
    let harness = JourneyHarness::create(temporary.path(), executable.clone())?;
    let socket = temporary.path().join("hypermind.sock");
    let config_path = temporary.path().join("hypermind.conf");
    let actor_directory = temporary.path().join("data").join(ACTOR.to_string());

    let daemon = harness.start_uds().await?;
    let receipt = append_turn(&socket).await?;
    if receipt.first_lsn != 1
        || receipt.last_lsn != 4
        || receipt.leaf_count != 4
        || receipt
            .last_leaf_hash
            .as_ref()
            .is_none_or(|hash| hash.len() != 32)
        || receipt
            .mmr_root
            .as_ref()
            .is_none_or(|root| root.len() != 32)
    {
        return Err("append acknowledgement omitted the MMR receipt".into());
    }
    let initial_root: [u8; 32] = receipt
        .mmr_root
        .as_deref()
        .ok_or("append acknowledgement omitted its root")?
        .try_into()?;
    daemon.kill()?;

    let checkpoint_path = latest_checkpoint(&actor_directory)?;
    let checkpoint = load_checkpoint(&checkpoint_path)?;
    if checkpoint.root != initial_root || checkpoint.leaf_count != 4 {
        return Err("checkpoint did not commit the append receipt root".into());
    }
    verify_without_key(
        &repository.join("target/debug/hm"),
        &actor_directory,
        &checkpoint_path,
        &checkpoint,
    )?;

    let client = McpProcess::start(&executable, &config_path).await?;
    let recalled = client
        .call(
            "recall",
            json!({"mode": "lexical", "query": "tripwire canary", "limit": 10}),
        )
        .await?;
    require_ok(&recalled, "post-restart recall")?;
    if !recalled["items"]
        .as_array()
        .is_some_and(|items| items.iter().any(|item| item["lsn"] == 1))
    {
        return Err("post-restart recall omitted the tripwire record".into());
    }

    let inspected = client
        .call("inspect", json!({"uri": format!("hm://{ACTOR}/lsn/4")}))
        .await?;
    require_ok(&inspected, "tool-result inspection")?;
    if inspected["provenance"] != json!(["hm://7/lsn/4", "hm://7/lsn/3"])
        || inspected["items"][1]["authority"] != "tool_observed"
        || inspected["items"][1]["integrity"]["leaf_hash"]
            .as_str()
            .is_none_or(|value| value.len() != 64)
    {
        return Err("tool result omitted its citation chain or MMR receipt".into());
    }

    require_ok(
        &client
            .call(
                "intend",
                json!({
                    "conversation": "trust-journey",
                    "action": {
                        "kind": "open_loop",
                        "loop_id": "laundering-attempt",
                        "objective": "reject assistant memory as observed evidence"
                    }
                }),
            )
            .await?,
        "open laundering test loop",
    )?;
    let laundering = client
        .call(
            "intend",
            json!({
                "conversation": "trust-journey",
                "action": {
                    "kind": "close_loop",
                    "loop_id": "laundering-attempt",
                    "reason": "done",
                    "cause": "model-authored memory claims completion",
                    "evidence_lsns": [2]
                }
            }),
        )
        .await?;
    if laundering["ok"] != false
        || laundering["effect_state"] != "rejected"
        || laundering["items"][0]["error"] != ErrorCode::CitationInvalid.as_str()
    {
        return Err("MCP did not reject the authority-laundering write".into());
    }

    let tripped = client
        .call(
            "activate",
            json!({
                "conversation": "trust-journey",
                "query": "tripwire canary",
                "turn_text": "",
                "budget_tokens": 4096
            }),
        )
        .await?;
    if tripped["ok"] != false
        || tripped["items"][0]["error"] != ErrorCode::Tripwire.as_str()
        || tripped["warnings"] != json!(["security_event:tripwire:lsn=1"])
    {
        return Err("activation did not expose the tripwire security event".into());
    }

    let final_checkpoint_path = latest_checkpoint(&actor_directory)?;
    let final_checkpoint = load_checkpoint(&final_checkpoint_path)?;
    let shredded = client
        .call(
            "forget",
            json!({"action": "crypto_shred", "admin_token": ADMIN_TOKEN_HEX}),
        )
        .await?;
    require_ok(&shredded, "crypto-shred")?;
    let receipt_bytes = base64::engine::general_purpose::STANDARD.decode(
        shredded["items"][0]["receipt_base64"]
            .as_str()
            .ok_or("crypto-shred omitted its receipt")?,
    )?;
    let deletion = decode_deletion_receipt(&receipt_bytes)?;
    verify_deletion_receipt(&deletion, &final_checkpoint.public_key)?;
    if deletion.checkpoint_root != final_checkpoint.root
        || deletion.key_fingerprint == [0; 32]
        || actor_directory.join("keys/KEYRING").exists()
    {
        return Err("crypto-shred receipt did not bind the destroyed key and final root".into());
    }
    client.close().await?;

    let reopened = ActorEngine::open(actor_config(&actor_directory)).await;
    if reopened
        .as_ref()
        .err()
        .is_none_or(|error| error.code != ErrorCode::KeyDestroyed)
    {
        return Err("crypto-shredded actor did not remain destroyed".into());
    }
    hm_eval::slice3::gate(None).map_err(|error| error.to_string())?;
    Ok(())
}

async fn append_turn(socket: &Path) -> HarnessResult<AppendAck> {
    let mut stream = UnixStream::connect(socket).await?;
    let welcome = exchange(
        &mut stream,
        WirePayload::Hello(Box::new(Hello {
            proto_version: CURRENT_PROTOCOL_VERSION,
            connection_id: vec![0x77; 16],
            capability_token: ACTOR_TOKEN.to_vec(),
        })),
    )
    .await?;
    if !matches!(welcome.payload, WirePayload::Welcome(_)) {
        return Err("daemon rejected actor capability".into());
    }
    let conversation = ConversationId::derive("trust-journey");
    let events = vec![
        append_event(
            EventKind::UserMsg,
            conversation,
            EventPayload::UserMsg(Box::new(UserMsg {
                content: b"tripwire canary".to_vec(),
            })),
            Authority::UserAsserted,
        ),
        append_event(
            EventKind::DeliveredMsg,
            conversation,
            EventPayload::DeliveredMsg(Box::new(DeliveredMsg {
                content: b"model-authored memory claims completion".to_vec(),
            })),
            Authority::AssistantGenerated,
        ),
        append_event(
            EventKind::ToolCall,
            conversation,
            EventPayload::ToolCall(Box::new(ToolCall {
                call_id: b"read-1".to_vec(),
                tool_name: "read_file".to_owned(),
                arguments: b"path=state.txt".to_vec(),
            })),
            Authority::RuntimeFact,
        ),
        append_event(
            EventKind::ToolResult,
            conversation,
            EventPayload::ToolResult(Box::new(ToolResult {
                call_id: b"read-1".to_vec(),
                tool_call_lsn: 3,
                result: b"observed state".to_vec(),
                ..ToolResult::default()
            })),
            Authority::ToolObserved,
        ),
    ];
    let response = exchange(
        &mut stream,
        WirePayload::Request(Box::new(Request {
            request_id: 1,
            payload: RequestPayload::Append(Box::new(Append {
                client_seq: 1,
                events,
            })),
        })),
    )
    .await?;
    let WirePayload::Response(response) = response.payload else {
        return Err("append returned a non-response envelope".into());
    };
    let Some(ResponsePayload::AppendAck(receipt)) = response.payload else {
        return Err("append returned no acknowledgement".into());
    };
    Ok(*receipt)
}

fn append_event(
    kind: EventKind,
    conversation: ConversationId,
    payload: EventPayload,
    authority: Authority,
) -> AppendEvent {
    AppendEvent {
        kind: kind as u8,
        conversation: conversation.into_bytes().to_vec(),
        payload: encode_event_envelope(&EventEnvelope {
            schema_version: CURRENT_SCHEMA_VERSION,
            payload,
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 0,
            origin_actor: ACTOR,
            run_id: None,
            model_provenance: None,
            authority,
            retention: Retention::Durable,
            sensitivity: Sensitivity::Personal,
            event_time_ns: 0,
        }),
    }
}

async fn exchange(stream: &mut UnixStream, payload: WirePayload) -> HarnessResult<WireEnvelope> {
    let wire = encode_wire_envelope(&WireEnvelope {
        proto_version: CURRENT_PROTOCOL_VERSION,
        payload,
    });
    stream.write_all(&encode_frame(&wire)?).await?;
    let mut header = [0; 8];
    stream.read_exact(&mut header).await?;
    let length = u32::from_le_bytes(header[..4].try_into()?) as usize;
    let mut payload = vec![0; length];
    stream.read_exact(&mut payload).await?;
    let mut parser = FrameParser::default();
    let frames = parser.push(&[header.as_slice(), payload.as_slice()].concat())?;
    Ok(verify_wire_envelope(
        frames.first().ok_or("daemon returned no frame")?,
    )?)
}

fn verify_without_key(
    hm: &Path,
    actor_directory: &Path,
    checkpoint_path: &Path,
    checkpoint: &Checkpoint,
) -> HarnessResult<()> {
    let keyring = actor_directory.join("keys/KEYRING");
    let unavailable = actor_directory.join("keys/KEYRING.offline");
    fs::rename(&keyring, &unavailable)?;
    let output = Command::new(hm)
        .arg("verify")
        .arg(actor_directory)
        .arg(ACTOR.to_string())
        .arg(checkpoint_path)
        .arg(hex(&checkpoint.public_key))
        .output();
    fs::rename(&unavailable, &keyring)?;
    let output = output?;
    require_success(&output, "offline hm verify")?;
    let stdout = String::from_utf8(output.stdout)?;
    if !stdout.contains(&hex(&checkpoint.root)) || !stdout.contains("lsn=4") {
        return Err("offline verification reported the wrong prefix".into());
    }
    Ok(())
}

fn latest_checkpoint(actor_directory: &Path) -> HarnessResult<PathBuf> {
    fs::read_dir(actor_directory.join("mmr/checkpoints"))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .ok_or_else(|| "actor has no checkpoint".into())
}

fn actor_config(actor_directory: &Path) -> ActorConfig {
    ActorConfig {
        actor_directory: actor_directory.to_owned(),
        actor: ActorId::new(ACTOR),
        user: [0x11; 16],
        kek: [0x22; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}

async fn run_mcp_daemon(config_path: &Path) -> HarnessResult<()> {
    let config = load(config_path)?;
    let capability = config.actors.first().ok_or("configuration has no actor")?;
    let actor_directory = config.actor_directory(capability.actor);
    let actor = ActorEngine::open_with_tripwires(
        ActorConfig {
            actor_directory,
            actor: ActorId::new(capability.actor),
            user: config.user,
            kek: config.kek,
            projection_map_bytes: config.projection_map_bytes,
        },
        [LSN::new(1)],
    )
    .await?;
    hm_mcp::serve_stdio(McpServer::new_with_admin(actor, config.admin_token))
        .await
        .map_err(|error| error.to_string().into())
}

struct McpProcess {
    service: RunningService<RoleClient, ()>,
}

impl McpProcess {
    async fn start(executable: &Path, config: &Path) -> HarnessResult<Self> {
        let mut command = tokio::process::Command::new(executable);
        command.arg("mcp").arg(config);
        let transport = TokioChildProcess::new(command)?;
        Ok(Self {
            service: ().serve(transport).await?,
        })
    }

    async fn call(&self, name: &str, arguments: Value) -> HarnessResult<Value> {
        let arguments = arguments
            .as_object()
            .cloned()
            .ok_or("MCP tool arguments must be a JSON object")?;
        let response = self
            .service
            .call_tool(CallToolRequestParams::new(name.to_owned()).with_arguments(arguments))
            .await?;
        response
            .structured_content
            .ok_or_else(|| format!("MCP tool {name} omitted its structured envelope").into())
    }

    async fn close(self) -> HarnessResult<()> {
        self.service.cancel().await?;
        Ok(())
    }
}

fn require_ok(envelope: &Value, operation: &str) -> HarnessResult<()> {
    if envelope["ok"] == true {
        Ok(())
    } else {
        Err(format!("{operation} failed: {envelope}").into())
    }
}

fn build_cli(repository: &Path) -> HarnessResult<()> {
    let output = Command::new("cargo")
        .args(["build", "-p", "hm-cli"])
        .current_dir(repository)
        .output()?;
    require_success(&output, "build hm CLI")
}

fn require_success(output: &Output, operation: &str) -> HarnessResult<()> {
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{operation} failed with {}\n{}{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .into())
    }
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(
        String::with_capacity(bytes.len() * 2),
        |mut output, byte| {
            let _ = write!(output, "{byte:02x}");
            output
        },
    )
}
