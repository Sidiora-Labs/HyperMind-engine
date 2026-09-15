use crate::types::{
    Distance, EmbedError, Embedder, Embedding, InputRole, Normalization, SpaceIdentity,
    checked_embeddings,
};

pub const LEXICAL_ONLY_LABEL: &str = "lexical_only";

#[derive(Clone, Debug)]
pub struct HashFeatureEmbedder {
    dimensions: usize,
}

impl HashFeatureEmbedder {
    pub fn new(dimensions: usize) -> Result<Self, EmbedError> {
        if dimensions == 0 {
            return Err(EmbedError::InvalidArgument("dimensions must be non-zero"));
        }
        Ok(Self { dimensions })
    }

    fn vector(&self, text: &str) -> Vec<f32> {
        let mut vector = vec![0.0_f32; self.dimensions];
        let dimensions = u64::try_from(self.dimensions).expect("dimensions fit in u64");
        for token in text.split_whitespace() {
            let digest = blake3::hash(token.as_bytes());
            let bytes = digest.as_bytes();
            let slot = u64::from_le_bytes(bytes[..8].try_into().expect("eight bytes")) % dimensions;
            let sign = if bytes[8] & 1 == 0 { 1.0 } else { -1.0 };
            vector[usize::try_from(slot).expect("slot fits in usize")] += sign;
        }
        l2_normalize(&mut vector);
        vector
    }
}

impl Embedder for HashFeatureEmbedder {
    fn identity(&self, role: InputRole) -> SpaceIdentity {
        SpaceIdentity {
            encoder_id: LEXICAL_ONLY_LABEL.to_owned(),
            revision: "hash-feature-v1".to_owned(),
            dimensions: self.dimensions,
            distance: Distance::Cosine,
            normalization: Normalization::L2,
            input_role: role,
        }
    }

    fn embed_query(&self, query: &str) -> Result<Embedding, EmbedError> {
        checked_embeddings(&self.identity(InputRole::Query), vec![self.vector(query)])
            .map(|mut embeddings| embeddings.remove(0))
    }

    fn embed_documents(&self, documents: &[&str]) -> Result<Vec<Embedding>, EmbedError> {
        checked_embeddings(
            &self.identity(InputRole::Document),
            documents
                .iter()
                .map(|document| self.vector(document))
                .collect(),
        )
    }

    fn report_label(&self) -> String {
        LEXICAL_ONLY_LABEL.to_owned()
    }
}

pub(crate) fn l2_normalize(vector: &mut [f32]) {
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in vector {
            *value /= norm;
        }
    }
}
