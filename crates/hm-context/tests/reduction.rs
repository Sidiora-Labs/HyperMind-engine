use hm_context::{reduction::*, types::*};
fn item(id: &str, tokens: u64, importance: u16, age: u64) -> ReductionItem {
    let bytes = b"original evidence";
    let original = ContextBlock {
        id: id.into(),
        text: "original evidence".into(),
        authority: Authority::UserAsserted,
        provenance: vec![SourceSpan {
            source_id: id.into(),
            source_digest: digest_bytes(bytes),
            byte_start: 0,
            byte_end: bytes.len() as u64,
        }],
        tokens,
        required: false,
    };
    let mut item = ReductionItem::original(original);
    item.importance = importance;
    item.age = age;
    for (index, size) in [tokens * 4 / 5, tokens * 3 / 5, tokens * 2 / 5, tokens / 5]
        .into_iter()
        .enumerate()
    {
        let mut block = item.original.clone();
        block.text = format!("summary tier {index}");
        block.tokens = size;
        block.authority = Authority::DerivedInference;
        item.summaries[index] = Some(block);
    }
    item
}
fn budget(available: u64) -> TokenBudget {
    TokenBudget {
        context_tokens: available + 20,
        reserved_output_tokens: 10,
        required_tokens: 10,
    }
}
#[test]
fn four_tiers_reclaim_and_exact_recovery() {
    for (available, tier) in [
        (80, SummaryTier::Detailed),
        (60, SummaryTier::Condensed),
        (40, SummaryTier::Brief),
        (20, SummaryTier::Outline),
    ] {
        let plan = select(
            &[item("old", 100, 1, 100)],
            budget(available),
            ReductionPolicy::default(),
        )
        .unwrap();
        assert_eq!(plan.selections[0].tier, Some(tier));
        assert_eq!(plan.tokens, available);
        assert_eq!(
            expand(&plan.selections[0].recovery[0], "old", b"original evidence").unwrap(),
            b"original evidence"
        );
        assert!(expand(&plan.selections[0].recovery[0], "old", b"changed evidence").is_err());
    }
}
#[test]
fn selection_decays_importance_deterministically() {
    let old = item("old", 100, 10, 10_000);
    let recent = item("recent", 100, 2, 0);
    let plan = select(
        &[recent.clone(), old.clone()],
        budget(180),
        ReductionPolicy::default(),
    )
    .unwrap();
    assert_eq!(plan.selections[0].tier, None);
    assert_eq!(plan.selections[1].tier, Some(SummaryTier::Detailed));
    let reverse = select(&[old, recent], budget(180), ReductionPolicy::default()).unwrap();
    assert_eq!(reverse.selections[0].tier, Some(SummaryTier::Detailed));
}
#[test]
fn live_work_dependencies_and_output_reservation_fail_closed() {
    let mut live = item("live", 100, 0, 0);
    live.current_work = true;
    live.dependencies = vec!["call".into()];
    let call = item("call", 100, 0, 999);
    let old = item("old", 100, 0, 9999);
    let plan = select(
        &[live.clone(), call.clone(), old],
        budget(220),
        ReductionPolicy::default(),
    )
    .unwrap();
    assert!(plan.selections[0].protected && plan.selections[1].protected);
    assert_eq!(plan.blocks[0].tokens, 100);
    assert_eq!(plan.blocks[1].tokens, 100);
    assert!(matches!(
        select(&[live, call], budget(199), ReductionPolicy::default()),
        Err(ReductionError::Overflow {
            required: 200,
            available: 199
        })
    ));
    assert!(select(
        &[],
        TokenBudget {
            context_tokens: 10,
            reserved_output_tokens: 11,
            required_tokens: 0
        },
        ReductionPolicy::default()
    )
    .is_err());
}
#[test]
fn queue_is_idempotent_and_fences_source_and_publication() {
    let mut value = item("old", 100, 0, 100);
    let summary = value.summaries[0].take().unwrap();
    value.summaries = [None, None, None, None];
    let plan = select(&[value.clone()], budget(80), ReductionPolicy::default()).unwrap();
    assert_eq!(plan.omitted.len(), 1);
    assert_eq!(plan.pending.len(), 4);
    let request = plan.pending[0].clone();
    let mut queue = ReductionQueue::default();
    assert!(queue.enqueue(request.clone()).unwrap());
    assert!(!queue.enqueue(request.clone()).unwrap());
    let checkpoint = serde_json::to_vec(&queue).unwrap();
    let mut queue: ReductionQueue = serde_json::from_slice(&checkpoint).unwrap();
    let mut changed = value.clone();
    changed.original.text = "corrected".into();
    assert!(matches!(
        queue.apply(&request, &mut changed, summary.clone()),
        Err(ReductionError::Stale)
    ));
    assert!(queue.apply(&request, &mut value, summary.clone()).unwrap());
    assert!(!queue.apply(&request, &mut value, summary.clone()).unwrap());
    let mut conflict = summary;
    conflict.text = "other".into();
    assert!(queue.apply(&request, &mut value, conflict).is_err());
}
#[test]
fn dependency_groups_cannot_be_partially_omitted() {
    let mut a = item("a", 100, 0, 0);
    let mut b = item("b", 100, 0, 0);
    a.summaries = [None, None, None, None];
    b.summaries = [None, None, None, None];
    a.dependencies.push("b".into());
    let plan = select(&[a, b], budget(100), ReductionPolicy::default()).unwrap();
    assert!(plan.blocks.is_empty());
    assert_eq!(plan.omitted.len(), 2);
    assert_eq!(plan.recovery.len(), 2);
    assert_eq!(
        expand(&plan.recovery["a"][0], "a", b"original evidence").unwrap(),
        b"original evidence"
    );
}
fn message(id: &str, ordinal: u64, parts: Vec<MessagePart>) -> SourceMessage {
    let mut m = SourceMessage {
        id: id.into(),
        ordinal,
        role: MessageRole::Tool,
        parts,
        occurred_at_ns: None,
        recorded_at_ns: 0,
        authority: Authority::ToolObserved,
        source_digest: String::new(),
    };
    m.source_digest = m.computed_digest().unwrap();
    m
}
#[test]
fn repeated_tool_ids_keep_distinct_arcs_and_open_calls_live() {
    let messages = vec![
        message(
            "c1",
            0,
            vec![MessagePart::ToolCall {
                call_id: "repeat".into(),
                name: "read".into(),
                arguments: "{}".into(),
            }],
        ),
        message(
            "r1",
            1,
            vec![MessagePart::ToolResult {
                call_id: "repeat".into(),
                content: "one".into(),
                failed: false,
            }],
        ),
        message(
            "c2",
            2,
            vec![MessagePart::ToolCall {
                call_id: "repeat".into(),
                name: "read".into(),
                arguments: "{}".into(),
            }],
        ),
    ];
    let mut items = vec![
        item("c1", 100, 0, 0),
        item("r1", 100, 0, 0),
        item("c2", 100, 0, 0),
    ];
    protect_tool_dependencies(&mut items, &messages).unwrap();
    assert_eq!(items[0].dependencies, vec!["r1"]);
    assert_eq!(items[1].dependencies, vec!["c1"]);
    assert!(items[2].current_work);
    let plan = select(&items, budget(100), ReductionPolicy::default()).unwrap();
    assert_eq!(plan.blocks.len(), 1);
    assert_eq!(plan.blocks[0].id, "c2");
}
#[test]
fn invalid_summary_coverage_is_rejected() {
    let mut value = item("x", 100, 0, 0);
    value.summaries[0].as_mut().unwrap().provenance.clear();
    assert!(select(&[value], budget(100), ReductionPolicy::default()).is_err());
}

#[test]
fn history_expansion_preserves_host_bytes_and_scope() {
    let scope = Scope {
        owner_id: "owner".into(),
        project_id: "project".into(),
        workspace_id: None,
    };
    let mut history = hm_context::history::SourceHistory::new(scope.clone(), "session").unwrap();
    let m = message(
        "source",
        0,
        vec![MessagePart::Text {
            text: "decoded text".into(),
        }],
    );
    let raw = b"  host wire bytes with original whitespace  ".to_vec();
    history.ingest(m, raw.clone()).unwrap();
    let span = history.source_span("source").unwrap();
    assert_eq!(expand_history(&history, &scope, &span).unwrap(), raw);
    let foreign = Scope {
        owner_id: "different".into(),
        ..scope
    };
    assert!(expand_history(&history, &foreign, &span).is_err());
}
