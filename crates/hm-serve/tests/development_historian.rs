use hm_context::{
    Authority, MessagePart, MessageRole, Scope, SourceMessage,
    development::*,
    historian::{ChunkLimits, select_chunks_with_spans},
    maintenance::Usage,
};
use hm_core::ActorId;
use hm_cortex::development_historian::HistorianProvider;
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_history::{self, RelationIngestion, SourceIngestion},
    context_memory::{self, MemoryCommand, MemoryRecord, MemoryRequest, RecordKind},
    development_admission,
    development_historian::{self, HistorianWorker},
};
use std::{collections::BTreeSet, time::Duration};
fn scope() -> Scope {
    Scope {
        owner_id: "historian-owner".into(),
        project_id: "knowledge".into(),
        workspace_id: None,
    }
}
fn worker_scope() -> Scope {
    Scope {
        owner_id: "historian-worker".into(),
        ..scope()
    }
}
fn config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.into(),
        actor: ActorId::new(42),
        user: [5; 16],
        kek: [8; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}
async fn command(actor: &ActorEngine, id: &str, command: MemoryCommand) {
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
fn capability(id: &str, summary: &str) -> WorkerCapability {
    WorkerCapability {
        id: id.into(),
        scope: scope(),
        principal: worker_scope(),
        worker_id: "history-runtime".into(),
        revision: 1,
        revoked: false,
        allowed_kinds: BTreeSet::from([DevelopmentKind::Historian]),
        source_ids: BTreeSet::from(["reading".into()]),
        record_ids: BTreeSet::from(["anchor".into()]),
        new_record_ids: BTreeSet::from([
            summary.into(),
            format!("{summary}-fact1"),
            format!("{summary}-fact2"),
            format!("{summary}-fact3"),
        ]),
        session_id: "session".into(),
        conversation: "conversation".into(),
        lease: DevelopmentLease {
            id: format!("lease-{id}"),
            attempt: 1,
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
fn request(id: &str) -> SnapshotRequest {
    SnapshotRequest {
        capability_id: id.into(),
        source_ids: BTreeSet::from(["reading".into()]),
        record_ids: BTreeSet::from(["anchor".into()]),
    }
}
#[tokio::test]
async fn actual_historian_atomic_coverage_facts_stale_cancel_and_restart() {
    let endpoint = std::env::var("HM_DEVELOPMENT_PROVIDER_ENDPOINT")
        .expect("configured actual generation endpoint");
    let model =
        std::env::var("HM_DEVELOPMENT_PROVIDER_MODEL").expect("configured actual generation model");
    let root = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(root.path())).await.unwrap();
    let content = "The deployment region is eu-central-1. The release date is 2026-10-06.";
    let mut message = SourceMessage {
        id: "reading".into(),
        ordinal: 0,
        role: MessageRole::User,
        parts: vec![MessagePart::Text {
            text: content.into(),
        }],
        occurred_at_ns: Some(1791287999123456789),
        recorded_at_ns: 1791288000123456789,
        authority: Authority::UserAsserted,
        source_digest: String::new(),
    };
    message.source_digest = message.computed_digest().unwrap();
    context_history::ingest(
        &actor,
        &scope(),
        &SourceIngestion {
            version: 1,
            scope: scope(),
            session_id: "session".into(),
            conversation: "conversation".into(),
            message: message.clone(),
            original_bytes: content.as_bytes().to_vec(),
        },
    )
    .await
    .unwrap();
    let span = context_history::replay(&actor, &scope(), "session", "conversation")
        .await
        .unwrap()
        .history
        .source_span("reading")
        .unwrap();
    let chunk = select_chunks_with_spans(
        &[message],
        &[span],
        ChunkLimits {
            max_messages: 4,
            max_bytes: 8192,
        },
    )
    .unwrap()
    .remove(0);
    let anchor = MemoryRecord::new(
        "anchor",
        RecordKind::Anchor,
        "The owner-approved anchor remains immutable.",
        1791287000000000000,
    );
    command(
        &actor,
        "create-anchor",
        MemoryCommand::Create { record: anchor },
    )
    .await;
    command(
        &actor,
        "grant-worker",
        MemoryCommand::RegisterWorker {
            capability: capability("cap", "summary"),
        },
    )
    .await;
    let provider = HistorianProvider::new(
        endpoint,
        model,
        DevelopmentKind::Historian,
        Some("summary".into()),
        vec![
            "summary-fact1".into(),
            "summary-fact2".into(),
            "summary-fact3".into(),
        ],
        Some(chunk.clone()),
    )
    .unwrap();
    let worker = HistorianWorker::new(provider, Duration::from_secs(120)).unwrap();
    command(
        &actor,
        "worker-read",
        MemoryCommand::SetGrant {
            grant: context_memory::MemoryGrant {
                principal_digest: None,
                id: "worker-read".into(),
                principal: worker_scope(),
                record_ids: BTreeSet::from(["anchor".into()]),
                categories: BTreeSet::from(["general".into()]),
                read: true,
                expires_at_ns: None,
                revoked: false,
                revision: 1,
                record_revisions: Default::default(),
            },
        },
    )
    .await;
    let receipt = development_historian::execute_historian(
        &actor,
        &scope(),
        &worker_scope(),
        "history-runtime",
        request("cap"),
        "historian-publication".into(),
        &worker,
    )
    .await
    .unwrap();
    assert!(matches!(receipt.usage,Usage::Known(tokens) if tokens>0));
    let summary = receipt.summary.unwrap();
    summary.validate(&chunk).unwrap();
    let state = context_memory::rebuild(&actor, &scope()).await.unwrap();
    assert!(state.records.contains_key("summary"));
    let facts = state
        .records
        .values()
        .filter(|record| record.kind == RecordKind::Fact)
        .collect::<Vec<_>>();
    assert!(!facts.is_empty());
    println!(
        "observed historian usage: {:?}; actual published facts: {}",
        receipt.usage,
        serde_json::to_string(&facts).unwrap()
    );
    for fact in facts {
        for provenance in &fact.provenance {
            let quoted =
                &content.as_bytes()[provenance.span_start as usize..provenance.span_end as usize];
            assert_eq!(provenance.quoted_digest, hm_context::digest_bytes(quoted));
            assert_eq!(
                provenance.source_digest,
                hm_context::digest_bytes(content.as_bytes())
            );
        }
        assert_eq!(fact.authority, Authority::DerivedInference);
        assert_eq!(fact.occurred_at_ns, Some(1791287999123456789));
        assert_eq!(fact.recorded_at_ns, 1791288000123456789);
        assert!(!fact.provenance.is_empty());
    }
    let anchor_before = state.records["anchor"].clone();
    assert_eq!(
        anchor_before.content,
        "The owner-approved anchor remains immutable."
    );
    let lsn = receipt.publication.ledger_lsn;
    assert!(receipt.publication.mutation_ids.len() >= 2);
    assert!(
        state
            .records
            .values()
            .filter(|r| r.id == "summary" || r.kind == RecordKind::Fact)
            .all(|r| r.last_lsn == lsn)
    );
    let frozen = development_admission::snapshot(
        &actor,
        &scope(),
        &worker_scope(),
        "history-runtime",
        request("cap"),
    )
    .await
    .unwrap();
    let cancelled_provider = worker.provider.clone();
    cancelled_provider.cancel();
    let cancelled = HistorianWorker::new(cancelled_provider, Duration::from_secs(120)).unwrap();
    let tail_before = actor.stats().await.unwrap().applied.last_lsn;
    assert!(
        cancelled
            .generate(frozen, "cancelled-plan".into())
            .await
            .is_err()
    );
    assert_eq!(actor.stats().await.unwrap().applied.last_lsn, tail_before);
    let mut stale_cap = capability("stale", "stale-summary");
    stale_cap.record_ids.extend(
        state
            .records
            .values()
            .filter(|r| r.kind == RecordKind::Fact)
            .map(|r| r.id.clone()),
    );
    command(
        &actor,
        "read-generated-facts",
        MemoryCommand::SetGrant {
            grant: context_memory::MemoryGrant {
                principal_digest: None,
                id: "worker-read".into(),
                principal: worker_scope(),
                record_ids: stale_cap.record_ids.clone(),
                categories: BTreeSet::new(),
                read: true,
                expires_at_ns: None,
                revoked: false,
                revision: 2,
                record_revisions: Default::default(),
            },
        },
    )
    .await;
    let selected_records = stale_cap.record_ids.clone();
    command(
        &actor,
        "stale-cap",
        MemoryCommand::RegisterWorker {
            capability: stale_cap,
        },
    )
    .await;
    let stale_request = SnapshotRequest {
        capability_id: "stale".into(),
        source_ids: BTreeSet::from(["reading".into()]),
        record_ids: selected_records,
    };
    let stale_evidence = development_admission::snapshot(
        &actor,
        &scope(),
        &worker_scope(),
        "history-runtime",
        stale_request,
    )
    .await
    .unwrap();
    let fresh_provider = HistorianProvider::new(
        worker.provider.endpoint.clone(),
        worker.provider.model.clone(),
        DevelopmentKind::Historian,
        Some("stale-summary".into()),
        vec![
            "stale-summary-fact1".into(),
            "stale-summary-fact2".into(),
            "stale-summary-fact3".into(),
        ],
        Some(chunk.clone()),
    )
    .unwrap();
    let late_worker = HistorianWorker::new(fresh_provider, Duration::from_secs(120)).unwrap();
    let late = late_worker
        .generate(stale_evidence, "late-publication".into())
        .await
        .unwrap();
    assert!(matches!(late.plan.usage,Usage::Known(tokens) if tokens>0));
    assert!(late.plan.mutations.iter().all(|mutation|!matches!(mutation,PlannedKnowledgeMutation::Create{record} if record.kind==DevelopmentRecordKind::Fact)),"categorical source facts were not deduplicated");
    context_history::relate(
        &actor,
        &scope(),
        &RelationIngestion {
            version: 1,
            scope: scope(),
            session_id: "session".into(),
            conversation: "conversation".into(),
            relation: hm_context::history::SourceRelation::Tombstone {
                id: "retire-reading".into(),
                source_id: "reading".into(),
            },
        },
    )
    .await
    .unwrap();
    let tail = actor.stats().await.unwrap().applied.last_lsn;
    assert!(
        development_admission::admit(
            &actor,
            &scope(),
            &worker_scope(),
            "history-runtime",
            late.plan.clone()
        )
        .await
        .is_err()
    );
    assert_eq!(actor.stats().await.unwrap().applied.last_lsn, tail);
    development_historian::cancel_attempt(
        &actor,
        &scope(),
        &scope(),
        "cancel-stale".into(),
        "stale".into(),
        1,
        &late_worker,
    )
    .await
    .unwrap();
    let tail = actor.stats().await.unwrap().applied.last_lsn;
    assert!(
        development_admission::admit(
            &actor,
            &scope(),
            &worker_scope(),
            "history-runtime",
            late.plan
        )
        .await
        .is_err()
    );
    assert_eq!(actor.stats().await.unwrap().applied.last_lsn, tail);
    let cap_revoked = context_memory::rebuild(&actor, &scope()).await.unwrap();
    assert!(cap_revoked.worker_capabilities["stale"].revoked);
    assert!(!cap_revoked.records.contains_key("stale-summary"));
    actor.shutdown().await.unwrap();
    let reopened = ActorEngine::open(config(root.path())).await.unwrap();
    let restored = context_memory::rebuild(&reopened, &scope()).await.unwrap();
    assert_eq!(restored.records["anchor"], anchor_before);
    assert_eq!(
        restored.records["summary"].metadata["historian_result"],
        serde_json::to_value(summary).unwrap()
    );
    assert!(restored.worker_capabilities["stale"].revoked);
    assert_eq!(
        restored.development_receipts["historian-publication"].ledger_lsn,
        lsn
    );
    reopened.shutdown().await.unwrap();
}
