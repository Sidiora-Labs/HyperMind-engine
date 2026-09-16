#![forbid(unsafe_code)]

pub mod chunk;
pub mod diff;
pub mod formats;
pub mod identity;
pub mod loader;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
