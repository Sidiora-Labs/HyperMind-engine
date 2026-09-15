#![forbid(unsafe_code)]

pub mod adjudicate;
pub mod authority;
pub mod ingest;
pub mod nli;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
