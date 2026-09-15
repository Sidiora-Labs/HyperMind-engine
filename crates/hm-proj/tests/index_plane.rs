#![forbid(unsafe_code)]

use hm_core::{ActorId, ConversationId, ErrorCode, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_proj::entities::EntityProjection;
use hm_proj::spaces::{SpaceCatalog, SpaceDefinition};
use hm_proj::store::{ProjectionId, ProjectionStore};
use hm_proj::vectors::VectorLane;
use hm_schema::event::encode_event_envelope;
use hm_schema::events::{Authority, EventEnvelope, EventPayload, Retention, Sensitivity, UserMsg};

#[test]
fn vector_file_prefilters_then_scores_and_reopens_canonically() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let lane =
        VectorLane::open(temporary.path(), "generation-a", "space-a", 4).expect("open vector lane");
    lane.append(LSN::new(1), &[10, 0, 0, 0], &[0b0000_0001])
        .expect("append first");
    lane.append(LSN::new(2), &[8, 8, 0, 0], &[0b0000_0011])
        .expect("append second");
    lane.append(LSN::new(3), &[127, 127, 0, 0], &[0b0000_1100])
        .expect("append distant prefilter");

    let hits = lane
        .search(&[10, 10, 0, 0], &[0b0000_0011], 2, 2)
        .expect("search");
    assert_eq!(
        hits.iter().map(|hit| hit.target_lsn).collect::<Vec<_>>(),
        [LSN::new(2), LSN::new(1)]
    );
    assert!(hits[0].score > hits[1].score);
    let canonical = lane.canonical_bytes().expect("canonical bytes");
    drop(lane);
    let reopened = VectorLane::open(temporary.path(), "generation-a", "space-a", 4)
        .expect("reopen vector lane");
    assert_eq!(reopened.validate().expect("valid records"), 3);
    assert_eq!(
        reopened.canonical_bytes().expect("canonical bytes"),
        canonical
    );
    let mut corrupted = canonical;
    *corrupted.last_mut().expect("record checksum") ^= 1;
    std::fs::write(reopened.path(), corrupted).expect("inject checksum corruption");
    assert_eq!(
        reopened.validate().expect_err("corrupt vector record").code,
        ErrorCode::ChecksumMismatch
    );
}

#[test]
fn equal_dimension_encoders_switch_atomically_and_retain_the_prior_generation() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let catalog = SpaceCatalog::open(temporary.path().join("spaces")).expect("space catalog");
    let vector_root = temporary.path().join("vectors");
    let first = definition("generation-a", "space-a", "encoder-a");
    let second = definition("generation-b", "space-b", "encoder-b");
    catalog.create(&first).expect("first space");
    catalog.create(&second).expect("second space");
    let first_lane = VectorLane::open(&vector_root, &first.generation_id, &first.space_id, 4)
        .expect("first lane");
    first_lane
        .append(LSN::new(1), &[1, 2, 3, 4], &[0b0000_1111])
        .expect("first embedding");
    catalog
        .validate_generation(&first.generation_id, &first_lane)
        .expect("validate first");
    catalog
        .activate(&first.generation_id, &first_lane)
        .expect("activate first");

    let second_lane = VectorLane::open(&vector_root, &second.generation_id, &second.space_id, 4)
        .expect("second lane");
    second_lane
        .append(LSN::new(1), &[4, 3, 2, 1], &[0b0000_1111])
        .expect("second embedding");
    assert_eq!(
        catalog
            .activate(&second.generation_id, &second_lane)
            .expect_err("unvalidated generation cannot activate")
            .code,
        ErrorCode::VectorIndexCorrupt
    );
    catalog
        .validate_generation(&second.generation_id, &second_lane)
        .expect("validate second");
    second_lane
        .append(LSN::new(2), &[3, 2, 1, 0], &[0b0000_0111])
        .expect("append after validation");
    assert_eq!(
        catalog
            .activate(&second.generation_id, &second_lane)
            .expect_err("changed generation requires revalidation")
            .code,
        ErrorCode::VectorIndexCorrupt
    );
    catalog
        .validate_generation(&second.generation_id, &second_lane)
        .expect("revalidate second");
    catalog
        .activate(&second.generation_id, &second_lane)
        .expect("activate second");
    assert_eq!(
        catalog.active().expect("active"),
        Some("generation-b".to_owned())
    );
    assert_eq!(
        catalog.previous_compatible().expect("prior generation"),
        Some("generation-a".to_owned())
    );
    assert!(first_lane.path().exists());
    assert_ne!(first.encoder_id, second.encoder_id);
    assert_eq!(first.dimensions, second.dimensions);
}

#[test]
fn entity_postings_cover_all_rules_and_rebuild_byte_identically() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).expect("store");
    let frames = [
        frame(
            1,
            "Alice Smith deployed GH-123 at https://www.Example.com/docs into src/main.rs with 0xdeadbeef",
        ),
        frame(
            2,
            "Bob Jones reviewed EX-77 at example.net in docs/design.md",
        ),
    ];
    for frame in &frames {
        EntityProjection::apply_event(&store, frame).expect("apply entity projection");
    }
    let snapshot = store.begin_snapshot().expect("snapshot");
    let canonical = snapshot
        .canonical_dump(ProjectionId::EntityIndex)
        .expect("canonical entity bytes");
    for (query, turn) in [
        ("GH-123", ""),
        ("example.com", ""),
        ("src/main.rs", ""),
        ("0xdeadbeef", ""),
        ("Who deployed it?", "Alice Smith"),
    ] {
        let hits = EntityProjection::query(&snapshot, query, turn, 5).expect("entity query");
        assert_eq!(hits.first().expect("entity hit").lsn, LSN::new(1));
    }
    drop(snapshot);

    store
        .reset(ProjectionId::EntityIndex)
        .expect("reset entities");
    for frame in &frames {
        EntityProjection::apply_event(&store, frame).expect("rebuild entities");
    }
    assert_eq!(
        store
            .begin_snapshot()
            .expect("snapshot")
            .canonical_dump(ProjectionId::EntityIndex)
            .expect("rebuilt bytes"),
        canonical
    );
}

fn definition(generation_id: &str, space_id: &str, encoder_id: &str) -> SpaceDefinition {
    SpaceDefinition {
        generation_id: generation_id.to_owned(),
        space_id: space_id.to_owned(),
        encoder_id: encoder_id.to_owned(),
        revision: "revision-1".to_owned(),
        dimensions: 4,
        distance: "dot".to_owned(),
        normalization: "l2".to_owned(),
        input_role: "document".to_owned(),
    }
}

fn frame(lsn: u64, text: &str) -> Frame {
    let envelope = EventEnvelope {
        schema_version: 2,
        payload: EventPayload::UserMsg(Box::new(UserMsg {
            content: text.as_bytes().to_vec(),
        })),
        connection_id: None,
        client_seq: 0,
        client_event_index: 0,
        client_event_count: 0,
        origin_actor: 0,
        run_id: None,
        model_provenance: None,
        authority: Authority::UserAsserted,
        retention: Retention::CurrentState,
        sensitivity: Sensitivity::Public,
        event_time_ns: 0,
    };
    Frame {
        header: FrameHeader {
            lsn: LSN::new(lsn),
            kind: EventKind::UserMsg,
            wall_timestamp_ns: UtcNanos::new(i64::try_from(lsn).expect("timestamp")),
            actor: ActorId::new(1),
            conversation: ConversationId::new([1; 16]),
        },
        sealed_payload: encode_event_envelope(&envelope),
    }
}
