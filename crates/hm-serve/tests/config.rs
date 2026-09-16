use hm_core::ErrorCode;
use hm_serve::config::{capability_equal, load};
use std::fs;
use std::os::unix::fs::PermissionsExt;

#[test]
fn config_requires_private_mode_and_disjoint_tokens() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join("hypermind.conf");
    let valid = format!(
        "socket={}\ndata={}\nuser={}\nkek={}\nadmin_token={}\nactor=7:{}\n",
        temporary.path().join("hm.sock").display(),
        temporary.path().join("data").display(),
        "11".repeat(16),
        "22".repeat(32),
        "33".repeat(32),
        "44".repeat(32),
    );
    fs::write(&path, valid).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(load(&path).unwrap().actors[0].actor, 7);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!(load(&path).unwrap_err().code, ErrorCode::CapabilityDenied);
    assert!(capability_equal(&[9; 32], &[9; 32]));
    assert!(!capability_equal(&[9; 32], &[9; 31]));
    assert!(!capability_equal(&[9; 32], &[8; 32]));
}

#[test]
fn lease_limits_parse_with_defaults_and_bounds() {
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join("hypermind.conf");
    let base = format!(
        "socket={}\ndata={}\nuser={}\nkek={}\nadmin_token={}\nactor=7:{}\n",
        temporary.path().join("hm.sock").display(),
        temporary.path().join("data").display(),
        "11".repeat(16),
        "22".repeat(32),
        "33".repeat(32),
        "44".repeat(32),
    );
    let write = |contents: &str| {
        fs::write(&path, contents).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    };
    write(&base);
    let defaults = load(&path).unwrap();
    assert_eq!(defaults.maximum_active_actors, 64);
    assert_eq!(defaults.maximum_heavy_jobs, 2);
    assert_eq!(defaults.lease_wait_ms, 250);

    write(&format!(
        "{base}maximum_active_actors=2\nmaximum_heavy_jobs=1\nlease_wait_ms=0\n"
    ));
    let configured = load(&path).unwrap();
    assert_eq!(configured.maximum_active_actors, 2);
    assert_eq!(configured.maximum_heavy_jobs, 1);
    assert_eq!(configured.lease_wait_ms, 0);

    for rejected in [
        "maximum_active_actors=0",
        "maximum_active_actors=4097",
        "maximum_heavy_jobs=0",
        "maximum_heavy_jobs=257",
        "lease_wait_ms=60001",
        "lease_leases=1",
    ] {
        write(&format!("{base}{rejected}\n"));
        assert_eq!(load(&path).unwrap_err().code, ErrorCode::InvalidArgument);
    }
}
