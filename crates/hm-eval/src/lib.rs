#![forbid(unsafe_code)]

pub mod slice1;
pub mod slice2;
pub mod suites;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
