#![forbid(unsafe_code)]

pub mod bench;
pub mod slice1;
pub mod slice2;
pub mod slice3;
pub mod slice4;
pub mod slice5;
pub mod slice6;
pub mod suites;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
