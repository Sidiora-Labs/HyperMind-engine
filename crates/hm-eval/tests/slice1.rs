use hm_eval::slice1::{probe_indices, seed_store};
use hm_serve::actor::RecallRequest;

#[tokio::test]
async fn seeded_store_has_retrievable_held_out_anchors() {
    let temporary = tempfile::tempdir().unwrap();
    let actor = seed_store(temporary.path(), 100, 0x5eed).await.unwrap();
    let probe = probe_indices(100, 0x5eed)[0];
    let recalled = actor
        .recall(RecallRequest::Lexical {
            query: format!("anchor{probe}hypermind"),
            limit: 10,
        })
        .await
        .unwrap();
    assert!(
        recalled
            .iter()
            .any(|item| item.lsn.get() == probe as u64 + 1)
    );
    actor.shutdown().await.unwrap();
}
