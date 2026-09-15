#![forbid(unsafe_code)]

pub mod apply;
pub mod frame;
pub mod idempotency;
pub mod keyring;
pub mod seal;
pub mod segment;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
