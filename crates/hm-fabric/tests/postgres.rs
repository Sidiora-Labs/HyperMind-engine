use hm_context::types::Scope;
use hm_fabric::postgres::{
    PostgresDescriptor, PostgresError, PostgresOperationalStore, PostgresSecretHandle, WriterStatus,
};
use hm_fabric::storage::Migration;
use postgres::{Config, NoTls};
use serde_json::Value;
use std::{fs, path::PathBuf, time::Duration};
const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "operational_rows",
    sql: "CREATE TABLE backend_operations(id BIGINT PRIMARY KEY,value TEXT NOT NULL);",
}];

fn settings() -> (PostgresDescriptor, PostgresSecretHandle, Config) {
    let path = std::env::var_os("HM_POSTGRES_TEST_CONFIG")
        .expect("actual PostgreSQL test configuration is required");
    let config: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let password = fs::read_to_string(config["password_file"].as_str().unwrap())
        .unwrap()
        .trim_end()
        .splitn(5, ':')
        .nth(4)
        .unwrap()
        .to_owned();
    let descriptor = PostgresDescriptor {
        scope: Scope {
            owner_id: "test_owner".into(),
            project_id: "postgres_operational".into(),
            workspace_id: None,
        },
        host: config["host"].as_str().unwrap().into(),
        port: config["port"].as_u64().unwrap() as u16,
        user: config["user"].as_str().unwrap().into(),
        database: config["databases"]["operations"].as_str().unwrap().into(),
        schema: config["schema"].as_str().unwrap().into(),
        namespace: "storage_contract_v1".into(),
    };
    let mut driver = Config::new();
    driver
        .host(&descriptor.host)
        .port(descriptor.port)
        .user(&descriptor.user)
        .dbname(&descriptor.database)
        .password(&password)
        .connect_timeout(Duration::from_secs(5));
    (descriptor, PostgresSecretHandle::new(password), driver)
}
fn open() -> PostgresOperationalStore {
    let (descriptor, secret, _) = settings();
    PostgresOperationalStore::open(&descriptor, &secret, MIGRATIONS).unwrap()
}
fn count(store: &mut PostgresOperationalStore) -> i64 {
    let epoch = store.epoch();
    store
        .read(epoch, |tx| {
            Ok(tx
                .query_one("SELECT count(*) FROM backend_operations", &[])?
                .get(0))
        })
        .unwrap()
}

#[test]
fn process_interruption_child() {
    if std::env::var_os("HM_POSTGRES_CHILD_MODE").is_none() {
        return;
    }
    let mut store = open();
    let epoch = store.epoch();
    store.transaction(epoch,|tx| {tx.execute("INSERT INTO backend_operations VALUES(41,'durable-before-exit') ON CONFLICT(id) DO UPDATE SET value=EXCLUDED.value",&[])?;Ok(())}).unwrap();
    let _: Result<(), PostgresError> = store.transaction(epoch, |tx| {
        tx.execute(
            "INSERT INTO backend_operations VALUES(42,'uncommitted-exit')",
            &[],
        )?;
        std::process::exit(74)
    });
}

#[test]
fn actual_postgres_sessions_fences_migrations_crash_and_restart() {
    let (descriptor, secret, driver) = settings();
    let mut invalid = descriptor.clone();
    invalid.host = "203.0.113.10".into();
    assert!(matches!(
        PostgresOperationalStore::open(&invalid, &secret, MIGRATIONS),
        Err(PostgresError::InvalidDescriptor)
    ));
    assert!(matches!(
        PostgresOperationalStore::open(
            &descriptor,
            &secret,
            &[Migration {
                version: 2,
                name: "gap",
                sql: "SELECT 1;"
            }]
        ),
        Err(PostgresError::MigrationMismatch)
    ));
    let mut store = open();
    let initial_epoch = store.epoch();
    store
        .transaction(initial_epoch, |tx| {
            tx.execute("DELETE FROM backend_operations", &[])?;
            Ok(())
        })
        .unwrap();
    assert!(matches!(
        PostgresOperationalStore::open(&descriptor, &secret, MIGRATIONS),
        Err(PostgresError::Busy)
    ));
    assert!(matches!(
        store.transaction(initial_epoch + 1, |_| Ok(())),
        Err(PostgresError::StaleEpoch)
    ));
    assert_eq!(store.status(), WriterStatus::Live);
    assert!(
        store
            .read(initial_epoch, |tx| {
                tx.execute(
                    "INSERT INTO backend_operations VALUES(10,'read-only violation')",
                    &[],
                )?;
                Ok(())
            })
            .is_err()
    );
    let rollback: Result<(), PostgresError> = store.transaction(initial_epoch, |tx| {
        tx.execute("INSERT INTO backend_operations VALUES(11,'rollback')", &[])?;
        Err(PostgresError::MigrationMismatch)
    });
    assert!(rollback.is_err());
    assert_eq!(count(&mut store), 0);
    store
        .transaction(initial_epoch, |tx| {
            tx.execute("INSERT INTO backend_operations VALUES(12,'committed')", &[])?;
            Ok(())
        })
        .unwrap();
    drop(store);
    assert!(matches!(
        PostgresOperationalStore::open(&descriptor, &secret, &[]),
        Err(PostgresError::StoreAhead {
            store: 1,
            supported: 0
        })
    ));
    assert!(matches!(
        PostgresOperationalStore::open(
            &descriptor,
            &secret,
            &[Migration {
                version: 1,
                name: "changed",
                sql: MIGRATIONS[0].sql
            }]
        ),
        Err(PostgresError::MigrationMismatch)
    ));
    let mut foreign = descriptor.clone();
    foreign.scope.project_id = "different_project".into();
    assert!(matches!(
        PostgresOperationalStore::open(&foreign, &secret, MIGRATIONS),
        Err(PostgresError::IdentityMismatch)
    ));
    let failing = [
        MIGRATIONS[0],
        Migration {
            version: 2,
            name: "invalid_atomic",
            sql: "CREATE TABLE half_migrated(id BIGINT); INSERT INTO absent_table VALUES(1);",
        },
    ];
    assert!(PostgresOperationalStore::open(&descriptor, &secret, &failing).is_err());
    let mut reopened = open();
    assert_eq!(reopened.epoch(), initial_epoch + 1);
    assert_eq!(count(&mut reopened), 1);
    let epoch = reopened.epoch();
    assert!(!reopened.read(epoch,|tx|Ok(tx.query_one("SELECT EXISTS(SELECT 1 FROM pg_class WHERE relname='half_migrated' AND relnamespace=current_schema()::regnamespace)",&[])?.get::<_,bool>(0))).unwrap());
    let mut other = driver.connect(NoTls).unwrap();
    other
        .batch_execute(&format!(
            "SET search_path TO \"{}\",pg_catalog",
            descriptor.schema
        ))
        .unwrap();
    other
        .execute(
            "UPDATE hm_postgres_meta SET epoch=epoch+1 WHERE singleton=1",
            &[],
        )
        .unwrap();
    assert!(matches!(
        reopened.transaction(epoch, |tx| {
            tx.execute("INSERT INTO backend_operations VALUES(13,'stale')", &[])?;
            Ok(())
        }),
        Err(PostgresError::StaleEpoch)
    ));
    assert_eq!(reopened.status(), WriterStatus::Stale);
    assert_eq!(reopened.epoch(), 0);
    drop(reopened);
    let mut live = open();
    let epoch = live.epoch();
    let pid: i32 = live
        .read(epoch, |tx| {
            Ok(tx.query_one("SELECT pg_backend_pid()", &[])?.get(0))
        })
        .unwrap();
    assert!(
        other
            .query_one("SELECT pg_terminate_backend($1)", &[&pid])
            .unwrap()
            .get::<_, bool>(0)
    );
    assert!(matches!(
        live.transaction(epoch, |_| Ok(())),
        Err(PostgresError::ConnectionLost) | Err(PostgresError::Driver(_))
    ));
    assert_eq!(live.status(), WriterStatus::Lost);
    assert_eq!(live.epoch(), 0);
    drop(live);
    drop(other);
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "process_interruption_child", "--nocapture"])
        .env("HM_POSTGRES_CHILD_MODE", "crash")
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(74));
    let mut crash_reopened = open();
    assert_eq!(count(&mut crash_reopened), 2);
    let crash_epoch = crash_reopened.epoch();
    let restart = PathBuf::from(
        std::env::var_os("HM_POSTGRES_RESTART_SCRIPT")
            .expect("owned PostgreSQL restart script is required"),
    );
    assert!(
        std::process::Command::new(restart)
            .status()
            .unwrap()
            .success()
    );
    assert!(matches!(
        crash_reopened.transaction(crash_epoch, |_| Ok(())),
        Err(PostgresError::ConnectionLost) | Err(PostgresError::Driver(_))
    ));
    assert_eq!(crash_reopened.status(), WriterStatus::Lost);
    drop(crash_reopened);
    let mut final_store = open();
    assert!(final_store.epoch() > crash_epoch);
    assert_eq!(count(&mut final_store), 2);
    let epoch = final_store.epoch();
    let admin_path = std::env::var_os("HM_POSTGRES_ADMIN_CONFIG")
        .expect("private administrator configuration is required");
    let admin: Value = serde_json::from_slice(&fs::read(admin_path).unwrap()).unwrap();
    let mut privileged_descriptor = descriptor.clone();
    privileged_descriptor.user = admin["user"].as_str().unwrap().into();
    let privileged_password = fs::read_to_string(admin["password_file"].as_str().unwrap())
        .unwrap()
        .trim_end()
        .splitn(5, ':')
        .nth(4)
        .unwrap()
        .to_owned();
    assert!(matches!(
        PostgresOperationalStore::open(
            &privileged_descriptor,
            &PostgresSecretHandle::new(privileged_password),
            MIGRATIONS
        ),
        Err(PostgresError::ProvisioningRequired)
    ));
    final_store
        .transaction(epoch, |tx| {
            tx.execute("DELETE FROM backend_operations", &[])?;
            Ok(())
        })
        .unwrap();
}
