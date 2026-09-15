#![forbid(unsafe_code)]
#![allow(clippy::missing_errors_doc)]

pub mod cache;
pub mod hash_feature;
pub mod local;
pub mod model;
pub mod quantize;
pub mod remote;
pub mod types;

pub use cache::CachedEmbedder;
pub use hash_feature::HashFeatureEmbedder;
pub use local::OnnxEmbedder;
pub use model::{Artifact, ArtifactFetcher, HttpFetcher, ModelKind, ModelSpec, ModelStore};
pub use quantize::{QuantizedEmbedding, quantize};
pub use remote::{
    HttpTransport, Provider, RecordedTransport, RemoteConfig, RemoteEmbedder, WireFixture,
    WireRequest, WireResponse, WireTransport,
};
pub use types::{
    Distance, EmbedError, Embedder, Embedding, InputRole, Normalization, SpaceIdentity,
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
