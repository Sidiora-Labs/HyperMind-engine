use hm_context::{cache::*, types::{digest_bytes, ContextError, Scope}};
fn fence() -> CacheFence { CacheFence { scope: Scope { owner_id: "owner".into(), project_id: "project".into(), workspace_id: None }, session_id: "session".into(), model_id: "model".into(), policy_revision: "policy-1".into(), source_revision: "source-1".into() } }
fn region(id: &str, ordinal: u64, bytes: &[u8]) -> CacheRegion { CacheRegion { bytes: bytes.to_vec(), sources: vec![CacheSource { id: id.into(), digest: digest_bytes(bytes), ordinal, required: true }] } }
fn cache() -> ContextCache { ContextCache::new(fence(), region("a", 1, b"base\0"), region("b", 2, b"delta\xff"), region("c", 3, b"tail\n")).unwrap() }
fn change(id: &str, kind: RegionKind, original: CacheRegion, bytes: &[u8]) -> PendingChange { PendingChange { id: id.into(), region: kind, source_identity: original.source_identity().unwrap(), replacement: CacheRegion { bytes: bytes.to_vec(), sources: original.sources } } }
#[test]
fn deferred_replay_is_byte_identical_and_queued_changes_commit_together() {
    let mut cache = cache();
    let original = cache.replay(&fence()).unwrap();
    let generation = cache.generation();
    let delta = change("reduce-delta", RegionKind::Delta, region("b", 2, b"delta\xff"), b"reduced-delta");
    cache.queue_change(generation, delta.clone()).unwrap();
    cache.queue_change(generation, delta).unwrap();
    cache.queue_change(generation, change("reduce-base", RegionKind::Baseline, region("a", 1, b"base\0"), b"reduced-base")).unwrap();
    assert_eq!(cache.reconcile(generation, &fence(), BoundaryChange::Defer).unwrap(), generation);
    assert_eq!(cache.replay(&fence()).unwrap(), original);
    cache.reconcile(generation, &fence(), BoundaryChange::SoftFold).unwrap();
    assert_eq!(cache.replay(&fence()).unwrap(), b"reduced-basereduced-deltatail\n");
    let checkpoint = cache.checkpoint().unwrap();
    assert!(checkpoint.live_tail.bytes.is_empty());
    assert_eq!(checkpoint.delta.sources.len(), 2);
    assert!(cache.pending().is_empty());
    assert!(matches!(cache.reconcile(generation, &fence(), BoundaryChange::HardFold), Err(ContextError::Conflict)));
}
#[test]
fn hard_fold_preserves_exact_bytes_and_source_boundaries() {
    let mut cache = cache();
    let bytes = cache.replay(&fence()).unwrap();
    cache.reconcile(1, &fence(), BoundaryChange::HardFold).unwrap();
    assert_eq!(cache.replay(&fence()).unwrap(), bytes);
    let checkpoint = cache.checkpoint().unwrap();
    assert_eq!(checkpoint.baseline.sources.len(), 3);
    assert!(checkpoint.delta.sources.is_empty());
    assert!(checkpoint.live_tail.sources.is_empty());
}
#[test]
fn every_identity_fence_refuses_replay() {
    let cache = cache();
    let mut changed = fence(); changed.scope.owner_id = "other".into();
    assert!(matches!(cache.replay(&changed), Err(ContextError::ScopeMismatch)));
    changed = fence(); changed.session_id = "other".into();
    assert!(matches!(cache.replay(&changed), Err(ContextError::ScopeMismatch)));
    changed = fence(); changed.model_id = "other".into(); assert!(cache.replay(&changed).is_err());
    changed = fence(); changed.policy_revision = "other".into(); assert!(cache.replay(&changed).is_err());
    changed = fence(); changed.source_revision = "other".into(); assert!(cache.replay(&changed).is_err());
}
#[test]
fn safety_invalidation_precedes_serving_and_discards_stale_reductions() {
    for reason in [SafetyInvalidation::AccessRevoked, SafetyInvalidation::RequiredEvidenceCorrected { source_id: "a".into() }] {
        let mut cache = cache();
        cache.queue_change(1, change("pending", RegionKind::Baseline, region("a", 1, b"base\0"), b"old")).unwrap();
        assert_eq!(cache.invalidate(reason).unwrap(), 2);
        assert!(cache.replay(&fence()).is_err());
        assert!(cache.pending().is_empty());
        assert!(cache.reconcile(2, &fence(), BoundaryChange::HardFold).is_err());
        let restored = ContextCache::restore(cache.checkpoint().unwrap(), &fence()).unwrap();
        assert!(restored.replay(&fence()).is_err());
        let mut corrected = fence(); corrected.source_revision = "source-2".into(); corrected.policy_revision = "policy-2".into();
        cache.reconcile(2, &corrected, BoundaryChange::Replace { baseline: region("new", 4, b"current"), delta: CacheRegion::default(), live_tail: CacheRegion::default() }).unwrap();
        assert_eq!(cache.replay(&corrected).unwrap(), b"current");
    }
}
#[test]
fn stale_reductions_and_invalid_boundaries_leave_state_unchanged() {
    let mut cache = cache(); let before = cache.checkpoint().unwrap();
    let mut pending = change("bad", RegionKind::Baseline, region("a", 1, b"base\0"), b"small");
    pending.replacement.sources[0].digest = digest_bytes(b"edited");
    assert!(cache.queue_change(1, pending).is_err());
    assert!(cache.reconcile(1, &fence(), BoundaryChange::Replace { baseline: region("a", 3, b"a"), delta: region("b", 2, b"b"), live_tail: CacheRegion::default() }).is_err());
    assert_eq!(cache.checkpoint().unwrap(), before);
}
#[test]
fn atomic_file_checkpoint_restores_pending_changes_and_rejects_corruption() {
    let temp = tempfile::tempdir().unwrap(); let path = temp.path().join("cache.json");
    let mut cache = cache();
    cache.queue_change(1, change("pending", RegionKind::Delta, region("b", 2, b"delta\xff"), b"summary")).unwrap();
    cache.save(&path).unwrap();
    let mut restored = ContextCache::load(&path, &fence()).unwrap();
    assert_eq!(restored.checkpoint().unwrap(), cache.checkpoint().unwrap());
    restored.reconcile(1, &fence(), BoundaryChange::HardFold).unwrap();
    restored.save(&path).unwrap();
    assert_eq!(ContextCache::load(&path, &fence()).unwrap().replay(&fence()).unwrap(), b"base\0summarytail\n");
    let mut malformed = restored.checkpoint().unwrap(); malformed.generation = 0;
    assert!(ContextCache::restore(malformed, &fence()).is_err());
    let mut malformed = restored.checkpoint().unwrap(); malformed.version += 1; malformed.digest.clear(); malformed.digest = digest_bytes(&serde_json::to_vec(&malformed).unwrap());
    assert!(ContextCache::restore(malformed, &fence()).is_err());
    std::fs::write(&path, b"{\"version\":1,\"unknown\":1}").unwrap();
    assert!(ContextCache::load(&path, &fence()).is_err());
}
