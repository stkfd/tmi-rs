//! Client module, includes websocket connection handling, listener and handler registration

use tokio::time::Duration;

pub use config::*;

mod config;

/// Managed connection pools
pub mod pool;
/// Create a raw connection with no extras
pub mod raw;
/// Single, "batteries included" connections
pub mod single;

const RECONNECT_DELAY: Duration = Duration::from_secs(5);
