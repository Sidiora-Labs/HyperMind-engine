use hm_context::{Cursor, Scope};
use hm_fabric::effects::{EffectObservation, EffectOutcome, EffectState, EffectStore};
fn scope(project: &str) -> Scope {
    Scope {
        owner_id: "owner".into(),
        project_id: project.into(),
        workspace_id: None,
    }
}
#[test]
fn restart_preserves_intents_and_requires_observed_reconciliation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("effects.sqlite");
    let scope = scope("project");
    let mut store = EffectStore::open(&path).unwrap();
    let intent = store
        .prepare(&scope, "request", "write", b"exact bytes")
        .unwrap();
    assert_eq!(intent.state, EffectState::Prepared);
    assert_eq!(
        store
            .prepare(&scope, "request", "write", b"exact bytes")
            .unwrap(),
        intent
    );
    assert!(store.prepare(&scope, "request", "write", b"other").is_err());
    assert!(EffectStore::open(&path).is_err());
    let dispatched = store
        .begin_dispatch(&scope, "request", intent.version)
        .unwrap();
    assert!(store
        .begin_dispatch(&scope, "request", intent.version)
        .is_err());
    drop(store);
    let mut store = EffectStore::open(&path).unwrap();
    let uncertain = store.get(&scope, "request").unwrap().unwrap();
    assert_eq!(uncertain.state, EffectState::Uncertain);
    assert_eq!(uncertain.version, dispatched.version + 1);
    assert!(store
        .begin_dispatch(&scope, "request", uncertain.version)
        .is_err());
    assert!(store
        .complete(
            &scope,
            "request",
            uncertain.version,
            EffectObservation {
                outcome: EffectOutcome::Succeeded,
                evidence: "external receipt".into()
            }
        )
        .is_err());
    assert!(store
        .reconcile(
            &scope,
            "request",
            uncertain.version,
            EffectObservation {
                outcome: EffectOutcome::Succeeded,
                evidence: " ".into()
            }
        )
        .is_err());
    let terminal = store
        .reconcile(
            &scope,
            "request",
            uncertain.version,
            EffectObservation {
                outcome: EffectOutcome::Succeeded,
                evidence: "observed result digest".into(),
            },
        )
        .unwrap();
    assert_eq!(terminal.state, EffectState::Terminal);
    assert!(store
        .begin_dispatch(&scope, "request", terminal.version)
        .is_err());
    let first = store.receipts(&scope, Cursor::default(), 2).unwrap();
    assert_eq!(first.len(), 2);
    store
        .acknowledge(&scope, "reader", first[1].cursor)
        .unwrap();
    let rest = store
        .receipts(&scope, store.acknowledged(&scope, "reader").unwrap(), 20)
        .unwrap();
    assert_eq!(rest.len(), 2);
    assert_eq!(rest[0].intent.state, EffectState::Uncertain);
    assert_eq!(rest[1].intent, terminal);
    store.acknowledge(&scope, "reader", rest[1].cursor).unwrap();
    assert!(store
        .acknowledge(&scope, "reader", first[0].cursor)
        .is_err());
    drop(store);
    let store = EffectStore::open(&path).unwrap();
    assert_eq!(store.get(&scope, "request").unwrap(), Some(terminal));
    assert!(store
        .receipts(&scope, store.acknowledged(&scope, "reader").unwrap(), 20)
        .unwrap()
        .is_empty());
}
#[test]
fn scope_fences_receipts_and_prepared_intents_survive_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("effects.sqlite");
    let a = scope("a");
    let b = scope("b");
    let mut store = EffectStore::open(&path).unwrap();
    let prepared = store.prepare(&a, "same-key", "send", b"one").unwrap();
    let other = store.prepare(&b, "same-key", "send", b"two").unwrap();
    let other_receipt = store.receipts(&b, Cursor::default(), 10).unwrap();
    assert!(store
        .acknowledge(&a, "reader", other_receipt[0].cursor)
        .is_err());
    assert_eq!(store.receipts(&a, Cursor::default(), 10).unwrap().len(), 1);
    assert_eq!(store.get(&b, "same-key").unwrap(), Some(other));
    drop(store);
    let mut store = EffectStore::open(&path).unwrap();
    assert_eq!(store.get(&a, "same-key").unwrap(), Some(prepared.clone()));
    let dispatch = store
        .begin_dispatch(&a, "same-key", prepared.version)
        .unwrap();
    let result = store
        .complete(
            &a,
            "same-key",
            dispatch.version,
            EffectObservation {
                outcome: EffectOutcome::Failed,
                evidence: "observed rejected operation".into(),
            },
        )
        .unwrap();
    assert_eq!(result.observation.unwrap().outcome, EffectOutcome::Failed);
}
#[test]
fn crash_writer_worker() {
    let Some(path) = std::env::var_os("HM_EFFECTS_CRASH_PATH") else {
        return;
    };
    let mut store = EffectStore::open(path).unwrap();
    let scope = scope("crash");
    let intent = store
        .prepare(&scope, "operation", "write", b"durable before dispatch")
        .unwrap();
    store
        .begin_dispatch(&scope, "operation", intent.version)
        .unwrap();
    std::process::exit(73);
}
#[test]
fn abrupt_process_exit_recovers_dispatched_without_replay() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("crash.sqlite");
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "crash_writer_worker", "--nocapture"])
        .env("HM_EFFECTS_CRASH_PATH", &path)
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(73));
    let mut store = EffectStore::open(path).unwrap();
    let scope = scope("crash");
    let intent = store.get(&scope, "operation").unwrap().unwrap();
    assert_eq!(intent.state, EffectState::Uncertain);
    assert_eq!(intent.payload, b"durable before dispatch");
    assert!(store
        .begin_dispatch(&scope, "operation", intent.version)
        .is_err());
}
