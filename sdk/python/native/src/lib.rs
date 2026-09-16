#![forbid(unsafe_code)]

use hm_compose::canonical::canonical_bytes;
use hm_compose::tokens::FallbackWeights;
use hm_core::{ActorId, ConversationId};
use hm_mcp::dispatcher::McpToolDispatcher;
use hm_serve::actor::{ActivateRequest, ActorConfig, ActorEngine};
use hm_serve::uds::ToolDispatcher;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use std::path::Path;
use std::sync::Arc;
use tokio::runtime::Runtime;

fn failure(error: impl std::fmt::Display) -> PyErr {
    PyRuntimeError::new_err(error.to_string())
}

fn hex<const N: usize>(value: &str) -> PyResult<[u8; N]> {
    if value.len() != N * 2 || !value.is_ascii() {
        return Err(PyValueError::new_err("invalid identity or key length"));
    }
    let mut result = [0; N];
    for (index, byte) in result.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| PyValueError::new_err("invalid hexadecimal identity or key"))?;
    }
    Ok(result)
}

#[pyclass]
struct NativeEngine {
    actor: Option<ActorEngine>,
    dispatcher: McpToolDispatcher,
    runtime: Arc<Runtime>,
}

#[pymethods]
impl NativeEngine {
    #[new]
    #[pyo3(signature = (path, actor, user_hex, kek_hex, projection_map_bytes=268_435_456, enable_providers=false))]
    fn new(
        py: Python<'_>,
        path: String,
        actor: u16,
        user_hex: String,
        kek_hex: String,
        projection_map_bytes: usize,
        enable_providers: bool,
    ) -> PyResult<Self> {
        if actor == 0 {
            return Err(PyValueError::new_err("actor must be nonzero"));
        }
        let config = ActorConfig {
            actor_directory: Path::new(&path).join(actor.to_string()),
            actor: ActorId::new(actor),
            user: hex(&user_hex)?,
            kek: hex(&kek_hex)?,
            projection_map_bytes,
        };
        py.allow_threads(move || {
            let runtime = Arc::new(Runtime::new().map_err(failure)?);
            let dispatcher = if enable_providers {
                McpToolDispatcher::from_env().map_err(failure)?
            } else {
                McpToolDispatcher::default()
            };
            let actor = runtime
                .block_on(ActorEngine::open(config))
                .map_err(failure)?;
            Ok(Self {
                actor: Some(actor),
                dispatcher,
                runtime,
            })
        })
    }

    fn call(&self, py: Python<'_>, verb: String, arguments_json: String) -> PyResult<String> {
        let actor = self
            .actor
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("engine is closed"))?
            .clone();
        py.allow_threads(|| {
            let bytes = self
                .runtime
                .block_on(
                    self.dispatcher
                        .dispatch(actor, verb, arguments_json.into_bytes()),
                )
                .map_err(failure)?;
            String::from_utf8(bytes).map_err(failure)
        })
    }

    fn activate(
        &self,
        py: Python<'_>,
        conversation_hex: String,
        query: String,
        budget_tokens: usize,
    ) -> PyResult<Vec<u8>> {
        let actor = self
            .actor
            .as_ref()
            .ok_or_else(|| PyRuntimeError::new_err("engine is closed"))?
            .clone();
        let conversation = ConversationId::new(hex(&conversation_hex)?);
        py.allow_threads(|| {
            let bundle = self
                .runtime
                .block_on(actor.activate(ActivateRequest {
                    conversation,
                    query,
                    turn_text: String::new(),
                    budget_tokens,
                    token_weights: FallbackWeights::default(),
                }))
                .map_err(failure)?;
            canonical_bytes(&bundle).map_err(failure)
        })
    }

    fn close(&mut self, py: Python<'_>) -> PyResult<()> {
        if let Some(actor) = self.actor.take() {
            py.allow_threads(|| self.runtime.block_on(actor.shutdown()).map_err(failure))?;
        }
        Ok(())
    }
}

#[pymodule]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<NativeEngine>()?;
    Ok(())
}
