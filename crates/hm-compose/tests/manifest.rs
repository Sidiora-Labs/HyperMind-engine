#![forbid(unsafe_code)]

#[allow(dead_code)]
mod common;

use hm_compose::bundle::{
    ActivationBundle, ActivationContext, ActivationRequest, Tier, WhyCode, activate_with_context,
};
use hm_compose::canonical::{canonical_bytes, verify_bundle_hash};
use hm_compose::manifest::manifest_id;
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

const RUN: &[u8] = b"run-manifest";
const SOURCE: u8 = 0x11;
const ACTIVE: u8 = 0x22;
const DERIVED: u8 = 0x55;
const OBSERVATION: &str = "the deployment key rotates every ninety days";
const QUOTED: &str = "the quarterly policy is published in the runbook";
const DEFINITION: &str = "keys rotate quarterly under the published policy";
const SECOND: &str = "the quarterly rotation policy is reviewed by the platform team";

const PINNED_MANIFEST_ID: [u8; 32] = [
    0x72, 0x07, 0xd5, 0xb8, 0x95, 0x9f, 0x59, 0xe3, 0x17, 0x57, 0x28, 0x2f, 0xf9, 0xfa, 0x21, 0x0b,
    0x17, 0x14, 0x56, 0x1c, 0xcb, 0x7a, 0x0c, 0x4c, 0x81, 0x81, 0x52, 0xcf, 0xc5, 0x1f, 0xfd, 0xcf,
];

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
            cadence_key: "manifest".to_owned(),
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

fn workload() -> Vec<Frame> {
    let source = common::conversation(SOURCE);
    vec![
        common::message_frame(1, source, EventKind::UserMsg, OBSERVATION),
        common::message_frame(2, source, EventKind::DeliveredMsg, QUOTED),
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

fn lsn_at(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().expect("eight bytes"))
}

#[test]
fn the_manifest_separates_assertion_support_from_included_text() {
    let bundle = bundle(&workload(), common::conversation(ACTIVE), 1_000_000);
    let items = &bundle.sections[Tier::Fused as usize].items;
    assert_eq!(items.len(), 3);
    assert_eq!(items[0].why, WhyCode::Lexical);
    assert_eq!(items[0].provenance, vec![LSN::new(2)]);
    assert_eq!(items[1].why, WhyCode::Fused);
    assert_eq!(items[1].provenance, vec![LSN::new(1)]);
    assert_eq!(items[2].why, WhyCode::Evidence);
    assert_eq!(items[2].provenance, vec![LSN::new(1)]);

    assert_eq!(bundle.manifest.support, vec![LSN::new(1)]);
    assert!(bundle.manifest.included.contains(&LSN::new(2)));
    assert!(!bundle.manifest.support.contains(&LSN::new(2)));
    assert!(!bundle.manifest.included.contains(&LSN::new(1)));
    verify_bundle_hash(&bundle).expect("bundle hash");
}

#[test]
fn trimmed_summaries_contribute_no_support() {
    let frames = workload();
    let active = common::conversation(ACTIVE);
    let whole = bundle(&frames, active, 1_000_000);
    assert_eq!(whole.manifest.support, vec![LSN::new(1)]);

    let budget = whole.spent_tokens - whole.sections[Tier::Temporal as usize].tokens - 1;
    let trimmed = bundle(&frames, active, budget);
    assert!(trimmed.spent_tokens <= budget);
    assert_eq!(count(&trimmed, WhyCode::Fused), 0);
    assert_eq!(count(&trimmed, WhyCode::Evidence), 0);
    assert_eq!(count(&trimmed, WhyCode::Lexical), 1);
    assert!(trimmed.manifest.support.is_empty());
    assert!(!trimmed.manifest.included.is_empty());
    verify_bundle_hash(&trimmed).expect("trimmed bundle hash");
}

#[test]
fn support_round_trips_through_canonical_bytes_without_moving_the_manifest_id() {
    let source = common::conversation(SOURCE);
    let frames = vec![
        common::message_frame(1, source, EventKind::UserMsg, OBSERVATION),
        common::message_frame(2, source, EventKind::DeliveredMsg, QUOTED),
        opened(3),
        minted(4, 1, DEFINITION, 1),
        minted(5, 2, SECOND, 2),
        closed(6, 2),
    ];
    let bundle = bundle(&frames, common::conversation(ACTIVE), 1_000_000);

    assert_eq!(count(&bundle, WhyCode::Fused), 2);
    assert_eq!(count(&bundle, WhyCode::Evidence), 1);
    assert_eq!(bundle.manifest.support, vec![LSN::new(1), LSN::new(2)]);
    assert!(bundle.manifest.included.contains(&LSN::new(2)));

    let bytes = canonical_bytes(&bundle).expect("canonical bytes");
    let tail = bytes.len() - 8 - 8 * bundle.manifest.support.len();
    assert_eq!(lsn_at(&bytes, tail), 2);
    assert_eq!(
        vec![
            LSN::new(lsn_at(&bytes, tail + 8)),
            LSN::new(lsn_at(&bytes, tail + 16)),
        ],
        bundle.manifest.support
    );
    assert_eq!(bytes[tail - 4], bundle.health.encoder as u8);
    assert_eq!(bytes[tail - 1], bundle.health.inclusion as u8);
    verify_bundle_hash(&bundle).expect("bundle hash");

    assert_eq!(
        manifest_id(
            &[0x11; 32],
            7,
            &[(LSN::new(2), [0x22; 32]), (LSN::new(5), [0x33; 32])],
        ),
        PINNED_MANIFEST_ID
    );
}
