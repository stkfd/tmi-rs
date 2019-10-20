use std::borrow::Borrow;
use std::fmt::Debug;
use std::hash::Hash;

pub mod client_messages;
pub mod event_filter;
pub mod events;

pub trait StringRef: Borrow<str> + Debug + Clone + Hash + Eq {}
impl<T> StringRef for T where T: Borrow<str> + Debug + Clone + Hash + Eq {}
