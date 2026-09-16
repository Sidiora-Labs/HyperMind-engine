use anyhow::{Result, ensure};
use clap::{Subcommand, ValueEnum};
use hm_embed::model::{ArtifactFetcher, ModelKind, ModelStore};
use hm_embed::types::EmbedError;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Choice {
    BgeSmall,
    Nomic,
}

impl Choice {
    fn kind(self) -> ModelKind {
        match self {
            Self::BgeSmall => ModelKind::BgeSmallEnV15,
            Self::Nomic => ModelKind::NomicEmbedTextV15,
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Ensure {
        #[arg(long)]
        directory: PathBuf,
        #[arg(long, value_enum)]
        model: Choice,
    },
    List {
        #[arg(long)]
        directory: PathBuf,
    },
}

pub async fn execute(command: Command) -> Result<Value> {
    match command {
        Command::List { directory } => status(&directory),
        Command::Ensure { directory, model } => ensure_model(directory, model).await,
    }
}

pub async fn ensure_model(directory: PathBuf, model: Choice) -> Result<Value> {
    tokio::task::spawn_blocking(move || {
        let spec = model.kind().spec();
        let fetcher = BoundedFetcher(reqwest::blocking::Client::builder().connect_timeout(Duration::from_secs(20)).timeout(Duration::from_secs(180)).build()?);
        let files = ModelStore::new(&directory).ensure(spec,&fetcher)?;
        Ok(json!({"ok":true,"model":spec.encoder_id,"revision":spec.revision,"model_path":files.model,"tokenizer_path":files.tokenizer,"digest_verified":true}))
    }).await?
}

struct BoundedFetcher(reqwest::blocking::Client);
impl ArtifactFetcher for BoundedFetcher {
    fn fetch(&self, url: &str) -> std::result::Result<Vec<u8>, EmbedError> {
        let response = self
            .0
            .get(url)
            .send()
            .and_then(reqwest::blocking::Response::error_for_status)
            .map_err(|e| EmbedError::Network(e.to_string()))?;
        let mut bytes = Vec::new();
        response
            .take(512 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > 512 * 1024 * 1024 {
            return Err(EmbedError::InvalidArgument(
                "model artifact exceeds 512 MiB",
            ));
        }
        Ok(bytes)
    }
}

#[allow(clippy::unnecessary_wraps)]
pub fn status(directory: &Path) -> Result<Value> {
    let mut models = Vec::new();
    for kind in [ModelKind::BgeSmallEnV15, ModelKind::NomicEmbedTextV15] {
        let spec = kind.spec();
        let root = directory
            .join(spec.encoder_id.replace('/', "--"))
            .join(spec.revision);
        let artifacts = [spec.model, spec.tokenizer]
            .iter()
            .map(|artifact| {
                let path = root.join(artifact.name);
                let status = if !path.exists() {
                    "missing"
                } else if digest(&path).is_ok_and(|actual| actual == artifact.sha256) {
                    "verified"
                } else {
                    "invalid"
                };
                json!({"name":artifact.name,"path":path,"sha256":artifact.sha256,"status":status})
            })
            .collect::<Vec<_>>();
        models.push(json!({"id":spec.encoder_id,"revision":spec.revision,"present":artifacts.iter().all(|a| a["status"] == "verified"),"artifacts":artifacts}));
    }
    Ok(
        json!({"directory":directory,"models":models,"onnx_runtime_configured":std::env::var_os("ORT_DYLIB_PATH").is_some(),"local_inference_tested":false}),
    )
}

#[allow(clippy::large_stack_arrays)]
fn digest(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    ensure!(
        file.metadata()?.is_file(),
        "model artifact is not a regular file"
    );
    let mut hasher = Sha256::new();
    let mut bytes = [0; 65536];
    loop {
        let count = file.read(&mut bytes)?;
        if count == 0 {
            break;
        }
        hasher.update(&bytes[..count]);
    }
    Ok(crate::hex(&hasher.finalize()))
}
