use crate::status::{HmStatus, set_last_error};
use hm_context::mobile_journal::{JournalBinding, MobileJournal};
use hm_fabric::{
    enrollment::{PublicTlsSession, client_config},
    mobile_delivery::EncryptedJournalSession,
};
use rustls::{
    RootCertStore,
    pki_types::{CertificateDer, PrivateKeyDer, ServerName},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    ffi::{CStr, CString, c_char},
    panic::{AssertUnwindSafe, catch_unwind},
    ptr,
    sync::Mutex,
};
use tokio::{net::TcpStream, runtime::Runtime};

pub struct HmMobileJournal {
    state: Mutex<State>,
}
struct State {
    journal: Option<MobileJournal>,
    binding: JournalBinding,
    session: Option<EncryptedJournalSession>,
    runtime: Runtime,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Open {
    path: String,
    binding: JournalBinding,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Connection {
    address: String,
    server_name: String,
    ca_der: Vec<u8>,
    certificate_der: Vec<u8>,
    private_key_der: Vec<u8>,
    ticket: Option<String>,
    grants: std::collections::BTreeSet<String>,
    server_certificate_sha256: String,
}
fn status(s: &State) -> Result<Value, String> {
    let j = s.journal.as_ref().ok_or("journal closed")?;
    Ok(
        json!({"cursor":j.cursor_receipt(), "records":j.records(), "session_id":s.session.as_ref().map(|s|s.session_id())}),
    )
}
fn execute(s: &mut State, operation: &str, args: Value) -> Result<Value, String> {
    if s.journal.is_none() {
        return Err("journal closed".into());
    }
    match operation {
        "status" => status(s),
        "enqueue" => {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Enqueue {
                operation_id: String,
                operation_kind: String,
                payload_digest: String,
            }
            let a: Enqueue = serde_json::from_value(args).map_err(|_| "invalid enqueue")?;
            let record = s
                .journal
                .as_mut()
                .unwrap()
                .enqueue_mutation(&a.operation_id, &a.operation_kind, &a.payload_digest)
                .map_err(|e| e.to_string())?;
            serde_json::to_value(record).map_err(|_| "record encoding".into())
        }
        "connect" => {
            let a: Connection =
                serde_json::from_value(args).map_err(|_| "invalid TLS configuration")?;
            if a.ca_der.len() > 65536
                || a.certificate_der.len() > 65536
                || a.private_key_der.len() > 65536
            {
                return Err("TLS material size".into());
            }
            let mut roots = RootCertStore::empty();
            roots
                .add(CertificateDer::from(a.ca_der))
                .map_err(|_| "invalid TLS root")?;
            let key = PrivateKeyDer::try_from(a.private_key_der)
                .map_err(|_| "invalid TLS private key")?;
            let config = client_config(roots, vec![CertificateDer::from(a.certificate_der)], key)
                .map_err(|_| "invalid TLS credentials")?;
            let name =
                ServerName::try_from(a.server_name).map_err(|_| "invalid TLS server name")?;
            let scope = s.binding.scope.clone();
            // Close the previous stream before reauthentication. A refused
            // connection never leaves an older session silently usable.
            s.session = None;
            let session = s.runtime.block_on(async {
                let stream = tokio::time::timeout(
                    std::time::Duration::from_secs(10),
                    TcpStream::connect(a.address),
                )
                .await
                .map_err(|_| "TLS connection timeout")?
                .map_err(|_| "TLS connection failed")?;
                PublicTlsSession::connect(stream, config, name, scope, a.ticket, a.grants)
                    .await
                    .map_err(|_| "TLS enrollment refused")
            })?;
            s.session = Some(
                EncryptedJournalSession::new(
                    session,
                    s.binding.clone(),
                    &a.server_certificate_sha256,
                )
                .map_err(|_| "TLS server pin or scope refused")?,
            );
            status(s)
        }
        "disconnect" => {
            s.session = None;
            status(s)
        }
        "dispatch" | "reconcile" => {
            let session = s.session.as_mut().ok_or("authenticated session required")?;
            let journal = s.journal.as_mut().unwrap();
            let result = if operation == "dispatch" {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Payload {
                    payload: Vec<u8>,
                }
                let p: Payload = serde_json::from_value(args).map_err(|_| "invalid payload")?;
                s.runtime.block_on(async {
                    tokio::time::timeout(
                        std::time::Duration::from_secs(20),
                        session.dispatch_pending(journal, &p.payload),
                    )
                    .await
                })
            } else {
                s.runtime.block_on(async {
                    tokio::time::timeout(
                        std::time::Duration::from_secs(20),
                        session.reconcile_pending(journal),
                    )
                    .await
                })
            }
            .map_err(|_| "encrypted journal exchange timeout; outcome remains unknown")?;
            let record = result.map_err(|e| e.to_string())?;
            serde_json::to_value(record).map_err(|_| "record encoding".into())
        }
        "close" => {
            s.session = None;
            s.journal = None;
            Ok(json!({}))
        }
        _ => Err("unsupported mobile journal operation".into()),
    }
}
unsafe fn text<'a>(p: *const c_char) -> Result<&'a str, HmStatus> {
    if p.is_null() {
        set_last_error("null journal argument");
        return Err(HmStatus::NullPointer);
    }
    unsafe { CStr::from_ptr(p) }.to_str().map_err(|_| {
        set_last_error("invalid journal UTF-8");
        HmStatus::InvalidUtf8
    })
}
fn boundary(action: impl FnOnce() -> Result<(), String>) -> HmStatus {
    match catch_unwind(AssertUnwindSafe(action)) {
        Ok(Ok(())) => HmStatus::Ok,
        Ok(Err(e)) => {
            set_last_error(e);
            HmStatus::InvalidArgument
        }
        Err(_) => {
            set_last_error("native journal panic");
            HmStatus::Panic
        }
    }
}
/// Opens a native metadata-only journal. All pointers must be valid C strings
/// or writable output locations; the returned handle is freed exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hm_mobile_journal_open(
    configuration: *const c_char,
    out: *mut *mut HmMobileJournal,
) -> HmStatus {
    if out.is_null() {
        return HmStatus::NullPointer;
    }
    unsafe {
        *out = ptr::null_mut();
    }
    let input = match unsafe { text(configuration) } {
        Ok(v) => v,
        Err(s) => return s,
    };
    boundary(|| {
        let request: Open =
            serde_json::from_str(input).map_err(|_| "invalid journal configuration")?;
        let journal = MobileJournal::open(request.path, request.binding.clone())
            .map_err(|e| e.to_string())?;
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .map_err(|_| "journal runtime failed")?;
        let handle = Box::new(HmMobileJournal {
            state: Mutex::new(State {
                journal: Some(journal),
                binding: request.binding,
                session: None,
                runtime,
            }),
        });
        unsafe { *out = Box::into_raw(handle) };
        Ok(())
    })
}
/// Executes a native journal operation; output is an owned JSON string freed
/// with hm_string_free. No receipt settlement operation is accepted.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hm_mobile_journal_call(
    handle: *mut HmMobileJournal,
    operation: *const c_char,
    args: *const c_char,
    out: *mut *mut c_char,
) -> HmStatus {
    if handle.is_null() || out.is_null() {
        return HmStatus::NullPointer;
    }
    unsafe {
        *out = ptr::null_mut();
    }
    let op = match unsafe { text(operation) } {
        Ok(v) => v,
        Err(s) => return s,
    };
    let input = match unsafe { text(args) } {
        Ok(v) => v,
        Err(s) => return s,
    };
    boundary(|| {
        let args = serde_json::from_str(input).map_err(|_| "invalid journal arguments")?;
        let mut state = unsafe { &*handle }
            .state
            .lock()
            .map_err(|_| "journal handle poisoned")?;
        let result = execute(&mut state, op, args)?;
        let result =
            CString::new(serde_json::to_string(&result).map_err(|_| "journal result encoding")?)
                .map_err(|_| "journal result contains NUL")?;
        unsafe {
            *out = result.into_raw();
        }
        Ok(())
    })
}
/// Frees a handle exactly once after all calls on that handle have returned.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn hm_mobile_journal_free(handle: *mut HmMobileJournal) {
    if !handle.is_null() {
        drop(unsafe { Box::from_raw(handle) });
    }
}
