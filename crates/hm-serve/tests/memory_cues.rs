use hm_context::{
    Authority, Scope, TokenBudget, digest_bytes,
    memory_cues::{CueProfile, VisualAvailability},
};
use hm_core::ActorId;
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_memory::{
        self, MemoryCommand, MemoryGrant, MemoryRecord, MemoryRequest, MemorySource, Provenance,
        RecordKind,
    },
    memory_cues::{self, CueRequest},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
fn scope(owner: &str) -> Scope {
    Scope {
        owner_id: owner.into(),
        project_id: "memory-cues".into(),
        workspace_id: None,
    }
}
fn config(path: &Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.into(),
        actor: ActorId::new(74),
        user: [31; 16],
        kek: [19; 32],
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
    .unwrap_or_else(|error| panic!("{id}: {error:?}"));
}
fn request() -> CueRequest {
    CueRequest {
        version: 1,
        scope: scope("owner"),
        cache_id: "compact".into(),
        record_ids: BTreeSet::from(["architecture".into(), "constraint".into()]),
        profile: CueProfile {
            model_id: "gpt-4o-mini".into(),
            max_cue_bytes: 30,
            request_visual: true,
        },
        budget: TokenBudget {
            context_tokens: 256,
            reserved_output_tokens: 32,
            required_tokens: 32,
        },
    }
}
fn large_budget() -> TokenBudget {
    TokenBudget {
        context_tokens: 4096,
        reserved_output_tokens: 32,
        required_tokens: 32,
    }
}
async fn grant(actor: &ActorEngine, revision: u64, revoked: bool) {
    command(
        actor,
        &format!("grant-{revision}"),
        MemoryCommand::SetGrant {
            grant: MemoryGrant {
                id: "reader".into(),
                principal: scope("reader"),
                principal_digest: None,
                record_ids: request().record_ids,
                categories: BTreeSet::new(),
                read: true,
                expires_at_ns: None,
                revoked,
                revision,
                record_revisions: BTreeMap::new(),
            },
        },
    )
    .await;
}
#[tokio::test]
async fn native_cue_cache_replays_expands_and_fences_revisions_and_grants() {
    let directory = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    let original=b"Unrelated private prefix.\nThe archive keeps immutable source bytes and revision-bound indexes.\nUnrelated suffix.";
    let start = original.iter().position(|b| *b == b'\n').unwrap() + 1;
    let end = original.len() - b"\nUnrelated suffix.".len();
    command(
        &actor,
        "source",
        MemoryCommand::Source {
            source: MemorySource {
                id: "observation".into(),
                digest: digest_bytes(original),
                content: original.to_vec(),
                locator: "observed-document".into(),
                occurred_at_ns: None,
                recorded_at_ns: 100,
                tombstoned: false,
            },
        },
    )
    .await;
    for (id, text, authority) in [
        (
            "architecture",
            format!(
                "Résumé: immutable sources. {}",
                "Detailed architecture constraint; ".repeat(20)
            ),
            Authority::UserAsserted,
        ),
        (
            "constraint",
            format!(
                "Index revisions must match source bytes. {}",
                "Retain precise provenance; ".repeat(20)
            ),
            Authority::DerivedInference,
        ),
    ] {
        let mut record = MemoryRecord::new(id, RecordKind::Note, text, 100);
        record.authority = authority;
        record.provenance = vec![Provenance {
            source_id: "observation".into(),
            source_digest: digest_bytes(original),
            span_start: start as u64,
            span_end: end as u64,
            quoted_digest: digest_bytes(&original[start..end]),
        }];
        command(
            &actor,
            &format!("record-{id}"),
            MemoryCommand::Create { record },
        )
        .await;
    }
    grant(&actor, 1, false).await;
    let first = memory_cues::assemble(&actor, &scope("owner"), &scope("reader"), request())
        .await
        .unwrap();
    assert!(!first.replayed);
    assert_eq!(first.plan.cues.len(), 2);
    assert_eq!(
        first.plan.visual,
        VisualAvailability::UnavailableTextFallback
    );
    assert_eq!(first.plan.accounting.image_tokens, 0);
    assert!(first.plan.cues.iter().all(|c| c.truncated));
    assert!(first.plan.accounting.input_record_tokens > first.plan.accounting.text_tokens);
    let exact = hm_compose::tokens::TokenCounter::for_model(
        "gpt-4o-mini",
        None,
        hm_compose::tokens::FallbackWeights::default(),
    )
    .unwrap();
    assert_eq!(
        first.plan.accounting.text_tokens,
        exact.count(&first.plan.rendered).unwrap() as u64
    );
    assert_eq!(
        first.plan.accounting.stored_bytes,
        first.plan.rendered.len() as u64
    );
    let before = actor.stats().await.unwrap().log_events;
    let replay = memory_cues::assemble(&actor, &scope("owner"), &scope("reader"), request())
        .await
        .unwrap();
    assert!(replay.replayed);
    assert_eq!(first.plan, replay.plan);
    assert_eq!(
        actor.stats().await.unwrap().log_events,
        before,
        "replay must not append another selection"
    );
    let cue = &first.plan.cues[0];
    let expanded = memory_cues::resolve_cue(
        &actor,
        &scope("owner"),
        &scope("reader"),
        "compact",
        first.plan.generation,
        &cue.id,
        large_budget(),
    )
    .await
    .unwrap();
    assert_eq!(expanded.record.authority, Authority::UserAsserted);
    assert_eq!(expanded.record.sources[0].bytes, original[start..end]);
    assert_eq!(expanded.record.sources[0].start, start as u64);
    assert!(expanded.record.content.len() > cue.text.len());
    let tiny = TokenBudget {
        context_tokens: 2,
        reserved_output_tokens: 1,
        required_tokens: 0,
    };
    assert!(
        memory_cues::resolve_cue(
            &actor,
            &scope("owner"),
            &scope("reader"),
            "compact",
            first.plan.generation,
            &cue.id,
            tiny
        )
        .await
        .is_err()
    );
    assert!(
        memory_cues::resolve_cue(
            &actor,
            &scope("owner"),
            &scope("stranger"),
            "compact",
            first.plan.generation,
            &cue.id,
            large_budget()
        )
        .await
        .is_err()
    );
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    let recovered = memory_cues::assemble(&actor, &scope("owner"), &scope("reader"), request())
        .await
        .unwrap();
    assert!(recovered.replayed);
    assert_eq!(recovered.plan, first.plan);
    let memory = context_memory::rebuild(&actor, &scope("owner"))
        .await
        .unwrap();
    let mut revised = memory.records["architecture"].clone();
    let revision = revised.revision;
    revised.revision += 1;
    revised.revision_digest.clear();
    revised.content =
        "Résumé: changed indexing architecture with immutable source observations.".into();
    command(
        &actor,
        "revise",
        MemoryCommand::Revise {
            record: revised,
            expected_revision: revision,
        },
    )
    .await;
    assert!(
        memory_cues::assemble(&actor, &scope("owner"), &scope("reader"), request())
            .await
            .is_err(),
        "edited content revokes prior grant bindings"
    );
    grant(&actor, 2, false).await;
    assert!(
        memory_cues::resolve_cue(
            &actor,
            &scope("owner"),
            &scope("reader"),
            "compact",
            first.plan.generation,
            &cue.id,
            large_budget()
        )
        .await
        .is_err()
    );
    let changed = memory_cues::assemble(&actor, &scope("owner"), &scope("reader"), request())
        .await
        .unwrap();
    assert!(changed.plan.generation > first.plan.generation);
    assert_ne!(changed.plan.snapshot_digest, first.plan.snapshot_digest);
    assert_ne!(changed.plan.cues[0].id, first.plan.cues[0].id);
    let mut limited = request();
    limited.cache_id = "limited".into();
    limited.budget = TokenBudget {
        context_tokens: 8,
        reserved_output_tokens: 2,
        required_tokens: 2,
    };
    let limited = memory_cues::assemble(&actor, &scope("owner"), &scope("reader"), limited)
        .await
        .unwrap();
    assert!(limited.plan.cues.is_empty());
    assert_eq!(limited.plan.omitted.len(), 2);
    let mut unsupported = request();
    unsupported.profile.model_id = "undeclared-tokenizer".into();
    assert!(
        memory_cues::assemble(&actor, &scope("owner"), &scope("reader"), unsupported)
            .await
            .is_err()
    );
    let mut malformed = changed.plan.clone();
    malformed.rendered.push(b'x');
    assert!(malformed.validate().is_err());
    grant(&actor, 3, true).await;
    assert!(
        memory_cues::assemble(&actor, &scope("owner"), &scope("reader"), request())
            .await
            .is_err()
    );
    assert!(
        memory_cues::resolve_cue(
            &actor,
            &scope("owner"),
            &scope("reader"),
            "compact",
            changed.plan.generation,
            &changed.plan.cues[0].id,
            large_budget()
        )
        .await
        .is_err()
    );
    let memory = context_memory::rebuild(&actor, &scope("owner"))
        .await
        .unwrap();
    assert_eq!(memory.sources["observation"].content, original);
    let mut owner_request = request();
    owner_request.cache_id = "owner".into();
    let owner = memory_cues::assemble(
        &actor,
        &scope("owner"),
        &scope("owner"),
        owner_request.clone(),
    )
    .await
    .unwrap();
    command(
        &actor,
        "forget-source",
        MemoryCommand::TombstoneSource {
            id: "observation".into(),
        },
    )
    .await;
    assert!(
        memory_cues::resolve_cue(
            &actor,
            &scope("owner"),
            &scope("owner"),
            "owner",
            owner.plan.generation,
            &owner.plan.cues[0].id,
            large_budget()
        )
        .await
        .is_err()
    );
    assert!(
        memory_cues::assemble(&actor, &scope("owner"), &scope("owner"), owner_request)
            .await
            .is_err()
    );
    let bridge_bytes = b"Native observed source.";
    let mut message = hm_context::SourceMessage {
        id: "bridged-source".into(),
        ordinal: 1,
        role: hm_context::MessageRole::User,
        parts: vec![hm_context::MessagePart::Text {
            text: String::from_utf8(bridge_bytes.to_vec()).unwrap(),
        }],
        occurred_at_ns: None,
        recorded_at_ns: 100,
        authority: Authority::UserAsserted,
        source_digest: String::new(),
    };
    message.source_digest = message.computed_digest().unwrap();
    hm_serve::context_history::ingest(
        &actor,
        &scope("owner"),
        &hm_serve::context_history::SourceIngestion {
            version: 1,
            scope: scope("owner"),
            session_id: "source-session".into(),
            conversation: "source-conversation".into(),
            message,
            original_bytes: bridge_bytes.to_vec(),
        },
    )
    .await
    .unwrap();
    command(
        &actor,
        "bridge-source",
        MemoryCommand::Source {
            source: MemorySource {
                id: "bridged-source".into(),
                digest: digest_bytes(bridge_bytes),
                content: bridge_bytes.to_vec(),
                locator: "conversation:source-session:bridged-source".into(),
                occurred_at_ns: None,
                recorded_at_ns: 100,
                tombstoned: false,
            },
        },
    )
    .await;
    let mut bridge_record = MemoryRecord::new(
        "bridged-record",
        RecordKind::Note,
        "Native observed source.",
        100,
    );
    bridge_record.provenance = vec![Provenance {
        source_id: "bridged-source".into(),
        source_digest: digest_bytes(bridge_bytes),
        span_start: 0,
        span_end: bridge_bytes.len() as u64,
        quoted_digest: digest_bytes(bridge_bytes),
    }];
    command(
        &actor,
        "bridge-record",
        MemoryCommand::Create {
            record: bridge_record,
        },
    )
    .await;
    let mut bridge_request = request();
    bridge_request.cache_id = "bridged-cache".into();
    bridge_request.record_ids = BTreeSet::from(["bridged-record".into()]);
    assert!(
        memory_cues::assemble(
            &actor,
            &scope("owner"),
            &scope("owner"),
            bridge_request.clone()
        )
        .await
        .is_err(),
        "missing original conversation binding must refuse"
    );
    command(
        &actor,
        "bridge-binding",
        MemoryCommand::RegisterWorker {
            capability: hm_context::development::WorkerCapability {
                id: "cue-source-binding".into(),
                scope: scope("owner"),
                principal: scope("cue-worker"),
                worker_id: "cue-worker".into(),
                revision: 1,
                revoked: false,
                allowed_kinds: BTreeSet::from([
                    hm_context::development::DevelopmentKind::Verification,
                ]),
                source_ids: BTreeSet::from(["bridged-source".into()]),
                record_ids: BTreeSet::from(["bridged-record".into()]),
                new_record_ids: BTreeSet::new(),
                session_id: "source-session".into(),
                conversation: "source-conversation".into(),
                lease: hm_context::development::DevelopmentLease {
                    id: "source-lease".into(),
                    attempt: 1,
                    expires_at_ns: i64::MAX,
                },
                budget: hm_context::development::DevelopmentBudget {
                    reserved_tokens: 64,
                    max_input_bytes: 4096,
                    max_output_bytes: 4096,
                    max_mutations: 1,
                },
            },
        },
    )
    .await;
    let bridge = memory_cues::assemble(
        &actor,
        &scope("owner"),
        &scope("owner"),
        bridge_request.clone(),
    )
    .await
    .unwrap();
    hm_serve::context_history::relate(
        &actor,
        &scope("owner"),
        &hm_serve::context_history::RelationIngestion {
            version: 1,
            scope: scope("owner"),
            session_id: "source-session".into(),
            conversation: "source-conversation".into(),
            relation: hm_context::history::SourceRelation::Tombstone {
                id: "forget-bridged-source".into(),
                source_id: "bridged-source".into(),
            },
        },
    )
    .await
    .unwrap();
    assert!(
        memory_cues::resolve_cue(
            &actor,
            &scope("owner"),
            &scope("owner"),
            "bridged-cache",
            bridge.plan.generation,
            &bridge.plan.cues[0].id,
            large_budget()
        )
        .await
        .is_err()
    );
    assert!(
        memory_cues::assemble(&actor, &scope("owner"), &scope("owner"), bridge_request)
            .await
            .is_err()
    );
    actor.shutdown().await.unwrap();
}
