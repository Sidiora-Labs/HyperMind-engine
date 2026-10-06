use hm_context::{Authority, Scope, development::*, digest_bytes, maintenance::Usage};
use hm_core::{ActorId, ConversationId};
use hm_ledger::frame::EventKind;
use hm_schema::{
    event::{CURRENT_SCHEMA_VERSION, encode_event_envelope},
    events::{EventEnvelope, EventPayload, Retention, Sensitivity, UserMsg},
};
use hm_serve::{
    actor::{ActorConfig, ActorEngine, IncomingEvent},
    context_memory::{
        self, MemoryCommand, MemoryGrant, MemoryRecord, MemoryRequest, MemorySource, Provenance,
        RecordKind, RecordStatus,
    },
    development_admission,
};
use std::collections::{BTreeMap, BTreeSet};
fn owner() -> Scope {
    Scope {
        owner_id: "owner".into(),
        project_id: "development".into(),
        workspace_id: None,
    }
}
fn worker() -> Scope {
    Scope {
        owner_id: "worker".into(),
        project_id: "development".into(),
        workspace_id: None,
    }
}
fn config(path: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.into(),
        actor: ActorId::new(31),
        user: [4; 16],
        kek: [9; 32],
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
fn capability(id: &str, kind: DevelopmentKind) -> WorkerCapability {
    WorkerCapability {
        id: id.into(),
        scope: owner(),
        principal: worker(),
        worker_id: "worker-runtime".into(),
        revision: 1,
        revoked: false,
        allowed_kinds: BTreeSet::from([kind]),
        source_ids: BTreeSet::from(["evidence".into()]),
        record_ids: BTreeSet::from(["existing".into()]),
        new_record_ids: BTreeSet::from(["new-one".into(), "new-two".into()]),
        session_id: "session".into(),
        conversation: "conversation".into(),
        lease: DevelopmentLease {
            id: format!("lease-{id}"),
            attempt: 1,
            expires_at_ns: i64::MAX,
        },
        budget: DevelopmentBudget {
            reserved_tokens: 1000,
            max_input_bytes: 65536,
            max_output_bytes: 65536,
            max_mutations: 8,
        },
    }
}
fn record(id: &str, content: &str) -> MemoryRecord {
    let mut record = MemoryRecord::new(id, RecordKind::Note, content, 100);
    record.provenance = vec![Provenance {
        source_id: "evidence".into(),
        source_digest: digest_bytes(b"source evidence"),
        span_start: 0,
        span_end: 15,
        quoted_digest: digest_bytes(b"source evidence"),
    }];
    record
}
fn generated(id: &str) -> DevelopmentRecord {
    let mut record = record(id, "source evidence");
    record.authority = Authority::DerivedInference;
    serde_json::from_value(serde_json::to_value(record).unwrap()).unwrap()
}
async fn setup(actor: &ActorEngine, kind: DevelopmentKind) {
    command(
        actor,
        "source",
        MemoryCommand::Source {
            source: MemorySource {
                id: "evidence".into(),
                digest: digest_bytes(b"source evidence"),
                content: b"source evidence".to_vec(),
                locator: "local:source".into(),
                occurred_at_ns: None,
                recorded_at_ns: 100,
                tombstoned: false,
            },
        },
    )
    .await;
    command(
        actor,
        "existing",
        MemoryCommand::Create {
            record: record("existing", "initial evidence record"),
        },
    )
    .await;
    command(
        actor,
        "grant",
        MemoryCommand::SetGrant {
            grant: MemoryGrant {
                principal_digest: None,
                id: "read-grant".into(),
                principal: worker(),
                record_ids: BTreeSet::from(["existing".into()]),
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
    command(
        actor,
        "capability",
        MemoryCommand::RegisterWorker {
            capability: capability("cap", kind),
        },
    )
    .await;
}
async fn snapshot(actor: &ActorEngine, cap: &str, records: bool) -> EvidenceSnapshot {
    development_admission::snapshot(
        actor,
        &owner(),
        &worker(),
        "worker-runtime",
        SnapshotRequest {
            capability_id: cap.into(),
            source_ids: BTreeSet::from(["evidence".into()]),
            record_ids: if records {
                BTreeSet::from(["existing".into()])
            } else {
                BTreeSet::new()
            },
        },
    )
    .await
    .unwrap()
}
fn plan(
    id: &str,
    kind: DevelopmentKind,
    evidence: EvidenceSnapshot,
    mutations: Vec<PlannedKnowledgeMutation>,
) -> DevelopmentPlan {
    DevelopmentPlan {
        version: 1,
        id: id.into(),
        kind,
        evidence,
        mutations,
        proposal: None,
        usage: Usage::Known(0),
    }
}
async fn native_append(actor: &ActorEngine) {
    actor
        .append(vec![IncomingEvent {
            kind: EventKind::UserMsg,
            conversation: ConversationId::derive("unrelated"),
            payload: encode_event_envelope(&EventEnvelope {
                schema_version: CURRENT_SCHEMA_VERSION,
                payload: EventPayload::UserMsg(Box::new(UserMsg {
                    content: b"foreground work".to_vec(),
                })),
                connection_id: None,
                client_seq: 0,
                client_event_index: 0,
                client_event_count: 0,
                origin_actor: 0,
                run_id: None,
                model_provenance: None,
                authority: hm_schema::events::Authority::UserAsserted,
                retention: Retention::Durable,
                sensitivity: Sensitivity::Personal,
                event_time_ns: 0,
            }),
        }])
        .await
        .unwrap();
}
#[tokio::test]
async fn invalid_batch_publishes_nothing_and_valid_batch_survives_unrelated_tail_and_restart() {
    let dir = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(dir.path())).await.unwrap();
    setup(&actor, DevelopmentKind::Curation).await;
    let evidence = snapshot(&actor, "cap", false).await;
    let mut invalid = generated("new-two");
    invalid.confidence = 1_000_001;
    let invalid_plan = plan(
        "invalid",
        DevelopmentKind::Curation,
        evidence.clone(),
        vec![
            PlannedKnowledgeMutation::Create {
                record: generated("new-one"),
            },
            PlannedKnowledgeMutation::Create { record: invalid },
        ],
    );
    let before = actor.stats().await.unwrap();
    assert!(
        development_admission::admit(&actor, &owner(), &worker(), "worker-runtime", invalid_plan)
            .await
            .is_err()
    );
    assert_eq!(actor.stats().await.unwrap(), before);
    assert!(
        !context_memory::rebuild(&actor, &owner())
            .await
            .unwrap()
            .records
            .contains_key("new-one")
    );
    native_append(&actor).await;
    let valid = plan(
        "valid",
        DevelopmentKind::Curation,
        evidence,
        vec![
            PlannedKnowledgeMutation::Create {
                record: generated("new-one"),
            },
            PlannedKnowledgeMutation::Create {
                record: generated("new-two"),
            },
        ],
    );
    let before = actor.stats().await.unwrap().log_events;
    let receipt =
        development_admission::admit(&actor, &owner(), &worker(), "worker-runtime", valid.clone())
            .await
            .unwrap();
    assert_eq!(actor.stats().await.unwrap().log_events, before + 1);
    let state = context_memory::rebuild(&actor, &owner()).await.unwrap();
    assert_eq!(
        state.records["new-one"].last_lsn,
        state.records["new-two"].last_lsn
    );
    assert_eq!(
        state.records["new-one"].authority,
        Authority::DerivedInference
    );
    assert!(
        development_admission::admit(&actor, &owner(), &worker(), "worker-runtime", valid.clone())
            .await
            .unwrap()
            .replayed
    );
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(dir.path())).await.unwrap();
    assert_eq!(
        context_memory::rebuild(&actor, &owner())
            .await
            .unwrap()
            .development_receipts["valid"],
        receipt
    );
    assert!(
        development_admission::admit(&actor, &owner(), &worker(), "worker-runtime", valid)
            .await
            .unwrap()
            .replayed
    );
    actor.shutdown().await.unwrap();
}
#[tokio::test]
async fn concurrent_record_revisions_have_one_winner_and_stale_plan_has_no_partial_writes() {
    let dir = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(dir.path())).await.unwrap();
    setup(&actor, DevelopmentKind::Curation).await;
    command(
        &actor,
        "second-cap",
        MemoryCommand::RegisterWorker {
            capability: capability("cap-two", DevelopmentKind::Curation),
        },
    )
    .await;
    let first = snapshot(&actor, "cap", true).await;
    let second = snapshot(&actor, "cap-two", true).await;
    let revision = |evidence: &EvidenceSnapshot, text: &str| {
        let mut record = evidence.records[0].clone();
        let expected_revision = record.revision;
        let expected_digest = record.revision_digest.clone();
        record.revision += 1;
        record.revision_digest.clear();
        record.content = text.into();
        record.authority = Authority::DerivedInference;
        PlannedKnowledgeMutation::Revise {
            record,
            expected_revision,
            expected_digest,
        }
    };
    let p1 = plan(
        "first",
        DevelopmentKind::Curation,
        first.clone(),
        vec![
            PlannedKnowledgeMutation::Create {
                record: generated("new-one"),
            },
            revision(&first, "first"),
        ],
    );
    let p2 = plan(
        "second",
        DevelopmentKind::Curation,
        second.clone(),
        vec![
            PlannedKnowledgeMutation::Create {
                record: generated("new-two"),
            },
            revision(&second, "second"),
        ],
    );
    let before = actor.stats().await.unwrap().log_events;
    let owner_scope = owner();
    let worker_scope = worker();
    let (one, two) = tokio::join!(
        development_admission::admit(&actor, &owner_scope, &worker_scope, "worker-runtime", p1),
        development_admission::admit(&actor, &owner_scope, &worker_scope, "worker-runtime", p2)
    );
    assert_ne!(one.is_ok(), two.is_ok());
    assert_eq!(actor.stats().await.unwrap().log_events, before + 1);
    let state = context_memory::rebuild(&actor, &owner()).await.unwrap();
    assert_ne!(
        state.records.contains_key("new-one"),
        state.records.contains_key("new-two")
    );
    assert_eq!(state.records["existing"].revision, 2);
    actor.shutdown().await.unwrap();
}
#[tokio::test]
async fn capabilities_authority_sources_and_protected_records_are_enforced() {
    let dir = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(dir.path())).await.unwrap();
    setup(&actor, DevelopmentKind::Curation).await;
    let registration = MemoryRequest {
        version: 1,
        scope: owner(),
        request_id: "self-grant".into(),
        command: MemoryCommand::RegisterWorker {
            capability: capability("self", DevelopmentKind::Curation),
        },
    };
    assert!(
        context_memory::execute(&actor, &owner(), &worker(), registration)
            .await
            .is_err()
    );
    let evidence = snapshot(&actor, "cap", false).await;
    let mut privileged = generated("new-one");
    privileged.authority = Authority::RuntimeFact;
    assert!(
        development_admission::admit(
            &actor,
            &owner(),
            &worker(),
            "worker-runtime",
            plan(
                "privileged",
                DevelopmentKind::Curation,
                evidence.clone(),
                vec![PlannedKnowledgeMutation::Create { record: privileged }]
            )
        )
        .await
        .is_err()
    );
    assert!(
        development_admission::admit(
            &actor,
            &owner(),
            &worker(),
            "wrong-runtime",
            plan(
                "wrong",
                DevelopmentKind::Curation,
                evidence.clone(),
                vec![PlannedKnowledgeMutation::Create {
                    record: generated("new-one")
                }]
            )
        )
        .await
        .is_err()
    );
    let old = context_memory::rebuild(&actor, &owner())
        .await
        .unwrap()
        .records["existing"]
        .clone();
    let mut pinned = old.clone();
    pinned.revision += 1;
    pinned.revision_digest.clear();
    pinned.pinned = true;
    command(
        &actor,
        "pin",
        MemoryCommand::Revise {
            record: pinned,
            expected_revision: old.revision,
        },
    )
    .await;
    command(
        &actor,
        "refresh-read-grant",
        MemoryCommand::SetGrant {
            grant: MemoryGrant {
                principal_digest: None,
                id: "read-grant".into(),
                principal: worker(),
                record_ids: BTreeSet::from(["existing".into()]),
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
    let protected = snapshot(&actor, "cap", true).await;
    let r = &protected.records[0];
    assert!(
        development_admission::admit(
            &actor,
            &owner(),
            &worker(),
            "worker-runtime",
            plan(
                "archive-pin",
                DevelopmentKind::Curation,
                protected.clone(),
                vec![PlannedKnowledgeMutation::SetStatus {
                    id: r.id.clone(),
                    status: DevelopmentRecordStatus::Archived,
                    expected_revision: r.revision,
                    expected_digest: r.revision_digest.clone()
                }]
            )
        )
        .await
        .is_err()
    );
    command(
        &actor,
        "source-tombstone",
        MemoryCommand::TombstoneSource {
            id: "evidence".into(),
        },
    )
    .await;
    let before = actor.stats().await.unwrap();
    assert!(
        development_admission::admit(
            &actor,
            &owner(),
            &worker(),
            "worker-runtime",
            plan(
                "stale-source",
                DevelopmentKind::Curation,
                evidence,
                vec![PlannedKnowledgeMutation::Create {
                    record: generated("new-one")
                }]
            )
        )
        .await
        .is_err()
    );
    assert_eq!(actor.stats().await.unwrap(), before);
    actor.shutdown().await.unwrap();
}
#[tokio::test]
async fn proposal_is_owner_only_revision_bound_and_restart_durable() {
    let dir = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(dir.path())).await.unwrap();
    setup(&actor, DevelopmentKind::ProfileProposal).await;
    let evidence = snapshot(&actor, "cap", false).await;
    let proposal = ApprovalProposal {
        id: "profile-proposal".into(),
        revision: 1,
        kind: DevelopmentKind::ProfileProposal,
        scope: owner(),
        worker_id: "worker-runtime".into(),
        evidence: evidence.clone(),
        mutations: vec![PlannedKnowledgeMutation::Create {
            record: generated("new-one"),
        }],
        status: ProposalStatus::Pending,
        digest: String::new(),
    };
    let mut proposed = plan(
        "proposal-plan",
        DevelopmentKind::ProfileProposal,
        evidence,
        vec![],
    );
    proposed.proposal = Some(proposal);
    development_admission::admit(&actor, &owner(), &worker(), "worker-runtime", proposed)
        .await
        .unwrap();
    assert!(
        !context_memory::rebuild(&actor, &owner())
            .await
            .unwrap()
            .records
            .contains_key("new-one")
    );
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(dir.path())).await.unwrap();
    let pending = context_memory::rebuild(&actor, &owner())
        .await
        .unwrap()
        .development_proposals["profile-proposal"]
        .clone();
    let decision = ProposalDecision {
        request_id: "approve".into(),
        proposal_id: pending.id,
        expected_revision: pending.revision,
        expected_digest: pending.digest,
        decision: ProposalDecisionKind::Accept,
    };
    assert!(
        development_admission::decide(&actor, &owner(), &worker(), decision.clone())
            .await
            .is_err()
    );
    let mut wrong = decision.clone();
    wrong.expected_revision += 1;
    assert!(
        development_admission::decide(&actor, &owner(), &owner(), wrong)
            .await
            .is_err()
    );
    let before = actor.stats().await.unwrap().log_events;
    development_admission::decide(&actor, &owner(), &owner(), decision.clone())
        .await
        .unwrap();
    assert_eq!(actor.stats().await.unwrap().log_events, before + 1);
    let state = context_memory::rebuild(&actor, &owner()).await.unwrap();
    assert!(state.records.contains_key("new-one"));
    assert_eq!(
        state.development_proposals["profile-proposal"].status,
        ProposalStatus::Accepted
    );
    assert!(
        development_admission::decide(&actor, &owner(), &owner(), decision)
            .await
            .unwrap()
            .replayed
    );
    command(
        &actor,
        "reject-capability",
        MemoryCommand::RegisterWorker {
            capability: capability("reject-cap", DevelopmentKind::ProfileProposal),
        },
    )
    .await;
    let evidence = snapshot(&actor, "reject-cap", false).await;
    let proposal = ApprovalProposal {
        id: "rejected-proposal".into(),
        revision: 1,
        kind: DevelopmentKind::ProfileProposal,
        scope: owner(),
        worker_id: "worker-runtime".into(),
        evidence: evidence.clone(),
        mutations: vec![PlannedKnowledgeMutation::Create {
            record: generated("new-two"),
        }],
        status: ProposalStatus::Pending,
        digest: String::new(),
    };
    let mut plan = plan(
        "reject-plan",
        DevelopmentKind::ProfileProposal,
        evidence,
        vec![],
    );
    plan.proposal = Some(proposal);
    development_admission::admit(&actor, &owner(), &worker(), "worker-runtime", plan)
        .await
        .unwrap();
    let state = context_memory::rebuild(&actor, &owner()).await.unwrap();
    let proposal = &state.development_proposals["rejected-proposal"];
    development_admission::decide(
        &actor,
        &owner(),
        &owner(),
        ProposalDecision {
            request_id: "reject".into(),
            proposal_id: proposal.id.clone(),
            expected_revision: proposal.revision,
            expected_digest: proposal.digest.clone(),
            decision: ProposalDecisionKind::Reject,
        },
    )
    .await
    .unwrap();
    let state = context_memory::rebuild(&actor, &owner()).await.unwrap();
    assert_eq!(
        state.development_proposals["rejected-proposal"].status,
        ProposalStatus::Rejected
    );
    assert!(!state.records.contains_key("new-two"));
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn original_conversation_evidence_is_bridged_with_generated_record_in_one_ledger_publication()
{
    let dir = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(dir.path())).await.unwrap();
    native_append(&actor).await;
    let source_id = format!(
        "lsn:{}",
        actor.stats().await.unwrap().applied.last_lsn.get()
    );
    let mut cap = capability("conversation-cap", DevelopmentKind::Extraction);
    cap.conversation = "unrelated".into();
    cap.source_ids = BTreeSet::from([source_id.clone()]);
    command(
        &actor,
        "conversation-capability",
        MemoryCommand::RegisterWorker { capability: cap },
    )
    .await;
    let evidence = development_admission::snapshot(
        &actor,
        &owner(),
        &worker(),
        "worker-runtime",
        SnapshotRequest {
            capability_id: "conversation-cap".into(),
            source_ids: BTreeSet::from([source_id.clone()]),
            record_ids: BTreeSet::new(),
        },
    )
    .await
    .unwrap();
    assert_eq!(evidence.sources[0].origin, EvidenceOrigin::Conversation);
    let source = &evidence.sources[0];
    let mut record = generated("new-one");
    record.provenance = vec![DevelopmentProvenance {
        source_id: source_id.clone(),
        source_digest: source.content_digest.clone(),
        span_start: 0,
        span_end: source.content.len() as u64,
        quoted_digest: source.content_digest.clone(),
    }];
    let expected_bytes = source.content.clone();
    let before = actor.stats().await.unwrap().log_events;
    development_admission::admit(
        &actor,
        &owner(),
        &worker(),
        "worker-runtime",
        plan(
            "extract-conversation",
            DevelopmentKind::Extraction,
            evidence,
            vec![PlannedKnowledgeMutation::Create { record }],
        ),
    )
    .await
    .unwrap();
    assert_eq!(actor.stats().await.unwrap().log_events, before + 1);
    let state = context_memory::rebuild(&actor, &owner()).await.unwrap();
    assert_eq!(state.sources[&source_id].content, expected_bytes);
    assert_eq!(
        state.records["new-one"].authority,
        Authority::DerivedInference
    );
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn verification_metadata_preserves_pinned_original_and_rejects_changed_source_quotes() {
    let dir = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(dir.path())).await.unwrap();
    setup(&actor, DevelopmentKind::Verification).await;
    let old = context_memory::rebuild(&actor, &owner())
        .await
        .unwrap()
        .records["existing"]
        .clone();
    let mut pinned = old.clone();
    pinned.pinned = true;
    pinned.content = "source evidence".into();
    pinned.revision += 1;
    pinned.revision_digest.clear();
    command(
        &actor,
        "pin-verification",
        MemoryCommand::Revise {
            record: pinned,
            expected_revision: old.revision,
        },
    )
    .await;
    command(
        &actor,
        "grant-verification",
        MemoryCommand::SetGrant {
            grant: MemoryGrant {
                principal_digest: None,
                id: "read-grant".into(),
                principal: worker(),
                record_ids: BTreeSet::from(["existing".into()]),
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
    let original = context_memory::rebuild(&actor, &owner())
        .await
        .unwrap()
        .records["existing"]
        .clone();
    let evidence = snapshot(&actor, "cap", true).await;
    let source = &evidence.sources[0];
    let metadata = serde_json::json!({
        "record_id":"existing", "fingerprint":digest_bytes(original.content.as_bytes()),
        "findings":[{"state":"supported","confidence":900000,"span":{
            "source_id":source.id,"source_revision":source.revision,"source_digest":source.digest,
            "content_digest":source.content_digest,"path":null,"start":0,"end":source.content.len(),"quoted_digest":digest_bytes(&source.content)
        }}]
    });
    let verification = DevelopmentVerification {
        id: "metadata-verification".into(),
        record_id: "existing".into(),
        revision_digest: original.revision_digest.clone(),
        state: DevelopmentVerificationState::Supported,
        evidence_source_id: Some(source.id.clone()),
        confidence: 900000,
        created_at_ns: 100,
        metadata: metadata.clone(),
    };
    let mut bad = verification.clone();
    bad.metadata["findings"][0]["span"]["quoted_digest"] =
        serde_json::json!(digest_bytes(b"changed quote"));
    let before = actor.stats().await.unwrap();
    assert!(
        development_admission::admit(
            &actor,
            &owner(),
            &worker(),
            "worker-runtime",
            plan(
                "bad-metadata",
                DevelopmentKind::Verification,
                evidence.clone(),
                vec![PlannedKnowledgeMutation::Verify { verification: bad }]
            )
        )
        .await
        .is_err()
    );
    assert_eq!(actor.stats().await.unwrap(), before);
    let mut oversized = verification.clone();
    oversized.metadata["extra"] = serde_json::json!("x".repeat(65536));
    assert!(
        development_admission::admit(
            &actor,
            &owner(),
            &worker(),
            "worker-runtime",
            plan(
                "oversized-metadata",
                DevelopmentKind::Verification,
                evidence.clone(),
                vec![PlannedKnowledgeMutation::Verify {
                    verification: oversized
                }]
            )
        )
        .await
        .is_err()
    );
    development_admission::admit(
        &actor,
        &owner(),
        &worker(),
        "worker-runtime",
        plan(
            "valid-metadata",
            DevelopmentKind::Verification,
            evidence,
            vec![PlannedKnowledgeMutation::Verify { verification }],
        ),
    )
    .await
    .unwrap();
    let state = context_memory::rebuild(&actor, &owner()).await.unwrap();
    assert_eq!(state.records["existing"], original);
    assert_eq!(
        state.verifications["metadata-verification"].metadata,
        metadata
    );
    let mut legacy = serde_json::to_value(&state.verifications["metadata-verification"]).unwrap();
    legacy.as_object_mut().unwrap().remove("metadata");
    let legacy: context_memory::Verification = serde_json::from_value(legacy).unwrap();
    assert!(legacy.metadata.is_null());
    assert!(
        serde_json::to_value(legacy)
            .unwrap()
            .get("metadata")
            .is_none()
    );
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(dir.path())).await.unwrap();
    let state = context_memory::rebuild(&actor, &owner()).await.unwrap();
    assert_eq!(state.records["existing"], original);
    assert_eq!(
        state.verifications["metadata-verification"].metadata,
        metadata
    );
    actor.shutdown().await.unwrap();
}

#[tokio::test]
async fn archived_initial_creation_is_one_atomic_publication_with_revision_two_and_native_refusals()
{
    let dir = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(dir.path())).await.unwrap();
    setup(&actor, DevelopmentKind::Retrospective).await;
    let evidence = snapshot(&actor, "cap", false).await;
    let mut archived = generated("new-one");
    archived.status = DevelopmentRecordStatus::Archived;
    let native: MemoryRecord =
        serde_json::from_value(serde_json::to_value(&archived).unwrap()).unwrap();
    archived.revision_digest = native.computed_revision_digest().unwrap();
    let before = actor.stats().await.unwrap();
    assert!(
        context_memory::execute(
            &actor,
            &owner(),
            &owner(),
            MemoryRequest {
                version: 1,
                scope: owner(),
                request_id: "direct-archived-create".into(),
                command: MemoryCommand::Create { record: native }
            }
        )
        .await
        .is_err()
    );
    assert_eq!(actor.stats().await.unwrap(), before);
    let mut pinned = archived.clone();
    pinned.pinned = true;
    pinned.revision_digest.clear();
    let mut anchor = archived.clone();
    anchor.kind = DevelopmentRecordKind::Anchor;
    anchor.revision_digest.clear();
    let mut ungrounded = archived.clone();
    ungrounded.provenance.clear();
    ungrounded.revision_digest.clear();
    let mut wrong_revision = archived.clone();
    wrong_revision.revision = 3;
    wrong_revision.revision_digest.clear();
    for (id, record) in [
        ("pin-archive", pinned),
        ("anchor-archive", anchor),
        ("ungrounded-archive", ungrounded),
        ("wrong-revision-archive", wrong_revision),
    ] {
        assert!(
            development_admission::admit(
                &actor,
                &owner(),
                &worker(),
                "worker-runtime",
                plan(
                    id,
                    DevelopmentKind::Retrospective,
                    evidence.clone(),
                    vec![PlannedKnowledgeMutation::Create { record }]
                )
            )
            .await
            .is_err()
        );
        assert_eq!(actor.stats().await.unwrap(), before);
    }
    let mut invalid = generated("new-two");
    invalid.confidence = 1_000_001;
    assert!(
        development_admission::admit(
            &actor,
            &owner(),
            &worker(),
            "worker-runtime",
            plan(
                "archive-rollback",
                DevelopmentKind::Retrospective,
                evidence.clone(),
                vec![
                    PlannedKnowledgeMutation::Create {
                        record: archived.clone()
                    },
                    PlannedKnowledgeMutation::Create { record: invalid }
                ]
            )
        )
        .await
        .is_err()
    );
    assert_eq!(actor.stats().await.unwrap(), before);
    assert!(
        !context_memory::rebuild(&actor, &owner())
            .await
            .unwrap()
            .records
            .contains_key("new-one")
    );
    let mut subscription = actor.subscribe();
    let receipt = development_admission::admit(
        &actor,
        &owner(),
        &worker(),
        "worker-runtime",
        plan(
            "archived-initial",
            DevelopmentKind::Retrospective,
            evidence,
            vec![PlannedKnowledgeMutation::Create { record: archived }],
        ),
    )
    .await
    .unwrap();
    assert_eq!(
        actor.stats().await.unwrap().log_events,
        before.log_events + 1
    );
    assert_eq!(
        subscription.try_recv().unwrap().header.lsn.get(),
        receipt.ledger_lsn
    );
    assert!(matches!(
        subscription.try_recv(),
        Err(tokio::sync::broadcast::error::TryRecvError::Empty)
    ));
    let state = context_memory::rebuild(&actor, &owner()).await.unwrap();
    let record = state.records["new-one"].clone();
    assert_eq!(record.status, context_memory::RecordStatus::Archived);
    assert_eq!(record.revision, 2);
    assert_eq!(
        record.revision_digest,
        record.computed_revision_digest().unwrap()
    );
    assert_eq!(record.authority, Authority::DerivedInference);
    assert_eq!(
        state.revisions[&("new-one".into(), 1)].last_lsn,
        record.last_lsn
    );
    assert!(
        state
            .visible_records(&owner(), 100)
            .unwrap()
            .iter()
            .all(|record| record.id != "new-one")
    );
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(dir.path())).await.unwrap();
    assert_eq!(
        context_memory::rebuild(&actor, &owner())
            .await
            .unwrap()
            .records["new-one"],
        record
    );
    actor.shutdown().await.unwrap();
}
