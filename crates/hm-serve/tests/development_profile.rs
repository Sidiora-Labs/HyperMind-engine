use hm_context::{development::*, Authority, MessagePart, MessageRole, Scope, SourceMessage};
use hm_core::ActorId;
use hm_llm::{ollama::Ollama, HttpTransport, LlmProvider, ModelTier, Pricing, ProviderConfig};
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_history::{self, SourceIngestion},
    context_memory::{self, MemoryCommand, MemoryGrant, MemoryRequest},
    development_admission, development_profile,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
fn owner() -> Scope {
    Scope {
        owner_id: "owner".into(),
        project_id: "profile".into(),
        workspace_id: None,
    }
}
fn worker() -> Scope {
    Scope {
        owner_id: "review-worker".into(),
        ..owner()
    }
}
fn config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.into(),
        actor: ActorId::new(53),
        user: [7; 16],
        kek: [8; 32],
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
async fn source(actor: &ActorEngine, session: &str, text: &str) {
    let mut message = SourceMessage {
        id: format!("source-{session}"),
        ordinal: 1,
        role: MessageRole::User,
        parts: vec![MessagePart::Text { text: text.into() }],
        authority: Authority::UserAsserted,
        occurred_at_ns: Some(100),
        recorded_at_ns: 101,
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
}
async fn cap(actor: &ActorEngine, id: &str, record_id: &str, sources: &BTreeSet<String>) {
    let state = context_memory::rebuild(actor, &owner()).await.unwrap();
    cmd(
        actor,
        &format!("grant-{id}"),
        MemoryCommand::SetGrant {
            grant: MemoryGrant {
                id: format!("read-{id}"),
                principal: worker(),
                principal_digest: None,
                record_ids: BTreeSet::from([development_profile::POLICY_ID.into()]),
                categories: BTreeSet::new(),
                read: true,
                expires_at_ns: None,
                revoked: false,
                revision: 1,
                record_revisions: BTreeMap::from([(
                    development_profile::POLICY_ID.into(),
                    state.records[development_profile::POLICY_ID]
                        .revision_digest
                        .clone(),
                )]),
            },
        },
    )
    .await;
    cmd(
        actor,
        &format!("cap-{id}"),
        MemoryCommand::RegisterWorker {
            capability: WorkerCapability {
                id: id.into(),
                scope: owner(),
                principal: worker(),
                worker_id: "profile-worker".into(),
                revision: 1,
                revoked: false,
                allowed_kinds: BTreeSet::from([DevelopmentKind::ProfileProposal]),
                source_ids: sources.clone(),
                record_ids: BTreeSet::from([development_profile::POLICY_ID.into()]),
                new_record_ids: BTreeSet::from([record_id.into()]),
                session_id: "first".into(),
                conversation: "room-first".into(),
                lease: DevelopmentLease {
                    id: format!("lease-{id}"),
                    attempt: 1,
                    expires_at_ns: i64::MAX,
                },
                budget: DevelopmentBudget {
                    reserved_tokens: 20000,
                    max_input_bytes: 65536,
                    max_output_bytes: 65536,
                    max_mutations: 4,
                },
            },
        },
    )
    .await;
}
fn request(cap: &str, sources: &BTreeSet<String>) -> SnapshotRequest {
    SnapshotRequest {
        capability_id: cap.into(),
        source_ids: sources.clone(),
        record_ids: BTreeSet::new(),
    }
}
fn decision(p: &ApprovalProposal, id: &str, kind: ProposalDecisionKind) -> ProposalDecision {
    ProposalDecision {
        request_id: id.into(),
        proposal_id: p.id.clone(),
        expected_revision: p.revision,
        expected_digest: p.digest.clone(),
        decision: kind,
    }
}
#[tokio::test]
async fn enabled_multisession_inference_owner_review_and_rejection_survive_restart() {
    let endpoint = std::env::var("HM_PROFILE_ENDPOINT")
        .expect("actual configured profile provider endpoint required");
    let model =
        std::env::var("HM_PROFILE_MODEL").expect("actual configured profile model required");
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
    source(
        &actor,
        "first",
        "I prefer concise replies with only the key facts. Please keep answers brief.",
    )
    .await;
    source(
        &actor,
        "second",
        "For this separate session, keep your responses concise and brief; focus on key facts.",
    )
    .await;
    let before = actor.stats().await.unwrap();
    assert!(development_profile::collect_session(
        &actor,
        &owner(),
        &owner(),
        "first",
        "room-first"
    )
    .await
    .is_err());
    assert_eq!(actor.stats().await.unwrap(), before);
    assert!(
        development_profile::set_enabled(&actor, &owner(), &worker(), true)
            .await
            .is_err()
    );
    development_profile::set_enabled(&actor, &owner(), &owner(), true)
        .await
        .unwrap();
    let mut sources: BTreeSet<String> =
        development_profile::collect_session(&actor, &owner(), &owner(), "first", "room-first")
            .await
            .unwrap()
            .into_iter()
            .collect();
    cap(&actor, "single", "single-profile", &sources).await;
    assert!(development_profile::propose(
        &actor,
        &owner(),
        &worker(),
        "profile-worker",
        request("single", &sources),
        "single-profile",
        provider.clone()
    )
    .await
    .is_err());
    sources.extend(
        development_profile::collect_session(&actor, &owner(), &owner(), "second", "room-second")
            .await
            .unwrap(),
    );
    cap(&actor, "proposal-one", "profile-one", &sources).await;
    let receipt = development_profile::propose(
        &actor,
        &owner(),
        &worker(),
        "profile-worker",
        request("proposal-one", &sources),
        "profile-one",
        provider.clone(),
    )
    .await
    .unwrap();
    let state = context_memory::rebuild(&actor, &owner()).await.unwrap();
    assert!(!state.records.contains_key("profile-one"));
    let p = state.development_proposals[receipt.proposal_id.as_ref().unwrap()].clone();
    assert_eq!(p.status, ProposalStatus::Pending);
    assert!(p.mutations.len() == 1);
    let accept = decision(&p, "accept-one", ProposalDecisionKind::Accept);
    assert!(
        development_profile::review(&actor, &owner(), &worker(), accept.clone())
            .await
            .is_err()
    );
    let mut stale = accept.clone();
    stale.expected_digest = "invalid".into();
    assert!(
        development_profile::review(&actor, &owner(), &owner(), stale)
            .await
            .is_err()
    );
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(dir.path())).await.unwrap();
    development_profile::review(&actor, &owner(), &owner(), accept)
        .await
        .unwrap();
    let state = context_memory::rebuild(&actor, &owner()).await.unwrap();
    assert!(state.records.contains_key("profile-one"));
    assert!(state.records["profile-one"].confidence <= 900000);
    assert_eq!(state.records["profile-one"].provenance.len(), 2);
    assert_eq!(
        state.records["profile-one"].authority,
        Authority::DerivedInference
    );
    cap(&actor, "proposal-two", "profile-two", &sources).await;
    let receipt = development_profile::propose(
        &actor,
        &owner(),
        &worker(),
        "profile-worker",
        request("proposal-two", &sources),
        "profile-two",
        provider.clone(),
    )
    .await
    .unwrap();
    let state = context_memory::rebuild(&actor, &owner()).await.unwrap();
    let p = state.development_proposals[receipt.proposal_id.as_ref().unwrap()].clone();
    cmd(
        &actor,
        "invalidate-profile-source",
        MemoryCommand::TombstoneSource {
            id: sources.iter().next().unwrap().clone(),
        },
    )
    .await;
    assert!(development_admission::decide(
        &actor,
        &owner(),
        &owner(),
        decision(&p, "stale-source-accept", ProposalDecisionKind::Accept)
    )
    .await
    .is_err());
    development_profile::review(
        &actor,
        &owner(),
        &owner(),
        decision(&p, "reject-two", ProposalDecisionKind::Reject),
    )
    .await
    .unwrap();
    assert!(development_profile::review(
        &actor,
        &owner(),
        &owner(),
        decision(&p, "reaccept-two", ProposalDecisionKind::Accept)
    )
    .await
    .is_err());
    development_profile::set_enabled(&actor, &owner(), &owner(), false)
        .await
        .unwrap();
    assert!(development_profile::collect_session(
        &actor,
        &owner(),
        &owner(),
        "second",
        "room-second"
    )
    .await
    .is_err());
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(dir.path())).await.unwrap();
    let state = context_memory::rebuild(&actor, &owner()).await.unwrap();
    assert_eq!(
        state.development_proposals[&p.id].status,
        ProposalStatus::Rejected
    );
    assert!(!state.records.contains_key("profile-two"));
    assert_eq!(
        development_profile::inspect(&actor, &owner(), &owner())
            .await
            .unwrap()["enabled"],
        false
    );
    assert!(development_admission::decide(
        &actor,
        &owner(),
        &worker(),
        decision(&p, "worker-approve", ProposalDecisionKind::Accept)
    )
    .await
    .is_err());
    actor.shutdown().await.unwrap();
    tokio::task::spawn_blocking(move || drop(provider))
        .await
        .unwrap();
}
