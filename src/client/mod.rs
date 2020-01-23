//! Client module, includes websocket connection handling, listener and handler registration

use tokio::time::Duration;

pub use config::*;
pub use single::*;

mod config;
mod pool;
mod raw;
mod single;

const RECONNECT_DELAY: Duration = Duration::from_secs(5);
