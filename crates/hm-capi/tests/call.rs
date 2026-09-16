use hypermind::{
    HmEngine, HmStatus, HmWaiter, hm_engine_call, hm_engine_close, hm_engine_free, hm_engine_open,
    hm_string_free, hm_waiter_callback, hm_waiter_free, hm_waiter_new, hm_waiter_wait,
};
use serde_json::Value;
use std::ffi::{CStr, CString, c_char, c_void};
use std::path::Path;
use std::ptr;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::ThreadId;
use std::time::{Duration, Instant};

const REMEMBER: &str = r#"{"conversation":"abi-review","kind":"user","content":"The release region is eu-central-1.","retention":"durable"}"#;
const RECALL: &str = r#"{"mode":"lexical","query":"release region","limit":8}"#;
const DEADLINE: Duration = Duration::from_secs(30);

type Outcome = (i32, i32, Option<String>, Option<String>);

struct Capture {
    count: AtomicUsize,
    status: Mutex<Option<Outcome>>,
    thread: Mutex<Option<ThreadId>>,
}

impl Capture {
    fn new() -> Self {
        Self {
            count: AtomicUsize::new(0),
            status: Mutex::new(None),
            thread: Mutex::new(None),
        }
    }

    fn calls(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }

    fn outcome(&self) -> Outcome {
        self.status
            .lock()
            .expect("the capture lock is not poisoned")
            .clone()
            .expect("the callback recorded an outcome")
    }

    fn thread(&self) -> ThreadId {
        self.thread
            .lock()
            .expect("the capture lock is not poisoned")
            .expect("the callback recorded its thread")
    }

    fn user_data(&self) -> *mut c_void {
        ptr::from_ref(self).cast::<c_void>().cast_mut()
    }
}

unsafe extern "C" fn capture(
    status: HmStatus,
    kernel_code: i32,
    result_json: *const c_char,
    error_message: *const c_char,
    user_data: *mut c_void,
) {
    let state = unsafe { &*user_data.cast::<Capture>() };
    let text = |value: *const c_char| -> Option<String> {
        (!value.is_null()).then(|| {
            unsafe { CStr::from_ptr(value) }
                .to_string_lossy()
                .into_owned()
        })
    };
    *state
        .status
        .lock()
        .expect("the capture lock is not poisoned") = Some((
        status as i32,
        kernel_code,
        text(result_json),
        text(error_message),
    ));
    *state
        .thread
        .lock()
        .expect("the capture lock is not poisoned") = Some(std::thread::current().id());
    state.count.fetch_add(1, Ordering::SeqCst);
}

fn wait_for(capture: &Capture) {
    let deadline = Instant::now() + DEADLINE;
    while capture.calls() == 0 {
        assert!(
            Instant::now() < deadline,
            "the callback did not fire within {DEADLINE:?}"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

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

fn open(directory: &Path) -> *mut HmEngine {
    let request = configuration(directory);
    let mut engine: *mut HmEngine = ptr::null_mut();
    let status = unsafe { hm_engine_open(request.as_ptr(), &raw mut engine) };
    assert_eq!(status, HmStatus::Ok);
    assert!(!engine.is_null());
    engine
}

fn shut_down(engine: *mut HmEngine) {
    assert_eq!(unsafe { hm_engine_close(engine) }, HmStatus::Ok);
    unsafe { hm_engine_free(engine) };
}

fn call(engine: *mut HmEngine, verb: &str, arguments: &str, state: &Capture) -> HmStatus {
    let verb = CString::new(verb).expect("the verb has no interior NUL");
    let arguments = CString::new(arguments).expect("the arguments have no interior NUL");
    unsafe {
        hm_engine_call(
            engine,
            verb.as_ptr(),
            arguments.as_ptr(),
            Some(capture),
            state.user_data(),
        )
    }
}

fn envelope(engine: *mut HmEngine, verb: &str, arguments: &str) -> Value {
    let state = Capture::new();
    assert_eq!(call(engine, verb, arguments, &state), HmStatus::Ok);
    wait_for(&state);
    let (status, kernel_code, result, message) = state.outcome();
    assert_eq!(status, HmStatus::Ok as i32);
    assert_eq!(kernel_code, -1);
    assert!(message.is_none(), "{message:?}");
    let result = result.expect("a successful call delivers an envelope");
    serde_json::from_str(&result).expect("the callback delivered a JSON envelope")
}

#[test]
fn remember_then_recall_through_the_abi() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let engine = open(directory.path());

    let remembered = envelope(engine, "remember", REMEMBER);
    assert_eq!(remembered["ok"], Value::Bool(true), "{remembered}");

    let recalled = envelope(engine, "recall", RECALL);
    assert_eq!(recalled["ok"], Value::Bool(true), "{recalled}");
    let items = recalled["items"]
        .as_array()
        .expect("the envelope carries an items array");
    assert!(!items.is_empty(), "{recalled}");
    let provenance = recalled["provenance"]
        .as_array()
        .expect("the envelope carries a provenance array");
    assert!(!provenance.is_empty(), "{recalled}");
    for entry in provenance {
        let entry = entry.as_str().expect("a provenance entry is a string");
        assert!(entry.starts_with("hm://7/"), "{entry}");
    }

    shut_down(engine);
}

#[test]
fn mutation_error_keeps_the_envelope_contract() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let engine = open(directory.path());

    let refused = envelope(engine, "remember", r#"{"conversation":"abi-review"}"#);
    assert_eq!(refused["ok"], Value::Bool(false), "{refused}");
    assert_eq!(
        refused["items"][0]["error"], "kInvalidArgument",
        "{refused}"
    );
    assert!(refused["effect_state"].is_string(), "{refused}");

    shut_down(engine);
}

#[test]
fn unknown_verb_is_a_kernel_error() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let engine = open(directory.path());

    let state = Capture::new();
    assert_eq!(call(engine, "hm_not_a_verb", "{}", &state), HmStatus::Ok);
    wait_for(&state);
    let (status, kernel_code, result, message) = state.outcome();
    assert_eq!(status, HmStatus::Kernel as i32);
    assert_eq!(kernel_code, hm_core::ErrorCode::CapabilityDenied as i32);
    assert_eq!(kernel_code, 47);
    assert_eq!(message.as_deref(), Some("kCapabilityDenied"));
    assert!(result.is_none(), "{result:?}");
    assert_eq!(state.calls(), 1);

    shut_down(engine);
}

#[test]
fn callback_runs_off_the_calling_thread() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let engine = open(directory.path());

    let state = Capture::new();
    assert_eq!(call(engine, "recall", RECALL, &state), HmStatus::Ok);
    wait_for(&state);
    assert_ne!(state.thread(), std::thread::current().id());

    shut_down(engine);
}

#[test]
fn callback_fires_exactly_once() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let engine = open(directory.path());

    let state = Capture::new();
    assert_eq!(call(engine, "remember", REMEMBER, &state), HmStatus::Ok);
    wait_for(&state);
    std::thread::sleep(Duration::from_millis(250));
    assert_eq!(state.calls(), 1);

    shut_down(engine);
}

#[test]
fn synchronous_rejections_never_fire_the_callback() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let engine = open(directory.path());
    let state = Capture::new();
    let verb = CString::new("recall").expect("the verb has no interior NUL");
    let arguments = CString::new(RECALL).expect("the arguments have no interior NUL");
    let invalid = CString::new(vec![0xff_u8, 0xfe]).expect("no interior NUL");

    let rejections: [(&str, HmStatus); 5] = [
        ("null engine", HmStatus::NullPointer),
        ("null verb", HmStatus::NullPointer),
        ("null arguments", HmStatus::NullPointer),
        ("null callback", HmStatus::NullPointer),
        ("non-utf8 verb", HmStatus::InvalidUtf8),
    ];
    for (case, expected) in rejections {
        let status = unsafe {
            match case {
                "null engine" => hm_engine_call(
                    ptr::null(),
                    verb.as_ptr(),
                    arguments.as_ptr(),
                    Some(capture),
                    state.user_data(),
                ),
                "null verb" => hm_engine_call(
                    engine,
                    ptr::null(),
                    arguments.as_ptr(),
                    Some(capture),
                    state.user_data(),
                ),
                "null arguments" => hm_engine_call(
                    engine,
                    verb.as_ptr(),
                    ptr::null(),
                    Some(capture),
                    state.user_data(),
                ),
                "null callback" => hm_engine_call(
                    engine,
                    verb.as_ptr(),
                    arguments.as_ptr(),
                    None,
                    state.user_data(),
                ),
                _ => hm_engine_call(
                    engine,
                    invalid.as_ptr(),
                    arguments.as_ptr(),
                    Some(capture),
                    state.user_data(),
                ),
            }
        };
        assert_eq!(status, expected, "{case}");
        assert_eq!(state.calls(), 0, "{case}");
    }

    let non_utf8_arguments = unsafe {
        hm_engine_call(
            engine,
            verb.as_ptr(),
            invalid.as_ptr(),
            Some(capture),
            state.user_data(),
        )
    };
    assert_eq!(non_utf8_arguments, HmStatus::InvalidUtf8);
    assert_eq!(state.calls(), 0);

    assert_eq!(unsafe { hm_engine_close(engine) }, HmStatus::Ok);
    let closed = unsafe {
        hm_engine_call(
            engine,
            verb.as_ptr(),
            arguments.as_ptr(),
            Some(capture),
            state.user_data(),
        )
    };
    assert_eq!(closed, HmStatus::HandleClosed);
    assert_eq!(state.calls(), 0);
    unsafe { hm_engine_free(engine) };
}

fn blocking_call(engine: *mut HmEngine, waiter: *mut HmWaiter, verb: &str, arguments: &str) {
    let verb = CString::new(verb).expect("the verb has no interior NUL");
    let arguments = CString::new(arguments).expect("the arguments have no interior NUL");
    let status = unsafe {
        hm_engine_call(
            engine,
            verb.as_ptr(),
            arguments.as_ptr(),
            Some(hm_waiter_callback),
            waiter.cast::<c_void>(),
        )
    };
    assert_eq!(status, HmStatus::Ok);
}

#[test]
fn waiter_blocks_and_transfers_ownership() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let engine = open(directory.path());

    let waiter = hm_waiter_new();
    assert!(!waiter.is_null());
    blocking_call(engine, waiter, "recall", RECALL);

    let mut kernel_code = 0_i32;
    let mut result: *mut c_char = ptr::null_mut();
    let status = unsafe { hm_waiter_wait(waiter, &raw mut kernel_code, &raw mut result) };
    assert_eq!(status, HmStatus::Ok);
    assert_eq!(kernel_code, -1);
    assert!(!result.is_null());

    let text = unsafe { CStr::from_ptr(result) }
        .to_str()
        .expect("the waiter delivered UTF-8")
        .to_owned();
    unsafe { hm_string_free(result) };
    let value: Value = serde_json::from_str(&text).expect("the waiter delivered a JSON envelope");
    assert_eq!(value["ok"], Value::Bool(true), "{value}");
    assert!(value["items"].is_array(), "{value}");

    unsafe { hm_waiter_free(waiter) };
    shut_down(engine);
}

#[test]
fn waiter_is_single_use() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let engine = open(directory.path());

    let waiter = hm_waiter_new();
    blocking_call(engine, waiter, "recall", RECALL);

    let mut kernel_code = 0_i32;
    let mut result: *mut c_char = ptr::null_mut();
    assert_eq!(
        unsafe { hm_waiter_wait(waiter, &raw mut kernel_code, &raw mut result) },
        HmStatus::Ok
    );
    unsafe { hm_string_free(result) };

    let mut second_code = 0_i32;
    let mut second: *mut c_char = ptr::null_mut();
    assert_eq!(
        unsafe { hm_waiter_wait(waiter, &raw mut second_code, &raw mut second) },
        HmStatus::InvalidArgument
    );
    assert!(second.is_null());
    assert_eq!(second_code, -1);

    unsafe { hm_waiter_free(waiter) };
    shut_down(engine);
}

#[test]
fn waiter_refuses_a_runtime_thread() {
    let waiter = hm_waiter_new();
    let runtime = tokio::runtime::Runtime::new().expect("runtime");
    let refused = runtime.block_on(async {
        let mut kernel_code = 0_i32;
        let mut result: *mut c_char = ptr::null_mut();
        let status = unsafe { hm_waiter_wait(waiter, &raw mut kernel_code, &raw mut result) };
        assert!(result.is_null());
        status
    });
    assert_eq!(refused, HmStatus::Runtime);
    drop(runtime);
    unsafe { hm_waiter_free(waiter) };
}
