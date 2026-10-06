use hm_context::{
    historian::SourceChunk,
    history::SourceRelation,
    provider::CapabilityProfile,
    provider_continuity::ProviderProfile,
    reduction::{ReductionItem, ReductionQueue, ReductionRequest, SummaryTier},
    types::{
        Authority, ContextBlock, MessagePart, MessageRole, Scope, SourceMessage, TokenBudget,
        digest_bytes,
    },
};
use hm_core::ActorId;
use hm_fabric::{
    role_compaction::*,
    role_runner::*,
    role_store::*,
    supervisor::{Probe, ProcessSpec, RestartPolicy},
    transport::Limits,
};
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_history::{self, RelationIngestion, SourceIngestion},
    context_tokenizer,
    usage_service::*,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as i64
}
fn scope() -> Scope {
    Scope {
        owner_id: "compact-owner".into(),
        project_id: "compact-project".into(),
        workspace_id: None,
    }
}
fn actor_config(root: &Path) -> ActorConfig {
    ActorConfig {
        actor_directory: root.join("native"),
        actor: ActorId::new(113),
        user: [59; 16],
        kek: [67; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}
fn declaration() -> CompactionDeclaration {
    let digest = "357c53fb659c5076de1d65ccb0b397446227b71a42be9d1603d46168015c9e4b";
    CompactionDeclaration{id:"native-model-compaction".into(),semantic_version:"1.0.0".into(),runner:RunnerDeclaration{id:"compact-model-producer".into(),semantic_version:"1.0.0".into(),endpoint:std::env::var("HM_FABRIC_RUNNER_ENDPOINT").expect("configured endpoint required"),model_digest:digest.into(),profile:ProviderProfile{model_id:"qwen2.5:3b".into(),model_revision:digest.into(),tokenizer_id:"ollama-native-counters".into(),tokenizer_revision:digest.into(),serializer_id:"ollama-structured-chat-v1".into(),serializer_revision:"1".into(),capabilities:CapabilityProfile{user:true,assistant:true,tool:false,text:true,tool_calls:false,tool_results:false,opaque:false}},budget:TokenBudget{context_tokens:4096,reserved_output_tokens:96,required_tokens:0},system:"Produce a concise factual summary of source evidence. Return JSON with a single answer string, under forty words.".into(),json_schema:serde_json::json!({"type":"object","properties":{"answer":{"type":"string"}},"required":["answer"],"additionalProperties":false}),max_prompt_bytes:8192,max_response_bytes:16384,timeout_ms:120000}}
}
fn process() -> ProcessSpec {
    ProcessSpec {
        module_id: "compaction-producer".into(),
        command: std::env::current_exe().unwrap(),
        args: vec![
            "--exact".into(),
            "compaction_model_child".into(),
            "--ignored".into(),
        ],
        env: BTreeMap::new(),
        cwd: PathBuf::from("/tmp"),
        readiness: Probe::ProcessAlive,
        health: Probe::ProcessAlive,
        readiness_timeout: Duration::from_secs(5),
        shutdown_timeout: Duration::from_millis(100),
        stderr_bytes: 8192,
        drain_message: None,
        restart: RestartPolicy {
            max_restarts: 0,
            initial_backoff: Duration::ZERO,
            max_backoff: Duration::ZERO,
        },
    }
}
async fn native_source(actor: &ActorEngine, id: &str, ordinal: u64, text: &str) -> SourceIngestion {
    let mut message = SourceMessage {
        id: id.into(),
        ordinal,
        role: MessageRole::User,
        parts: vec![MessagePart::Text { text: text.into() }],
        occurred_at_ns: Some(now()),
        recorded_at_ns: now(),
        authority: Authority::UserAsserted,
        source_digest: String::new(),
    };
    message.source_digest = message.computed_digest().unwrap();
    let source = SourceIngestion {
        version: 1,
        scope: scope(),
        session_id: "compact-session".into(),
        conversation: "compact-room".into(),
        message,
        original_bytes: format!("original wire source {id}: {text}\r\n").into_bytes(),
    };
    context_history::ingest(actor, &scope(), &source)
        .await
        .unwrap();
    source
}
async fn prepare(
    actor: &ActorEngine,
    reg: &mut DurableRoleRegistry,
    d: &CompactionDeclaration,
    id: &str,
    selected: &str,
) -> CompactionInput {
    let history = context_history::replay(actor, &scope(), "compact-session", "compact-room")
        .await
        .unwrap()
        .history;
    let message = history.message(selected).unwrap().clone();
    let spans = vec![history.source_span(selected).unwrap()];
    let sources = vec![message];
    let chunk = SourceChunk {
        digest: digest_bytes(&serde_json::to_vec(&(&sources, &spans)).unwrap()),
        sources,
        spans,
    };
    let original_text = match &chunk.sources[0].parts[0] {
        MessagePart::Text { text } => text.clone(),
        _ => panic!("plaintext source"),
    };
    let original = ContextBlock {
        id: format!("block-{id}"),
        text: original_text.clone(),
        authority: Authority::UserAsserted,
        provenance: chunk.spans.clone(),
        tokens: context_tokenizer::counter_for_model("qwen2.5:3b")
            .unwrap()
            .count(original_text.as_bytes())
            .unwrap() as u64,
        required: false,
    };
    let request = ReductionRequest {
        id: original.id.clone(),
        original_digest: digest_bytes(&serde_json::to_vec(&original).unwrap()),
        tier: SummaryTier::Brief,
    };
    let runner_id = format!("producer-{id}");
    let input = CompactionInput {
        chunk,
        original,
        reduction: request,
        runner_work_id: runner_id.clone(),
    };
    let fence = source_fence(&history, &input.chunk).unwrap();
    let prompt = compaction_prompt(&input.chunk, input.reduction.tier).unwrap();
    let usage = reserve(
        actor,
        &scope(),
        &scope(),
        ReservationRequest {
            request_id: format!("usage-{id}"),
            attribution: UsageAttribution {
                job_id: format!("job-{id}"),
                worker_id: "compaction-producer".into(),
                session_id: "compact-session".into(),
                turn_id: id.into(),
                provider_id: "ollama-local".into(),
                model_id: "qwen2.5:3b".into(),
                source_ids: BTreeSet::from([selected.into()]),
            },
            kind: hm_context::maintenance::JobKind::Historian,
            reserved_tokens: 4096,
            input_digest: digest_bytes(prompt.as_bytes()),
        },
    )
    .await
    .unwrap();
    let binding = CanonicalUsageBinding {
        lease: usage.lease,
        reservation_id: usage.id,
        reservation_digest: usage.request_digest,
        input_digest: usage.input_digest,
        reserved_tokens: usage.reserved_tokens,
    };
    let producer = d.runner.descriptor().unwrap();
    reg.register(producer.clone()).unwrap();
    let producer = reg
        .hold(RoleWork {
            id: runner_id.clone(),
            role_id: producer.id,
            pin: producer.pin,
            semantic_version: producer.semantic_version,
            operation: "run".into(),
            capability_version: 1,
            source: fence.clone(),
            payload: serde_json::to_vec(&RunnerInput {
                prompt,
                usage: binding,
            })
            .unwrap(),
            blocks: vec![],
            hook: "runner_dispatch".into(),
            deadline_ns: now() + 120_000_000_000,
        })
        .unwrap();
    reg.approve(&runner_id, producer.revision).unwrap();
    let descriptor = d.descriptor().unwrap();
    reg.register(descriptor.clone()).unwrap();
    let held = reg
        .hold(RoleWork {
            id: id.into(),
            role_id: descriptor.id,
            pin: descriptor.pin,
            semantic_version: descriptor.semantic_version,
            operation: "compact".into(),
            capability_version: 1,
            source: fence,
            payload: serde_json::to_vec(&input).unwrap(),
            blocks: vec![],
            hook: "compaction_boundary".into(),
            deadline_ns: now() + 120_000_000_000,
        })
        .unwrap();
    reg.approve(id, held.revision).unwrap();
    input
}
async fn spawn_compaction(
    root: &Path,
    d: &CompactionDeclaration,
    reg: &DurableRoleRegistry,
) -> CompactionConsumer {
    let worker = RunnerWorkerSession::launch(
        process(),
        RunnerWorkerConfig {
            root: root.into(),
            registry_path: root.join("roles.sqlite"),
            scope: scope(),
            host_key: [73; 32],
            worker_key: [89; 32],
            declaration: d.runner.clone(),
        },
        reg,
        Limits {
            io_timeout: Duration::from_secs(130),
            ..Limits::default()
        },
    )
    .await
    .unwrap();
    CompactionConsumer::new(worker, d.clone()).unwrap()
}
#[tokio::test]
#[ignore]
async fn compaction_model_child() {
    runner_worker_from_env().await.unwrap();
}
#[tokio::test]
async fn actual_native_compaction_publication_recovery_staleness_and_cancel_restart() {
    let dir = tempfile::tempdir().unwrap();
    let mut actor = ActorEngine::open(actor_config(dir.path())).await.unwrap();
    configure(
        &actor,
        &scope(),
        &scope(),
        UsageLimits {
            total_tokens: 16384,
            hourly_tokens: 16384,
            daily_tokens: 16384,
            job_tokens: 4096,
            concurrency: 1,
            lease_ms: 180000,
        },
    )
    .await
    .unwrap();
    let text="The buoy reports a water depth of 18 metres. The measurement is provisional because the sensor calibration expires tomorrow. ".repeat(9);
    let original_source = native_source(&actor, "buoy-source", 1, &text).await;
    let d = declaration();
    let mut reg = DurableRoleRegistry::open(dir.path().join("roles.sqlite"), scope()).unwrap();
    let input = prepare(&actor, &mut reg, &d, "compact-first", "buoy-source").await;
    let history = context_history::replay(&actor, &scope(), "compact-session", "compact-room")
        .await
        .unwrap()
        .history;
    let usage = begin_runner_dispatch(
        &actor,
        &scope(),
        &scope(),
        &reg,
        &input.runner_work_id,
        &d.runner,
        "compaction-producer",
    )
    .await
    .unwrap();
    let mut consumer = spawn_compaction(dir.path(), &d, &reg).await;
    consumer
        .submit(&mut reg, "compact-first", &history)
        .await
        .unwrap();
    let completed = consumer.receive(&mut reg, &history).await.unwrap();
    assert_eq!(completed.record.state, WorkState::Terminal);
    let observation =
        ingest_runner_receipt(&actor, &scope(), &scope(), &completed.provider_receipt)
            .await
            .unwrap();
    assert_eq!(observation.reservation_id, usage.reservation_id);
    assert!(observation.snapshot.tokens.input.unwrap() > 0);
    let mut queue = ReductionQueue::default();
    queue.enqueue(input.reduction.clone()).unwrap();
    let mut item = ReductionItem::original(input.original.clone());
    assert!(
        consumer
            .publish(&mut reg, "compact-first", &history, &mut queue, &mut item)
            .unwrap()
    );
    assert!(
        !consumer
            .publish(&mut reg, "compact-first", &history, &mut queue, &mut item)
            .unwrap()
    );
    let summary = item.summaries[2].as_ref().unwrap();
    assert_eq!(summary.authority, Authority::DerivedInference);
    assert_eq!(summary.provenance, input.original.provenance);
    assert!(summary.tokens < input.original.tokens);
    assert!(summary.text.contains("18"));
    assert_eq!(
        history.recover(&scope(), &input.chunk.spans[0]).unwrap(),
        original_source.original_bytes
    );
    context_history::relate(
        &actor,
        &scope(),
        &RelationIngestion {
            version: 1,
            scope: scope(),
            session_id: "compact-session".into(),
            conversation: "compact-room".into(),
            relation: SourceRelation::Tombstone {
                id: "retire-buoy-source".into(),
                source_id: "buoy-source".into(),
            },
        },
    )
    .await
    .unwrap();
    consumer.shutdown().unwrap();
    drop(consumer);
    drop(reg);
    actor.shutdown().await.unwrap();
    actor = ActorEngine::open(actor_config(dir.path())).await.unwrap();
    reg = DurableRoleRegistry::open(dir.path().join("roles.sqlite"), scope()).unwrap();
    let current = context_history::replay(&actor, &scope(), "compact-session", "compact-room")
        .await
        .unwrap()
        .history;
    let mut consumer = spawn_compaction(dir.path(), &d, &reg).await;
    assert!(
        consumer
            .publish(&mut reg, "compact-first", &current, &mut queue, &mut item)
            .is_err()
    );
    assert_eq!(
        current.recover(&scope(), &input.chunk.spans[0]).unwrap(),
        original_source.original_bytes
    );
    consumer.shutdown().unwrap();
    drop(consumer);
    native_source(&actor, "second-buoy-source", 2, &text).await;
    let second = prepare(&actor, &mut reg, &d, "compact-cancel", "second-buoy-source").await;
    let current = context_history::replay(&actor, &scope(), "compact-session", "compact-room")
        .await
        .unwrap()
        .history;
    let cancelled_usage = begin_runner_dispatch(
        &actor,
        &scope(),
        &scope(),
        &reg,
        &second.runner_work_id,
        &d.runner,
        "compaction-producer",
    )
    .await
    .unwrap();
    let mut active = spawn_compaction(dir.path(), &d, &reg).await;
    active
        .submit(&mut reg, "compact-cancel", &current)
        .await
        .unwrap();
    active.cancel(&mut reg).await.unwrap();
    active.kill_worker(&mut reg).unwrap();
    mark_unknown(&actor, &scope(), &scope(), &cancelled_usage.reservation_id)
        .await
        .unwrap();
    drop(active);
    drop(reg);
    actor.shutdown().await.unwrap();
    actor = ActorEngine::open(actor_config(dir.path())).await.unwrap();
    let mut reg = DurableRoleRegistry::open(dir.path().join("roles.sqlite"), scope()).unwrap();
    assert_eq!(
        reg.get("compact-cancel").unwrap().unwrap().state,
        WorkState::Uncertain
    );
    assert_eq!(
        reg.get(&second.runner_work_id).unwrap().unwrap().state,
        WorkState::Uncertain
    );
    let canonical = inspect(&actor, &scope(), &scope()).await.unwrap();
    assert_eq!(canonical.observations.len(), 1);
    assert_eq!(
        canonical.reservations[&cancelled_usage.reservation_id].settled_tokens,
        None
    );
    assert_eq!(
        canonical.accounting.unknown_usage.get(&format!(
            "{}:{}",
            cancelled_usage.lease.job_id, cancelled_usage.lease.attempt
        )),
        Some(&4096)
    );
    let mut restarted = spawn_compaction(dir.path(), &d, &reg).await;
    assert!(
        restarted
            .submit(&mut reg, "compact-cancel", &current)
            .await
            .is_err()
    );
    restarted.shutdown().unwrap();
    actor.shutdown().await.unwrap();
}
