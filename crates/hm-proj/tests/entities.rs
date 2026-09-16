use hm_core::{ActorId, ConversationId, LSN, UtcNanos};
use hm_ledger::frame::{EventKind, Frame, FrameHeader};
use hm_proj::entities::EntityProjection;
use hm_proj::rebuild::rebuild_projection_stream;
use hm_proj::store::{ProjectionId, ProjectionStore};
use hm_schema::event::{REPOSITORY_SNAPSHOT_PROVIDER, encode_event_envelope};
use hm_schema::events::{
    Authority, EventEnvelope, EventPayload, ProviderFrame, Retention, Sensitivity, UserMsg,
};

const SNAPSHOT_CONTENT: &str = concat!(
    "{\"contract\": \"hypermind.repository-graph.v1\", \"repository\": \"hypermind\"}\n",
    "{\"kind\": \"file\", \"name\": \"crates/hm-proj/src/graph.rs\", \"repository\": \"hypermind\"}\n",
    "{\"kind\": \"symbol\", \"name\": \"GraphProjection\", \"repository\": \"hypermind\"}\n"
);

const UNRELATED_CONTENT: &str =
    "{\"wake\": \"UnrelatedBeacon\", \"path\": \"vendor/unrelated/beacon.rs\"}";

fn frame(lsn: u64, kind: EventKind, payload: EventPayload, authority: Authority) -> Frame {
    let envelope = EventEnvelope {
        schema_version: 2,
        payload,
        connection_id: None,
        client_seq: 0,
        client_event_index: 0,
        client_event_count: 1,
        origin_actor: 0,
        run_id: None,
        model_provenance: None,
        authority,
        retention: Retention::CurrentState,
        sensitivity: Sensitivity::Public,
        event_time_ns: 0,
    };
    Frame {
        header: FrameHeader {
            lsn: LSN::new(lsn),
            kind,
            wall_timestamp_ns: UtcNanos::new(9_000_000 + i64::try_from(lsn).expect("timestamp")),
            actor: ActorId::new(11),
            conversation: ConversationId::derive("repository:hypermind"),
        },
        sealed_payload: encode_event_envelope(&envelope),
    }
}

fn provider_frame(lsn: u64, provider: &str, api_content: Vec<u8>) -> Frame {
    frame(
        lsn,
        EventKind::ProviderFrame,
        EventPayload::ProviderFrame(Box::new(ProviderFrame {
            provider: provider.to_owned(),
            api_content,
        })),
        Authority::ExternalObserved,
    )
}

fn frames() -> Vec<Frame> {
    vec![
        frame(
            1,
            EventKind::UserMsg,
            EventPayload::UserMsg(Box::new(UserMsg {
                content: b"index the repository snapshot please".to_vec(),
            })),
            Authority::UserAsserted,
        ),
        provider_frame(
            2,
            REPOSITORY_SNAPSHOT_PROVIDER,
            SNAPSHOT_CONTENT.as_bytes().to_vec(),
        ),
        provider_frame(3, "filesystem", UNRELATED_CONTENT.as_bytes().to_vec()),
    ]
}

fn frames_with_invalid_snapshot() -> Vec<Frame> {
    let mut invalid = b"{\"name\": \"vendor/invalid/marker.rs\"}".to_vec();
    invalid.push(0xff);
    let mut frames = frames();
    frames.push(provider_frame(4, REPOSITORY_SNAPSHOT_PROVIDER, invalid));
    frames
}

#[test]
fn repository_snapshot_frames_enter_the_entity_index() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).expect("open store");
    let frames = frames();
    rebuild_projection_stream(&store, &frames, true, usize::MAX).expect("rebuild");
    let snapshot = store.begin_snapshot().expect("snapshot");
    let path = EntityProjection::query(&snapshot, "crates/hm-proj/src/graph.rs", "", 10)
        .expect("path query");
    assert_eq!(
        path.iter().map(|hit| hit.lsn).collect::<Vec<_>>(),
        vec![LSN::new(2)]
    );
    let symbol =
        EntityProjection::query(&snapshot, "GraphProjection", "", 10).expect("symbol query");
    assert_eq!(
        symbol.iter().map(|hit| hit.lsn).collect::<Vec<_>>(),
        vec![LSN::new(2)]
    );
    assert!(
        path.iter()
            .chain(symbol.iter())
            .all(|hit| hit.lsn != LSN::new(3))
    );
}

#[test]
fn non_snapshot_provider_frames_are_skipped_without_stalling() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).expect("open store");
    let frames = frames_with_invalid_snapshot();
    rebuild_projection_stream(&store, &frames, true, usize::MAX).expect("rebuild");
    let snapshot = store.begin_snapshot().expect("snapshot");
    assert_eq!(
        snapshot
            .checkpoint(ProjectionId::EntityIndex)
            .expect("checkpoint"),
        LSN::new(u64::try_from(frames.len()).expect("frame count"))
    );
    assert!(
        EntityProjection::query(&snapshot, "UnrelatedBeacon", "", 10)
            .expect("unrelated query")
            .is_empty()
    );
    assert!(
        EntityProjection::query(&snapshot, "vendor/unrelated/beacon.rs", "", 10)
            .expect("unrelated path query")
            .is_empty()
    );
    assert!(
        EntityProjection::query(&snapshot, "vendor/invalid/marker.rs", "", 10)
            .expect("invalid snapshot query")
            .is_empty()
    );
}

#[test]
fn entity_rebuild_is_canonical_byte_identical() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).expect("open store");
    let frames = frames_with_invalid_snapshot();
    rebuild_projection_stream(&store, &frames, true, usize::MAX).expect("rebuild");
    let expected = store
        .begin_snapshot()
        .expect("snapshot")
        .canonical_dump(ProjectionId::EntityIndex)
        .expect("dump");
    store.reset(ProjectionId::EntityIndex).expect("reset");
    for frame in &frames {
        EntityProjection::apply_event(&store, frame).expect("apply entities");
    }
    let rebuilt = store
        .begin_snapshot()
        .expect("snapshot")
        .canonical_dump(ProjectionId::EntityIndex)
        .expect("dump");
    assert_eq!(rebuilt, expected);
}
