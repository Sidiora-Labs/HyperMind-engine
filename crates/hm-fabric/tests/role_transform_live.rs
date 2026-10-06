use hm_context::{
    history::SourceRelation,
    types::{
        Authority, ContextBlock, MessagePart, MessageRole, Scope, SourceMessage, digest_bytes,
    },
};
use hm_core::ActorId;
use hm_fabric::{
    role_store::*,
    role_transform::*,
    supervisor::{Probe, ProcessSpec, RestartPolicy},
    transport::Limits,
};
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_history::{self, RelationIngestion, SourceIngestion},
    context_tokenizer,
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
        owner_id: "transform-owner".into(),
        project_id: "native-transform".into(),
        workspace_id: None,
    }
}
fn actor_config(root: &Path) -> ActorConfig {
    ActorConfig {
        actor_directory: root.join("source-ledger"),
        actor: ActorId::new(119),
        user: [97; 16],
        kek: [103; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}
fn process() -> ProcessSpec {
    let (command, args) = match std::env::var_os("HM_FABRIC_TRANSFORM_BIN") {
        Some(bin) => (PathBuf::from(bin), vec!["fabric-transform-worker".into()]),
        None => (
            std::env::current_exe().unwrap(),
            vec![
                "--exact".into(),
                "transform_production_child".into(),
                "--ignored".into(),
            ],
        ),
    };
    ProcessSpec {
        module_id: "native-policy-worker".into(),
        command,
        args,
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
fn config(root: &Path, policy: TransformPolicy) -> TransformWorkerConfig {
    TransformWorkerConfig {
        root: root.into(),
        registry_path: root.join("roles.sqlite"),
        scope: scope(),
        host_key: [107; 32],
        worker_key: [109; 32],
        policy,
    }
}
async fn ingest(
    actor: &ActorEngine,
    id: &str,
    ordinal: u64,
    role: MessageRole,
    authority: Authority,
    text: &str,
) -> SourceIngestion {
    let mut message = SourceMessage {
        id: id.into(),
        ordinal,
        role,
        authority,
        parts: vec![MessagePart::Text { text: text.into() }],
        occurred_at_ns: Some(now()),
        recorded_at_ns: now(),
        source_digest: String::new(),
    };
    message.source_digest = message.computed_digest().unwrap();
    let input = SourceIngestion {
        version: 1,
        scope: scope(),
        session_id: "transform-session".into(),
        conversation: "transform-room".into(),
        message,
        original_bytes: format!("immutable {id} wire: {text}\r\n").into_bytes(),
    };
    context_history::ingest(actor, &scope(), &input)
        .await
        .unwrap();
    input
}
fn blocks(history: &hm_context::history::SourceHistory) -> Vec<ContextBlock> {
    let counter = context_tokenizer::counter_for_model("gpt-4o").unwrap();
    history
        .visible_messages()
        .iter()
        .map(|source| {
            let text = source
                .parts
                .iter()
                .filter_map(|p| match p {
                    MessagePart::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            ContextBlock {
                id: source.id.clone(),
                tokens: counter.count(text.as_bytes()).unwrap() as u64,
                text,
                authority: source.authority,
                provenance: vec![history.source_span(&source.id).unwrap()],
                required: source.id == "mandatory-first",
            }
        })
        .collect()
}
fn hold(
    reg: &mut DurableRoleRegistry,
    policy: &TransformPolicy,
    id: &str,
    history: &hm_context::history::SourceHistory,
    blocks: Vec<ContextBlock>,
) -> u64 {
    let descriptor = policy.descriptor().unwrap();
    reg.register(descriptor.clone()).unwrap();
    let held = reg
        .hold(RoleWork {
            id: id.into(),
            role_id: descriptor.id,
            pin: descriptor.pin,
            semantic_version: descriptor.semantic_version,
            operation: "transform".into(),
            capability_version: 1,
            source: history_fence(history).unwrap(),
            payload: serde_json::to_vec(&TransformBoundary {
                current_work_ids: BTreeSet::from(["mandatory-first".into()]),
            })
            .unwrap(),
            blocks,
            hook: "context_assembly".into(),
            deadline_ns: now() + 30_000_000_000,
        })
        .unwrap();
    held.revision
}
#[tokio::test]
#[ignore]
async fn transform_production_child() {
    transform_worker_from_env().await.unwrap();
}
#[tokio::test]
async fn real_native_transform_narrows_policy_fences_late_result_and_process_restart() {
    let dir = tempfile::tempdir().unwrap();
    let mut actor = ActorEngine::open(actor_config(dir.path())).await.unwrap();
    let original = ingest(
        &actor,
        "mandatory-first",
        1,
        MessageRole::User,
        Authority::UserAsserted,
        "Keep the vessel identifier Helix-54.",
    )
    .await;
    ingest(
        &actor,
        "assistant-optional",
        2,
        MessageRole::Assistant,
        Authority::AssistantGenerated,
        "Unverified draft speculation.",
    )
    .await;
    ingest(
        &actor,
        "optional-too-large",
        3,
        MessageRole::User,
        Authority::ExternalObserved,
        &"Sonar observations remain provisional. ".repeat(75),
    )
    .await;
    ingest(
        &actor,
        "last-user",
        4,
        MessageRole::User,
        Authority::UserAsserted,
        "Keep the depth qualifier.",
    )
    .await;
    let history = context_history::replay(&actor, &scope(), "transform-session", "transform-room")
        .await
        .unwrap()
        .history;
    let input = blocks(&history);
    let max_output_tokens = input[0].tokens + input[3].tokens;
    let policy = TransformPolicy {
        id: "native-policy-transform".into(),
        semantic_version: "1.0.0".into(),
        input_authorities: vec![
            Authority::UserAsserted,
            Authority::ExternalObserved,
            Authority::AssistantGenerated,
        ],
        authorities: vec![Authority::UserAsserted, Authority::ExternalObserved],
        hooks: BTreeSet::from(["context_assembly".into()]),
        max_input_tokens: input.iter().map(|b| b.tokens).sum::<u64>() + 100,
        max_output_tokens,
    };
    let mut reg = DurableRoleRegistry::open(dir.path().join("roles.sqlite"), scope()).unwrap();
    let revision = hold(
        &mut reg,
        &policy,
        "selected-transform",
        &history,
        input.clone(),
    );
    reg.approve("selected-transform", revision).unwrap();
    let receipts_before = reg.receipts(0).unwrap().len();
    let mut worker = TransformWorkerSession::launch(
        process(),
        config(dir.path(), policy.clone()),
        &reg,
        Limits::default(),
    )
    .await
    .unwrap();
    assert_eq!(reg.receipts(0).unwrap().len(), receipts_before);
    worker
        .submit(&mut reg, "selected-transform", &history)
        .await
        .unwrap();
    assert_eq!(
        worker.receive(&mut reg, &history).await.unwrap().state,
        WorkState::Terminal
    );
    let selected = worker
        .consume(&reg, "selected-transform", &history)
        .unwrap();
    assert_eq!(selected, vec![input[0].clone(), input[3].clone()]);
    let payload = serde_json::to_vec(&selected).unwrap();
    assert!(
        reg.deliver("selected-transform", "delivery-one", &payload)
            .unwrap()
    );
    assert!(
        !reg.deliver("selected-transform", "delivery-one", &payload)
            .unwrap()
    );
    assert!(
        reg.deliver("selected-transform", "delivery-one", b"changed")
            .is_err()
    );
    assert_eq!(
        history.recover(&scope(), &input[0].provenance[0]).unwrap(),
        original.original_bytes
    );
    let mut required_disallowed = input.clone();
    required_disallowed[1].required = true;
    let revision = hold(
        &mut reg,
        &policy,
        "required-refusal",
        &history,
        required_disallowed,
    );
    reg.approve("required-refusal", revision).unwrap();
    assert!(
        worker
            .submit(&mut reg, "required-refusal", &history)
            .await
            .is_err()
    );
    assert_eq!(
        reg.get("required-refusal").unwrap().unwrap().state,
        WorkState::Approved
    );
    let mut current_unprotected = input.clone();
    current_unprotected[0].required = false;
    let revision = hold(
        &mut reg,
        &policy,
        "current-work-refusal",
        &history,
        current_unprotected,
    );
    reg.approve("current-work-refusal", revision).unwrap();
    assert!(
        worker
            .submit(&mut reg, "current-work-refusal", &history)
            .await
            .is_err()
    );
    let revision = hold(&mut reg, &policy, "late-transform", &history, input.clone());
    reg.approve("late-transform", revision).unwrap();
    worker
        .submit(&mut reg, "late-transform", &history)
        .await
        .unwrap();
    let prepared = worker.collect(&mut reg).await.unwrap();
    reg.mark_uncertain(
        reg.get("late-transform")
            .unwrap()
            .unwrap()
            .ticket
            .as_ref()
            .unwrap(),
    )
    .unwrap();
    let late = worker.commit(&mut reg, &history, prepared).unwrap();
    assert_eq!(late.state, WorkState::AwaitingAcknowledgement);
    let digest = digest_bytes(&serde_json::to_vec(late.output.as_ref().unwrap()).unwrap());
    let epoch = reg.epoch();
    assert!(
        reg.acknowledge_late(
            "late-transform",
            "wrong",
            epoch,
            &history_fence(&history).unwrap()
        )
        .is_err()
    );
    assert_eq!(
        reg.acknowledge_late(
            "late-transform",
            &digest,
            epoch,
            &history_fence(&history).unwrap()
        )
        .unwrap()
        .state,
        WorkState::Terminal
    );
    assert_eq!(
        worker.consume(&reg, "late-transform", &history).unwrap(),
        selected
    );
    let revision = hold(
        &mut reg,
        &policy,
        "stale-transform",
        &history,
        input.clone(),
    );
    reg.approve("stale-transform", revision).unwrap();
    worker
        .submit(&mut reg, "stale-transform", &history)
        .await
        .unwrap();
    let prepared = worker.collect(&mut reg).await.unwrap();
    context_history::relate(
        &actor,
        &scope(),
        &RelationIngestion {
            version: 1,
            scope: scope(),
            session_id: "transform-session".into(),
            conversation: "transform-room".into(),
            relation: SourceRelation::Tombstone {
                id: "retire-optional-source".into(),
                source_id: "optional-too-large".into(),
            },
        },
    )
    .await
    .unwrap();
    let current = context_history::replay(&actor, &scope(), "transform-session", "transform-room")
        .await
        .unwrap()
        .history;
    assert_eq!(
        worker.commit(&mut reg, &current, prepared).unwrap().state,
        WorkState::AwaitingAcknowledgement
    );
    assert!(worker.consume(&reg, "stale-transform", &current).is_err());
    assert!(
        worker
            .consume(&reg, "selected-transform", &current)
            .is_err()
    );
    let current_input = blocks(&current);
    let revision = hold(
        &mut reg,
        &policy,
        "killed-transform",
        &current,
        current_input.clone(),
    );
    reg.approve("killed-transform", revision).unwrap();
    worker
        .submit(&mut reg, "killed-transform", &current)
        .await
        .unwrap();
    let orphaned_result = worker.collect(&mut reg).await.unwrap();
    drop(orphaned_result);
    assert_eq!(
        reg.get("killed-transform").unwrap().unwrap().state,
        WorkState::Dispatched
    );
    worker.kill_worker(&mut reg).unwrap();
    drop(worker);
    drop(reg);
    actor.shutdown().await.unwrap();
    actor = ActorEngine::open(actor_config(dir.path())).await.unwrap();
    let current = context_history::replay(&actor, &scope(), "transform-session", "transform-room")
        .await
        .unwrap()
        .history;
    let mut reg = DurableRoleRegistry::open(dir.path().join("roles.sqlite"), scope()).unwrap();
    assert_eq!(
        reg.get("killed-transform").unwrap().unwrap().state,
        WorkState::Uncertain
    );
    let mut restarted = TransformWorkerSession::launch(
        process(),
        config(dir.path(), policy.clone()),
        &reg,
        Limits::default(),
    )
    .await
    .unwrap();
    assert!(
        restarted
            .submit(&mut reg, "killed-transform", &current)
            .await
            .is_err()
    );
    assert_eq!(
        current.recover(&scope(), &input[0].provenance[0]).unwrap(),
        original.original_bytes
    );
    restarted.shutdown().unwrap();
    drop(restarted);
    let revision = hold(
        &mut reg,
        &policy,
        "revision-held",
        &current,
        current_input.clone(),
    );
    let mut tight = policy.clone();
    tight.authorities = vec![Authority::UserAsserted];
    tight.semantic_version = "1.1.0".into();
    reg.register(tight.descriptor().unwrap()).unwrap();
    assert!(reg.approve("revision-held", revision).is_err());
    let revision = hold(&mut reg, &tight, "tight-transform", &current, current_input);
    reg.approve("tight-transform", revision).unwrap();
    let mut tight_worker = TransformWorkerSession::launch(
        process(),
        config(dir.path(), tight.clone()),
        &reg,
        Limits::default(),
    )
    .await
    .unwrap();
    tight_worker
        .submit(&mut reg, "tight-transform", &current)
        .await
        .unwrap();
    assert_eq!(
        tight_worker
            .receive(&mut reg, &current)
            .await
            .unwrap()
            .state,
        WorkState::Terminal
    );
    let tightened = tight_worker
        .consume(&reg, "tight-transform", &current)
        .unwrap();
    assert!(
        tightened
            .iter()
            .all(|b| b.authority == Authority::UserAsserted)
    );
    let revision = hold(
        &mut reg,
        &tight,
        "withdrawn-transform",
        &current,
        blocks(&current),
    );
    reg.approve("withdrawn-transform", revision).unwrap();
    tight_worker
        .submit(&mut reg, "withdrawn-transform", &current)
        .await
        .unwrap();
    let prepared = tight_worker.collect(&mut reg).await.unwrap();
    let withdrawal = reg
        .withdraw(&tight.id, "owner withdrew exact policy")
        .unwrap();
    assert_eq!(
        reg.withdraw(&tight.id, "different retry reason").unwrap(),
        withdrawal
    );
    let withdrawn = tight_worker.commit(&mut reg, &current, prepared).unwrap();
    assert_eq!(withdrawn.state, WorkState::AwaitingAcknowledgement);
    assert!(
        tight_worker
            .consume(&reg, "withdrawn-transform", &current)
            .is_err()
    );
    let digest = digest_bytes(&serde_json::to_vec(withdrawn.output.as_ref().unwrap()).unwrap());
    let epoch = reg.epoch();
    assert_eq!(
        reg.acknowledge_late(
            "withdrawn-transform",
            &digest,
            epoch,
            &history_fence(&current).unwrap()
        )
        .unwrap()
        .state,
        WorkState::Terminal
    );
    assert!(
        tight_worker
            .consume(&reg, "withdrawn-transform", &current)
            .is_err()
    );
    tight_worker.shutdown().unwrap();
    actor.shutdown().await.unwrap();
}
