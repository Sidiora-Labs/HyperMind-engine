use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Distance {
    Cosine,
    Dot,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Normalization {
    None,
    L2,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum InputRole {
    Query,
    Document,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SpaceIdentity {
    pub encoder_id: String,
    pub revision: String,
    pub dimensions: usize,
    pub distance: Distance,
    pub normalization: Normalization,
    pub input_role: InputRole,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Embedding {
    pub space: SpaceIdentity,
    pub values: Vec<f32>,
}

pub trait Embedder: Send + Sync {
    fn identity(&self, role: InputRole) -> SpaceIdentity;

    fn embed_query(&self, query: &str) -> Result<Embedding, EmbedError>;

    fn embed_documents(&self, documents: &[&str]) -> Result<Vec<Embedding>, EmbedError>;

    fn report_label(&self) -> String {
        self.identity(InputRole::Document).encoder_id
    }
}

#[derive(Debug)]
pub enum EmbedError {
    InvalidArgument(&'static str),
    Dimension { expected: usize, actual: usize },
    ArtifactDigest { name: String },
    Io(std::io::Error),
    Network(String),
    Runtime(String),
    Tokenizer(String),
    Wire(String),
}

impl fmt::Display for EmbedError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArgument(message) => formatter.write_str(message),
            Self::Dimension { expected, actual } => {
                write!(
                    formatter,
                    "embedding dimension {actual}, expected {expected}"
                )
            }
            Self::ArtifactDigest { name } => write!(formatter, "artifact digest mismatch: {name}"),
            Self::Io(error) => error.fmt(formatter),
            Self::Network(error)
            | Self::Runtime(error)
            | Self::Tokenizer(error)
            | Self::Wire(error) => formatter.write_str(error),
        }
    }
}

impl std::error::Error for EmbedError {}

impl From<std::io::Error> for EmbedError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

pub(crate) fn checked_embeddings(
    identity: &SpaceIdentity,
    vectors: Vec<Vec<f32>>,
) -> Result<Vec<Embedding>, EmbedError> {
    vectors
        .into_iter()
        .map(|values| {
            if values.len() != identity.dimensions {
                return Err(EmbedError::Dimension {
                    expected: identity.dimensions,
                    actual: values.len(),
                });
            }
            if values.iter().any(|value| !value.is_finite()) {
                return Err(EmbedError::InvalidArgument(
                    "embedding contains a non-finite value",
                ));
            }
            Ok(Embedding {
                space: identity.clone(),
                values,
            })
        })
        .collect()
}
