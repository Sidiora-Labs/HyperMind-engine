#![forbid(unsafe_code)]

pub mod authority;
pub mod ingest;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
