mod common;

use hm_compose::bundle::{ActivationRequest, GapKind, HealthStatus, Tier, WhyCode, activate};
use hm_compose::canonical::{CANONICAL_MAGIC, canonical_bytes, verify_bundle_hash};
use hm_compose::tokens::{FallbackWeights, TokenCounter};
use hm_core::ActorId;
use hm_proj::rebuild::rebuild_projection_stream;
use hm_proj::store::ProjectionStore;

#[test]
fn activation_declares_ten_tiers_and_populates_conversation_and_fused() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).expect("store");
    let (conversation, frames) = common::workload();
    rebuild_projection_stream(&store, &frames, true, usize::MAX).expect("projections");
    let snapshot = store.begin_snapshot().expect("snapshot");
    let counter =
        TokenCounter::for_model("fallback", None, FallbackWeights::default()).expect("counter");
    let request = ActivationRequest {
        actor: ActorId::new(19),
        conversation,
        query: "alpha rare".to_owned(),
        turn_text: "inspect alpha".to_owned(),
        budget_tokens: 1_000_000,
        token_counter: &counter,
        maximum_candidates: 64,
        maximum_conversation_records: 64,
    };
    let first = activate(&snapshot, &request).expect("activate");
    let second = activate(&snapshot, &request).expect("activate deterministically");
    assert_eq!(first, second);
    assert_eq!(first.sections.len(), 10);
    for (section, tier) in first.sections.iter().zip(Tier::ALL) {
        assert_eq!(section.tier, tier);
        assert_eq!(section.required, tier.required());
    }
    assert_eq!(first.sections[Tier::Conversation as usize].items.len(), 2);
    assert_eq!(first.sections[Tier::Fused as usize].items.len(), 2);
    assert!(
        first.sections[Tier::Fused as usize]
            .items
            .iter()
            .all(|item| {
                item.uri.starts_with("hm://19/")
                    && item.uri.contains("lr=")
                    && item.uri.contains("why=lexical")
                    && item.why == WhyCode::Lexical
            })
    );
    assert_eq!(first.manifest.candidates.len(), 2);
    assert_eq!(first.manifest.selected, first.manifest.included);
    assert!(first.manifest.used.is_empty());
    assert_eq!(first.health.inclusion, HealthStatus::LexicalOnly);
    assert_eq!(
        &canonical_bytes(&first).expect("canonical")[..4],
        CANONICAL_MAGIC
    );
    verify_bundle_hash(&first).expect("bundle hash");
}

#[test]
fn trim_drops_tail_tiers_then_coarsens_conversation_with_visible_gaps() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).expect("store");
    let (conversation, frames) = common::workload();
    rebuild_projection_stream(&store, &frames, true, usize::MAX).expect("projections");
    let snapshot = store.begin_snapshot().expect("snapshot");
    let counter =
        TokenCounter::for_model("fallback", None, FallbackWeights::default()).expect("counter");
    let request = ActivationRequest {
        actor: ActorId::new(19),
        conversation,
        query: "alpha".to_owned(),
        turn_text: String::new(),
        budget_tokens: 35,
        token_counter: &counter,
        maximum_candidates: 1,
        maximum_conversation_records: 64,
    };
    let bundle = activate(&snapshot, &request).expect("trimmed activation");
    assert!(bundle.spent_tokens <= request.budget_tokens);
    assert!(
        bundle.sections[..4]
            .iter()
            .all(|section| section.required && section.trimmed_items == 0)
    );
    assert!(bundle.sections[Tier::Fused as usize].items.is_empty());
    assert!(bundle.sections[Tier::Conversation as usize].coarsened_items > 0);
    assert!(
        bundle
            .gaps
            .iter()
            .any(|gap| gap.kind == GapKind::DroppedTier && gap.tier == Some(Tier::Fused))
    );
    assert!(
        bundle
            .gaps
            .iter()
            .any(|gap| gap.kind == GapKind::TruncatedLane)
    );
    verify_bundle_hash(&bundle).expect("trimmed hash");
}

#[test]
fn empty_query_builds_a_conversation_only_bundle() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).expect("store");
    let (conversation, frames) = common::workload();
    rebuild_projection_stream(&store, &frames, true, usize::MAX).expect("projections");
    let snapshot = store.begin_snapshot().expect("snapshot");
    let counter =
        TokenCounter::for_model("fallback", None, FallbackWeights::default()).expect("counter");
    let bundle = activate(
        &snapshot,
        &ActivationRequest {
            actor: ActorId::new(19),
            conversation,
            query: String::new(),
            turn_text: String::new(),
            budget_tokens: 1_000,
            token_counter: &counter,
            maximum_candidates: 64,
            maximum_conversation_records: 64,
        },
    )
    .expect("conversation activation");
    assert_eq!(bundle.sections[Tier::Conversation as usize].items.len(), 2);
    assert!(bundle.sections[Tier::Fused as usize].items.is_empty());
    assert!(bundle.manifest.candidate_lanes.is_empty());
}
