use hm_context::{Scope, development::*, development_schedule::*, digest_bytes};
use hm_core::ActorId;
use hm_cortex::development_curation::CurationAction;
use hm_mcp::dispatcher::McpToolDispatcher;
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_memory::{
        self, MemoryCommand, MemoryRecord, MemoryRequest, MemorySource, Provenance, RecordKind,
    },
    development_service::{DevelopmentAction, DevelopmentRequest, ServiceSchedule},
    development_workers::{WorkerConfiguration, WorkerOperation},
    uds::ToolDispatcher,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
fn scope() -> Scope {
    Scope {
        owner_id: "owner".into(),
        project_id: "development-workers".into(),
        workspace_id: None,
    }
}
fn actor_config(root: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: root.join("actor"),
        actor: ActorId::new(1),
        user: [1; 16],
        kek: [2; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}
fn dispatcher(root: &std::path::Path) -> McpToolDispatcher {
    let mut d = McpToolDispatcher::from_env().unwrap().with_context_scope(
        hm_serve::context_config::TrustedContextConfig {
            version: 1,
            actor: 1,
            scope: scope(),
        },
    );
    d.development_runtime = Some(
        d.development_runtime
            .take()
            .expect("actual configured provider")
            .with_workers(vec![
                WorkerConfiguration {
                    id: "classify".into(),
                    operation: WorkerOperation::Curation {
                        actions: vec![CurationAction::Classify { id: "claim".into() }],
                    },
                },
                WorkerConfiguration {
                    id: "verify".into(),
                    operation: WorkerOperation::Verification {
                        root: root.into(),
                        files: BTreeMap::from([("evidence".into(), "measurement.txt".into())]),
                    },
                },
            ])
            .unwrap(),
    );
    d
}
async fn call(d: &McpToolDispatcher, a: &ActorEngine, verb: &str, args: Value) -> Value {
    serde_json::from_slice(
        &d.dispatch(a.clone(), verb.into(), serde_json::to_vec(&args).unwrap())
            .await
            .unwrap(),
    )
    .unwrap()
}
async fn op(d: &McpToolDispatcher, a: &ActorEngine, operation: Value) -> Value {
    call(
        d,
        a,
        "remember",
        json!({"conversation":"conversation","content":"","kind":"user","context":operation}),
    )
    .await
}
async fn command(d: &McpToolDispatcher, a: &ActorEngine, id: &str, command: MemoryCommand) {
    let v=op(d,a,json!({"operation":"memory","request":MemoryRequest{version:1,scope:scope(),request_id:id.into(),command}})).await;
    assert_eq!(v["ok"], true, "{v}");
}
async fn development(
    d: &McpToolDispatcher,
    a: &ActorEngine,
    id: &str,
    action: DevelopmentAction,
) -> Value {
    op(d,a,json!({"operation":"development","request":DevelopmentRequest{version:1,scope:scope(),request_id:id.into(),action}})).await
}
async fn configure(d: &McpToolDispatcher, a: &ActorEngine, worker: &str, phase: &str) -> String {
    let registered = development(
        d,
        a,
        &format!("register-{worker}-{phase}"),
        DevelopmentAction::Register {
            worker_id: worker.into(),
            capability_id: format!("cap-{worker}"),
            session_id: "session".into(),
            conversation: "conversation".into(),
            source_ids: BTreeSet::from(["evidence".into()]),
            record_ids: BTreeSet::from(["claim".into()]),
            new_record_ids: BTreeSet::new(),
            budget: DevelopmentBudget {
                reserved_tokens: 8192,
                max_input_bytes: 65536,
                max_output_bytes: 65536,
                max_mutations: 8,
            },
            lease_ms: 600_000,
        },
    )
    .await;
    assert_eq!(registered["ok"], true, "{registered}");
    let cap: WorkerCapability =
        serde_json::from_value(registered["items"][0]["capability"].clone()).unwrap();
    let configured = development(
        d,
        a,
        &format!("configure-{worker}-{phase}"),
        DevelopmentAction::Configure {
            schedule: ServiceSchedule {
                id: worker.into(),
                mode: ScheduleMode::Manual,
                worker_id: worker.into(),
                snapshot: SnapshotRequest {
                    capability_id: cap.id,
                    source_ids: cap.source_ids,
                    record_ids: cap.record_ids,
                },
                reservation: 8192,
                timeout_ms: 60_000,
                backoff_ms: 0,
                max_attempts: 2,
                identical_failure_limit: 2,
            },
        },
    )
    .await;
    assert_eq!(configured["ok"], true, "{configured}");
    let queued = development(
        d,
        a,
        &format!("enqueue-{worker}-{phase}"),
        DevelopmentAction::Enqueue {
            schedule_id: worker.into(),
        },
    )
    .await;
    assert_eq!(queued["ok"], true, "{queued}");
    queued["items"][0]["job_id"].as_str().unwrap().into()
}
#[tokio::test]
async fn real_owner_registry_classification_mapping_verification_and_restart() {
    assert_eq!(std::env::var("HM_DEVELOPMENT_PROVIDER").unwrap(), "ollama");
    let root = tempfile::tempdir().unwrap();
    let repository = root.path().join("repository");
    std::fs::create_dir(&repository).unwrap();
    let content = b"the vault access code is 4821.";
    std::fs::write(repository.join("measurement.txt"), content).unwrap();
    let actor = ActorEngine::open(actor_config(root.path())).await.unwrap();
    let d = dispatcher(&repository);
    command(
        &d,
        &actor,
        "source",
        MemoryCommand::Source {
            source: MemorySource {
                id: "evidence".into(),
                digest: digest_bytes(content),
                content: content.to_vec(),
                locator: "repository:measurement.txt".into(),
                occurred_at_ns: None,
                recorded_at_ns: 100,
                tombstoned: false,
            },
        },
    )
    .await;
    let mut claim = MemoryRecord::new(
        "claim",
        RecordKind::Note,
        String::from_utf8(content.to_vec()).unwrap(),
        100,
    );
    claim.provenance = vec![Provenance {
        source_id: "evidence".into(),
        source_digest: digest_bytes(content),
        span_start: 0,
        span_end: content.len() as u64,
        quoted_digest: digest_bytes(content),
    }];
    command(&d, &actor, "claim", MemoryCommand::Create { record: claim }).await;
    let inspected = call(
        &d,
        &actor,
        "inspect",
        json!({"uri":"hm://1/context-development"}),
    )
    .await;
    assert_eq!(inspected["ok"], true, "{inspected}");
    let workers = inspected["items"][0]["runtime_workers"].as_array().unwrap();
    assert!(
        workers
            .iter()
            .any(|w| w["id"] == "classify" && w["kind"] == "curation")
    );
    assert!(
        workers
            .iter()
            .any(|w| w["id"] == "verify" && w["kind"] == "verification")
    );
    assert!(!workers.iter().any(|w| w["kind"] == "conditional_note"));
    let original = context_memory::rebuild(&actor, &scope())
        .await
        .unwrap()
        .records["claim"]
        .clone();
    for worker in ["classify", "verify"] {
        let job = configure(&d, &actor, worker, "first").await;
        let completed = development(
            &d,
            &actor,
            &format!("dispatch-{worker}"),
            DevelopmentAction::Dispatch {
                job_id: job.clone(),
            },
        )
        .await;
        assert_eq!(completed["ok"], true, "{completed}");
        let state: DevelopmentSchedules =
            serde_json::from_value(completed["items"][0].clone()).unwrap();
        assert!(
            matches!(state.jobs[&job].status, DispatchStatus::Complete { .. }),
            "{state:?}"
        );
        assert!(state.accounting.spent > 0);
    }
    let state = context_memory::rebuild(&actor, &scope()).await.unwrap();
    let classified = &state.records["claim"];
    assert_eq!(classified.content, original.content);
    assert_eq!(classified.provenance, original.provenance);
    assert_eq!(classified.revision, 2);
    assert!(classified.metadata["classification"].is_object());
    assert!(
        state
            .verifications
            .values()
            .any(|v| v.record_id == "claim"
                && v.state == context_memory::VerificationState::Supported)
    );
    std::fs::write(
        repository.join("measurement.txt"),
        b"the vault access code is 9912.",
    )
    .unwrap();
    let job = configure(&d, &actor, "verify", "stale").await;
    let stale = development(
        &d,
        &actor,
        "dispatch-stale",
        DevelopmentAction::Dispatch {
            job_id: job.clone(),
        },
    )
    .await;
    assert_eq!(stale["ok"], true, "{stale}");
    let schedules: DevelopmentSchedules =
        serde_json::from_value(stale["items"][0].clone()).unwrap();
    assert!(!matches!(
        schedules.jobs[&job].status,
        DispatchStatus::Complete { .. }
    ));
    assert!(
        schedules.progress["verify"]
            .last_error
            .as_deref()
            .is_some_and(|e| e.contains("stale"))
    );
    assert_eq!(
        context_memory::rebuild(&actor, &scope())
            .await
            .unwrap()
            .verifications
            .len(),
        state.verifications.len()
    );
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(actor_config(root.path())).await.unwrap();
    let d = dispatcher(&repository);
    let recovered = call(
        &d,
        &actor,
        "inspect",
        json!({"uri":"hm://1/context-development"}),
    )
    .await;
    assert_eq!(recovered["ok"], true, "{recovered}");
    assert_eq!(
        context_memory::rebuild(&actor, &scope())
            .await
            .unwrap()
            .records["claim"]
            .revision_digest,
        classified.revision_digest
    );
    actor.shutdown().await.unwrap();
}
