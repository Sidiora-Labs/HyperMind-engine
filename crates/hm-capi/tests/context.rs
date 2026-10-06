use hm_context::{Authority, MessagePart, MessageRole, Scope, SourceMessage};
use hm_serve::{context_config::TrustedContextConfig, embedded::{EmbeddedConfig, HyperMind}};
use hypermind::*;
use serde_json::{Value, json};
use std::ffi::{CStr, CString, c_void};
use std::ptr;

fn scope() -> Scope { Scope { owner_id: "owner".into(), project_id: "project".into(), workspace_id: None } }
fn source() -> SourceMessage {
    let mut source = SourceMessage { id: "original".into(), ordinal: 0, role: MessageRole::User, parts: vec![MessagePart::Text { text: "parsed source".into() }], occurred_at_ns: None, recorded_at_ns: 1, authority: Authority::UserAsserted, source_digest: String::new() };
    source.source_digest = source.computed_digest().unwrap(); source
}
fn config(path: &std::path::Path, context: Option<Value>) -> CString {
    let mut value = json!({"path":path,"actor":7,"user_hex":"11".repeat(16),"kek_hex":"22".repeat(32),"projection_map_bytes":16_777_216});
    if let Some(context) = context { value["context_scope"] = context; }
    CString::new(value.to_string()).unwrap()
}
fn trusted() -> Value { json!({"version":1,"actor":7,"scope":scope()}) }
fn open(path: &std::path::Path, context: Option<Value>) -> *mut HmEngine {
    let mut engine = ptr::null_mut();
    assert_eq!(unsafe { hm_engine_open(config(path, context).as_ptr(), &mut engine) }, HmStatus::Ok);
    engine
}
fn close(engine: *mut HmEngine) {
    assert_eq!(unsafe { hm_engine_close(engine) }, HmStatus::Ok);
    unsafe { hm_engine_free(engine) };
}
fn call(engine: *mut HmEngine, verb: &str, arguments: Value) -> Value {
    let verb = CString::new(verb).unwrap();
    let args = CString::new(arguments.to_string()).unwrap();
    let waiter = hm_waiter_new();
    assert_eq!(unsafe { hm_engine_call(engine, verb.as_ptr(), args.as_ptr(), Some(hm_waiter_callback), waiter.cast::<c_void>()) }, HmStatus::Ok);
    let mut code = 0;
    let mut output = ptr::null_mut();
    assert_eq!(unsafe { hm_waiter_wait(waiter, &mut code, &mut output) }, HmStatus::Ok);
    assert_eq!(code, -1);
    let value = serde_json::from_slice(unsafe { CStr::from_ptr(output) }.to_bytes()).unwrap();
    unsafe { hm_string_free(output); hm_waiter_free(waiter); }
    value
}
fn activation() -> Value {
    json!({"version":1,"scope":scope(),"session_id":"session","budget":{"context_tokens":4096,"reserved_output_tokens":256,"required_tokens":0},"model_id":"gpt-4o","defer_reductions":true})
}
#[test]
fn actual_callback_context_restart_and_exact_source_recovery() {
    let directory = tempfile::tempdir().unwrap();
    let bytes = vec![0, 255, 13, 10, 65];
    let engine = open(directory.path(), Some(trusted()));
    let result = call(engine, "remember", json!({"conversation":"conversation","context":{"operation":"source","request":{"version":1,"scope":scope(),"session_id":"session","conversation":"conversation","message":source(),"original_bytes":bytes}}}));
    assert_eq!(result["ok"], true, "{result}");
    let result = call(engine, "activate", json!({"conversation":"conversation","query":"","budget_tokens":4096,"context":activation()}));
    assert_eq!(result["ok"], true, "{result}");
    assert!(result["items"][0]["report"]["digest"].is_string());
    let job = call(engine, "remember", json!({"conversation":"conversation","context":{"operation":"job","request":{"version":1,"scope":scope(),"request_id":"inspect","action":{"action":"inspect"}}}}));
    assert_eq!(job["ok"], true, "{job}");
    close(engine);
    let engine = open(directory.path(), Some(trusted()));
    let result = call(engine, "inspect", json!({"uri":"hm://7/context/session/source/original"}));
    assert_eq!(result["ok"], true, "{result}");
    assert_eq!(result["items"][0]["original_bytes"], json!(bytes));
    let mut foreign = scope(); foreign.owner_id = "other".into();
    let result = call(engine, "remember", json!({"conversation":"conversation","context":{"operation":"source","request":{"version":1,"scope":foreign,"session_id":"session","conversation":"conversation","message":source(),"original_bytes":bytes}}}));
    assert_eq!(result["ok"], false, "{result}");
    close(engine);
}
#[test]
fn constructor_binding_refuses_mismatch_and_requests_cannot_supply_trust() {
    let directory = tempfile::tempdir().unwrap();
    let mut context = trusted(); context["actor"] = json!(8);
    let mut engine = ptr::null_mut();
    assert_ne!(unsafe { hm_engine_open(config(directory.path(), Some(context)).as_ptr(), &mut engine) }, HmStatus::Ok);
    assert!(engine.is_null());
    let engine = open(directory.path(), None);
    let result = call(engine, "activate", json!({"conversation":"conversation","query":"","budget_tokens":4096,"context":activation()}));
    assert_eq!(result["ok"], false, "{result}");
    close(engine);
}
#[test]
fn rust_embedded_uses_same_history_and_trusted_context() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        let config = EmbeddedConfig { actor: hm_core::ActorId::new(7), user: [17;16], kek: [34;32], projection_map_bytes: 16_777_216 };
        let engine = HyperMind::open_with_context(directory.path(), config, TrustedContextConfig { version:1, actor:7, scope:scope() }).await.unwrap();
        let session = engine.actor().context_session("session", "conversation").unwrap();
        let bytes = vec![255,0,10,13];
        let receipt = session.source(source(), bytes.clone()).await.unwrap();
        assert_eq!(session.recover(receipt.source_span.as_ref().unwrap()).await.unwrap(), bytes);
        let child = session.fork("child", "child-conversation").await.unwrap();
        assert_eq!(child.recover(receipt.source_span.as_ref().unwrap()).await.unwrap(), bytes);
        session.activate(serde_json::from_value(activation()).unwrap()).await.unwrap();
        session.job("inspect", hm_serve::context_jobs::ContextJobAction::Inspect).await.unwrap();
        engine.actor().shutdown().await.unwrap();
    });
}
