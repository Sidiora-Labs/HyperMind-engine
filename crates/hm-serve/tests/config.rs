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
