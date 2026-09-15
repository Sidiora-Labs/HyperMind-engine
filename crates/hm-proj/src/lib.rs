#![deny(unsafe_code)]

pub mod lexical;
pub mod rebuild;
#[allow(unsafe_code)]
pub mod store;
pub mod timeline;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
