use hm_context::{development::*, digest_bytes, Scope};
use hm_core::ActorId;
use hm_cortex::development_documents::DocumentationPatch;
use hm_llm::{ollama::Ollama, HttpTransport, LlmProvider, ModelTier, Pricing, ProviderConfig};
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_memory::{self, MemoryCommand, MemoryRequest, MemorySource},
    development_documents as docs,
};
use std::{collections::BTreeSet, sync::Arc};
fn scope(owner: &str) -> Scope {
    Scope {
        owner_id: owner.into(),
        project_id: "documents".into(),
        workspace_id: None,
    }
}
async fn command(actor: &ActorEngine, id: &str, command: MemoryCommand) {
    context_memory::execute(
        actor,
        &scope("owner"),
        &scope("owner"),
        MemoryRequest {
            version: 1,
            scope: scope("owner"),
            request_id: id.into(),
            command,
        },
    )
    .await
    .unwrap();
}
#[tokio::test]
async fn real_repository_review_stale_refusal_owner_application_and_receipt() {
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    let before = b"pub const MAX_RESULTS: usize = 10;\n".to_vec();
    std::fs::write(repo.join("limits.rs"), &before).unwrap();
    let baseline = std::fs::read(repo.join("limits.rs")).unwrap();
    std::fs::write(
        repo.join("limits.rs"),
        b"pub const MAX_RESULTS: usize = 20;\n",
    )
    .unwrap();
    let document = b"The maximum number of results is 10.\n".to_vec();
    std::fs::write(repo.join("README.md"), &document).unwrap();
    let delta = docs::capture(&repo, "limits.rs", baseline, "README.md").unwrap();
    let bytes = delta.evidence_bytes().unwrap();
    let config = ActorConfig {
        actor_directory: temp.path().join("ledger"),
        actor: ActorId::new(54),
        user: [8; 16],
        kek: [9; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    };
    let actor = ActorEngine::open(config.clone()).await.unwrap();
    command(
        &actor,
        "source",
        MemoryCommand::Source {
            source: MemorySource {
                id: "delta".into(),
                digest: digest_bytes(&bytes),
                content: bytes,
                locator: "repository:limits.rs".into(),
                occurred_at_ns: None,
                recorded_at_ns: 1,
                tombstoned: false,
            },
        },
    )
    .await;
    command(
        &actor,
        "cap",
        MemoryCommand::RegisterWorker {
            capability: WorkerCapability {
                id: "cap".into(),
                scope: scope("owner"),
                principal: scope("worker"),
                worker_id: "docs-worker".into(),
                revision: 1,
                revoked: false,
                allowed_kinds: BTreeSet::from([DevelopmentKind::DocumentationProposal]),
                source_ids: BTreeSet::from(["delta".into()]),
                record_ids: BTreeSet::new(),
                new_record_ids: BTreeSet::from(["patch".into()]),
                session_id: "session".into(),
                conversation: "conversation".into(),
                lease: DevelopmentLease {
                    id: "lease".into(),
                    attempt: 1,
                    expires_at_ns: i64::MAX,
                },
                budget: DevelopmentBudget {
                    reserved_tokens: 16384,
                    max_input_bytes: 65536,
                    max_output_bytes: 65536,
                    max_mutations: 4,
                },
            },
        },
    )
    .await;
    let provider: Arc<dyn LlmProvider> = tokio::task::spawn_blocking(|| -> Arc<dyn LlmProvider> {
        Arc::new(
            Ollama::new(
                ProviderConfig {
                    endpoint: std::env::var("HM_TEST_OLLAMA_ENDPOINT").expect("set HM_TEST_OLLAMA_ENDPOINT to the configured local generation endpoint"),
                    api_key: None,
                    model: "qwen2.5:3b".into(),
                    tier: ModelTier::Economy,
                    pricing: Pricing::default(),
                },
                HttpTransport::default(),
            )
            .unwrap(),
        )
    })
    .await
    .unwrap();
    docs::propose(
        &actor,
        &scope("owner"),
        &scope("worker"),
        "docs-worker",
        &repo,
        delta,
        docs::DocumentRequest {
            plan_id: "plan".into(),
            proposal_id: "proposal".into(),
            patch_record_id: "patch".into(),
            source_id: "delta".into(),
            snapshot: SnapshotRequest {
                capability_id: "cap".into(),
                source_ids: BTreeSet::from(["delta".into()]),
                record_ids: BTreeSet::new(),
            },
        },
        provider,
    )
    .await
    .unwrap();
    assert_eq!(std::fs::read(repo.join("README.md")).unwrap(), document);
    let state = context_memory::rebuild(&actor, &scope("owner"))
        .await
        .unwrap();
    let proposal = state.development_proposals["proposal"].clone();
    let decision = ProposalDecision {
        request_id: "review".into(),
        proposal_id: proposal.id.clone(),
        expected_revision: proposal.revision,
        expected_digest: proposal.digest.clone(),
        decision: ProposalDecisionKind::Accept,
    };
    assert!(docs::review(
        &actor,
        &scope("owner"),
        &scope("worker"),
        &repo,
        decision.clone()
    )
    .await
    .is_err());
    std::fs::write(repo.join("README.md"), b"Owner edit\n").unwrap();
    assert!(docs::review(
        &actor,
        &scope("owner"),
        &scope("owner"),
        &repo,
        decision.clone()
    )
    .await
    .is_err());
    std::fs::write(repo.join("README.md"), &document).unwrap();
    docs::review(&actor, &scope("owner"), &scope("owner"), &repo, decision)
        .await
        .unwrap();
    let state = context_memory::rebuild(&actor, &scope("owner"))
        .await
        .unwrap();
    let accepted = state.development_proposals["proposal"].clone();
    let patch: DocumentationPatch =
        serde_json::from_value(state.records["patch"].metadata.clone()).unwrap();
    assert!(String::from_utf8_lossy(&patch.replacement).contains("20"));
    std::fs::write(repo.join("README.md"), b"Owner second edit\n").unwrap();
    assert!(docs::stage(
        &actor,
        &scope("owner"),
        &scope("owner"),
        &repo,
        "proposal",
        accepted.revision,
        &accepted.digest
    )
    .await
    .is_err());
    assert_eq!(
        std::fs::read(repo.join("README.md")).unwrap(),
        b"Owner second edit\n"
    );
    std::fs::write(repo.join("README.md"), &document).unwrap();
    let stage = docs::stage(
        &actor,
        &scope("owner"),
        &scope("owner"),
        &repo,
        "proposal",
        accepted.revision,
        &accepted.digest,
    )
    .await
    .unwrap();
    assert_eq!(std::fs::read(repo.join("README.md")).unwrap(), document);
    assert!(docs::acknowledge_applied(
        &actor,
        &scope("owner"),
        &scope("owner"),
        &repo,
        "proposal",
        accepted.revision,
        &accepted.digest
    )
    .await
    .is_err());
    std::fs::copy(&stage, repo.join("README.md")).unwrap();
    let receipt = docs::acknowledge_applied(
        &actor,
        &scope("owner"),
        &scope("owner"),
        &repo,
        "proposal",
        accepted.revision,
        &accepted.digest,
    )
    .await
    .unwrap();
    assert!(receipt.applied);
    let before = actor
        .frames_since(hm_core::LSN::new(0), None, usize::MAX)
        .await
        .unwrap()
        .len();
    drop(actor);
    let actor = ActorEngine::open(config).await.unwrap();
    let replay = docs::acknowledge_applied(
        &actor,
        &scope("owner"),
        &scope("owner"),
        &repo,
        "proposal",
        accepted.revision,
        &accepted.digest,
    )
    .await
    .unwrap();
    assert_eq!(receipt, replay);
    assert_eq!(
        before,
        actor
            .frames_since(hm_core::LSN::new(0), None, usize::MAX)
            .await
            .unwrap()
            .len()
    );
}
