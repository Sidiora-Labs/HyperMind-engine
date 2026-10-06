use hm_context::{Authority, Scope, development::*, digest_bytes};
use hm_core::ActorId;
use hm_cortex::development_mapping::{AuthorizedRepository, MappingClass};
use hm_llm::{HttpTransport, ModelTier, Pricing, ProviderConfig, ollama::Ollama};
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_memory::{
        self, MemoryCommand, MemoryGrant, MemoryRecord, MemoryRequest, MemorySource, Provenance,
        RecordKind,
    },
    development_mapping,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
fn owner() -> Scope {
    Scope {
        owner_id: "owner".into(),
        project_id: "mapping".into(),
        workspace_id: None,
    }
}
fn worker() -> Scope {
    Scope {
        owner_id: "verifier".into(),
        project_id: "mapping".into(),
        workspace_id: None,
    }
}
fn config(root: &Path) -> ActorConfig {
    ActorConfig {
        actor_directory: root.into(),
        actor: ActorId::new(35),
        user: [5; 16],
        kek: [3; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}
async fn command(actor: &ActorEngine, id: &str, command: MemoryCommand) {
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
fn cap(revision: u64) -> WorkerCapability {
    WorkerCapability {
        id: "mapping-capability".into(),
        scope: owner(),
        principal: worker(),
        worker_id: "source-verifier".into(),
        revision,
        revoked: false,
        allowed_kinds: BTreeSet::from([DevelopmentKind::Verification]),
        source_ids: BTreeSet::from(["archive-source".into()]),
        record_ids: BTreeSet::from([
            "supported".into(),
            "contradicted".into(),
            "unresolved".into(),
        ]),
        new_record_ids: BTreeSet::new(),
        session_id: "session".into(),
        conversation: "conversation".into(),
        lease: DevelopmentLease {
            id: "mapping-lease".into(),
            attempt: revision,
            expires_at_ns: i64::MAX,
        },
        budget: DevelopmentBudget {
            reserved_tokens: 8192,
            max_input_bytes: 65536,
            max_output_bytes: 65536,
            max_mutations: 8,
        },
    }
}
fn request() -> SnapshotRequest {
    SnapshotRequest {
        capability_id: "mapping-capability".into(),
        source_ids: BTreeSet::from(["archive-source".into()]),
        record_ids: BTreeSet::from([
            "supported".into(),
            "contradicted".into(),
            "unresolved".into(),
        ]),
    }
}
fn grant() -> MemoryGrant {
    MemoryGrant {
        id: "mapping-read".into(),
        principal: worker(),
        principal_digest: None,
        record_ids: request().record_ids,
        categories: BTreeSet::new(),
        read: true,
        expires_at_ns: None,
        revoked: false,
        revision: 1,
        record_revisions: BTreeMap::new(),
    }
}
fn provider() -> Ollama<HttpTransport> {
    Ollama::new(
        ProviderConfig {
            endpoint: std::env::var("HYPERMIND_MAPPING_ENDPOINT")
                .expect("actual provider endpoint required"),
            api_key: None,
            model: std::env::var("HYPERMIND_MAPPING_MODEL")
                .expect("actual provider model required"),
            tier: ModelTier::Economy,
            pricing: Pricing::default(),
        },
        HttpTransport::default(),
    )
    .unwrap()
}
#[tokio::test]
async fn actual_repository_and_provider_publish_incrementally_with_native_fences() {
    let directory = tempfile::tempdir().unwrap();
    let repository = tempfile::tempdir().unwrap();
    let bytes = b"The archive is stored in Paris, France.";
    std::fs::write(repository.path().join("archive.txt"), bytes).unwrap();
    let files = AuthorizedRepository {
        root: repository.path().into(),
        files: BTreeMap::from([("archive-source".into(), "archive.txt".into())]),
    };
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    command(
        &actor,
        "original-source",
        MemoryCommand::Source {
            source: MemorySource {
                id: "archive-source".into(),
                digest: digest_bytes(bytes),
                content: bytes.to_vec(),
                locator: "repository:archive.txt".into(),
                occurred_at_ns: None,
                recorded_at_ns: 100,
                tombstoned: false,
            },
        },
    )
    .await;
    for (id, claim) in [
        ("supported", "The archive is stored in Paris."),
        (
            "contradicted",
            "The archive is stored in London, not Paris.",
        ),
        ("unresolved", "The archive was founded in 1901."),
    ] {
        let mut record = MemoryRecord::new(id, RecordKind::Note, claim, 100);
        record.authority = Authority::DerivedInference;
        record.provenance = vec![Provenance {
            source_id: "archive-source".into(),
            source_digest: digest_bytes(bytes),
            span_start: 0,
            span_end: bytes.len() as u64,
            quoted_digest: digest_bytes(bytes),
        }];
        command(
            &actor,
            &format!("create-{id}"),
            MemoryCommand::Create { record },
        )
        .await;
    }
    command(
        &actor,
        "read-sharing",
        MemoryCommand::SetGrant { grant: grant() },
    )
    .await;
    assert!(
        development_mapping::map_evidence(
            &actor,
            &owner(),
            &worker(),
            "source-verifier",
            request(),
            &files
        )
        .await
        .is_err()
    );
    command(
        &actor,
        "register-worker",
        MemoryCommand::RegisterWorker { capability: cap(1) },
    )
    .await;
    let (snapshot, mappings) = development_mapping::map_evidence(
        &actor,
        &owner(),
        &worker(),
        "source-verifier",
        request(),
        &files,
    )
    .await
    .unwrap();
    assert!(
        mappings
            .records
            .iter()
            .all(|r| r.class == MappingClass::Repository)
    );
    let independent = AuthorizedRepository {
        root: repository.path().into(),
        files: BTreeMap::new(),
    };
    let independent_mappings =
        hm_cortex::development_mapping::map_evidence(&snapshot, &independent).unwrap();
    assert!(
        independent_mappings
            .records
            .iter()
            .all(|r| r.class == MappingClass::SourceIndependent)
    );
    let mut generated = snapshot.clone();
    generated.sources[0].authority = Authority::DerivedInference;
    generated.digest = generated.computed_digest().unwrap();
    assert!(hm_cortex::development_mapping::map_evidence(&generated, &files).is_err());
    let mut bounded = snapshot.clone();
    bounded.budget.max_mutations = 1;
    bounded.digest = bounded.computed_digest().unwrap();
    let backlog = hm_cortex::development_mapping::map_evidence(&bounded, &files).unwrap();
    assert_eq!(backlog.records.len(), 1);
    assert_eq!(backlog.backlog.len(), 2);
    let provider: std::sync::Arc<dyn hm_llm::LlmProvider> = tokio::task::spawn_blocking(|| {
        std::sync::Arc::new(provider()) as std::sync::Arc<dyn hm_llm::LlmProvider>
    })
    .await
    .unwrap();
    let batch = development_mapping::verify_changed(
        &actor,
        &owner(),
        &worker(),
        "source-verifier",
        snapshot.clone(),
        &files,
        provider.clone(),
        "mapping-first",
        1000,
    )
    .await
    .unwrap();
    assert_eq!(batch.records.len(), 3);
    for (id, state) in [
        ("supported", DevelopmentVerificationState::Supported),
        ("contradicted", DevelopmentVerificationState::Contradicted),
        ("unresolved", DevelopmentVerificationState::Unresolved),
    ] {
        let finding = &batch
            .records
            .iter()
            .find(|r| r.record_id == id)
            .unwrap()
            .findings[0];
        assert_eq!(finding.state, state);
        assert_eq!(
            digest_bytes(&bytes[finding.span.start as usize..finding.span.end as usize]),
            finding.span.quoted_digest
        );
    }
    let before = actor.stats().await.unwrap().log_events;
    std::fs::rename(
        repository.path().join("archive.txt"),
        repository.path().join("moved.txt"),
    )
    .unwrap();
    assert!(
        development_mapping::publish(
            &actor,
            &owner(),
            &worker(),
            "source-verifier",
            &files,
            batch.clone()
        )
        .await
        .is_err()
    );
    assert_eq!(actor.stats().await.unwrap().log_events, before);
    std::fs::rename(
        repository.path().join("moved.txt"),
        repository.path().join("archive.txt"),
    )
    .unwrap();
    std::fs::write(repository.path().join("archive.txt"), b"The archive moved.").unwrap();
    assert!(
        development_mapping::publish(
            &actor,
            &owner(),
            &worker(),
            "source-verifier",
            &files,
            batch.clone()
        )
        .await
        .is_err()
    );
    assert_eq!(actor.stats().await.unwrap().log_events, before);
    std::fs::write(repository.path().join("archive.txt"), bytes).unwrap();
    let receipt = development_mapping::publish(
        &actor,
        &owner(),
        &worker(),
        "source-verifier",
        &files,
        batch,
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(receipt.mutation_ids.len(), 3);
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    command(
        &actor,
        "renew-worker",
        MemoryCommand::RegisterWorker { capability: cap(2) },
    )
    .await;
    let (snapshot, _) = development_mapping::map_evidence(
        &actor,
        &owner(),
        &worker(),
        "source-verifier",
        request(),
        &files,
    )
    .await
    .unwrap();
    let unchanged = development_mapping::verify_changed(
        &actor,
        &owner(),
        &worker(),
        "source-verifier",
        snapshot,
        &files,
        provider.clone(),
        "mapping-unchanged",
        2000,
    )
    .await
    .unwrap();
    assert_eq!(unchanged.skipped.len(), 3);
    assert!(unchanged.records.is_empty());
    let before = actor.stats().await.unwrap().log_events;
    assert!(
        development_mapping::publish(
            &actor,
            &owner(),
            &worker(),
            "source-verifier",
            &files,
            unchanged
        )
        .await
        .unwrap()
        .is_none()
    );
    assert_eq!(actor.stats().await.unwrap().log_events, before);
    let mut revised = context_memory::rebuild(&actor, &owner())
        .await
        .unwrap()
        .records["unresolved"]
        .clone();
    let revision = revised.revision;
    revised.revision += 1;
    revised.content = "The archive is stored in Paris, France.".into();
    revised.revision_digest.clear();
    command(
        &actor,
        "change-one-claim",
        MemoryCommand::Revise {
            record: revised,
            expected_revision: revision,
        },
    )
    .await;
    let mut renewed_grant = grant();
    renewed_grant.revision = 2;
    command(
        &actor,
        "share-revised-claim",
        MemoryCommand::SetGrant {
            grant: renewed_grant,
        },
    )
    .await;
    let (snapshot, _) = development_mapping::map_evidence(
        &actor,
        &owner(),
        &worker(),
        "source-verifier",
        request(),
        &files,
    )
    .await
    .unwrap();
    let changed = development_mapping::verify_changed(
        &actor,
        &owner(),
        &worker(),
        "source-verifier",
        snapshot,
        &files,
        provider.clone(),
        "mapping-changed",
        3000,
    )
    .await
    .unwrap();
    assert_eq!(changed.records.len(), 1);
    assert_eq!(changed.skipped.len(), 2);
    let mut revoked = grant();
    revoked.revoked = true;
    revoked.revision = 3;
    command(
        &actor,
        "revoke-sharing",
        MemoryCommand::SetGrant { grant: revoked },
    )
    .await;
    let before = actor.stats().await.unwrap().log_events;
    assert!(
        development_mapping::publish(
            &actor,
            &owner(),
            &worker(),
            "source-verifier",
            &files,
            changed
        )
        .await
        .is_err()
    );
    assert_eq!(actor.stats().await.unwrap().log_events, before);
    actor.shutdown().await.unwrap();
    tokio::task::spawn_blocking(move || drop(provider))
        .await
        .unwrap();
}
