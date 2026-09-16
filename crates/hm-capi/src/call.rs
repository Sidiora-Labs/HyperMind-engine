#![allow(clippy::missing_panics_doc)]

use crate::engine::HmEngine;
use crate::status::{self, HmStatus, set_last_error};
use hm_serve::uds::ToolDispatcher;
use std::ffi::{CStr, CString, c_char, c_void};
use std::ptr;

/// Receives the outcome of exactly one `hm_engine_call`.
///
/// `kernel_code` carries the numeric kernel error discriminant when `status` is
/// `HmStatus::Kernel` and `-1` otherwise. `result_json` is the serialized tool
/// envelope on success and null on failure; `error_message` is the stable
/// kernel error name on a kernel failure and null on success. Both strings are
/// borrowed and stay valid only until the callback returns.
pub type HmResultCallback = unsafe extern "C" fn(
    status: HmStatus,
    kernel_code: i32,
    result_json: *const c_char,
    error_message: *const c_char,
    user_data: *mut c_void,
);

struct SendUserData(*mut c_void);

impl SendUserData {
    const fn pointer(&self) -> *mut c_void {
        self.0
    }
}

unsafe impl Send for SendUserData {}

fn owned(value: impl Into<Vec<u8>>) -> CString {
    let mut bytes: Vec<u8> = value.into();
    bytes.retain(|byte| *byte != 0);
    CString::new(bytes).unwrap_or_default()
}

/// Runs one tool verb against the handle's actor and reports the result through
/// `callback`.
///
/// `verb` is one of the tool names the kernel exposes and `arguments_json` is
/// that verb's arguments as a JSON document; both are borrowed for the duration
/// of the call. The call returns immediately: the work runs on the handle's own
/// runtime and the callback fires exactly once on one of that runtime's worker
/// threads. `HmStatus::Ok` is returned if and only if the callback will fire;
/// every other return value means it was never invoked and never will be, and
/// `hm_last_error_message` describes the fault on the calling thread.
///
/// # Safety
///
/// `engine` must be a handle that has not been freed yet and must outlive the
/// callback, `verb` and `arguments_json` must be NUL-terminated C strings, and
/// `user_data` must stay valid until the callback has returned.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hm_engine_call(
    engine: *const HmEngine,
    verb: *const c_char,
    arguments_json: *const c_char,
    callback: Option<HmResultCallback>,
    user_data: *mut c_void,
) -> HmStatus {
    if engine.is_null() || verb.is_null() || arguments_json.is_null() {
        set_last_error("engine, verb and arguments_json must not be null");
        return HmStatus::NullPointer;
    }
    let Some(callback) = callback else {
        set_last_error("callback must not be null");
        return HmStatus::NullPointer;
    };
    let (Ok(verb), Ok(arguments)) = (
        unsafe { CStr::from_ptr(verb) }.to_str(),
        unsafe { CStr::from_ptr(arguments_json) }.to_str(),
    ) else {
        set_last_error("verb and arguments_json must be valid UTF-8");
        return HmStatus::InvalidUtf8;
    };
    let handle = unsafe { &*engine };
    let Some(actor) = handle.inner.actor() else {
        set_last_error("the handle was already closed");
        return HmStatus::HandleClosed;
    };
    let dispatcher = handle.inner.dispatcher().clone();
    let verb = verb.to_owned();
    let arguments = arguments.as_bytes().to_vec();
    let user_data = SendUserData(user_data);
    handle.inner.runtime().handle().spawn(async move {
        let inner = tokio::spawn(async move { dispatcher.dispatch(actor, verb, arguments).await });
        let (status, kernel_code, result, message) = match inner.await {
            Ok(Ok(bytes)) => (HmStatus::Ok, -1, Some(owned(bytes)), None),
            Ok(Err(error)) => {
                let (kind, code, name) = status::kernel_status(error);
                (kind, code, None, Some(owned(name)))
            }
            Err(join) if join.is_panic() => (
                HmStatus::Panic,
                -1,
                None,
                Some(owned("the tool call panicked")),
            ),
            Err(_) => (
                HmStatus::Runtime,
                -1,
                None,
                Some(owned("the tool call was cancelled")),
            ),
        };
        let result_json = result.as_ref().map_or(ptr::null(), |value| value.as_ptr());
        let error_message = message.as_ref().map_or(ptr::null(), |value| value.as_ptr());
        unsafe {
            callback(
                status,
                kernel_code,
                result_json,
                error_message,
                user_data.pointer(),
            );
        }
    });
    HmStatus::Ok
}
