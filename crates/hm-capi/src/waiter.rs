#![allow(clippy::missing_panics_doc)]

use crate::status::{HmStatus, set_last_error};
use std::ffi::{CStr, CString, c_char, c_void};
use std::ptr;
use std::sync::{Condvar, Mutex, PoisonError};
use tokio::runtime::Handle;

#[derive(Default)]
struct WaiterSlot {
    result: Option<(HmStatus, i32, Option<CString>, Option<CString>)>,
    consumed: bool,
}

/// Single-use rendezvous that turns one deferred `hm_engine_call` into a
/// blocking one for embedders that cannot host a callback.
pub struct HmWaiter {
    slot: Mutex<WaiterSlot>,
    ready: Condvar,
}

/// Allocates a waiter that carries exactly one call result. The pointer is
/// released with `hm_waiter_free` and must outlive the call it is paired with.
#[unsafe(no_mangle)]
pub extern "C" fn hm_waiter_new() -> *mut HmWaiter {
    Box::into_raw(Box::new(HmWaiter {
        slot: Mutex::new(WaiterSlot::default()),
        ready: Condvar::new(),
    }))
}

/// Records one call result into the waiter passed as `user_data` and wakes the
/// thread blocked in `hm_waiter_wait`. The signature matches `HmResultCallback`
/// exactly, so it is passed to `hm_engine_call` as the callback with the waiter
/// as its user data. Both strings are copied into storage the waiter owns.
///
/// # Safety
///
/// `user_data` must be null or a waiter that has not been freed yet, and
/// `result_json` and `error_message` must each be null or a NUL-terminated C
/// string that is valid for the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hm_waiter_callback(
    status: HmStatus,
    kernel_code: i32,
    result_json: *const c_char,
    error_message: *const c_char,
    user_data: *mut c_void,
) {
    if user_data.is_null() {
        return;
    }
    let waiter = unsafe { &*user_data.cast::<HmWaiter>() };
    let copy = |value: *const c_char| -> Option<CString> {
        (!value.is_null()).then(|| unsafe { CStr::from_ptr(value) }.to_owned())
    };
    let mut slot = waiter.slot.lock().unwrap_or_else(PoisonError::into_inner);
    slot.result = Some((status, kernel_code, copy(result_json), copy(error_message)));
    drop(slot);
    waiter.ready.notify_all();
}

/// Blocks the calling thread until the paired call reports, then hands the
/// result over.
///
/// `out_kernel_code` receives the numeric kernel error discriminant when the
/// returned status is `HmStatus::Kernel` and `-1` otherwise. `out_result_json`
/// receives an owned `char*` the caller releases with `hm_string_free`, or null
/// when the call produced no envelope. Both out-parameters are written before
/// any refusal, so a refused wait leaves a null string behind. A waiter
/// delivers exactly one result: a second wait is refused with
/// `HmStatus::InvalidArgument`, and a wait issued from inside an async runtime
/// thread is refused with `HmStatus::Runtime` rather than blocking it.
///
/// # Safety
///
/// `waiter` must be a waiter that has not been freed yet, and
/// `out_kernel_code` and `out_result_json` must each point at one writable
/// value of their type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hm_waiter_wait(
    waiter: *mut HmWaiter,
    out_kernel_code: *mut i32,
    out_result_json: *mut *mut c_char,
) -> HmStatus {
    if waiter.is_null() || out_kernel_code.is_null() || out_result_json.is_null() {
        set_last_error("waiter, out_kernel_code and out_result_json must not be null");
        return HmStatus::NullPointer;
    }
    unsafe {
        *out_kernel_code = -1;
        *out_result_json = ptr::null_mut();
    }
    if Handle::try_current().is_ok() {
        set_last_error("hm_waiter_wait blocks and cannot run inside an async runtime");
        return HmStatus::Runtime;
    }
    let handle = unsafe { &*waiter };
    let mut slot = handle.slot.lock().unwrap_or_else(PoisonError::into_inner);
    if slot.consumed {
        set_last_error("a waiter delivers one result and was already waited on");
        return HmStatus::InvalidArgument;
    }
    while slot.result.is_none() {
        slot = handle
            .ready
            .wait(slot)
            .unwrap_or_else(PoisonError::into_inner);
    }
    slot.consumed = true;
    let (status, kernel_code, result, message) =
        slot.result.take().expect("the waiter carries a result");
    drop(slot);
    unsafe { *out_kernel_code = kernel_code };
    if let Some(result) = result {
        unsafe { *out_result_json = result.into_raw() };
    }
    if let Some(message) = message {
        set_last_error(message.to_string_lossy().into_owned());
    }
    status
}

/// Releases a waiter; passing null is a no-op. Freeing must not happen while a
/// call that was given this waiter is still in flight.
///
/// # Safety
///
/// `waiter` must be null or a waiter that has not been freed yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hm_waiter_free(waiter: *mut HmWaiter) {
    if waiter.is_null() {
        return;
    }
    drop(unsafe { Box::from_raw(waiter) });
}
