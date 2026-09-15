#![forbid(unsafe_code)]

pub mod actor;
pub mod config;
pub mod embedded;
pub mod protocol;
pub mod uds;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
