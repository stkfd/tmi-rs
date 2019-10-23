//! Convenience functions for matching specific event types

use crate::events::Event;
use crate::StringRef;

/// Match any normal channel message
pub fn priv_msg<T: StringRef>(evt: &Event<T>) -> bool {
    match evt {
        Event::PrivMsg(_) => true,
        _ => false,
    }
}
