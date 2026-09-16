#![forbid(unsafe_code)]

mod error;
mod ids;
pub mod telemetry;

pub use error::{Error, ErrorCode};
pub use ids::{ActorId, ConversationId, EntityId, LSN, Lsn, SchemaVersion, UtcNanos};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
