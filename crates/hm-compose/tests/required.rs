use hm_compose::budget::BudgetProfile;
use hm_compose::bundle::{
    ActivationContext, ActivationRequest, GapKind, Tier, activate, activate_with_context,
};
use hm_compose::tokens::{FallbackWeights, TokenCounter};
use hm_core::{ActorId, ConversationId, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_proj::bindings::BindingRequirement;
use hm_proj::rebuild::rebuild_projection_stream;
use hm_proj::store::ProjectionStore;
use hm_schema::event::encode_event_envelope;
use hm_schema::events::{
    Authority, Binding, Effect, EffectState, EventEnvelope, EventPayload, IntentSet, LoopOpened,
    Retention, Sensitivity, ToolCall, UserMsg,
};

fn event(
    lsn: u64,
    conversation: ConversationId,
    kind: EventKind,
    payload: EventPayload,
    event_time_ns: i64,
) -> Frame {
    let authority = match kind {
        EventKind::ToolCall => Authority::AssistantGenerated,
        EventKind::Effect | EventKind::Binding => Authority::RuntimeFact,
        _ => Authority::UserAsserted,
    };
    Frame {
        header: FrameHeader {
            lsn: LSN::new(lsn),
            kind,
            wall_timestamp_ns: UtcNanos::new(event_time_ns),
            actor: ActorId::new(19),
            conversation,
        },
        sealed_payload: encode_event_envelope(&EventEnvelope {
            schema_version: 2,
            payload,
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 0,
            origin_actor: 19,
            run_id: None,
            model_provenance: None,
            authority,
            retention: Retention::CurrentState,
            sensitivity: Sensitivity::Public,
            event_time_ns,
        }),
    }
}

fn workload(conversation: ConversationId) -> Vec<Frame> {
    vec![
        event(
            1,
            conversation,
            EventKind::UserMsg,
            EventPayload::UserMsg(Box::new(UserMsg {
                content: b"evidence for active bindings and optional conversation text".to_vec(),
            })),
            1_000,
        ),
        event(
            2,
            conversation,
            EventKind::IntentSet,
            EventPayload::IntentSet(Box::new(IntentSet {
                objective:
                    b"finish the complete continuity wave without losing exact operational state"
                        .to_vec(),
            })),
            1_100,
        ),
        event(
            3,
            conversation,
            EventKind::LoopOpened,
            EventPayload::LoopOpened(Box::new(LoopOpened {
                loop_id: b"older-task".to_vec(),
                objective: b"retain this older open loop while the active subtask is affordable"
                    .to_vec(),
            })),
            1_200,
        ),
        event(
            4,
            conversation,
            EventKind::LoopOpened,
            EventPayload::LoopOpened(Box::new(LoopOpened {
                loop_id: b"active-task".to_vec(),
                objective: b"dispatch the exact write and reconcile its uncertain outcome".to_vec(),
            })),
            1_300,
        ),
        binding(5, conversation, "repo", "revision", b"r1", 5_000, 1_400),
        binding(6, conversation, "cache", "revision", b"c1", 100, 1_000),
        event(
            7,
            conversation,
            EventKind::ToolCall,
            EventPayload::ToolCall(Box::new(ToolCall {
                call_id: b"call-write".to_vec(),
                tool_name: "write".to_owned(),
                arguments: b"large exact argument payload for continuity".to_vec(),
            })),
            1_900,
        ),
        event(
            8,
            conversation,
            EventKind::Effect,
            EventPayload::Effect(Box::new(Effect {
                effect_id: b"effect-write".to_vec(),
                tool_call_lsn: 7,
                state: EffectState::OutcomeUnknown,
            })),
            2_000,
        ),
    ]
}

fn binding(
    lsn: u64,
    conversation: ConversationId,
    entity: &str,
    property: &str,
    revision: &[u8],
    freshness_requirement_ns: u64,
    event_time_ns: i64,
) -> Frame {
    event(
        lsn,
        conversation,
        EventKind::Binding,
        EventPayload::Binding(Box::new(Binding {
            task: Some(b"active-task".to_vec()),
            scope: None,
            canonical_entity: entity.to_owned(),
            property: property.to_owned(),
            evidence_lsn: 1,
            revision: revision.to_vec(),
            freshness_requirement_ns,
        })),
        event_time_ns,
    )
}

fn request(
    conversation: ConversationId,
    counter: &TokenCounter,
    budget_tokens: usize,
) -> ActivationRequest<'_> {
    ActivationRequest {
        actor: ActorId::new(19),
        conversation,
        query: "evidence continuity".to_owned(),
        turn_text: "continue active task".to_owned(),
        budget_tokens,
        token_counter: counter,
        maximum_candidates: 64,
        maximum_conversation_records: 64,
    }
}

#[test]
fn intent_bindings_and_unreconciled_work_are_required_and_never_shed() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 32 * 1024 * 1024).expect("store");
    let conversation = ConversationId::new([0x55; 16]);
    rebuild_projection_stream(&store, &workload(conversation), true, usize::MAX)
        .expect("projections");
    let snapshot = store.begin_snapshot().expect("snapshot");
    let counter =
        TokenCounter::for_model("fallback", None, FallbackWeights::default()).expect("counter");
    let full = activate(&snapshot, &request(conversation, &counter, 1_000_000)).expect("full");
    assert_eq!(full.sections[Tier::Intent as usize].items.len(), 3);
    assert_eq!(full.sections[Tier::Bindings as usize].items.len(), 2);
    assert_eq!(full.sections[Tier::WorkLedger as usize].items.len(), 2);
    let required_tokens: usize = full.sections[..=Tier::WorkLedger as usize]
        .iter()
        .map(|section| section.tokens)
        .sum();
    let trimmed = activate(&snapshot, &request(conversation, &counter, required_tokens))
        .expect("optional tiers shrink");
    for tier in [
        Tier::Resident,
        Tier::Intent,
        Tier::Bindings,
        Tier::WorkLedger,
    ] {
        assert!(trimmed.sections[tier as usize].required);
        assert_eq!(trimmed.sections[tier as usize].trimmed_items, 0);
        assert_eq!(
            trimmed.sections[tier as usize].items,
            full.sections[tier as usize].items
        );
    }
    assert!(trimmed.spent_tokens <= required_tokens);
}

#[test]
fn required_overflow_narrows_to_active_loop_with_a_visible_gap() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 32 * 1024 * 1024).expect("store");
    let conversation = ConversationId::new([0x56; 16]);
    rebuild_projection_stream(&store, &workload(conversation), true, usize::MAX)
        .expect("projections");
    let snapshot = store.begin_snapshot().expect("snapshot");
    let counter =
        TokenCounter::for_model("fallback", None, FallbackWeights::default()).expect("counter");
    let full = activate(&snapshot, &request(conversation, &counter, 1_000_000)).expect("full");
    let required_tokens: usize = full.sections[..=Tier::WorkLedger as usize]
        .iter()
        .map(|section| section.tokens)
        .sum();
    let narrowed = (1..required_tokens)
        .find_map(|budget| {
            activate(&snapshot, &request(conversation, &counter, budget))
                .ok()
                .filter(|bundle| {
                    bundle
                        .gaps
                        .iter()
                        .any(|gap| gap.kind == GapKind::NarrowedSubtask)
                })
        })
        .expect("a compact active subtask fits");
    assert_eq!(narrowed.sections[Tier::Intent as usize].items.len(), 1);
    assert!(narrowed.sections[Tier::Intent as usize].items[0].coarsened);
    assert!(
        narrowed.sections[..=Tier::WorkLedger as usize]
            .iter()
            .all(|section| section.trimmed_items == 0)
    );
    assert!(narrowed.spent_tokens <= narrowed.budget_tokens);
}

#[test]
fn missing_stale_and_conflicting_bindings_are_explicit_gaps() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 32 * 1024 * 1024).expect("store");
    let conversation = ConversationId::new([0x57; 16]);
    rebuild_projection_stream(&store, &workload(conversation), true, usize::MAX)
        .expect("projections");
    let snapshot = store.begin_snapshot().expect("snapshot");
    let counter =
        TokenCounter::for_model("fallback", None, FallbackWeights::default()).expect("counter");
    let context = ActivationContext {
        task: Some(b"active-task".to_vec()),
        required_bindings: vec![
            requirement("repo", b"wrong", 5_000),
            requirement("cache", b"c1", 100),
            requirement("missing", b"m1", 100),
        ],
        now_ns: Some(UtcNanos::new(2_000)),
        budget_profile: BudgetProfile::default(),
    };
    let bundle = activate_with_context(
        &snapshot,
        &request(conversation, &counter, 1_000_000),
        &context,
    )
    .expect("activation");
    for expected in [
        GapKind::MissingBinding,
        GapKind::StaleBinding,
        GapKind::ConflictingBinding,
    ] {
        assert!(bundle.gaps.iter().any(|gap| gap.kind == expected));
    }
}

#[test]
fn default_budget_profile_is_exact_and_custom_profiles_must_sum_to_one_hundred() {
    assert_eq!(
        BudgetProfile::default()
            .allocate(1_000)
            .expect("allocation"),
        hm_compose::budget::BudgetAllocation {
            mandatory: 150,
            intent_bindings_work: 100,
            conversation: 400,
            recall: 150,
            tools: 100,
            reserve: 100,
        }
    );
    assert!(
        BudgetProfile {
            reserve_percent: 9,
            ..BudgetProfile::default()
        }
        .validate()
        .is_err()
    );
}

fn requirement(entity: &str, revision: &[u8], freshness: u64) -> BindingRequirement {
    BindingRequirement {
        canonical_entity: entity.to_owned(),
        property: "revision".to_owned(),
        revision: Some(revision.to_vec()),
        freshness_requirement_ns: Some(freshness),
    }
}
