use hm_core::ErrorCode;
use hm_serve::leases::{LeaseClass, LeaseLimits, LeaseRegistry, LeaseSnapshot};
use std::collections::BTreeSet;
use std::time::Duration;
use tokio::time::Instant;

#[tokio::test]
async fn actor_leases_bound_distinct_actors_and_pin_the_admitted() {
    let registry = LeaseRegistry::new(LeaseLimits {
        maximum_active_actors: 1,
        maximum_heavy_jobs: 1,
        wait_ms: 0,
    });
    let first = registry.acquire(7, LeaseClass::Actor).await.unwrap();
    assert_eq!(first.actor(), 7);
    assert_eq!(first.class(), LeaseClass::Actor);
    let refused = registry.acquire(8, LeaseClass::Actor).await.err().unwrap();
    assert_eq!(refused.code, ErrorCode::CapacityExceeded);
    let second = registry.acquire(7, LeaseClass::Actor).await.unwrap();
    assert!(registry.is_pinned(7));
    assert!(!registry.is_pinned(8));
    assert_eq!(registry.active_actors(), BTreeSet::from([7]));
    assert_eq!(registry.snapshot().active_leases, 2);
    drop(first);
    assert!(registry.is_pinned(7));
    assert_eq!(registry.snapshot().active_leases, 1);
    drop(second);
    assert!(!registry.is_pinned(7));
    assert_eq!(registry.active_actors(), BTreeSet::new());
    let granted = registry.acquire(8, LeaseClass::Actor).await.unwrap();
    assert_eq!(granted.actor(), 8);
    assert_eq!(registry.active_actors(), BTreeSet::from([8]));
    assert_eq!(registry.snapshot().refusals, 1);
}

#[tokio::test]
async fn heavy_jobs_are_bounded_independently_of_actor_slots() {
    let registry = LeaseRegistry::new(LeaseLimits {
        maximum_active_actors: 4,
        maximum_heavy_jobs: 1,
        wait_ms: 0,
    });
    let heavy = registry.acquire(7, LeaseClass::HeavyJob).await.unwrap();
    assert_eq!(heavy.class(), LeaseClass::HeavyJob);
    let refused = registry
        .acquire(8, LeaseClass::HeavyJob)
        .await
        .err()
        .unwrap();
    assert_eq!(refused.code, ErrorCode::CapacityExceeded);
    assert!(!registry.is_pinned(8));
    let ordinary = registry.acquire(8, LeaseClass::Actor).await.unwrap();
    assert_eq!(ordinary.class(), LeaseClass::Actor);
    let snapshot = registry.snapshot();
    assert_eq!(snapshot.heavy_jobs, 1);
    assert_eq!(snapshot.active_actors, 2);
    assert_eq!(snapshot.active_leases, 2);
    drop(ordinary);
    drop(heavy);
    assert_eq!(
        registry.snapshot(),
        LeaseSnapshot {
            active_actors: 0,
            active_leases: 0,
            heavy_jobs: 0,
            refusals: 1,
        }
    );
}

#[tokio::test]
async fn a_bounded_wait_expires_rather_than_queueing_forever() {
    let registry = LeaseRegistry::new(LeaseLimits {
        maximum_active_actors: 1,
        maximum_heavy_jobs: 1,
        wait_ms: 60,
    });
    let held = registry.acquire(7, LeaseClass::Actor).await.unwrap();
    let started = Instant::now();
    let refused = registry.acquire(8, LeaseClass::Actor).await.err().unwrap();
    assert_eq!(refused.code, ErrorCode::CapacityExceeded);
    assert!(started.elapsed() >= Duration::from_millis(50));
    drop(held);
    assert!(registry.acquire(8, LeaseClass::Actor).await.is_ok());
}

#[tokio::test]
async fn a_zero_limit_serialises_rather_than_deadlocking_and_the_snapshot_returns_to_zero() {
    let registry = LeaseRegistry::new(LeaseLimits {
        maximum_active_actors: 0,
        maximum_heavy_jobs: 0,
        wait_ms: 0,
    });
    assert_eq!(registry.limits().maximum_active_actors, 1);
    assert_eq!(registry.limits().maximum_heavy_jobs, 1);
    let heavy = registry.acquire(7, LeaseClass::HeavyJob).await.unwrap();
    assert_eq!(registry.snapshot().heavy_jobs, 1);
    let refused = registry.acquire(8, LeaseClass::Actor).await.err().unwrap();
    assert_eq!(refused.code, ErrorCode::CapacityExceeded);
    drop(heavy);
    assert_eq!(
        registry.snapshot(),
        LeaseSnapshot {
            active_actors: 0,
            active_leases: 0,
            heavy_jobs: 0,
            refusals: 1,
        }
    );
}

#[tokio::test]
async fn the_registry_is_shared_across_tasks_and_leases_release_on_drop() {
    fn assert_send_sync<T: Send + Sync>(_value: &T) {}
    fn assert_send<T: Send>(_value: &T) {}

    let registry = LeaseRegistry::new(LeaseLimits::default());
    assert_send_sync(&registry);
    assert_eq!(registry.limits(), LeaseLimits::default());
    let cloned = registry.clone();
    let observed = tokio::spawn(async move {
        let lease = cloned.acquire(7, LeaseClass::HeavyJob).await.unwrap();
        assert_send(&lease);
        cloned.snapshot()
    })
    .await
    .unwrap();
    assert_eq!(observed.active_actors, 1);
    assert_eq!(observed.active_leases, 1);
    assert_eq!(observed.heavy_jobs, 1);
    assert_eq!(
        registry.snapshot(),
        LeaseSnapshot {
            active_actors: 0,
            active_leases: 0,
            heavy_jobs: 0,
            refusals: 0,
        }
    );
}
