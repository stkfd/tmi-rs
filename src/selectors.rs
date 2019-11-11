//! Convenience methods for matching specific event types

use std::hint::unreachable_unchecked;
use std::ops::Deref;
use std::sync::Arc;

use futures_core::Future;
use futures_util::future::ready;

use crate::event::{Event, EventData, PrivMsgEvent};

/// Contains a PRIVMSG event
#[derive(Clone, Debug)]
pub struct PrivMsgEventWrapper(Arc<Event<String>>);
impl Deref for PrivMsgEventWrapper {
    type Target = EventData<String, PrivMsgEvent<String>>;

    fn deref(&self) -> &Self::Target {
        match &*self.0 {
            Event::PrivMsg(evt) => evt,
            _ => unsafe { unreachable_unchecked() },
        }
    }
}

/// Match PRIVMSG events
pub fn priv_msg(item: Arc<Event<String>>) -> impl Future<Output = Option<PrivMsgEventWrapper>> {
    ready(match &*item {
        Event::PrivMsg(_) => Some(PrivMsgEventWrapper(item)),
        _ => None,
    })
}
