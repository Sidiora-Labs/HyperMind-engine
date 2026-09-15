mod common;

use hm_core::LSN;
use hm_ledger::frame::EventKind;
use hm_proj::rebuild::rebuild_projection_stream;
use hm_proj::store::{ProjectionId, ProjectionStore};
use hm_proj::timeline::{
    read_conversation_record, read_conversation_records, scan_latest_conversation_record_of_kind,
};

#[test]
fn timeline_reads_by_conversation_lsn_and_latest_kind() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).expect("open store");
    let frames = common::frames(80);
    let progress = rebuild_projection_stream(&store, &frames, true, usize::MAX).expect("rebuild");
    assert!(progress.complete);
    let snapshot = store.begin_snapshot().expect("snapshot");
    assert_eq!(
        snapshot
            .checkpoint(ProjectionId::ConversationHeads)
            .expect("checkpoint"),
        LSN::new(80)
    );
    let conversation = frames[0].header.conversation;
    let records = read_conversation_records(&snapshot, conversation, 100).expect("conversation");
    assert_eq!(
        records
            .iter()
            .map(|record| record.lsn.get())
            .collect::<Vec<_>>(),
        [1, 14, 27, 40, 53, 66, 79]
    );
    let exact = read_conversation_record(&snapshot, LSN::new(27))
        .expect("exact")
        .expect("record");
    assert_eq!(exact.conversation, conversation);
    assert_eq!(exact.kind, EventKind::UserMsg);
    let latest = scan_latest_conversation_record_of_kind(
        &snapshot,
        conversation,
        EventKind::DeliveredMsg,
        LSN::new(81),
        100,
    )
    .expect("latest scan");
    assert_eq!(latest.record.expect("latest delivered").lsn, LSN::new(66));
}

#[test]
fn selective_reset_and_rebuild_is_byte_identical() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).expect("open store");
    let frames = common::frames(64);
    rebuild_projection_stream(&store, &frames, true, usize::MAX).expect("first rebuild");
    let expected = store
        .begin_snapshot()
        .expect("snapshot")
        .canonical_dump(ProjectionId::ConversationHeads)
        .expect("dump");
    store
        .reset(ProjectionId::ConversationHeads)
        .expect("reset timeline");
    for frame in &frames {
        hm_proj::timeline::apply_timeline(&store, frame).expect("apply timeline");
    }
    let rebuilt = store
        .begin_snapshot()
        .expect("snapshot")
        .canonical_dump(ProjectionId::ConversationHeads)
        .expect("dump");
    assert_eq!(rebuilt, expected);
}
