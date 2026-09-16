use crate::{EmbeddingRuntime, Envelope, McpServer, ReconstructionRuntime};
use hm_core::{Error, ErrorCode};
use hm_serve::actor::ActorEngine;
use hm_serve::uds::ToolDispatcher;

#[derive(Clone, Default)]
pub struct McpToolDispatcher {
    pub embedding_runtime: Option<EmbeddingRuntime>,
    pub reconstruction_runtime: Option<ReconstructionRuntime>,
}

impl McpToolDispatcher {
    pub fn from_env() -> Result<Self, Error> {
        Ok(Self {
            embedding_runtime: EmbeddingRuntime::from_env()
                .map_err(|_| Error::new(ErrorCode::InvalidArgument))?,
            reconstruction_runtime: ReconstructionRuntime::from_env()?,
        })
    }

    pub fn server(&self, actor: ActorEngine) -> McpServer {
        let mut server = McpServer::new(actor);
        server.embedding_runtime = self.embedding_runtime.clone();
        server.reconstruction_runtime = self.reconstruction_runtime.clone();
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
