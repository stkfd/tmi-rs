use std::borrow::Borrow;
use std::fmt::{Debug, Display};
use std::hash::Hash;

pub mod client_messages;
pub mod event_filter;
pub mod events;

/// Trait that is used when generically referring to a &str, String, or other type that can be
/// used just like a borrowed string
pub trait StringRef: Borrow<str> + Debug + Clone + Hash + Eq + Display {}
impl<T> StringRef for T where T: Borrow<str> + Debug + Clone + Hash + Eq + Display {}
