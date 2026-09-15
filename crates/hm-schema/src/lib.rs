#![deny(unsafe_code)]

#[allow(clippy::all, clippy::pedantic, unsafe_code)]
mod events_generated;
#[allow(clippy::all, clippy::pedantic, unsafe_code)]
mod protocol_generated;

pub mod event;
pub mod protocol;
pub mod validate;

pub use events_generated::hypermind::schema as events;
pub use protocol_generated::hypermind::protocol as wire;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
