#![deny(unsafe_code)]

pub mod attention;
pub mod attestations;
pub mod beliefs;
pub mod bindings;
pub mod checkpoint;
pub mod documents;
pub mod entities;
pub mod fsrs;
pub mod generation;
pub mod graph;
pub mod intent;
pub mod intentions;
pub mod ladder;
pub mod lease;
pub mod ledger;
pub mod lexical;
pub mod memories;
pub mod predictions;
pub mod procedures;
pub mod protected;
pub mod rebuild;
pub mod runs;
pub mod spaces;
#[allow(unsafe_code)]
pub mod store;
pub mod timeline;
#[allow(unsafe_code)]
pub mod vectors;
#[cfg(feature = "hnsw")]
pub mod vectors_hnsw;
pub mod vocabulary;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
