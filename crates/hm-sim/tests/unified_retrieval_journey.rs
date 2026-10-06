use hm_context::retrieval::{RetrievalRequest, SemanticStatus, SourceKind};
use hm_context::{
    Authority, MessagePart, MessageRole, Scope, SourceMessage, SourceSpan, digest_bytes,
};
use hm_core::ActorId;
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_retrieval::{
        self, EmbeddingMode, EmbeddingRegistration, FileIndexRequest, SourceRecord,
    },
};

fn scope() -> Scope {
    Scope {
        owner_id: "retrieval-owner".into(),
        project_id: "retrieval-project".into(),
        workspace_id: None,
    }
}
fn config(root: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: root.join("actor"),
        actor: ActorId::new(29),
        user: [4; 16],
        kek: [8; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}
fn req() -> RetrievalRequest {
    RetrievalRequest {
        scope: scope(),
        now_ns: 100,
        grants: vec![],
        visible: vec![],
        time_filter: None,
        query_vector: None,
        max_candidates: 1000,
        max_results: 100,
        max_tokens: 4000,
    }
}
fn source(kind: SourceKind, id: &str, text: &str, revision: u64) -> SourceRecord {
    SourceRecord {
        scope: scope(),
        kind,
        id: id.into(),
        revision,
        text: text.into(),
        content_digest: digest_bytes(text.as_bytes()),
        authority: Authority::ExternalObserved,
        provenance: vec![SourceSpan {
            source_id: format!("source:{id}"),
            source_digest: digest_bytes(text.as_bytes()),
            byte_start: 0,
            byte_end: text.len() as u64,
        }],
        occurred_at_ns: Some(50),
        recorded_at_ns: 60,
        expires_at_ns: None,
        tombstoned: false,
    }
}
fn tokenizer() -> hm_compose::tokens::TokenCounter {
    hm_compose::tokens::TokenCounter::for_model(
        "gpt-4o",
        None,
        hm_compose::tokens::FallbackWeights::default(),
    )
    .unwrap()
}

#[tokio::test]
async fn ledger_sources_lexical_revisions_tombstones_restart_and_unavailable_embeddings() {
    let temporary = tempfile::tempdir().unwrap();
    let config = config(temporary.path());
    let actor = ActorEngine::open(config.clone()).await.unwrap();
    let text = "Atlas archive contains recoverable history";
    let mut message = SourceMessage {
        id: "raw-history".into(),
        ordinal: 1,
        role: MessageRole::User,
        parts: vec![MessagePart::Text { text: text.into() }],
        occurred_at_ns: Some(50),
        recorded_at_ns: 60,
        authority: Authority::UserAsserted,
        source_digest: String::new(),
    };
    message.source_digest = message.computed_digest().unwrap();
    hm_serve::context_history::ingest(
        &actor,
        &scope(),
        &hm_serve::context_history::SourceIngestion {
            version: 1,
            scope: scope(),
            session_id: "retrieval-session".into(),
            conversation: "retrieval-conversation".into(),
            message,
            original_bytes: text.as_bytes().to_vec(),
        },
    )
    .await
    .unwrap();
    let mut memory = hm_serve::context_memory::MemoryRecord::new(
        "atlas-memory",
        hm_serve::context_memory::RecordKind::Fact,
        "Atlas durable memory observation",
        60,
    );
    memory.authority = Authority::UserAsserted;
    hm_serve::context_memory::execute(
        &actor,
        &scope(),
        &scope(),
        hm_serve::context_memory::MemoryRequest {
            version: 1,
            scope: scope(),
            request_id: "memory-observation".into(),
            command: hm_serve::context_memory::MemoryCommand::Create { record: memory },
        },
    )
    .await
    .unwrap();
    for (kind, id, text) in [
        (
            SourceKind::Document,
            "document",
            "Atlas document records the storage protocol",
        ),
        (SourceKind::Entity, "entity", "Atlas is the archive service"),
        (
            SourceKind::Relationship,
            "relationship",
            "Atlas depends on its vault",
        ),
    ] {
        context_retrieval::register_source(&actor, &scope(), 0, source(kind, id, text, 1))
            .await
            .unwrap();
    }
    std::fs::write(
        temporary.path().join("file.txt"),
        "Atlas canonical file source",
    )
    .unwrap();
    context_retrieval::index_file(
        &actor,
        &scope(),
        &FileIndexRequest {
            scope: scope(),
            project_root: temporary.path().into(),
            path: "file.txt".into(),
            id: "file".into(),
            expected_revision: 0,
            maximum_bytes: 4096,
            recorded_at_ns: 60,
        },
    )
    .await
    .unwrap();
    // A source commit is created with plumbing in an isolated repository.
    let repo = temporary.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    let git = |args: &[&str]| {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args(args)
            .output()
            .unwrap();
        assert!(out.status.success());
        String::from_utf8(out.stdout).unwrap().trim().to_owned()
    };
    git(&["init", "--quiet"]);
    let tree = git(&["mktree"]);
    let commit = std::process::Command::new("git")
        .arg("-C")
        .arg(&repo)
        .args(["commit-tree", &tree, "-m", "Atlas opt-in repository record"])
        .env("GIT_AUTHOR_NAME", "Archive")
        .env("GIT_AUTHOR_EMAIL", "archive@example.invalid")
        .env("GIT_COMMITTER_NAME", "Archive")
        .env("GIT_COMMITTER_EMAIL", "archive@example.invalid")
        .output()
        .unwrap();
    assert!(commit.status.success());
    let oid = String::from_utf8(commit.stdout).unwrap().trim().to_owned();
    context_retrieval::index_commit(
        &actor,
        &scope(),
        &context_retrieval::CommitIndexRequest {
            scope: scope(),
            repository: repo,
            object_id: oid,
            id: "commit".into(),
            expected_revision: 0,
            opt_in: true,
            maximum_bytes: 4096,
            maximum_millis: 2000,
            recorded_at_ns: 60,
        },
    )
    .await
    .unwrap();
    let report = context_retrieval::retrieve(&actor, &scope(), "Atlas", &req(), &tokenizer(), None)
        .await
        .unwrap();
    let kinds: std::collections::BTreeSet<_> =
        report.results.iter().map(|r| r.candidate.kind).collect();
    assert_eq!(kinds.len(), 7);
    assert!(matches!(
        report.semantic,
        SemanticStatus::Unavailable { .. }
    ));
    context_retrieval::register_embedding(
        &actor,
        &scope(),
        0,
        EmbeddingRegistration {
            id: "off".into(),
            revision: 1,
            mode: EmbeddingMode::Off,
            fingerprint: None,
        },
    )
    .await
    .unwrap();
    let fill = context_retrieval::backfill(&actor, &scope(), None, 1, 4096, 100)
        .await
        .unwrap();
    assert_eq!(fill.embedded, 0);
    assert!(fill.unavailable.is_some());
    assert!(fill.checkpoint.is_none());
    context_retrieval::register_source(
        &actor,
        &scope(),
        1,
        source(
            SourceKind::Document,
            "document",
            "Revised document no longer contains the old word",
            2,
        ),
    )
    .await
    .unwrap();
    context_retrieval::tombstone(&actor, &scope(), SourceKind::Entity, "entity", 1)
        .await
        .unwrap();
    let mut forbidden = source(
        SourceKind::Memory,
        "not-canonical",
        "Atlas unsupported memory index",
        1,
    );
    assert!(
        context_retrieval::register_source(&actor, &scope(), 0, forbidden.clone())
            .await
            .is_err()
    );
    forbidden.kind = SourceKind::Document;
    forbidden.scope.owner_id = "foreign".into();
    assert!(
        context_retrieval::register_source(&actor, &scope(), 0, forbidden)
            .await
            .is_err()
    );
    let foreign = Scope {
        owner_id: "foreign-owner".into(),
        project_id: "foreign-project".into(),
        workspace_id: None,
    };
    let mut shared = source(
        SourceKind::Document,
        "shared",
        "Atlas authorized shared source",
        1,
    );
    shared.scope = foreign.clone();
    context_retrieval::register_source(&actor, &foreign, 0, shared.clone())
        .await
        .unwrap();
    let grant = hm_context::retrieval::EvidenceGrant {
        source_scope: foreign.clone(),
        recipient_scope: scope(),
        kind: shared.kind,
        source_id: shared.id.clone(),
        source_digest: shared.content_digest.clone(),
        source_revision: 1,
        expires_at_ns: 200,
    };
    let mut shared_req = req();
    shared_req.grants.push(grant.clone());
    assert!(
        !context_retrieval::retrieve(&actor, &scope(), "Atlas", &shared_req, &tokenizer(), None)
            .await
            .unwrap()
            .results
            .iter()
            .any(|r| r.candidate.id == "shared")
    );
    context_retrieval::set_grant(&actor, &foreign, grant, 100)
        .await
        .unwrap();
    assert!(
        context_retrieval::retrieve(&actor, &scope(), "Atlas", &shared_req, &tokenizer(), None)
            .await
            .unwrap()
            .results
            .iter()
            .any(|r| r.candidate.id == "shared")
    );
    context_retrieval::revoke_grant(&actor, &foreign, &scope(), SourceKind::Document, "shared")
        .await
        .unwrap();
    assert!(
        !context_retrieval::retrieve(&actor, &scope(), "Atlas", &shared_req, &tokenizer(), None)
            .await
            .unwrap()
            .results
            .iter()
            .any(|r| r.candidate.id == "shared")
    );
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config).await.unwrap();
    let report = context_retrieval::retrieve(&actor, &scope(), "Atlas", &req(), &tokenizer(), None)
        .await
        .unwrap();
    assert!(
        !report
            .results
            .iter()
            .any(|r| r.candidate.id == "document" || r.candidate.id == "entity")
    );
    let mut visible = req();
    visible.visible = report
        .results
        .iter()
        .flat_map(|r| r.candidate.provenance.clone())
        .collect();
    assert!(
        context_retrieval::retrieve(&actor, &scope(), "Atlas", &visible, &tokenizer(), None)
            .await
            .unwrap()
            .results
            .is_empty()
    );
    assert_eq!(
        context_retrieval::inspect(&actor, &scope()).await.unwrap()["vector_count"],
        0
    );
    actor.shutdown().await.unwrap();
}

#[tokio::test]
#[ignore = "requires an actual configured embedding endpoint; incurs live embedding requests"]
async fn actual_provider_backfill_resumes_and_semantic_query_survives_restart() {
    use hm_embed::{HttpTransport, Provider, RemoteConfig, RemoteEmbedder};
    use std::sync::Arc;
    let endpoint =
        std::env::var("HM_RETRIEVAL_EMBEDDING_ENDPOINT").expect("embedding endpoint configuration");
    let model =
        std::env::var("HM_RETRIEVAL_EMBEDDING_MODEL").expect("embedding model configuration");
    let revision =
        std::env::var("HM_RETRIEVAL_EMBEDDING_REVISION").expect("embedding revision configuration");
    let dimensions = std::env::var("HM_RETRIEVAL_EMBEDDING_DIMENSIONS")
        .expect("embedding dimension configuration")
        .parse()
        .unwrap();
    let remote = RemoteEmbedder::new(
        Provider::OpenAi,
        RemoteConfig {
            endpoint,
            model,
            revision,
            dimensions,
            maximum_batch: 2,
            api_key: std::env::var("HM_RETRIEVAL_EMBEDDING_KEY").ok(),
        },
        HttpTransport::default(),
    )
    .unwrap();
    let provider = context_retrieval::EmbeddingProvider::new(
        "actual-encoder",
        1,
        EmbeddingMode::RemoteCompatible,
        Arc::new(remote),
    )
    .unwrap();
    actual_provider_journey(provider).await;
}

#[tokio::test]
#[ignore = "requires pinned BGE model files and an actual ONNX Runtime installation"]
async fn actual_local_model_backfill_resumes() {
    let directory =
        std::env::var("HM_RETRIEVAL_MODEL_DIRECTORY").expect("model directory configuration");
    let encoder = tokio::task::spawn_blocking(move || {
        hm_embed::OnnxEmbedder::download(
            hm_embed::ModelKind::BgeSmallEnV15,
            &hm_embed::ModelStore::new(directory),
            &hm_embed::HttpFetcher,
        )
    })
    .await
    .unwrap()
    .expect("actual pinned BGE encoder");
    let provider = context_retrieval::EmbeddingProvider::new(
        "actual-local-encoder",
        1,
        EmbeddingMode::Local,
        std::sync::Arc::new(encoder),
    )
    .unwrap();
    let fingerprint = provider.registration.fingerprint.as_ref().unwrap();
    assert_eq!(
        fingerprint.model,
        hm_embed::ModelKind::BgeSmallEnV15.spec().encoder_id
    );
    assert_eq!(fingerprint.dimensions, 384);
    assert_eq!(
        fingerprint.revision,
        digest_bytes(
            format!(
                "{}:Cosine:L2",
                hm_embed::ModelKind::BgeSmallEnV15.spec().revision
            )
            .as_bytes()
        )
    );
    actual_provider_journey(provider).await;
}

async fn actual_provider_journey(provider: context_retrieval::EmbeddingProvider) {
    let temporary = tempfile::tempdir().unwrap();
    let config = config(temporary.path());
    let actor = ActorEngine::open(config.clone()).await.unwrap();
    context_retrieval::register_source(
        &actor,
        &scope(),
        0,
        source(
            SourceKind::Document,
            "vault",
            "The Atlas disaster recovery copy is stored in the Berlin vault.",
            1,
        ),
    )
    .await
    .unwrap();
    context_retrieval::register_source(
        &actor,
        &scope(),
        0,
        source(
            SourceKind::Document,
            "fruit",
            "Ripe oranges and bananas are stored in the kitchen.",
            1,
        ),
    )
    .await
    .unwrap();
    context_retrieval::register_embedding(&actor, &scope(), 0, provider.registration.clone())
        .await
        .unwrap();
    let first = context_retrieval::backfill(&actor, &scope(), Some(&provider), 1, 4096, 100)
        .await
        .unwrap();
    assert_eq!(first.embedded, 1);
    assert_eq!(first.remaining, 1);
    assert!(first.unavailable.is_none());
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config).await.unwrap();
    let second = context_retrieval::backfill(&actor, &scope(), Some(&provider), 1, 4096, 100)
        .await
        .unwrap();
    assert_eq!(second.embedded, 1);
    assert_eq!(second.remaining, 0);
    let report = context_retrieval::retrieve(
        &actor,
        &scope(),
        "Where is the database recovery backup?",
        &req(),
        &tokenizer(),
        Some(&provider),
    )
    .await
    .unwrap();
    assert!(matches!(
        report.semantic,
        SemanticStatus::Available {
            scored: 2,
            unavailable: 0
        }
    ));
    for result in &report.results {
        let vector = result.candidate.vector.as_ref().unwrap();
        assert_eq!(
            Some(&vector.fingerprint),
            provider.registration.fingerprint.as_ref()
        );
        assert_eq!(vector.content_digest, result.candidate.content_digest);
    }
    let vault = report
        .results
        .iter()
        .find(|r| r.candidate.id == "vault")
        .unwrap()
        .semantic_score
        .unwrap();
    let fruit = report
        .results
        .iter()
        .find(|r| r.candidate.id == "fruit")
        .unwrap()
        .semantic_score
        .unwrap();
    println!("semantic scores: vault={vault} fruit={fruit}");
    assert!(vault > fruit);
    assert_eq!(
        context_retrieval::backfill(&actor, &scope(), Some(&provider), 1, 4096, 100)
            .await
            .unwrap()
            .embedded,
        0
    );
    context_retrieval::register_source(
        &actor,
        &scope(),
        1,
        source(
            SourceKind::Document,
            "vault",
            "The current database recovery backup is now stored in the Paris vault.",
            2,
        ),
    )
    .await
    .unwrap();
    let report = context_retrieval::retrieve(
        &actor,
        &scope(),
        "recovery",
        &req(),
        &tokenizer(),
        Some(&provider),
    )
    .await
    .unwrap();
    assert_eq!(
        report
            .results
            .iter()
            .find(|r| r.candidate.id == "vault")
            .unwrap()
            .semantic_score,
        None
    );
    let repaired = context_retrieval::backfill(&actor, &scope(), Some(&provider), 1, 4096, 100)
        .await
        .unwrap();
    assert_eq!(repaired.embedded, 1);
    assert_eq!(repaired.remaining, 0);
    let fresh = context_retrieval::retrieve(
        &actor,
        &scope(),
        "Where is the recovery backup?",
        &req(),
        &tokenizer(),
        Some(&provider),
    )
    .await
    .unwrap();
    assert!(
        fresh
            .results
            .iter()
            .find(|r| r.candidate.id == "vault")
            .unwrap()
            .semantic_score
            .is_some()
    );
    actor.shutdown().await.unwrap();
}
