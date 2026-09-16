use std::cell::RefCell;
use std::ffi::{CString, c_char};
use std::ptr;

/// Boundary status tier. Values `0` through `6` are faults raised at the ABI
/// edge; `HM_STATUS_KERNEL` means the kernel itself returned an error and the
/// numeric `hm_core::ErrorCode` discriminant travels separately.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum HmStatus {
    Ok = 0,
    NullPointer = 1,
    InvalidUtf8 = 2,
    InvalidArgument = 3,
    HandleClosed = 4,
    Runtime = 5,
    Panic = 6,
    Kernel = 7,
}

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

pub(crate) fn set_last_error(message: impl Into<String>) {
    let mut text = message.into();
    text.retain(|character| character != '\0');
    let value = CString::new(text).unwrap_or_default();
    LAST_ERROR.with(|slot| {
        *slot.borrow_mut() = Some(value);
    });
}

pub(crate) fn kernel_status(error: hm_core::Error) -> (HmStatus, i32, String) {
    (
        HmStatus::Kernel,
        i32::from(error.code as u8),
        error.code.as_str().to_owned(),
    )
}

/// Returns the last boundary error recorded on the calling thread, or null when
/// none was recorded. The pointer is owned by the library, is borrowed by the
/// caller, and stays valid until the next entry point runs on this thread.
#[unsafe(no_mangle)]
pub extern "C" fn hm_last_error_message() -> *const c_char {
    LAST_ERROR.with(|slot| {
        slot.borrow()
            .as_ref()
            .map_or(ptr::null(), |message| message.as_ptr())
    })
}

/// Drops the last boundary error recorded on the calling thread. Any pointer
/// previously returned by `hm_last_error_message` is dangling afterwards.
#[unsafe(no_mangle)]
pub extern "C" fn hm_last_error_clear() {
    LAST_ERROR.with(|slot| {
        *slot.borrow_mut() = None;
    });
}

/// Releases a string this library handed out as an owned `char*`.
///
/// # Safety
///
/// `value` must either be null or a pointer this library produced and that has
/// not been released yet. Passing null is a no-op.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hm_string_free(value: *mut c_char) {
    if value.is_null() {
        return;
    }
    drop(unsafe { CString::from_raw(value) });
}
