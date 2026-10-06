use hm_context::{ContextError, Scope};
use hm_fabric::bus::{scoped_name, Bus, Disposition, Grant};
fn scope(project: &str) -> Scope {
    Scope {
        owner_id: "owner".into(),
        project_id: project.into(),
        workspace_id: None,
    }
}
fn grant(bus: &mut Bus) {
    bus.grant(Grant {
        principal: "worker".into(),
        stream: "events".into(),
        publish: true,
        subscribe: true,
        register: true,
    })
    .unwrap();
}
#[test]
fn persisted_delivery_replay_and_dispositions() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bus.db");
    let mut bus = Bus::open(&path, scope("one")).unwrap();
    grant(&mut bus);
    assert!(!bus.capabilities().distributed);
    let first = bus.append("worker", "events", b"one").unwrap();
    bus.append("worker", "events", b"two").unwrap();
    bus.append("worker", "events", b"three").unwrap();
    bus.subscribe("worker", "events", "reader", 2).unwrap();
    let d = bus
        .next("worker", "events", "reader", 10, 20)
        .unwrap()
        .unwrap();
    assert_eq!(d.event, first);
    bus.disposition("worker", &d, Disposition::Progress { lease_ms: 40 }, 11)
        .unwrap();
    assert!(bus
        .next("worker", "events", "reader", 35, 20)
        .unwrap()
        .is_none());
    bus.disposition("worker", &d, Disposition::Nak, 36).unwrap();
    let retry = bus
        .next("worker", "events", "reader", 37, 20)
        .unwrap()
        .unwrap();
    assert_eq!(retry.attempt, 2);
    assert!(matches!(
        bus.disposition("worker", &d, Disposition::Ack, 38),
        Err(ContextError::Stale)
    ));
    drop(bus);
    let mut bus = Bus::open(&path, scope("one")).unwrap();
    let second = bus
        .next("worker", "events", "reader", 58, 20)
        .unwrap()
        .unwrap();
    assert_eq!(second.event.payload, b"two");
    let dead = bus.dead_letters("worker", "events", "reader").unwrap();
    assert_eq!(dead.len(), 1);
    assert_eq!(dead[0].delivery.event, first);
    bus.disposition("worker", &second, Disposition::Term, 59)
        .unwrap();
    let third = bus
        .next("worker", "events", "reader", 60, 20)
        .unwrap()
        .unwrap();
    bus.disposition("worker", &third, Disposition::Ack, 61)
        .unwrap();
    drop(bus);
    let mut bus = Bus::open(&path, scope("one")).unwrap();
    bus.subscribe("worker", "events", "reader", 2).unwrap();
    assert!(bus
        .next("worker", "events", "reader", 100, 20)
        .unwrap()
        .is_none());
    assert_eq!(
        bus.dead_letters("worker", "events", "reader")
            .unwrap()
            .len(),
        2
    );
    assert_eq!(bus.replay("worker", "events", 0, 20).unwrap().len(), 3);
    bus.subscribe("worker", "events", "other", 1).unwrap();
    assert_eq!(
        bus.next("worker", "events", "other", 100, 20)
            .unwrap()
            .unwrap()
            .event,
        first
    );
}
#[test]
fn scope_grants_and_revision_cas_are_persistent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bus.db");
    let mut a = Bus::open(&path, scope("one")).unwrap();
    grant(&mut a);
    assert!(a.append("intruder", "events", b"bad").is_err());
    assert!(a.append("worker", "wild.*", b"bad").is_err());
    a.append("worker", "events", b"secret").unwrap();
    let reg = a
        .register_cas("worker", "events", "state", 0, b"first")
        .unwrap();
    assert_eq!(reg.revision, 1);
    assert!(matches!(
        Bus::open(&path, scope("one")),
        Err(ContextError::Conflict)
    ));
    drop(a);
    let mut b = Bus::open(&path, scope("one")).unwrap();
    assert!(matches!(
        b.register_cas("worker", "events", "state", 0, b"stale"),
        Err(ContextError::Conflict)
    ));
    b.register_cas("worker", "events", "state", 1, b"second")
        .unwrap();
    assert_eq!(
        b.register_get("worker", "events", "state")
            .unwrap()
            .unwrap()
            .value,
        b"second"
    );
    assert!(b.subscribe("worker", "events", "reader", 0).is_err());
    drop(b);
    let mut isolated = Bus::open(&path, scope("two")).unwrap();
    assert!(isolated.replay("worker", "events", 0, 20).is_err());
    grant(&mut isolated);
    assert!(isolated
        .replay("worker", "events", 0, 20)
        .unwrap()
        .is_empty());
    assert!(isolated
        .register_get("worker", "events", "state")
        .unwrap()
        .is_none());
    assert_ne!(
        scoped_name(&scope("one"), "events").unwrap(),
        scoped_name(&scope("two"), "events").unwrap()
    );
    assert!(scoped_name(&scope("one"), "events.foo").is_err());
}

#[test]
fn dead_letter_failure_keeps_subscription_retryable() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bus.db");
    let mut bus = Bus::open(&path, scope("one")).unwrap();
    grant(&mut bus);
    let event = bus.append("worker", "events", b"durable").unwrap();
    bus.subscribe("worker", "events", "reader", 1).unwrap();
    let delivery = bus
        .next("worker", "events", "reader", 10, 20)
        .unwrap()
        .unwrap();
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.execute_batch("CREATE TRIGGER reject_dead BEFORE INSERT ON bus_dead BEGIN SELECT RAISE(ABORT,'dead letter unavailable'); END;").unwrap();
    assert!(bus
        .disposition("worker", &delivery, Disposition::Term, 11)
        .is_err());
    assert!(bus.next("worker", "events", "reader", 31, 20).is_err());
    assert!(bus
        .dead_letters("worker", "events", "reader")
        .unwrap()
        .is_empty());
    connection
        .execute_batch("DROP TRIGGER reject_dead")
        .unwrap();
    assert!(bus
        .next("worker", "events", "reader", 32, 20)
        .unwrap()
        .is_none());
    assert_eq!(
        bus.dead_letters("worker", "events", "reader").unwrap()[0]
            .delivery
            .event,
        event
    );
}

#[test]
fn process_restart_preserves_inflight_attempt() {
    const PATH_ENV: &str = "HM_BUS_RESTART_TEST_PATH";
    if let Some(path) = std::env::var_os(PATH_ENV) {
        let mut bus = Bus::open(path, scope("process")).unwrap();
        grant(&mut bus);
        bus.append("worker", "events", b"persisted").unwrap();
        bus.subscribe("worker", "events", "reader", 2).unwrap();
        assert_eq!(
            bus.next("worker", "events", "reader", 10, 20)
                .unwrap()
                .unwrap()
                .attempt,
            1
        );
        bus.register_cas("worker", "events", "state", 0, b"child")
            .unwrap();
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("child.db");
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "process_restart_preserves_inflight_attempt"])
        .env(PATH_ENV, &path)
        .status()
        .unwrap();
    assert!(status.success());
    let mut bus = Bus::open(path, scope("process")).unwrap();
    assert_eq!(
        bus.register_get("worker", "events", "state")
            .unwrap()
            .unwrap()
            .value,
        b"child"
    );
    let delivery = bus
        .next("worker", "events", "reader", 31, 20)
        .unwrap()
        .unwrap();
    assert_eq!(delivery.attempt, 2);
    assert_eq!(delivery.event.payload, b"persisted");
    bus.disposition("worker", &delivery, Disposition::Ack, 32)
        .unwrap();
}

#[test]
fn idempotent_append_survives_restart_and_rejects_content_change() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("once.db");
    let mut bus = Bus::open(&path, scope("one")).unwrap();
    grant(&mut bus);
    let first = bus
        .append_once("worker", "events", "receipt:1", b"receipt")
        .unwrap();
    assert_eq!(
        bus.append_once("worker", "events", "receipt:1", b"receipt")
            .unwrap(),
        first
    );
    assert!(matches!(
        bus.append_once("worker", "events", "receipt:1", b"changed"),
        Err(ContextError::Conflict)
    ));
    drop(bus);
    let mut bus = Bus::open(&path, scope("one")).unwrap();
    assert_eq!(
        bus.append_once("worker", "events", "receipt:1", b"receipt")
            .unwrap(),
        first
    );
    assert_eq!(bus.replay("worker", "events", 0, 20).unwrap().len(), 1);
    assert!(bus.append_once("worker", "events", "", b"bad").is_err());
}

#[test]
fn canonical_writer_lock_rejects_aliases() {
    const LOCK_ENV: &str = "HM_BUS_LOCK_TEST_PATH";
    if let Some(path) = std::env::var_os(LOCK_ENV) {
        assert!(matches!(
            Bus::open(path, scope("one")),
            Err(ContextError::Conflict)
        ));
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("writer.db");
    let bus = Bus::open(&path, scope("one")).unwrap();
    assert_eq!(bus.path(), path);
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "canonical_writer_lock_rejects_aliases"])
        .env(LOCK_ENV, &path)
        .status()
        .unwrap();
    assert!(status.success());
    let alias = dir.path().join(".").join("writer.db");
    assert!(matches!(
        Bus::open(alias, scope("one")),
        Err(ContextError::Conflict)
    ));
    #[cfg(unix)]
    {
        let link = dir.path().join("link.db");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(Bus::open(link, scope("one")).is_err());
        let hard = dir.path().join("hard.db");
        std::fs::hard_link(&path, &hard).unwrap();
        assert!(Bus::open(hard, scope("one")).is_err());
        std::fs::remove_file(dir.path().join("hard.db")).unwrap();
    }
    drop(bus);
    assert!(Bus::open(path, scope("one")).is_ok());
}
