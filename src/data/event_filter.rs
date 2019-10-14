use crate::events::{Event, EventContent};
use std::fmt::Debug;
use std::hash::Hash;

pub fn channel_message<T: Clone + Debug + Eq + Hash>(evt: &Event<T>) -> bool {
    match evt.event() {
        EventContent::PrivMsg(_) => true,
        _ => false,
    }
}
