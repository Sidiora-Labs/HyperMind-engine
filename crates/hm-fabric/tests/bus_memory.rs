use hm_context::types::digest_bytes;
use hm_context::{ContextError, Scope};
use hm_fabric::bus::{Disposition, Grant};
use hm_fabric::bus_contract::{BusTopology, CensusGuard, StreamFamily};
use hm_fabric::bus_memory::{MemoryBus, MemoryBusLimits};
use std::sync::{Arc, Barrier};
fn scope(project: &str) -> Scope {
    Scope {
        owner_id: "owner".into(),
        project_id: project.into(),
        workspace_id: None,
    }
}
fn digest(text: &str) -> Vec<u8> {
    digest_bytes(text.as_bytes()).into_bytes()
}
fn grant(bus: &MemoryBus, principal: &str, resource: &str) {
    bus.grant(Grant {
        principal: principal.into(),
        stream: resource.into(),
        publish: true,
        subscribe: true,
        register: true,
    })
    .unwrap();
}
#[test]
fn six_stream_families_and_descriptor_collisions() {
    let topology = BusTopology::new(Scope {
        owner_id: "owner".into(),
        project_id: "project".into(),
        workspace_id: Some("work".into()),
    })
    .unwrap();
    let bus = MemoryBus::new(topology.scope.clone(), MemoryBusLimits::default()).unwrap();
    let tokens = [
        "room-posts",
        "wake-signals",
        "peer-deliveries",
        "effect-intents",
        "dead-letter-effects",
        "module-events",
    ];
    for (family, token) in StreamFamily::ALL.into_iter().zip(tokens) {
        assert_eq!(family.token(), token);
        assert_eq!(
            topology.stream_subject(family),
            format!("hm.owner.project.work.{token}.>")
        );
        let name = topology.stream_name(family);
        assert_eq!(
            name,
            format!(
                "HM_OWNER_PROJECT_WORK_{}",
                token.replace('-', "_").to_uppercase()
            )
        );
        assert_eq!(
            topology.subject(family, "worker", "changed").unwrap(),
            format!("hm.owner.project.work.{token}.worker.changed")
        );
        grant(&bus, "worker", &name);
        let event = bus.append_family("worker", family, &digest(token)).unwrap();
        assert_eq!(bus.replay("worker", &name, 0, 8).unwrap(), vec![event]);
    }
    assert_eq!(topology.census_bucket, "HM_CENSUS_owner_project_work");
    let durables = topology.durables("agent", "module").unwrap();
    assert_eq!(durables.agent, "hm_owner_project_work_agent_agent");
    assert_eq!(durables.module, "hm_owner_project_work_module_module");
    topology.validate_families().unwrap();
    assert!(topology
        .subject(StreamFamily::ModuleEvents, "worker", ">")
        .is_err());
    assert!(BusTopology::new(scope("project.escape")).is_err());
    let first = MemoryBus::new(
        Scope {
            owner_id: "bound_part".into(),
            project_id: "root".into(),
            workspace_id: None,
        },
        MemoryBusLimits::default(),
    )
    .unwrap();
    assert!(matches!(
        MemoryBus::new(
            Scope {
                owner_id: "bound".into(),
                project_id: "part_root".into(),
                workspace_id: None
            },
            MemoryBusLimits::default()
        ),
        Err(ContextError::Conflict)
    ));
    drop(first);
    let first = MemoryBus::new(
        Scope {
            owner_id: "CaseBound".into(),
            project_id: "case".into(),
            workspace_id: None,
        },
        MemoryBusLimits::default(),
    )
    .unwrap();
    assert!(matches!(
        MemoryBus::new(
            Scope {
                owner_id: "casebound".into(),
                project_id: "case".into(),
                workspace_id: None
            },
            MemoryBusLimits::default()
        ),
        Err(ContextError::Conflict)
    ));
    drop(first);
}
#[test]
fn concurrent_publish_cas_and_settlement_have_one_winner() {
    let bus = MemoryBus::new(scope("concurrent"), MemoryBusLimits::default()).unwrap();
    grant(&bus, "worker", "events");
    let start = Arc::new(Barrier::new(17));
    let mut workers = Vec::new();
    for n in 0..16 {
        let bus = bus.clone();
        let start = start.clone();
        workers.push(std::thread::spawn(move || {
            start.wait();
            bus.append("worker", "events", &digest(&format!("event-{n}")))
                .unwrap()
        }));
    }
    start.wait();
    let mut sequences: Vec<u64> = workers
        .into_iter()
        .map(|w| w.join().unwrap().sequence)
        .collect();
    sequences.sort();
    assert_eq!(sequences, (1..=16).collect::<Vec<_>>());
    let start = Arc::new(Barrier::new(3));
    let mut workers = Vec::new();
    for n in 0..2 {
        let bus = bus.clone();
        let start = start.clone();
        workers.push(std::thread::spawn(move || {
            start.wait();
            bus.register_cas(
                "worker",
                "events",
                "state",
                0,
                &digest(&format!("value-{n}")),
            )
        }));
    }
    start.wait();
    let results: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert!(results
        .iter()
        .any(|r| matches!(r, Err(ContextError::Conflict))));
    bus.subscribe("worker", "events", "reader", 3).unwrap();
    let delivery = bus
        .next("worker", "events", "reader", 10, 30)
        .unwrap()
        .unwrap();
    let start = Arc::new(Barrier::new(3));
    let mut workers = Vec::new();
    for _ in 0..2 {
        let bus = bus.clone();
        let start = start.clone();
        let delivery = delivery.clone();
        workers.push(std::thread::spawn(move || {
            start.wait();
            bus.disposition("worker", &delivery, Disposition::Ack, 11)
        }));
    }
    start.wait();
    let results: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert!(results
        .iter()
        .any(|r| matches!(r, Err(ContextError::Stale))));
}
#[test]
fn stream_attempt_fences_and_dlq_capacity_precede_terminal() {
    let limits = MemoryBusLimits {
        dead_letters: 1,
        events: 2,
        ..Default::default()
    };
    let bus = MemoryBus::new(scope("stream"), limits).unwrap();
    grant(&bus, "worker", "events");
    let first = bus
        .append_once("worker", "events", "one", &digest("one"))
        .unwrap();
    assert_eq!(
        bus.append_once("worker", "events", "one", &digest("one"))
            .unwrap(),
        first
    );
    assert!(matches!(
        bus.append_once("worker", "events", "one", &digest("changed")),
        Err(ContextError::Conflict)
    ));
    bus.append("worker", "events", &digest("two")).unwrap();
    assert!(matches!(
        bus.append("worker", "events", &digest("three")),
        Err(ContextError::Capacity)
    ));
    assert!(bus
        .append("worker", "events", b"payload authority")
        .is_err());
    bus.subscribe("worker", "events", "reader", 2).unwrap();
    let d = bus
        .next("worker", "events", "reader", 10, 10)
        .unwrap()
        .unwrap();
    bus.disposition("worker", &d, Disposition::Progress { lease_ms: 20 }, 11)
        .unwrap();
    assert!(bus
        .next("worker", "events", "reader", 21, 10)
        .unwrap()
        .is_none());
    bus.disposition("worker", &d, Disposition::Nak, 22).unwrap();
    assert!(matches!(
        bus.disposition("worker", &d, Disposition::Nak, 22),
        Err(ContextError::Stale)
    ));
    let retry = bus
        .next("worker", "events", "reader", 23, 10)
        .unwrap()
        .unwrap();
    assert_eq!(retry.attempt, 2);
    assert!(matches!(
        bus.disposition("worker", &d, Disposition::Ack, 24),
        Err(ContextError::Stale)
    ));
    let second = bus
        .next("worker", "events", "reader", 34, 10)
        .unwrap()
        .unwrap();
    assert_eq!(second.event.payload, digest("two"));
    assert_eq!(
        bus.dead_letters("worker", "events", "reader").unwrap()[0]
            .delivery
            .event,
        first
    );
    assert!(matches!(
        bus.disposition("worker", &second, Disposition::Term, 35),
        Err(ContextError::Capacity)
    ));
    assert!(bus
        .next("worker", "events", "reader", 36, 10)
        .unwrap()
        .is_none());
    bus.disposition("worker", &second, Disposition::Ack, 37)
        .unwrap();
    assert!(bus
        .next("worker", "events", "reader", 38, 10)
        .unwrap()
        .is_none());
}
#[test]
fn competing_queue_claims_retry_and_settle_once() {
    let limits = MemoryBusLimits {
        queue_depth: 2,
        dead_letters: 1,
        ..Default::default()
    };
    let bus = MemoryBus::new(scope("queue"), limits).unwrap();
    grant(&bus, "a", "jobs");
    grant(&bus, "b", "jobs");
    bus.queue_create("a", "jobs", 2).unwrap();
    let first = bus.queue_push("a", "jobs", &digest("job1")).unwrap();
    bus.queue_push("a", "jobs", &digest("job2")).unwrap();
    assert!(matches!(
        bus.queue_push("a", "jobs", &digest("overflow")),
        Err(ContextError::Capacity)
    ));
    let start = Arc::new(Barrier::new(3));
    let mut workers = Vec::new();
    for principal in ["a", "b"] {
        let bus = bus.clone();
        let start = start.clone();
        workers.push(std::thread::spawn(move || {
            start.wait();
            (
                principal,
                bus.queue_next(principal, "jobs", principal, 10, 20)
                    .unwrap(),
            )
        }));
    }
    start.wait();
    let results: Vec<_> = workers.into_iter().map(|w| w.join().unwrap()).collect();
    let claimed: Vec<_> = results
        .into_iter()
        .filter_map(|(p, d)| d.map(|d| (p, d)))
        .collect();
    assert_eq!(claimed.len(), 1);
    let (principal, d) = &claimed[0];
    assert_eq!(d.event, first);
    bus.queue_disposition(principal, d, Disposition::Nak, 11)
        .unwrap();
    assert!(matches!(
        bus.queue_disposition(principal, d, Disposition::Nak, 11),
        Err(ContextError::Stale)
    ));
    let retry = bus.queue_next("b", "jobs", "b", 12, 20).unwrap().unwrap();
    assert_eq!(retry.attempt, 2);
    bus.queue_disposition("b", &retry, Disposition::Term, 13)
        .unwrap();
    assert!(matches!(
        bus.queue_disposition("b", &retry, Disposition::Term, 13),
        Err(ContextError::Stale)
    ));
    assert_eq!(
        bus.dead_letters("b", "jobs", "b").unwrap()[0]
            .delivery
            .event,
        first
    );
    let second = bus.queue_next("a", "jobs", "a", 14, 20).unwrap().unwrap();
    assert!(matches!(
        bus.queue_disposition("a", &second, Disposition::Term, 15),
        Err(ContextError::Capacity)
    ));
    assert!(bus.queue_next("b", "jobs", "b", 16, 20).unwrap().is_none());
    bus.queue_disposition("a", &second, Disposition::Ack, 17)
        .unwrap();
    assert!(bus.queue_next("a", "jobs", "a", 18, 20).unwrap().is_none());
}
#[test]
fn register_census_is_revisioned_and_absence_requires_observation() {
    let bus = MemoryBus::new(scope("register"), MemoryBusLimits::default()).unwrap();
    grant(&bus, "worker", "census");
    let mut guard = CensusGuard::default();
    assert!(!guard.proven_absent());
    guard.observe_snapshot(&bus.register_snapshot("worker", "census").unwrap(), "agent");
    assert!(guard.proven_absent());
    let entry = bus
        .register_cas("worker", "census", "agent", 0, &digest("presence"))
        .unwrap();
    let updates = bus.register_watch_after("worker", "census", 0, 8).unwrap();
    guard.observe(&updates[0], "agent");
    assert!(!guard.proven_absent());
    assert_eq!(
        bus.register_delete("worker", "census", "agent", entry.revision)
            .unwrap(),
        2
    );
    let updates = bus.register_watch_after("worker", "census", 1, 8).unwrap();
    guard.observe(&updates[0], "agent");
    assert!(guard.proven_absent());
    guard.observe(&updates[0], "agent");
    assert_eq!(guard.revision(), Some(2));
    guard.disconnect();
    assert!(!guard.proven_absent());
    guard.observe_snapshot(&bus.register_snapshot("worker", "census").unwrap(), "agent");
    assert!(guard.proven_absent());
    assert!(matches!(
        bus.register_watch_after("worker", "census", 99, 8),
        Err(ContextError::Stale)
    ));
    let replacement = bus
        .register_cas("worker", "census", "agent", 0, &digest("new presence"))
        .unwrap();
    assert_eq!(replacement.revision, 3);
    assert!(matches!(
        bus.register_cas(
            "worker",
            "census",
            "agent",
            entry.revision,
            &digest("stale")
        ),
        Err(ContextError::Conflict)
    ));
}
#[test]
fn bounds_grants_scope_and_process_lifetime_are_explicit() {
    let limits = MemoryBusLimits {
        resources: 1,
        events: 2,
        consumers: 1,
        grants: 2,
        registers: 1,
        dead_letters: 1,
        queue_depth: 1,
    };
    let bus = MemoryBus::new(scope("lifetime"), limits).unwrap();
    grant(&bus, "worker", "events");
    grant(&bus, "other", "events");
    assert!(matches!(
        bus.grant(Grant {
            principal: "third".into(),
            stream: "events".into(),
            publish: true,
            subscribe: true,
            register: true
        }),
        Err(ContextError::Capacity)
    ));
    let event = bus.append("worker", "events", &digest("event")).unwrap();
    bus.subscribe("worker", "events", "reader", 2).unwrap();
    assert!(bus.subscribe("other", "events", "reader", 2).is_err());
    assert!(matches!(
        bus.subscribe("worker", "events", "second", 2),
        Err(ContextError::Capacity)
    ));
    assert!(matches!(
        bus.next("other", "events", "reader", 10, 20),
        Err(ContextError::ScopeMismatch)
    ));
    assert!(bus
        .census("other", 10)
        .unwrap()
        .iter()
        .all(|e| !e.name.contains("reader")));
    assert!(matches!(
        bus.census("worker", 1),
        Err(ContextError::Capacity)
    ));
    bus.register_cas("worker", "events", "one", 0, &digest("one"))
        .unwrap();
    assert!(matches!(
        bus.register_cas("worker", "events", "two", 0, &digest("two")),
        Err(ContextError::Capacity)
    ));
    bus.revoke("worker", "events").unwrap();
    assert!(matches!(
        bus.replay("worker", "events", 0, 2),
        Err(ContextError::ScopeMismatch)
    ));
    let fresh = MemoryBus::new(scope("lifetime"), limits).unwrap();
    grant(&fresh, "worker", "events");
    assert!(fresh.replay("worker", "events", 0, 2).unwrap().is_empty());
    assert_eq!(event.payload.len(), 64);
    assert!(!bus.capabilities().durable);
    assert!(!bus.capabilities().distributed);
    assert_eq!(bus.capabilities().backend, "process_local");
    assert!(MemoryBus::new(
        scope("invalid"),
        MemoryBusLimits {
            events: 0,
            ..limits
        }
    )
    .is_err());
}
