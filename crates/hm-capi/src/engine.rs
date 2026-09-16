use crate::status::{self, HmStatus, set_last_error};
use hm_core::ActorId;
use hm_mcp::dispatcher::McpToolDispatcher;
use hm_serve::actor::{ActorConfig, ActorEngine};
use serde::Deserialize;
use std::ffi::{CStr, c_char};
use std::path::Path;
use std::ptr;
use std::sync::{Arc, Mutex, PoisonError};
use tokio::runtime::{Handle, Runtime};

const fn default_projection_map_bytes() -> usize {
    268_435_456
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OpenRequest {
    path: String,
    actor: u16,
    user_hex: String,
    kek_hex: String,
    #[serde(default = "default_projection_map_bytes")]
    projection_map_bytes: usize,
    #[serde(default)]
    providers_from_environment: bool,
}

fn hex<const N: usize>(value: &str) -> Result<[u8; N], HmStatus> {
    if value.len() != N * 2 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(HmStatus::InvalidArgument);
    }
    let mut result = [0; N];
    for (index, byte) in result.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| HmStatus::InvalidArgument)?;
    }
    Ok(result)
}

pub(crate) struct EngineState {
    runtime: Runtime,
    dispatcher: McpToolDispatcher,
    actor: Mutex<Option<ActorEngine>>,
}

impl EngineState {
    pub(crate) fn actor(&self) -> Option<ActorEngine> {
        self.actor
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub(crate) fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    pub(crate) fn dispatcher(&self) -> &McpToolDispatcher {
        &self.dispatcher
    }

    fn take_actor(&self) -> Option<ActorEngine> {
        self.actor
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    }
}

/// Opaque handle over one shared embedded kernel state. Every handle is freed
/// exactly once with `hm_engine_free`; closing is a separate, idempotent step.
pub struct HmEngine {
    pub(crate) inner: Arc<EngineState>,
}

/// Opens an embedded actor from a strict JSON configuration object and writes a
/// fresh handle through `out_engine`.
///
/// The configuration object carries `path`, `actor`, `user_hex`, `kek_hex` and
/// the optional `projection_map_bytes` and `providers_from_environment` keys;
/// unknown keys are rejected. On any failure `out_engine` is left untouched and
/// `hm_last_error_message` describes the fault on the calling thread.
///
/// # Safety
///
/// `configuration_json` must be null or a NUL-terminated C string, and
/// `out_engine` must be null or point to one writable `HmEngine` pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hm_engine_open(
    configuration_json: *const c_char,
    out_engine: *mut *mut HmEngine,
) -> HmStatus {
    if configuration_json.is_null() || out_engine.is_null() {
        set_last_error("configuration_json and out_engine must not be null");
        return HmStatus::NullPointer;
    }
    let Ok(text) = (unsafe { CStr::from_ptr(configuration_json) }).to_str() else {
        set_last_error("configuration_json is not valid UTF-8");
        return HmStatus::InvalidUtf8;
    };
    let request = match serde_json::from_str::<OpenRequest>(text) {
        Ok(request) => request,
        Err(error) => {
            set_last_error(format!("configuration is not an open request: {error}"));
            return HmStatus::InvalidArgument;
        }
    };
    if request.actor == 0 {
        set_last_error("actor must be nonzero");
        return HmStatus::InvalidArgument;
    }
    let (Ok(user), Ok(kek)) = (hex::<16>(&request.user_hex), hex::<32>(&request.kek_hex)) else {
        set_last_error("user_hex and kek_hex must be 32 and 64 hexadecimal characters");
        return HmStatus::InvalidArgument;
    };
    if Handle::try_current().is_ok() {
        set_last_error("hm_engine_open blocks and cannot run inside an async runtime");
        return HmStatus::Runtime;
    }
    let runtime = match Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => {
            set_last_error(format!("could not start a runtime: {error}"));
            return HmStatus::Runtime;
        }
    };
    let dispatcher = if request.providers_from_environment {
        match McpToolDispatcher::from_env() {
            Ok(dispatcher) => dispatcher,
            Err(error) => {
                let (kind, _, message) = status::kernel_status(error);
                set_last_error(message);
                return kind;
            }
        }
    } else {
        McpToolDispatcher::default()
    };
    let config = ActorConfig {
        actor_directory: Path::new(&request.path).join(request.actor.to_string()),
        actor: ActorId::new(request.actor),
        user,
        kek,
        projection_map_bytes: request.projection_map_bytes,
    };
    let actor = match runtime.block_on(ActorEngine::open(config)) {
        Ok(actor) => actor,
        Err(error) => {
            let (kind, _, message) = status::kernel_status(error);
            set_last_error(message);
            return kind;
        }
    };
    let engine = HmEngine {
        inner: Arc::new(EngineState {
            runtime,
            dispatcher,
            actor: Mutex::new(Some(actor)),
        }),
    };
    unsafe { *out_engine = Box::into_raw(Box::new(engine)) };
    HmStatus::Ok
}

/// Returns a second handle over the same shared state, or null when `engine` is
/// null. The returned pointer is distinct and must itself be freed exactly once
/// with `hm_engine_free`.
///
/// # Safety
///
/// `engine` must be null or a handle that has not been freed yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hm_engine_clone(engine: *const HmEngine) -> *mut HmEngine {
    if engine.is_null() {
        set_last_error("engine must not be null");
        return ptr::null_mut();
    }
    let handle = unsafe { &*engine };
    Box::into_raw(Box::new(HmEngine {
        inner: Arc::clone(&handle.inner),
    }))
}

/// Shuts the actor down for every handle sharing this state and releases the
/// actor directory. The call is idempotent and blocks until the actor stops.
///
/// # Safety
///
/// `engine` must be null or a handle that has not been freed yet.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hm_engine_close(engine: *const HmEngine) -> HmStatus {
    if engine.is_null() {
        set_last_error("engine must not be null");
        return HmStatus::NullPointer;
    }
    if Handle::try_current().is_ok() {
        set_last_error("hm_engine_close blocks and cannot run inside an async runtime");
        return HmStatus::Runtime;
    }
    let handle = unsafe { &*engine };
    let Some(actor) = handle.inner.take_actor() else {
        return HmStatus::Ok;
    };
    match handle.inner.runtime().block_on(actor.shutdown()) {
        Ok(()) => HmStatus::Ok,
        Err(error) => {
            let (kind, _, message) = status::kernel_status(error);
            set_last_error(message);
            kind
        }
    }
}

/// Releases exactly one handle. The shared state is dropped once the last
/// handle over it is freed. Passing null is a no-op.
///
/// # Safety
///
/// `engine` must be null or a handle that has not been freed yet, and no call
/// may be in flight on it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hm_engine_free(engine: *mut HmEngine) {
    if engine.is_null() {
        return;
    }
    drop(unsafe { Box::from_raw(engine) });
}
