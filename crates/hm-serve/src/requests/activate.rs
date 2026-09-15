#![allow(clippy::missing_errors_doc)]

use hm_embed::{EmbedError, Embedder, InputRole, QuantizedEmbedding, SpaceIdentity, quantize};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryEmbeddingSource {
    ConfiguredEmbedder,
    Precomputed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PreparedQueryEmbedding {
    pub space: SpaceIdentity,
    pub quantized: Vec<i8>,
    pub binary_prefilter: Vec<u8>,
    pub inverse_scale: f32,
    pub source: QueryEmbeddingSource,
}

pub fn prepare_query_embedding<E: Embedder>(
    embedder: Option<&E>,
    query: &str,
    precomputed: Option<QuantizedEmbedding>,
) -> Result<PreparedQueryEmbedding, EmbedError> {
    if let Some(precomputed) = precomputed {
        validate_precomputed(&precomputed)?;
        return Ok(PreparedQueryEmbedding {
            space: precomputed.space,
            quantized: precomputed.values,
            binary_prefilter: precomputed.binary_prefilter,
            inverse_scale: precomputed.inverse_scale,
            source: QueryEmbeddingSource::Precomputed,
        });
    }
    let embedder = embedder.ok_or(EmbedError::InvalidArgument(
        "query embedder or precomputed embedding is required",
    ))?;
    let quantized = quantize(&embedder.embed_query(query)?)?;
    validate_precomputed(&quantized)?;
    Ok(PreparedQueryEmbedding {
        space: quantized.space,
        quantized: quantized.values,
        binary_prefilter: quantized.binary_prefilter,
        inverse_scale: quantized.inverse_scale,
        source: QueryEmbeddingSource::ConfiguredEmbedder,
    })
}

fn validate_precomputed(embedding: &QuantizedEmbedding) -> Result<(), EmbedError> {
    if embedding.space.input_role != InputRole::Query
        || embedding.space.dimensions == 0
        || embedding.values.len() != embedding.space.dimensions
        || embedding.binary_prefilter.len() != embedding.space.dimensions.div_ceil(8)
        || !embedding.inverse_scale.is_finite()
        || embedding.inverse_scale <= 0.0
    {
        Err(EmbedError::InvalidArgument(
            "precomputed query embedding is invalid",
        ))
    } else {
        Ok(())
    }
}
