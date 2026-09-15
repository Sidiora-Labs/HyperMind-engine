#![forbid(unsafe_code)]

pub mod bm25;
pub mod entity_rules;
pub mod simd;
pub mod tokenize;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
