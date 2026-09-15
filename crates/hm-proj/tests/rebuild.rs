mod common;

use hm_core::LSN;
use hm_proj::lexical::LexicalProjection;
use hm_proj::rebuild::rebuild_projection_stream;
use hm_proj::store::{ProjectionId, ProjectionStore};
use hm_proj::timeline::apply_timeline;
use std::process::Command;

#[test]
fn rebuild_resumes_from_minimum_checkpoint() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).expect("open store");
    let frames = common::frames(20);
    for frame in &frames[..7] {
        apply_timeline(&store, frame).expect("timeline prefix");
    }
    for frame in &frames[..3] {
        LexicalProjection::apply_event(&store, frame).expect("lexical prefix");
    }
    let progress = rebuild_projection_stream(&store, &frames, false, usize::MAX).expect("resume");
    assert!(progress.complete);
    let snapshot = store.begin_snapshot().expect("snapshot");
    assert_eq!(
        snapshot
            .checkpoint(ProjectionId::ConversationHeads)
            .expect("timeline checkpoint"),
        LSN::new(20)
    );
    assert_eq!(
        snapshot
            .checkpoint(ProjectionId::Bm25)
            .expect("lexical checkpoint"),
        LSN::new(20)
    );
}

#[test]
fn sigkill_mid_rebuild_resumes_to_canonical_bytes() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let expected_path = temporary.path().join("expected");
    let actor_path = temporary.path().join("killed");
    let frames = common::frames(96);
    let expected_store =
        ProjectionStore::open(&expected_path, 16 * 1024 * 1024).expect("expected store");
    rebuild_projection_stream(&expected_store, &frames, true, usize::MAX)
        .expect("expected rebuild");
    let expected_timeline = expected_store
        .begin_snapshot()
        .expect("snapshot")
        .canonical_dump(ProjectionId::ConversationHeads)
        .expect("timeline dump");
    let expected_lexical = expected_store
        .begin_snapshot()
        .expect("snapshot")
        .canonical_dump(ProjectionId::Bm25)
        .expect("lexical dump");

    let status = Command::new(std::env::current_exe().expect("test executable"))
        .arg("--exact")
        .arg("sigkill_rebuild_child")
        .arg("--nocapture")
        .env("HM_REBUILD_ACTOR", &actor_path)
        .status()
        .expect("run rebuild child");
    assert!(!status.success());
    let store = ProjectionStore::open(&actor_path, 16 * 1024 * 1024).expect("post-kill open");
    let snapshot = store.begin_snapshot().expect("post-kill snapshot");
    assert_eq!(
        snapshot
            .checkpoint(ProjectionId::ConversationHeads)
            .expect("timeline checkpoint"),
        LSN::new(37)
    );
    assert_eq!(
        snapshot
            .checkpoint(ProjectionId::Bm25)
            .expect("lexical checkpoint"),
        LSN::new(37)
    );
    drop(snapshot);
    rebuild_projection_stream(&store, &frames, false, usize::MAX).expect("resume after kill");
    let snapshot = store.begin_snapshot().expect("final snapshot");
    assert_eq!(
        snapshot
            .canonical_dump(ProjectionId::ConversationHeads)
            .expect("timeline dump"),
        expected_timeline
    );
    assert_eq!(
        snapshot
            .canonical_dump(ProjectionId::Bm25)
            .expect("lexical dump"),
        expected_lexical
    );
}

#[test]
fn sigkill_rebuild_child() {
    let Ok(actor_path) = std::env::var("HM_REBUILD_ACTOR") else {
        return;
    };
    let store = ProjectionStore::open(actor_path, 16 * 1024 * 1024).expect("child store");
    let frames = common::frames(96);
    let progress = rebuild_projection_stream(&store, &frames, true, 37).expect("child rebuild");
    assert_eq!(progress.applied_lsn, LSN::new(37));
    let status = Command::new("kill")
        .arg("-9")
        .arg(std::process::id().to_string())
        .status();
    panic!("SIGKILL failed: {status:?}");
}
