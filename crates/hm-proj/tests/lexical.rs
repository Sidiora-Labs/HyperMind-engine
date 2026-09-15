mod common;

use hm_core::LSN;
use hm_proj::lexical::LexicalProjection;
use hm_proj::rebuild::rebuild_projection_stream;
use hm_proj::store::{ProjectionId, ProjectionStore};

#[test]
fn lexical_postings_reach_exact_terms_and_rank_deterministically() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).expect("open store");
    let frames = common::frames(100);
    rebuild_projection_stream(&store, &frames, true, usize::MAX).expect("rebuild");
    let snapshot = store.begin_snapshot().expect("snapshot");
    let alpha = LexicalProjection::query(&snapshot, "ALPHA durable", 10).expect("alpha query");
    assert_eq!(alpha.len(), 10);
    assert!(alpha.iter().all(|hit| hit.lsn.get() % 2 == 1));
    assert!(
        alpha
            .windows(2)
            .all(|pair| pair[0].score_q32 > pair[1].score_q32
                || (pair[0].score_q32 == pair[1].score_q32 && pair[0].lsn < pair[1].lsn))
    );
    let exact = LexicalProjection::query(&snapshot, "item 42", 5).expect("exact query");
    assert_eq!(exact[0].lsn, LSN::new(43));
    assert_eq!(
        snapshot.checkpoint(ProjectionId::Bm25).expect("checkpoint"),
        LSN::new(100)
    );
}

#[test]
fn lexical_rebuild_is_canonical_byte_identical() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let store = ProjectionStore::open(temporary.path(), 16 * 1024 * 1024).expect("open store");
    let frames = common::frames(48);
    rebuild_projection_stream(&store, &frames, true, usize::MAX).expect("first rebuild");
    let expected = store
        .begin_snapshot()
        .expect("snapshot")
        .canonical_dump(ProjectionId::Bm25)
        .expect("dump");
    store.reset(ProjectionId::Bm25).expect("reset lexical");
    for frame in &frames {
        LexicalProjection::apply_event(&store, frame).expect("apply lexical");
    }
    let rebuilt = store
        .begin_snapshot()
        .expect("snapshot")
        .canonical_dump(ProjectionId::Bm25)
        .expect("dump");
    assert_eq!(rebuilt, expected);
}
