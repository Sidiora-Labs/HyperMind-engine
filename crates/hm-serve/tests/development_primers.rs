use hm_context::{development::*, Authority, MessagePart, MessageRole, Scope, SourceMessage};
use hm_core::ActorId;
use hm_cortex::development_primers::INDEX_ID;
use hm_llm::{ollama::Ollama, HttpTransport, LlmProvider, ModelTier, Pricing, ProviderConfig};
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_history::{self, SourceIngestion},
    context_memory::{self, MemoryCommand, MemoryGrant, MemoryRequest, RecordStatus},
    development_primers,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
fn owner() -> Scope {
    Scope {
        owner_id: "owner".into(),
        project_id: "primers".into(),
        workspace_id: None,
    }
}
fn worker() -> Scope {
    Scope {
        owner_id: "primer-worker".into(),
        ..owner()
    }
}
fn config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.into(),
        actor: ActorId::new(54),
        user: [6; 16],
        kek: [3; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}
async fn cmd(actor: &ActorEngine, id: &str, command: MemoryCommand) {
    context_memory::execute(
        actor,
        &owner(),
        &owner(),
        MemoryRequest {
            version: 1,
            scope: owner(),
            request_id: id.into(),
            command,
        },
    )
    .await
    .unwrap();
}
async fn collect(actor: &ActorEngine, session: &str, text: &str, time: i64) -> Vec<String> {
    let mut message = SourceMessage {
        id: format!("source-{session}"),
        ordinal: 1,
        role: MessageRole::User,
        parts: vec![MessagePart::Text { text: text.into() }],
        authority: Authority::UserAsserted,
        occurred_at_ns: Some(time),
        recorded_at_ns: time + 1,
        source_digest: String::new(),
    };
    message.source_digest = message.computed_digest().unwrap();
    context_history::ingest(
        actor,
        &owner(),
        &SourceIngestion {
            version: 1,
            scope: owner(),
            session_id: session.into(),
            conversation: format!("room-{session}"),
            message,
            original_bytes: text.as_bytes().to_vec(),
        },
    )
    .await
    .unwrap();
    development_primers::collect_session(
        actor,
        &owner(),
        &owner(),
        session,
        &format!("room-{session}"),
    )
    .await
    .unwrap()
}
async fn capability(
    actor: &ActorEngine,
    id: &str,
    job: &str,
    sources: &BTreeSet<String>,
    refresh: bool,
    budget: u64,
) -> SnapshotRequest {
    let state = context_memory::rebuild(actor, &owner()).await.unwrap();
    let mut records = BTreeSet::from([INDEX_ID.to_string(), job.to_string()]);
    if refresh {
        records.insert("release-primer".into());
    }
    let revisions = records
        .iter()
        .map(|id| (id.clone(), state.records[id].revision_digest.clone()))
        .collect::<BTreeMap<_, _>>();
    cmd(
        actor,
        &format!("read-{id}"),
        MemoryCommand::SetGrant {
            grant: MemoryGrant {
                id: format!("grant-{id}"),
                principal: worker(),
                principal_digest: None,
                record_ids: records.clone(),
                categories: BTreeSet::new(),
                read: true,
                expires_at_ns: None,
                revoked: false,
                revision: 1,
                record_revisions: revisions,
            },
        },
    )
    .await;
    cmd(
        actor,
        &format!("register-{id}"),
        MemoryCommand::RegisterWorker {
            capability: WorkerCapability {
                id: id.into(),
                scope: owner(),
                principal: worker(),
                worker_id: "primer-runtime".into(),
                revision: 1,
                revoked: false,
                allowed_kinds: BTreeSet::from([DevelopmentKind::Primer]),
                source_ids: sources.clone(),
                record_ids: records,
                new_record_ids: if refresh {
                    BTreeSet::new()
                } else {
                    BTreeSet::from(["release-primer".into()])
                },
                session_id: "first".into(),
                conversation: "room-first".into(),
                lease: DevelopmentLease {
                    id: format!("lease-{id}"),
                    attempt: 1,
                    expires_at_ns: i64::MAX,
                },
                budget: DevelopmentBudget {
                    reserved_tokens: budget,
                    max_input_bytes: 65536,
                    max_output_bytes: 65536,
                    max_mutations: 4,
                },
            },
        },
    )
    .await;
    SnapshotRequest {
        capability_id: id.into(),
        source_ids: sources.clone(),
        record_ids: BTreeSet::new(),
    }
}
#[tokio::test]
async fn recurring_primer_actual_provider_refresh_protection_and_durable_backlog() {
    let endpoint = std::env::var("HM_PRIMER_ENDPOINT")
        .expect("actual configured primer provider endpoint required");
    let model = std::env::var("HM_PRIMER_MODEL").expect("actual configured primer model required");
    let provider: Arc<dyn LlmProvider> = tokio::task::spawn_blocking(move || {
        Arc::new(
            Ollama::new(
                ProviderConfig {
                    endpoint,
                    model,
                    api_key: None,
                    tier: ModelTier::Economy,
                    pricing: Pricing::default(),
                },
                HttpTransport::default(),
            )
            .unwrap(),
        ) as Arc<dyn LlmProvider>
    })
    .await
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(dir.path())).await.unwrap();
    let mut sources: BTreeSet<_> = collect(
        &actor,
        "first",
        "Standing question: Who approves a release? The owner must approve every release.",
        100,
    )
    .await
    .into_iter()
    .collect();
    sources.extend(collect(&actor,"second","For this separate session: who approves releases? Every release requires owner approval.",200).await);
    assert!(development_primers::enqueue(
        &actor,
        &owner(),
        &worker(),
        "forged",
        "release-primer",
        false
    )
    .await
    .is_err());
    development_primers::enqueue(
        &actor,
        &owner(),
        &owner(),
        "develop",
        "release-primer",
        false,
    )
    .await
    .unwrap();
    let req = capability(&actor, "create", "develop", &sources, false, 20000).await;
    development_primers::run(
        &actor,
        &owner(),
        &worker(),
        "primer-runtime",
        req,
        "develop",
        provider.clone(),
    )
    .await
    .unwrap();
    let state = context_memory::rebuild(&actor, &owner()).await.unwrap();
    let initial = state.records["release-primer"].clone();
    assert_eq!(initial.kind, context_memory::RecordKind::Primer);
    assert!(initial.metadata["answer"]
        .as_str()
        .unwrap()
        .to_lowercase()
        .contains("owner"));
    assert_eq!(initial.provenance.len(), 2);
    assert_eq!(state.records["develop"].status, RecordStatus::Archived);
    assert_eq!(initial.authority, Authority::DerivedInference);
    let mut updated:BTreeSet<_>=collect(&actor,"third","Updated release approval rule: Who approves a release? Two independent reviewers must approve, not the owner alone.",300).await.into_iter().collect();
    updated.extend(collect(&actor,"fourth","Current standing question: who approves releases now? Release approval now requires two independent reviewers; the old owner-only rule is superseded.",400).await);
    development_primers::enqueue(
        &actor,
        &owner(),
        &owner(),
        "refresh",
        "release-primer",
        true,
    )
    .await
    .unwrap();
    let req = capability(&actor, "refresh-cap", "refresh", &updated, true, 20000).await;
    development_primers::run(
        &actor,
        &owner(),
        &worker(),
        "primer-runtime",
        req,
        "refresh",
        provider.clone(),
    )
    .await
    .unwrap();
    let state = context_memory::rebuild(&actor, &owner()).await.unwrap();
    let refreshed = state.records["release-primer"].clone();
    assert_eq!(refreshed.revision, 2);
    assert_eq!(refreshed.metadata["question"], initial.metadata["question"]);
    assert_eq!(
        refreshed.metadata["answer_history"][0]["answer"],
        initial.metadata["answer"]
    );
    assert!(refreshed.metadata["answer"]
        .as_str()
        .unwrap()
        .to_lowercase()
        .contains("reviewer"));
    assert!(refreshed.confidence <= 900000);
    assert_ne!(
        refreshed.metadata["checked_source_digests"],
        initial.metadata["checked_source_digests"]
    );
    development_primers::enqueue(
        &actor,
        &owner(),
        &owner(),
        "budget-blocked",
        "release-primer",
        true,
    )
    .await
    .unwrap();
    let req = capability(&actor, "tiny-budget", "budget-blocked", &updated, true, 1).await;
    assert!(development_primers::run(
        &actor,
        &owner(),
        &worker(),
        "primer-runtime",
        req,
        "budget-blocked",
        provider.clone()
    )
    .await
    .is_err());
    development_primers::enqueue(
        &actor,
        &owner(),
        &owner(),
        "stale-blocked",
        "release-primer",
        true,
    )
    .await
    .unwrap();
    let req = capability(
        &actor,
        "stale-source",
        "stale-blocked",
        &sources,
        true,
        20000,
    )
    .await;
    cmd(
        &actor,
        "tombstone-observation",
        MemoryCommand::TombstoneSource {
            id: sources.iter().next().unwrap().clone(),
        },
    )
    .await;
    assert!(development_primers::run(
        &actor,
        &owner(),
        &worker(),
        "primer-runtime",
        req,
        "stale-blocked",
        provider.clone()
    )
    .await
    .is_err());
    let mut pinned = refreshed.clone();
    pinned.revision += 1;
    pinned.revision_digest.clear();
    pinned.pinned = true;
    cmd(
        &actor,
        "pin-primer",
        MemoryCommand::Revise {
            record: pinned,
            expected_revision: refreshed.revision,
        },
    )
    .await;
    development_primers::enqueue(
        &actor,
        &owner(),
        &owner(),
        "protected-blocked",
        "release-primer",
        true,
    )
    .await
    .unwrap();
    let req = capability(
        &actor,
        "protected",
        "protected-blocked",
        &updated,
        true,
        20000,
    )
    .await;
    assert!(development_primers::run(
        &actor,
        &owner(),
        &worker(),
        "primer-runtime",
        req,
        "protected-blocked",
        provider.clone()
    )
    .await
    .is_err());
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(dir.path())).await.unwrap();
    let state = context_memory::rebuild(&actor, &owner()).await.unwrap();
    assert_eq!(
        state.records["release-primer"].metadata["answer_history"][0]["answer"],
        initial.metadata["answer"]
    );
    for id in ["budget-blocked", "stale-blocked", "protected-blocked"] {
        assert_eq!(state.records[id].status, RecordStatus::Active);
    }
    assert_eq!(
        development_primers::inspect(&actor, &owner(), &owner())
            .await
            .unwrap()["pending"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert!(development_primers::inspect(&actor, &owner(), &worker())
        .await
        .is_err());
    actor.shutdown().await.unwrap();
    tokio::task::spawn_blocking(move || drop(provider))
        .await
        .unwrap();
}
