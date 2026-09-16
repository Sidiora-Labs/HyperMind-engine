//! C application binary interface over the embedded kernel.

mod engine;
mod status;

pub use engine::{HmEngine, hm_engine_clone, hm_engine_close, hm_engine_free, hm_engine_open};
pub use status::{HmStatus, hm_last_error_clear, hm_last_error_message, hm_string_free};

/// Major component of the wire-visible ABI version. It changes when a symbol is
/// removed or a signature changes.
pub const HM_ABI_VERSION_MAJOR: u32 = 1;

/// Minor component of the wire-visible ABI version. It changes when a symbol is
/// added.
pub const HM_ABI_VERSION_MINOR: u32 = 0;

/// Every `extern "C"` symbol this library exports, sorted by name. The header is
/// locked against this table by `tests/header.rs`.
pub const HM_EXPORTED_SYMBOLS: [&str; 8] = [
    "hm_abi_version",
    "hm_engine_clone",
    "hm_engine_close",
    "hm_engine_free",
    "hm_engine_open",
    "hm_last_error_clear",
    "hm_last_error_message",
    "hm_string_free",
];

/// Every boundary status enumerator with its wire value, in declaration order.
/// Values are append-only.
pub const HM_STATUS_TABLE: [(&str, i32); 8] = [
    ("HM_STATUS_OK", 0),
    ("HM_STATUS_NULL_POINTER", 1),
    ("HM_STATUS_INVALID_UTF8", 2),
    ("HM_STATUS_INVALID_ARGUMENT", 3),
    ("HM_STATUS_HANDLE_CLOSED", 4),
    ("HM_STATUS_RUNTIME", 5),
    ("HM_STATUS_PANIC", 6),
    ("HM_STATUS_KERNEL", 7),
];

/// Returns the ABI version as `(major << 16) | minor`.
#[unsafe(no_mangle)]
pub extern "C" fn hm_abi_version() -> u32 {
    (HM_ABI_VERSION_MAJOR << 16) | HM_ABI_VERSION_MINOR
}
