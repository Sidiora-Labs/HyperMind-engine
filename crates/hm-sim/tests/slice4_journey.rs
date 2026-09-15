#![forbid(unsafe_code)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::too_many_lines
)]

use hm_compose::bundle::{ActivationRequest, RetrievalLane, WhyCode, activate};
use hm_compose::deadline::DeadlineActivator;
use hm_compose::fusion::{LaneRanking, fuse};
use hm_compose::lanes::{entity, vector};
use hm_compose::planner::{RecallMode, plan};
use hm_compose::tokens::{FallbackWeights, TokenCounter};
use hm_core::{ActorId, ConversationId, LSN, UtcNanos};
use hm_embed::{
    CachedEmbedder, Embedder, HttpFetcher, InputRole, ModelKind, ModelStore, OnnxEmbedder, quantize,
};
use hm_ledger::frame::{EventKind, FrameHeader};
use hm_ledger::keyring::{KeyHierarchy, OsEntropy};
use hm_ledger::segment::{AppendRequest, SegmentLog, SegmentLogOptions};
use hm_proj::entities::EntityProjection;
use hm_proj::store::ProjectionStore;
use hm_proj::vectors::VectorLane;
use hm_schema::event::{Boundary, CURRENT_SCHEMA_VERSION, encode_event_envelope, verify_event};
use hm_schema::events::{Authority, EventEnvelope, EventPayload, Retention, Sensitivity, UserMsg};
use std::path::Path;
use std::time::Duration;

const ACTOR: ActorId = ActorId::new(7);
const EVENT_COUNT: usize = 10_000;
const DOCUMENTS: [&str; 12] = [
    "The automobile needs petrol from a service station. The implementation path is src/hypermind/runtime.rs",
    "A physician prescribed medicine for the illness.",
    "The laptop battery is charged with a USB-C adapter.",
    "The train departs the railway platform before sunrise.",
    "The puppy sleeps beside the fireplace every evening.",
    "Fresh vegetables are stored in the refrigerator drawer.",
    "The software defect was corrected in the latest release.",
    "The attorney filed the contract with the county clerk.",
    "Heavy rainfall flooded the road near the old bridge.",
    "The musician tuned the guitar before the concert.",
    "The spacecraft entered orbit around the distant planet.",
    "The baker kneaded dough for the morning bread.",
];

#[test]
fn ten_thousand_event_meaning_journey_survives_restart_and_deadline_pressure() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let actor_directory = temporary.path().join("actor-7");
    let conversation = ConversationId::derive("slice4-meaning-journey");
    let user = [0x11; 16];
    let kek = [0x22; 32];
    let mut entropy = OsEntropy;
    let keys =
        KeyHierarchy::open_or_create(&actor_directory, ACTOR, user, &kek, &mut entropy, true)
            .expect("key hierarchy");
    let mut log =
        SegmentLog::open(&actor_directory, ACTOR, SegmentLogOptions::default()).expect("ledger");
    let requests = (0..EVENT_COUNT)
        .map(|index| {
            let lsn = LSN::new(index as u64 + 1);
            let header = FrameHeader {
                lsn,
                kind: EventKind::UserMsg,
                wall_timestamp_ns: UtcNanos::new(index as i64 + 1),
                actor: ACTOR,
                conversation,
            };
            let content = DOCUMENTS[index % DOCUMENTS.len()].as_bytes().to_vec();
            let payload = encode_event_envelope(&EventEnvelope {
                schema_version: CURRENT_SCHEMA_VERSION,
                payload: EventPayload::UserMsg(Box::new(UserMsg { content })),
                connection_id: None,
                client_seq: 0,
                client_event_index: 0,
                client_event_count: 1,
                origin_actor: ACTOR.get(),
                run_id: None,
                model_provenance: None,
                authority: Authority::ExternalObserved,
                retention: Retention::Daily,
                sensitivity: Sensitivity::Public,
                event_time_ns: index as i64 + 1,
            });
            AppendRequest {
                kind: EventKind::UserMsg,
                wall_timestamp_ns: header.wall_timestamp_ns,
                conversation,
                sealed_payload: keys
                    .seal(&header, &payload, &mut entropy)
                    .expect("seal event"),
            }
        })
        .collect::<Vec<_>>();
    let committed = log.append_batch(&requests).expect("append 10k events");
    assert_eq!(committed.first_lsn, LSN::new(1));
    assert_eq!(committed.last_lsn, LSN::new(EVENT_COUNT as u64));
    drop(log);
    drop(keys);

    let mut entropy = OsEntropy;
    let keys =
        KeyHierarchy::open_or_create(&actor_directory, ACTOR, user, &kek, &mut entropy, false)
            .expect("reopen keys");
    let log = SegmentLog::open(&actor_directory, ACTOR, SegmentLogOptions::default())
        .expect("reopen ledger");
    let frames = log.read_all().expect("read restarted ledger");
    assert_eq!(frames.len(), EVENT_COUNT);
    let mut first = frames[0].clone();
    first.sealed_payload = keys
        .unseal(&first.header, &first.sealed_payload)
        .expect("unseal first event");
    verify_event(
        &first.sealed_payload,
        hm_schema::event::EventKind::UserMsg,
        Boundary::Disk,
    )
    .expect("verify restarted event");

    let projection = ProjectionStore::open(temporary.path().join("entity"), 16 * 1024 * 1024)
        .expect("entity projection");
    EntityProjection::apply_event(&projection, &first).expect("project path entity");
    let snapshot = projection.begin_snapshot().expect("entity snapshot");
    let entity_hits =
        entity::search(&snapshot, "src/hypermind/runtime.rs", "", 10).expect("entity lane");
    assert_eq!(entity_hits[0].lsn, LSN::new(1));
    drop(snapshot);

    let model_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/hm-models");
    let embedder = OnnxEmbedder::download(
        ModelKind::BgeSmallEnV15,
        &ModelStore::new(model_root),
        &HttpFetcher,
    )
    .expect("pinned local encoder");
    let embedder = CachedEmbedder::new(embedder, 32).expect("embedding cache");
    let _ = embedder
        .embed_documents(&DOCUMENTS)
        .expect("embed unique documents");
    assert_eq!(embedder.cached_items(), DOCUMENTS.len());
    let identity = embedder.identity(InputRole::Document);
    let lane = VectorLane::open(
        temporary.path().join("vectors"),
        "slice4-bge-v1",
        &format!("{}@{}", identity.encoder_id, identity.revision),
        identity.dimensions,
    )
    .expect("vector lane");
    for start in (0..EVENT_COUNT).step_by(256) {
        let end = (start + 256).min(EVENT_COUNT);
        let texts: Vec<_> = (start..end)
            .map(|index| DOCUMENTS[index % DOCUMENTS.len()])
            .collect();
        let batch_embeddings = embedder.embed_documents(&texts).expect("cached ingest");
        for (offset, embedding) in batch_embeddings.iter().enumerate() {
            let index = start + offset;
            let quantized = quantize(embedding).expect("quantize cached document");
            lane.append(
                LSN::new(index as u64 + 1),
                &quantized.values,
                &quantized.binary_prefilter,
            )
            .expect("append vector");
        }
    }
    assert_eq!(embedder.cached_items(), DOCUMENTS.len());

    let query_text = "Where can the car be refueled?";
    let query = quantize(&embedder.embed_query(query_text).expect("embed query once"))
        .expect("quantize query");
    let vector_hits = vector::search(&lane, &query.values, &query.binary_prefilter, 10)
        .expect("vector paraphrase recall");
    assert!(vector_hits.iter().any(|hit| (hit.lsn.get() - 1) % 12 == 0));
    assert_eq!(embedder.cached_items(), DOCUMENTS.len() + 1);

    let semantic_plan = plan(query_text, "", RecallMode::Semantic, None).expect("semantic plan");
    assert!(
        semantic_plan
            .lanes
            .iter()
            .any(|lane| lane.lane == RetrievalLane::Vector)
    );
    let fused = fuse(
        ACTOR,
        conversation,
        &[
            LaneRanking {
                lane: RetrievalLane::Entity,
                weight_q16: 6 << 16,
                candidates: entity_hits,
            },
            LaneRanking {
                lane: RetrievalLane::Vector,
                weight_q16: 4 << 16,
                candidates: vector_hits,
            },
        ],
        10,
    )
    .expect("fuse lanes");
    assert!(fused.iter().any(|hit| hit.lsn == LSN::new(1)
        && hit.why == WhyCode::Fused
        && hit.uri.contains("vector:")
        && hit.uri.contains("entity:1")
        && hit.uri.contains("why=fused")));

    let base = base_bundle(temporary.path(), conversation);
    let activator = DeadlineActivator::new();
    let primed = base.clone();
    let initial = activator
        .activate_default(conversation, LSN::new(1), move || Ok(primed))
        .expect("prime deadline cache");
    assert!(!initial.degraded);
    let subscription = activator.subscribe().expect("subscribe to fresh bundles");
    let mut fresh = base;
    fresh.manifest.query_digest = *blake3::hash(query_text.as_bytes()).as_bytes();
    let stale = activator
        .activate(
            conversation,
            LSN::new(EVENT_COUNT as u64),
            Duration::from_millis(1),
            move || {
                std::thread::sleep(Duration::from_millis(20));
                Ok(fresh)
            },
        )
        .expect("stale fallback");
    assert!(stale.degraded);
    assert_eq!(stale.stale_by_lsn, EVENT_COUNT as u64 - 1);
    assert!(
        stale
            .bundle
            .gaps
            .iter()
            .any(|gap| gap.detail.contains("deadline missed"))
    );
    let pushed = subscription
        .recv_timeout(Duration::from_secs(2))
        .expect("fresh bundle subscription");
    assert_eq!(
        pushed.manifest.query_digest,
        *blake3::hash(query_text.as_bytes()).as_bytes()
    );

    let gate = hm_eval::slice4::run().expect("slice4 evaluation gate");
    assert_eq!(gate.encoder, "lexical_only");
    assert!(
        gate.judge_free
            .iter()
            .any(|metric| { metric.name == "recall_at_10_10000" && metric.value >= 0.95 })
    );
}

fn base_bundle(root: &Path, conversation: ConversationId) -> hm_compose::bundle::ActivationBundle {
    let store =
        ProjectionStore::open(root.join("bundle"), 16 * 1024 * 1024).expect("bundle projection");
    let snapshot = store.begin_snapshot().expect("bundle snapshot");
    let counter =
        TokenCounter::for_model("slice4", None, FallbackWeights::default()).expect("token counter");
    activate(
        &snapshot,
        &ActivationRequest {
            actor: ACTOR,
            conversation,
            query: "cached context".to_owned(),
            turn_text: String::new(),
            budget_tokens: 2_048,
            token_counter: &counter,
            maximum_candidates: 64,
            maximum_conversation_records: 64,
        },
    )
    .expect("activation bundle")
}
