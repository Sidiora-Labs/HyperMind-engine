#![forbid(unsafe_code)]

pub mod bm25;
pub mod entity_rules;
#[cfg(feature = "hnsw")]
pub mod hnsw;
pub mod simd;
pub mod tokenize;
pub mod vocabulary;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
