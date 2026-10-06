use hm_context::{
    historian::{
        ChunkLimits, HistorianResult, SummaryTier as HistorianTier, select_chunks_with_spans,
    },
    history::SourceRelation,
    maintenance::Usage,
    reduction::{ReductionItem, SummaryTier},
    source_continuity::*,
    types::*,
};
use hm_core::ActorId;
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_history::{self, ForkRequest, RelationIngestion, SourceIngestion},
    context_projection::{self, ProjectionRequest, SummaryLevel},
};
fn scope() -> Scope {
    Scope {
        owner_id: "owner".into(),
        project_id: "continuity".into(),
        workspace_id: None,
    }
}
fn config(root: &std::path::Path) -> ActorConfig {
    ActorConfig {
        actor_directory: root.into(),
        actor: ActorId::new(39),
        user: [8; 16],
        kek: [5; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}
fn source(id: &str, ordinal: u64, text: &str) -> SourceMessage {
    let mut message = SourceMessage {
        id: id.into(),
        ordinal,
        role: MessageRole::User,
        parts: vec![MessagePart::Text { text: text.into() }],
        occurred_at_ns: Some(1791288000123456780 + ordinal as i64),
        recorded_at_ns: 1791288000123456790 + ordinal as i64,
        authority: Authority::UserAsserted,
        source_digest: String::new(),
    };
    message.source_digest = message.computed_digest().unwrap();
    message
}
async fn ingest(
    actor: &ActorEngine,
    session: &str,
    conversation: &str,
    id: &str,
    ordinal: u64,
    text: &str,
) -> Vec<u8> {
    let mut bytes =
        serde_json::to_vec(&serde_json::json!({"id":id,"text":text,"sequence":ordinal})).unwrap();
    bytes.extend_from_slice(b"\r\n");
    context_history::ingest(
        actor,
        &scope(),
        &SourceIngestion {
            version: 1,
            scope: scope(),
            session_id: session.into(),
            conversation: conversation.into(),
            message: source(id, ordinal, text),
            original_bytes: bytes.clone(),
        },
    )
    .await
    .unwrap();
    bytes
}
async fn history(
    actor: &ActorEngine,
    session: &str,
    conversation: &str,
) -> hm_context::history::SourceHistory {
    context_history::replay(actor, &scope(), session, conversation)
        .await
        .unwrap()
        .history
}
async fn tail(actor: &ActorEngine) -> hm_core::LSN {
    actor.stats().await.unwrap().applied.last_lsn
}
fn limits() -> ContributionLimits {
    ContributionLimits {
        max_blocks: 2,
        max_sources: 4,
        max_bytes: 4096,
        max_output_tokens: 1024,
        max_child_tokens: 100,
        max_child_calls: 2,
    }
}
fn request() -> ProjectionRequest {
    ProjectionRequest {
        model_id: "gpt-4o".into(),
        policy_revision: "source-policy-1".into(),
        permission_revision: "source-permission-1".into(),
        required_blocks: vec![],
        required_message_ids: vec![],
        tier: SummaryLevel::Detailed,
        defer_reductions: false,
    }
}
#[tokio::test]
async fn native_frozen_fork_edits_compaction_and_bounded_child_contributions_survive_restart() {
    let dir = tempfile::tempdir().unwrap();
    let actor = ActorEngine::open(config(dir.path())).await.unwrap();
    let original = ingest(
        &actor,
        "parent",
        "parent-conversation",
        "pressure-original",
        1,
        "Pressure measured 20 kPa.",
    )
    .await;
    let temperature = ingest(
        &actor,
        "parent",
        "parent-conversation",
        "temperature",
        2,
        "Temperature measured 25 C.",
    )
    .await;
    let parent = history(&actor, "parent", "parent-conversation").await;
    let fork_plan = plan_fork(&parent, "child", limits()).unwrap();
    fork_plan.validate_parent(&parent).unwrap();
    let fork = ForkRequest {
        version: 1,
        scope: scope(),
        parent_session_id: "parent".into(),
        parent_conversation: "parent-conversation".into(),
        child_session_id: "child".into(),
        child_conversation: "child-conversation".into(),
    };
    context_history::fork(&actor, &scope(), &fork)
        .await
        .unwrap();
    let child = history(&actor, "child", "child-conversation").await;
    fork_plan.validate_child(&child).unwrap();
    assert_eq!(
        child
            .recover(&scope(), &child.source_span("pressure-original").unwrap())
            .unwrap(),
        original
    );
    let corrected = ingest(
        &actor,
        "parent",
        "parent-conversation",
        "pressure-corrected",
        3,
        "Pressure corrected to 21 kPa.",
    )
    .await;
    let parent = history(&actor, "parent", "parent-conversation").await;
    let edit = plan_edit(
        &parent,
        SourceRelation::Edit {
            id: "parent-pressure-edit".into(),
            original_id: "pressure-original".into(),
            replacement_id: "pressure-corrected".into(),
        },
    )
    .unwrap();
    edit.validate(&parent).unwrap();
    context_history::relate(
        &actor,
        &scope(),
        &RelationIngestion {
            version: 1,
            scope: scope(),
            session_id: "parent".into(),
            conversation: "parent-conversation".into(),
            relation: edit.relation.clone(),
        },
    )
    .await
    .unwrap();
    let parent = history(&actor, "parent", "parent-conversation").await;
    assert!(edit.validate(&parent).is_err());
    assert!(fork_plan.validate_parent(&parent).is_err());
    let child = history(&actor, "child", "child-conversation").await;
    fork_plan.validate_child(&child).unwrap();
    assert!(
        child
            .visible_messages()
            .iter()
            .any(|m| m.id == "pressure-original")
    );
    assert!(child.message("pressure-corrected").is_err());
    let child_bytes = ingest(
        &actor,
        "child",
        "child-conversation",
        "child-pressure",
        3,
        "Child measured pressure 22 kPa.",
    )
    .await;
    let child = history(&actor, "child", "child-conversation").await;
    let child_edit = plan_edit(
        &child,
        SourceRelation::Edit {
            id: "child-pressure-edit".into(),
            original_id: "pressure-original".into(),
            replacement_id: "child-pressure".into(),
        },
    )
    .unwrap();
    context_history::relate(
        &actor,
        &scope(),
        &RelationIngestion {
            version: 1,
            scope: scope(),
            session_id: "child".into(),
            conversation: "child-conversation".into(),
            relation: child_edit.relation,
        },
    )
    .await
    .unwrap();
    let active_bytes = ingest(
        &actor,
        "child",
        "child-conversation",
        "child-active",
        4,
        "Active work awaits analysis.",
    )
    .await;
    let child = history(&actor, "child", "child-conversation").await;
    let parent = history(&actor, "parent", "parent-conversation").await;
    let parent_bytes = parent.export_canonical().unwrap();
    let child_snapshot = child.export_canonical().unwrap();
    let tokenizer =
        hm_compose::tokens::TokenCounter::for_model("gpt-4o", None, Default::default()).unwrap();
    assert!(matches!(
        tokenizer,
        hm_compose::tokens::TokenCounter::Tiktoken { .. }
    ));
    let counter = |bytes: &[u8]| {
        tokenizer
            .count(bytes)
            .map(|n| n as u64)
            .map_err(|_| ContextError::Unavailable("token counter".into()))
    };
    let block = ContextBlock {
        id: "child-contribution".into(),
        text: "Child pressure was 22 kPa.".into(),
        authority: Authority::DerivedInference,
        provenance: vec![child.source_span("child-pressure").unwrap()],
        tokens: 1,
        required: false,
    };
    let before = actor.stats().await.unwrap().log_events;
    let contribution = plan_contribution(
        &parent,
        &child,
        &fork_plan,
        &[block.clone()],
        Usage::Known(45),
        1,
        &counter,
    )
    .unwrap();
    contribution
        .validate(&parent, &child, &fork_plan, &counter)
        .unwrap();
    assert!(contribution.blocks[0].tokens > 1);
    assert_eq!(contribution.child_usage, Usage::Known(45));
    assert!(
        plan_contribution(
            &parent,
            &child,
            &fork_plan,
            &[block.clone()],
            Usage::Unknown,
            1,
            &counter
        )
        .is_err()
    );
    assert!(
        plan_contribution(
            &parent,
            &child,
            &fork_plan,
            &[block.clone()],
            Usage::Known(101),
            1,
            &counter
        )
        .is_err()
    );
    assert!(
        plan_contribution(
            &parent,
            &child,
            &fork_plan,
            &[block.clone()],
            Usage::Known(45),
            3,
            &counter
        )
        .is_err()
    );
    let mut privileged = block.clone();
    privileged.authority = Authority::RuntimeFact;
    assert!(
        plan_contribution(
            &parent,
            &child,
            &fork_plan,
            &[privileged],
            Usage::Known(45),
            1,
            &counter
        )
        .is_err()
    );
    let mut wrong = block.clone();
    wrong.provenance[0].byte_end += 1;
    assert!(
        plan_contribution(
            &parent,
            &child,
            &fork_plan,
            &[wrong],
            Usage::Known(45),
            1,
            &counter
        )
        .is_err()
    );
    assert_eq!(actor.stats().await.unwrap().log_events, before);
    assert_eq!(parent.export_canonical().unwrap(), parent_bytes);
    assert_eq!(child.export_canonical().unwrap(), child_snapshot);
    assert!(parent.message("child-pressure").is_err());
    let coverage: Vec<_> = child
        .visible_messages()
        .into_iter()
        .take(2)
        .map(|m| child.source_span(&m.id).unwrap())
        .collect();
    let sources: Vec<_> = child
        .visible_messages()
        .into_iter()
        .take(2)
        .cloned()
        .collect();
    let chunk = select_chunks_with_spans(
        &sources,
        &coverage,
        ChunkLimits {
            max_messages: 8,
            max_bytes: 16384,
        },
    )
    .unwrap()
    .remove(0);
    let tiers = [
        "Temperature 25 C; child pressure 22 kPa.",
        "25 C; 22 kPa.",
        "25 C, 22 kPa.",
        "25 C/22 kPa.",
    ]
    .map(|text| HistorianTier {
        text: text.into(),
        coverage: coverage.clone(),
    });
    let result = HistorianResult {
        source_digest: chunk.digest.clone(),
        tiers,
    };
    let raw = ContextBlock {
        id: "child-compaction".into(),
        text: "Temperature measured 25 C. Child measured pressure 22 kPa.".into(),
        authority: Authority::UserAsserted,
        provenance: coverage.clone(),
        tokens: 32,
        required: false,
    };
    let item = ReductionItem::original(raw.clone());
    let summary = ContextBlock {
        id: raw.id,
        text: result.tiers[0].text.clone(),
        authority: Authority::DerivedInference,
        provenance: coverage.clone(),
        tokens: 12,
        required: false,
    };
    let relation = reduction_relation(&child, &item, &summary, SummaryTier::Detailed).unwrap();
    relation.validate(&child, &summary).unwrap();
    let first = context_projection::assemble(&actor, &child, request(), tail(&actor).await)
        .await
        .unwrap();
    context_projection::publish_summary(
        &actor,
        &child,
        &first.fence,
        chunk.clone(),
        result.clone(),
        tail(&actor).await,
    )
    .await
    .unwrap();
    let folded = context_projection::assemble(&actor, &child, request(), tail(&actor).await)
        .await
        .unwrap();
    assert_eq!(folded.messages.len(), 1);
    assert_eq!(folded.messages[0].id, "child-active");
    assert_eq!(folded.blocks[0].authority, Authority::DerivedInference);
    assert_eq!(folded.blocks[0].provenance, coverage);
    let folded_bytes = folded.bytes.clone();
    let count = child.messages().len();
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(dir.path())).await.unwrap();
    let parent = history(&actor, "parent", "parent-conversation").await;
    let child = history(&actor, "child", "child-conversation").await;
    fork_plan.validate_child(&child).unwrap();
    relation.validate(&child, &summary).unwrap();
    contribution
        .validate(&parent, &child, &fork_plan, &counter)
        .unwrap();
    assert_eq!(child.messages().len(), count);
    assert_eq!(
        context_projection::assemble(&actor, &child, request(), tail(&actor).await)
            .await
            .unwrap()
            .bytes,
        folded_bytes
    );
    for (id, bytes, expected) in [
        (
            "pressure-original",
            original,
            source("pressure-original", 1, "Pressure measured 20 kPa."),
        ),
        (
            "temperature",
            temperature,
            source("temperature", 2, "Temperature measured 25 C."),
        ),
        (
            "child-pressure",
            child_bytes,
            source("child-pressure", 3, "Child measured pressure 22 kPa."),
        ),
    ] {
        assert_eq!(child.message(id).unwrap(), &expected);
        let span = child.source_span(id).unwrap();
        assert_eq!(
            context_history::recover(&actor, &scope(), "child", "child-conversation", &span)
                .await
                .unwrap(),
            bytes
        );
        assert_eq!(
            context_projection::expand(&actor, &scope(), "child", &span)
                .await
                .unwrap(),
            bytes
        );
    }
    assert_eq!(
        context_history::recover(
            &actor,
            &scope(),
            "child",
            "child-conversation",
            &child.source_span("child-active").unwrap()
        )
        .await
        .unwrap(),
        active_bytes
    );
    assert_eq!(
        parent
            .recover(&scope(), &parent.source_span("pressure-corrected").unwrap())
            .unwrap(),
        corrected
    );
    assert_eq!(
        parent
            .visible_messages()
            .iter()
            .map(|m| m.id.as_str())
            .collect::<Vec<_>>(),
        vec!["temperature", "pressure-corrected"]
    );
    let hidden = SourceRelation::Tombstone {
        id: "child-source-withdrawn".into(),
        source_id: "child-pressure".into(),
    };
    context_history::relate(
        &actor,
        &scope(),
        &RelationIngestion {
            version: 1,
            scope: scope(),
            session_id: "child".into(),
            conversation: "child-conversation".into(),
            relation: hidden,
        },
    )
    .await
    .unwrap();
    let changed = history(&actor, "child", "child-conversation").await;
    assert!(relation.validate(&changed, &summary).is_err());
    assert!(
        contribution
            .validate(&parent, &changed, &fork_plan, &counter)
            .is_err()
    );
    let fresh = context_projection::assemble(&actor, &changed, request(), tail(&actor).await)
        .await
        .unwrap();
    assert!(fresh.blocks.is_empty());
    assert_eq!(fresh.messages.len(), 2);
    assert!(
        context_projection::publish_summary(
            &actor,
            &changed,
            &fresh.fence,
            chunk,
            result,
            tail(&actor).await
        )
        .await
        .is_err()
    );
    assert_eq!(
        history(&actor, "parent", "parent-conversation")
            .await
            .export_canonical()
            .unwrap(),
        parent_bytes
    );
    actor.shutdown().await.unwrap();
}
