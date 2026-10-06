use hm_context::{Scope, digest_bytes, notes::Predicate};
use hm_core::ActorId;
use hm_cortex::development_conditions::{AuthorizedFactReader, ConditionLimits, ConditionOutcome};
use hm_serve::{
    actor::{ActorConfig, ActorEngine},
    context_memory::{
        self, MemoryCommand, MemoryRecord, MemoryRequest, MemorySource, Provenance, RecordKind,
    },
    development_conditions::{deliver_once, evaluate_note},
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::Command,
};
fn scope() -> Scope {
    Scope {
        owner_id: "owner".into(),
        project_id: "conditions".into(),
        workspace_id: None,
    }
}
fn config(path: &Path) -> ActorConfig {
    ActorConfig {
        actor_directory: path.into(),
        actor: ActorId::new(39),
        user: [4; 16],
        kek: [9; 32],
        projection_map_bytes: 16 * 1024 * 1024,
    }
}
fn git_binary() -> PathBuf {
    std::env::var_os("HM_GIT_BINARY")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
                .map(|path| path.join("git"))
                .find(|path| path.is_file())
                .expect("actual Git executable required")
        })
}
fn reader(root: &Path) -> AuthorizedFactReader {
    AuthorizedFactReader::open(
        scope(),
        BTreeMap::from([("root".into(), root.into())]),
        git_binary(),
    )
    .unwrap()
}
fn equals(key: &str, value: &str) -> Predicate {
    Predicate::Equals {
        key: key.into(),
        value: value.into(),
    }
}
fn run_git(root: &Path, args: &[&str]) -> String {
    let output = Command::new(git_binary())
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "actual Git failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}
async fn command(actor: &ActorEngine, id: &str, command: MemoryCommand) {
    context_memory::execute(
        actor,
        &scope(),
        &scope(),
        MemoryRequest {
            version: 1,
            scope: scope(),
            request_id: id.into(),
            command,
        },
    )
    .await
    .unwrap();
}
#[test]
fn actual_files_bounds_scopes_and_symlinks_preserve_outcome_classes() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("ready.txt"), "ready").unwrap();
    let reader = reader(directory.path());
    let limits = ConditionLimits::default();
    std::fs::write(directory.path().join("literal-false"), "false").unwrap();
    assert_eq!(
        reader
            .evaluate(
                &Predicate::Exists("file.content:root:literal-false".into()),
                &scope(),
                limits
            )
            .unwrap()
            .outcome,
        ConditionOutcome::True
    );
    assert_eq!(
        reader
            .evaluate(
                &equals("file.content:root:ready.txt", "ready"),
                &scope(),
                limits
            )
            .unwrap()
            .outcome,
        ConditionOutcome::True
    );
    assert_eq!(
        reader
            .evaluate(
                &Predicate::Exists("file.exists:root:absent".into()),
                &scope(),
                limits
            )
            .unwrap()
            .outcome,
        ConditionOutcome::False
    );
    assert_eq!(
        reader
            .evaluate(
                &equals("file.content:root:ready.txt", "other"),
                &scope(),
                limits
            )
            .unwrap()
            .outcome,
        ConditionOutcome::False
    );
    assert_eq!(
        reader
            .evaluate(
                &Predicate::Exists("file.mtime:root:ready.txt".into()),
                &scope(),
                limits
            )
            .unwrap()
            .outcome,
        ConditionOutcome::True
    );
    assert_eq!(
        reader
            .evaluate(
                &Predicate::Exists("file.content:root:../escape".into()),
                &scope(),
                limits
            )
            .unwrap()
            .outcome,
        ConditionOutcome::Refused
    );
    assert_eq!(
        reader
            .evaluate(
                &Predicate::Exists("shell:root:echo".into()),
                &scope(),
                limits
            )
            .unwrap()
            .outcome,
        ConditionOutcome::Refused
    );
    assert_eq!(
        reader
            .evaluate(
                &Predicate::Exists("file.content:root:ready.txt".into()),
                &scope(),
                ConditionLimits {
                    max_bytes: 2,
                    ..limits
                }
            )
            .unwrap()
            .outcome,
        ConditionOutcome::Refused
    );
    std::fs::write(directory.path().join("binary"), [255u8]).unwrap();
    assert_eq!(
        reader
            .evaluate(
                &Predicate::Exists("file.content:root:binary".into()),
                &scope(),
                limits
            )
            .unwrap()
            .outcome,
        ConditionOutcome::Unknown
    );
    let mut foreign = scope();
    foreign.owner_id = "foreign".into();
    assert_eq!(
        reader
            .evaluate(&Predicate::True, &foreign, limits)
            .unwrap()
            .outcome,
        ConditionOutcome::Refused
    );
    #[cfg(unix)]
    {
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret"), "outside").unwrap();
        std::os::unix::fs::symlink(outside.path(), directory.path().join("link")).unwrap();
        assert_eq!(
            reader
                .evaluate(
                    &Predicate::Exists("file.content:root:link/secret".into()),
                    &scope(),
                    limits
                )
                .unwrap()
                .outcome,
            ConditionOutcome::Refused
        );
        std::fs::remove_file(directory.path().join("ready.txt")).unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("secret"),
            directory.path().join("ready.txt"),
        )
        .unwrap();
        assert_eq!(
            reader
                .evaluate(
                    &Predicate::Exists("file.content:root:ready.txt".into()),
                    &scope(),
                    limits
                )
                .unwrap()
                .outcome,
            ConditionOutcome::Refused
        );
    }
}
#[test]
fn actual_git_ancestry_and_tags_are_bounded_local_observations() {
    let directory = tempfile::tempdir().unwrap();
    run_git(directory.path(), &["init", "--quiet"]);
    run_git(directory.path(), &["config", "user.name", "Condition Test"]);
    run_git(
        directory.path(),
        &["config", "user.email", "condition@example.invalid"],
    );
    std::fs::write(directory.path().join("file"), "one").unwrap();
    run_git(directory.path(), &["add", "file"]);
    run_git(directory.path(), &["commit", "--quiet", "-m", "first"]);
    let first = run_git(directory.path(), &["rev-parse", "HEAD"]);
    std::fs::write(directory.path().join("file"), "two").unwrap();
    run_git(directory.path(), &["commit", "--quiet", "-am", "second"]);
    run_git(directory.path(), &["tag", "release"]);
    let reader = reader(directory.path());
    let limits = ConditionLimits::default();
    assert_eq!(
        reader
            .evaluate(
                &equals(&format!("git.ancestor:root:{first}:HEAD"), "true"),
                &scope(),
                limits
            )
            .unwrap()
            .outcome,
        ConditionOutcome::True
    );
    assert_eq!(
        reader
            .evaluate(
                &equals(&format!("git.ancestor:root:HEAD:{first}"), "true"),
                &scope(),
                limits
            )
            .unwrap()
            .outcome,
        ConditionOutcome::False
    );
    assert_eq!(
        reader
            .evaluate(
                &Predicate::Exists("git.tag:root:release".into()),
                &scope(),
                limits
            )
            .unwrap()
            .outcome,
        ConditionOutcome::True
    );
    assert_eq!(
        reader
            .evaluate(
                &Predicate::Exists("git.tag:root:missing".into()),
                &scope(),
                limits
            )
            .unwrap()
            .outcome,
        ConditionOutcome::False
    );
    assert_eq!(
        reader
            .evaluate(
                &Predicate::Exists("git.ancestor:root:missing:HEAD".into()),
                &scope(),
                limits
            )
            .unwrap()
            .outcome,
        ConditionOutcome::Unknown
    );
    assert_eq!(
        reader
            .evaluate(
                &Predicate::Exists("git.tag:root:--exec=bad".into()),
                &scope(),
                limits
            )
            .unwrap()
            .outcome,
        ConditionOutcome::Refused
    );
}
#[tokio::test]
async fn native_readiness_revalidates_changes_and_delivery_is_once_only_after_restart() {
    let directory = tempfile::tempdir().unwrap();
    let files = tempfile::tempdir().unwrap();
    std::fs::write(files.path().join("ready"), "yes").unwrap();
    let reader = reader(files.path());
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    command(
        &actor,
        "source",
        MemoryCommand::Source {
            source: MemorySource {
                id: "source".into(),
                digest: digest_bytes(b"original"),
                content: b"original".to_vec(),
                locator: "authorized:source".into(),
                occurred_at_ns: None,
                recorded_at_ns: 10,
                tombstoned: false,
            },
        },
    )
    .await;
    let mut note = MemoryRecord::new(
        "note",
        RecordKind::ConditionalNote,
        "Owner-authored deferred note",
        11,
    );
    note.predicate = Some(equals("file.content:root:ready", "yes"));
    note.provenance = vec![Provenance {
        source_id: "source".into(),
        source_digest: digest_bytes(b"original"),
        span_start: 0,
        span_end: 8,
        quoted_digest: digest_bytes(b"original"),
    }];
    command(&actor, "note", MemoryCommand::Create { record: note }).await;
    let ready = evaluate_note(
        &actor,
        &scope(),
        &reader,
        "note",
        ConditionLimits::default(),
    )
    .await
    .unwrap();
    assert_eq!(ready.evidence.outcome, ConditionOutcome::True);
    let before = actor.stats().await.unwrap().log_events;
    let repeated = evaluate_note(
        &actor,
        &scope(),
        &reader,
        "note",
        ConditionLimits::default(),
    )
    .await
    .unwrap();
    assert_eq!(ready.occurrence_id, repeated.occurrence_id);
    assert_eq!(actor.stats().await.unwrap().log_events, before);
    std::fs::write(files.path().join("ready"), "no").unwrap();
    assert!(
        deliver_once(
            &actor,
            &scope(),
            &reader,
            &ready,
            ConditionLimits::default()
        )
        .await
        .is_err()
    );
    let not_ready = evaluate_note(
        &actor,
        &scope(),
        &reader,
        "note",
        ConditionLimits::default(),
    )
    .await
    .unwrap();
    assert_eq!(not_ready.evidence.outcome, ConditionOutcome::False);
    assert!(not_ready.occurrence_id.is_none());
    std::fs::write(files.path().join("ready"), "yes").unwrap();
    let ready = evaluate_note(
        &actor,
        &scope(),
        &reader,
        "note",
        ConditionLimits::default(),
    )
    .await
    .unwrap();
    let delivered = deliver_once(
        &actor,
        &scope(),
        &reader,
        &ready,
        ConditionLimits::default(),
    )
    .await
    .unwrap();
    assert_eq!(delivered.text, "Owner-authored deferred note");
    assert!(!delivered.replayed);
    let before = actor.stats().await.unwrap().log_events;
    assert!(
        deliver_once(
            &actor,
            &scope(),
            &reader,
            &ready,
            ConditionLimits::default()
        )
        .await
        .unwrap()
        .replayed
    );
    assert_eq!(actor.stats().await.unwrap().log_events, before);
    actor.shutdown().await.unwrap();
    let actor = ActorEngine::open(config(directory.path())).await.unwrap();
    let before = actor.stats().await.unwrap().log_events;
    let replay = deliver_once(
        &actor,
        &scope(),
        &reader,
        &ready,
        ConditionLimits::default(),
    )
    .await
    .unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.occurrence_id, delivered.occurrence_id);
    assert_eq!(actor.stats().await.unwrap().log_events, before);
    let mut revised = context_memory::rebuild(&actor, &scope())
        .await
        .unwrap()
        .records["note"]
        .clone();
    revised.revision += 1;
    revised.revision_digest.clear();
    revised.predicate = Some(equals("file.content:root:ready", "never"));
    command(
        &actor,
        "note-revise",
        MemoryCommand::Revise {
            record: revised,
            expected_revision: 1,
        },
    )
    .await;
    let changed = evaluate_note(
        &actor,
        &scope(),
        &reader,
        "note",
        ConditionLimits::default(),
    )
    .await
    .unwrap();
    assert_eq!(changed.evidence.outcome, ConditionOutcome::False);
    assert!(!changed.delivered);
    command(
        &actor,
        "source-revoke",
        MemoryCommand::TombstoneSource {
            id: "source".into(),
        },
    )
    .await;
    assert!(
        evaluate_note(
            &actor,
            &scope(),
            &reader,
            "note",
            ConditionLimits::default()
        )
        .await
        .is_err()
    );
    actor.shutdown().await.unwrap();
}
#[cfg(unix)]
#[test]
fn replacing_an_authorized_root_refuses_future_reads() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("root");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("file"), "value").unwrap();
    let reader = reader(&root);
    std::fs::rename(&root, directory.path().join("old")).unwrap();
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("file"), "value").unwrap();
    assert_eq!(
        reader
            .evaluate(
                &Predicate::Exists("file.exists:root:file".into()),
                &scope(),
                ConditionLimits::default()
            )
            .unwrap()
            .outcome,
        ConditionOutcome::Refused
    );
}
