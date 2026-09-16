#![forbid(unsafe_code)]

use hm_core::{ActorId, ConversationId, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_proj::media::{
    DESCRIPTION_KIND_BYTE, MediaCatalogProjection, TRANSCRIPT_KIND_BYTE, derivation_call_id,
};
use hm_proj::rebuild::rebuild_projection_stream;
use hm_proj::store::{ProjectionId, ProjectionStore};
use hm_schema::event::encode_event_envelope;
use hm_schema::events::{
    Authority, EventEnvelope, EventPayload, MediaRef, ModelProvenance, ProviderFrame, Retention,
    Sensitivity, UserMsg,
};

const SOURCE_URI: &str = "https://example.test/notes/meeting.png";
const SOURCE_MEDIA_TYPE: &str = "image/png";
const SOURCE_BYTES: &[u8] = b"hypermind media catalog fixture bytes";
const OTHER_BYTES: &[u8] = b"a provider frame that belongs to nothing";

fn conversation(byte: u8) -> ConversationId {
    let mut raw = [0_u8; 16];
    raw[0] = byte;
    raw[1] = 0x6d;
    ConversationId::new(raw)
}

fn frame(
    lsn: u64,
    kind: EventKind,
    conversation: ConversationId,
    envelope: &EventEnvelope,
) -> Frame {
    Frame {
        header: FrameHeader {
            lsn: LSN::new(lsn),
            kind,
            wall_timestamp_ns: UtcNanos::new(7_000_000 + i64::try_from(lsn).expect("timestamp")),
            actor: ActorId::new(41),
            conversation,
        },
        sealed_payload: encode_event_envelope(envelope),
    }
}

fn envelope(payload: EventPayload, authority: Authority) -> EventEnvelope {
    EventEnvelope {
        schema_version: 2,
        payload,
        connection_id: None,
        client_seq: 0,
        client_event_index: 0,
        client_event_count: 0,
        origin_actor: 0,
        run_id: None,
        model_provenance: None,
        authority,
        retention: Retention::CurrentState,
        sensitivity: Sensitivity::Public,
        event_time_ns: 0,
    }
}

fn media_ref_frame(lsn: u64, digest: &[u8; 32]) -> Frame {
    let payload = EventPayload::MediaRef(Box::new(MediaRef {
        uri: SOURCE_URI.to_owned(),
        media_type: SOURCE_MEDIA_TYPE.to_owned(),
        digest: digest.to_vec(),
    }));
    frame(
        lsn,
        EventKind::MediaRef,
        conversation(1),
        &envelope(payload, Authority::ExternalObserved),
    )
}

fn provider_frame(lsn: u64, bytes: &[u8]) -> Frame {
    let payload = EventPayload::ProviderFrame(Box::new(ProviderFrame {
        provider: "hm-web-source@1".to_owned(),
        api_content: bytes.to_vec(),
    }));
    frame(
        lsn,
        EventKind::ProviderFrame,
        conversation(2),
        &envelope(payload, Authority::ExternalObserved),
    )
}

fn derived_frame(lsn: u64, call_id: &[u8; 32], prompt_id: &str) -> Frame {
    let payload = EventPayload::UserMsg(Box::new(UserMsg {
        content: b"a derived description of the attached image".to_vec(),
    }));
    let mut value = envelope(payload, Authority::DerivedInference);
    value.model_provenance = Some(Box::new(ModelProvenance {
        model_id: "centra/structured".to_owned(),
        prompt_id: prompt_id.to_owned(),
        prompt_version: 1,
        temperature: 0.0,
        call_id: Some(call_id.to_vec()),
        input_tokens: 0,
        output_tokens: 0,
        cache_read_tokens: 0,
        cache_write_tokens: 0,
        cost_microusd: 0,
    }));
    frame(lsn, EventKind::UserMsg, conversation(1), &value)
}

fn digest() -> [u8; 32] {
    *blake3::hash(SOURCE_BYTES).as_bytes()
}

fn open_store(path: &std::path::Path) -> ProjectionStore {
    ProjectionStore::open(path, 16 * 1024 * 1024).expect("open store")
}

fn dump(store: &ProjectionStore) -> Vec<u8> {
    store
        .begin_snapshot()
        .expect("snapshot")
        .canonical_dump(ProjectionId::MediaCatalog)
        .expect("media catalog dump")
}

#[test]
fn media_catalog_records_source_retention_and_derivation() {
    let digest = digest();
    let frames = vec![
        media_ref_frame(1, &digest),
        provider_frame(2, SOURCE_BYTES),
        derived_frame(
            3,
            &derivation_call_id(&digest, DESCRIPTION_KIND_BYTE),
            "media-description",
        ),
    ];
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = open_store(temporary.path());
    let progress = rebuild_projection_stream(&store, &frames, true, usize::MAX).expect("rebuild");
    assert!(progress.complete);

    let snapshot = store.begin_snapshot().expect("snapshot");
    let record = MediaCatalogProjection::get(&snapshot, &digest)
        .expect("record read")
        .expect("record present");
    assert_eq!(record.digest, digest.to_vec());
    assert_eq!(record.uri, SOURCE_URI);
    assert_eq!(record.media_type, SOURCE_MEDIA_TYPE);
    assert_eq!(record.media_ref_lsn, 1);
    assert_eq!(record.retained_lsn, 2);
    assert_eq!(record.derived_lsn, 3);
    assert_eq!(record.derived_prompt_id, "media-description");
    assert_eq!(
        MediaCatalogProjection::list(&snapshot, 16).expect("list"),
        vec![record]
    );
    assert!(
        MediaCatalogProjection::pending(&snapshot, 16)
            .expect("pending")
            .is_empty()
    );
    assert_eq!(
        snapshot
            .checkpoint(ProjectionId::MediaCatalog)
            .expect("checkpoint"),
        LSN::new(3)
    );
}

#[test]
fn media_without_a_derivation_stays_pending() {
    let digest = digest();
    let frames = vec![media_ref_frame(1, &digest), provider_frame(2, SOURCE_BYTES)];
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = open_store(temporary.path());
    rebuild_projection_stream(&store, &frames, true, usize::MAX).expect("rebuild");

    let snapshot = store.begin_snapshot().expect("snapshot");
    let pending = MediaCatalogProjection::pending(&snapshot, 16).expect("pending");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].digest, digest.to_vec());
    assert_eq!(pending[0].retained_lsn, 2);
    assert_eq!(pending[0].derived_lsn, 0);
    assert!(pending[0].derived_prompt_id.is_empty());
    assert!(MediaCatalogProjection::list(&snapshot, 0).is_err());
    assert!(MediaCatalogProjection::pending(&snapshot, 0).is_err());
}

#[test]
fn unknown_call_ids_and_unrelated_provider_frames_apply_nothing() {
    let digest = digest();
    let unrelated = *blake3::hash(OTHER_BYTES).as_bytes();
    let settled = vec![media_ref_frame(1, &digest), provider_frame(2, SOURCE_BYTES)];
    let mut extended = settled.clone();
    extended.push(derived_frame(
        3,
        &derivation_call_id(&unrelated, TRANSCRIPT_KIND_BYTE),
        "media-transcript",
    ));
    extended.push(provider_frame(4, OTHER_BYTES));

    let temporary = tempfile::tempdir().expect("temporary directory");
    let settled_store = open_store(&temporary.path().join("settled"));
    rebuild_projection_stream(&settled_store, &settled, true, usize::MAX).expect("settled rebuild");
    let extended_store = open_store(&temporary.path().join("extended"));
    rebuild_projection_stream(&extended_store, &extended, true, usize::MAX)
        .expect("extended rebuild");

    let settled_dump = dump(&settled_store);
    let extended_dump = dump(&extended_store);
    assert_eq!(settled_dump[8..], extended_dump[8..]);
    let snapshot = extended_store.begin_snapshot().expect("snapshot");
    assert_eq!(
        snapshot
            .checkpoint(ProjectionId::MediaCatalog)
            .expect("checkpoint"),
        LSN::new(4)
    );
    let record = MediaCatalogProjection::get(&snapshot, &digest)
        .expect("record read")
        .expect("record present");
    assert_eq!(record.derived_lsn, 0);
    assert_eq!(record.retained_lsn, 2);
}

#[test]
fn rebuild_is_byte_identical() {
    let digest = digest();
    let frames = vec![
        media_ref_frame(1, &digest),
        provider_frame(2, SOURCE_BYTES),
        derived_frame(
            3,
            &derivation_call_id(&digest, TRANSCRIPT_KIND_BYTE),
            "media-transcript",
        ),
    ];
    let temporary = tempfile::tempdir().expect("temporary directory");
    let first = open_store(&temporary.path().join("first"));
    rebuild_projection_stream(&first, &frames, true, usize::MAX).expect("first rebuild");
    let second = open_store(&temporary.path().join("second"));
    rebuild_projection_stream(&second, &frames, true, usize::MAX).expect("second rebuild");
    assert_eq!(dump(&first), dump(&second));

    let resumed = open_store(&temporary.path().join("resumed"));
    rebuild_projection_stream(&resumed, &frames, true, 2).expect("partial rebuild");
    assert_ne!(dump(&resumed), dump(&first));
    rebuild_projection_stream(&resumed, &frames, false, usize::MAX).expect("resumed rebuild");
    assert_eq!(dump(&resumed), dump(&first));
}

#[test]
fn projection_identity_is_stable() {
    assert_eq!(ProjectionId::COUNT, 22);
    assert_eq!(ProjectionId::MediaCatalog.name(), "media_catalog");
    assert_eq!(ProjectionId::MediaCatalog as usize, ProjectionId::COUNT - 1);
    for (projection, name) in [
        (ProjectionId::BeliefStore, "belief_store"),
        (ProjectionId::EntityIndex, "entity_index"),
        (ProjectionId::VectorLane, "vector_lane"),
        (ProjectionId::Bm25, "bm25"),
        (ProjectionId::TemporalLadder, "temporal_ladder"),
        (ProjectionId::IntentFrame, "intent_frame"),
        (ProjectionId::WorkLedger, "work_ledger"),
        (ProjectionId::ConversationHeads, "conversation_heads"),
        (ProjectionId::Bindings, "bindings"),
        (ProjectionId::Memories, "memories"),
        (ProjectionId::Graph, "graph"),
        (ProjectionId::Fsrs, "fsrs"),
        (ProjectionId::Runs, "runs"),
        (ProjectionId::Intentions, "intentions"),
        (ProjectionId::AttentionHistory, "attention_history"),
        (ProjectionId::Predictions, "predictions"),
        (ProjectionId::Procedures, "procedures"),
        (ProjectionId::Attestations, "attestations"),
        (ProjectionId::Vocabulary, "vocabulary"),
        (ProjectionId::Documents, "documents"),
        (ProjectionId::SourceConnectors, "source_connectors"),
    ] {
        assert_eq!(projection.name(), name);
    }
}
