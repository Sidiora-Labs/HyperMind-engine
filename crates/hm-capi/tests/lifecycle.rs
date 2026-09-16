use hypermind::{
    HmEngine, HmStatus, hm_abi_version, hm_engine_clone, hm_engine_close, hm_engine_free,
    hm_engine_open, hm_last_error_clear, hm_last_error_message, hm_string_free,
};
use std::ffi::CString;
use std::path::Path;
use std::ptr;

fn configuration(directory: &Path) -> CString {
    let value = serde_json::json!({
        "path": directory.to_str().expect("temporary directory path is UTF-8"),
        "actor": 7,
        "user_hex": "11".repeat(16),
        "kek_hex": "22".repeat(32),
        "projection_map_bytes": 16_777_216,
    });
    CString::new(value.to_string()).expect("configuration JSON has no interior NUL")
}

fn open(configuration: &CString) -> (HmStatus, *mut HmEngine) {
    let mut engine: *mut HmEngine = ptr::null_mut();
    let status = unsafe { hm_engine_open(configuration.as_ptr(), &raw mut engine) };
    (status, engine)
}

#[test]
fn open_then_close_round_trip() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let request = configuration(directory.path());
    let (status, engine) = open(&request);
    assert_eq!(status, HmStatus::Ok);
    assert!(!engine.is_null());
    assert!(directory.path().join("7").exists());

    assert_eq!(unsafe { hm_engine_close(engine) }, HmStatus::Ok);
    assert_eq!(unsafe { hm_engine_close(engine) }, HmStatus::Ok);
    unsafe { hm_engine_free(engine) };

    let (status, reopened) = open(&request);
    assert_eq!(status, HmStatus::Ok);
    assert!(!reopened.is_null());
    assert_eq!(unsafe { hm_engine_close(reopened) }, HmStatus::Ok);
    unsafe { hm_engine_free(reopened) };
}

#[test]
fn clone_shares_state() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let request = configuration(directory.path());
    let (status, engine) = open(&request);
    assert_eq!(status, HmStatus::Ok);

    let clone = unsafe { hm_engine_clone(engine) };
    assert!(!clone.is_null());
    assert!(!ptr::eq(clone, engine));
    unsafe { hm_engine_free(clone) };

    assert_eq!(unsafe { hm_engine_close(engine) }, HmStatus::Ok);
    unsafe { hm_engine_free(engine) };

    assert!(unsafe { hm_engine_clone(ptr::null()) }.is_null());
}

#[test]
fn rejects_bad_configuration() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().to_str().expect("path is UTF-8");
    let user = "11".repeat(16);
    let kek = "22".repeat(32);
    let cases: [(CString, HmStatus); 5] = [
        (
            CString::new(vec![0x7b_u8, 0xff, 0xfe, 0x7d]).expect("no interior NUL"),
            HmStatus::InvalidUtf8,
        ),
        (CString::new("{").expect("no interior NUL"), HmStatus::InvalidArgument),
        (
            CString::new(format!(
                "{{\"path\":\"{path}\",\"actor\":0,\"user_hex\":\"{user}\",\"kek_hex\":\"{kek}\"}}"
            ))
            .expect("no interior NUL"),
            HmStatus::InvalidArgument,
        ),
        (
            CString::new(format!(
                "{{\"path\":\"{path}\",\"actor\":7,\"user_hex\":\"{user}\",\"kek_hex\":\"{kek}\",\"extra\":1}}"
            ))
            .expect("no interior NUL"),
            HmStatus::InvalidArgument,
        ),
        (
            CString::new(format!(
                "{{\"path\":\"{path}\",\"actor\":7,\"user_hex\":\"{}\",\"kek_hex\":\"{kek}\"}}",
                &user[..30]
            ))
            .expect("no interior NUL"),
            HmStatus::InvalidArgument,
        ),
    ];

    for (request, expected) in &cases {
        hm_last_error_clear();
        let mut engine: *mut HmEngine = ptr::null_mut();
        let status = unsafe { hm_engine_open(request.as_ptr(), &raw mut engine) };
        assert_eq!(status, *expected, "{request:?}");
        assert!(engine.is_null(), "{request:?}");
        assert!(!hm_last_error_message().is_null(), "{request:?}");
    }

    hm_last_error_clear();
    let mut engine: *mut HmEngine = ptr::null_mut();
    assert_eq!(
        unsafe { hm_engine_open(ptr::null(), &raw mut engine) },
        HmStatus::NullPointer
    );
    assert!(engine.is_null());
    assert!(!hm_last_error_message().is_null());

    hm_last_error_clear();
    let valid = configuration(directory.path());
    assert_eq!(
        unsafe { hm_engine_open(valid.as_ptr(), ptr::null_mut()) },
        HmStatus::NullPointer
    );
    assert!(!hm_last_error_message().is_null());

    hm_last_error_clear();
    assert!(hm_last_error_message().is_null());
}

#[test]
fn null_frees_are_noops() {
    unsafe { hm_engine_free(ptr::null_mut()) };
    unsafe { hm_string_free(ptr::null_mut()) };

    let owned = CString::new("released through the C string contract")
        .expect("no interior NUL")
        .into_raw();
    unsafe { hm_string_free(owned) };
}

#[test]
fn abi_version_packs_the_major_and_minor_components() {
    assert_eq!(hm_abi_version(), (1 << 16) | 1);
}

#[test]
fn close_inside_runtime_is_refused() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let request = configuration(directory.path());
    let (status, engine) = open(&request);
    assert_eq!(status, HmStatus::Ok);

    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    let refused = runtime.block_on(async { unsafe { hm_engine_close(engine) } });
    assert_eq!(refused, HmStatus::Runtime);
    drop(runtime);

    assert_eq!(unsafe { hm_engine_close(engine) }, HmStatus::Ok);
    unsafe { hm_engine_free(engine) };
}
