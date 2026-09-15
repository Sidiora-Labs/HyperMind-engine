#![forbid(unsafe_code)]

pub mod bm25;
pub mod tokenize;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
