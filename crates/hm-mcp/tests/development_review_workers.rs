use hm_context::{Authority, MessagePart, MessageRole, SourceMessage, history::SourceRelation};
use hm_context::{Scope, development::*, development_schedule::*};
use hm_core::ActorId;
use hm_mcp::dispatcher::McpToolDispatcher;
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_memory::{self, RecordKind},
    development_service::{DevelopmentAction, DevelopmentRequest, ServiceSchedule},
    development_workers::{WorkerConfiguration, WorkerOperation},
    uds::ToolDispatcher,
};
use hm_serve::{development_primers, development_profile};
use serde_json::{Value, json};
use std::collections::BTreeSet;
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
fn dispatcher() -> McpToolDispatcher {
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
                    id: "profile-one".into(),
                    operation: WorkerOperation::Profile {
                        target_id: "profile-one-result".into(),
                    },
                },
                WorkerConfiguration {
                    id: "profile-two".into(),
                    operation: WorkerOperation::Profile {
                        target_id: "profile-two-result".into(),
                    },
                },
                WorkerConfiguration {
                    id: "primer-create".into(),
                    operation: WorkerOperation::Primer {
                        job_id: "primer-create-job".into(),
                    },
                },
                WorkerConfiguration {
                    id: "primer-refresh".into(),
                    operation: WorkerOperation::Primer {
                        job_id: "primer-refresh-job".into(),
                    },
                },
                WorkerConfiguration {
                    id: "retrospective".into(),
                    operation: WorkerOperation::Retrospective {
                        checkpoint_id: "checkpoint".into(),
                        lesson_ids: vec!["lesson-one".into()],
                    },
                },
                WorkerConfiguration {
                    id: "retrospective-resume".into(),
                    operation: WorkerOperation::Retrospective {
                        checkpoint_id: "checkpoint".into(),
                        lesson_ids: vec!["lesson-two".into()],
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
        json!({"conversation":operation["request"]["conversation"].as_str().unwrap_or("conversation"),"content":"","kind":"user","context":operation}),
    )
    .await
}
async fn development(
    d: &McpToolDispatcher,
    a: &ActorEngine,
    id: &str,
    action: DevelopmentAction,
) -> Value {
    op(d,a,json!({"operation":"development","request":DevelopmentRequest{version:1,scope:scope(),request_id:id.into(),action}})).await
}

fn ids(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|v| v.to_string()).collect()
}
async fn run(
    d: &McpToolDispatcher,
    a: &ActorEngine,
    worker: &str,
    sources: BTreeSet<String>,
    records: BTreeSet<String>,
    outputs: BTreeSet<String>,
) -> DevelopmentSchedules {
    let registered = development(
        d,
        a,
        &format!("register-{worker}"),
        DevelopmentAction::Register {
            worker_id: worker.into(),
            capability_id: format!("cap-{worker}"),
            session_id: "session".into(),
            conversation: "conversation".into(),
            source_ids: sources,
            record_ids: records,
            new_record_ids: outputs,
            budget: DevelopmentBudget {
                reserved_tokens: 20000,
                max_input_bytes: 65536,
                max_output_bytes: 65536,
                max_mutations: 8,
            },
            lease_ms: 600000,
        },
    )
    .await;
    assert_eq!(registered["ok"], true, "{registered}");
    let cap: WorkerCapability =
        serde_json::from_value(registered["items"][0]["capability"].clone()).unwrap();
    assert_ne!(cap.principal, scope());
    let configured = development(
        d,
        a,
        &format!("configure-{worker}"),
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
                reservation: 20000,
                timeout_ms: 60000,
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
        &format!("enqueue-{worker}"),
        DevelopmentAction::Enqueue {
            schedule_id: worker.into(),
        },
    )
    .await;
    assert_eq!(queued["ok"], true, "{queued}");
    let job = queued["items"][0]["job_id"].as_str().unwrap().to_string();
    let before_dispatch = hm_serve::development_scheduler::inspect(a, &scope())
        .await
        .unwrap();
    let result = development(
        d,
        a,
        &format!("dispatch-{worker}"),
        DevelopmentAction::Dispatch {
            job_id: job.clone(),
        },
    )
    .await;
    assert_eq!(result["ok"], true, "{result}");
    let state: DevelopmentSchedules = serde_json::from_value(result["items"][0].clone()).unwrap();
    assert!(
        matches!(state.jobs[&job].status, DispatchStatus::Complete { .. }),
        "{state:?}"
    );
    if worker == "retrospective-resume" {
        assert_eq!(
            state.accounting.watermarks,
            before_dispatch.accounting.watermarks
        );
        assert_eq!(state.accounting.spent, before_dispatch.accounting.spent);
    }
    state
}
async fn source(
    d: &McpToolDispatcher,
    a: &ActorEngine,
    session: &str,
    id: &str,
    ordinal: u64,
    text: &str,
) {
    let mut message = SourceMessage {
        id: id.into(),
        ordinal,
        role: MessageRole::User,
        parts: vec![MessagePart::Text { text: text.into() }],
        authority: Authority::UserAsserted,
        occurred_at_ns: None,
        recorded_at_ns: 100 + ordinal as i64,
        source_digest: String::new(),
    };
    message.source_digest = message.computed_digest().unwrap();
    let result=op(d,a,json!({"operation":"source","request":hm_serve::context_history::SourceIngestion{
        version:1,scope:scope(),session_id:session.into(),conversation:if session=="session" {"conversation".into()} else {format!("room-{session}")},message,original_bytes:text.as_bytes().to_vec()}})).await;
    assert_eq!(result["ok"], true, "{result}");
}
async fn review(
    d: &McpToolDispatcher,
    a: &ActorEngine,
    p: &ApprovalProposal,
    id: &str,
    kind: ProposalDecisionKind,
    digest: String,
) -> Value {
    development(
        d,
        a,
        id,
        DevelopmentAction::Review {
            decision: ProposalDecision {
                request_id: id.into(),
                proposal_id: p.id.clone(),
                expected_revision: p.revision,
                expected_digest: digest,
                decision: kind,
            },
        },
    )
    .await
}
#[tokio::test]
async fn real_registry_review_refresh_correction_and_restart_watermark() {
    assert_eq!(std::env::var("HM_DEVELOPMENT_PROVIDER").unwrap(), "ollama");
    let root = tempfile::tempdir().unwrap();
    let a = ActorEngine::open(actor_config(root.path())).await.unwrap();
    let d = dispatcher();
    source(
        &d,
        &a,
        "first",
        "profile-first",
        1,
        "I prefer concise replies with only the key facts. Please keep answers brief.",
    )
    .await;
    source(
        &d,
        &a,
        "second",
        "profile-second",
        1,
        "For this separate session keep responses concise and brief; focus on key facts.",
    )
    .await;
    assert!(
        development_profile::collect_session(&a, &scope(), &scope(), "first", "room-first")
            .await
            .is_err()
    );
    development_profile::set_enabled(&a, &scope(), &scope(), true)
        .await
        .unwrap();
    let mut ps: BTreeSet<_> =
        development_profile::collect_session(&a, &scope(), &scope(), "first", "room-first")
            .await
            .unwrap()
            .into_iter()
            .collect();
    ps.extend(
        development_profile::collect_session(&a, &scope(), &scope(), "second", "room-second")
            .await
            .unwrap(),
    );
    for (worker, output, decision) in [
        (
            "profile-one",
            "profile-one-result",
            ProposalDecisionKind::Accept,
        ),
        (
            "profile-two",
            "profile-two-result",
            ProposalDecisionKind::Reject,
        ),
    ] {
        run(
            &d,
            &a,
            worker,
            ps.clone(),
            ids(&[development_profile::POLICY_ID]),
            ids(&[output]),
        )
        .await;
        let state = context_memory::rebuild(&a, &scope()).await.unwrap();
        assert!(!state.records.contains_key(output));
        let p = state
            .development_proposals
            .values()
            .find(|p| p.status == ProposalStatus::Pending)
            .unwrap()
            .clone();
        assert_ne!(
            review(
                &d,
                &a,
                &p,
                &format!("stale-{worker}"),
                decision.clone(),
                "invalid".into()
            )
            .await["ok"],
            true
        );
        let r = review(
            &d,
            &a,
            &p,
            &format!("review-{worker}"),
            decision,
            p.digest.clone(),
        )
        .await;
        assert_eq!(r["ok"], true, "{r}");
    }
    let mut initial = BTreeSet::new();
    let mut changed = BTreeSet::new();
    for (session, id, text, updated) in [
        (
            "primer-first",
            "primer-source-first",
            "Standing question: Who approves a release? The owner must approve every release.",
            false,
        ),
        (
            "primer-second",
            "primer-source-second",
            "Who approves releases? Every release requires owner approval.",
            false,
        ),
        (
            "primer-third",
            "primer-source-third",
            "Updated rule: Who approves a release? Two independent reviewers must approve, not the owner alone.",
            true,
        ),
        (
            "primer-fourth",
            "primer-source-fourth",
            "Who approves releases now? Two independent reviewers approve; the owner-only rule is superseded.",
            true,
        ),
    ] {
        source(&d, &a, session, id, 1, text).await;
        let collected = development_primers::collect_session(
            &a,
            &scope(),
            &scope(),
            session,
            &format!("room-{session}"),
        )
        .await
        .unwrap();
        if updated {
            changed.extend(collected)
        } else {
            initial.extend(collected)
        }
    }
    development_primers::enqueue(
        &a,
        &scope(),
        &scope(),
        "primer-create-job",
        "release-primer",
        false,
    )
    .await
    .unwrap();
    run(
        &d,
        &a,
        "primer-create",
        initial,
        ids(&[
            hm_cortex::development_primers::INDEX_ID,
            "primer-create-job",
        ]),
        ids(&["release-primer"]),
    )
    .await;
    let old =
        context_memory::rebuild(&a, &scope()).await.unwrap().records["release-primer"].clone();
    assert_eq!(old.kind, RecordKind::Primer);
    development_primers::enqueue(
        &a,
        &scope(),
        &scope(),
        "primer-refresh-job",
        "release-primer",
        true,
    )
    .await
    .unwrap();
    run(
        &d,
        &a,
        "primer-refresh",
        changed,
        ids(&[
            hm_cortex::development_primers::INDEX_ID,
            "primer-refresh-job",
            "release-primer",
        ]),
        BTreeSet::new(),
    )
    .await;
    let new =
        context_memory::rebuild(&a, &scope()).await.unwrap().records["release-primer"].clone();
    assert_eq!(new.revision, old.revision + 1);
    assert_eq!(
        new.metadata["answer_history"][0]["answer"],
        old.metadata["answer"]
    );
    assert!(
        new.metadata["answer"]
            .as_str()
            .unwrap()
            .to_lowercase()
            .contains("reviewer")
    );
    source(
        &d,
        &a,
        "session",
        "original",
        1,
        "Use a force push to publish a shared branch.",
    )
    .await;
    source(&d,&a,"session","correction",2,"Correction: never force-push a shared branch. Create a reviewed pull request and wait for approval before merging.").await;
    let relation=op(&d,&a,json!({"operation":"relation","request":hm_serve::context_history::RelationIngestion{version:1,scope:scope(),session_id:"session".into(),conversation:"conversation".into(),relation:SourceRelation::Edit{id:"user-correction".into(),original_id:"original".into(),replacement_id:"correction".into()}}})).await;
    assert_eq!(relation["ok"], true, "{relation}");
    let published = run(
        &d,
        &a,
        "retrospective",
        ids(&["correction"]),
        BTreeSet::new(),
        ids(&["checkpoint", "lesson-one"]),
    )
    .await;
    let spent = published.accounting.spent;
    let state = context_memory::rebuild(&a, &scope()).await.unwrap();
    assert_eq!(
        state.records["lesson-one"].provenance[0].source_id,
        "correction"
    );
    assert_eq!(
        state.records["lesson-one"].last_lsn,
        state.records["checkpoint"].last_lsn
    );
    let checkpoint = state.records["checkpoint"].revision_digest.clone();
    a.shutdown().await.unwrap();
    let a = ActorEngine::open(actor_config(root.path())).await.unwrap();
    let d = dispatcher();
    let resumed = run(
        &d,
        &a,
        "retrospective-resume",
        ids(&["correction"]),
        ids(&["checkpoint", "lesson-one"]),
        ids(&["lesson-two"]),
    )
    .await;
    assert_eq!(resumed.accounting.spent, spent);
    let receipts = hm_serve::development_scheduler::no_work_receipts(&a, &scope(), &scope())
        .await
        .unwrap();
    assert_eq!(receipts.len(), 1);
    let receipt = &receipts[0];
    assert_eq!(receipt.reason, "no_signal");
    assert_eq!(receipt.provider_calls, 0);
    assert_eq!(receipt.usage, hm_context::maintenance::Usage::Known(0));
    assert!(!receipt.knowledge_published);
    assert_eq!(
        resumed.jobs[&receipt.job_id].status,
        DispatchStatus::Complete {
            receipt_id: receipt.id.clone()
        }
    );
    assert_eq!(
        resumed.accounting.jobs[&resumed.jobs[&receipt.job_id].accounting_id].status,
        hm_context::maintenance::JobStatus::NoWork
    );
    assert!(
        resumed.progress["retrospective-resume"]
            .last_error
            .is_none()
    );
    let state = context_memory::rebuild(&a, &scope()).await.unwrap();
    assert!(!state.records.contains_key("lesson-two"));
    assert_eq!(state.records["checkpoint"].revision_digest, checkpoint);
    assert_eq!(
        state.records["release-primer"].revision_digest,
        new.revision_digest
    );
    assert!(state.records.contains_key("profile-one-result"));
    assert!(!state.records.contains_key("profile-two-result"));
    let inspected = call(
        &d,
        &a,
        "inspect",
        json!({"uri":"hm://1/context-development"}),
    )
    .await;
    assert_eq!(inspected["ok"], true, "{inspected}");
    assert!(
        inspected["items"][0]["runtime_workers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["id"] == "retrospective")
    );
    let receipt_json = serde_json::to_value(receipt).unwrap();
    let no_work_job = receipt.job_id.clone();
    a.shutdown().await.unwrap();
    let a = ActorEngine::open(actor_config(root.path())).await.unwrap();
    let d = dispatcher();
    let restarted = hm_serve::development_scheduler::no_work_receipts(&a, &scope(), &scope())
        .await
        .unwrap();
    assert_eq!(restarted.len(), 1);
    assert_eq!(serde_json::to_value(&restarted[0]).unwrap(), receipt_json);
    let replay = development(
        &d,
        &a,
        "replay-no-work",
        DevelopmentAction::Dispatch {
            job_id: no_work_job.clone(),
        },
    )
    .await;
    assert_eq!(replay["ok"], true, "{replay}");
    let replayed: DevelopmentSchedules =
        serde_json::from_value(replay["items"][0].clone()).unwrap();
    assert_eq!(replayed.accounting.spent, spent);
    assert_eq!(
        replayed.accounting.watermarks,
        resumed.accounting.watermarks
    );
    assert_eq!(
        replayed.jobs[&no_work_job].status,
        resumed.jobs[&no_work_job].status
    );
    let replay_receipts = hm_serve::development_scheduler::no_work_receipts(&a, &scope(), &scope())
        .await
        .unwrap();
    assert_eq!(replay_receipts.len(), 1);
    assert_eq!(
        serde_json::to_value(&replay_receipts[0]).unwrap(),
        receipt_json
    );
    assert_eq!(
        context_memory::rebuild(&a, &scope()).await.unwrap().records["checkpoint"].revision_digest,
        checkpoint
    );
    a.shutdown().await.unwrap();
}
