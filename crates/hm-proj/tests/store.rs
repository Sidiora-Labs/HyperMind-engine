use hm_core::{ErrorCode, LSN};
use hm_proj::store::{Mutation, ProjectionId, ProjectionStore};

#[test]
fn checkpoints_mutations_and_ordered_scans_are_transactional() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).expect("open store");
    for projection in [
        ProjectionId::BeliefStore,
        ProjectionId::EntityIndex,
        ProjectionId::VectorLane,
        ProjectionId::Bm25,
        ProjectionId::TemporalLadder,
        ProjectionId::IntentFrame,
        ProjectionId::WorkLedger,
        ProjectionId::ConversationHeads,
    ] {
        assert_eq!(
            store
                .begin_snapshot()
                .expect("snapshot")
                .checkpoint(projection)
                .expect("checkpoint"),
            LSN::new(0)
        );
    }

    store
        .apply(
            ProjectionId::EntityIndex,
            LSN::new(1),
            &[
                Mutation::put(b"ab1", b"one"),
                Mutation::put(b"ab2", b"two"),
                Mutation::put(b"ab3", b"three"),
                Mutation::put(b"ac1", b"other"),
            ],
        )
        .expect("apply");
    let snapshot = store.begin_snapshot().expect("snapshot");
    assert_eq!(
        snapshot
            .get(ProjectionId::EntityIndex, b"ab2")
            .expect("get"),
        Some(b"two".to_vec())
    );
    assert_eq!(
        snapshot
            .first_at_or_after(ProjectionId::EntityIndex, b"ab15")
            .expect("first")
            .expect("item")
            .key,
        b"ab2"
    );
    let forward = snapshot
        .scan_prefix(ProjectionId::EntityIndex, b"ab", 8)
        .expect("prefix");
    assert_eq!(
        forward
            .iter()
            .map(|item| item.key.as_slice())
            .collect::<Vec<_>>(),
        [b"ab1", b"ab2", b"ab3"]
    );
    let reverse = snapshot
        .scan_prefix_reverse(ProjectionId::EntityIndex, b"ab", 2)
        .expect("reverse");
    assert_eq!(
        reverse
            .iter()
            .map(|item| item.key.as_slice())
            .collect::<Vec<_>>(),
        [b"ab3", b"ab2"]
    );
    let before = snapshot
        .scan_prefix_reverse_before(ProjectionId::EntityIndex, b"ab", b"ab3", 8)
        .expect("before");
    assert_eq!(
        before
            .iter()
            .map(|item| item.key.as_slice())
            .collect::<Vec<_>>(),
        [b"ab2", b"ab1"]
    );
    assert_eq!(
        snapshot
            .checkpoint(ProjectionId::EntityIndex)
            .expect("checkpoint"),
        LSN::new(1)
    );

    assert_eq!(
        store
            .apply(ProjectionId::EntityIndex, LSN::new(3), &[])
            .expect_err("skipped checkpoint")
            .code,
        ErrorCode::ProjectionCheckpoint
    );
    assert_eq!(
        store
            .apply(ProjectionId::BeliefStore, LSN::new(1), &[])
            .expect_err("belief gate")
            .code,
        ErrorCode::BeliefWriteGate
    );
}

#[test]
fn read_snapshot_is_pinned_while_writer_advances() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).expect("open store");
    store
        .apply(
            ProjectionId::EntityIndex,
            LSN::new(1),
            &[Mutation::put(b"key", b"old")],
        )
        .expect("first write");
    std::thread::scope(|scope| {
        let reader = scope.spawn(|| {
            let snapshot = store.begin_snapshot().expect("pinned snapshot");
            let before = snapshot
                .canonical_dump(ProjectionId::EntityIndex)
                .expect("before");
            std::thread::sleep(std::time::Duration::from_millis(50));
            let after = snapshot
                .canonical_dump(ProjectionId::EntityIndex)
                .expect("after");
            assert_eq!(before, after);
        });
        std::thread::sleep(std::time::Duration::from_millis(10));
        store
            .apply(
                ProjectionId::EntityIndex,
                LSN::new(2),
                &[Mutation::put(b"key", b"new")],
            )
            .expect("second write");
        reader.join().expect("reader thread");
    });
    assert_eq!(
        store
            .begin_snapshot()
            .expect("snapshot")
            .get(ProjectionId::EntityIndex, b"key")
            .expect("get"),
        Some(b"new".to_vec())
    );
}
