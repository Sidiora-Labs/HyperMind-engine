use hm_context::Scope;
use hm_fabric::backend_config::{
    BackendConfigError, BackendDescriptor, BusBackendDescriptor, HomeDescriptor, MissingHomePolicy,
    OperationalBackendDescriptor, SelectedBus, SelectedOperationalStore, select_backend,
    validate_descriptor,
};
use hm_fabric::bus::Grant;
use hm_fabric::bus_memory::MemoryBusLimits;
use hm_fabric::postgres::{PostgresDescriptor, PostgresSecretHandle};
use hm_fabric::storage::Migration;
use serde_json::Value;
use std::{fs, path::Path};
const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "selected_rows",
    sql: "CREATE TABLE selected_records(id BIGINT PRIMARY KEY,value TEXT NOT NULL);",
}];
fn scope() -> Scope {
    Scope {
        owner_id: "backend_owner".into(),
        project_id: "native_selection".into(),
        workspace_id: None,
    }
}
fn descriptor(path: &Path) -> BackendDescriptor {
    BackendDescriptor {
        version: 1,
        scope: scope(),
        home: HomeDescriptor {
            path: path.to_owned(),
            base: None,
            missing: MissingHomePolicy::Refuse,
        },
        operational: OperationalBackendDescriptor::Sqlite,
        bus: BusBackendDescriptor::Sqlite,
    }
}
fn no_secret(_: &str) -> Result<PostgresSecretHandle, BackendConfigError> {
    Err(BackendConfigError::SecretUnavailable)
}
fn grant() -> Grant {
    Grant {
        principal: "worker".into(),
        stream: "events".into(),
        publish: true,
        subscribe: true,
        register: true,
    }
}

#[test]
fn explicit_home_resolution_and_descriptor_refusal() {
    let directory = tempfile::tempdir().unwrap();
    let existing = descriptor(directory.path());
    let canonical = validate_descriptor(&existing).unwrap();
    let mut relative = existing.clone();
    relative.home.path = ".".into();
    assert!(matches!(
        validate_descriptor(&relative),
        Err(BackendConfigError::RelativeBaseRequired)
    ));
    relative.home.base = Some(directory.path().to_owned());
    assert_eq!(
        validate_descriptor(&relative).unwrap().identity_digest(),
        canonical.identity_digest()
    );
    relative.home.path = "../escape".into();
    assert!(validate_descriptor(&relative).is_err());
    let missing = directory.path().join("created/nested");
    let mut future = descriptor(&missing);
    assert!(matches!(
        validate_descriptor(&future),
        Err(BackendConfigError::MissingHome)
    ));
    future.home.missing = MissingHomePolicy::Create { max_missing: 1 };
    assert!(validate_descriptor(&future).is_err());
    future.home.missing = MissingHomePolicy::Create { max_missing: 2 };
    let validated = validate_descriptor(&future).unwrap();
    assert_eq!(validated.missing_components(), 2);
    assert!(!missing.exists());
    let mut selected = select_backend(validated, MIGRATIONS, no_secret).unwrap();
    assert_eq!(selected.home(), missing);
    assert_eq!(selected.readiness().unwrap().operational_backend, "sqlite");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&missing).unwrap().permissions().mode() & 0o777,
            0o700
        );
        let alias = directory.path().join("alias");
        std::os::unix::fs::symlink(directory.path(), &alias).unwrap();
        assert_eq!(
            validate_descriptor(&descriptor(&alias))
                .unwrap()
                .identity_digest(),
            canonical.identity_digest()
        );
    }
    let mut unsupported = existing;
    unsupported.version = 99;
    assert!(validate_descriptor(&unsupported).is_err());
}

#[test]
fn real_sqlite_selection_scope_binding_and_restart() {
    let directory = tempfile::tempdir().unwrap();
    let descriptor = descriptor(directory.path());
    let mut selected = select_backend(
        validate_descriptor(&descriptor).unwrap(),
        MIGRATIONS,
        no_secret,
    )
    .unwrap();
    assert!(
        select_backend(
            validate_descriptor(&descriptor).unwrap(),
            MIGRATIONS,
            no_secret
        )
        .is_err()
    );
    let ready = selected.readiness().unwrap();
    assert!(ready.owner_epoch > 0 && ready.operational_epoch > 0);
    assert!(ready.bus_capabilities.durable);
    let SelectedOperationalStore::Sqlite(store) = &mut selected.operational else {
        panic!("wrong actual operational backend")
    };
    store
        .transaction(store.epoch(), |tx| {
            tx.execute("INSERT INTO selected_records VALUES(1,'persisted')", [])?;
            Ok(())
        })
        .unwrap();
    let SelectedBus::Sqlite(bus) = &mut selected.bus else {
        panic!("wrong actual bus backend")
    };
    bus.grant(grant()).unwrap();
    let event = bus
        .append("worker", "events", b"actual-sqlite-bus")
        .unwrap();
    assert_eq!(bus.replay("worker", "events", 0, 10).unwrap(), vec![event]);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for file in [
            "backend-owner.sqlite",
            "backend-operations.sqlite",
            "backend-bus.sqlite",
            "backend-bus.sqlite.writer-lock",
        ] {
            assert_eq!(
                fs::metadata(directory.path().join(file))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
    }
    drop(selected);
    let mut changed = descriptor.clone();
    changed.scope.project_id = "another_project".into();
    assert!(matches!(
        select_backend(
            validate_descriptor(&changed).unwrap(),
            MIGRATIONS,
            no_secret
        ),
        Err(BackendConfigError::IdentityMismatch)
    ));
    let mut reopened = select_backend(
        validate_descriptor(&descriptor).unwrap(),
        MIGRATIONS,
        no_secret,
    )
    .unwrap();
    assert!(reopened.readiness().unwrap().operational_epoch > ready.operational_epoch);
    let SelectedOperationalStore::Sqlite(store) = &mut reopened.operational else {
        panic!("wrong backend")
    };
    assert_eq!(
        store
            .read(|db| Ok(db.query_row(
                "SELECT value FROM selected_records WHERE id=1",
                [],
                |row| row.get::<_, String>(0)
            )?))
            .unwrap(),
        "persisted"
    );
    let SelectedBus::Sqlite(bus) = &mut reopened.bus else {
        panic!("wrong bus")
    };
    assert_eq!(bus.replay("worker", "events", 0, 10).unwrap().len(), 1);
}

#[test]
fn actual_process_local_bus_and_unavailable_refusal() {
    let directory = tempfile::tempdir().unwrap();
    let mut descriptor = descriptor(directory.path());
    descriptor.bus = BusBackendDescriptor::ProcessLocal {
        limits: MemoryBusLimits::default(),
    };
    let mut selected = select_backend(
        validate_descriptor(&descriptor).unwrap(),
        MIGRATIONS,
        no_secret,
    )
    .unwrap();
    assert!(!selected.readiness().unwrap().bus_capabilities.durable);
    let SelectedBus::ProcessLocal(bus) = &selected.bus else {
        panic!("wrong bus")
    };
    bus.grant(grant()).unwrap();
    let source_reference = hm_context::types::digest_bytes(b"actual-process-local-bus");
    let event = bus
        .append("worker", "events", source_reference.as_bytes())
        .unwrap();
    assert_eq!(bus.replay("worker", "events", 0, 10).unwrap(), vec![event]);
    let unavailable_home = directory.path().join("unavailable");
    let mut unavailable = descriptor.clone();
    unavailable.home.path = unavailable_home.clone();
    unavailable.home.missing = MissingHomePolicy::Create { max_missing: 1 };
    unavailable.bus = BusBackendDescriptor::Nats {
        server_url: "nats://127.0.0.1:4222".into(),
    };
    assert!(
        matches!(select_backend(validate_descriptor(&unavailable).unwrap(),MIGRATIONS,no_secret),Err(BackendConfigError::Unavailable{backend,..}) if backend=="nats")
    );
    assert!(!unavailable_home.exists());
    unavailable.bus = BusBackendDescriptor::Nats {
        server_url: "nats://user:password@127.0.0.1:4222".into(),
    };
    assert!(validate_descriptor(&unavailable).is_err());
    let mut serialized = serde_json::to_value(&descriptor).unwrap();
    serialized["password"] = serde_json::json!("inline-credential");
    assert!(serde_json::from_value::<BackendDescriptor>(serialized).is_err());
}

#[test]
fn actual_postgres_selection_and_opaque_secret_resolution() {
    let path = std::env::var_os("HM_POSTGRES_TEST_CONFIG")
        .expect("actual pre-provisioned PostgreSQL configuration is required");
    let config: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let mut descriptor = descriptor(directory.path());
    let postgres = PostgresDescriptor {
        scope: scope(),
        host: config["host"].as_str().unwrap().into(),
        port: config["port"].as_u64().unwrap() as u16,
        user: config["user"].as_str().unwrap().into(),
        database: config["databases"]["effects"].as_str().unwrap().into(),
        schema: config["schema"].as_str().unwrap().into(),
        namespace: "native_selection_v1".into(),
    };
    descriptor.operational = OperationalBackendDescriptor::Postgres {
        descriptor: postgres.clone(),
        secret_ref: "operational_database".into(),
    };
    let mut mismatch = descriptor.clone();
    if let OperationalBackendDescriptor::Postgres {
        descriptor: postgres,
        ..
    } = &mut mismatch.operational
    {
        postgres.scope.project_id = "wrong_scope".into();
    }
    assert!(matches!(
        validate_descriptor(&mismatch),
        Err(BackendConfigError::IdentityMismatch)
    ));
    assert!(matches!(
        select_backend(
            validate_descriptor(&descriptor).unwrap(),
            MIGRATIONS,
            no_secret
        ),
        Err(BackendConfigError::SecretUnavailable)
    ));
    assert!(!directory.path().join("backend-owner.sqlite").exists());
    let mut calls = 0;
    let mut resolve = |reference: &str| {
        assert_eq!(reference, "operational_database");
        calls += 1;
        let password = fs::read_to_string(config["password_file"].as_str().unwrap())
            .unwrap()
            .trim_end()
            .splitn(5, ':')
            .nth(4)
            .unwrap()
            .to_owned();
        Ok(PostgresSecretHandle::new(password))
    };
    let mut selected = select_backend(
        validate_descriptor(&descriptor).unwrap(),
        MIGRATIONS,
        &mut resolve,
    )
    .unwrap();
    let ready = selected.readiness().unwrap();
    assert_eq!(ready.operational_backend, "postgresql");
    assert!(ready.operational_epoch > 0);
    let SelectedOperationalStore::Postgres(store) = &mut selected.operational else {
        panic!("wrong backend")
    };
    store.transaction(store.epoch(),|tx|{tx.execute("INSERT INTO selected_records VALUES(1,'actual-selected-postgres') ON CONFLICT(id) DO UPDATE SET value=EXCLUDED.value",&[])?;Ok(())}).unwrap();
    let SelectedBus::Sqlite(bus) = &mut selected.bus else {
        panic!("wrong actual local bus")
    };
    bus.grant(grant()).unwrap();
    bus.append("worker", "events", b"selected-postgres-bus-event")
        .unwrap();
    drop(selected);
    let mut reopened = select_backend(
        validate_descriptor(&descriptor).unwrap(),
        MIGRATIONS,
        &mut resolve,
    )
    .unwrap();
    assert!(reopened.readiness().unwrap().operational_epoch > ready.operational_epoch);
    let SelectedOperationalStore::Postgres(store) = &mut reopened.operational else {
        panic!("wrong backend")
    };
    assert_eq!(
        store
            .read(store.epoch(), |tx| Ok(tx
                .query_one("SELECT value FROM selected_records WHERE id=1", &[])?
                .get::<_, String>(0)))
            .unwrap(),
        "actual-selected-postgres"
    );
    store
        .transaction(store.epoch(), |tx| {
            tx.execute("DELETE FROM selected_records WHERE id=1", &[])?;
            Ok(())
        })
        .unwrap();
    drop(reopened);
    drop(resolve);
    assert_eq!(calls, 2);
    assert!(
        !serde_json::to_string(&descriptor)
            .unwrap()
            .contains("password_file")
    );
}
