use crate::types::{EmbedError, Embedder, Embedding, InputRole, SpaceIdentity};
use std::collections::HashMap;
use std::sync::Mutex;

pub struct CachedEmbedder<E> {
    inner: E,
    maximum_batch: usize,
    cache: Mutex<HashMap<[u8; 32], Embedding>>,
}

impl<E> CachedEmbedder<E> {
    pub fn new(inner: E, maximum_batch: usize) -> Result<Self, EmbedError> {
        if maximum_batch == 0 {
            return Err(EmbedError::InvalidArgument(
                "maximum batch must be non-zero",
            ));
        }
        Ok(Self {
            inner,
            maximum_batch,
            cache: Mutex::new(HashMap::new()),
        })
    }

    #[must_use]
    pub fn cached_items(&self) -> usize {
        self.cache.lock().map_or(0, |cache| cache.len())
    }
}

impl<E: Embedder> CachedEmbedder<E> {
    fn key(&self, role: InputRole, text: &str) -> [u8; 32] {
        let identity = self.inner.identity(role);
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"hypermind.embedding-cache.v1\0");
        hasher.update(identity.encoder_id.as_bytes());
        hasher.update(&[0]);
        hasher.update(identity.revision.as_bytes());
        hasher.update(&identity.dimensions.to_le_bytes());
        hasher.update(&[role as u8]);
        hasher.update(text.as_bytes());
        *hasher.finalize().as_bytes()
    }

    fn embed_role(&self, role: InputRole, texts: &[&str]) -> Result<Vec<Embedding>, EmbedError> {
        let keys: Vec<_> = texts.iter().map(|text| self.key(role, text)).collect();
        let mut output: Vec<Option<Embedding>> = vec![None; texts.len()];
        {
            let cache = self
                .cache
                .lock()
                .map_err(|_| EmbedError::Runtime("embedding cache poisoned".to_owned()))?;
            for (index, key) in keys.iter().enumerate() {
                output[index] = cache.get(key).cloned();
            }
        }
        let missing: Vec<_> = output
            .iter()
            .enumerate()
            .filter_map(|(index, value)| value.is_none().then_some(index))
            .collect();
        for indices in missing.chunks(self.maximum_batch) {
            let batch: Vec<_> = indices.iter().map(|index| texts[*index]).collect();
            let embeddings = match role {
                InputRole::Query => batch
                    .iter()
                    .map(|text| self.inner.embed_query(text))
                    .collect::<Result<Vec<_>, _>>()?,
                InputRole::Document => self.inner.embed_documents(&batch)?,
            };
            if embeddings.len() != indices.len() {
                return Err(EmbedError::Wire(
                    "embedder returned the wrong batch size".to_owned(),
                ));
            }
            let mut cache = self
                .cache
                .lock()
                .map_err(|_| EmbedError::Runtime("embedding cache poisoned".to_owned()))?;
            for (batch_index, embedding) in embeddings.into_iter().enumerate() {
                let index = indices[batch_index];
                let key = keys[index];
                cache.insert(key, embedding.clone());
                output[index] = Some(embedding);
            }
        }
        output
            .into_iter()
            .map(|value| {
                value.ok_or_else(|| EmbedError::Runtime("embedding cache miss".to_owned()))
            })
            .collect()
    }
}

impl<E: Embedder> Embedder for CachedEmbedder<E> {
    fn identity(&self, role: InputRole) -> SpaceIdentity {
        self.inner.identity(role)
    }

    fn embed_query(&self, query: &str) -> Result<Embedding, EmbedError> {
        self.embed_role(InputRole::Query, &[query])
            .map(|mut values| values.remove(0))
    }

    fn embed_documents(&self, documents: &[&str]) -> Result<Vec<Embedding>, EmbedError> {
        self.embed_role(InputRole::Document, documents)
    }

    fn report_label(&self) -> String {
        self.inner.report_label()
    }
}
