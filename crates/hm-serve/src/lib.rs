#![forbid(unsafe_code)]

pub mod actor;
pub mod admin;
pub mod anticipation;
pub mod auth;
pub mod config;
pub mod embedded;
pub mod errors;
pub mod protocol;
pub mod requests;
pub mod uds;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
