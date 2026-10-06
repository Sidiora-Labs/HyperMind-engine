use hm_context::{Authority, Scope, development::*, digest_bytes, maintenance::Usage};
use hm_core::ActorId;
use hm_cortex::development_mapping::AuthorizedRepository;
use hm_llm::{HttpTransport, ModelTier, Pricing, ProviderConfig, ollama::Ollama};
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_memory::{
        self, MemoryCommand, MemoryGrant, MemoryRecord, MemoryRequest, MemorySource, Provenance,
        RecordKind,
    },
    development_verification,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::Arc,
    time::Duration,
};
fn scope(owner: &str) -> Scope {
    Scope {
        owner_id: owner.into(),
        project_id: "broad-verification".into(),
        workspace_id: None,
    }
}
fn config(path: &Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.into(),
        actor: ActorId::new(71),
        user: [17; 16],
        kek: [23; 32],
        projection_map_bytes: 16 * 1024 * 1024,
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
    .unwrap_or_else(|error| panic!("command {id}: {error:?}"));
}
fn cap(revision: u64) -> WorkerCapability {
    WorkerCapability {
        id: "broad-cap".into(),
        scope: scope("owner"),
        principal: scope("worker"),
        worker_id: "broad-worker".into(),
        revision,
        revoked: false,
        allowed_kinds: BTreeSet::from([DevelopmentKind::Verification]),
        source_ids: BTreeSet::from(["source".into()]),
        record_ids: BTreeSet::from(["a-gap".into(), "b-paris".into(), "c-london".into()]),
        new_record_ids: BTreeSet::new(),
        session_id: "session".into(),
        conversation: "conversation".into(),
        lease: DevelopmentLease {
            id: "broad-lease".into(),
            attempt: revision,
            expires_at_ns: i64::MAX,
        },
        budget: DevelopmentBudget {
            reserved_tokens: 8192,
            max_input_bytes: 65536,
            max_output_bytes: 65536,
            max_mutations: 1,
        },
    }
}
async fn register(actor: &ActorEngine, revision: u64) {
    command(
        actor,
        &format!("cap-{revision}"),
        MemoryCommand::RegisterWorker {
            capability: cap(revision),
        },
    )
    .await;
}
async fn batch(
    actor: &ActorEngine,
    repo: &AuthorizedRepository,
    provider: Arc<dyn hm_llm::LlmProvider>,
    id: &str,
    timeout: Duration,
) -> development_verification::CycleState {
    development_verification::run_batch(
        actor,
        &scope("owner"),
        &scope("worker"),
        "broad-worker",
        id,
        repo,
        provider,
        1,
        timeout,
        1000,
    )
    .await
    .unwrap()
}
#[tokio::test]
async fn actual_broad_cycle_timeout_cancellation_restart_and_contiguous_coverage() {
    let directory = tempfile::tempdir().unwrap();
    let repository = tempfile::tempdir().unwrap();
    let bytes = b"The observatory is in Paris, France.";
    std::fs::write(repository.path().join("observatory.txt"), bytes).unwrap();
    let mut repo = AuthorizedRepository {
        root: repository.path().into(),
        files: BTreeMap::from([("source".into(), "observatory.txt".into())]),
    };
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    command(
        &actor,
        "source",
        MemoryCommand::Source {
            source: MemorySource {
                id: "source".into(),
                digest: digest_bytes(bytes),
                content: bytes.to_vec(),
                locator: "repository:observatory.txt".into(),
                occurred_at_ns: None,
                recorded_at_ns: 100,
                tombstoned: false,
            },
        },
    )
    .await;
    for (id, claim, mapped) in [
        ("a-gap", "The observatory has a director.", false),
        ("b-paris", "The observatory is in Paris.", true),
        ("c-london", "The observatory is in London, not Paris.", true),
    ] {
        let mut record = MemoryRecord::new(id, RecordKind::Note, claim, 100);
        record.authority = Authority::DerivedInference;
        if mapped {
            record.provenance = vec![Provenance {
                source_id: "source".into(),
                source_digest: digest_bytes(bytes),
                span_start: 0,
                span_end: bytes.len() as u64,
                quoted_digest: digest_bytes(bytes),
            }];
        }
        command(
            &actor,
            &format!("record-{id}"),
            MemoryCommand::Create { record },
        )
        .await;
    }
    command(
        &actor,
        "grant",
        MemoryCommand::SetGrant {
            grant: MemoryGrant {
                id: "read".into(),
                principal: scope("worker"),
                principal_digest: None,
                record_ids: cap(1).record_ids,
                categories: BTreeSet::new(),
                read: true,
                expires_at_ns: None,
                revoked: false,
                revision: 1,
                record_revisions: BTreeMap::new(),
            },
        },
    )
    .await;
    register(&actor, 1).await;
    let provider: Arc<dyn hm_llm::LlmProvider> = tokio::task::spawn_blocking(|| {
        Arc::new(
            Ollama::new(
                ProviderConfig {
                    endpoint: std::env::var("HYPERMIND_VERIFICATION_ENDPOINT")
                        .expect("real endpoint required"),
                    api_key: None,
                    model: std::env::var("HYPERMIND_VERIFICATION_MODEL")
                        .expect("real model required"),
                    tier: ModelTier::Economy,
                    pricing: Pricing::default(),
                },
                HttpTransport::default(),
            )
            .unwrap(),
        ) as Arc<dyn hm_llm::LlmProvider>
    })
    .await
    .unwrap();
    let opened = development_verification::open_cycle(
        &actor,
        &scope("owner"),
        &scope("worker"),
        "broad-worker",
        "broad-cap",
        &repo,
        "cycle-one",
    )
    .await
    .unwrap();
    assert_eq!(opened.cycle.records.len(), 3);
    let timeout = batch(&actor, &repo, provider.clone(), "cycle-one", Duration::ZERO).await;
    assert!(timeout.last_error.is_some());
    assert_eq!(timeout.last_usage, Usage::Unknown);
    assert_eq!(timeout.cycle.contiguous, 0);
    register(&actor, 2).await;
    let first = batch(
        &actor,
        &repo,
        provider.clone(),
        "cycle-one",
        Duration::from_secs(90),
    )
    .await;
    assert!(first.last_error.is_none(), "{:?}", first.last_error);
    assert!(first.cycle.records[1].receipt.is_some());
    assert_eq!(
        first.cycle.contiguous, 0,
        "later success cannot cover earlier unmapped record"
    );
    let cancelled = development_verification::set_cancelled(
        &actor,
        &scope("owner"),
        &scope("owner"),
        "cycle-one",
        true,
    )
    .await
    .unwrap();
    assert!(cancelled.cycle.cancelled);
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    let replay =
        development_verification::inspect(&actor, &scope("owner"), &scope("worker"), "cycle-one")
            .await
            .unwrap();
    assert!(replay.cycle.cancelled);
    assert_eq!(
        replay.cycle.records[1].result,
        first.cycle.records[1].result
    );
    development_verification::set_cancelled(
        &actor,
        &scope("owner"),
        &scope("owner"),
        "cycle-one",
        false,
    )
    .await
    .unwrap();
    register(&actor, 3).await;
    let second = batch(
        &actor,
        &repo,
        provider.clone(),
        "cycle-one",
        Duration::from_secs(90),
    )
    .await;
    assert!(second.last_error.is_none(), "{:?}", second.last_error);
    assert_eq!(
        second.cycle.records[2].result.as_ref().unwrap().findings[0].state,
        DevelopmentVerificationState::Contradicted
    );
    assert_eq!(second.cycle.contiguous, 0);
    let before = context_memory::rebuild(&actor, &scope("owner"))
        .await
        .unwrap()
        .verifications
        .len();
    register(&actor, 4).await;
    development_verification::open_cycle(
        &actor,
        &scope("owner"),
        &scope("worker"),
        "broad-worker",
        "broad-cap",
        &repo,
        "cycle-two",
    )
    .await
    .unwrap();
    let broad = batch(
        &actor,
        &repo,
        provider.clone(),
        "cycle-two",
        Duration::from_secs(90),
    )
    .await;
    assert!(broad.cycle.records[1].receipt.is_some());
    assert_eq!(
        context_memory::rebuild(&actor, &scope("owner"))
            .await
            .unwrap()
            .verifications
            .len(),
        before + 1,
        "new broad cycle forces unchanged verification"
    );
    let changed_bytes = b"The observatory is in London, England.";
    std::fs::write(repository.path().join("new-observation.txt"), changed_bytes).unwrap();
    command(
        &actor,
        "new-observation",
        MemoryCommand::Source {
            source: MemorySource {
                id: "source-two".into(),
                digest: digest_bytes(changed_bytes),
                content: changed_bytes.to_vec(),
                locator: "repository:new-observation.txt".into(),
                occurred_at_ns: None,
                recorded_at_ns: 200,
                tombstoned: false,
            },
        },
    )
    .await;
    let memory = context_memory::rebuild(&actor, &scope("owner"))
        .await
        .unwrap();
    let mut changed = memory.records["b-paris"].clone();
    changed.provenance.push(Provenance {
        source_id: "source-two".into(),
        source_digest: digest_bytes(changed_bytes),
        span_start: 0,
        span_end: changed_bytes.len() as u64,
        quoted_digest: digest_bytes(changed_bytes),
    });
    let old_revision = changed.revision;
    changed.revision += 1;
    changed.revision_digest.clear();
    command(
        &actor,
        "revise-evidence",
        MemoryCommand::Revise {
            expected_revision: old_revision,
            record: changed,
        },
    )
    .await;
    command(
        &actor,
        "renew-grant",
        MemoryCommand::SetGrant {
            grant: MemoryGrant {
                id: "read".into(),
                principal: scope("worker"),
                principal_digest: None,
                record_ids: cap(5).record_ids,
                categories: BTreeSet::new(),
                read: true,
                expires_at_ns: None,
                revoked: false,
                revision: 2,
                record_revisions: BTreeMap::new(),
            },
        },
    )
    .await;
    let mut renewed = cap(5);
    renewed.source_ids.insert("source-two".into());
    command(
        &actor,
        "cap-5",
        MemoryCommand::RegisterWorker {
            capability: renewed,
        },
    )
    .await;
    repo.files
        .insert("source-two".into(), "new-observation.txt".into());
    let recheck = batch(
        &actor,
        &repo,
        provider.clone(),
        "cycle-two",
        Duration::from_secs(90),
    )
    .await;
    assert!(recheck.last_error.is_none(), "{:?}", recheck.last_error);
    let findings = &recheck.cycle.records[1].result.as_ref().unwrap().findings;
    assert_eq!(findings.len(), 2);
    assert!(
        findings
            .iter()
            .any(|f| f.state == DevelopmentVerificationState::Supported)
    );
    assert!(
        findings
            .iter()
            .any(|f| f.state == DevelopmentVerificationState::Contradicted)
    );
    assert_ne!(
        recheck.cycle.records[1].fingerprint,
        broad.cycle.records[1].fingerprint
    );
    assert_eq!(recheck.cycle.contiguous, 0);
    let memory = context_memory::rebuild(&actor, &scope("owner"))
        .await
        .unwrap();
    let mut gap = memory.records["a-gap"].clone();
    let old_revision = gap.revision;
    gap.revision += 1;
    gap.revision_digest.clear();
    gap.provenance = vec![Provenance {
        source_id: "source".into(),
        source_digest: digest_bytes(bytes),
        span_start: 0,
        span_end: bytes.len() as u64,
        quoted_digest: digest_bytes(bytes),
    }];
    command(
        &actor,
        "map-gap",
        MemoryCommand::Revise {
            record: gap,
            expected_revision: old_revision,
        },
    )
    .await;
    command(
        &actor,
        "renew-grant-three",
        MemoryCommand::SetGrant {
            grant: MemoryGrant {
                id: "read".into(),
                principal: scope("worker"),
                principal_digest: None,
                record_ids: cap(6).record_ids,
                categories: BTreeSet::new(),
                read: true,
                expires_at_ns: None,
                revoked: false,
                revision: 3,
                record_revisions: BTreeMap::new(),
            },
        },
    )
    .await;
    for revision in [6, 7] {
        let mut renewed = cap(revision);
        renewed.source_ids.insert("source-two".into());
        command(
            &actor,
            &format!("cap-{revision}"),
            MemoryCommand::RegisterWorker {
                capability: renewed,
            },
        )
        .await;
        let completed = batch(
            &actor,
            &repo,
            provider.clone(),
            "cycle-two",
            Duration::from_secs(90),
        )
        .await;
        assert!(completed.last_error.is_none(), "{:?}", completed.last_error);
        assert_eq!(
            completed.cycle.contiguous,
            if revision == 6 { 2 } else { 3 }
        );
    }
    let mut malformed = broad.cycle.clone();
    malformed.contiguous = 3;
    assert!(malformed.validate().is_err());
    println!(
        "real broad verification receipts={}, contiguous={}, forced_unchanged=true, timeout_usage=unknown",
        before + 1,
        broad.cycle.contiguous
    );
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    let finished =
        development_verification::inspect(&actor, &scope("owner"), &scope("worker"), "cycle-two")
            .await
            .unwrap();
    assert_eq!(finished.cycle.contiguous, 3);
    assert_eq!(
        finished.cycle.records[1]
            .result
            .as_ref()
            .unwrap()
            .findings
            .len(),
        2
    );
    actor.shutdown().await.unwrap();
    tokio::task::spawn_blocking(move || drop(provider))
        .await
        .unwrap();
}
