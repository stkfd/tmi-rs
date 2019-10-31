//! Convenience functions for matching specific event types

use crate::event::Event;
use crate::{Error, StringRef};
use std::sync::Arc;

/*pub async fn all_ok<'a, T: StringRef>(evt: &'a Arc<Result<Event<T>, Error>>) -> bool {
    (*evt).is_ok()
}*/

/// Match any normal channel message
pub async fn priv_msg<T: StringRef>(evt: &Arc<Result<Event<T>, Error>>) -> bool {
    match **evt {
        Ok(Event::PrivMsg(_)) => true,
        _ => false,
    }
}
