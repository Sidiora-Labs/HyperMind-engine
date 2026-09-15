#![forbid(unsafe_code)]

pub mod budget;
pub mod bundle;
pub mod canonical;
pub mod lanes;
pub mod manifest;
pub mod safety;
pub mod tiers;
pub mod tokens;
pub mod trim;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
