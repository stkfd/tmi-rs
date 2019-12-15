//! Helpers and extension traits to deal with streams of received events and sent messages.

use std::ops::Deref;
use std::pin::Pin;
use std::sync::Arc;

use futures_channel::mpsc;
use futures_core::Stream;

use dedup::*;
use rate_limits::*;

use crate::event::Event;
use crate::stream::split_oversize::SplitOversize;
use crate::{ClientMessage, Error};

pub mod dedup;
pub mod rate_limits;
pub mod split_oversize;

impl<St> SendStreamExt for St where St: Stream<Item = ClientMessage<String>> {}

/// Extension trait with function to manipulate the stream of outgoing messages
pub trait SendStreamExt: Stream<Item = ClientMessage<String>> {
    /// Composes a fixed size buffered rate limiter in front of this sink. Messages passed into
    /// the sink are treated according to their implementations of [`RateLimitable`](rate_limits/trait.RateLimitable.html).
    /// Messages that report rate limiting applies are put into a buffer which is sent  
    fn rate_limited(
        self,
        capacity: usize,
        rate_limiter: &Arc<RateLimiter>,
    ) -> BufferedRateLimiter<Self, ClientMessage<String>>
    where
        Self: Sized + Unpin,
    {
        BufferedRateLimiter::new(self, capacity, rate_limiter)
    }

    /// Automatically add `\u{0}` to de-duplicate subsequent messages, to bypass Twitch's 30 second
    /// duplicate message detection
    fn dedup(self) -> DedupMessages<Self>
    where
        Self: Sized + Unpin,
    {
        DedupMessages::new(self)
    }

    /// Splits messages over the given size limit into separate messages
    fn split_oversize(self, max_len: usize) -> SplitOversize<Self>
    where
        Self: Sized + Unpin,
    {
        SplitOversize::new(self, max_len)
    }
}

impl<St, E: Deref<Target = Event<String>>> ReceiveStreamExt<E> for St where St: Stream<Item = E> {}

/// Extension trait with functions to manipulate the incoming stream of events
pub trait ReceiveStreamExt<E: Deref<Target = Event<String>>>: Stream<Item = E> {}

/// Auto-implemented shortctut trait for a stream of `Event<String>`, used in receiver middlewares
pub trait EventStream: Stream<Item = Result<Event<String>, Error>> + Unpin + Send {}

impl<T: Stream<Item = Result<Event<String>, Error>> + Unpin + Send> EventStream for T {}

/// Setup function for message receiver middleware
pub type RecvMiddlewareConstructor =
    Arc<dyn Fn(Box<dyn EventStream>) -> Pin<Box<dyn EventStream>> + Send + Sync + 'static>;

/// Auto-implemented shortcut trait for any stream of `ClientMessage` objects
pub trait ClientMessageStream: Stream<Item = ClientMessage<String>> + Unpin + Send {}

impl<T: Stream<Item = ClientMessage<String>> + Unpin + Send> ClientMessageStream for T {}

/// Setup function for message sender middleware
pub type SendMiddlewareConstructor = Arc<
    dyn Fn(mpsc::UnboundedReceiver<ClientMessage<String>>) -> Pin<Box<dyn ClientMessageStream>>
        + Send
        + Sync
        + 'static,
>;
