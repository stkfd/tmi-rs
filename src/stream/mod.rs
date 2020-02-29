//! Helpers and extension traits to deal with streams of received events and sent messages.

use std::ops::Deref;
use std::sync::Arc;

use futures_core::Stream;

use dedup::*;
use rate_limits::*;

use crate::event::Event;
use crate::stream::split_oversize::SplitOversize;
use crate::{ClientMessage, Error, MessageResponse, MessageSendError};
use std::borrow::Borrow;
use tokio::sync::oneshot;

pub mod dedup;
pub mod rate_limits;
pub mod split_oversize;

/// A message and a receiver for its result/response
#[derive(Debug)]
pub struct SentClientMessage {
    pub(crate) message: ClientMessage,
    pub(crate) responder: MessageResponder,
}

/// Sender part of a oneshot channel to send the result or response of sending a message back
/// to the code that sent it
pub(crate) type MessageResponder = oneshot::Sender<Result<MessageResponse, MessageSendError>>;

#[inline]
pub(crate) fn message_responder_channel() -> (
    MessageResponder,
    oneshot::Receiver<Result<MessageResponse, MessageSendError>>,
) {
    oneshot::channel()
}

impl RateLimitable for SentClientMessage {
    fn channel_limits(&self) -> Option<&str> {
        self.message.channel_limits()
    }
}

pub(crate) trait RespondWithErrors {
    fn respond_with_errors(self, responder: MessageResponder);
}

impl<T, E> RespondWithErrors for Result<T, E>
where
    E: Into<MessageSendError>,
{
    fn respond_with_errors(self, responder: MessageResponder) {
        self.map_err(|e| responder.send(Err(e.into()))).ok();
    }
}

impl<St> SendStreamExt for St where St: Stream<Item = SentClientMessage> {}

/// Extension trait with function to manipulate the stream of outgoing messages
pub trait SendStreamExt: Stream<Item = SentClientMessage> {
    /// Composes a fixed size buffered rate limiter in front of this sink. Messages passed into
    /// the sink are treated according to their implementations of [`RateLimitable`](rate_limits/trait.RateLimitable.html).
    /// Messages that report rate limiting applies are put into a buffer which is sent  
    fn rate_limited<Rl: Borrow<RateLimiter>>(
        self,
        capacity: usize,
        rate_limiter: Rl,
    ) -> BufferedRateLimiter<Self, SentClientMessage, Rl>
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
pub type RecvMiddleware =
    Arc<dyn Fn(Result<Event, Error>) -> Result<Option<Event>, Error> + Send + Sync + 'static>;

/// Auto-implemented shortcut trait for any stream of `ClientMessage` objects
pub trait ClientMessageStream: Stream<Item = SentClientMessage> + Unpin + Send + Sync {}

impl<T: Stream<Item = SentClientMessage> + Unpin + Send + Sync> ClientMessageStream for T {}

/// Setup function for message sender middleware
pub type SendMiddleware =
    Arc<dyn Fn(ClientMessage) -> Result<Option<SentClientMessage>, Error> + Send + Sync + 'static>;
