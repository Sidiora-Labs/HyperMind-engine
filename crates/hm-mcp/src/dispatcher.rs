use crate::{ConsolidationRuntime, EmbeddingRuntime, Envelope, McpServer, ReconstructionRuntime};
use hm_core::{Error, ErrorCode};
use hm_serve::actor::ActorEngine;
use hm_serve::uds::ToolDispatcher;

#[derive(Clone, Default)]
pub struct McpToolDispatcher {
    pub embedding_runtime: Option<EmbeddingRuntime>,
    pub reconstruction_runtime: Option<ReconstructionRuntime>,
    pub consolidation_runtime: Option<ConsolidationRuntime>,
}

impl McpToolDispatcher {
    pub fn from_env() -> Result<Self, Error> {
        Ok(Self {
            embedding_runtime: EmbeddingRuntime::from_env()
                .map_err(|_| Error::new(ErrorCode::InvalidArgument))?,
            reconstruction_runtime: ReconstructionRuntime::from_env()?,
            consolidation_runtime: ConsolidationRuntime::from_env()?,
        })
    }

    pub fn server(&self, actor: ActorEngine) -> McpServer {
        let mut server = McpServer::new(actor);
        server.embedding_runtime = self.embedding_runtime.clone();
        server.reconstruction_runtime = self.reconstruction_runtime.clone();
        server.consolidation_runtime = self.consolidation_runtime.clone();
        server
    }
}

impl ToolDispatcher for McpToolDispatcher {
    fn dispatch(
        &self,
        actor: ActorEngine,
        verb: String,
        arguments_json: Vec<u8>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>, Error>> + Send + '_>>
    {
        Box::pin(async move {
            let server = self.server(actor);
            let parse_error = |_| Error::new(ErrorCode::InvalidArgument);
            let result = match verb.as_str() {
                "remember" => match serde_json::from_slice(&arguments_json) {
                    Ok(input) => server.remember_envelope(input).await,
                    Err(error) => Envelope::error(parse_error(error), true),
                },
                "activate" => match serde_json::from_slice(&arguments_json) {
                    Ok(input) => server.activate_envelope(input).await,
                    Err(error) => Envelope::error(parse_error(error), false),
                },
                "believe" => match serde_json::from_slice(&arguments_json) {
                    Ok(input) => server.believe_envelope(input).await,
                    Err(error) => Envelope::error(parse_error(error), true),
                },
                "retract" => match serde_json::from_slice(&arguments_json) {
                    Ok(input) => server.retract_envelope(input).await,
                    Err(error) => Envelope::error(parse_error(error), true),
                },
                "dispute" => match serde_json::from_slice(&arguments_json) {
                    Ok(input) => server.dispute_envelope(input).await,
                    Err(error) => Envelope::error(parse_error(error), true),
                },
                "bind" => match serde_json::from_slice(&arguments_json) {
                    Ok(input) => server.bind_envelope(input).await,
                    Err(error) => Envelope::error(parse_error(error), true),
                },
                "attest" => match serde_json::from_slice(&arguments_json) {
                    Ok(input) => server.attest_envelope(input).await,
                    Err(error) => Envelope::error(parse_error(error), true),
                },
                "consolidate" => match serde_json::from_slice(&arguments_json) {
                    Ok(input) => server.consolidate_envelope(input).await,
                    Err(error) => Envelope::error(parse_error(error), true),
                },
                "forget" => match serde_json::from_slice(&arguments_json) {
                    Ok(input) => server.forget_envelope(input).await,
                    Err(error) => Envelope::error(parse_error(error), true),
                },
                "intend" => match serde_json::from_slice(&arguments_json) {
                    Ok(input) => server.intend_envelope(input).await,
                    Err(error) => Envelope::error(parse_error(error), true),
                },
                "predict" => match serde_json::from_slice(&arguments_json) {
                    Ok(input) => server.predict_envelope(input).await,
                    Err(error) => Envelope::error(parse_error(error), true),
                },
                "outcome" => match serde_json::from_slice(&arguments_json) {
                    Ok(input) => server.outcome_envelope(input).await,
                    Err(error) => Envelope::error(parse_error(error), true),
                },
                "inspect" => match serde_json::from_slice(&arguments_json) {
                    Ok(input) => server.inspect_envelope(input).await,
                    Err(error) => Envelope::error(parse_error(error), false),
                },
                "recall" => match serde_json::from_slice(&arguments_json) {
                    Ok(input) => server.recall_envelope(input).await,
                    Err(error) => Envelope::error(parse_error(error), false),
                },
                _ => return Err(Error::new(ErrorCode::CapabilityDenied)),
            };
            serde_json::to_vec(&result).map_err(|_| Error::new(ErrorCode::SchemaInvalid))
        })
    }
}
