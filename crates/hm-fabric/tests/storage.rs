use hm_fabric::storage::{CanonicalRoot, FencedStore, Migration, StorageError, resolve_recovery};
const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "operations",
    sql: "CREATE TABLE operations(id INTEGER PRIMARY KEY, value TEXT NOT NULL);",
}];

#[test]
fn canonical_admission_and_explicit_bounded_recovery() {
    let directory = tempfile::tempdir().unwrap();
    let root = CanonicalRoot::admit(directory.path().join(".")).unwrap();
    assert_eq!(root.path(), directory.path().canonicalize().unwrap());
    let missing = directory.path().join("gone/child");
    assert!(CanonicalRoot::admit(&missing).is_err());
    assert!(resolve_recovery(&missing, 1).is_err());
    let recovery = resolve_recovery(&missing, 2).unwrap();
    assert_eq!(recovery.missing_components, 2);
    assert_eq!(recovery.resolved, missing);
    assert!(!missing.exists());
    assert!(resolve_recovery(directory.path().join("../escape"), 2).is_err());
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(directory.path(), directory.path().join("alias")).unwrap();
        assert_eq!(
            CanonicalRoot::admit(directory.path().join("alias")).unwrap(),
            root
        );
    }
}

#[test]
fn exclusive_writer_reopen_epoch_atomicity_and_privacy() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("operations.sqlite");
    let mut store = FencedStore::open(&path, MIGRATIONS).unwrap();
    let epoch = store.epoch();
    assert!(
        store
            .read(|db| {
                db.execute("INSERT INTO operations VALUES(99,'read mutation')", [])?;
                Ok(())
            })
            .is_err()
    );
    assert!(matches!(
        FencedStore::open(&path, MIGRATIONS),
        Err(StorageError::Busy)
    ));
    assert!(matches!(
        store.transaction(epoch + 1, |_| Ok(())),
        Err(StorageError::StaleEpoch)
    ));
    let failure: Result<(), _> = store.transaction(epoch, |tx| {
        tx.execute("INSERT INTO operations VALUES(1,'rollback')", [])?;
        Err(StorageError::StaleEpoch)
    });
    assert!(failure.is_err());
    store
        .transaction(epoch, |tx| {
            tx.execute("INSERT INTO operations VALUES(2,'committed')", [])?;
            Ok(())
        })
        .unwrap();
    let result = store.transaction(epoch, |tx| {
        tx.execute("UPDATE hm_store_meta SET epoch=999", [])?;
        Ok(())
    });
    assert!(matches!(result, Err(StorageError::StaleEpoch)));
    drop(store);
    let reopened = FencedStore::open(&path, MIGRATIONS).unwrap();
    assert_eq!(reopened.epoch(), epoch + 1);
    assert_eq!(
        reopened
            .read(|db| Ok(
                db.query_row("SELECT count(*) FROM operations", [], |row| row
                    .get::<_, u32>(0))?
            ))
            .unwrap(),
        1
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            std::fs::metadata(directory.path().join("operations.sqlite.writer-lock"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}

#[test]
fn ordered_checksummed_migrations_and_ahead_refusal() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("migration.sqlite");
    assert!(matches!(
        FencedStore::open(
            &path,
            &[Migration {
                version: 2,
                name: "gap",
                sql: "SELECT 1;"
            }]
        ),
        Err(StorageError::MigrationMismatch)
    ));
    drop(FencedStore::open(&path, MIGRATIONS).unwrap());
    assert!(matches!(
        FencedStore::open(&path, &[]),
        Err(StorageError::StoreAhead {
            store: 1,
            supported: 0
        })
    ));
    assert!(matches!(
        FencedStore::open(
            &path,
            &[Migration {
                version: 1,
                name: "changed",
                sql: MIGRATIONS[0].sql
            }]
        ),
        Err(StorageError::MigrationMismatch)
    ));
    let migrations = [
        MIGRATIONS[0],
        Migration {
            version: 2,
            name: "index",
            sql: "CREATE INDEX operation_values ON operations(value);",
        },
    ];
    let store = FencedStore::open(&path, &migrations).unwrap();
    assert_eq!(store.epoch(), 2);
    drop(store);
    let bad = [
        migrations[0],
        migrations[1],
        Migration {
            version: 3,
            name: "invalid",
            sql: "CREATE TABLE half_applied(id INTEGER); INSERT INTO no_such_table VALUES(1);",
        },
    ];
    assert!(FencedStore::open(&path, &bad).is_err());
    let store = FencedStore::open(&path, &migrations).unwrap();
    assert_eq!(
        store
            .read(|db| Ok(db.query_row(
                "SELECT count(*) FROM sqlite_master WHERE name='half_applied'",
                [],
                |row| row.get::<_, u32>(0)
            )?))
            .unwrap(),
        0
    );
}

#[test]
fn crash_child() {
    let Some(path) = std::env::var_os("HM_STORAGE_CRASH_PATH") else {
        return;
    };
    let mut store = FencedStore::open(path, MIGRATIONS).unwrap();
    let epoch = store.epoch();
    store
        .transaction(epoch, |tx| {
            tx.execute("INSERT INTO operations VALUES(1,'durable')", [])?;
            Ok(())
        })
        .unwrap();
    let _: Result<(), StorageError> = store.transaction(epoch, |tx| {
        tx.execute("INSERT INTO operations VALUES(2,'interrupted')", [])?;
        std::process::exit(73);
    });
}

#[test]
fn process_crash_rolls_back_and_releases_writer() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("crash.sqlite");
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "crash_child", "--nocapture"])
        .env("HM_STORAGE_CRASH_PATH", &path)
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(73));
    let store = FencedStore::open(path, MIGRATIONS).unwrap();
    assert_eq!(store.epoch(), 2);
    assert_eq!(
        store
            .read(|db| Ok(
                db.query_row("SELECT count(*) FROM operations", [], |row| row
                    .get::<_, u32>(0))?
            ))
            .unwrap(),
        1
    );
}

#[cfg(unix)]
#[test]
fn replaced_database_refuses_old_handle() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("original.sqlite");
    let mut original = FencedStore::open(&path, MIGRATIONS).unwrap();
    let replacement_path = directory.path().join("replacement.sqlite");
    let replacement = FencedStore::open(&replacement_path, MIGRATIONS).unwrap();
    drop(replacement);
    std::fs::rename(replacement_path, path).unwrap();
    assert!(matches!(
        original.transaction(original.epoch(), |_| Ok(())),
        Err(StorageError::StaleEpoch)
    ));
}

#[cfg(unix)]
#[test]
fn linked_database_and_replaced_lock_refused() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("operations.sqlite");
    let mut store = FencedStore::open(&path, MIGRATIONS).unwrap();
    let alias = directory.path().join("alias.sqlite");
    std::fs::hard_link(&path, &alias).unwrap();
    assert!(matches!(
        FencedStore::open(&alias, MIGRATIONS),
        Err(StorageError::InvalidPath)
    ));
    std::fs::remove_file(&alias).unwrap();
    let lock = directory.path().join("operations.sqlite.writer-lock");
    std::fs::remove_file(&lock).unwrap();
    std::fs::write(&lock, b"").unwrap();
    assert!(matches!(
        store.transaction(store.epoch(), |_| Ok(())),
        Err(StorageError::StaleEpoch)
    ));
}
