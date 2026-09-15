#![forbid(unsafe_code)]

pub mod env;
pub mod fault;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
