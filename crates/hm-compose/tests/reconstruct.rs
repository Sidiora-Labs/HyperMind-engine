use hm_compose::reconstruct::{
    PROMPT_ID, ReconstructionAnchor, guard_remember, is_reconstruction, reconstruct,
    reconstruction_request,
};
use hm_core::{ErrorCode, LSN};
use hm_llm::openai_compat::OpenAiCompatible;
use hm_llm::{LlmError, ModelTier, Pricing, ProviderConfig, RecordedTransport, WireFixture};
use hm_schema::events::Authority;

fn anchors() -> Vec<ReconstructionAnchor> {
    vec![
        ReconstructionAnchor {
            lsn: LSN::new(1),
            uri: "hm://7/00000000000000000000000000000000/1".to_owned(),
            content: "At 09:00 release 7 was queued.".to_owned(),
            authority: Authority::UserAsserted,
        },
        ReconstructionAnchor {
            lsn: LSN::new(2),
            uri: "hm://7/00000000000000000000000000000000/2".to_owned(),
            content: "At 09:05 receipt confirms release 7 completed.".to_owned(),
            authority: Authority::ToolObserved,
        },
    ]
}

#[test]
fn recorded_live_reconstruction_preserves_anchors_and_never_becomes_evidence() {
    let provider = OpenAiCompatible::new(
        ProviderConfig {
            endpoint: "https://gateway.centra.ag/v1/chat/completions".to_owned(),
            api_key: None,
            model: "openrouter/openai/gpt-4o-mini".to_owned(),
            tier: ModelTier::Economy,
            pricing: Pricing::default(),
        },
        RecordedTransport::from_json(include_str!("fixtures/reconstruct-centra.json")).unwrap(),
    )
    .unwrap();
    let result = reconstruct(&provider, &anchors(), 256).unwrap();
    assert_eq!(result.authority, Authority::AssistantGenerated);
    assert_eq!(result.anchor_lsns, [LSN::new(1), LSN::new(2)]);
    assert_eq!(result.prompt_id, PROMPT_ID);
    assert!(result.content.starts_with("RECONSTRUCTION\n"));
    assert!(result.content.contains("timeline lacks information"));
    assert_eq!(result.usage.input_tokens, 283);
    assert_eq!(result.usage.output_tokens, 92);
    assert_eq!(
        guard_remember(&result.content).unwrap_err().code,
        ErrorCode::ForbiddenKind
    );
    assert_eq!(provider.transport().remaining(), 0);
}

#[test]
fn reconstruction_requires_ordered_bounded_semantic_anchors() {
    assert!(reconstruction_request(&anchors()[..1], 256).is_err());
    assert!(reconstruction_request(&anchors(), 0).is_err());
    assert!(reconstruction_request(&anchors(), 4_097).is_err());
    let mut unordered = anchors();
    unordered.reverse();
    assert!(reconstruction_request(&unordered, 256).is_err());
    let mut invalid = anchors();
    for content in ["", "NCEV raw bytes", "RECONSTRUCTION\nprevious narrative"] {
        invalid[0].content = content.to_owned();
        assert!(reconstruction_request(&invalid, 256).is_err());
    }
    for content in [
        "RECONSTRUCTION",
        "  RECONSTRUCTION\nuncertain",
        "RECONSTRUCTION: uncertain",
    ] {
        assert!(is_reconstruction(content));
        assert!(guard_remember(content).is_err());
    }
    assert!(guard_remember("independently observed receipt").is_ok());
    assert!(!is_reconstruction("RECONSTRUCTIONS is a title"));
}

#[test]
fn corrupted_recorded_narrative_rejects_unknown_anchors_and_wrong_shapes() {
    for content in [
        r#"{"narrative":"uncertain","anchor_lsns":[1,999]}"#,
        r#"{"narrative":false,"anchor_lsns":[1,2]}"#,
        r#"{"narrative":"uncertain","anchor_lsns":[1,2],"authority":"tool_observed"}"#,
    ] {
        let mut fixtures: Vec<WireFixture> =
            serde_json::from_str(include_str!("fixtures/reconstruct-centra.json")).unwrap();
        fixtures[0].response.body["choices"][0]["message"]["content"] = content.into();
        let provider = OpenAiCompatible::new(
            ProviderConfig {
                endpoint: "https://gateway.centra.ag/v1/chat/completions".to_owned(),
                api_key: None,
                model: "openrouter/openai/gpt-4o-mini".to_owned(),
                tier: ModelTier::Economy,
                pricing: Pricing::default(),
            },
            RecordedTransport::new(fixtures),
        )
        .unwrap();
        assert!(matches!(
            reconstruct(&provider, &anchors(), 256),
            Err(LlmError::Schema(_))
        ));
    }
}

#[test]
fn renderer_cannot_promote_reconstruction_to_observed_authority() {
    let temporary = tempfile::tempdir().unwrap();
    let store = hm_proj::store::ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).unwrap();
    let (conversation, frames) = common::workload();
    hm_proj::rebuild::rebuild_projection_stream(&store, &frames, true, usize::MAX).unwrap();
    let snapshot = store.begin_snapshot().unwrap();
    let counter = hm_compose::tokens::TokenCounter::for_model(
        "fallback",
        None,
        hm_compose::tokens::FallbackWeights::default(),
    )
    .unwrap();
    let mut bundle = activate(
        &snapshot,
        &ActivationRequest {
            actor: hm_core::ActorId::new(19),
            conversation,
            query: "alpha".to_owned(),
            turn_text: String::new(),
            budget_tokens: 10_000,
            token_counter: &counter,
            maximum_candidates: 64,
            maximum_conversation_records: 64,
        },
    )
    .unwrap();
    let item = &mut bundle.sections[Tier::Conversation as usize].items[0];
    item.content = b"RECONSTRUCTION\nThe intervening actions are unknown.".to_vec();
    item.authority = Authority::ToolObserved;
    let rendered = hm_compose::safety::render(&bundle, []).unwrap();
    let item = &rendered.sections[Tier::Conversation as usize].items[0];
    assert_eq!(item.authority, Authority::AssistantGenerated);
    assert_eq!(item.role, "user");
    assert!(is_reconstruction(&item.content));
}
mod common;

use hm_compose::bundle::{ActivationRequest, Tier, activate};
