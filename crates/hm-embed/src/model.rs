use crate::types::EmbedError;
use sha2::{Digest as _, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelKind {
    NomicEmbedTextV15,
    BgeSmallEnV15,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Artifact {
    pub name: &'static str,
    pub url: &'static str,
    pub sha256: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelSpec {
    pub kind: ModelKind,
    pub encoder_id: &'static str,
    pub revision: &'static str,
    pub dimensions: usize,
    pub maximum_tokens: usize,
    pub model: Artifact,
    pub tokenizer: Artifact,
}

impl ModelKind {
    #[must_use]
    pub const fn spec(self) -> ModelSpec {
        match self {
            Self::NomicEmbedTextV15 => ModelSpec {
                kind: self,
                encoder_id: "nomic-ai/nomic-embed-text-v1.5",
                revision: "a15734e81021ea6c92b09050d2c7085001db8f36",
                dimensions: 768,
                maximum_tokens: 512,
                model: Artifact {
                    name: "model_quantized.onnx",
                    url: "https://huggingface.co/nomic-ai/nomic-embed-text-v1.5/resolve/a15734e81021ea6c92b09050d2c7085001db8f36/onnx/model_quantized.onnx",
                    sha256: "b4342336debaea79de872370664b0aaeb67dea4605513d00ee236ea871a81f27",
                },
                tokenizer: Artifact {
                    name: "tokenizer.json",
                    url: "https://huggingface.co/nomic-ai/nomic-embed-text-v1.5/resolve/a15734e81021ea6c92b09050d2c7085001db8f36/tokenizer.json",
                    sha256: "d241a60d5e8f04cc1b2b3e9ef7a4921b27bf526d9f6050ab90f9267a1f9e5c66",
                },
            },
            Self::BgeSmallEnV15 => ModelSpec {
                kind: self,
                encoder_id: "BAAI/bge-small-en-v1.5",
                revision: "c5ac6c397e27c80e0229ec647987f2e553fc0ba9",
                dimensions: 384,
                maximum_tokens: 512,
                model: Artifact {
                    name: "model_quantized.onnx",
                    url: "https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/c5ac6c397e27c80e0229ec647987f2e553fc0ba9/onnx/model_quantized.onnx",
                    sha256: "6c9c6101a956d62dfb5e7190c538226c0c5bb9cb27b651234b6df063ee7dbfe4",
                },
                tokenizer: Artifact {
                    name: "tokenizer.json",
                    url: "https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/c5ac6c397e27c80e0229ec647987f2e553fc0ba9/tokenizer.json",
                    sha256: "d241a60d5e8f04cc1b2b3e9ef7a4921b27bf526d9f6050ab90f9267a1f9e5c66",
                },
            },
        }
    }
}

pub trait ArtifactFetcher: Send + Sync {
    fn fetch(&self, url: &str) -> Result<Vec<u8>, EmbedError>;
}

#[derive(Clone, Debug, Default)]
pub struct HttpFetcher;

impl ArtifactFetcher for HttpFetcher {
    fn fetch(&self, url: &str) -> Result<Vec<u8>, EmbedError> {
        let response = reqwest::blocking::get(url)
            .map_err(|error| EmbedError::Network(error.to_string()))?
            .error_for_status()
            .map_err(|error| EmbedError::Network(error.to_string()))?;
        response
            .bytes()
            .map(|bytes| bytes.to_vec())
            .map_err(|error| EmbedError::Network(error.to_string()))
    }
}

#[derive(Clone, Debug)]
pub struct ModelStore {
    root: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelFiles {
    pub model: PathBuf,
    pub tokenizer: PathBuf,
}

impl ModelStore {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn ensure(
        &self,
        spec: ModelSpec,
        fetcher: &impl ArtifactFetcher,
    ) -> Result<ModelFiles, EmbedError> {
        let directory = self
            .root
            .join(spec.encoder_id.replace('/', "--"))
            .join(spec.revision);
        fs::create_dir_all(&directory)?;
        Ok(ModelFiles {
            model: ensure_artifact(&directory, spec.model, fetcher)?,
            tokenizer: ensure_artifact(&directory, spec.tokenizer, fetcher)?,
        })
    }
}

fn ensure_artifact(
    directory: &Path,
    artifact: Artifact,
    fetcher: &impl ArtifactFetcher,
) -> Result<PathBuf, EmbedError> {
    let path = directory.join(artifact.name);
    if path.exists() {
        verify_digest(&path, artifact)?;
        return Ok(path);
    }
    let bytes = fetcher.fetch(artifact.url)?;
    if sha256(&bytes) != artifact.sha256 {
        return Err(EmbedError::ArtifactDigest {
            name: artifact.name.to_owned(),
        });
    }
    let partial = directory.join(format!("{}.part", artifact.name));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&partial)?;
    file.write_all(&bytes)?;
    file.sync_data()?;
    fs::rename(partial, &path)?;
    FileSync::sync(directory)?;
    Ok(path)
}

fn verify_digest(path: &Path, artifact: Artifact) -> Result<(), EmbedError> {
    let bytes = fs::read(path)?;
    if sha256(&bytes) == artifact.sha256 {
        Ok(())
    } else {
        Err(EmbedError::ArtifactDigest {
            name: artifact.name.to_owned(),
        })
    }
}

fn sha256(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            let _ = write!(output, "{byte:02x}");
            output
        })
}

struct FileSync;

impl FileSync {
    fn sync(directory: &Path) -> Result<(), EmbedError> {
        std::fs::File::open(directory)?.sync_all()?;
        Ok(())
    }
}
