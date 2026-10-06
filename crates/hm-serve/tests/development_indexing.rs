use hm_context::{
    Authority, MessagePart, MessageRole, Scope, SourceMessage,
    history::SourceRelation,
    retrieval::{RetrievalRequest, SourceKind},
};
use hm_core::ActorId;
use hm_embed::{HttpFetcher, ModelKind, ModelStore, OnnxEmbedder};
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_history::{self, RelationIngestion, SourceIngestion},
    context_memory::{self, MemoryCommand, MemoryRecord, MemoryRequest, RecordKind},
    context_retrieval::{self, EmbeddingMode, EmbeddingProvider},
    development_indexing::{self, FileTarget, GitTarget, IndexingPolicy},
};
use std::{path::Path, sync::Arc};
fn scope() -> Scope {
    Scope {
        owner_id: "index-owner".into(),
        project_id: "index-project".into(),
        workspace_id: None,
    }
}
fn config(path: &Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.join("actor"),
        actor: ActorId::new(71),
        user: [4; 16],
        kek: [8; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}
fn request() -> RetrievalRequest {
    RetrievalRequest {
        scope: scope(),
        now_ns: 100,
        grants: vec![],
        visible: vec![],
        time_filter: None,
        query_vector: None,
        max_candidates: 100,
        max_results: 100,
        max_tokens: 10000,
    }
}
async fn memory(actor: &ActorEngine, id: &str, command: MemoryCommand) {
    context_memory::execute(
        actor,
        &scope(),
        &scope(),
        MemoryRequest {
            version: 1,
            scope: scope(),
            request_id: id.into(),
            command,
        },
    )
    .await
    .unwrap();
}
async fn history(actor: &ActorEngine, id: &str, ordinal: u64, text: &str) {
    let mut message = SourceMessage {
        id: id.into(),
        ordinal,
        role: MessageRole::User,
        parts: vec![MessagePart::Text { text: text.into() }],
        occurred_at_ns: Some(50),
        recorded_at_ns: 60,
        authority: Authority::UserAsserted,
        source_digest: String::new(),
    };
    message.source_digest = message.computed_digest().unwrap();
    context_history::ingest(
        actor,
        &scope(),
        &SourceIngestion {
            version: 1,
            scope: scope(),
            session_id: "session".into(),
            conversation: "conversation".into(),
            message,
            original_bytes: text.as_bytes().to_vec(),
        },
    )
    .await
    .unwrap();
}
#[tokio::test]
#[ignore = "requires verified local model artifacts and an authorized actual Git repository"]
async fn actual_bge_indexing_frontier_scan_correction_grants_and_restart() {
    let directory = std::env::var("HM_RETRIEVAL_MODEL_DIRECTORY").expect("actual model directory");
    let repository = std::path::PathBuf::from(
        std::env::var("HM_INDEXING_GIT_REPOSITORY").expect("authorized Git repository"),
    );
    let head = std::process::Command::new("git")
        .arg("-C")
        .arg(&repository)
        .args(["rev-parse", "--verify", "HEAD"])
        .output()
        .unwrap();
    assert!(head.status.success());
    let object_id = String::from_utf8(head.stdout).unwrap().trim().to_owned();
    let embedder = tokio::task::spawn_blocking(move || {
        OnnxEmbedder::download(
            ModelKind::BgeSmallEnV15,
            &ModelStore::new(directory),
            &HttpFetcher,
        )
    })
    .await
    .unwrap()
    .unwrap();
    let provider =
        EmbeddingProvider::new("index-bge", 1, EmbeddingMode::Local, Arc::new(embedder)).unwrap();
    assert_eq!(
        provider
            .registration
            .fingerprint
            .as_ref()
            .unwrap()
            .dimensions,
        384
    );
    let root = tempfile::tempdir().unwrap();
    let files = root.path().join("files");
    std::fs::create_dir(&files).unwrap();
    std::fs::write(
        files.join("vault.txt"),
        "The database recovery backup is stored in the Paris vault.",
    )
    .unwrap();
    std::fs::write(
        files.join("fruit.txt"),
        "Fresh bananas and apples are sold at the fruit market.",
    )
    .unwrap();
    let actor = ActorEngine::open(config(root.path())).await.unwrap();
    history(
        &actor,
        "original",
        1,
        "The backup location is the Warsaw secure storage vault.",
    )
    .await;
    memory(
        &actor,
        "memory",
        MemoryCommand::Create {
            record: MemoryRecord::new(
                "memory",
                RecordKind::Note,
                "The recovery backup uses encrypted storage.",
                60,
            ),
        },
    )
    .await;
    context_retrieval::register_embedding(&actor, &scope(), 0, provider.registration.clone())
        .await
        .unwrap();
    let policy = IndexingPolicy {
        scope: scope(),
        principal: scope(),
        revision: 1,
        enabled: true,
        files: vec![
            FileTarget {
                root: files.clone(),
                path: "vault.txt".into(),
            },
            FileTarget {
                root: files.clone(),
                path: "fruit.txt".into(),
            },
        ],
        commits: vec![GitTarget {
            repository,
            object_id,
            opt_in: true,
        }],
        maximum_sources: 1,
        maximum_source_bytes: 128 * 1024,
        maximum_embedding_items: 1,
        maximum_embedding_bytes: 128 * 1024,
    };
    let foreign = Scope {
        owner_id: "foreign".into(),
        ..scope()
    };
    assert!(
        development_indexing::configure(&actor, &scope(), &foreign, 0, policy.clone())
            .await
            .is_err()
    );
    let mut git_declined = policy.clone();
    git_declined.commits[0].opt_in = false;
    assert!(
        development_indexing::configure(&actor, &scope(), &scope(), 0, git_declined)
            .await
            .is_err()
    );
    development_indexing::configure(&actor, &scope(), &scope(), 0, policy.clone())
        .await
        .unwrap();
    let first = development_indexing::reconcile(&actor, &scope(), &scope(), Some(&provider), 100)
        .await
        .unwrap();
    assert_eq!(first.scanned, 1);
    assert_eq!(first.embedded, 1);
    assert!(first.earliest_dirty.is_some());
    assert!(first.remaining > 0);
    let earliest = first.earliest_dirty;
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(root.path())).await.unwrap();
    assert_eq!(
        development_indexing::inspect(&actor, &scope())
            .await
            .unwrap()["scan_cursor"],
        1
    );
    let mut last = first;
    for _ in 0..15 {
        last = development_indexing::reconcile(&actor, &scope(), &scope(), Some(&provider), 100)
            .await
            .unwrap();
        if last.earliest_dirty.is_none() {
            break;
        }
        assert_eq!(last.earliest_dirty, earliest);
    }
    assert_eq!(last.remaining, 0);
    assert!(last.earliest_dirty.is_none());
    assert!(last.frontier > 0);
    let tokenizer =
        hm_compose::tokens::TokenCounter::for_model("gpt-4o", None, Default::default()).unwrap();
    let report = context_retrieval::retrieve(
        &actor,
        &scope(),
        "Where is the recovery backup?",
        &request(),
        &tokenizer,
        Some(&provider),
    )
    .await
    .unwrap();
    let vault = report
        .results
        .iter()
        .find(|r| r.candidate.text.contains("Paris vault"))
        .unwrap();
    let fruit = report
        .results
        .iter()
        .find(|r| r.candidate.text.contains("fruit market"))
        .unwrap();
    assert!(vault.semantic_score.unwrap() > fruit.semantic_score.unwrap());
    assert!(
        report
            .results
            .iter()
            .any(|r| r.candidate.kind == SourceKind::Commit)
    );
    let vault_id = vault.candidate.id.clone();
    let vector_digest = vault.candidate.content_digest.clone();
    history(
        &actor,
        "replacement",
        2,
        "The backup location is now the Madrid vault.",
    )
    .await;
    context_history::relate(
        &actor,
        &scope(),
        &RelationIngestion {
            version: 1,
            scope: scope(),
            session_id: "session".into(),
            conversation: "conversation".into(),
            relation: SourceRelation::Edit {
                id: "correction".into(),
                original_id: "original".into(),
                replacement_id: "replacement".into(),
            },
        },
    )
    .await
    .unwrap();
    let corrected = context_retrieval::retrieve(
        &actor,
        &scope(),
        "backup",
        &request(),
        &tokenizer,
        Some(&provider),
    )
    .await
    .unwrap();
    assert!(
        !corrected
            .results
            .iter()
            .any(|r| r.candidate.text.contains("Warsaw"))
    );
    let replacement = corrected
        .results
        .iter()
        .find(|r| r.candidate.text.contains("Madrid"))
        .unwrap();
    assert!(replacement.semantic_score.is_none());
    std::fs::write(
        files.join("vault.txt"),
        "The database recovery backup moved to the Lisbon vault.",
    )
    .unwrap();
    let mut changed = false;
    for _ in 0..3 {
        let step = development_indexing::reconcile(&actor, &scope(), &scope(), None, 100)
            .await
            .unwrap();
        assert!(step.unavailable.is_some());
        assert!(step.earliest_dirty.is_some());
        if step.changed > 0 {
            changed = true;
            break;
        }
    }
    assert!(changed);
    let corrected = context_retrieval::retrieve(
        &actor,
        &scope(),
        "backup",
        &request(),
        &tokenizer,
        Some(&provider),
    )
    .await
    .unwrap();
    let current = corrected
        .results
        .iter()
        .find(|r| r.candidate.id == vault_id)
        .unwrap();
    assert_ne!(current.candidate.content_digest, vector_digest);
    assert!(current.semantic_score.is_none());
    for _ in 0..15 {
        let step =
            development_indexing::reconcile(&actor, &scope(), &scope(), Some(&provider), 100)
                .await
                .unwrap();
        if step.earliest_dirty.is_none() {
            break;
        }
    }
    let fresh = context_retrieval::retrieve(
        &actor,
        &scope(),
        "backup",
        &request(),
        &tokenizer,
        Some(&provider),
    )
    .await
    .unwrap();
    assert!(
        fresh
            .results
            .iter()
            .find(|r| r.candidate.id == vault_id)
            .unwrap()
            .semantic_score
            .is_some()
    );
    let mut disabled = policy.clone();
    disabled.revision = 2;
    disabled.enabled = false;
    development_indexing::configure(&actor, &scope(), &scope(), 1, disabled)
        .await
        .unwrap();
    assert!(
        development_indexing::reconcile(&actor, &scope(), &scope(), Some(&provider), 100)
            .await
            .is_err()
    );
    let revoked = context_retrieval::retrieve(
        &actor,
        &scope(),
        "backup",
        &request(),
        &tokenizer,
        Some(&provider),
    )
    .await
    .unwrap();
    assert!(
        !revoked
            .results
            .iter()
            .any(|r| matches!(r.candidate.kind, SourceKind::File | SourceKind::Commit))
    );
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(root.path())).await.unwrap();
    assert_eq!(
        development_indexing::inspect(&actor, &scope())
            .await
            .unwrap()["enabled"],
        false
    );
    assert_eq!(
        context_retrieval::inspect(&actor, &scope()).await.unwrap()["vector_count"],
        2
    );
    actor.shutdown().await.unwrap();
}
