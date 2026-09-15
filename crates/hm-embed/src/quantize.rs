use crate::types::{EmbedError, Embedding, SpaceIdentity};

#[derive(Clone, Debug, PartialEq)]
pub struct QuantizedEmbedding {
    pub space: SpaceIdentity,
    pub values: Vec<i8>,
    pub binary_prefilter: Vec<u8>,
    pub inverse_scale: f32,
}

#[allow(clippy::cast_possible_truncation)]
pub fn quantize(embedding: &Embedding) -> Result<QuantizedEmbedding, EmbedError> {
    if embedding.values.len() != embedding.space.dimensions {
        return Err(EmbedError::Dimension {
            expected: embedding.space.dimensions,
            actual: embedding.values.len(),
        });
    }
    if embedding.values.iter().any(|value| !value.is_finite()) {
        return Err(EmbedError::InvalidArgument(
            "embedding contains a non-finite value",
        ));
    }
    let maximum = embedding
        .values
        .iter()
        .map(|value| value.abs())
        .fold(0.0_f32, f32::max);
    let inverse_scale = if maximum == 0.0 { 1.0 } else { maximum / 127.0 };
    let values = embedding
        .values
        .iter()
        .map(|value| (value / inverse_scale).round().clamp(-127.0, 127.0) as i8)
        .collect();
    let mut binary_prefilter = vec![0_u8; embedding.values.len().div_ceil(8)];
    for (index, value) in embedding.values.iter().enumerate() {
        if *value >= 0.0 {
            binary_prefilter[index / 8] |= 1 << (index % 8);
        }
    }
    Ok(QuantizedEmbedding {
        space: embedding.space.clone(),
        values,
        binary_prefilter,
        inverse_scale,
    })
}
