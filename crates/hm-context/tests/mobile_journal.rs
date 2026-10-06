use hm_context::{
    Cursor, Scope,
    mobile_journal::{
        JournalBinding, JournalError, MobileJournal, MutationState, NeutralReceipt, ReceiptOutcome,
        WakeIdentifier,
    },
    types::digest_bytes,
};
use std::fs;
fn binding() -> JournalBinding {
    JournalBinding {
        scope: Scope {
            owner_id: "journal-owner".into(),
            project_id: "journal-project".into(),
            workspace_id: None,
        },
        device_id: "device-a".into(),
        session_id: "application-session".into(),
    }
}
#[test]
fn durable_pending_unknown_terminal_and_exact_sparse_receipt_fences() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("outbound.json");
    let payload = b"knowledge content is not part of the outbound journal";
    let mut journal = MobileJournal::open(&path, binding()).unwrap();
    let first = journal
        .enqueue_mutation("operation-a", "tool", &digest_bytes(payload))
        .unwrap();
    assert_eq!(first.state, MutationState::Pending);
    assert_eq!(first.wake_id.as_str().len(), 64);
    assert_eq!(
        serde_json::to_value(&first.wake_id).unwrap(),
        first.wake_id.as_str()
    );
    assert!(WakeIdentifier::parse("memory:private contents").is_err());
    assert!(matches!(
        MobileJournal::open(&path, binding()),
        Err(JournalError::Context(_))
    ));
    assert!(matches!(
        journal.enqueue_mutation("operation-b", "tool", &digest_bytes(b"different")),
        Err(JournalError::Busy)
    ));
    drop(journal);
    let mut journal = MobileJournal::open(&path, binding()).unwrap();
    assert_eq!(journal.pending().unwrap().state, MutationState::Pending);
    let unknown = journal.mark_dispatched("operation-a", "tls-first").unwrap();
    assert_eq!(unknown.state, MutationState::Unknown);
    assert_eq!(unknown.dispatched_session_id.as_deref(), Some("tls-first"));
    drop(journal);
    let mut journal = MobileJournal::open(&path, binding()).unwrap();
    assert_eq!(journal.pending().unwrap().wake_id, first.wake_id);
    assert!(
        journal
            .mark_dispatched("operation-a", "tls-resumed")
            .is_err()
    );
    assert!(matches!(
        journal.enqueue_mutation("operation-b", "tool", &digest_bytes(b"different")),
        Err(JournalError::Busy)
    ));
    // Neutral receipt vectors exercise storage fences; encrypted native receipt validation is a separate adapter gate.
    let receipt = NeutralReceipt {
        identity: first.identity.clone(),
        intent_version: 2,
        cursor: Cursor {
            epoch: 1,
            sequence: 19,
        },
        receipt_digest: digest_bytes(b"neutral receipt contract vector"),
        outcome: ReceiptOutcome::Unknown,
        observed_session_id: "tls-resumed".into(),
    };
    let mut wrong = receipt.clone();
    wrong.observed_session_id = "tls-first".into();
    assert!(journal.reconcile_before_next_write(&wrong).is_err());
    wrong = receipt.clone();
    wrong.identity.binding.device_id = "device-b".into();
    assert!(journal.reconcile_before_next_write(&wrong).is_err());
    wrong = receipt.clone();
    wrong.identity.payload_digest = digest_bytes(b"foreign payload");
    assert!(journal.reconcile_before_next_write(&wrong).is_err());
    assert_eq!(
        journal.reconcile_before_next_write(&receipt).unwrap().state,
        MutationState::Unknown
    );
    assert_eq!(
        journal.cursor_receipt(),
        Some(Cursor {
            epoch: 1,
            sequence: 19
        })
    );
    drop(journal);
    let mut journal = MobileJournal::open(&path, binding()).unwrap();
    let terminal = NeutralReceipt {
        intent_version: 3,
        cursor: Cursor {
            epoch: 1,
            sequence: 31,
        },
        receipt_digest: digest_bytes(b"terminal neutral receipt vector"),
        outcome: ReceiptOutcome::Succeeded,
        observed_session_id: "tls-next-resume".into(),
        ..receipt
    };
    assert_eq!(
        journal
            .reconcile_before_next_write(&terminal)
            .unwrap()
            .state,
        MutationState::Terminal
    );
    assert!(journal.pending().is_none());
    assert_eq!(
        journal.cursor_receipt(),
        Some(Cursor {
            epoch: 1,
            sequence: 31
        })
    );
    assert_eq!(
        journal
            .reconcile_before_next_write(&terminal)
            .unwrap()
            .state,
        MutationState::Terminal
    );
    assert!(
        journal
            .enqueue_mutation("operation-a", "tool", &digest_bytes(payload))
            .is_err()
    );
    assert!(
        journal
            .enqueue_mutation("operation-a", "tool", &digest_bytes(b"changed"))
            .is_err()
    );
    let second = journal
        .enqueue_mutation("operation-b", "tool", &digest_bytes(b"different"))
        .unwrap();
    assert_ne!(second.wake_id, first.wake_id);
    assert_ne!(second.wake_id.as_str(), second.identity.operation_id);
    let bytes = fs::read(&path).unwrap();
    assert!(
        !String::from_utf8(bytes.clone())
            .unwrap()
            .contains(std::str::from_utf8(payload).unwrap())
    );
    assert!(!String::from_utf8(bytes).unwrap().contains("different"));
    drop(journal);
    let journal = MobileJournal::open(&path, binding()).unwrap();
    assert_eq!(journal.records().len(), 2);
    assert_eq!(journal.cursor_receipt().unwrap().sequence, 31);
    assert_eq!(
        journal.records()[0].receipt.as_ref().unwrap().outcome,
        ReceiptOutcome::Succeeded
    );
}
#[test]
fn corrupt_snapshot_scope_and_stale_receipt_are_refused_without_settlement() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("journal.json");
    let mut journal = MobileJournal::open(&path, binding()).unwrap();
    let record = journal
        .enqueue_mutation("op", "tool", &digest_bytes(b"body"))
        .unwrap();
    journal.mark_dispatched("op", "tls-one").unwrap();
    let unknown = NeutralReceipt {
        identity: record.identity,
        intent_version: 2,
        cursor: Cursor {
            epoch: 2,
            sequence: 55,
        },
        receipt_digest: digest_bytes(b"unknown"),
        outcome: ReceiptOutcome::Unknown,
        observed_session_id: "tls-one".into(),
    };
    journal.reconcile_before_next_write(&unknown).unwrap();
    let before = fs::read(&path).unwrap();
    let stale = NeutralReceipt {
        intent_version: 3,
        cursor: Cursor {
            epoch: 1,
            sequence: 1000,
        },
        receipt_digest: digest_bytes(b"stale"),
        outcome: ReceiptOutcome::NotApplied,
        ..unknown
    };
    assert!(journal.reconcile_before_next_write(&stale).is_err());
    assert_eq!(fs::read(&path).unwrap(), before);
    drop(journal);
    let mut other = binding();
    other.scope.project_id = "foreign".into();
    assert!(MobileJournal::open(&path, other).is_err());
    let mut corrupt: serde_json::Value = serde_json::from_slice(&before).unwrap();
    corrupt["snapshot"]["records"][0]["state"] = "terminal".into();
    fs::write(&path, serde_json::to_vec(&corrupt).unwrap()).unwrap();
    assert!(MobileJournal::open(&path, binding()).is_err());
}

#[test]
fn actual_process_restart_keeps_uncertainty_and_releases_os_lock() {
    const PHASE: &str = "HM_MOBILE_JOURNAL_TEST_PHASE";
    const PATH: &str = "HM_MOBILE_JOURNAL_TEST_PATH";
    if let Ok(phase) = std::env::var(PHASE) {
        let path = std::path::PathBuf::from(std::env::var_os(PATH).unwrap());
        let mut journal = MobileJournal::open(&path, binding()).unwrap();
        if phase == "dispatch" {
            journal
                .enqueue_mutation("process-op", "tool", &digest_bytes(b"private body"))
                .unwrap();
            journal
                .mark_dispatched("process-op", "tls-before-crash")
                .unwrap();
            fs::write(path.with_extension("ready"), b"ready").unwrap();
            loop {
                std::thread::park();
            }
        }
        assert_eq!(phase, "reopen");
        let record = journal.pending().unwrap();
        assert_eq!(record.state, MutationState::Unknown);
        assert_eq!(
            record.dispatched_session_id.as_deref(),
            Some("tls-before-crash")
        );
        assert!(
            journal
                .mark_dispatched("process-op", "tls-after-crash")
                .is_err()
        );
        assert!(matches!(
            journal.enqueue_mutation("later-op", "tool", &digest_bytes(b"later")),
            Err(JournalError::Busy)
        ));
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("journal.json");
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "actual_process_restart_keeps_uncertainty_and_releases_os_lock",
            "--nocapture",
        ])
        .env(PHASE, "dispatch")
        .env(PATH, &path)
        .spawn()
        .unwrap();
    let start = std::time::Instant::now();
    while !path.with_extension("ready").exists() {
        if child.try_wait().unwrap().is_some()
            || start.elapsed() > std::time::Duration::from_secs(5)
        {
            let _ = child.kill();
            let _ = child.wait();
            panic!("journal child failed before durable dispatch");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    child.kill().unwrap();
    assert!(!child.wait().unwrap().success());
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "actual_process_restart_keeps_uncertainty_and_releases_os_lock",
            "--nocapture",
        ])
        .env(PHASE, "reopen")
        .env(PATH, &path)
        .status()
        .unwrap();
    assert!(status.success());
}
