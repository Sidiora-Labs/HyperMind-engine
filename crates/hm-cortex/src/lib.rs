#![forbid(unsafe_code)]

pub mod adjudicate;
pub mod authority;
pub mod budget;
pub mod citations;
pub mod fsrs;
pub mod ingest;
pub mod nli;
pub mod nrem;
pub mod quality;
pub mod rem;
pub mod review;
pub mod run;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
