#![forbid(unsafe_code)]

#[allow(dead_code)]
mod common;

use hm_compose::bundle::{
    ActivationBundle, ActivationContext, ActivationRequest, GapKind, MAXIMUM_PAIRED_EVIDENCE, Tier,
    WhyCode, activate_with_context,
};
use hm_compose::canonical::verify_bundle_hash;
use hm_compose::tokens::{FallbackWeights, TokenCounter};
use hm_core::{ActorId, ConversationId, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_proj::rebuild::rebuild_projection_stream;
use hm_proj::store::ProjectionStore;
use hm_schema::event::encode_event_envelope;
use hm_schema::events::{
    Authority, ConsolidationBudget, ConsolidationClosed, ConsolidationOpened,
    ConsolidationPhaseName, EventEnvelope, EventPayload, MemoryMinted, ModelProvenance,
    PromptVersion, ProvenanceRange, Retention, Sensitivity,
};

const RUN: &[u8] = b"run-summaries";
const SOURCE: u8 = 0x11;
const ACTIVE: u8 = 0x22;
const DERIVED: u8 = 0x55;
const DEFINITION: &str = "keys rotate quarterly under the published policy";
const OBSERVATION: &str = "the deployment key rotates every ninety days";

fn model(call: u8) -> ModelProvenance {
    ModelProvenance {
        model_id: "fixture-model".to_owned(),
        prompt_id: "merge-cluster".to_owned(),
        prompt_version: 1,
        temperature: 0.0,
        call_id: Some(vec![call]),
        input_tokens: 20,
        output_tokens: 5,
        cache_read_tokens: 0,
        cache_write_tokens: 0,
        cost_microusd: 7,
    }
}

fn run_frame(
    lsn: u64,
    kind: EventKind,
    payload: EventPayload,
    model_provenance: Option<ModelProvenance>,
) -> Frame {
    let timestamp = i64::try_from(lsn).expect("timestamp") * 1_000;
    Frame {
        header: FrameHeader {
            lsn: LSN::new(lsn),
            kind,
            wall_timestamp_ns: UtcNanos::new(timestamp),
            actor: ActorId::new(19),
            conversation: common::conversation(DERIVED),
        },
        sealed_payload: encode_event_envelope(&EventEnvelope {
            schema_version: 2,
            payload,
            connection_id: None,
            client_seq: 0,
            client_event_index: 0,
            client_event_count: 0,
            origin_actor: 0,
            run_id: Some(RUN.to_vec()),
            model_provenance: model_provenance.map(Box::new),
            authority: Authority::DerivedInference,
            retention: Retention::Durable,
            sensitivity: Sensitivity::Public,
            event_time_ns: timestamp,
        }),
    }
}

fn opened(lsn: u64) -> Frame {
    run_frame(
        lsn,
        EventKind::ConsolidationOpened,
        EventPayload::ConsolidationOpened(Box::new(ConsolidationOpened {
            scope_digest: vec![0x5a; 32],
            cadence_key: "summaries".to_owned(),
            generation: 1,
            expected_active_generation: 0,
            phases: vec![ConsolidationPhaseName::Nrem],
            prompts: vec![PromptVersion {
                prompt_id: "merge-cluster".to_owned(),
                version: 1,
                model_id: "fixture-model".to_owned(),
            }],
            budget: Box::new(ConsolidationBudget {
                max_llm_calls: 128,
                max_tokens: 100_000,
                max_microusd: 10_000,
                max_wall_ms: 60_000,
            }),
            source_first_lsn: 0,
            source_last_lsn: 0,
        })),
        None,
    )
}

fn closed(lsn: u64, records: u64) -> Frame {
    run_frame(
        lsn,
        EventKind::ConsolidationClosed,
        EventPayload::ConsolidationClosed(Box::new(ConsolidationClosed {
            generation: 1,
            expected_active_generation: 0,
            derived_records: records,
            dropped_candidates: 0,
            llm_calls: records,
            input_tokens: records * 20,
            output_tokens: records * 5,
            cost_microusd: records * 7,
        })),
        None,
    )
}

fn minted(lsn: u64, index: u8, definition: &str, citation_lsn: u64) -> Frame {
    run_frame(
        lsn,
        EventKind::MemoryMinted,
        EventPayload::MemoryMinted(Box::new(MemoryMinted {
            memory_id: format!("memory-{index:03}").into_bytes(),
            name: format!("Rotation {index:03}"),
            definition: definition.as_bytes().to_vec(),
            tags: vec!["policy".to_owned()],
            salience_micros: 800_000,
            citations: vec![ProvenanceRange {
                first_lsn: citation_lsn,
                last_lsn: citation_lsn,
                byte_start: 0,
                byte_end: 8,
            }],
        })),
        Some(model(index)),
    )
}

fn paired_workload() -> Vec<Frame> {
    let source = common::conversation(SOURCE);
    vec![
        common::message_frame(1, source, EventKind::UserMsg, OBSERVATION),
        common::message_frame(
            2,
            source,
            EventKind::DeliveredMsg,
            "the runbook records each rotation",
        ),
        opened(3),
        minted(4, 1, DEFINITION, 1),
        closed(5, 1),
    ]
}

fn bundle(
    frames: &[Frame],
    conversation: ConversationId,
    budget_tokens: usize,
) -> ActivationBundle {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).expect("store");
    rebuild_projection_stream(&store, frames, true, usize::MAX).expect("projections");
    let snapshot = store.begin_snapshot().expect("snapshot");
    let counter =
        TokenCounter::for_model("fallback", None, FallbackWeights::default()).expect("counter");
    activate_with_context(
        &snapshot,
        &ActivationRequest {
            actor: ActorId::new(19),
            conversation,
            query: "quarterly policy".to_owned(),
            turn_text: String::new(),
            budget_tokens,
            token_counter: &counter,
            maximum_candidates: 128,
            maximum_conversation_records: 64,
        },
        &ActivationContext {
            now_ns: Some(UtcNanos::new(1_000_000)),
            ..ActivationContext::default()
        },
    )
    .expect("activation")
}

fn count(bundle: &ActivationBundle, why: WhyCode) -> usize {
    bundle.sections[Tier::Fused as usize]
        .items
        .iter()
        .filter(|item| item.why == why)
        .count()
}

fn every_evidence_follows_its_summary(bundle: &ActivationBundle) -> bool {
    let items = &bundle.sections[Tier::Fused as usize].items;
    items.iter().enumerate().all(|(index, item)| {
        item.why != WhyCode::Evidence
            || index
                .checked_sub(1)
                .is_some_and(|previous| items[previous].why == WhyCode::Fused)
    })
}

#[test]
fn a_summary_hit_is_paired_with_the_original_evidence_it_cites() {
    let bundle = bundle(&paired_workload(), common::conversation(ACTIVE), 1_000_000);
    let items = &bundle.sections[Tier::Fused as usize].items;
    assert_eq!(items.len(), 2);

    let summary = &items[0];
    assert_eq!(summary.why, WhyCode::Fused);
    assert_eq!(summary.content, DEFINITION.as_bytes());

    let evidence = &items[1];
    assert_eq!(evidence.why, WhyCode::Evidence);
    assert_eq!(evidence.content, OBSERVATION.as_bytes());
    assert_eq!(evidence.provenance, vec![LSN::new(1)]);
    assert_eq!(evidence.tier, Tier::Fused);
    assert!(evidence.uri.contains("why=evidence"));
    assert!(evidence.uri.starts_with("hm://19/"));

    assert_eq!(
        bundle.spent_tokens,
        bundle
            .sections
            .iter()
            .map(|section| section.tokens)
            .sum::<usize>()
    );
    verify_bundle_hash(&bundle).expect("bundle hash");
}

#[test]
fn paired_evidence_keeps_the_original_authority_and_skips_unusable_citations() {
    let source = common::conversation(SOURCE);
    let active = common::conversation(ACTIVE);
    let frames = vec![
        common::message_frame(1, source, EventKind::UserMsg, OBSERVATION),
        common::message_frame(
            2,
            active,
            EventKind::UserMsg,
            "a local note about the rotation runbook",
        ),
        opened(3),
        minted(4, 1, DEFINITION, 1),
        minted(5, 2, DEFINITION, 1),
        minted(6, 3, DEFINITION, 2),
        minted(7, 4, DEFINITION, 3),
        minted(8, 5, DEFINITION, 999),
        closed(9, 5),
    ];
    let bundle = bundle(&frames, active, 1_000_000);

    assert_eq!(count(&bundle, WhyCode::Fused), 5);
    assert_eq!(count(&bundle, WhyCode::Evidence), 1);

    let items = &bundle.sections[Tier::Fused as usize].items;
    let evidence = items
        .iter()
        .find(|item| item.why == WhyCode::Evidence)
        .expect("paired evidence");
    assert_eq!(evidence.authority, Authority::UserAsserted);
    assert_eq!(evidence.provenance, vec![LSN::new(1)]);
    assert_eq!(evidence.content, OBSERVATION.as_bytes());
    assert!(
        items
            .iter()
            .filter(|item| item.why == WhyCode::Fused)
            .all(|item| item.authority == Authority::DerivedInference)
    );
    assert!(every_evidence_follows_its_summary(&bundle));
    verify_bundle_hash(&bundle).expect("bundle hash");
}

#[test]
fn a_dropped_summary_takes_its_paired_evidence_with_it() {
    let frames = paired_workload();
    let active = common::conversation(ACTIVE);
    let whole = bundle(&frames, active, 1_000_000);
    assert_eq!(count(&whole, WhyCode::Evidence), 1);
    assert_eq!(whole.sections[Tier::Fused as usize].items.len(), 2);

    let budget = whole.spent_tokens - whole.sections[Tier::Temporal as usize].tokens - 1;
    let trimmed = bundle(&frames, active, budget);
    assert!(trimmed.spent_tokens <= budget);
    assert!(every_evidence_follows_its_summary(&trimmed));
    assert_eq!(count(&trimmed, WhyCode::Evidence), 0);
    assert_eq!(count(&trimmed, WhyCode::Fused), 0);
    assert_eq!(trimmed.sections[Tier::Fused as usize].trimmed_items, 2);
    assert!(
        trimmed
            .gaps
            .iter()
            .any(|gap| gap.kind == GapKind::DroppedTier && gap.tier == Some(Tier::Fused))
    );
    verify_bundle_hash(&trimmed).expect("trimmed bundle hash");
}

#[test]
fn paired_evidence_is_capped_and_reports_the_cap() {
    let source = common::conversation(SOURCE);
    let memories = 70_u8;
    let mut frames = Vec::new();
    for index in 1..=memories {
        frames.push(common::message_frame(
            u64::from(index),
            source,
            EventKind::UserMsg,
            &format!("observation {index} about the rotation window"),
        ));
    }
    frames.push(opened(u64::from(memories) + 1));
    for index in 1..=memories {
        frames.push(minted(
            u64::from(memories) + 1 + u64::from(index),
            index,
            DEFINITION,
            u64::from(index),
        ));
    }
    frames.push(closed(u64::from(memories) * 2 + 2, u64::from(memories)));

    let bundle = bundle(&frames, common::conversation(ACTIVE), 1_000_000);
    assert_eq!(count(&bundle, WhyCode::Fused), usize::from(memories));
    assert_eq!(count(&bundle, WhyCode::Evidence), MAXIMUM_PAIRED_EVIDENCE);
    assert_eq!(
        bundle
            .gaps
            .iter()
            .filter(|gap| gap.kind == GapKind::TruncatedLane
                && gap.tier == Some(Tier::Fused)
                && gap.lane.is_none()
                && gap.detail == "paired evidence limit reached")
            .count(),
        1
    );
    assert!(every_evidence_follows_its_summary(&bundle));
    verify_bundle_hash(&bundle).expect("bundle hash");
}
