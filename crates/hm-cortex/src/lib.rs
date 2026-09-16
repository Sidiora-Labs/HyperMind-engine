#![forbid(unsafe_code)]

pub mod adjudicate;
pub mod attention;
pub mod authority;
pub mod budget;
pub mod citations;
pub mod connectors;
pub mod fsrs;
pub mod ingest;
pub mod media;
pub mod nli;
pub mod nrem;
pub mod playbook;
pub mod predict;
pub mod procedures;
pub mod prospective;
pub mod quality;
pub mod relations;
pub mod rem;
pub mod repograph;
pub mod review;
pub mod run;
pub mod vocabulary;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
