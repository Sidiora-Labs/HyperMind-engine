#![forbid(unsafe_code)]

pub mod bundle;
pub mod canonical;
pub mod lanes;
pub mod tokens;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
