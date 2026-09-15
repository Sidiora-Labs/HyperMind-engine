#![deny(unsafe_code)]

pub mod bindings;
pub mod checkpoint;
pub mod entities;
pub mod intent;
pub mod ledger;
pub mod lexical;
pub mod rebuild;
pub mod spaces;
#[allow(unsafe_code)]
pub mod store;
pub mod timeline;
#[allow(unsafe_code)]
pub mod vectors;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
