#![allow(clippy::missing_errors_doc)]

use hm_core::{Error, ErrorCode};
use tokenizers::Tokenizer;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FallbackWeights {
    pub per_byte_q8: [u16; 256],
    pub item_overhead: u32,
}

impl Default for FallbackWeights {
    fn default() -> Self {
        Self {
            per_byte_q8: [256; 256],
            item_overhead: 0,
        }
    }
}

pub enum TokenCounter {
    Tiktoken {
        model_id: String,
    },
    HuggingFace {
        model_id: String,
        tokenizer: Box<Tokenizer>,
    },
    Fallback {
        model_id: String,
        weights: Box<FallbackWeights>,
    },
}

impl TokenCounter {
    pub fn for_model(
        model_id: impl Into<String>,
        hugging_face_tokenizer_json: Option<&[u8]>,
        fallback: FallbackWeights,
    ) -> Result<Self, Error> {
        let model_id = model_id.into();
        if tiktoken_rs::bpe_for_model(&model_id).is_ok() {
            return Ok(Self::Tiktoken { model_id });
        }
        if let Some(json) = hugging_face_tokenizer_json {
            let tokenizer =
                Tokenizer::from_bytes(json).map_err(|_| Error::new(ErrorCode::InvalidArgument))?;
            return Ok(Self::HuggingFace {
                model_id,
                tokenizer: Box::new(tokenizer),
            });
        }
        Ok(Self::Fallback {
            model_id,
            weights: Box::new(fallback),
        })
    }

    #[must_use]
    pub fn model_id(&self) -> &str {
        match self {
            Self::Tiktoken { model_id }
            | Self::HuggingFace { model_id, .. }
            | Self::Fallback { model_id, .. } => model_id,
        }
    }

    pub fn count(&self, content: &[u8]) -> Result<usize, Error> {
        let tokens = match self {
            Self::Tiktoken { model_id } => {
                let text = std::str::from_utf8(content)
                    .map_err(|_| Error::new(ErrorCode::InvalidArgument))?;
                tiktoken_rs::bpe_for_model(model_id)
                    .map_err(|_| Error::new(ErrorCode::InvalidArgument))?
                    .encode_ordinary(text)
                    .len()
            }
            Self::HuggingFace { tokenizer, .. } => {
                let text = std::str::from_utf8(content)
                    .map_err(|_| Error::new(ErrorCode::InvalidArgument))?;
                tokenizer
                    .encode(text, false)
                    .map_err(|_| Error::new(ErrorCode::InvalidArgument))?
                    .len()
            }
            Self::Fallback { weights, .. } => {
                let weighted = content.iter().try_fold(0_u64, |total, byte| {
                    total
                        .checked_add(u64::from(weights.per_byte_q8[usize::from(*byte)]))
                        .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))
                })?;
                usize::try_from(
                    u64::from(weights.item_overhead)
                        .checked_add((weighted + 255) >> 8)
                        .ok_or_else(|| Error::new(ErrorCode::CapacityExceeded))?,
                )
                .map_err(|_| Error::new(ErrorCode::CapacityExceeded))?
            }
        };
        Ok(tokens.max(1))
    }
}
