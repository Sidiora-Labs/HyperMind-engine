#![forbid(unsafe_code)]

use hm_context::{
    historian::{
        select_chunks_with_spans, ChunkLimits, HistorianClaim, HistorianResult, SummaryTier,
    },
    history::SourceRelation,
    maintenance::{JobKind, JobLease, JobRequest, Usage},
    types::{
        digest_bytes, Authority, Cursor, MessagePart, MessageRole, Scope, SourceMessage, SourceSpan,
    },
};
use hm_fabric::{
    effects::EffectState,
    runtime::{RuntimeConfig, RuntimeService, WorkerRequest},
    supervisor::{Probe, ProcessSpec, RestartPolicy},
};
use hm_schema::{
    protocol::{
        encode_wire_envelope, verify_wire_envelope, CURRENT_PROTOCOL_VERSION,
        MAXIMUM_PROTOCOL_PAYLOAD_BYTES,
    },
    wire::{
        Hello, Request, RequestPayload, ResponsePayload, ResponseStatus, ToolRequest, WireEnvelope,
        WirePayload,
    },
};
use hm_serve::{
    context_history::{RelationIngestion, SourceIngestion},
    context_jobs::{ContextJobAction, ContextJobRequest},
    hypermid_import::{ImportBundle, ImportEntry},
    protocol::{encode_frame, FrameParser},
};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    process::Stdio,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
    process::{Child, Command},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
const ACTOR: u16 = 7;
const SESSION: &str = "pressure-session";
const CONVERSATION: &str = "pressure-conversation";
const TOKEN: [u8; 32] = [0x44; 32];

fn scope() -> Scope {
    Scope {
        owner_id: "journey-owner".into(),
        project_id: "journey-project".into(),
        workspace_id: Some("workspace".into()),
    }
}
fn executable() -> Result<PathBuf> {
    let path = std::env::var_os("HM_DAEMON_BIN")
        .map(PathBuf::from)
        .unwrap_or(
            std::env::current_exe()?
                .parent()
                .ok_or("test executable has no directory")?
                .parent()
                .ok_or("test executable has no target directory")?
                .join("hm"),
        );
    if !path.is_file() {
        return Err(format!(
            "actual hm executable required at {}; build hm-cli or set HM_DAEMON_BIN",
            path.display()
        )
        .into());
    }
    Ok(path)
}
fn private_file(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}
struct Daemon {
    child: Child,
}
impl Daemon {
    async fn start(
        binary: &Path,
        config: &Path,
        binding: &Path,
        socket: &Path,
        log: &Path,
    ) -> Result<Self> {
        let stderr = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(log)?;
        let mut child = Command::new(binary)
            .args(["--json", "serve", "--config"])
            .arg(config)
            .arg("--context-scope")
            .arg(binding)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(stderr)
            .kill_on_drop(true)
            .spawn()?;
        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait()? {
                return Err(format!("daemon exited {status}: {}", fs::read_to_string(log)?).into());
            }
            if socket.exists() && UnixStream::connect(socket).await.is_ok() {
                return Ok(Self { child });
            }
            if started.elapsed() > Duration::from_secs(15) {
                return Err(
                    format!("daemon did not become ready: {}", fs::read_to_string(log)?).into(),
                );
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }
    async fn crash(mut self) -> Result<()> {
        self.child.kill().await?;
        self.child.wait().await?;
        Ok(())
    }
}
struct Client {
    stream: UnixStream,
    next: u64,
}
impl Client {
    async fn connect(socket: &Path) -> Result<Self> {
        let mut stream = UnixStream::connect(socket).await?;
        let reply = exchange(
            &mut stream,
            WirePayload::Hello(Box::new(Hello {
                proto_version: CURRENT_PROTOCOL_VERSION,
                connection_id: vec![0x55; 16],
                capability_token: TOKEN.to_vec(),
            })),
        )
        .await?;
        if !matches!(reply.payload, WirePayload::Welcome(_)) {
            return Err("daemon rejected actor authentication".into());
        }
        Ok(Self { stream, next: 0 })
    }
    async fn raw(&mut self, verb: &str, arguments: Value) -> Result<Value> {
        self.next = self.next.checked_add(1).ok_or("request id exhausted")?;
        let response = exchange(
            &mut self.stream,
            WirePayload::Request(Box::new(Request {
                request_id: self.next,
                payload: RequestPayload::ToolRequest(Box::new(ToolRequest {
                    verb: verb.into(),
                    arguments_json: serde_json::to_vec(&arguments)?,
                })),
            })),
        )
        .await?;
        let WirePayload::Response(response) = response.payload else {
            return Err("daemon omitted response envelope".into());
        };
        if response.request_id != self.next || response.status != ResponseStatus::Ok {
            return Err(format!("daemon wire failure: {response:?}").into());
        }
        let Some(ResponsePayload::BytesResult(bytes)) = response.payload else {
            return Err("daemon omitted tool envelope".into());
        };
        Ok(serde_json::from_slice(&bytes.bytes)?)
    }
    async fn call(&mut self, verb: &str, args: Value) -> Result<Value> {
        let operation = args["context"]["operation"].as_str().unwrap_or("native").to_owned();
        let action = args["context"]["request"]["action"]["action"].as_str().unwrap_or("none").to_owned();
        let value = self.raw(verb, args).await?;
        if value["ok"] != true {
            return Err(format!("{verb} operation={operation} action={action} failed: {value}").into());
        }
        Ok(value)
    }
    async fn inspect(&mut self, suffix: &str) -> Result<Value> {
        self.call(
            "inspect",
            json!({"uri":format!("hm://{ACTOR}/context/{SESSION}{suffix}")}),
        )
        .await
    }
    async fn job(&mut self, id: &str, action: ContextJobAction) -> Result<Value> {
        self.call("remember",json!({"conversation":CONVERSATION,"kind":"user","context":{"operation":"job","request":ContextJobRequest {version:1,scope:scope(),request_id:id.into(),action}}})).await
    }
    async fn activate(&mut self, tier: &str, budget: u64) -> Result<Value> {
        self.call("activate", activation(tier, budget)).await
    }
    async fn ingest(&mut self, source: &SourceIngestion) -> Result<Value> {
        self.call("remember",json!({"conversation":CONVERSATION,"kind":"user","context":{"operation":"source","request":source}})).await
    }
}
async fn exchange(stream: &mut UnixStream, payload: WirePayload) -> Result<WireEnvelope> {
    tokio::time::timeout(Duration::from_secs(20), async {
        stream
            .write_all(&encode_frame(&encode_wire_envelope(&WireEnvelope {
                proto_version: CURRENT_PROTOCOL_VERSION,
                payload,
            }))?)
            .await?;
        let mut header = [0u8; 8];
        stream.read_exact(&mut header).await?;
        let length = u32::from_le_bytes(header[..4].try_into()?) as usize;
        if length > MAXIMUM_PROTOCOL_PAYLOAD_BYTES {
            return Err("daemon frame exceeds protocol bound".into());
        }
        let mut body = vec![0; length];
        stream.read_exact(&mut body).await?;
        let mut parser = FrameParser::default();
        let frames = parser.push(&[header.as_slice(), body.as_slice()].concat())?;
        let bytes = frames.first().ok_or("daemon frame missing")?;
        Ok(verify_wire_envelope(bytes)?)
    })
    .await?
}
fn activation(tier: &str, budget: u64) -> Value {
    json!({"conversation":CONVERSATION,"query":"","turn_text":"","budget_tokens":budget,"context":{"version":1,"scope":scope(),"session_id":SESSION,"generation":0,"model_id":"gpt-4o","tier":tier,"defer_reductions":false,"budget":{"context_tokens":budget,"reserved_output_tokens":64,"required_tokens":0}}})
}
fn source(id: &str, ordinal: u64, text: String) -> Result<SourceIngestion> {
    let mut message = SourceMessage {
        id: id.into(),
        ordinal,
        role: MessageRole::User,
        parts: vec![MessagePart::Text { text: text.clone() }],
        occurred_at_ns: Some(1_791_288_000_123_456_789 + ordinal as i64),
        recorded_at_ns: 1_791_288_000_123_456_789 + ordinal as i64,
        authority: Authority::UserAsserted,
        source_digest: String::new(),
    };
    message.source_digest = message.computed_digest()?;
    let original_bytes = format!(
        "{{ \"id\": \"{id}\", \"text\": {} }}\r\n",
        serde_json::to_string(&text)?
    )
    .into_bytes();
    Ok(SourceIngestion {
        version: 1,
        scope: scope(),
        session_id: SESSION.into(),
        conversation: CONVERSATION.into(),
        message,
        original_bytes,
    })
}
async fn assert_source(client: &mut Client, request: &SourceIngestion) -> Result<()> {
    let recovered = client
        .inspect(&format!("/source/{}", request.message.id))
        .await?;
    assert_eq!(
        recovered["items"][0]["source"],
        serde_json::to_value(&request.message)?
    );
    assert_eq!(
        recovered["items"][0]["original_bytes"],
        json!(request.original_bytes)
    );
    assert_eq!(
        recovered["items"][0]["source"]["recorded_at_ns"],
        request.message.recorded_at_ns.to_string()
    );
    Ok(())
}
fn summaries(chunk: &hm_context::historian::SourceChunk) -> Result<HistorianResult> {
    let readings = chunk
        .sources
        .iter()
        .map(|message| match &message.parts[0] {
            MessagePart::Text { text } => text
                .split('.')
                .next()
                .map(str::to_owned)
                .ok_or("missing source sentence"),
            _ => Err("non-text source"),
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let tiers = [
        readings
            .iter()
            .map(|s| format!("{s}."))
            .collect::<Vec<_>>()
            .join(" "),
        readings.join("; "),
        readings
            .iter()
            .map(|s| s.strip_prefix("Measured pressure is ").unwrap_or(s))
            .collect::<Vec<_>>()
            .join(" → "),
        readings
            .iter()
            .map(|s| s.strip_prefix("Measured pressure is ").unwrap_or(s))
            .collect::<Vec<_>>()
            .join("\n"),
    ]
    .map(|text| SummaryTier {
        text,
        coverage: chunk.spans.clone(),
    });
    Ok(HistorianResult {
        source_digest: chunk.digest.clone(),
        tiers,
    })
}
fn assert_budget(value: &Value, budget: u64) {
    assert!(value["items"][0]["report"]["token_count"].as_u64().unwrap() <= budget - 64);
    assert_eq!(value["items"][0]["report"]["scope"], json!(scope()));
    assert_eq!(value["health"]["tokenizer"], "supported");
}
fn process(binary: &Path, root: &Path) -> ProcessSpec {
    ProcessSpec {
        module_id: "digest-worker".into(),
        command: binary.into(),
        args: vec!["fabric-worker".into()],
        env: BTreeMap::new(),
        cwd: root.into(),
        readiness: Probe::ProcessAlive,
        health: Probe::ProcessAlive,
        readiness_timeout: Duration::from_secs(10),
        shutdown_timeout: Duration::from_secs(1),
        stderr_bytes: 16384,
        drain_message: None,
        restart: RestartPolicy {
            max_restarts: 0,
            initial_backoff: Duration::ZERO,
            max_backoff: Duration::ZERO,
        },
    }
}

#[tokio::test]
async fn unified_context_journey() -> Result<()> {
    let binary = executable()?;
    let directory = tempfile::tempdir()?;
    let root = directory.path();
    let socket = root.join("hm.sock");
    let data = root.join("data");
    fs::create_dir(&data)?;
    let config = root.join("hm.conf");
    let binding = root.join("scope.json");
    private_file(&config,format!("socket={}\ndata={}\nuser={}\nkek={}\nadmin_token={}\nactor={ACTOR}:{}\nprojection_map_bytes={}\n",socket.display(),data.display(),"11".repeat(16),"22".repeat(32),"33".repeat(32),"44".repeat(32),16*1024*1024).as_bytes())?;
    private_file(
        &binding,
        &serde_json::to_vec(&json!({"version":1,"actor":ACTOR,"scope":scope()}))?,
    )?;
    fs::set_permissions(&binding, fs::Permissions::from_mode(0o644))?;
    let rejected = tokio::time::timeout(
        Duration::from_secs(10),
        Command::new(&binary)
            .args(["serve", "--config"])
            .arg(&config)
            .arg("--context-scope")
            .arg(&binding)
            .output(),
    )
    .await??;
    assert!(!rejected.status.success());
    assert!(!socket.exists());
    fs::set_permissions(&binding, fs::Permissions::from_mode(0o600))?;
    let daemon = Daemon::start(
        &binary,
        &config,
        &binding,
        &socket,
        &root.join("daemon-first.log"),
    )
    .await?;
    let mut unauthenticated = UnixStream::connect(&socket).await?;
    let rejected = exchange(
        &mut unauthenticated,
        WirePayload::Hello(Box::new(Hello {
            proto_version: CURRENT_PROTOCOL_VERSION,
            connection_id: vec![0x66; 16],
            capability_token: vec![0; 32],
        })),
    )
    .await;
    let error = rejected
        .err()
        .ok_or("invalid actor capability unexpectedly received a response")?;
    assert_eq!(
        error
            .downcast_ref::<std::io::Error>()
            .map(std::io::Error::kind),
        Some(std::io::ErrorKind::UnexpectedEof)
    );
    drop(unauthenticated);
    let mut client = Client::connect(&socket).await?;
    let discovery = client
        .call("inspect", json!({"mode":"discover","limit":64}))
        .await?;
    assert_eq!(discovery["health"]["advertised_tools"], 14);
    let verbs: BTreeSet<_> = discovery["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["verb"].as_str().unwrap())
        .collect();
    assert_eq!(
        verbs,
        [
            "activate",
            "attest",
            "believe",
            "bind",
            "consolidate",
            "dispute",
            "forget",
            "inspect",
            "intend",
            "outcome",
            "predict",
            "recall",
            "remember",
            "retract"
        ]
        .into()
    );
    let entries = (1..=2).map(|revision| {
        let payload=json!({"record_id":"pressure-record","revision":revision,"content":format!("Observed pressure revision {revision}"),"recorded_at_ns":"1791288000123456789","provenance":["pressure-sensor"]});
        Ok(ImportEntry {source_id:format!("pressure-revision-{revision}"),kind:"revision".into(),digest:digest_bytes(&serde_json::to_vec(&payload)?),payload})
    }).collect::<Result<Vec<_>>>()?;
    let mut bundle = ImportBundle {
        version: 1,
        import_id: "pressure-knowledge-export".into(),
        scope: scope(),
        entries,
        digest: String::new(),
    };
    bundle.digest = bundle.computed_digest()?;
    let import_args = |max_entries| json!({"conversation":CONVERSATION,"kind":"user","context":{"operation":"import","request":bundle,"max_entries":max_entries}});
    let partial = client.call("remember", import_args(1)).await?;
    assert_eq!(partial["items"][0]["accepted"], 1);
    assert_eq!(partial["items"][0]["complete"], false);
    let imported = client.call("remember", import_args(128)).await?;
    assert_eq!(imported["items"][0]["accepted"], 2);
    assert_eq!(imported["items"][0]["complete"], true);
    let imported_event=client.call("inspect",json!({"uri":format!("hm://{ACTOR}/lsn/{}",imported["items"][0]["last_lsn"].as_u64().unwrap())})).await?;
    assert!(imported_event["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["authority"] == "external_observed"));
    println!("daemon authenticated discovery and knowledge import passed");
    let original = [
        source("reading-20", 1, "Measured pressure is 20 kPa. ".repeat(90))?,
        source("reading-21", 2, "Measured pressure is 21 kPa. ".repeat(90))?,
        source("reading-22", 3, "Current pressure is 22 kPa.".into())?,
    ];
    for request in &original {
        assert_eq!(client.ingest(request).await?["items"][0]["replayed"], false);
    }
    println!("versioned source ingestion passed");
    let mut widened = original[0].clone();
    widened.message.id = "widened-authority".into();
    widened.message.ordinal = 4;
    widened.message.authority = Authority::RuntimeFact;
    widened.message.source_digest = widened.message.computed_digest()?;
    let refused = client.raw("remember", json!({"conversation":CONVERSATION,"kind":"user","context":{"operation":"source","request":widened}})).await?;
    assert_eq!(refused["ok"], false);
    let initial = client.activate("detailed", 4096).await?;
    assert_budget(&initial, 4096);
    assert_eq!(initial["items"][0]["history"]["message_count"], 3);
    let bounded = client.activate("detailed", 512).await?;
    assert_budget(&bounded, 512);
    assert!(!bounded["items"][0]["report"]["omitted"]
        .as_array()
        .unwrap()
        .is_empty());
    for request in &original {
        assert_source(&mut client, request).await?;
    }
    println!("source activation, bounded rendering and exact recovery passed");
    let history = client.inspect("/history").await?;
    let state = &history["items"][0];
    let messages: Vec<SourceMessage> = serde_json::from_value(state["messages"].clone())?;
    let spans: Vec<SourceSpan> = serde_json::from_value(state["spans"].clone())?;
    let cursor: Cursor = serde_json::from_value(state["cursor"].clone())?;
    let chunk = select_chunks_with_spans(
        &messages[..2],
        &spans[..2],
        ChunkLimits {
            max_messages: 10,
            max_bytes: 65536,
        },
    )?
    .remove(0);
    let result = summaries(&chunk)?;
    println!("source history and summary source chunk validation passed");
    client
        .job(
            "summarize-enqueue",
            ContextJobAction::HistorianEnqueue {
                session_id: SESSION.into(),
                cursor,
                policy_revision: 1,
                reservation: 100,
                chunk: chunk.clone(),
                now_ms: 10,
            },
        )
        .await?;
    println!("historian enqueue passed");
    let claim: HistorianClaim = serde_json::from_value(
        client
            .job(
                "summarize-claim",
                ContextJobAction::HistorianClaim {
                    worker: "extractive-worker".into(),
                    now_ms: 10,
                    lease_ms: 10000,
                },
            )
            .await?["items"][0]["result"]
            .clone(),
    )?;
    println!("historian claim passed");
    result.validate(&chunk)?;
    let before_invalid = client.job("inspect-before-invalid-tier", ContextJobAction::Inspect).await?;
    let mut invalid_result = result.clone();
    invalid_result.tiers[3].text = format!("- {}", result.tiers[0].text);
    let invalid_completion = ContextJobRequest {
        version: 1,
        scope: scope(),
        request_id: "reject-expanding-outline".into(),
        action: ContextJobAction::HistorianComplete {
            claim: claim.clone(),
            result: invalid_result,
            usage: Usage::Known(7),
            now_ms: 11,
        },
    };
    let rejected = client.raw("remember", json!({"conversation":CONVERSATION,"kind":"user","context":{"operation":"job","request":invalid_completion}})).await?;
    assert_eq!(rejected["ok"], false);
    let after_invalid = client.job("inspect-after-invalid-tier", ContextJobAction::Inspect).await?;
    assert_eq!(before_invalid["items"][0], after_invalid["items"][0]);
    let completion = ContextJobAction::HistorianComplete {
        claim,
        result: result.clone(),
        usage: Usage::Known(7),
        now_ms: 11,
    };
    let completed = client.job("summarize-complete", completion.clone()).await?;
    println!("historian completion passed");
    assert_eq!(
        completed["items"][0]["result"]["authority"],
        "derived_inference"
    );
    assert_eq!(
        client.job("summarize-complete", completion).await?,
        completed
    );
    for (index, tier) in ["detailed", "condensed", "brief", "outline"]
        .into_iter()
        .enumerate()
    {
        let reduced = client.activate(tier, 4096).await?;
        assert_budget(&reduced, 4096);
        let summaries = reduced["items"][0]["report"]["blocks"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|b| b["id"].as_str().unwrap().starts_with("summary:"))
            .collect::<Vec<_>>();
        assert_eq!(summaries.len(), 1, "{reduced}");
        assert_eq!(summaries[0]["text"], result.tiers[index].text);
        assert_eq!(summaries[0]["authority"], "derived_inference");
        assert!(reduced["items"][0]["messages"]
            .as_array()
            .unwrap()
            .iter()
            .all(|message| message["role"] == "user"
                && (message["authority"] == "user_asserted"
                    || message["authority"] == "derived_inference")));
        assert_eq!(summaries[0]["provenance"], json!(chunk.spans));
        assert_eq!(reduced["items"][0]["messages"].as_array().unwrap().len(), 2);
    }
    for request in &original {
        assert_source(&mut client, request).await?;
    }
    println!("source ingestion, bounded rendering, four-tier reduction and exact expansion passed");
    let runtime_root = root.join("runtime");
    fs::create_dir(&runtime_root)?;
    let runtime_config = || RuntimeConfig::new(&runtime_root, scope(), [31; 32], [57; 32]);
    let mut runtime = RuntimeService::open(runtime_config())?;
    let epoch = runtime.writer_epoch();
    let launch = runtime.launch_worker(process(&binary, root)).await?;
    assert_eq!(launch.module_id, "digest-worker");
    let work = WorkerRequest::digest("source-batch-digest", serde_json::to_vec(&chunk)?);
    let digest = runtime.execute(work.clone()).await?;
    assert_eq!(digest.digest, digest_bytes(&work.bytes));
    assert_eq!(
        runtime.effect(&work.id)?.unwrap().state,
        EffectState::Terminal
    );
    client
        .job(
            "verify-enqueue",
            ContextJobAction::MaintenanceEnqueue {
                session_id: SESSION.into(),
                request: JobRequest {
                    kind: JobKind::Verification,
                    sources: chunk.sources.clone(),
                    cursor,
                    source_revision: cursor.sequence,
                    policy_revision: 1,
                    reservation: 100,
                },
            },
        )
        .await?;
    let lease: JobLease = serde_json::from_value(
        client
            .job(
                "verify-claim",
                ContextJobAction::MaintenanceClaim { now_ms: 20 },
            )
            .await?["items"][0]["result"]
            .clone(),
    )?;
    client
        .job(
            "verify-complete",
            ContextJobAction::MaintenanceComplete {
                lease,
                usage: Usage::Known(7),
                output_digest: digest.digest.clone(),
                now_ms: 21,
            },
        )
        .await?;
    println!("actual authenticated worker and ledger verification settlement passed");
    let correction = source(
        "reading-corrected",
        4,
        "Correction: the first measured pressure was 19 kPa.".into(),
    )?;
    client.ingest(&correction).await?;
    let relation = RelationIngestion {
        version: 1,
        scope: scope(),
        session_id: SESSION.into(),
        conversation: CONVERSATION.into(),
        relation: SourceRelation::Edit {
            id: "pressure-correction".into(),
            original_id: original[0].message.id.clone(),
            replacement_id: correction.message.id.clone(),
        },
    };
    client.call("remember",json!({"conversation":CONVERSATION,"kind":"user","context":{"operation":"relation","request":relation}})).await?;
    let corrected = client.activate("detailed", 4096).await?;
    assert_budget(&corrected, 4096);
    assert!(
        corrected["items"][0]["cache"]["generation"]
            .as_u64()
            .unwrap()
            > initial["items"][0]["cache"]["generation"].as_u64().unwrap()
    );
    assert!(corrected["items"][0]["report"]["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .all(|b| !b["id"].as_str().unwrap().starts_with("summary:")));
    assert!(serde_json::to_string(&corrected["items"][0]["messages"])?
        .contains("first measured pressure was 19"));
    assert_eq!(
        corrected["items"][0]["history"]["relations"][0]["kind"],
        "edit"
    );
    let mut foreign = activation("detailed", 4096);
    foreign["context"]["scope"]["project_id"] = json!("foreign");
    assert_eq!(client.raw("activate", foreign).await?["ok"], false);
    println!("immutable correction, stale summary invalidation and scope rejection passed");
    let before = client.call("inspect", json!({})).await?;
    let history_before = client.inspect("/history").await?;
    let receipts_before = runtime.receipts(Cursor::default(), 100)?;
    let events_before = runtime.events(0, 100)?;
    assert_eq!(receipts_before.len(), 3);
    assert_eq!(events_before.len(), 3);
    runtime.shutdown().await?;
    drop(runtime);
    drop(client);
    daemon.crash().await?;
    let daemon = Daemon::start(
        &binary,
        &config,
        &binding,
        &socket,
        &root.join("daemon-restarted.log"),
    )
    .await?;
    let mut client = Client::connect(&socket).await?;
    let resumed = client.inspect("").await?;
    assert_eq!(
        resumed["items"][0]["report"]["digest"],
        corrected["items"][0]["report"]["digest"]
    );
    assert!(resumed["items"][0]["migrations"]["recent_receipts"]
        .as_array()
        .unwrap()
        .contains(&imported["items"][0]));
    assert_eq!(
        resumed["items"][0]["cache"]["materialization_digest"],
        corrected["items"][0]["cache"]["materialization_digest"]
    );
    assert_eq!(client.inspect("/history").await?, history_before);
    for request in original.iter().chain(std::iter::once(&correction)) {
        assert_source(&mut client, request).await?;
    }
    assert_eq!(
        client.ingest(&original[0]).await?["items"][0]["replayed"],
        true
    );
    assert_eq!(client.call("remember", import_args(128)).await?, imported);
    let after = client.call("inspect", json!({})).await?;
    for field in ["log_events", "applied_lsn", "applied_digest"] {
        assert_eq!(
            after["items"][0][field], before["items"][0][field],
            "restart or exact replay changed {field}"
        );
    }
    let mut runtime = RuntimeService::open(runtime_config())?;
    assert!(runtime.writer_epoch() > epoch);
    assert_eq!(runtime.execute(work).await?, digest);
    assert_eq!(runtime.receipts(Cursor::default(), 100)?, receipts_before);
    assert_eq!(runtime.events(0, 100)?.len(), events_before.len());
    assert_eq!(
        client.inspect("").await?["items"][0]["jobs"]["state"]["maintenance"]["spent"],
        14
    );
    runtime.shutdown().await?;
    drop(runtime);
    drop(client);
    daemon.crash().await?;
    println!("unified context journey passed: actual scoped daemon, authenticated fourteen-verb discovery, exact source bytes, bounded omissions, four summary tiers, correction, daemon restart, actual authenticated worker, durable effect replay and maintenance settlement");
    Ok(())
}

#[test]
fn historian_request_tier_contract() -> Result<()> {
    let observations = [
        source("reading-20", 1, "Measured pressure is 20 kPa. ".repeat(90))?,
        source("reading-21", 2, "Measured pressure is 21 kPa. ".repeat(90))?,
    ];
    let messages = observations.iter().map(|item|item.message.clone()).collect::<Vec<_>>();
    let spans = observations.iter().map(|item|SourceSpan{source_id:item.message.id.clone(),source_digest:item.message.source_digest.clone(),byte_start:0,byte_end:item.original_bytes.len() as u64}).collect::<Vec<_>>();
    let chunk=select_chunks_with_spans(&messages,&spans,ChunkLimits{max_messages:10,max_bytes:65536})?.remove(0);
    let result=summaries(&chunk)?;
    result.validate(&chunk)?;
    let mut invalid = result.clone();
    invalid.tiers[3].text = format!("- {}", result.tiers[0].text);
    assert!(invalid.validate(&chunk).is_err());
    assert!(invalid.tiers[3].text.len() > invalid.tiers[2].text.len());
    Ok(())
}
