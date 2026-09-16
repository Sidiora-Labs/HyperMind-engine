#![forbid(unsafe_code)]

pub mod adjudicate;
pub mod authority;
pub mod citations;
pub mod ingest;
pub mod nli;
pub mod nrem;
pub mod quality;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
