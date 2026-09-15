use crate::hash_feature::l2_normalize;
use crate::model::{ArtifactFetcher, ModelKind, ModelStore};
use crate::types::{
    Distance, EmbedError, Embedder, Embedding, InputRole, Normalization, SpaceIdentity,
    checked_embeddings,
};
use ort::session::Session;
use ort::value::Tensor;
use std::path::Path;
use std::sync::Mutex;
use tokenizers::Tokenizer;

pub struct OnnxEmbedder {
    kind: ModelKind,
    tokenizer: Tokenizer,
    session: Mutex<Session>,
}

impl OnnxEmbedder {
    pub fn download(
        kind: ModelKind,
        store: &ModelStore,
        fetcher: &impl ArtifactFetcher,
    ) -> Result<Self, EmbedError> {
        let files = store.ensure(kind.spec(), fetcher)?;
        Self::open(kind, &files.model, &files.tokenizer)
    }

    pub fn open(
        kind: ModelKind,
        model_path: &Path,
        tokenizer_path: &Path,
    ) -> Result<Self, EmbedError> {
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|error| EmbedError::Tokenizer(error.to_string()))?;
        let session = Session::builder()
            .and_then(|mut builder| builder.commit_from_file(model_path))
            .map_err(|error| EmbedError::Runtime(error.to_string()))?;
        Ok(Self {
            kind,
            tokenizer,
            session: Mutex::new(session),
        })
    }

    fn embed(&self, role: InputRole, texts: &[&str]) -> Result<Vec<Embedding>, EmbedError> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let spec = self.kind.spec();
        let prefixed: Vec<_> = texts
            .iter()
            .map(|text| prefix(self.kind, role, text))
            .collect();
        let encodings = prefixed
            .iter()
            .map(|text| {
                self.tokenizer
                    .encode(text.as_str(), true)
                    .map_err(|error| EmbedError::Tokenizer(error.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let sequence_length = encodings
            .iter()
            .map(|encoding| encoding.get_ids().len().min(spec.maximum_tokens))
            .max()
            .unwrap_or(0);
        if sequence_length == 0 {
            return Err(EmbedError::InvalidArgument("tokenizer returned no tokens"));
        }
        let batch = encodings.len();
        let capacity = batch * sequence_length;
        let mut input_ids = vec![0_i64; capacity];
        let mut attention_mask = vec![0_i64; capacity];
        let mut token_type_ids = vec![0_i64; capacity];
        for (row, encoding) in encodings.iter().enumerate() {
            let length = encoding.get_ids().len().min(sequence_length);
            for column in 0..length {
                let offset = row * sequence_length + column;
                input_ids[offset] = i64::from(encoding.get_ids()[column]);
                attention_mask[offset] = 1;
                token_type_ids[offset] = i64::from(encoding.get_type_ids()[column]);
            }
        }
        let shape = [batch, sequence_length];
        let mut session = self
            .session
            .lock()
            .map_err(|_| EmbedError::Runtime("ONNX session lock poisoned".to_owned()))?;
        let outputs = session
            .run(ort::inputs! {
                "input_ids" => Tensor::from_array((shape, input_ids.into_boxed_slice()))
                    .map_err(|error| EmbedError::Runtime(error.to_string()))?,
                "attention_mask" => Tensor::from_array((shape, attention_mask.clone().into_boxed_slice()))
                    .map_err(|error| EmbedError::Runtime(error.to_string()))?,
                "token_type_ids" => Tensor::from_array((shape, token_type_ids.into_boxed_slice()))
                    .map_err(|error| EmbedError::Runtime(error.to_string()))?,
            })
            .map_err(|error| EmbedError::Runtime(error.to_string()))?;
        let (output_shape, values) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|error| EmbedError::Runtime(error.to_string()))?;
        if output_shape.len() != 3
            || usize::try_from(output_shape[0]).ok() != Some(batch)
            || usize::try_from(output_shape[1]).ok() != Some(sequence_length)
            || usize::try_from(output_shape[2]).ok() != Some(spec.dimensions)
        {
            return Err(EmbedError::Dimension {
                expected: batch * sequence_length * spec.dimensions,
                actual: values.len(),
            });
        }
        let mut vectors = Vec::with_capacity(batch);
        for row in 0..batch {
            let mut vector = vec![0.0_f32; spec.dimensions];
            let mut tokens = 0_u16;
            for column in 0..sequence_length {
                if attention_mask[row * sequence_length + column] == 0 {
                    continue;
                }
                tokens += 1;
                let start = (row * sequence_length + column) * spec.dimensions;
                for (output, component) in vector
                    .iter_mut()
                    .zip(&values[start..start + spec.dimensions])
                {
                    *output += *component;
                }
            }
            if tokens == 0 {
                return Err(EmbedError::Runtime(
                    "ONNX output has no unmasked tokens".to_owned(),
                ));
            }
            for value in &mut vector {
                *value /= f32::from(tokens);
            }
            l2_normalize(&mut vector);
            vectors.push(vector);
        }
        checked_embeddings(&self.identity(role), vectors)
    }
}

impl Embedder for OnnxEmbedder {
    fn identity(&self, role: InputRole) -> SpaceIdentity {
        let spec = self.kind.spec();
        SpaceIdentity {
            encoder_id: spec.encoder_id.to_owned(),
            revision: spec.revision.to_owned(),
            dimensions: spec.dimensions,
            distance: Distance::Cosine,
            normalization: Normalization::L2,
            input_role: role,
        }
    }

    fn embed_query(&self, query: &str) -> Result<Embedding, EmbedError> {
        self.embed(InputRole::Query, &[query])
            .map(|mut embeddings| embeddings.remove(0))
    }

    fn embed_documents(&self, documents: &[&str]) -> Result<Vec<Embedding>, EmbedError> {
        self.embed(InputRole::Document, documents)
    }
}

fn prefix(kind: ModelKind, role: InputRole, text: &str) -> String {
    match (kind, role) {
        (ModelKind::NomicEmbedTextV15, InputRole::Query) => format!("search_query: {text}"),
        (ModelKind::NomicEmbedTextV15, InputRole::Document) => {
            format!("search_document: {text}")
        }
        (ModelKind::BgeSmallEnV15, InputRole::Query) => {
            format!("Represent this sentence for searching relevant passages: {text}")
        }
        (ModelKind::BgeSmallEnV15, InputRole::Document) => text.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asymmetric_models_apply_role_prefixes() {
        assert_eq!(
            prefix(ModelKind::NomicEmbedTextV15, InputRole::Query, "hello"),
            "search_query: hello"
        );
        assert_eq!(
            prefix(ModelKind::NomicEmbedTextV15, InputRole::Document, "hello"),
            "search_document: hello"
        );
        assert!(prefix(ModelKind::BgeSmallEnV15, InputRole::Query, "hello").ends_with("hello"));
        assert_eq!(
            prefix(ModelKind::BgeSmallEnV15, InputRole::Document, "hello"),
            "hello"
        );
    }
}
